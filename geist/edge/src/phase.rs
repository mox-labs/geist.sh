//! Phase result and processing mode types.
//!
//! These are thin ergonomic wrappers over ext_proc proto types.
//!
//! # Why PhaseResult (not raw ProcessingResponse)
//!
//! `ProcessingResponse::default()` is invalid — the `response` oneof is required
//! and must match the request phase (Dijkstra I1). `PhaseResult` lets processors
//! express intent without knowing which phase they're in. The compositor/adapter
//! translates `PhaseResult` into the correct phase-specific `ProcessingResponse`.

use envoy_grpc_ext_proc::envoy::service::ext_proc::v3::{
    HeaderMutation, ImmediateResponse,
};

/// Result of a processor handling one phase of the HTTP lifecycle.
///
/// The compositor translates this into the correct `ProcessingResponse` variant
/// based on which phase produced it.
#[derive(Debug, Clone)]
pub enum PhaseResult {
    /// No mutation. Continue to the next processor in the pipeline.
    Continue,

    /// Apply header mutations, then continue to the next processor.
    ///
    /// Mutations accumulate across processors in the pipeline and are
    /// returned to the adapter for application. Processors see the
    /// original message — not prior mutations.
    Mutate(HeaderMutation),

    /// Short-circuit: send this response directly to the client.
    ///
    /// Terminates all subsequent processors in the current phase AND
    /// all subsequent phases (Dijkstra I2). No upstream forwarding,
    /// no response phases.
    Respond(ImmediateResponse),
}

impl PhaseResult {
    /// Returns `true` if this result terminates the pipeline.
    pub fn is_terminal(&self) -> bool {
        matches!(self, PhaseResult::Respond(_))
    }
}

/// Declares which phases a processor participates in.
///
/// Request and response headers are always processed — only body phases
/// are configurable. This is a simplified view of the ext_proc
/// `ProcessingMode` — we only expose what matters for in-process
/// pipelines (headers + buffered body). Streaming and trailer modes
/// are not relevant for the in-process case.
///
/// The pipeline computes the aggregate mode as the union (most-permissive)
/// of all processor modes: if any processor opts into body processing,
/// the adapter buffers and delivers body phases to all processors.
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

        let mutation = HeaderMutation::default();
        assert!(!PhaseResult::Mutate(mutation).is_terminal());

        let response = ImmediateResponse::default();
        assert!(PhaseResult::Respond(response).is_terminal());
    }
}
