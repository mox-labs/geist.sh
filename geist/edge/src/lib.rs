//! geist-edge: Composable data plane runtime for geist.sh.
//!
//! Processor pipeline with phase-native processing, using `http::` types
//! as protocol vocabulary for zero-copy adapter integration.
//!
//! # Architecture
//!
//! - **Processors** enforce policies (access control, rate limiting, auth, etc.)
//! - **Compositors** orchestrate processors (Sequence for linear pipelines)
//! - **Adapters** bridge runtime natives ↔ processors (axum, pingora)
//!
//! # Writing an Extension
//!
//! 1. Implement [`Processor`](processor::Processor) — `name()` + phase methods
//!    (wrap async blocks with `Box::pin(async move { Ok(PhaseResult::Continue) })`)
//! 2. Implement [`IntoProcessor`](registry::IntoProcessor) — `Config` type + `from_config()`
//! 3. Call [`register_processor!`] — one line, handles deserialization + registration
//! 4. Add a [`TypedConfig`](registry::TypedConfig) entry in pipeline JSON config
//!
//! See the [`registry`] module for the full pattern and examples.
//!
//! # Policy-Processor Model
//!
//! Policies define WHAT (user config). Processors define HOW (enforcement).
//! There is no standalone "PolicyProcessor" — every processor enforces some policy type.

pub mod compositor;
pub mod phase;
pub mod processor;
pub mod processors;
pub mod registry;

#[cfg(feature = "adapter-axum")]
pub mod adapter;

/// Prelude for convenient imports.
pub mod prelude {
    pub use crate::compositor::{FailureMode, Sequence, SequenceBuilder, SequenceOutcome};
    pub use crate::phase::{HeaderMutations, ImmediateResponse, PhaseResult, ProcessingMode};
    pub use crate::processor::{BoxFuture, Processor, ProcessorError};
    pub use crate::registry::{
        collect_processor_extensions, IntoProcessor, ProcessorRegistration, ProcessorRegistry,
        TypedConfig, TypedRegistry, TypedRegistryBuilder,
    };
}
