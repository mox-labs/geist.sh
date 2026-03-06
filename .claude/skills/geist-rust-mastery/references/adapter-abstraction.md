# ACES Adapter Abstraction — Runtime Swapping for geist-edge

> How to design geist-edge so axum/pingora/ext_proc are interchangeable.
> Synthesized from axum-mastery, pingora-mastery, tower-mastery, hyper-mastery, tonic-mastery.

---

## The Core Insight

All three target runtimes converge on `http::` types:

| Runtime | Request Access | How |
|---------|---------------|-----|
| **axum** | `http::Request<Body>` → `into_parts()` → `(Parts, Body)` | Native — zero copy |
| **pingora** | `Session::req_header()` → `&RequestHeader` → wraps `http::request::Parts` | Zero copy |
| **ext_proc** | `ProcessingRequest` (protobuf) → deserialize → construct `Parts` | Serialization at boundary |

**Design principle:** Processors take `http::` types. Adapters provide them. Serialization happens only
at wire boundaries (ext_proc), never for inline processing.

---

## Processor Trait: http:: Types, Not Protos

```rust
// BEFORE (current — ext_proc types in hot path, triple header copying)
fn process_request_headers(&self, msg: &HttpMessage) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>>

// AFTER — http:: types, zero copy from axum/pingora
fn process_request_headers(&self, parts: &http::request::Parts) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>>
fn process_response_headers(&self, parts: &http::response::Parts) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>>
```

**Why `http::request::Parts`:**
- Contains `method`, `uri`, `version`, `headers`, `extensions` — everything processors need
- Type-system distinguishes request from response (currently both use `&HttpMessage`)
- `extensions` field replaces custom Metadata for inter-processor communication
- axum gives you `Parts` via `into_parts()` — zero allocation
- pingora's `RequestHeader` wraps `Parts` — zero allocation

**Why NOT a custom view type:**
- `http::request::Parts` is THE standard. Used by axum, hyper, tonic, reqwest, warp.
- Custom types force conversion. Standard types compose.
- "When a standard type exists, use it." — Boy Scout Rule applied to types.

---

## PhaseResult: http:: Types Too

```rust
// BEFORE — ext_proc HeaderMutation, ImmediateResponse (protobuf types)
pub enum PhaseResult {
    Continue,
    Mutate(HeaderMutation),            // ext_proc proto type
    Respond(ImmediateResponse),        // ext_proc proto type
}

// AFTER — http:: types
pub enum PhaseResult {
    Continue,
    Mutate(HeaderMutationSet),
    Respond(ImmediateResponse),
}

pub struct HeaderMutationSet {
    pub set: Vec<(http::HeaderName, http::HeaderValue)>,
    pub remove: Vec<http::HeaderName>,
}

pub struct ImmediateResponse {
    pub status: http::StatusCode,
    pub headers: http::HeaderMap,
    pub body: bytes::Bytes,
}
```

Mutation semantics stay the same (set first, then remove — ext_proc ordering). Types change
from protobuf to `http::`.

---

## Adapter Modules, Not Adapter Trait

Each adapter is a standalone module that wires the pipeline into its runtime. No shared
`Adapter` trait — the runtimes are too different for a useful common interface.

| Concern | axum adapter | pingora adapter | ext_proc adapter |
|---------|-------------|-----------------|------------------|
| **Architecture** | Request handler | Lifecycle callbacks | gRPC bidi stream |
| **Pipeline call** | In handler fn | In `request_filter()` | In stream processor |
| **Concurrency** | tokio tasks (axum handles) | Framework-driven | Stream-per-request |
| **Body streaming** | `Body` pass-through or buffer | Body filter callbacks | Chunked proto messages |
| **Shutdown** | `axum::serve` + CancellationToken | `Server` hot restart | gRPC stream close |
| **Connection pooling** | hyper_util Client | Built-in Peer pool | N/A (sidecar) |

**Why no trait:** axum's adapter is a handler function. pingora's is a `ProxyHttp` impl. ext_proc's
is a tonic server. These have fundamentally different shapes. A common trait would be either too
abstract to be useful or too specific to accommodate all three.

