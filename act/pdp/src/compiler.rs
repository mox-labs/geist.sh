//! Compiler: `AgentOpMatch` -> rumi `Matcher<AgentOp, A>`
//!
//! Translates user-friendly match configuration into efficient
//! runtime matchers operating on [`AgentOp`].

use crate::config::{AgentOpMatch, MetadataMatch};
use crate::context::AgentOp;
use crate::inputs::{
    AgentIdInput, MetadataInput, OperationInput, ResourceInput, SessionIdInput, ToolNameInput,
};
use rumi::prelude::*;

/// A catch-all predicate that matches any `AgentOp`.
fn catch_all() -> Predicate<AgentOp> {
    Predicate::Single(SinglePredicate::new(
        Box::new(AgentIdInput),
        Box::new(PrefixMatcher::new("")), // matches any agent ID string
    ))
}

/// Extension trait for compiling `AgentOpMatch` to rumi `Matcher`.
pub trait AgentOpMatchExt {
    /// Compile this match into a rumi `Matcher`.
    ///
    /// The resulting matcher operates on [`AgentOp`] and returns
    /// the provided action when all conditions match.
    ///
    /// # Errors
    ///
    /// Returns [`MatcherError`] if any regex pattern is invalid.
    fn compile<A: Clone + Send + Sync + 'static>(
        &self,
        action: A,
    ) -> Result<Matcher<AgentOp, A>, MatcherError>;

    /// Compile this match into a `Predicate` (without action).
    ///
    /// # Errors
    ///
    /// Returns [`MatcherError`] if any regex pattern is invalid.
    fn to_predicate(&self) -> Result<Predicate<AgentOp>, MatcherError>;
}

impl AgentOpMatchExt for AgentOpMatch {
    fn compile<A: Clone + Send + Sync + 'static>(
        &self,
        action: A,
    ) -> Result<Matcher<AgentOp, A>, MatcherError> {
        let predicate = self.to_predicate()?;
        Ok(Matcher::from_predicate(predicate, action, None))
    }

    fn to_predicate(&self) -> Result<Predicate<AgentOp>, MatcherError> {
        let mut predicates: Vec<Predicate<AgentOp>> = Vec::new();

        if let Some(agent_id) = &self.agent_id {
            predicates.push(agent_id.to_predicate(Box::new(AgentIdInput))?);
        }

        if let Some(tool_name) = &self.tool_name {
            predicates.push(tool_name.to_predicate(Box::new(ToolNameInput))?);
        }

        if let Some(resource) = &self.resource {
            predicates.push(resource.to_predicate(Box::new(ResourceInput))?);
        }

        if let Some(operation) = &self.operation {
            predicates.push(operation.to_predicate(Box::new(OperationInput))?);
        }

        if let Some(session_id) = &self.session_id {
            predicates.push(session_id.to_predicate(Box::new(SessionIdInput))?);
        }

        // Metadata matches are all ANDed
        if let Some(metadata) = &self.metadata {
            for meta_match in metadata {
                predicates.push(compile_metadata_match(meta_match)?);
            }
        }

        Ok(Predicate::from_all(predicates, catch_all()))
    }
}

/// Compile a metadata match to a predicate.
fn compile_metadata_match(
    meta_match: &MetadataMatch,
) -> Result<Predicate<AgentOp>, MatcherError> {
    meta_match
        .value
        .to_predicate(Box::new(MetadataInput::new(&meta_match.key)))
}

