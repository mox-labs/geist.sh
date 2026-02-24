# Reverse Proxy Patterns — axum + hyper + tower Synthesis

> Patterns for building a governed reverse proxy with axum.
> Synthesized from axum-mastery, hyper-mastery, tower-mastery extracts.
> Focused on geist-edge's axum adapter (M1).

---

## 1. Connection Pooling and Keep-Alive

**The question:** How should the proxy maintain upstream connections?

Creating a new TCP connection per request adds ~1.8x latency overhead (measured in hyper benchmarks).
hyper 1.0 moved `Client` and connection pooling to `hyper-util` because maintaining concurrent
persistent connections is a separate concern from protocol correctness.

**Pattern:** Use `hyper_util::client::legacy::Client<HttpConnector>` with default pooling.
The client maintains a pool keyed by (scheme, authority). Keep-alive is on by default.
For a single upstream (geist-edge proxying to Anthropic API), one `Arc<Client>` suffices.

```rust
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;

let client = Client::builder(TokioExecutor::new()).build_http();
// Share via Arc in axum state
```

**[hyper: Insight 11, Issue #3164; hyper-util extractive surgery]**

---

## 2. Hop-by-Hop Header Filtering

**The question:** Which headers must NOT be forwarded?

HTTP/1.1 defines hop-by-hop headers that are meaningful only for a single transport connection,
not for the end-to-end request. A proxy MUST NOT forward these:

- `Connection` (and any headers listed in its value)
- `Transfer-Encoding`
- `TE`
- `Trailer`
- `Upgrade`
- `Proxy-Authenticate`
- `Proxy-Authorization`
- `Keep-Alive`

**Pattern:** Filter before forwarding, both on request and response:

```rust
const HOP_BY_HOP: &[&str] = &[
    "connection", "transfer-encoding", "te", "trailer",
    "upgrade", "proxy-authenticate", "proxy-authorization", "keep-alive",
];

fn strip_hop_by_hop(headers: &mut HeaderMap) {
    for name in HOP_BY_HOP {
        headers.remove(*name);
    }
}
```

**For H1→H2 proxying:** Also translate `Transfer-Encoding: chunked` into H2 DATA frames.
**For H2→H1:** Translate frame boundaries into chunks.

**[RFC 7230 Section 6.1; hyper Content-Length semantics, Insight 8]**

---

## 3. Host/Authority Rewriting

**The question:** How should the proxy handle the Host header?

The incoming `Host` header reflects the client's view (e.g., `localhost:8080`). The upstream
expects its own authority (e.g., `api.anthropic.com`). The proxy must decide: forward the
original Host, or rewrite to the upstream's authority?

axum's Issue #2998 shows that `Host` is spoofable. The proxy is the trust boundary —
it attests to the upstream's identity.

**Pattern:** Rewrite `Host` (H1) or `:authority` (H2) to the upstream address:

```rust
fn rewrite_host(headers: &mut HeaderMap, upstream: &Uri) {
    if let Some(authority) = upstream.authority() {
        headers.insert(
            http::header::HOST,
            HeaderValue::from_str(authority.as_str()).unwrap(),
        );
    }
}
```

**[axum: Issue #2998, spoofable Host; hyper: Insight 8]**

---

## 4. axum State Sharing with Upstream Client

**The question:** How does the handler access the upstream client and pipeline?

axum's "Missing State" pattern (`Router<S>` means state S is missing, not present) combined
with hyper's `&self` wrapper algebra (`Arc<Client>` works because Service uses `&self`).

**Pattern:**

```rust
#[derive(Clone)]
struct AppState {
    pipeline: Arc<Sequence>,
    client: Client<HttpConnector, Body>,
    upstream: Uri,
}

async fn proxy_handler(
    State(state): State<AppState>,
    req: Request,
) -> Response {
    // state.pipeline, state.client, state.upstream all available
}

let app = Router::new()
    .fallback(proxy_handler)
    .with_state(state);
```

`State` implements `FromRequestParts`, so it extracts without consuming the body.
The `Arc<Sequence>` is shared across all concurrent requests.

**[axum: Insight 4, FromRef pattern; hyper: PR #3607, &self algebra]**

---

## 5. Request Forwarding Architecture

**The question:** How should the proxy model the request/response lifecycle?

A proxy has two independent pipelines (reading from client, writing to upstream) that
must be composed. hyper models this as a product FSM (Reading × Writing).

For geist-edge, the lifecycle is linear for headers-only processing:

```
Client Request
  → extract headers → build ProcessingRequest(request_headers)
  → run Sequence pipeline
  → match SequenceOutcome {
      Respond(ir)    → return ImmediateResponse as HTTP response
      Error(e)       → return 500
      Continue(muts) → apply mutations to headers
    }
  → strip hop-by-hop headers
  → rewrite Host
  → forward to upstream via Client
  → receive upstream response
  → extract response headers → build ProcessingRequest(response_headers)
  → run Sequence pipeline (same branching as above)
  → apply response mutations
  → return to client
```

For body processing (when aggregate mode opts in):
```
  → buffer request body
  → build ProcessingRequest(request_body)
  → run Sequence pipeline
  → ... same pattern ...
```

**[hyper: Insight 4, product FSM; axum: Insight 1, Parts/Body split]**

---

## 6. Body Streaming Without Buffering

**The question:** How should the proxy handle request/response bodies?

For geist-edge M1 (headers-only), bodies are forwarded as-is without inspection.
The proxy streams the body between client and upstream without buffering.

hyper's zero-capacity channel (`mpsc::channel(0)`) ensures physical backpressure:
the sender cannot push data until the receiver polls. This prevents a fast client
from filling proxy memory.

**Pattern:** Forward the body directly:

```rust
// Request body: pass through from client to upstream
let upstream_req = Request::builder()
    .method(req.method().clone())
    .uri(upstream_uri)
    .body(req.into_body())  // stream through, no buffering
    .unwrap();
```

When body processing IS needed (aggregate mode has `request_body: true`),
buffer the entire body first:

```rust
let body_bytes = axum::body::to_bytes(req.into_body(), MAX_BODY_SIZE).await?;
let outcome = sequence.process_request_body(&body_bytes).await;
```

**[hyper: Insight 3, zero-capacity channels; body/incoming.rs:114-137]**

---

## 7. Error Handling: Upstream Failures

**The question:** How should the proxy handle upstream errors?

Following axum's philosophy: errors ARE responses, not middleware failures.
A rate-limit rejection is 429, an auth failure is 403, an upstream timeout is 504.

| Upstream Condition | HTTP Response |
|-------------------|--------------|
| Connection refused | 502 Bad Gateway |
| Timeout | 504 Gateway Timeout |
| 5xx from upstream | Forward as-is (upstream's response) |
| Pipeline Error | 500 Internal Server Error |
| Pipeline Respond | Return ImmediateResponse directly |

Track timeout provenance (hyper Insight 19): default timeouts degrade gracefully (warn),
explicit timeouts fail loudly.

```rust
match tokio::time::timeout(Duration::from_secs(30), client.request(req)).await {
    Ok(Ok(response)) => { /* run response pipeline */ }
    Ok(Err(e)) => {
        tracing::error!(error = %e, "upstream connection failed");
        Response::builder().status(502).body(Body::empty()).unwrap()
    }
    Err(_) => {
        tracing::warn!("upstream timeout");
        Response::builder().status(504).body(Body::empty()).unwrap()
    }
}
```

**[axum: Insight 5, Infallible philosophy; hyper: Insight 19, timeout provenance]**

---

## 8. Mutation Application Semantics

**The question:** In what order should HeaderMutations be applied?

From ext_proc specification: within a single `HeaderMutation`, `set_headers` are applied first
(add or replace), then `remove_headers` (delete). Multiple mutations from different processors
are applied in processor registration order (the Sequence guarantees this).

```rust
fn apply_mutations(headers: &mut http::HeaderMap, mutations: Vec<HeaderMutation>) {
    for mutation in mutations {
        // Set first
        for header_opt in &mutation.set_headers {
            if let Some(hv) = &header_opt.header {
                if let Ok(name) = HeaderName::from_bytes(hv.key.as_bytes()) {
                    if let Ok(value) = HeaderValue::from_str(&hv.value) {
                        headers.insert(name, value);
                    }
                }
            }
        }
        // Then remove
        for name in &mutation.remove_headers {
            headers.remove(name.as_str());
        }
    }
}
```

**[ext_proc specification; geist-edge Sequence compositor guarantees ordering]**

---

## 9. ImmediateResponse to HTTP Response

**The question:** How to translate ext_proc ImmediateResponse to axum Response?

`ImmediateResponse` has: `status` (HttpStatus), `headers` (HeaderMutation), `body` (bytes),
`grpc_status` (optional). For HTTP, we use status + headers + body.

```rust
fn immediate_to_response(ir: &ImmediateResponse) -> Response {
    let status = ir.status
        .as_ref()
        .and_then(|s| StatusCode::from_u16(s.code as u16).ok())
        .unwrap_or(StatusCode::FORBIDDEN);

    let mut response = Response::builder().status(status);

    // Apply headers from ImmediateResponse
    if let Some(ref mutation) = ir.headers {
        for hvo in &mutation.set_headers {
            if let Some(ref hv) = hvo.header {
                if let (Ok(name), Ok(value)) = (
                    HeaderName::from_bytes(hv.key.as_bytes()),
                    HeaderValue::from_str(&hv.value),
                ) {
                    response = response.header(name, value);
                }
            }
        }
    }

    response.body(Body::from(ir.body.clone())).unwrap()
}
```

**[Dijkstra I1: response variant must match request phase — PhaseResult abstracts this]**

---

## 10. ProcessingRequest Construction

**The question:** How to build an HttpMessage from an axum Request?

`HttpMessage` is an indexed O(1) view over a `ProcessingRequest` envelope.
Build the envelope from the incoming request's headers:

```rust
use geist_edge::prelude::*;

fn request_to_processing(req: &Request) -> ProcessingRequest {
    let mut headers = vec![];
    for (name, value) in req.headers() {
        headers.push(HeaderValueOption {
            header: Some(HeaderValue {
                key: name.as_str().to_string(),
                value: value.to_str().unwrap_or("").to_string(),
                raw_value: vec![],
            }),
            ..Default::default()
        });
    }

    // Add pseudo-headers (method, path, scheme, authority)
    let pseudo_headers = vec![
        (":method", req.method().as_str()),
        (":path", req.uri().path()),
        (":scheme", req.uri().scheme_str().unwrap_or("http")),
        (":authority", req.uri().authority().map(|a| a.as_str()).unwrap_or("")),
    ];
    for (key, value) in pseudo_headers {
        headers.push(HeaderValueOption {
            header: Some(HeaderValue {
                key: key.to_string(),
                value: value.to_string(),
                raw_value: vec![],
            }),
            ..Default::default()
        });
    }

    ProcessingRequest {
        request: Some(Request::RequestHeaders(HttpHeaders {
            headers: Some(HeaderMap { headers }),
            end_of_stream: false, // Set based on Content-Length
            ..Default::default()
        })),
        ..Default::default()
    }
}
```

**[rumi-http HttpMessage; ext_proc ProcessingRequest schema]**

---

## 11. Graceful Shutdown

**The question:** How should the proxy shut down cleanly?

tokio's dual close bit pattern: (1) stop accepting new connections, (2) drain in-flight requests.

```rust
let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(());

let server = axum::serve(listener, app)
    .with_graceful_shutdown(async move {
        shutdown_rx.changed().await.ok();
    });

// On SIGTERM/SIGINT:
// shutdown_tx.send(()).ok();
// Server drains in-flight requests, then exits
```

CancellationToken is preferred over Drop-based cancellation (tokio Insight 15):
`mem::forget` is safe in Rust, so Drop-based cleanup is unsound in async contexts.

**[tokio: Insight 8, dual close bits; Insight 15, CancellationToken]**

---

## Source Traceability

| Concern | axum | hyper | tower | ext_proc |
|---------|------|-------|-------|----------|
| State sharing | Insight 4 (Missing State) | PR #3607 (&self) | — | — |
| Body handling | Insight 1 (Parts/Body) | Insight 3 (channel(0)) | — | ProcessingMode |
| Error handling | Insight 5 (Infallible) | Insight 19 (provenance) | Issue #131 | ProcessorError |
| Type erasure | Insight 6 (BoxedIntoRoute) | — | BoxService | — |
| Hop-by-hop | Issue #2998 (Host) | Insight 8 (C-L) | — | HeaderMutation |
| Shutdown | — | — | — | — (tokio Insight 8, 15) |