Instead: **the pipeline IS the abstraction**. All adapters call the same `Sequence::process_*` methods.
The adapter translates its runtime's types into `http::` types and back.

**Evidence:**
- tonic decoupled protocol from transport (2 years, 5 commits) — feature-gate transport, not protocol
- pingora's `Server` has no notion of proxy — `HttpProxy<SV>` is one `ServerApp` implementor
- hyper 1.0's extractive surgery — moved Client/Server to hyper-util, kept protocol

---

## Feature Gates

```toml
[features]
default = []
adapter-axum = ["dep:axum", "dep:tower", "dep:hyper", "dep:hyper-util", "dep:tokio"]
adapter-pingora = ["dep:pingora-proxy", "dep:pingora-core"]
```

Core pipeline + registry + processors: zero HTTP runtime deps.
Adapters: feature-gated, bringing in only what they need.

---

## Deployment Models (ACES in Practice)

```
┌─────────────────────────────────────────────────────────────┐
│                    geist-edge (core)                         │
│  Processor trait (http:: types) → Sequence → PhaseResult     │
│  Registry, Extensions                                        │
└──────────────┬──────────────────┬───────────────┬───────────┘
               │                  │               │
        ┌──────┴──────┐   ┌──────┴──────┐  ┌─────┴──────┐
        │ axum adapter │   │   pingora   │  │  ext_proc  │
        │  (library)   │   │  adapter    │  │  adapter   │
        │              │   │ (standalone │  │ (sidecar)  │
        │ Embedded in  │   │   daemon)   │  │            │
        │ Tauri shell  │   │             │  │ geist-run  │
        └──────────────┘   └─────────────┘  └────────────┘
```

| Model | Adapter | When |
|-------|---------|------|
| **Library** | axum | Embedded in Tauri (geist-shell), Claude Code hook, single-agent |
| **Standalone proxy** | pingora | Multi-agent gateway, production deployment, hot restart needed |
| **Remote processor** | ext_proc | geist-run (agent orchestration), Envoy sidecar, distributed |

**Independent deployment:** geist-edge (with axum/pingora) and geist-run (with ext_proc) can
be deployed separately. geist-edge governs. geist-run orchestrates. They communicate via ext_proc
gRPC when remote, or share a pipeline when co-located.

---

## Adapter Patterns (from research)

### axum Adapter — Request Handler

```rust
// The handler is a catch-all fallback
async fn proxy_handler(
    State(state): State<AppState>,
    req: http::Request<Body>,
) -> http::Response<Body> {
    let (parts, body) = req.into_parts();  // Zero copy

    // Run pipeline on request headers
    match state.pipeline.process_request_headers(&parts).await {
        SequenceOutcome::Respond(ir) => ir.into_response(),
        SequenceOutcome::Error(e) => e.into_response(),
        SequenceOutcome::Continue(mutations) => {
            let mut headers = parts.headers.clone();  // Clone only when mutating
            apply_mutations(&mut headers, &mutations);
            // Forward to upstream...
        }
    }
}
```

**Key patterns from research:**
- `State(AppState)` via `FromRequestParts` — no body consumption (axum Insight 4)
- `req.into_parts()` — zero-copy extraction (axum Insight 1)
- Body type erased at boundary (axum Insight 3: `req.map(Body::new)`)
- `Arc<Sequence>` shared across concurrent requests (hyper `&self` algebra)
- Errors ARE responses (axum Insight 5: Infallible architecture)

### pingora Adapter — ProxyHttp Lifecycle

