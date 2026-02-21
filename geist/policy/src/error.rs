//! Error types for the policy decision point.

/// Errors from policy compilation and evaluation.
#[derive(Debug)]
#[non_exhaustive]
pub enum PolicyError {
    /// A rumi matcher error in a specific rule.
    InvalidRule {
        /// Which rule list: "deny" or "allow".
        rule_type: &'static str,
        /// Zero-based index of the rule that failed.
        rule_index: usize,
        /// Which field contained the invalid pattern (e.g., "tool_name", "resource").
        field: Option<String>,
        /// The underlying matcher error.
        source: rumi::MatcherError,
    },
    /// Policy configuration failed to parse.
    InvalidPolicy {
        /// The underlying error message.
        source: String,
    },
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRule {
                rule_type,
                rule_index,
                field,
                source,
            } => {
                write!(f, "{rule_type} rule [{rule_index}]")?;
                if let Some(field) = field {
                    write!(f, " field '{field}'")?;
                }
                write!(f, ": {source}")
            }
            Self::InvalidPolicy { source } => write!(f, "invalid policy: {source}"),
        }
    }
}

impl std::error::Error for PolicyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidRule { source, .. } => Some(source),
            Self::InvalidPolicy { .. } => None,
        }
    }
}

impl From<rumi::MatcherError> for PolicyError {
    fn from(e: rumi::MatcherError) -> Self {
        Self::InvalidRule {
            rule_type: "unknown",
            rule_index: 0,
            field: None,
            source: e,
        }
    }
}

impl From<serde_json::Error> for PolicyError {
    fn from(e: serde_json::Error) -> Self {
        Self::InvalidPolicy {
            source: e.to_string(),
        }
    }
}
