//! Integration test: governed reverse proxy with access control.
//!
//! Spins up a mock upstream, configures the proxy with an ACL processor,
//! and verifies that allowed requests are forwarded and denied requests
//! are rejected with 403.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::response::IntoResponse;
use axum::Router;
use tokio::net::TcpListener;

use geist_edge::adapter::axum::{build_sequence, ProxyConfig, ProxyState};
use geist_edge::prelude::*;

// ─── Test Helpers ───────────────────────────────────────────────────────────

/// Start a mock upstream that returns 200 with an identifying header.
async fn start_upstream() -> SocketAddr {
    let app = Router::new().fallback(|| async {
        (
            axum::http::StatusCode::OK,
            [("x-upstream", "mock"), ("content-type", "text/plain")],
            "upstream response",
        )
            .into_response()
    });

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    addr
}

/// Start the governed proxy with the given config.
async fn start_proxy(upstream_addr: SocketAddr) -> SocketAddr {
    let config = ProxyConfig {
        listen: "127.0.0.1:0".parse().unwrap(),
        upstream: format!("http://{}", upstream_addr),
        pipeline: vec![TypedConfig {
            type_url: "test.acl.v1".into(),
            config: serde_json::json!({
                "deny": [
                    { "reason": "Bash is restricted", "matches": [{ "tool_name": { "Exact": "Bash" } }] }
                ],
                "allow": [
                    { "matches": [{ "agent_id": { "Exact": "claude-main" } }] }
                ]
            }),
        }],
        failure_mode: Default::default(),
    };

    // Build registry with the ACL processor registered explicitly.
    let registry = ProcessorRegistryBuilder::new()
        .with::<geist_acl::AccessControlProcessor>("test.acl.v1")
        .build();

    let sequence = build_sequence(&registry, &config).unwrap();

    let upstream_uri: hyper::Uri = config.upstream.parse().unwrap();
    let client =
        hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
            .build_http();

    let state = Arc::new(ProxyState {
        sequence,
        upstream: upstream_uri,
        client,
    });

    let app = Router::new()
        .fallback(geist_edge::adapter::axum::proxy_handler)
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    addr
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn allowed_request_forwarded_to_upstream() {
    let upstream = start_upstream().await;
    let proxy = start_proxy(upstream).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/api/test", proxy))
        .header("x-geist-agent-id", "claude-main")
        .header("x-geist-tool-name", "Read")
        .header("x-geist-resource", "/src/lib.rs")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers().get("x-upstream").unwrap(), "mock");

    let body = resp.text().await.unwrap();
    assert_eq!(body, "upstream response");
}

#[tokio::test]
async fn denied_request_returns_403() {
    let upstream = start_upstream().await;
    let proxy = start_proxy(upstream).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/api/test", proxy))
        .header("x-geist-agent-id", "claude-main")
        .header("x-geist-tool-name", "Bash")
        .header("x-geist-resource", "/")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 403);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "access_denied");
    assert!(body["reason"].as_str().unwrap().contains("Bash"));
}

#[tokio::test]
async fn unknown_agent_denied_by_default() {
    let upstream = start_upstream().await;
    let proxy = start_proxy(upstream).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/api/test", proxy))
        .header("x-geist-agent-id", "rogue-agent")
        .header("x-geist-tool-name", "Read")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn request_without_agent_headers_denied() {
    let upstream = start_upstream().await;
    let proxy = start_proxy(upstream).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/test", proxy))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 403);
}
