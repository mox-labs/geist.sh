//! Access logging processor extension for geist-edge.
//!
//! Writes structured access log entries per request. Headers-only — never
//! touches the body.
//!
//! # Extension Registration
//!
//! Self-registers via [`geist_edge::register_processor!`] with type URL
//! `mox.geist.processors.v1.AccessLog`. No code changes needed in
//! geist-edge or the binary — just add this crate as a dependency.
//!
//! # Example Config
//!
//! ```json
//! {
//!   "type_url": "mox.geist.processors.v1.AccessLog",
//!   "config": { "path": "/var/log/geist/access.log" }
//! }
//! ```

mod config;
mod processor;

pub use config::AccessLogConfig;
pub use processor::AccessLogProcessor;
