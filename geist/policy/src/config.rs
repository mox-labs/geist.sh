//! User-facing configuration types for agent operation matching.
//!
//! These types provide a declarative API for specifying match rules.
//! The [compiler](crate::compiler) translates them into runtime `Matcher` trees.

/// How to match a string value.
///
/// Re-export of [`rumi::StringMatchSpec`] — domain-agnostic string matching
/// that compiles to runtime matchers (Exact, Prefix, Suffix, Contains, Regex).
pub use rumi::StringMatchSpec as StringMatch;

/// User-friendly configuration for matching agent operations.
///
/// All fields are optional. Omitted fields match anything.
/// All present fields are `ANDed` (every condition must match).
///
/// # Example
///
/// ```
/// use geist_policy::prelude::*;
///
/// let rule = AgentOpMatch {
///     agent_id: Some(StringMatch::Exact("claude-main".into())),
///     tool_name: Some(StringMatch::Prefix("file.".into())),
///     resource: Some(StringMatch::Prefix("/home/user/project/".into())),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentOpMatch {
    /// Match on agent identity.
    pub agent_id: Option<StringMatch>,
    /// Match on tool/command name.
    pub tool_name: Option<StringMatch>,
    /// Match on target resource path.
    pub resource: Option<StringMatch>,
    /// Match on operation verb.
    pub operation: Option<StringMatch>,
    /// Match on session ID.
    pub session_id: Option<StringMatch>,
    /// Match on metadata key-value pairs (all `ANDed`).
    pub metadata: Option<Vec<MetadataMatch>>,
}

/// Match a specific metadata key-value pair.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataMatch {
    /// The metadata key to extract.
    pub key: String,
    /// How to match the metadata value.
    pub value: StringMatch,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_op_match_default_is_empty() {
        let m = AgentOpMatch::default();
        assert!(m.agent_id.is_none());
        assert!(m.tool_name.is_none());
        assert!(m.resource.is_none());
        assert!(m.operation.is_none());
        assert!(m.session_id.is_none());
        assert!(m.metadata.is_none());
    }

    #[test]
    fn agent_op_match_roundtrips_json() {
        let m = AgentOpMatch {
            agent_id: Some(StringMatch::Exact("claude-main".into())),
            tool_name: Some(StringMatch::Prefix("file.".into())),
            resource: Some(StringMatch::Prefix("/home/".into())),
            operation: Some(StringMatch::Exact("read".into())),
            ..Default::default()
        };
        let json = serde_json::to_string(&m).unwrap();
        let m2: AgentOpMatch = serde_json::from_str(&json).unwrap();
        assert!(m2.agent_id.is_some());
        assert!(m2.tool_name.is_some());
        assert!(m2.resource.is_some());
        assert!(m2.operation.is_some());
        assert!(m2.session_id.is_none());
        assert!(m2.metadata.is_none());
    }

    #[test]
    fn metadata_match_roundtrips_json() {
        let m = MetadataMatch {
            key: "command".into(),
            value: StringMatch::Contains("rm -rf".into()),
        };
        let json = serde_json::to_string(&m).unwrap();
        let m2: MetadataMatch = serde_json::from_str(&json).unwrap();
        assert_eq!(m2.key, "command");
    }
}
