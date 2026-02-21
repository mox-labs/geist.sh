//! `DataInput` implementations for extracting data from `AgentOp`.

use crate::context::AgentOp;
use rumi::prelude::*;

/// Extracts the agent ID.
#[derive(Debug, Clone)]
pub struct AgentIdInput;

impl DataInput<AgentOp> for AgentIdInput {
    fn get(&self, ctx: &AgentOp) -> MatchingData {
        MatchingData::String(ctx.agent_id().to_string())
    }
}

/// Extracts the tool/command name.
#[derive(Debug, Clone)]
pub struct ToolNameInput;

impl DataInput<AgentOp> for ToolNameInput {
    fn get(&self, ctx: &AgentOp) -> MatchingData {
        MatchingData::String(ctx.tool_name().to_string())
    }
}

/// Extracts the target resource path.
#[derive(Debug, Clone)]
pub struct ResourceInput;

impl DataInput<AgentOp> for ResourceInput {
    fn get(&self, ctx: &AgentOp) -> MatchingData {
        MatchingData::String(ctx.resource().to_string())
    }
}

/// Extracts the operation verb.
#[derive(Debug, Clone)]
pub struct OperationInput;

impl DataInput<AgentOp> for OperationInput {
    fn get(&self, ctx: &AgentOp) -> MatchingData {
        MatchingData::String(ctx.operation().to_string())
    }
}

/// Extracts the session ID.
#[derive(Debug, Clone)]
pub struct SessionIdInput;

impl DataInput<AgentOp> for SessionIdInput {
    fn get(&self, ctx: &AgentOp) -> MatchingData {
        MatchingData::String(ctx.session_id().to_string())
    }
}

/// Extracts a metadata value by key.
///
/// Returns `MatchingData::None` if the key is not present
/// (which evaluates to `false` per rumi invariant).
#[derive(Debug, Clone)]
pub struct MetadataInput {
    key: String,
}

impl MetadataInput {
    /// Create a new metadata extractor for the given key.
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

impl DataInput<AgentOp> for MetadataInput {
    fn get(&self, ctx: &AgentOp) -> MatchingData {
        ctx.meta(&self.key)
            .map_or(MatchingData::None, |s| MatchingData::String(s.to_string()))
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Registry support (feature = "registry")
// ═══════════════════════════════════════════════════════════════════════════════

/// Configuration for [`MetadataInput`] (requires a key name).
#[cfg(feature = "registry")]
#[derive(serde::Deserialize)]
pub struct MetadataInputConfig {
    /// The metadata key to extract.
    pub key: String,
}

#[cfg(feature = "registry")]
impl rumi::IntoDataInput<AgentOp> for AgentIdInput {
    type Config = rumi::UnitConfig;

    fn from_config(
        _: rumi::UnitConfig,
    ) -> Result<Box<dyn rumi::DataInput<AgentOp>>, rumi::MatcherError> {
        Ok(Box::new(AgentIdInput))
    }
}

#[cfg(feature = "registry")]
impl rumi::IntoDataInput<AgentOp> for ToolNameInput {
    type Config = rumi::UnitConfig;

    fn from_config(
        _: rumi::UnitConfig,
    ) -> Result<Box<dyn rumi::DataInput<AgentOp>>, rumi::MatcherError> {
        Ok(Box::new(ToolNameInput))
    }
}

#[cfg(feature = "registry")]
impl rumi::IntoDataInput<AgentOp> for ResourceInput {
    type Config = rumi::UnitConfig;

    fn from_config(
        _: rumi::UnitConfig,
    ) -> Result<Box<dyn rumi::DataInput<AgentOp>>, rumi::MatcherError> {
        Ok(Box::new(ResourceInput))
    }
}

#[cfg(feature = "registry")]
impl rumi::IntoDataInput<AgentOp> for OperationInput {
    type Config = rumi::UnitConfig;

    fn from_config(
        _: rumi::UnitConfig,
    ) -> Result<Box<dyn rumi::DataInput<AgentOp>>, rumi::MatcherError> {
        Ok(Box::new(OperationInput))
    }
}

#[cfg(feature = "registry")]
impl rumi::IntoDataInput<AgentOp> for SessionIdInput {
    type Config = rumi::UnitConfig;

    fn from_config(
        _: rumi::UnitConfig,
    ) -> Result<Box<dyn rumi::DataInput<AgentOp>>, rumi::MatcherError> {
        Ok(Box::new(SessionIdInput))
    }
}

#[cfg(feature = "registry")]
impl rumi::IntoDataInput<AgentOp> for MetadataInput {
    type Config = MetadataInputConfig;

    fn from_config(
        config: Self::Config,
    ) -> Result<Box<dyn rumi::DataInput<AgentOp>>, rumi::MatcherError> {
        Ok(Box::new(MetadataInput::new(config.key)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_id_input() {
        let op = AgentOp::new("claude-main", "file.read", "/src/lib.rs");
        assert_eq!(
            AgentIdInput.get(&op),
            MatchingData::String("claude-main".into())
        );
    }

    #[test]
    fn tool_name_input() {
        let op = AgentOp::new("agent-1", "Bash", "/tmp");
        assert_eq!(
            ToolNameInput.get(&op),
            MatchingData::String("Bash".into())
        );
    }

    #[test]
    fn resource_input() {
        let op = AgentOp::new("agent-1", "file.write", "/etc/passwd");
        assert_eq!(
            ResourceInput.get(&op),
            MatchingData::String("/etc/passwd".into())
        );
    }

    #[test]
    fn operation_input() {
        let op = AgentOp::new("agent-1", "file.write", "/tmp/out")
            .with_operation("write");
        assert_eq!(
            OperationInput.get(&op),
            MatchingData::String("write".into())
        );
    }

    #[test]
    fn operation_input_empty_default() {
        let op = AgentOp::new("agent-1", "file.read", "/src");
        assert_eq!(
            OperationInput.get(&op),
            MatchingData::String(String::new())
        );
    }

    #[test]
    fn session_id_input() {
        let op = AgentOp::new("agent-1", "tool", "/")
            .with_session_id("sess-123");
        assert_eq!(
            SessionIdInput.get(&op),
            MatchingData::String("sess-123".into())
        );
    }

    #[test]
    fn metadata_input_present() {
        let op = AgentOp::new("agent-1", "Bash", "/")
            .with_meta("command", "ls -la");
        assert_eq!(
            MetadataInput::new("command").get(&op),
            MatchingData::String("ls -la".into())
        );
    }

    #[test]
    fn metadata_input_absent() {
        let op = AgentOp::new("agent-1", "Bash", "/");
        assert_eq!(
            MetadataInput::new("command").get(&op),
            MatchingData::None
        );
    }
}
