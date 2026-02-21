//! Policy evaluator: deny-first evaluation over compiled matchers.
//!
//! The evaluator is the layer ABOVE rumi. Rumi's `Matcher` is the engine;
//! this wraps it with deny-first semantics.
//!
//! Uses `&self` (not `&mut self`) — enables `Arc<PolicyEvaluator>` for
//! concurrent sharing across request handlers.

use crate::compiler::compile_agent_op_matches;
use crate::context::AgentOp;
use crate::decision::{AllowRule, DenyAllowPolicy, DenyRule, PolicyDecision};
use crate::error::PolicyError;
use rumi::prelude::*;

/// A compiled deny rule: matcher + reason.
struct CompiledDenyRule {
    matcher: Matcher<AgentOp, ()>,
    reason: String,
}

/// A compiled allow rule: matcher only.
struct CompiledAllowRule {
    matcher: Matcher<AgentOp, ()>,
}

/// Evaluates agent operations against compiled deny/allow policy.
///
/// Evaluation order (non-negotiable):
/// 1. Deny rules — if any match, `Deny` with reason (absolute)
/// 2. Allow rules — if any match, `Allow`
/// 3. Default — `Deny` with "no matching allow rule"
///
/// # Thread Safety
///
/// `PolicyEvaluator` is `Send + Sync` and uses `&self` for evaluation,
/// enabling `Arc<PolicyEvaluator>` for concurrent request handling.
pub struct PolicyEvaluator {
    deny_rules: Vec<CompiledDenyRule>,
    allow_rules: Vec<CompiledAllowRule>,
}

impl PolicyEvaluator {
    /// Compile a deny/allow policy into an evaluator.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError`] if any regex pattern is invalid.
    pub fn compile(policy: DenyAllowPolicy) -> Result<Self, PolicyError> {
        let deny_rules = policy
            .deny
            .into_iter()
            .enumerate()
            .map(|(i, rule)| compile_deny_rule(rule, i))
            .collect::<Result<_, _>>()?;

        let allow_rules = policy
            .allow
            .into_iter()
            .enumerate()
            .map(|(i, rule)| compile_allow_rule(rule, i))
            .collect::<Result<_, _>>()?;

        Ok(Self {
            deny_rules,
            allow_rules,
        })
    }

    /// Load a policy from JSON and compile it.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError`] if JSON is invalid or patterns fail to compile.
    pub fn from_json(json: &str) -> Result<Self, PolicyError> {
        let policy: DenyAllowPolicy = serde_json::from_str(json)?;
        Self::compile(policy)
    }

    /// Evaluate an agent operation against the compiled policy.
    ///
    /// This is `&self` — safe for concurrent use behind `Arc`.
    #[must_use]
    pub fn evaluate(&self, op: &AgentOp) -> PolicyDecision {
        // Step 1: Deny rules (absolute)
        for rule in &self.deny_rules {
            if rule.matcher.evaluate(op).is_some() {
                return PolicyDecision::Deny {
                    reason: rule.reason.clone(),
                };
            }
        }

        // Step 2: Allow rules
        for rule in &self.allow_rules {
            if rule.matcher.evaluate(op).is_some() {
                return PolicyDecision::Allow;
            }
        }

        // Step 3: Default deny
        PolicyDecision::Deny {
            reason: "no matching allow rule".into(),
        }
    }
}

/// Compile a deny rule's matches into a single OR matcher.
fn compile_deny_rule(rule: DenyRule, index: usize) -> Result<CompiledDenyRule, PolicyError> {
    let matcher = compile_agent_op_matches(&rule.matches, (), None).map_err(|e| {
        PolicyError::InvalidRule {
            rule_type: "deny",
            rule_index: index,
            field: None,
            source: e,
        }
    })?;
    Ok(CompiledDenyRule {
        matcher,
        reason: rule.reason,
    })
}

/// Compile an allow rule's matches into a single OR matcher.
fn compile_allow_rule(rule: AllowRule, index: usize) -> Result<CompiledAllowRule, PolicyError> {
    let matcher = compile_agent_op_matches(&rule.matches, (), None).map_err(|e| {
        PolicyError::InvalidRule {
            rule_type: "allow",
            rule_index: index,
            field: None,
            source: e,
        }
    })?;
    Ok(CompiledAllowRule { matcher })
}

