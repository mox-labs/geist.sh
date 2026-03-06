//! Runtime adapters — bridge runtime natives to the processor pipeline.
//!
//! Each adapter is a standalone module (not a shared trait). The runtimes
//! are too different for a useful common interface — axum handler function,
//! pingora ProxyHttp lifecycle, ext_proc tonic bidi stream.
//!
//! The pipeline IS the abstraction. All adapters call the same
//! `Sequence::process_*` methods with `http::` types.

#[cfg(feature = "adapter-axum")]
pub mod axum;
