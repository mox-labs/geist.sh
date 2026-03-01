//! Access log processor — emits structured tracing events to a non-blocking
//! file writer via `tracing-appender`.
//!
//! Request path: format fields + `tracing::info!()` (no I/O, no locks).
//! Background: `tracing-appender::NonBlocking` drains to file on a dedicated
//! thread — same pattern as logback AsyncAppender and Envoy's flush thread.

use std::sync::Arc;
use std::time::SystemTime;

use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};

use geist_edge::prelude::*;

use crate::config::AccessLogConfig;

/// Type URL for this processor extension.
pub const TYPE_URL: &str = "mox.geist.processors.v1.AccessLog";

/// Target log entry size (~3KB). Pad with context to reach this for fair
/// benchmark comparison against Envoy file access log and SCG logback.
const TARGET_ENTRY_BYTES: usize = 3072;

/// Access log processor. Emits structured JSON access log entries via
/// `tracing-appender` non-blocking writer.
///
/// Headers-only — never touches the body. Thread-safe via `&self`.
pub struct AccessLogProcessor {
    /// Keeps the background writer thread alive. Dropped on processor drop,
    /// which flushes remaining entries.
    _guard: WorkerGuard,
    /// Non-blocking writer — channel send, no I/O on request path.
    writer: NonBlocking,
}

impl AccessLogProcessor {
    pub fn new(config: AccessLogConfig) -> Result<Self, ProcessorError> {
        let (writer, guard) = if config.path == "-" || config.path == "stdout" {
            tracing_appender::non_blocking(std::io::stdout())
        } else {
            // Ensure parent directory exists.
            let path = std::path::Path::new(&config.path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ProcessorError::new(TYPE_URL, format!("failed to create log dir: {e}"))
                })?;
            }

            let dir = parent_dir(&config.path)?;
            let filename = file_name(&config.path)?;
            let file_appender = tracing_appender::rolling::never(dir, filename);
            tracing_appender::non_blocking(file_appender)
        };

        Ok(Self {
            _guard: guard,
            writer,
        })
    }

    /// Format a ~3KB structured JSON log entry.
    fn format_entry(&self, msg: &HttpMessage) -> String {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let ts_secs = now.as_secs();
        let ts_nanos = now.subsec_nanos();

        let method = msg.header(":method").unwrap_or("-");
        let path = msg.header(":path").unwrap_or("-");
        let authority = msg.header(":authority").unwrap_or("-");
        let agent = msg.header("x-geist-agent-id").unwrap_or("-");
        let tool = msg.header("x-geist-tool-name").unwrap_or("-");
        let resource = msg.header("x-geist-resource").unwrap_or("-");
        let session = msg.header("x-geist-session-id").unwrap_or("-");
        let operation = msg.header("x-geist-operation").unwrap_or("-");
        let content_type = msg.header("content-type").unwrap_or("-");
        let user_agent = msg.header("user-agent").unwrap_or("-");
        let accept = msg.header("accept").unwrap_or("-");
        let host = msg.header("host").unwrap_or("-");

        let mut entry = format!(
            "{{\"ts\":\"{ts_secs}.{ts_nanos:09}\",\"method\":\"{method}\",\"path\":\"{path}\",\
             \"authority\":\"{authority}\",\"host\":\"{host}\",\
             \"agent\":\"{agent}\",\"tool\":\"{tool}\",\
             \"resource\":\"{resource}\",\"session\":\"{session}\",\
             \"operation\":\"{operation}\",\
             \"content_type\":\"{content_type}\",\"user_agent\":\"{user_agent}\",\
             \"accept\":\"{accept}\",\"processor\":\"geist-edge\",\
             \"pipeline_pos\":0"
        );

        // Pad to ~3KB for fair I/O comparison.
        let current_len = entry.len() + 2; // +2 for closing "}\n"
        if current_len < TARGET_ENTRY_BYTES {
            let pad_len = TARGET_ENTRY_BYTES - current_len - 11; // 11 for ,"_pad":"" wrapper
            entry.push_str(",\"_pad\":\"");
            for _ in 0..pad_len {
                entry.push('.');
            }
            entry.push('"');
        }

        entry.push_str("}\n");
        entry
    }
}

impl Processor for AccessLogProcessor {
    fn name(&self) -> &str {
        "access-log"
    }

