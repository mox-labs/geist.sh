//! geist-edge: Composable data plane runtime for geist.sh.
//!
//! Processor pipeline with phase-native processing, built on the ext_proc
//! processing model (ProcessingRequest/ProcessingResponse as universal contract).
//!
//! # Architecture
//!
//! - **Processors** enforce policies (access control, rate limiting, auth, etc.)
//! - **Compositors** orchestrate processors (Sequence for linear pipelines)
//! - **Adapters** bridge runtime natives ↔ ext_proc types (axum, pingora)
//!
//! # Policy-Processor Model
//!
//! Policies define WHAT (user config). Processors define HOW (enforcement).
//! There is no standalone "PolicyProcessor" — every processor enforces some policy type.

pub mod compositor;
pub mod phase;
pub mod processor;

/// Prelude for convenient imports.
pub mod prelude {
    pub use crate::compositor::{FailureMode, Sequence, SequenceBuilder};
    pub use crate::phase::{PhaseResult, ProcessingMode};
    pub use crate::processor::{BoxFuture, Processor, ProcessorError};
    pub use rumi_http::HttpMessage;

    // Re-export ext_proc types that processor implementors need.
    // This lets downstream crates (e.g. geist bin) avoid a direct
    // envoy-grpc-ext-proc dependency.
    pub use envoy_grpc_ext_proc::envoy::{
        config::core::v3::{HeaderMap, HeaderValue, HeaderValueOption},
        r#type::v3::HttpStatus,
        service::ext_proc::v3::{
            processing_request::Request, HeaderMutation, HttpHeaders, ImmediateResponse,
            ProcessingRequest,
        },
    };
}