/// Compile multiple `AgentOpMatch` entries into a single `Matcher`.
///
/// Multiple matches are `ORed` together: the first matching rule wins.
///
/// # Errors
///
/// Returns [`MatcherError`] if any regex pattern is invalid.
pub fn compile_agent_op_matches<A: Clone + Send + Sync + 'static>(
    matches: &[AgentOpMatch],
    action: A,
    on_no_match: Option<A>,
) -> Result<Matcher<AgentOp, A>, MatcherError> {
    let predicates: Vec<Predicate<AgentOp>> = matches
        .iter()
        .map(AgentOpMatchExt::to_predicate)
        .collect::<Result<_, _>>()?;

    let or_pred = Predicate::from_any(predicates, catch_all());
    Ok(Matcher::from_predicate(or_pred, action, on_no_match))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StringMatch;

    // ========== Predicate Structure ==========

    #[test]
    fn empty_match_compiles_to_single_predicate() {
        let m = AgentOpMatch::default();
        let predicate = m.to_predicate().unwrap();
        assert!(matches!(predicate, Predicate::Single(_)));
    }

    #[test]
    fn single_field_compiles_to_single_predicate() {
        let m = AgentOpMatch {
            agent_id: Some(StringMatch::Exact("claude-main".into())),
            ..Default::default()
        };
        let predicate = m.to_predicate().unwrap();
        assert!(matches!(predicate, Predicate::Single(_)));
    }

    #[test]
    fn multiple_fields_compiles_to_and_predicate() {
        let m = AgentOpMatch {
            agent_id: Some(StringMatch::Exact("claude-main".into())),
            tool_name: Some(StringMatch::Exact("Bash".into())),
            ..Default::default()
        };
        let predicate = m.to_predicate().unwrap();
        assert!(matches!(predicate, Predicate::And(_)));
    }

    // ========== E2E: Agent ID Matching ==========

    #[test]
    fn e2e_agent_id_exact() {
        let m = AgentOpMatch {
            agent_id: Some(StringMatch::Exact("claude-main".into())),
            ..Default::default()
        };
        let matcher = m.compile("matched").unwrap();

        let op = AgentOp::new("claude-main", "Bash", "/tmp");
        assert_eq!(matcher.evaluate(&op), Some("matched"));

        let op = AgentOp::new("rogue-agent", "Bash", "/tmp");
        assert_eq!(matcher.evaluate(&op), None);
    }

    // ========== E2E: Tool Name Matching ==========

    #[test]
    fn e2e_tool_name_prefix() {
        let m = AgentOpMatch {
            tool_name: Some(StringMatch::Prefix("file.".into())),
            ..Default::default()
        };
        let matcher = m.compile("file_op").unwrap();

        let op = AgentOp::new("agent-1", "file.read", "/src/lib.rs");
        assert_eq!(matcher.evaluate(&op), Some("file_op"));

        let op = AgentOp::new("agent-1", "Bash", "/tmp");
        assert_eq!(matcher.evaluate(&op), None);
    }

    // ========== E2E: Resource Matching ==========

    #[test]
    fn e2e_resource_prefix() {
        let m = AgentOpMatch {
            resource: Some(StringMatch::Prefix("/etc/".into())),
            ..Default::default()
        };
        let matcher = m.compile("system_file").unwrap();

        let op = AgentOp::new("agent-1", "file.write", "/etc/passwd");
        assert_eq!(matcher.evaluate(&op), Some("system_file"));

        let op = AgentOp::new("agent-1", "file.write", "/home/user/readme.md");
        assert_eq!(matcher.evaluate(&op), None);
    }

    // ========== E2E: Operation Matching ==========

    #[test]
    fn e2e_operation_exact() {
        let m = AgentOpMatch {
            operation: Some(StringMatch::Exact("delete".into())),
            ..Default::default()
        };
        let matcher = m.compile("destructive").unwrap();

        let op = AgentOp::new("agent-1", "file.delete", "/tmp/file")
            .with_operation("delete");
        assert_eq!(matcher.evaluate(&op), Some("destructive"));

        let op = AgentOp::new("agent-1", "file.read", "/tmp/file")
            .with_operation("read");
        assert_eq!(matcher.evaluate(&op), None);
    }

    // ========== E2E: Metadata Matching ==========

    #[test]
    fn e2e_metadata_contains() {
        let m = AgentOpMatch {
            metadata: Some(vec![MetadataMatch {
                key: "command".into(),
                value: StringMatch::Contains("rm -rf".into()),
            }]),
            ..Default::default()
        };
        let matcher = m.compile("dangerous").unwrap();

        let op = AgentOp::new("agent-1", "Bash", "/")
            .with_meta("command", "sudo rm -rf /");
        assert_eq!(matcher.evaluate(&op), Some("dangerous"));

        let op = AgentOp::new("agent-1", "Bash", "/")
            .with_meta("command", "ls -la");
        assert_eq!(matcher.evaluate(&op), None);
    }

    #[test]
    fn e2e_metadata_missing_returns_no_match() {
        let m = AgentOpMatch {
            metadata: Some(vec![MetadataMatch {
                key: "command".into(),
                value: StringMatch::Exact("ls".into()),
            }]),
            ..Default::default()
        };
        let matcher = m.compile("found").unwrap();

        // No metadata → DataInput returns None → predicate false
        let op = AgentOp::new("agent-1", "Bash", "/");
        assert_eq!(matcher.evaluate(&op), None);
    }

    // ========== E2E: Combined Conditions (AND) ==========

    #[test]
    fn e2e_combined_agent_and_tool() {
        let m = AgentOpMatch {
            agent_id: Some(StringMatch::Exact("claude-main".into())),
            tool_name: Some(StringMatch::Prefix("file.".into())),
            resource: Some(StringMatch::Prefix("/home/user/project/".into())),
            ..Default::default()
        };
        let matcher = m.compile("allowed").unwrap();

        // All match
        let op = AgentOp::new("claude-main", "file.read", "/home/user/project/src/lib.rs");
        assert_eq!(matcher.evaluate(&op), Some("allowed"));

        // Wrong agent
        let op = AgentOp::new("rogue", "file.read", "/home/user/project/src/lib.rs");
        assert_eq!(matcher.evaluate(&op), None);

        // Wrong resource
        let op = AgentOp::new("claude-main", "file.read", "/etc/passwd");
        assert_eq!(matcher.evaluate(&op), None);
    }

    // ========== E2E: Empty Match ==========

    #[test]
    fn e2e_empty_match_matches_everything() {
        let m = AgentOpMatch::default();
        let matcher = m.compile("catch_all").unwrap();

        let op = AgentOp::new("any-agent", "any-tool", "/any/path");
        assert_eq!(matcher.evaluate(&op), Some("catch_all"));
    }

    // ========== E2E: Multiple Rules (OR) ==========

    #[test]
    fn e2e_multiple_rules_or() {
        let rules = vec![
            AgentOpMatch {
                tool_name: Some(StringMatch::Exact("Bash".into())),
                ..Default::default()
            },
            AgentOpMatch {
                tool_name: Some(StringMatch::Exact("Write".into())),
                ..Default::default()
            },
        ];

        let matcher = compile_agent_op_matches(&rules, "blocked", None).unwrap();

        let op = AgentOp::new("agent-1", "Bash", "/");
        assert_eq!(matcher.evaluate(&op), Some("blocked"));

        let op = AgentOp::new("agent-1", "Write", "/");
        assert_eq!(matcher.evaluate(&op), Some("blocked"));

        let op = AgentOp::new("agent-1", "Read", "/");
        assert_eq!(matcher.evaluate(&op), None);
    }

    #[test]
    fn e2e_multiple_rules_with_fallback() {
        let rules = vec![AgentOpMatch {
            tool_name: Some(StringMatch::Exact("Bash".into())),
            ..Default::default()
        }];

        let matcher = compile_agent_op_matches(&rules, "block", Some("allow")).unwrap();

        let op = AgentOp::new("agent-1", "Bash", "/");
        assert_eq!(matcher.evaluate(&op), Some("block"));

        let op = AgentOp::new("agent-1", "Read", "/");
        assert_eq!(matcher.evaluate(&op), Some("allow"));
    }

    // ========== Error Cases ==========

    #[test]
    fn compile_invalid_regex_returns_error() {
        let m = AgentOpMatch {
            tool_name: Some(StringMatch::Regex("[bad".into())),
            ..Default::default()
        };
        assert!(m.compile::<&str>("x").is_err());
    }

    // ========== Realistic Scenarios ==========

    #[test]
    fn scenario_block_system_file_deletion() {
        let rule = AgentOpMatch {
            operation: Some(StringMatch::Exact("delete".into())),
            resource: Some(StringMatch::Prefix("/etc/".into())),
            ..Default::default()
        };
        let matcher = rule.compile("block").unwrap();

        let op = AgentOp::new("agent-1", "file.delete", "/etc/passwd")
            .with_operation("delete");
        assert_eq!(matcher.evaluate(&op), Some("block"));

        let op = AgentOp::new("agent-1", "file.delete", "/tmp/scratch")
            .with_operation("delete");
        assert_eq!(matcher.evaluate(&op), None);
    }

    #[test]
    fn scenario_allow_specific_agent_in_project() {
        let rule = AgentOpMatch {
            agent_id: Some(StringMatch::Exact("claude-main".into())),
            tool_name: Some(StringMatch::Prefix("file.".into())),
            resource: Some(StringMatch::Prefix("/home/user/project/".into())),
            ..Default::default()
        };
        let matcher = rule.compile("allow").unwrap();

        let op = AgentOp::new("claude-main", "file.write", "/home/user/project/src/main.rs");
        assert_eq!(matcher.evaluate(&op), Some("allow"));

        // Different agent — denied
        let op = AgentOp::new("rogue-agent", "file.write", "/home/user/project/src/main.rs");
        assert_eq!(matcher.evaluate(&op), None);
    }
}
