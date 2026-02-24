//! Access control policy configuration types.

use rumi::StringMatchSpec;
use serde::Deserialize;

/// Access control policy: deny rules + allow rules.
///
/// Evaluation order (deny-first):
/// 1. If any deny rule matches → Deny
/// 2. If any allow rule matches → Allow
/// 3. Default → Deny (fail-closed)
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AccessControlPolicy {
    /// Deny rules — checked first. If any match, the request is denied.
    #[serde(default)]
    pub deny: Vec<DenyRule>,
    /// Allow rules — checked second. If any match, the request is allowed.
    #[serde(default)]
    pub allow: Vec<AllowRule>,
}

/// A deny rule. If any match in `matches` hits, the request is denied
/// with the given reason.
#[derive(Debug, Clone, Deserialize)]
pub struct DenyRule {
    /// Human-readable denial reason (included in response body).
    pub reason: String,
    /// Match conditions (OR semantics — any match triggers the rule).
    pub matches: Vec<AgentOpMatch>,
}

/// An allow rule. If any match in `matches` hits, the request is allowed.
#[derive(Debug, Clone, Deserialize)]
pub struct AllowRule {
    /// Match conditions (OR semantics — any match triggers the rule).
    pub matches: Vec<AgentOpMatch>,
}

/// Match condition for an agent operation.
///
/// All present fields are ANDed: if both `agent_id` and `tool_name` are
/// set, both must match. Absent fields match anything.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AgentOpMatch {
    #[serde(default)]
    pub agent_id: Option<StringMatchSpec>,
    #[serde(default)]
    pub tool_name: Option<StringMatchSpec>,
    #[serde(default)]
    pub resource: Option<StringMatchSpec>,
    #[serde(default)]
    pub operation: Option<StringMatchSpec>,
    #[serde(default)]
    pub session_id: Option<StringMatchSpec>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_deserializes_from_json() {
        let json = r#"{
            "deny": [{"reason": "No Bash", "matches": [{"tool_name": {"Exact": "Bash"}}]}],
            "allow": [{"matches": [{"agent_id": {"Exact": "claude-main"}}]}]
        }"#;
        let policy: AccessControlPolicy = serde_json::from_str(json).unwrap();
        assert_eq!(policy.deny.len(), 1);
        assert_eq!(policy.allow.len(), 1);
        assert_eq!(policy.deny[0].reason, "No Bash");
    }

    #[test]
    fn empty_policy_deserializes() {
        let json = "{}";
        let policy: AccessControlPolicy = serde_json::from_str(json).unwrap();
        assert!(policy.deny.is_empty());
        assert!(policy.allow.is_empty());
    }

    #[test]
    fn agent_op_match_all_fields() {
        let json = r#"{
            "agent_id": {"Exact": "claude"},
            "tool_name": {"Prefix": "File"},
            "resource": {"Contains": "/src/"},
            "operation": {"Exact": "read"},
            "session_id": {"Exact": "sess-123"}
        }"#;
        let m: AgentOpMatch = serde_json::from_str(json).unwrap();
        assert!(m.agent_id.is_some());
        assert!(m.tool_name.is_some());
        assert!(m.resource.is_some());
        assert!(m.operation.is_some());
        assert!(m.session_id.is_some());
    }
}
