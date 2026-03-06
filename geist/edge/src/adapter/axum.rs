//! Axum adapter — bridges axum HTTP server to the processor pipeline.
//!
//! Zero-copy: `Request::into_parts()` gives `&Parts` directly to processors.
//! No intermediate protobuf types, no header copying.
//!
//! # P2 Scope
//!
//! Pipeline runs, returns result directly. No upstream forwarding (P2.5).
//! - Denied → 403 with ImmediateResponse body
//! - Allowed → 200 stub (pipeline passed, no upstream yet)
//! - Error → 500

use std::sync::Arc;

use ::axum::{
    extract::State,
    response::{IntoResponse, Response},
    Router,
};
use http::StatusCode;
use tracing::Instrument;

use crate::compositor::{Sequence, SequenceOutcome};
use crate::phase::ImmediateResponse;

/// Shared state for the proxy handler.
#[derive(Clone)]
pub struct ProxyState {
    pub pipeline: Arc<Sequence>,
}

/// Catch-all proxy handler.
///
/// Zero-copy path: `req.into_parts().0` gives `http::request::Parts`
/// directly — no allocation, no copying.
async fn proxy_handler(
    State(state): State<ProxyState>,
    req: ::axum::extract::Request,
) -> Response {
    let (parts, _body) = req.into_parts();
    let span = tracing::info_span!(
        "request",
        method = %parts.method,
        uri = %parts.uri,
        otel.kind = "server",
    );

    // Use instrument(), not span.enter() — enter() is not Send-safe
    // across .await points in multi-threaded tokio.
    handle_request(state, parts).instrument(span).await
}

async fn handle_request(state: ProxyState, parts: http::request::Parts) -> Response {
    let outcome = state.pipeline.process_request_headers(&parts).await;

    match outcome {
        SequenceOutcome::Continue(_mutations) => {
            // P2: no upstream forwarding yet. Return stub 200.
            // mutations would be applied to forwarded request in P2.5.
            tracing::info!(status = 200, "pipeline passed");
            (StatusCode::OK, "pipeline passed").into_response()
        }
        SequenceOutcome::Respond(immediate) => {
            tracing::info!(status = %immediate.status, "immediate response");
            immediate.into_response()
        }
        SequenceOutcome::Error(err) => {
            tracing::error!(error = %err, "pipeline error");
            (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
        }
    }
}

impl IntoResponse for ImmediateResponse {
    fn into_response(self) -> Response {
        let mut res = Response::new(::axum::body::Body::from(self.body));
        *res.status_mut() = self.status;
        *res.headers_mut() = self.headers;
        res
    }
}

/// Build an axum Router with the processor pipeline as a catch-all handler.
pub fn router(pipeline: Arc<Sequence>) -> Router {
    let state = ProxyState { pipeline };
    Router::new()
        .fallback(proxy_handler)
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::axum::body::Body;
    use http::Request;
    use tower::ServiceExt;

    use crate::phase::PhaseResult;
    use crate::processor::{BoxFuture, Processor, ProcessorError};

    // -- Test processors --

    struct AllowProcessor;
    impl Processor for AllowProcessor {
        fn name(&self) -> &str {
            "allow"
        }
    }

    struct DenyProcessor;
    impl Processor for DenyProcessor {
        fn name(&self) -> &str {
            "deny"
        }
        fn process_request_headers(
            &self,
            _parts: &http::request::Parts,
        ) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>> {
            Box::pin(async {
                Ok(PhaseResult::Respond(
                    ImmediateResponse::with_status(StatusCode::FORBIDDEN)
                        .body("access denied")
                        .header(
                            http::header::CONTENT_TYPE,
                            http::HeaderValue::from_static("text/plain"),
                        ),
                ))
            })
        }
    }

    struct ErrorProcessor;
    impl Processor for ErrorProcessor {
        fn name(&self) -> &str {
            "error"
        }
        fn process_request_headers(
            &self,
            _parts: &http::request::Parts,
        ) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>> {
            Box::pin(async {
                Err(ProcessorError::new("error", "something broke"))
            })
        }
    }

    async fn response_body(resp: Response) -> String {
        let bytes = ::axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn empty_pipeline_returns_200() {
        let app = router(Arc::new(Sequence::builder().build()));
        let req = Request::get("/test").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(response_body(resp).await, "pipeline passed");
    }

    #[tokio::test]
    async fn allow_processor_returns_200() {
        let pipeline = Sequence::builder()
            .processor(Arc::new(AllowProcessor))
            .build();
        let app = router(Arc::new(pipeline));
        let req = Request::get("/test").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn deny_processor_returns_403() {
        let pipeline = Sequence::builder()
            .processor(Arc::new(DenyProcessor))
            .build();
        let app = router(Arc::new(pipeline));
        let req = Request::get("/test").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            resp.headers().get(http::header::CONTENT_TYPE).unwrap(),
            "text/plain"
        );
        assert_eq!(response_body(resp).await, "access denied");
    }

    #[tokio::test]
    async fn error_processor_returns_500() {
        let pipeline = Sequence::builder()
            .processor(Arc::new(ErrorProcessor))
            .build();
        let app = router(Arc::new(pipeline));
        let req = Request::get("/test").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn catches_all_paths_and_methods() {
        let app = router(Arc::new(Sequence::builder().build()));

        // GET /foo
        let req = Request::get("/foo").body(Body::empty()).unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // POST /bar/baz
        let req = Request::post("/bar/baz").body(Body::empty()).unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // DELETE /
        let req = Request::delete("/").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
