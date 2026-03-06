//! Access control processor — deny-first evaluation.
//!
//! Enforces access control policies on inbound requests.
//! Configured with [`AccessControlPolicy`] — deny rules checked first,
//! then allow rules, then default deny.
//!
//! # Type URL
//!
//! `mox.geist.processors.v1.AccessControl`

mod config;
mod evaluator;

use std::sync::Arc;

use bytes::Bytes;

use crate::phase::{ImmediateResponse, PhaseResult};
use crate::processor::{BoxFuture, Processor, ProcessorError};
use crate::registry::IntoProcessor;

use config::AccessControlPolicy;
use evaluator::{AgentOp, PolicyDecision, PolicyEvaluator};

pub use config::{AccessControlPolicy as Policy, AgentOpMatch, AllowRule, DenyRule};

/// Type URL for this processor extension.
pub const TYPE_URL: &str = "mox.geist.processors.v1.AccessControl";

/// Access control processor. Enforces deny-first access control policies
/// on inbound request headers.
///
/// Extracts agent context from `x-geist-*` synthetic headers (provisional —
/// will evolve with the adapter layer to extract from real HTTP attributes,
/// MCP request body, etc.).
///
/// Configured with [`AccessControlPolicy`]. Policy is compiled to rumi
/// matchers at construction time. Thread-safe via `&self`.
pub struct AccessControlProcessor {
    evaluator: PolicyEvaluator,
}

impl AccessControlProcessor {
    pub fn new(policy: AccessControlPolicy) -> Result<Self, ProcessorError> {
        let evaluator = PolicyEvaluator::compile(policy).map_err(|e| {
            ProcessorError::new(TYPE_URL, format!("policy compilation failed: {e}"))
        })?;
        Ok(Self { evaluator })
    }
}

/// Extract a header value from request parts, returning the given default if missing.
fn get_header<'a>(parts: &'a http::request::Parts, name: &str, default: &'a str) -> &'a str {
    parts
        .headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .unwrap_or(default)
}

impl Processor for AccessControlProcessor {
    fn name(&self) -> &str {
        "access-control"
    }

    fn process_request_headers(
        &self,
        parts: &http::request::Parts,
    ) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>> {
        let op = AgentOp {
            agent_id: get_header(parts, "x-geist-agent-id", "unknown"),
            tool_name: get_header(parts, "x-geist-tool-name", "unknown"),
            resource: get_header(parts, "x-geist-resource", "/"),
            operation: get_header(parts, "x-geist-operation", ""),
            session_id: get_header(parts, "x-geist-session-id", ""),
        };

        let decision = self.evaluator.evaluate(&op);

        Box::pin(async move {
            match decision {
                PolicyDecision::Allow => Ok(PhaseResult::Continue),
                PolicyDecision::Deny { reason } => {
                    let body = serde_json::json!({
                        "error": "access_denied",
                        "reason": reason
                    })
                    .to_string();
                    Ok(PhaseResult::Respond(
                        ImmediateResponse::with_status(http::StatusCode::FORBIDDEN)
                            .body(Bytes::from(body))
                            .header(
                                http::header::CONTENT_TYPE,
                                http::HeaderValue::from_static("application/json"),
                            ),
                    ))
                }
            }
        })
    }
}

impl IntoProcessor for AccessControlProcessor {
    type Config = AccessControlPolicy;

    fn from_config(config: Self::Config) -> Result<Arc<dyn Processor>, ProcessorError> {
        Ok(Arc::new(AccessControlProcessor::new(config)?))
    }
}

// Self-registration via inventory.
crate::register_processor!(TYPE_URL, AccessControlProcessor);

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(headers: Vec<(&str, &str)>) -> http::request::Parts {
        let mut builder = http::Request::builder();
        for (k, v) in headers {
            builder = builder.header(k, v);
        }
        builder.body(()).unwrap().into_parts().0
    }

    fn simple_policy() -> AccessControlPolicy {
        AccessControlPolicy {
            deny: vec![DenyRule {
                reason: "No Bash".into(),
                matches: vec![AgentOpMatch {
                    tool_name: Some(rumi::StringMatchSpec::Exact("Bash".into())),
                    ..Default::default()
                }],
            }],
            allow: vec![AllowRule {
                matches: vec![AgentOpMatch {
                    agent_id: Some(rumi::StringMatchSpec::Exact("claude-main".into())),
                    ..Default::default()
                }],
            }],
        }
    }

    #[tokio::test]
    async fn allowed_operation_continues() {
        let proc = AccessControlProcessor::new(simple_policy()).unwrap();
        let parts = make_request(vec![
            ("x-geist-agent-id", "claude-main"),
            ("x-geist-tool-name", "Read"),
            ("x-geist-resource", "/src/lib.rs"),
        ]);
        let result = proc.process_request_headers(&parts).await.unwrap();
        assert!(matches!(result, PhaseResult::Continue));
    }

    #[tokio::test]
    async fn denied_tool_returns_403() {
        let proc = AccessControlProcessor::new(simple_policy()).unwrap();
        let parts = make_request(vec![
            ("x-geist-agent-id", "claude-main"),
            ("x-geist-tool-name", "Bash"),
            ("x-geist-resource", "/"),
        ]);
        let result = proc.process_request_headers(&parts).await.unwrap();
        match result {
            PhaseResult::Respond(resp) => {
                assert_eq!(resp.status, http::StatusCode::FORBIDDEN);
                let body = String::from_utf8(resp.body.to_vec()).unwrap();
                assert!(body.contains("access_denied"));
                assert!(body.contains("No Bash"));
            }
            _ => panic!("expected Respond(403)"),
        }
    }

    #[tokio::test]
    async fn unknown_agent_denied_by_default() {
        let proc = AccessControlProcessor::new(simple_policy()).unwrap();
        let parts = make_request(vec![
            ("x-geist-agent-id", "rogue-agent"),
            ("x-geist-tool-name", "Read"),
            ("x-geist-resource", "/src/lib.rs"),
        ]);
        let result = proc.process_request_headers(&parts).await.unwrap();
        assert!(matches!(result, PhaseResult::Respond(_)));
    }

    #[tokio::test]
    async fn missing_headers_use_defaults() {
        let proc = AccessControlProcessor::new(simple_policy()).unwrap();
        let parts = make_request(vec![]);
        let result = proc.process_request_headers(&parts).await.unwrap();
        assert!(matches!(result, PhaseResult::Respond(_)));
    }

    #[tokio::test]
    async fn into_processor_from_json() {
        let config: AccessControlPolicy = serde_json::from_value(serde_json::json!({
            "deny": [{"reason": "No Bash", "matches": [{"tool_name": {"Exact": "Bash"}}]}],
            "allow": [{"matches": [{"agent_id": {"Exact": "claude-main"}}]}]
        }))
        .unwrap();

        let processor = AccessControlProcessor::from_config(config).unwrap();
        assert_eq!(processor.name(), "access-control");
    }
}
