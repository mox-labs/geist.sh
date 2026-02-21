//! Agent operation context for policy matching.

use std::collections::HashMap;

/// An operation requested by an autonomous agent.
///
/// Carries all data available at the policy enforcement point:
/// agent identity, tool/command being invoked, target resource,
/// and session metadata.
///
/// # Construction
///
/// Use the `new` constructor with the three required fields,
/// then chain builder methods for optional fields:
///
/// ```
/// use geist_policy::AgentOp;
///
/// let op = AgentOp::new("claude-main", "file.read", "/home/user/project/src/lib.rs")
///     .with_operation("read")
///     .with_session_id("sess-abc-123")
///     .with_meta("encoding", "utf-8");
/// ```
#[derive(Debug, Clone)]
pub struct AgentOp {
    agent_id: String,
    tool_name: String,
    resource: String,
    operation: String,
    session_id: String,
    metadata: HashMap<String, String>,
}

impl AgentOp {
    /// Create a new agent operation with the three required fields.
    #[must_use]
    pub fn new(
        agent_id: impl Into<String>,
        tool_name: impl Into<String>,
        resource: impl Into<String>,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            tool_name: tool_name.into(),
            resource: resource.into(),
            operation: String::new(),
            session_id: String::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set the operation verb (builder pattern).
    #[must_use]
    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = operation.into();
        self
    }

    /// Set the session ID (builder pattern).
    #[must_use]
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = session_id.into();
        self
    }

    /// Add a metadata key-value pair (builder pattern).
    #[must_use]
    pub fn with_meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get the agent ID.
    #[must_use]
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Get the tool/command name (MCP tool or command name).
    #[must_use]
    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }

    /// Get the target resource path.
    #[must_use]
    pub fn resource(&self) -> &str {
        &self.resource
    }

    /// Get the operation verb (read, write, execute, delete, etc.).
    #[must_use]
    pub fn operation(&self) -> &str {
        &self.operation
    }

    /// Get the session ID.
    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get a metadata value by key.
    #[must_use]
    pub fn meta(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(String::as_str)
    }
}