    fn process_request_headers(
        &self,
        msg: &HttpMessage,
    ) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>> {
        let entry = self.format_entry(msg);

        Box::pin(async move {
            use std::io::Write;
            // Clone shares the same channel — NonBlocking is designed for this.
            // write_all is a channel send, no disk I/O on the request path.
            let mut w = self.writer.clone();
            w.write_all(entry.as_bytes())
                .map_err(|e| ProcessorError::new(TYPE_URL, format!("log write failed: {e}")))?;
            Ok(PhaseResult::Continue)
        })
    }
}

impl IntoProcessor for AccessLogProcessor {
    type Config = AccessLogConfig;

    fn from_config(config: Self::Config) -> Result<Arc<dyn Processor>, ProcessorError> {
        Ok(Arc::new(AccessLogProcessor::new(config)?))
    }
}

// Self-registration — zero code changes to core.
geist_edge::register_processor!(TYPE_URL, AccessLogProcessor);

/// Extract parent directory from a file path.
fn parent_dir(path: &str) -> Result<&str, ProcessorError> {
    std::path::Path::new(path)
        .parent()
        .and_then(|p| p.to_str())
        .ok_or_else(|| ProcessorError::new(TYPE_URL, format!("invalid log path: {path}")))
}

/// Extract file name from a path.
fn file_name(path: &str) -> Result<&str, ProcessorError> {
    std::path::Path::new(path)
        .file_name()
        .and_then(|f| f.to_str())
        .ok_or_else(|| ProcessorError::new(TYPE_URL, format!("invalid log filename: {path}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(headers: Vec<(&str, &str)>) -> HttpMessage {
        let header_values: Vec<HeaderValue> = headers
            .into_iter()
            .map(|(k, v)| HeaderValue {
                key: k.to_string(),
                value: v.to_string(),
                raw_value: vec![],
            })
            .collect();

        let req = ProcessingRequest {
            request: Some(Request::RequestHeaders(HttpHeaders {
                headers: Some(HeaderMap {
                    headers: header_values,
                }),
                ..Default::default()
            })),
            ..Default::default()
        };
        HttpMessage::from(&req)
    }

    #[tokio::test]
    async fn logs_to_stdout_and_continues() {
        let config = AccessLogConfig {
            path: "stdout".into(),
        };
        let proc = AccessLogProcessor::new(config).unwrap();
        let msg = make_request(vec![
            (":method", "POST"),
            (":path", "/api/test"),
            ("x-geist-agent-id", "claude-main"),
            ("x-geist-tool-name", "Read"),
        ]);
        let result = proc.process_request_headers(&msg).await.unwrap();
        assert!(matches!(result, PhaseResult::Continue));
    }

    #[tokio::test]
    async fn logs_to_file_with_3kb_entry() {
        let dir = std::env::temp_dir().join("geist-log-test");
        let path = dir.join("access.log");

        // Clean up from prior runs.
        let _ = std::fs::remove_dir_all(&dir);

        let config = AccessLogConfig {
            path: path.to_string_lossy().into_owned(),
        };
        let proc = AccessLogProcessor::new(config).unwrap();
        let msg = make_request(vec![
            (":method", "GET"),
            (":path", "/health"),
            ("x-geist-agent-id", "monitor"),
        ]);
        let result = proc.process_request_headers(&msg).await.unwrap();
        assert!(matches!(result, PhaseResult::Continue));

        // Drop the processor to flush the non-blocking writer.
        drop(proc);

        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("\"method\":\"GET\""));
        assert!(contents.contains("\"path\":\"/health\""));
        assert!(contents.contains("\"agent\":\"monitor\""));
        assert!(
            contents.len() >= 3000,
            "log entry should be ~3KB, got {} bytes",
            contents.len()
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn missing_headers_use_defaults() {
        let config = AccessLogConfig {
            path: "stdout".into(),
        };
        let proc = AccessLogProcessor::new(config).unwrap();
        let msg = make_request(vec![]);
        let result = proc.process_request_headers(&msg).await.unwrap();
        assert!(matches!(result, PhaseResult::Continue));
    }

    #[tokio::test]
    async fn into_processor_from_json() {
        let config: AccessLogConfig = serde_json::from_value(serde_json::json!({
            "path": "stdout"
        }))
        .unwrap();
        let processor = AccessLogProcessor::from_config(config).unwrap();
        assert_eq!(processor.name(), "access-log");
    }

    #[test]
    fn processor_is_headers_only() {
        assert_eq!(ProcessingMode::default(), ProcessingMode::HEADERS_ONLY);
    }
}
