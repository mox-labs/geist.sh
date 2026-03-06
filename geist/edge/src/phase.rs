//! Phase result and processing mode types.
//!
//! These are geist-edge's own protocol vocabulary, using `http::` types
//! (StatusCode, HeaderName, HeaderValue, HeaderMap) and `bytes::Bytes`.
//!
//! # Why PhaseResult (not raw response types)
//!
//! `PhaseResult` lets processors express intent without knowing which phase
//! they're in. The compositor/adapter translates `PhaseResult` into the
//! correct response.

use bytes::Bytes;

/// Result of a processor handling one phase of the HTTP lifecycle.
///
/// The compositor translates this into the correct response variant
/// based on which phase produced it.
#[derive(Debug)]
pub enum PhaseResult {
    /// No mutation. Continue to the next processor in the pipeline.
    Continue,

    /// Apply header mutations, then continue to the next processor.
    ///
    /// Mutations accumulate across processors in the pipeline and are
    /// returned to the adapter for application. Processors see the
    /// original message — not prior mutations.
    Mutate(HeaderMutations),

    /// Short-circuit: send this response directly to the client.
    ///
    /// Terminates all subsequent processors in the current phase AND
    /// all subsequent phases. No upstream forwarding, no response phases.
    Respond(ImmediateResponse),
}

impl PhaseResult {
    /// Returns `true` if this result terminates the pipeline.
    pub fn is_terminal(&self) -> bool {
        matches!(self, PhaseResult::Respond(_))
    }
}

/// Header mutations to apply after pipeline processing.
///
/// The adapter applies these to a cloned `HeaderMap` at the end —
/// one bulk operation, not per-processor.
#[derive(Debug)]
pub struct HeaderMutations {
    /// Headers to set (add or overwrite).
    pub set: Vec<(http::HeaderName, http::HeaderValue)>,
    /// Headers to remove.
    pub remove: Vec<http::HeaderName>,
}

impl HeaderMutations {
    /// Create an empty mutation set.
    pub fn new() -> Self {
        Self {
            set: Vec::new(),
            remove: Vec::new(),
        }
    }

    /// Add a header to set.
    pub fn set_header(mut self, name: http::HeaderName, value: http::HeaderValue) -> Self {
        self.set.push((name, value));
        self
    }

    /// Add a header to remove.
    pub fn remove_header(mut self, name: http::HeaderName) -> Self {
        self.remove.push(name);
        self
    }
}

impl Default for HeaderMutations {
    fn default() -> Self {
        Self::new()
    }
}

/// Immediate response — short-circuit the pipeline and send directly.
///
/// Uses `http::` vocabulary types for zero-copy integration with adapters.
#[derive(Debug)]
pub struct ImmediateResponse {
    /// HTTP status code.
    pub status: http::StatusCode,
    /// Response headers.
    pub headers: http::HeaderMap,
    /// Response body.
    pub body: Bytes,
}

impl ImmediateResponse {
    /// Create a response with the given status code.
    pub fn with_status(status: http::StatusCode) -> Self {
        Self {
            status,
            headers: http::HeaderMap::new(),
            body: Bytes::new(),
        }
    }

    /// Set the response body.
    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = body.into();
        self
    }

    /// Add a response header.
    pub fn header(mut self, name: http::HeaderName, value: http::HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }
}

/// Declares which phases a processor participates in.
///
/// Request and response headers are always processed — only body phases
/// are configurable. The pipeline computes the aggregate mode as the
/// union (most-permissive) of all processor modes: if any processor
/// opts into body processing, the adapter buffers and delivers body
/// phases to all processors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessingMode {
    /// Process request body (buffered). Default: false.
    pub request_body: bool,
    /// Process response body (buffered). Default: false.
    pub response_body: bool,
}

impl ProcessingMode {
    /// Headers-only: process request + response headers, skip body.
    /// This is the safe default — most processors only need headers.
    pub const HEADERS_ONLY: Self = Self {
        request_body: false,
        response_body: false,
    };

    /// Full: process all phases including body.
    pub const FULL: Self = Self {
        request_body: true,
        response_body: true,
    };

    /// Compute the union (most-permissive) of two modes.
    ///
    /// Used by the pipeline builder to compute the aggregate mode
    /// across all processors.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self {
            request_body: self.request_body || other.request_body,
            response_body: self.response_body || other.response_body,
        }
    }
}

impl Default for ProcessingMode {
    fn default() -> Self {
        Self::HEADERS_ONLY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_only_is_default() {
        assert_eq!(ProcessingMode::default(), ProcessingMode::HEADERS_ONLY);
        assert!(!ProcessingMode::HEADERS_ONLY.request_body);
        assert!(!ProcessingMode::HEADERS_ONLY.response_body);
    }

    #[test]
    fn full_enables_all_phases() {
        let mode = ProcessingMode::FULL;
        assert!(mode.request_body);
        assert!(mode.response_body);
    }

    #[test]
    fn union_takes_most_permissive() {
        let a = ProcessingMode::HEADERS_ONLY;
        let b = ProcessingMode {
            request_body: true,
            ..ProcessingMode::HEADERS_ONLY
        };
        let merged = a.union(b);
        assert!(merged.request_body);
        assert!(!merged.response_body);
    }

    #[test]
    fn phase_result_terminal() {
        assert!(!PhaseResult::Continue.is_terminal());

        let mutation = HeaderMutations::new();
        assert!(!PhaseResult::Mutate(mutation).is_terminal());

        let response = ImmediateResponse::with_status(http::StatusCode::OK);
        assert!(PhaseResult::Respond(response).is_terminal());
    }

    #[test]
    fn immediate_response_builder() {
        let resp = ImmediateResponse::with_status(http::StatusCode::FORBIDDEN)
            .body("denied")
            .header(
                http::header::CONTENT_TYPE,
                http::HeaderValue::from_static("application/json"),
            );
        assert_eq!(resp.status, http::StatusCode::FORBIDDEN);
        assert_eq!(resp.body, "denied");
        assert_eq!(
            resp.headers.get(http::header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
    }

    #[test]
    fn header_mutations_builder() {
        let mutations = HeaderMutations::new()
            .set_header(
                http::header::HeaderName::from_static("x-custom"),
                http::HeaderValue::from_static("value"),
            )
            .remove_header(http::header::HeaderName::from_static("x-remove"));
        assert_eq!(mutations.set.len(), 1);
        assert_eq!(mutations.remove.len(), 1);
    }
}
