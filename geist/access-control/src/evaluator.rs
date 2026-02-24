//! Policy evaluator — compiles deny/allow rules to rumi matchers.

use rumi::{InputMatcher, MatchingData, StringMatchSpec};

use crate::config::{AccessControlPolicy, AgentOpMatch};

/// Compiled policy evaluator. Immutable after construction.
///
/// Compiles deny/allow rules into rumi matchers at construction time.
/// Evaluation is O(rules * fields) with compiled matchers — no regex
/// compilation per request.
pub struct PolicyEvaluator {
    deny_rules: Vec<CompiledDenyRule>,
    allow_rules: Vec<CompiledAllowRule>,
}

struct CompiledDenyRule {
    reason: String,
    matchers: Vec<CompiledMatch>,
}

struct CompiledAllowRule {
    matchers: Vec<CompiledMatch>,
}

/// A compiled match — each field is an optional rumi InputMatcher.
struct CompiledMatch {
    agent_id: Option<Box<dyn InputMatcher>>,
    tool_name: Option<Box<dyn InputMatcher>>,
    resource: Option<Box<dyn InputMatcher>>,
    operation: Option<Box<dyn InputMatcher>>,
    session_id: Option<Box<dyn InputMatcher>>,
}

/// Result of policy evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    Deny { reason: String },
}

impl PolicyDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, PolicyDecision::Allow)
    }
}

/// Agent operation context extracted from request headers.
pub struct AgentOp<'a> {
    pub agent_id: &'a str,
    pub tool_name: &'a str,
    pub resource: &'a str,
    pub operation: &'a str,
    pub session_id: &'a str,
}

impl PolicyEvaluator {
    /// Compile an access control policy into matchers.
    pub fn compile(policy: AccessControlPolicy) -> Result<Self, rumi::MatcherError> {
        let deny_rules = policy
            .deny
            .into_iter()
            .map(|rule| {
                let matchers = rule
                    .matches
                    .into_iter()
                    .map(compile_match)
                    .collect::<Result<_, _>>()?;
                Ok(CompiledDenyRule {
                    reason: rule.reason,
                    matchers,
                })
            })
            .collect::<Result<_, rumi::MatcherError>>()?;

        let allow_rules = policy
            .allow
            .into_iter()
            .map(|rule| {
                let matchers = rule
                    .matches
                    .into_iter()
                    .map(compile_match)
                    .collect::<Result<_, _>>()?;
                Ok(CompiledAllowRule { matchers })
            })
            .collect::<Result<_, rumi::MatcherError>>()?;

        Ok(Self {
            deny_rules,
            allow_rules,
        })
    }

    /// Evaluate a policy decision for an agent operation.
    ///
    /// Deny-first: deny rules → allow rules → default deny.
    pub fn evaluate(&self, op: &AgentOp<'_>) -> PolicyDecision {
        // Phase 1: check deny rules
        for rule in &self.deny_rules {
            if rule.matchers.iter().any(|m| matches_op(m, op)) {
                return PolicyDecision::Deny {
                    reason: rule.reason.clone(),
                };
            }
        }

        // Phase 2: check allow rules
        for rule in &self.allow_rules {
            if rule.matchers.iter().any(|m| matches_op(m, op)) {
                return PolicyDecision::Allow;
            }
        }

        // Phase 3: default deny
        PolicyDecision::Deny {
            reason: "no matching allow rule".into(),
        }
    }
}

fn compile_spec(spec: StringMatchSpec) -> Result<Box<dyn InputMatcher>, rumi::MatcherError> {
    spec.to_input_matcher()
}

fn compile_match(m: AgentOpMatch) -> Result<CompiledMatch, rumi::MatcherError> {
    Ok(CompiledMatch {
        agent_id: m.agent_id.map(compile_spec).transpose()?,
        tool_name: m.tool_name.map(compile_spec).transpose()?,
        resource: m.resource.map(compile_spec).transpose()?,
        operation: m.operation.map(compile_spec).transpose()?,
        session_id: m.session_id.map(compile_spec).transpose()?,
    })
}

fn matches_op(m: &CompiledMatch, op: &AgentOp<'_>) -> bool {
    matches_field(&m.agent_id, op.agent_id)
        && matches_field(&m.tool_name, op.tool_name)
        && matches_field(&m.resource, op.resource)
        && matches_field(&m.operation, op.operation)
        && matches_field(&m.session_id, op.session_id)
}