```rust
impl ProxyHttp for GeistProxy {
    type CTX = GeistContext;

    fn new_ctx(&self) -> Self::CTX { GeistContext::new() }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        let parts = session.req_header().as_parts();

        match self.pipeline.process_request_headers(&parts).await {
            SequenceOutcome::Respond(ir) => {
                session.write_response_header(ir.into_header()).await?;
                session.write_response_body(Some(ir.body.into()), true).await?;
                Ok(true)  // skip upstream
            }
            SequenceOutcome::Continue(mutations) => {
                apply_mutations(session.req_header_mut(), &mutations);
                Ok(false)  // continue to upstream
            }
            // ...
        }
    }

    async fn upstream_peer(&self, _session: &mut Session, _ctx: &mut Self::CTX) -> Result<Box<HttpPeer>> {
        // Return configured upstream
    }
}
```

**Key patterns from research:**
- Lifecycle callbacks, not middleware (pingora Insight 1)
- `Session` owns the connection; `CTX` is per-request state (pingora core)
- `request_filter` returning `true` = skip upstream (like `PhaseResult::Respond`)
- H1/H2 asymmetry surfaced, not hidden (pingora Insight 2)
- Built-in connection pooling via `HttpPeer` identity (pingora Insight 5)
- Hot restart via `Server` — your code is unaware (pingora Insight 4)

### ext_proc Adapter — Wire Protocol

ext_proc is the wire format between geist-edge and geist-run, NOT the internal type:

```rust
// Receive ProcessingRequest from proxy/agent
// Convert to http:: types at boundary
// Process through pipeline
// Convert result back to ProcessingResponse
// Send back

async fn process_stream(stream: tonic::Streaming<ProcessingRequest>) -> ... {
    while let Some(req) = stream.message().await? {
        // Deserialize at boundary (this is the ONLY place ext_proc types live)
        let parts = proto_to_parts(&req);
        let outcome = pipeline.process_request_headers(&parts).await;
        let response = outcome_to_proto(&outcome);
        tx.send(response).await?;
    }
}
```

**Key pattern:** ext_proc types at the wire boundary only. Never in the pipeline.

---

## What This Eliminates

The 65% CPU problem (triple header copying) disappears:

| Step | Before | After |
|------|--------|-------|
| axum → pipeline | `http::Request` → `ProcessingRequest` → `HttpMessage` | `http::Request` → `into_parts()` |
| Header lookup | `msg.header()` re-lowercases on every call | `parts.headers.get()` — O(1) |
| Mutation | `HeaderMutation` (protobuf) | `HeaderMutationSet` (http:: types) |
| Allocations (40 headers, 5K rps) | ~1.2M String/sec | ~0 (zero copy from axum) |

---

## Decision Record

**Why http::request::Parts over custom view type:**
- Standard type used across the Rust HTTP ecosystem
- Zero conversion from axum and pingora
- Type-distinguishes request vs response (currently both `&HttpMessage`)
- `extensions` field provides inter-processor communication
- Forward-compatible with any HTTP framework that uses the `http` crate

**Why adapter modules over adapter trait:**
- Three runtimes have fundamentally different shapes (handler, lifecycle, stream)
- A common trait would be either too abstract or too specific (tonic transport decoupling lesson)
- The pipeline IS the abstraction — adapters are glue code, not a polymorphic boundary

**Why feature gates over workspace crates:**
- Keeps the build simple for the common case (axum only)
- Prevents unused dependency compilation
- Same pattern as geist-edge's existing `adapter-axum` feature declaration

---

## Source Traceability

| Design Decision | Sources |
|----------------|---------|
| `http::` types as convergence point | axum Insight 1 (Parts split), pingora `RequestHeader` wraps Parts |
| No adapter trait | tonic Insight 10 (transport decoupling, 2 years), pingora Insight 4 (Server ≠ proxy) |
| Feature-gated adapters | tonic transport feature gate, geist-edge existing Cargo.toml |
| Mutation model (set then remove) | ext_proc specification, preserved semantics |
| `&self` Processor + `Arc<Sequence>` | hyper PR #3607, tower Insight 1 (`&mut self` kills sharing) |
| Body stream-through vs buffer | hyper Insight 3 (channel(0) backpressure), axum Insight 1 |
| CancellationToken for shutdown | tokio Insight 15 (Drop unsound in async) |
| Connection pooling via hyper_util | hyper Insight 11, pingora Insight 5 (Peer identity) |
