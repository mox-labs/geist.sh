//! Axum adapter — governed reverse proxy.
//!
//! Translates axum HTTP requests into ext_proc `ProcessingRequest` envelopes,
//! runs them through the processor pipeline, forwards to upstream, and runs
//! the response through the pipeline before returning to the client.
//!
//! # Request Flow (headers-only for P2)
//!
//! ```text
//! client → axum → translate → Sequence(request_headers)
//!                                  ↓
//!          client ← translate ← Sequence(response_headers) ← upstream
//! ```

use std::net::SocketAddr;
use std::sync::Arc;

use ::axum::extract::{Request, State};
use ::axum::response::{IntoResponse, Response};
use ::axum::Router;
use hyper::Uri;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;

use envoy_grpc_ext_proc::envoy::{
    config::core::v3::{HeaderMap as ExtHeaderMap, HeaderValue as ExtHeaderValue},
    service::ext_proc::v3::{
        processing_request::Request as ExtRequest, HeaderMutation, HttpHeaders,
        ImmediateResponse, ProcessingRequest,
    },
};
use rumi_http::HttpMessage;

use crate::compositor::{FailureMode, Sequence, SequenceOutcome};
use crate::registry::{ProcessorRegistry, TypedConfig};

// ─── Configuration ──────────────────────────────────────────────────────────

/// Proxy configuration loaded from JSON.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProxyConfig {
    /// Listen address (e.g., "127.0.0.1:3000").
    pub listen: SocketAddr,
    /// Upstream target URL (e.g., "http://127.0.0.1:8080").
    pub upstream: String,
    /// Processor pipeline config entries.
    pub pipeline: Vec<TypedConfig>,
    /// Pipeline failure mode. Default: fail-closed.
    #[serde(default)]
    pub failure_mode: FailureModeConfig,
}

/// Serialization wrapper for [`FailureMode`].
#[derive(Debug, Clone, Copy, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureModeConfig {
    #[default]
    FailClosed,
    FailOpen,
}

impl From<FailureModeConfig> for FailureMode {
    fn from(c: FailureModeConfig) -> Self {
        match c {
            FailureModeConfig::FailClosed => FailureMode::FailClosed,
            FailureModeConfig::FailOpen => FailureMode::FailOpen,
        }
    }
}

// ─── Shared State ───────────────────────────────────────────────────────────

/// Shared state for the proxy handler. Wrapped in `Arc` by axum.
pub struct ProxyState {
    pub sequence: Sequence,
    pub upstream: Uri,
    pub client: Client<hyper_util::client::legacy::connect::HttpConnector, axum::body::Body>,
}

// ─── Translation: axum → ext_proc ───────────────────────────────────────────

/// Build a `ProcessingRequest(request_headers)` from an axum request.
///
/// Includes HTTP/2-style pseudo-headers (`:method`, `:path`, `:authority`)
/// so processors see a normalized view regardless of HTTP version.
pub fn request_to_processing_request(req: &Request) -> ProcessingRequest {
    let mut headers: Vec<ExtHeaderValue> = Vec::with_capacity(req.headers().len() + 3);

    // Pseudo-headers first (matches ext_proc HTTP/2 convention).
    headers.push(ExtHeaderValue {
        key: ":method".into(),
        value: req.method().to_string(),
        raw_value: vec![],
    });
    headers.push(ExtHeaderValue {
        key: ":path".into(),
        value: req
            .uri()
            .path_and_query()
            .map(|pq| pq.to_string())
            .unwrap_or_else(|| "/".into()),
        raw_value: vec![],
    });
    if let Some(authority) = req.uri().authority() {
        headers.push(ExtHeaderValue {
            key: ":authority".into(),
            value: authority.to_string(),
            raw_value: vec![],
        });
    } else if let Some(host) = req.headers().get("host") {
        headers.push(ExtHeaderValue {
            key: ":authority".into(),
            value: host.to_str().unwrap_or("").to_string(),
            raw_value: vec![],
        });
    }

    // Regular headers.
    for (name, value) in req.headers() {
        headers.push(ExtHeaderValue {
            key: name.to_string(),
            value: value.to_str().unwrap_or("").to_string(),
            raw_value: vec![],
        });
    }

    let has_body = !matches!(
        req.method(),
        &hyper::Method::GET | &hyper::Method::HEAD | &hyper::Method::DELETE | &hyper::Method::OPTIONS
    );

    ProcessingRequest {
        request: Some(ExtRequest::RequestHeaders(HttpHeaders {
            headers: Some(ExtHeaderMap { headers }),
            end_of_stream: !has_body,
            ..Default::default()
        })),
        ..Default::default()
    }
}

