# Production Proxy Patterns — Beyond axum+hyper+tower

> Patterns from pingora, tokio, tonic, bytes, and rustls that matter when
> building a production reverse proxy. Complements proxy-patterns.md (M1 adapter)
> and middleware-gateway.md (tower composition). Focused on what Claude gets wrong
> when reasoning about proxy engineering at scale.
>
> Sourced from `~/oss/research/*-mastery.md` extracts + `~/oss/pingora/` codebase.

---

## 1. Lifecycle Callbacks vs Composable Middleware

**The design spectrum:**

| Approach | Framework | Composability | Performance | When |
|----------|-----------|--------------|-------------|------|
| Composable Service | tower | Excellent (Layer) | Allocation per layer | Flexibility matters more than raw throughput |
| Filter chain + factory | Envoy | Good (typed extension) | Vtable per filter | Dynamic extensibility via xDS |
| Lifecycle callbacks | pingora | Limited (single trait) | Zero overhead | Ordering invariants, extreme scale |

pingora's `ProxyHttp` trait has 40+ optional callbacks with a user-defined `CTX` type threaded through all phases. Two required methods: `new_ctx()` and `upstream_peer()`. Framework drives the state machine — you can't get the ordering wrong.

**Why pingora chose callbacks over composition:**
- No allocation per layer (at 40M req/s, those nanoseconds compound)
- Framework enforces phase ordering (connect → request → upstream → cache → response → retry → logging)
- CTX is shared across all callbacks — no threading context through middleware parameters

**The trade-off:** You can't compose two `ProxyHttp` implementations. If you need different proxy behaviors, run separate `Service<HttpProxy<T>>` instances.

**geist-edge's position:** Pipeline of processors via Sequence (typed extension registry). This is between tower and pingora — composable like tower, but with framework-driven ordering like pingora. The Sequence compositor enforces phase ordering; processors are independently extensible.

**[pingora-proxy/src/proxy_trait.rs; tower-mastery Insight 1; geist-edge Sequence design]**

---

## 2. H1 vs H2 Body Asymmetry

**What Claude gets wrong:** Treats HTTP/1.1 and HTTP/2 body streaming as equivalent. They are not.

| Behavior | H1 | H2 |
|----------|----|----|
| Body pipe | Bounded, blocks sender | **Unbounded, never blocks** |
| Writing 0 bytes | **Finishes the stream** (chunked `0\r\n\r\n`) | Noop — skip if `data.is_empty() && !end` |
| Connection reuse | Pool by keep-alive | Persistent at protocol level (multiplexed) |
| Duplex streaming | Sequential | `tokio::try_join!` with mpsc channels |

**The 0-byte trap:** If a body filter produces empty intermediate chunks, writing them to H1 terminates the stream. H2 ignores them. This is not an edge case — it's a guaranteed correctness bug.

**Backpressure asymmetry:** H2 body pipes are unbounded. A fast upstream can flood the proxy without backpressure. H1 bodies use bounded channels that physically block the sender.

**For geist-edge:** When the axum adapter forwards bodies, it must handle both protocols correctly. If body processing is added (post-M1), body filters must check `data.is_empty() && !end` before writing.

**[pingora-proxy/src/proxy_h1.rs, proxy_h2.rs; hyper-mastery Insight 3]**

---

## 3. Connection Pooling by Full Identity

**What Claude gets wrong:** Pools by `(host, port)`. Correct: pool by full Peer identity.

Two connections are reusable ONLY when they share the exact same identity:

```
hash(address, scheme, sni, client_cert, verify_cert, verify_hostname, alternative_cn, proxy)
```

**Pool architecture (pingora):**
- **Hot pool**: Lock-free, one slot per hash, CAS to take connection (fast path)
- **Cold pool**: Mutex-locked overflow (slow path)
- **H2 is NOT pooled**: Multiplexes over persistent connection — nothing to pool

**For geist-edge:** When proxying to Anthropic API (single upstream), `hyper_util::Client` pooling by `(scheme, authority)` is sufficient. When proxying to multiple upstreams (future), adopt the full Peer identity pattern including TLS context.

**[pingora-core/src/upstreams/peer.rs; hyper-util pooling]**

---

## 4. Retry Classification — Transport vs Semantic

**What Claude gets wrong:** Retries all errors uniformly, or doesn't retry at all.

| Error | Retry-able? | Why |
|-------|-------------|-----|
| Connection refused | Yes | Server never saw the request |
| Connection reset | Yes | Server may not have processed |
| H2 REFUSED_STREAM | Yes | Server explicitly rejected without processing |
| H2 GOAWAY | Yes | Server shutting down gracefully |
| Connect timeout | Yes | Server never saw the request |
| **Read timeout** | **No** | Server accepted, may have side effects |
| **HTTP 4xx/5xx** | **No** | Upstream's semantic decision |
| **Body send failure** | **No** | Partial request may have been processed |

