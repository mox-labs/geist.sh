//! Access control processor extension for geist-edge.
//!
//! Enforces deny-first access control policies on inbound requests.
//! Configured with [`AccessControlPolicy`] — deny rules checked first,
//! then allow rules, then default deny.
//!
//! # Extension Registration
//!
//! Self-registers via [`geist_edge::register_processor!`] with type URL
//! `mox.geist.processors.v1.AccessControl`. No code changes needed in
//! geist-edge or the binary — just add this crate as a dependency.
//!
//! # Example Config
//!
//! ```json
//! {
//!   "type_url": "mox.geist.processors.v1.AccessControl",
//!   "config": {
//!     "deny": [{ "reason": "No Bash", "matches": [{ "tool_name": { "Exact": "Bash" } }] }],
//!     "allow": [{ "matches": [{ "agent_id": { "Exact": "claude-main" } }] }]
//!   }
//! }
//! ```

mod config;
mod evaluator;
mod processor;

pub use config::{AccessControlPolicy, AgentOpMatch, AllowRule, DenyRule};
pub use evaluator::PolicyEvaluator;
pub use processor::AccessControlProcessor;