/// Build a `ProcessingRequest(response_headers)` from an upstream response.
pub fn response_to_processing_request(resp: &hyper::Response<hyper::body::Incoming>) -> ProcessingRequest {
    let mut headers: Vec<ExtHeaderValue> = Vec::with_capacity(resp.headers().len() + 1);

    // :status pseudo-header.
    headers.push(ExtHeaderValue {
        key: ":status".into(),
        value: resp.status().as_u16().to_string(),
        raw_value: vec![],
    });

    for (name, value) in resp.headers() {
        headers.push(ExtHeaderValue {
            key: name.to_string(),
            value: value.to_str().unwrap_or("").to_string(),
            raw_value: vec![],
        });
    }

    ProcessingRequest {
        request: Some(ExtRequest::ResponseHeaders(HttpHeaders {
            headers: Some(ExtHeaderMap { headers }),
            end_of_stream: false,
            ..Default::default()
        })),
        ..Default::default()
    }
}

// ─── Mutation Application ───────────────────────────────────────────────────

/// Apply header mutations to an outgoing request (before forwarding to upstream).
pub fn apply_request_mutations(
    req: &mut hyper::Request<axum::body::Body>,
    mutations: &[HeaderMutation],
) {
    for mutation in mutations {
        for hvo in &mutation.set_headers {
            if let Some(hv) = &hvo.header {
                // Skip pseudo-headers — they can't be set as regular headers.
                if hv.key.starts_with(':') {
                    continue;
                }
                if let (Ok(name), Ok(value)) = (
                    hyper::header::HeaderName::from_bytes(hv.key.as_bytes()),
                    hyper::header::HeaderValue::from_str(&hv.value),
                ) {
                    req.headers_mut().insert(name, value);
                }
            }
        }
        for name in &mutation.remove_headers {
            if !name.starts_with(':') {
                req.headers_mut().remove(name.as_str());
            }
        }
    }
}

/// Apply header mutations to a response (before returning to client).
pub fn apply_response_mutations(
    resp: &mut hyper::Response<axum::body::Body>,
    mutations: &[HeaderMutation],
) {
    for mutation in mutations {
        for hvo in &mutation.set_headers {
            if let Some(hv) = &hvo.header {
                if hv.key.starts_with(':') {
                    continue;
                }
                if let (Ok(name), Ok(value)) = (
                    hyper::header::HeaderName::from_bytes(hv.key.as_bytes()),
                    hyper::header::HeaderValue::from_str(&hv.value),
                ) {
                    resp.headers_mut().insert(name, value);
                }
            }
        }
        for name in &mutation.remove_headers {
            if !name.starts_with(':') {
                resp.headers_mut().remove(name.as_str());
            }
        }
    }
}

// ─── ImmediateResponse Conversion ───────────────────────────────────────────

/// Convert an ext_proc `ImmediateResponse` into an axum response.
fn immediate_to_response(resp: ImmediateResponse) -> Response {
    let status = resp
        .status
        .map(|s| {
            hyper::StatusCode::from_u16(s.code as u16)
                .unwrap_or(hyper::StatusCode::INTERNAL_SERVER_ERROR)
        })
        .unwrap_or(hyper::StatusCode::INTERNAL_SERVER_ERROR);

    let mut builder = hyper::Response::builder().status(status);

    if let Some(headers) = resp.headers {
        for hvo in headers.set_headers {
            if let Some(hv) = hvo.header {
                if let (Ok(name), Ok(value)) = (
                    hyper::header::HeaderName::from_bytes(hv.key.as_bytes()),
                    hyper::header::HeaderValue::from_str(&hv.value),
                ) {
                    builder = builder.header(name, value);
                }
            }
        }
    }

    builder
        .body(axum::body::Body::from(resp.body))
        .unwrap_or_else(|_| {
            hyper::Response::builder()
                .status(500)
                .body(axum::body::Body::from("internal error"))
                .unwrap()
        })
        .into_response()
}

// ─── Proxy Handler ──────────────────────────────────────────────────────────

