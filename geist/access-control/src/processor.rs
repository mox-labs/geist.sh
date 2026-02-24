//! Access control processor — bridges geist-edge pipeline with policy evaluator.

use std::sync::Arc;

use geist_edge::prelude::*;

use crate::config::AccessControlPolicy;
use crate::evaluator::{AgentOp, PolicyDecision, PolicyEvaluator};

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

impl Processor for AccessControlProcessor {
    fn name(&self) -> &str {
        "access-control"
    }

    fn process_request_headers(
        &self,
        msg: &HttpMessage,
    ) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>> {
        let op = AgentOp {
            agent_id: msg.header("x-geist-agent-id").unwrap_or("unknown"),
            tool_name: msg.header("x-geist-tool-name").unwrap_or("unknown"),
            resource: msg.header("x-geist-resource").unwrap_or("/"),
            operation: msg.header("x-geist-operation").unwrap_or(""),
            session_id: msg.header("x-geist-session-id").unwrap_or(""),
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
                    Ok(PhaseResult::Respond(ImmediateResponse {
                        status: Some(HttpStatus { code: 403 }),
                        body: body.into_bytes(),
                        ..Default::default()
                    }))
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

// Self-registration — zero code changes to core or binary.
geist_edge::register_processor!(TYPE_URL, AccessControlProcessor);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentOpMatch, AllowRule, DenyRule};

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
        let msg = make_request(vec![
            ("x-geist-agent-id", "claude-main"),
            ("x-geist-tool-name", "Read"),
            ("x-geist-resource", "/src/lib.rs"),
        ]);
        let result = proc.process_request_headers(&msg).await.unwrap();
        assert!(matches!(result, PhaseResult::Continue));
    }

    #[tokio::test]
    async fn denied_tool_returns_403() {
        let proc = AccessControlProcessor::new(simple_policy()).unwrap();
        let msg = make_request(vec![
            ("x-geist-agent-id", "claude-main"),
            ("x-geist-tool-name", "Bash"),
            ("x-geist-resource", "/"),
        ]);
        let result = proc.process_request_headers(&msg).await.unwrap();
        match result {
            PhaseResult::Respond(resp) => {
                assert_eq!(resp.status.unwrap().code, 403);
                let body = String::from_utf8(resp.body).unwrap();
                assert!(body.contains("access_denied"));
                assert!(body.contains("No Bash"));
            }
            _ => panic!("expected Respond(403)"),
        }
    }

    #[tokio::test]
    async fn unknown_agent_denied_by_default() {
        let proc = AccessControlProcessor::new(simple_policy()).unwrap();
        let msg = make_request(vec![
            ("x-geist-agent-id", "rogue-agent"),
            ("x-geist-tool-name", "Read"),
            ("x-geist-resource", "/src/lib.rs"),
        ]);
        let result = proc.process_request_headers(&msg).await.unwrap();
        assert!(matches!(result, PhaseResult::Respond(_)));
    }

    #[tokio::test]
    async fn missing_headers_use_defaults() {
        let proc = AccessControlProcessor::new(simple_policy()).unwrap();
        let msg = make_request(vec![]);
        let result = proc.process_request_headers(&msg).await.unwrap();
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
