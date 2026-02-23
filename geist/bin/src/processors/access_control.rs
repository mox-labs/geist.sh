//! Access control processor — enforces access control policies.
//!
//! Follows Envoy's RBAC pattern: policy config (the WHAT) is separate from
//! the processor (the HOW). The processor compiles the policy into matchers
//! internally — callers configure with an [`AccessControlPolicy`], not a matcher.
//!
//! # Header Convention (Provisional)
//!
//! Currently extracts agent context from synthetic headers. In practice,
//! the edge will extract from real HTTP attributes (path, auth headers)
//! and MCP request body (tool name, arguments). This will evolve with
//! the adapter layer.
//!
//! - `x-geist-agent-id` — agent identity
//! - `x-geist-tool-name` — tool/command being invoked
//! - `x-geist-resource` — target resource path
//! - `x-geist-operation` — operation verb (optional)
//! - `x-geist-session-id` — session identifier (optional)

use geist_edge::prelude::*;
use geist_policy::{AgentOp, AccessControlPolicy, PolicyDecision, PolicyError, PolicyEvaluator};

/// Enforces access control policies on inbound requests.
///
/// Configured with an [`AccessControlPolicy`]. The policy
/// is compiled to matchers at construction time. Thread-safe — uses `&self`
/// for evaluation, safe behind `Arc`.
///
/// Follows the policy-processor model: the policy defines WHAT (user config),
/// the processor defines HOW (enforcement). Internally uses rumi matchers
/// with deny-first semantics, similar to Envoy's RBAC engine.
pub struct AccessControlProcessor {
    evaluator: PolicyEvaluator,
}

impl AccessControlProcessor {
    /// Create a new access control processor from an access control policy.
    ///
    /// Compiles the policy into matchers at construction time.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError`] if any regex pattern in the policy is invalid.
    pub fn new(policy: AccessControlPolicy) -> Result<Self, PolicyError> {
        let evaluator = PolicyEvaluator::compile(policy)?;
        Ok(Self { evaluator })
    }

    /// Create a new access control processor from a JSON policy string.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError`] if JSON is invalid or patterns fail to compile.
    pub fn from_json(json: &str) -> Result<Self, PolicyError> {
        let evaluator = PolicyEvaluator::from_json(json)?;
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
        let agent_id = msg.header("x-geist-agent-id").unwrap_or("unknown");
        let tool_name = msg.header("x-geist-tool-name").unwrap_or("unknown");
        let resource = msg.header("x-geist-resource").unwrap_or("/");

        let mut op = AgentOp::new(agent_id, tool_name, resource);

        if let Some(operation) = msg.header("x-geist-operation") {
            op = op.with_operation(operation);
        }
        if let Some(session_id) = msg.header("x-geist-session-id") {
            op = op.with_session_id(session_id);
        }

        let decision = self.evaluator.evaluate(&op);

        Box::pin(async move {
            match decision {
                PolicyDecision::Allow => Ok(PhaseResult::Continue),
                PolicyDecision::Deny { reason } => {
                    let body = format!("{{\"error\":\"access_denied\",\"reason\":\"{reason}\"}}");
                    Ok(PhaseResult::Respond(ImmediateResponse {
                        status: Some(HttpStatus { code: 403 }),
                        body: body.into_bytes(),
                        ..Default::default()
                    }))
                }
                // PolicyDecision is #[non_exhaustive] — fail-closed on unknown variants.
                _ => Ok(PhaseResult::Respond(ImmediateResponse {
                    status: Some(HttpStatus { code: 403 }),
                    body: b"{\"error\":\"access_denied\",\"reason\":\"unknown decision\"}".to_vec(),
                    ..Default::default()
                })),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geist_policy::{AgentOpMatch, AllowRule, DenyRule, StringMatch};

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
                    tool_name: Some(StringMatch::Exact("Bash".into())),
                    ..Default::default()
                }],
            }],
            allow: vec![AllowRule {
                matches: vec![AgentOpMatch {
                    agent_id: Some(StringMatch::Exact("claude-main".into())),
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
        match result {
            PhaseResult::Respond(resp) => {
                assert_eq!(resp.status.unwrap().code, 403);
            }
            _ => panic!("expected Respond(403)"),
        }
    }

    #[tokio::test]
    async fn missing_headers_use_defaults() {
        let proc = AccessControlProcessor::new(simple_policy()).unwrap();
        let msg = make_request(vec![]);

        let result = proc.process_request_headers(&msg).await.unwrap();
        assert!(matches!(result, PhaseResult::Respond(_)));
    }

    #[tokio::test]
    async fn from_json_constructs_processor() {
        let json = r#"{
            "deny": [{ "reason": "No Bash", "matches": [{ "tool_name": { "Exact": "Bash" } }] }],
            "allow": [{ "matches": [{ "agent_id": { "Exact": "claude-main" } }] }]
        }"#;
        let proc = AccessControlProcessor::from_json(json).unwrap();

        let msg = make_request(vec![
            ("x-geist-agent-id", "claude-main"),
            ("x-geist-tool-name", "Read"),
            ("x-geist-resource", "/src/lib.rs"),
        ]);
        let result = proc.process_request_headers(&msg).await.unwrap();
        assert!(matches!(result, PhaseResult::Continue));
    }
}