/// The axum handler that runs the full governed proxy flow.
pub async fn proxy_handler(
    State(state): State<Arc<ProxyState>>,
    req: Request,
) -> Response {
    // ── Phase 1: Request headers through pipeline ──

    let proc_req = request_to_processing_request(&req);
    let msg = HttpMessage::from(&proc_req);
    let outcome = state.sequence.process_request_headers(&msg).await;

    let request_mutations = match outcome {
        SequenceOutcome::Continue(mutations) => mutations,
        SequenceOutcome::Respond(resp) => return immediate_to_response(resp),
        SequenceOutcome::Error(err) => {
            tracing::error!(error = %err, "pipeline error on request headers");
            return (hyper::StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
        }
    };

    // ── Phase 2: Forward to upstream ──

    let (parts, body) = req.into_parts();

    // Build the forwarded URI.
    let upstream_uri = {
        let path_and_query = parts
            .uri
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/");
        format!("{}{}", state.upstream, path_and_query)
            .parse::<Uri>()
            .unwrap_or_else(|_| state.upstream.clone())
    };

    let mut forwarded = hyper::Request::builder()
        .method(parts.method)
        .uri(upstream_uri)
        .body(body)
        .unwrap();

    // Copy original headers.
    *forwarded.headers_mut() = parts.headers;

    // Apply request mutations from the pipeline.
    apply_request_mutations(&mut forwarded, &request_mutations);

    let upstream_resp = match state.client.request(forwarded).await {
        Ok(resp) => resp,
        Err(err) => {
            tracing::error!(error = %err, "upstream request failed");
            return (hyper::StatusCode::BAD_GATEWAY, "upstream unreachable").into_response();
        }
    };

    // ── Phase 3: Response headers through pipeline ──

    let resp_proc_req = response_to_processing_request(&upstream_resp);
    let resp_msg = HttpMessage::from(&resp_proc_req);
    let resp_outcome = state.sequence.process_response_headers(&resp_msg).await;

    let response_mutations = match resp_outcome {
        SequenceOutcome::Continue(mutations) => mutations,
        SequenceOutcome::Respond(resp) => return immediate_to_response(resp),
        SequenceOutcome::Error(err) => {
            tracing::error!(error = %err, "pipeline error on response headers");
            return (hyper::StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
        }
    };

    // ── Phase 4: Build client response ──

    let (resp_parts, resp_body) = upstream_resp.into_parts();
    let body = axum::body::Body::new(resp_body);
    let mut response = hyper::Response::from_parts(resp_parts, body);

    apply_response_mutations(&mut response, &response_mutations);

    response.into_response()
}

// ─── Server Bootstrap ───────────────────────────────────────────────────────

/// Build the processor pipeline from config and registry.
pub fn build_sequence(
    registry: &ProcessorRegistry,
    config: &ProxyConfig,
) -> Result<Sequence, crate::processor::ProcessorError> {
    let pipeline = registry.create_pipeline(&config.pipeline)?;
    Ok(Sequence::builder()
        .processors(pipeline)
        .failure_mode(config.failure_mode.into())
        .build())
}

/// Start the governed reverse proxy.
///
/// Builds the processor pipeline, creates the hyper client, and starts
/// the axum server. Blocks until the server shuts down.
pub async fn serve(
    config: ProxyConfig,
    registry: &ProcessorRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    let sequence = build_sequence(registry, &config)?;

    let upstream: Uri = config
        .upstream
        .parse()
        .map_err(|e| format!("invalid upstream URL '{}': {e}", config.upstream))?;

    let client = Client::builder(TokioExecutor::new()).build_http();

    let state = Arc::new(ProxyState {
        sequence,
        upstream,
        client,
    });

    tracing::info!(
        listen = %config.listen,
        upstream = %config.upstream,
        processors = state.sequence.len(),
        "starting governed proxy"
    );

    let app = Router::new().fallback(proxy_handler).with_state(state);

    let listener = tokio::net::TcpListener::bind(config.listen).await?;
    ::axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use envoy_grpc_ext_proc::envoy::service::ext_proc::v3::processing_request::Request as ExtRequest;

    fn make_get_request() -> Request {
        Request::builder()
            .method("GET")
            .uri("http://localhost:8080/api/v1/users?page=1")
            .header("host", "localhost:8080")
            .header("accept", "application/json")
            .header("x-geist-agent-id", "claude-main")
            .body(axum::body::Body::empty())
            .unwrap()
    }

    fn make_post_request() -> Request {
        Request::builder()
            .method("POST")
            .uri("http://localhost:8080/api/v1/users")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(r#"{"name":"test"}"#))
            .unwrap()
    }

    #[test]
    fn translates_get_request_headers() {
        let req = make_get_request();
        let proc_req = request_to_processing_request(&req);

        match &proc_req.request {
            Some(ExtRequest::RequestHeaders(hh)) => {
                assert!(hh.end_of_stream, "GET should have end_of_stream=true");
                let headers = hh.headers.as_ref().unwrap();

                // Pseudo-headers present.
                let method = headers
                    .headers
                    .iter()
                    .find(|h| h.key == ":method")
                    .unwrap();
                assert_eq!(method.value, "GET");

                let path = headers
                    .headers
                    .iter()
                    .find(|h| h.key == ":path")
                    .unwrap();
                assert_eq!(path.value, "/api/v1/users?page=1");

                let authority = headers
                    .headers
                    .iter()
                    .find(|h| h.key == ":authority")
                    .unwrap();
                assert_eq!(authority.value, "localhost:8080");

                // Regular headers present.
                let accept = headers
                    .headers
                    .iter()
                    .find(|h| h.key == "accept")
                    .unwrap();
                assert_eq!(accept.value, "application/json");
            }
            _ => panic!("expected RequestHeaders"),
        }
    }

    #[test]
    fn translates_post_request_with_body() {
        let req = make_post_request();
        let proc_req = request_to_processing_request(&req);

        match &proc_req.request {
            Some(ExtRequest::RequestHeaders(hh)) => {
                assert!(
                    !hh.end_of_stream,
                    "POST should have end_of_stream=false"
                );
            }
            _ => panic!("expected RequestHeaders"),
        }
    }

    #[test]
    fn applies_request_mutations() {
        let mut req = hyper::Request::builder()
            .uri("http://upstream/test")
            .header("x-original", "yes")
            .body(axum::body::Body::empty())
            .unwrap();

        let mutations = vec![HeaderMutation {
            set_headers: vec![envoy_grpc_ext_proc::envoy::config::core::v3::HeaderValueOption {
                header: Some(envoy_grpc_ext_proc::envoy::config::core::v3::HeaderValue {
                    key: "x-added".into(),
                    value: "by-pipeline".into(),
                    raw_value: vec![],
                }),
                ..Default::default()
            }],
            remove_headers: vec!["x-original".into()],
        }];

        apply_request_mutations(&mut req, &mutations);

        assert_eq!(
            req.headers().get("x-added").unwrap().to_str().unwrap(),
            "by-pipeline"
        );
        assert!(req.headers().get("x-original").is_none());
    }

    #[test]
    fn immediate_response_converts() {
        use envoy_grpc_ext_proc::envoy::r#type::v3::HttpStatus;

        let immediate = ImmediateResponse {
            status: Some(HttpStatus { code: 403 }),
            body: b"access denied".to_vec(),
            ..Default::default()
        };

        let resp = immediate_to_response(immediate);
        assert_eq!(resp.status(), 403);
    }

    #[test]
    fn skips_pseudo_header_mutations() {
        let mut req = hyper::Request::builder()
            .uri("http://upstream/test")
            .body(axum::body::Body::empty())
            .unwrap();

        let mutations = vec![HeaderMutation {
            set_headers: vec![envoy_grpc_ext_proc::envoy::config::core::v3::HeaderValueOption {
                header: Some(envoy_grpc_ext_proc::envoy::config::core::v3::HeaderValue {
                    key: ":method".into(),
                    value: "DELETE".into(),
                    raw_value: vec![],
                }),
                ..Default::default()
            }],
            remove_headers: vec![":path".into()],
        }];

        apply_request_mutations(&mut req, &mutations);

        // Method should NOT change via header mutation.
        assert_eq!(req.method(), hyper::Method::GET);
    }

    #[test]
    fn host_fallback_to_authority() {
        let req = Request::builder()
            .method("GET")
            .uri("/relative-path")
            .header("host", "example.com")
            .body(axum::body::Body::empty())
            .unwrap();

        let proc_req = request_to_processing_request(&req);

        match &proc_req.request {
            Some(ExtRequest::RequestHeaders(hh)) => {
                let headers = hh.headers.as_ref().unwrap();
                let authority = headers
                    .headers
                    .iter()
                    .find(|h| h.key == ":authority")
                    .unwrap();
                assert_eq!(authority.value, "example.com");
            }
            _ => panic!("expected RequestHeaders"),
        }
    }
}
