//! Processor trait and supporting types for the geist-edge pipeline.
//!
//! # Policy-Processor Model
//!
//! Policies define WHAT (user configuration). Processors define HOW (enforcement).
//! Every processor enforces some policy type:
//! - `AccessControlProcessor` enforces access control policies
//! - `RateLimiterProcessor` enforces rate limiting policies
//! - `AuthProcessor` enforces auth policies
//!
//! # Phase-Native Processing
//!
//! Processors declare which phases they participate in via [`ProcessingMode`].
//! Default is headers-only. If you implement body methods, declare it in `mode()`.
//! The pipeline validates mode consistency at registration time.

use std::future::Future;
use std::pin::Pin;

use crate::phase::{PhaseResult, ProcessingMode};

/// Boxed future for dyn-compatible async trait methods.
///
/// `async fn` in traits is not dyn-compatible, and the pipeline uses
/// `Arc<dyn Processor>`. Wrap your async block: `Box::pin(async { ... })`.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Error returned by a processor when it fails to process a phase.
///
/// The compositor decides how to handle this: fail-open (continue) or
/// fail-closed (generate error response).
#[derive(Debug)]
pub struct ProcessorError {
    /// Which processor failed.
    pub processor: String,
    /// What went wrong.
    pub message: String,
    /// Optional source error.
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl ProcessorError {
    pub fn new(processor: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            processor: processor.into(),
            message: message.into(),
            source: None,
        }
    }

    pub fn with_source(
        processor: impl Into<String>,
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            processor: processor.into(),
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

impl std::fmt::Display for ProcessorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "processor '{}': {}", self.processor, self.message)
    }
}

impl std::error::Error for ProcessorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}

/// A processor in the geist-edge pipeline.
///
/// Each processor enforces some policy type. The trait uses default methods
/// so you only override the phases you care about:
///
/// - Headers-only processors (default): override `process_request_headers`
///   and/or `process_response_headers`
/// - Body processors: override body methods AND declare it in `mode()`
///
/// All methods return `Result<PhaseResult, ProcessorError>`:
/// - `PhaseResult::Continue` — no mutation, pass to next processor
/// - `PhaseResult::Mutate(HeaderMutations)` — apply header mutations, continue
/// - `PhaseResult::Respond(ImmediateResponse)` — short-circuit, send response to client
/// - `Err(ProcessorError)` — processor failure, compositor handles (fail-open/closed)
///
/// # Send + Sync
///
/// Required for `Arc` sharing across async tasks (hyper school pattern).
pub trait Processor: Send + Sync {
    /// Human-readable name for logging and diagnostics.
    fn name(&self) -> &str;

    /// Declares which body phases this processor participates in.
    ///
    /// Default: headers-only (no body processing).
    /// Override if you implement body methods.
    fn mode(&self) -> ProcessingMode {
        ProcessingMode::HEADERS_ONLY
    }

    /// Process request headers phase.
    ///
    /// Called with request [`Parts`](http::request::Parts) — zero-copy
    /// from the adapter (axum: `req.into_parts().0`).
    fn process_request_headers(
        &self,
        _parts: &http::request::Parts,
    ) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>> {
        Box::pin(async { Ok(PhaseResult::Continue) })
    }

    /// Process request body phase.
    ///
    /// Only called if `mode()` opts into request body processing
    /// AND the request has a body.
    fn process_request_body(
        &self,
        _body: &[u8],
    ) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>> {
        Box::pin(async { Ok(PhaseResult::Continue) })
    }

    /// Process response headers phase.
    ///
    /// Called with response [`Parts`](http::response::Parts) from upstream.
    fn process_response_headers(
        &self,
        _parts: &http::response::Parts,
    ) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>> {
        Box::pin(async { Ok(PhaseResult::Continue) })
    }

    /// Process response body phase.
    ///
    /// Only called if `mode()` opts into response body processing.
    fn process_response_body(
        &self,
        _body: &[u8],
    ) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>> {
        Box::pin(async { Ok(PhaseResult::Continue) })
    }
}