fn matches_field(matcher: &Option<Box<dyn InputMatcher>>, value: &str) -> bool {
    match matcher {
        None => true, // absent field matches anything
        Some(m) => m.matches(&MatchingData::String(value.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AllowRule, DenyRule};

    fn simple_policy() -> AccessControlPolicy {
        AccessControlPolicy {
            deny: vec![DenyRule {
                reason: "No Bash".into(),
                matches: vec![AgentOpMatch {
                    tool_name: Some(StringMatchSpec::Exact("Bash".into())),
                    ..Default::default()
                }],
            }],
            allow: vec![AllowRule {
                matches: vec![AgentOpMatch {
                    agent_id: Some(StringMatchSpec::Exact("claude-main".into())),
                    ..Default::default()
                }],
            }],
        }
    }

    fn op<'a>(agent_id: &'a str, tool_name: &'a str, resource: &'a str) -> AgentOp<'a> {
        AgentOp {
            agent_id,
            tool_name,
            resource,
            operation: "",
            session_id: "",
        }
    }

    #[test]
    fn allow_authorized_agent() {
        let eval = PolicyEvaluator::compile(simple_policy()).unwrap();
        let decision = eval.evaluate(&op("claude-main", "Read", "/src/lib.rs"));
        assert_eq!(decision, PolicyDecision::Allow);
    }

    #[test]
    fn deny_blocked_tool() {
        let eval = PolicyEvaluator::compile(simple_policy()).unwrap();
        let decision = eval.evaluate(&op("claude-main", "Bash", "/"));
        assert!(matches!(decision, PolicyDecision::Deny { reason } if reason == "No Bash"));
    }

    #[test]
    fn deny_takes_precedence() {
        let eval = PolicyEvaluator::compile(simple_policy()).unwrap();
        let decision = eval.evaluate(&op("claude-main", "Bash", "/"));
        assert!(matches!(decision, PolicyDecision::Deny { .. }));
    }

    #[test]
    fn default_deny_unknown_agent() {
        let eval = PolicyEvaluator::compile(simple_policy()).unwrap();
        let decision = eval.evaluate(&op("rogue-agent", "Read", "/src/lib.rs"));
        assert!(matches!(decision, PolicyDecision::Deny { reason } if reason == "no matching allow rule"));
    }

    #[test]
    fn empty_policy_denies_everything() {
        let eval = PolicyEvaluator::compile(AccessControlPolicy::default()).unwrap();
        let decision = eval.evaluate(&op("anyone", "anything", "/"));
        assert!(matches!(decision, PolicyDecision::Deny { .. }));
    }

    #[test]
    fn multi_field_and_semantics() {
        let policy = AccessControlPolicy {
            deny: vec![],
            allow: vec![AllowRule {
                matches: vec![AgentOpMatch {
                    agent_id: Some(StringMatchSpec::Exact("claude".into())),
                    tool_name: Some(StringMatchSpec::Exact("Read".into())),
                    ..Default::default()
                }],
            }],
        };
        let eval = PolicyEvaluator::compile(policy).unwrap();

        assert_eq!(eval.evaluate(&op("claude", "Read", "/")), PolicyDecision::Allow);
        assert!(matches!(eval.evaluate(&op("claude", "Write", "/")), PolicyDecision::Deny { .. }));
        assert!(matches!(eval.evaluate(&op("other", "Read", "/")), PolicyDecision::Deny { .. }));
    }

    #[test]
    fn prefix_matching() {
        let policy = AccessControlPolicy {
            deny: vec![DenyRule {
                reason: "No system files".into(),
                matches: vec![AgentOpMatch {
                    resource: Some(StringMatchSpec::Prefix("/etc/".into())),
                    ..Default::default()
                }],
            }],
            allow: vec![AllowRule {
                matches: vec![AgentOpMatch::default()],
            }],
        };
        let eval = PolicyEvaluator::compile(policy).unwrap();

        assert!(matches!(eval.evaluate(&op("a", "b", "/etc/passwd")), PolicyDecision::Deny { .. }));
        assert_eq!(eval.evaluate(&op("a", "b", "/src/lib.rs")), PolicyDecision::Allow);
    }
}
