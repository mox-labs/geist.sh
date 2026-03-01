//! Runtime adapters that bridge HTTP servers into the processor pipeline.
//!
//! Each adapter translates its runtime's native types into ext_proc
//! `ProcessingRequest`/`ProcessingResponse` — the pipeline's universal
//! contract — and back. The pipeline never sees runtime-specific types.
//!
//! # Available Adapters
//!
//! - **axum** (`adapter-axum` feature): Reverse proxy powered by axum + hyper-util.

#[cfg(feature = "adapter-axum")]
pub mod axum;
