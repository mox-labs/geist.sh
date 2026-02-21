//! act-pdp: Policy Decision Point for autonomous agent operations.
//!
//! Provides context types, `DataInput` extractors, a domain compiler,
//! and a deny-first policy evaluator for autonomous agent operations.
//!
//! # Quick Start
//!
//! ```
//! use act_pdp::prelude::*;
//!
//! // Load policy from JSON
//! let json = r#"{
//!     "deny": [{
//!         "reason": "No Bash",
//!         "matches": [{ "tool_name": { "Exact": "Bash" } }]
//!     }],
//!     "allow": [{
//!         "matches": [{ "agent_id": { "Exact": "claude-main" } }]
//!     }]
//! }"#;
//!
//! let evaluator = PolicyEvaluator::from_json(json).unwrap();
//!
//! // Evaluate an operation
//! let op = AgentOp::new("claude-main", "Read", "/src/lib.rs");
//! assert!(evaluator.evaluate(&op).is_allowed());
//!
//! let op = AgentOp::new("claude-main", "Bash", "/");
//! assert!(evaluator.evaluate(&op).is_denied());
//! ```
//!
//! # Domain Compiler
//!
//! For programmatic rule construction:
//!
//! ```
//! use act_pdp::prelude::*;
//!
//! let rule = AgentOpMatch {
//!     agent_id: Some(StringMatch::Exact("claude-main".into())),
//!     tool_name: Some(StringMatch::Prefix("file.".into())),
//!     resource: Some(StringMatch::Prefix("/home/user/project/".into())),
//!     ..Default::default()
//! };
//! let matcher = rule.compile("allowed").unwrap();
//!
//! let op = AgentOp::new("claude-main", "file.read", "/home/user/project/src/lib.rs");
//! assert_eq!(matcher.evaluate(&op), Some("allowed"));
//! ```

mod compiler;
mod config;
mod context;
mod decision;
mod error;
mod evaluator;
mod inputs;

pub use compiler::*;
pub use config::*;
pub use context::*;
pub use decision::*;
pub use error::*;
pub use evaluator::*;
pub use inputs::*;

// Registry config types (feature-gated)
#[cfg(feature = "registry")]
pub use inputs::MetadataInputConfig;

/// Register all act-pdp types for [`AgentOp`] with the given builder.
///
/// Registers core matchers and ACT domain inputs:
/// - `act.v1.AgentIdInput` -> [`AgentIdInput`]
/// - `act.v1.ToolNameInput` -> [`ToolNameInput`]
/// - `act.v1.ResourceInput` -> [`ResourceInput`]
/// - `act.v1.OperationInput` -> [`OperationInput`]
/// - `act.v1.SessionIdInput` -> [`SessionIdInput`]
/// - `act.v1.MetadataInput` -> [`MetadataInput`]
#[cfg(feature = "registry")]
#[must_use]
pub fn register(
    builder: rumi::RegistryBuilder<AgentOp>,
) -> rumi::RegistryBuilder<AgentOp> {
    rumi::register_core_matchers(builder)
        .input::<AgentIdInput>("act.v1.AgentIdInput")
        .input::<ToolNameInput>("act.v1.ToolNameInput")
        .input::<ResourceInput>("act.v1.ResourceInput")
        .input::<OperationInput>("act.v1.OperationInput")
        .input::<SessionIdInput>("act.v1.SessionIdInput")
        .input::<MetadataInput>("act.v1.MetadataInput")
}

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::{
        compile_agent_op_matches, AgentIdInput, AgentOp, AgentOpMatch, AgentOpMatchExt,
        AllowRule, DenyAllowPolicy, DenyRule, GatewayError, MetadataInput, MetadataMatch,
        OperationInput, PolicyDecision, PolicyError, PolicyEvaluator, ResourceInput, SessionIdInput,
        StringMatch, ToolNameInput,
    };
    pub use rumi::prelude::*;
}

#[cfg(test)]
mod tests {
    use super::*;
    use rumi::prelude::*;

    #[test]
    fn test_agent_op_builder() {
        let op = AgentOp::new("claude-main", "file.read", "/src/lib.rs")
            .with_operation("read")
            .with_session_id("sess-123")
            .with_meta("encoding", "utf-8");

        assert_eq!(op.agent_id(), "claude-main");
        assert_eq!(op.tool_name(), "file.read");
        assert_eq!(op.resource(), "/src/lib.rs");
        assert_eq!(op.operation(), "read");
        assert_eq!(op.session_id(), "sess-123");
        assert_eq!(op.meta("encoding"), Some("utf-8"));
    }

    #[test]
    fn test_data_input_extraction() {
        let op = AgentOp::new("agent-1", "Bash", "/tmp")
            .with_meta("command", "ls -la");

        assert_eq!(
            AgentIdInput.get(&op),
            MatchingData::String("agent-1".into())
        );
        assert_eq!(
            ToolNameInput.get(&op),
            MatchingData::String("Bash".into())
        );
        assert_eq!(
            MetadataInput::new("command").get(&op),
            MatchingData::String("ls -la".into())
        );
    }

    #[test]
    fn test_end_to_end_policy_evaluation() {
        let policy = DenyAllowPolicy {
            deny: vec![DenyRule {
                reason: "No Bash".into(),
                matches: vec![AgentOpMatch {
                    tool_name: Some(StringMatch::Exact("Bash".into())),
                    ..Default::default()
                }],
            }],
            allow: vec![AllowRule {
                matches: vec![AgentOpMatch {
                    agent_id: Some(StringMatch::Exact("claude-main".into())),
                    ..Default::default()
                }],
            }],
        };

        let evaluator = PolicyEvaluator::compile(policy).unwrap();

        // Allow: claude-main using non-Bash tool
        let op = AgentOp::new("claude-main", "Read", "/src");
        assert!(evaluator.evaluate(&op).is_allowed());

        // Deny: any agent using Bash
        let op = AgentOp::new("claude-main", "Bash", "/");
        assert!(evaluator.evaluate(&op).is_denied());

        // Default deny: unknown agent
        let op = AgentOp::new("rogue", "Read", "/src");
        assert!(evaluator.evaluate(&op).is_denied());
    }
}