// Compile-time assertion: PolicyEvaluator must be Send + Sync.
#[allow(dead_code)]
const _: () = {
    fn assert_send_sync<T: Send + Sync>() {}
    fn check() {
        assert_send_sync::<PolicyEvaluator>();
    }
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentOpMatch, StringMatch};

    fn simple_policy() -> DenyAllowPolicy {
        DenyAllowPolicy {
            deny: vec![DenyRule {
                reason: "No deleting system files".into(),
                matches: vec![AgentOpMatch {
                    operation: Some(StringMatch::Exact("delete".into())),
                    resource: Some(StringMatch::Prefix("/etc/".into())),
                    ..Default::default()
                }],
            }],
            allow: vec![AllowRule {
                matches: vec![AgentOpMatch {
                    agent_id: Some(StringMatch::Exact("claude-main".into())),
                    tool_name: Some(StringMatch::Prefix("file.".into())),
                    resource: Some(StringMatch::Prefix("/home/user/project/".into())),
                    ..Default::default()
                }],
            }],
        }
    }

    #[test]
    fn deny_rule_blocks_system_file_deletion() {
        let eval = PolicyEvaluator::compile(simple_policy()).unwrap();

        let op = AgentOp::new("claude-main", "file.delete", "/etc/passwd")
            .with_operation("delete");
        let decision = eval.evaluate(&op);

        assert!(decision.is_denied());
        assert_eq!(
            decision,
            PolicyDecision::Deny {
                reason: "No deleting system files".into()
            }
        );
    }

    #[test]
    fn allow_rule_permits_authorized_agent() {
        let eval = PolicyEvaluator::compile(simple_policy()).unwrap();

        let op = AgentOp::new("claude-main", "file.read", "/home/user/project/src/lib.rs");
        assert_eq!(eval.evaluate(&op), PolicyDecision::Allow);
    }

    #[test]
    fn default_deny_when_no_rules_match() {
        let eval = PolicyEvaluator::compile(simple_policy()).unwrap();

        let op = AgentOp::new("unknown-agent", "unknown-tool", "/unknown/path");
        let decision = eval.evaluate(&op);

        assert!(decision.is_denied());
        assert_eq!(
            decision,
            PolicyDecision::Deny {
                reason: "no matching allow rule".into()
            }
        );
    }

    #[test]
    fn deny_takes_precedence_over_allow() {
        let policy = DenyAllowPolicy {
            deny: vec![DenyRule {
                reason: "No destructive operations".into(),
                matches: vec![AgentOpMatch {
                    operation: Some(StringMatch::Exact("delete".into())),
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
        let eval = PolicyEvaluator::compile(policy).unwrap();

        // claude-main would match allow, but "delete" matches deny
        let op = AgentOp::new("claude-main", "file.delete", "/tmp/file")
            .with_operation("delete");
        let decision = eval.evaluate(&op);

        assert!(decision.is_denied());
        assert_eq!(
            decision,
            PolicyDecision::Deny {
                reason: "No destructive operations".into()
            }
        );
    }

    #[test]
    fn empty_policy_denies_everything() {
        let eval = PolicyEvaluator::compile(DenyAllowPolicy::default()).unwrap();

        let op = AgentOp::new("any", "any", "/any");
        assert!(eval.evaluate(&op).is_denied());
    }

    #[test]
    fn from_json_compiles_policy() {
        let json = r#"{
            "deny": [{
                "reason": "No Bash",
                "matches": [{ "tool_name": { "Exact": "Bash" } }]
            }],
            "allow": [{
                "matches": [{ "agent_id": { "Exact": "claude-main" } }]
            }]
        }"#;

        let eval = PolicyEvaluator::from_json(json).unwrap();

        // Deny Bash
        let op = AgentOp::new("claude-main", "Bash", "/");
        assert!(eval.evaluate(&op).is_denied());

        // Allow non-Bash for claude-main
        let op = AgentOp::new("claude-main", "Read", "/");
        assert!(eval.evaluate(&op).is_allowed());

        // Default deny for unknown agent
        let op = AgentOp::new("rogue", "Read", "/");
        assert!(eval.evaluate(&op).is_denied());
    }

    #[test]
    fn from_json_invalid_returns_error() {
        assert!(PolicyEvaluator::from_json("not json").is_err());
    }

    #[test]
    fn from_json_invalid_regex_returns_error() {
        let json = r#"{
            "deny": [{
                "reason": "bad",
                "matches": [{ "tool_name": { "Regex": "[bad" } }]
            }]
        }"#;
        let result = PolicyEvaluator::from_json(json);
        assert!(result.is_err());
        // Verify structured error has rule context
        match result.err().unwrap() {
            PolicyError::InvalidRule {
                rule_type,
                rule_index,
                ..
            } => {
                assert_eq!(rule_type, "deny");
                assert_eq!(rule_index, 0);
            }
            other => panic!("expected InvalidRule, got {other}"),
        }
    }

    // ========== Realistic Scenario ==========

    #[test]
    fn scenario_full_policy() {
        let json = r#"{
            "deny": [
                {
                    "reason": "No agent may delete system files",
                    "matches": [
                        {
                            "operation": { "Exact": "delete" },
                            "resource": { "Prefix": "/etc/" }
                        }
                    ]
                },
                {
                    "reason": "Destructive bash commands forbidden",
                    "matches": [
                        {
                            "tool_name": { "Exact": "Bash" },
                            "metadata": [
                                { "key": "command", "value": { "Contains": "rm -rf" } }
                            ]
                        }
                    ]
                }
            ],
            "allow": [
                {
                    "matches": [
                        {
                            "agent_id": { "Exact": "claude-main" },
                            "tool_name": { "Prefix": "file." },
                            "resource": { "Prefix": "/home/user/project/" }
                        }
                    ]
                }
            ]
        }"#;

        let eval = PolicyEvaluator::from_json(json).unwrap();

        // Allowed: claude-main doing file ops in project
        let op = AgentOp::new(
            "claude-main",
            "file.read",
            "/home/user/project/src/lib.rs",
        );
        assert!(eval.evaluate(&op).is_allowed());

        // Denied: deleting system files
        let op = AgentOp::new("claude-main", "file.delete", "/etc/passwd")
            .with_operation("delete");
        assert!(eval.evaluate(&op).is_denied());

        // Denied: rm -rf via Bash
        let op = AgentOp::new("claude-main", "Bash", "/home/user/project")
            .with_meta("command", "rm -rf /important");
        assert!(eval.evaluate(&op).is_denied());

        // Denied: unknown agent
        let op = AgentOp::new("rogue-agent", "file.read", "/home/user/project/src/lib.rs");
        assert!(eval.evaluate(&op).is_denied());

        // Denied: claude-main outside project
        let op = AgentOp::new("claude-main", "file.write", "/root/.ssh/authorized_keys");
        assert!(eval.evaluate(&op).is_denied());
    }
}