**Retry buffer:** Request bodies must be buffered for retry. Bound the buffer — streaming large bodies and supporting retry are in tension. pingora's `retry_buffer_not_full()` callback lets you control the trade-off.

**For geist-edge:** The axum adapter should classify upstream errors and retry only transport failures. Read timeouts and HTTP errors are NOT retry-able.

**[pingora-proxy/src/proxy_common.rs; pingora retry loop architecture]**

---

## 5. Cooperative Scheduling Under Proxy Load

**What Claude gets wrong:** Assumes `now_or_never()` reliably detects ready futures.

tokio's cooperative scheduling has a budget (128 operations). When the budget is exhausted, a future voluntarily yields even if its value IS ready. If you call `.now_or_never()` on a future whose budget is consumed, it returns `None` despite being completable.

**The fix:**
```rust
// WRONG: budget can lie to you
let result = future.now_or_never();

// RIGHT: bypass budget for conditional polling
tokio::task::unconstrained(async { future.now_or_never() }).await
```

**Why proxies care:** Hot-path optimizations use `now_or_never()` to avoid allocation when data is already buffered. Cooperative scheduling silently defeats this, causing unnecessary slow-path fallback.

**Broader principle:** Proxy processors must respect the budget. A policy processor doing CPU-intensive work (regex matching, crypto verification) should yield periodically via `tokio::task::yield_now()` to avoid starving other connections.

**[pingora `unconstrained` fix; tokio-mastery Insight on scheduling trilemma]**

---

## 6. Verification Markers as Compile-Time Proofs

**What Claude gets wrong:** Uses boolean flags for security-critical state. Flags can be set incorrectly.

rustls uses zero-sized marker types that can only be constructed by verification functions:

```rust
// Zero-sized, private constructor — unforgeable
pub struct PolicyEvaluated(());

impl PolicyEvaluated {
    // Only the policy engine can create this
    pub(crate) fn assertion() -> Self { Self(()) }
}

// Request can only reach backend with proof of evaluation
struct GovernedRequest {
    request: ProcessingRequest,
    proof: PolicyEvaluated,  // compile-time guarantee
}
```

**For geist-edge:** When the Sequence compositor produces `SequenceOutcome::Continue`, the proof of evaluation is structural — the request went through all processors. But if processors are optional or can be bypassed, verification markers prevent accidental skip.

**[rustls-mastery: HandshakeSignatureValid, PeerVerified markers]**

---

## 7. Graduated Escape Hatches

**What Claude gets wrong:** Provides `skip_policy: bool` as a convenience flag.

rustls requires calling `.dangerous()`, implementing a full trait, and having "dangerous" in the type name, module name, and documentation to disable certificate verification.

**For geist-edge:** If policy bypass is ever needed (testing, debugging):

```rust
impl ProcessorBuilder {
    /// You probably don't want this. See docs for why.
    pub fn dangerous(&self) -> DangerousProcessorBuilder {
        DangerousProcessorBuilder(self)
    }
}

impl DangerousProcessorBuilder {
    pub fn skip_policy_evaluation(self) -> Ungoverned {
        // Visibly named, visibly typed, visibly documented
    }
}
```

Make the insecure path inconvenient, not impossible.

**[rustls-mastery: graduated escape hatch pattern]**

---

## 8. Sequence Limits as DoS Protection

**What Claude gets wrong:** Trusts that connections are finite. They're not — a single persistent connection can issue unlimited requests.

rustls tracks sequence numbers and triggers key rotation before exhaustion. For proxies, track requests per connection:

```rust
const REQUESTS_SOFT_LIMIT: u64 = u64::MAX - 0xffff;
// At soft limit → send GOAWAY, drain gracefully
// At hard limit → refuse, close connection
```

**For geist-edge:** Track request count per client connection. At soft limit, stop accepting new requests on that connection. At hard limit, close. This prevents a single client from monopolizing proxy resources.

**[rustls-mastery: BoringSSL sequence limit defense-in-depth]**

---

## 9. Zero-Copy Through the Pipeline

**What Claude gets wrong:** Converts `Bytes` to `Vec<u8>` for intermediate processing, then back.

bytes achieves zero-copy by design: `split()` is O(1), `freeze()` is O(1), `clone()` is O(1). All are refcount bumps and pointer adjustments.

**Rules for geist-edge:**
- Response bodies flow as `Bytes`, never `Vec<u8>`
- Frame boundaries use `split()`, not copying
- Cached responses shared via `clone()` (refcount, not memcpy)
- Buffer traits (`Buf`/`BufMut`) are infallible — check `remaining()` upfront, no `Result` in the hot path

**The promotable pattern:** bytes doesn't allocate refcount until the first clone. Most responses go to a single client (no sharing). The allocation only happens when responses are actually cached/shared.

**[bytes-mastery: Insight on promotable pattern, zero-copy design]**

---

## 10. Per-Connection Processor Stacks

**What Claude gets wrong:** Shares a single processor pipeline across all connections.

tonic instantiates a fresh middleware stack per TCP connection. This is how per-connection rate limits work — the limit is per-connection, not global.

