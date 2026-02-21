//! Policy decision types and deny/allow policy configuration.
//!
//! These are pure domain types — zero HTTP types. Translation to
//! HTTP 403/200 happens in the adapter (geist-edge), not here.

use crate::config::AgentOpMatch;

/// The result of evaluating an agent operation against policy.
///
/// Only two outcomes: allowed or denied with reason.
/// This is a domain type — HTTP status mapping is the adapter's concern.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PolicyDecision {
    /// The operation is permitted.
    Allow,
    /// The operation is denied.
    Deny {
        /// Human-readable reason for the denial.
        reason: String,
    },
}

impl PolicyDecision {
    /// Returns `true` if the decision is `Allow`.
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow)
    }

    /// Returns `true` if the decision is `Deny`.
    #[must_use]
    pub fn is_denied(&self) -> bool {
        matches!(self, Self::Deny { .. })
    }
}

/// A deny/allow policy configuration.
///
/// Evaluation order is non-negotiable:
/// 1. Deny rules evaluated first (absolute — if any match, operation is denied)
/// 2. Allow rules evaluated second (if any match, operation is allowed)
/// 3. Default: deny (if nothing matches)
///
/// # Example
///
/// ```
/// use geist_policy::prelude::*;
///
/// let policy = DenyAllowPolicy {
///     deny: vec![DenyRule {
///         reason: "No deleting system files".into(),
///         matches: vec![AgentOpMatch {
///             operation: Some(StringMatch::Exact("delete".into())),
///             resource: Some(StringMatch::Prefix("/etc/".into())),
///             ..Default::default()
///         }],
///     }],
///     allow: vec![AllowRule {
///         matches: vec![AgentOpMatch {
///             agent_id: Some(StringMatch::Exact("claude-main".into())),
///             ..Default::default()
///         }],
///     }],
/// };
/// ```
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DenyAllowPolicy {
    /// Deny rules — evaluated first, absolute.
    #[serde(default)]
    pub deny: Vec<DenyRule>,
    /// Allow rules — evaluated second.
    #[serde(default)]
    pub allow: Vec<AllowRule>,
}

/// A deny rule with a human-readable reason.
///
/// Multiple `matches` within a rule are `ORed` — any match triggers denial.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DenyRule {
    /// Why this operation is denied.
    pub reason: String,
    /// Match conditions (ORed — any match triggers denial).
    pub matches: Vec<AgentOpMatch>,
}

/// An allow rule.
///
/// Multiple `matches` within a rule are `ORed` — any match grants access.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllowRule {
    /// Match conditions (ORed — any match grants access).
    pub matches: Vec<AgentOpMatch>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StringMatch;

    #[test]
    fn policy_decision_is_allowed() {
        assert!(PolicyDecision::Allow.is_allowed());
        assert!(!PolicyDecision::Allow.is_denied());
    }

    #[test]
    fn policy_decision_is_denied() {
        let d = PolicyDecision::Deny {
            reason: "nope".into(),
        };
        assert!(d.is_denied());
        assert!(!d.is_allowed());
    }

    #[test]
    fn deny_allow_policy_default_is_empty() {
        let p = DenyAllowPolicy::default();
        assert!(p.deny.is_empty());
        assert!(p.allow.is_empty());
    }

    #[test]
    fn deny_allow_policy_roundtrips_json() {
        let policy = DenyAllowPolicy {
            deny: vec![DenyRule {
                reason: "No deletes".into(),
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

        let json = serde_json::to_string_pretty(&policy).unwrap();
        let policy2: DenyAllowPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(policy2.deny.len(), 1);
        assert_eq!(policy2.allow.len(), 1);
        assert_eq!(policy2.deny[0].reason, "No deletes");
    }
}
