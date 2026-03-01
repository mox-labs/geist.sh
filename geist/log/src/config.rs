//! Access log configuration types.

use serde::Deserialize;

/// Access log configuration.
///
/// Specifies where to write access log entries.
#[derive(Debug, Clone, Deserialize)]
pub struct AccessLogConfig {
    /// File path for log output. Use "-" or "stdout" for stdout.
    pub path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_deserializes_from_json() {
        let json = r#"{"path": "/var/log/geist/access.log"}"#;
        let config: AccessLogConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.path, "/var/log/geist/access.log");
    }

    #[test]
    fn config_deserializes_stdout() {
        let json = r#"{"path": "stdout"}"#;
        let config: AccessLogConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.path, "stdout");
    }
}