**For geist-edge:** The `Arc<Sequence>` pipeline is shared (processors are stateless, `&self`). But per-connection state (rate limit counters, request count, session ID) should be instantiated per connection via Metadata, not stored globally.

**[tonic-mastery: per-connection service stack pattern]**

---

## 11. Metadata-Body Separation (tonic Interceptor Pattern)

**What Claude gets wrong:** Passes full request to every processor, including body.

tonic interceptors receive `Request<()>` — body stripped. This is a constraint, not a limitation. It prevents metadata processors (auth, policy) from accidentally modifying binary-framed data.

**For geist-edge:** This is already the design. `ProcessingRequest` with `request_headers` phase gives processors headers only. Body processing is a separate phase (`request_body`), opted into explicitly. The ext_proc model enforces this separation.

**[tonic-mastery: interceptor body stripping]**

---

## 12. Hot Restart via FD Passing

**Not needed for M1-M3 (Tauri-embedded). Relevant for standalone daemon mode.**

pingora's Server handles hot restart by passing listening socket FDs to the new process via Unix domain socket (`sendmsg` with `SCM_RIGHTS`). The `ProxyHttp` implementation is completely unaware — only a `ShutdownWatch` channel coordinates.

**For geist-edge standalone mode (future):** When running as a daemon (not embedded in Tauri), adopt this pattern. The operational lifecycle (start/stop/restart/upgrade) must be fully decoupled from the application lifecycle (request/response/error).

**[pingora-core/src/server/mod.rs; docs/user_guide/internals.md]**

---

## 13. Memory Ordering — Don't Optimize

**What Claude gets wrong:** Weakens memory ordering for performance.

tokio had a bug where weakening park/unpark from `SeqCst` to `Relaxed` caused lost wakeups on ARM. The fix was to restore `SeqCst`.

**Rule for geist-edge:** Use `SeqCst` for all synchronization primitives in the proxy. The cost is negligible compared to network I/O. Don't optimize memory ordering without formal proof.

**[tokio-mastery: Insight on memory ordering oscillation, commit #525]**

---

## 14. State Machine as Types, Not Enum

rustls models handshake states as distinct types. Transitions consume `self: Box<Self>` and produce the next state type. Invalid transitions are unrepresentable.

**For geist-edge connection lifecycle:**

```rust
// Each state only exposes valid operations
struct Accepting;           // Can: accept()
struct ProcessingHeaders;   // Can: run pipeline, forward, reject
struct AwaitingUpstream;    // Can: receive response, timeout
struct SendingResponse;     // Can: write response, apply mutations

trait ConnectionState {
    fn handle(self: Box<Self>, event: Event) -> Result<Box<dyn ConnectionState>>;
}
```

**When to apply:** Connection-level state machines benefit from typed states. Request-level processing (the Sequence pipeline) already has structural ordering via phases — typed states would over-engineer it.

**[rustls-mastery: state machine as types pattern]**

---

## 15. pingora as L1 Adapter Candidate

geist-edge's three-layer model makes L1 swappable. pingora is a candidate alongside axum:

| Dimension | axum (M1) | pingora (future) |
|-----------|-----------|-------------------|
| Architecture | Tower middleware | Lifecycle callbacks (40+ methods) |
| Proxy features | Build yourself | Built-in (pool, retry, cache, H2, hot restart) |
| Composability | Excellent (Layer) | Limited (single ProxyHttp impl) |
| Complexity | Low (1 crate) | Medium (21 crates) |
| Hot restart | Not built-in | Built-in (FD passing) |
| When | Tauri-embedded, M1-M3 | Standalone daemon, high-scale deployment |

**The CTX ↔ Metadata analogy:** pingora's per-request `type CTX` (user-defined, typed) maps to geist-edge's `Metadata` (type-erased, shared across processors). Different trade-offs: CTX gives type safety within one impl; Metadata gives extensibility across a pipeline of independent processors.

**[~/oss/pingora/ full codebase analysis]**

---

## Source Traceability

| Pattern | Source Extract | Insight # |
|---------|---------------|-----------|
| Lifecycle callbacks | pingora-mastery | 1 |
| H1/H2 asymmetry | pingora-mastery | 2 |
| Peer identity pooling | pingora-mastery | 5 |
| Retry classification | pingora-mastery | 8 |
| Cooperative scheduling | tokio-mastery | Scheduling trilemma |
| Verification markers | rustls-mastery | HandshakeSignatureValid |
| Graduated escape hatches | rustls-mastery | `.dangerous()` pattern |
| Sequence limits | rustls-mastery | BoringSSL defense-in-depth |
| Zero-copy pipeline | bytes-mastery | Promotable pattern |
| Per-connection stacks | tonic-mastery | Per-connection Service |
| Metadata-body separation | tonic-mastery | Interceptor body stripping |
| Hot restart | pingora-mastery | 4 |
| Memory ordering | tokio-mastery | SeqCst restoration |
| State machine types | rustls-mastery | Typed state transitions |
