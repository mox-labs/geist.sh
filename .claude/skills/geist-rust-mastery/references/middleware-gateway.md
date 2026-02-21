# x.uma Microgateway — Middleware & Protocol Reference

> Synthesized from tower, axum, and hyper mastery extracts.
> Organized by architectural concern for gateway design judgment.

---

## Table of Contents

1. [Service Trait Composition](#1-service-trait-composition)
2. [Backpressure and Readiness](#2-backpressure-and-readiness)
3. [Connection State Machines](#3-connection-state-machines)
4. [Request/Response Extraction](#4-requestresponse-extraction)
5. [Error Handling Across Middleware Layers](#5-error-handling-across-middleware-layers)
6. [Compile-Time vs Runtime Type Trade-offs](#6-compile-time-vs-runtime-type-trade-offs)
7. [Body Handling and Backpressure](#7-body-handling-and-backpressure)

---

## 1. Service Trait Composition

**The question for x.uma:** How do policy checks compose as middleware in a tower-compatible stack?

### The Two Service Traits

tower and hyper settled on different Service signatures after years of oscillation:

| Crate | Signature | poll_ready | Rationale |
|-------|-----------|------------|-----------|
| tower | `call(&mut self, req)` | Required | Reservation protocol for backpressure |
| hyper 1.0 | `call(&self, req)` | Dropped | Honest about interior mutability; enables wrapper algebra |

hyper's resolution: if you need shared state, you already have `Arc<Mutex<_>>`. Pretending `&mut self` gives exclusive access is dishonest. The sealed `HttpService` keeps `&mut self` internally; the public `Service` uses `&self` and blanket-implements `HttpService`. **[hyper: service/service.rs:32-57, service/http.rs:20-54]**

### Layer = Configuration Carrier

tower's `Layer` trait separates configuration from instantiation. Layers hold config; services hold state. This enables reusable middleware stacks:

```rust
let stack = ServiceBuilder::new()
    .layer(TimeoutLayer::new(Duration::from_secs(30)))
    .layer(RateLimitLayer::new(100, Duration::from_secs(1)))
    .layer(ConcurrencyLimitLayer::new(10));
let service = stack.service(inner);
```

The same config can wrap different inner services. **[tower: Layer trait, tower-layer/src/lib.rs]**

### The Clone Trap in Middleware

When a middleware clones the inner service to move it into a future, readiness is NOT preserved. The correct pattern:

```rust
fn call(&mut self, req: R) -> Self::Future {
    let clone = self.inner.clone();
    let mut inner = std::mem::replace(&mut self.inner, clone);
    // `inner` IS ready (it was poll_ready'd). The clone left behind is fresh.
    Box::pin(async move { inner.call(req).await })
}
```

**[tower: Insight 3, "Clone does not preserve invisible state"]**

### The &self Wrapper Algebra

hyper's `&self` decision yields blanket impls for `&S`, `&mut S`, `Box<S>`, `Rc<S>`, `Arc<S>` — the full algebra of standard wrapper types. tower's `&mut self` blocks `Arc<S>` and `Rc<S>` (shared pointers cannot provide `&mut`). **[hyper: PR #3607]**

### Transferable Principle for x.uma

x.uma policy middleware should use `&self` on its core evaluation trait (like hyper's Service) so that `Arc<PolicyEngine>` works naturally. Use `Layer` for policy configuration (rate limits, auth config) and `Service` for policy state. If x.uma needs tower compatibility at the boundary, the hyper two-layer pattern (public `&self` trait blanket-implementing internal `&mut self` trait) bridges the gap.

---

## 2. Backpressure and Readiness

**The question for x.uma:** Should policy evaluation participate in backpressure? How?

### poll_ready as Reservation Protocol

tower's `poll_ready` returning `Ready(Ok(()))` is a reservation. The caller MUST call `call` or drop the service. This is a social contract, not compiler-enforced. Services MAY panic if `call` is invoked without prior `poll_ready`. **[tower: Insight 2, service trait docs lines 346-350]**

### Readiness as Decision Input

`poll_ready` is not just a gate — it is a decision input. tower's load balancer selects only from ready replicas. A simpler `fn ready(&self) -> bool` was rejected because `poll_ready` performs the work necessary to become ready (e.g., acquires a semaphore permit). Checking and acquiring are atomic — splitting them creates TOCTOU races. **[tower: Issue #626, 23 comments]**

### The O(n) Readiness Problem

At scale, polling all inner services synchronously in `poll_ready` is O(n). Linkerd hit this with 30+ endpoints. Fix: spawn each inner service as its own task. The executor handles scheduling. **[tower: Issue #286]**

### hyper's Resolution: Drop It

hyper 1.0 dropped `poll_ready` entirely. The rationale: the obligation was routinely violated in practice (tower's own Buffer tests violated it — [tower: PR #476]), and honest interior mutability makes the reservation concept unnecessary for the common case.

### Transferable Principle for x.uma

Policy checks (auth, rate limiting) are typically stateless or use shared state behind `Arc<Mutex<_>>`. The `poll_ready` reservation protocol adds complexity without proportional benefit for policy middleware. Follow hyper: drop `poll_ready` from x.uma's core policy trait. If x.uma needs backpressure for specific policy checks (e.g., external auth service calls), use tower's `Buffer` at that specific layer rather than baking readiness into the core trait.

---

## 3. Connection State Machines

**The question for x.uma:** How should x.uma model connection-level protocol state?

### Product State Machines

hyper models HTTP/1.1 connections as the product of two independent FSMs:

- `Reading`: Init | Continue(Decoder) | Body(Decoder) | KeepAlive | Closed
- `Writing`: Init | Body(Encoder) | KeepAlive | Closed
- `KA`: Idle | Busy | Disabled

5 x 4 x 3 = 60 theoretical states, most invalid. Guards enforce valid transitions (`can_read_head()`, `can_write_body()`, etc.). The product of two independent FSMs is more expressive than a single combined FSM because reading and writing genuinely are independent. **[hyper: conn.rs:38-89, 963-976]**

### State Machines Beat Futures for Protocol Code

hyper benchmarked ~40% performance improvement with hand-written state machines vs futures-based implementation. State machines compile to direct match statements; futures require vtable dispatch and heap allocation. Resolution: state machines for protocol core, futures at the API boundary. **[hyper: Issue #395, 127 comments]**

### Client/Server Symmetry via Dispatch Trait

Both client and server implement the same `Dispatch` trait with inverted associated types:
- Server: sends `StatusCode`, receives `RequestHead`
- Client: sends `RequestHead`, receives `ResponseHead`

The asymmetry is captured by `T::should_read_first()`, not by different implementations. **[hyper: dispatch.rs:30-43]**

### Fairness via Iteration Counting

The Dispatcher processes at most 16 iterations of read-write-flush before yielding. Counting loop iterations is the cheapest possible fairness mechanism — zero syscall overhead vs timer-based approaches. **[hyper: dispatch.rs:165-193]**

### Transferable Principle for x.uma

x.uma's gateway proxy should model connection state as a product FSM (reading x writing x policy-state). Use hand-written state machines for the protocol-critical hot path (request routing, policy enforcement), futures at the API boundary. For multi-stream connections (HTTP/2), use iteration counting for fairness between streams rather than timers.

---

## 4. Request/Response Extraction

**The question for x.uma:** How should x.uma extract policy-relevant data from requests?

### The FromRequestParts / FromRequest Split

axum's core insight: HTTP bodies can only be consumed once, and this should be a type error, not a runtime panic.

- `FromRequestParts<S>`: extracts from headers, URI, state (shared access, multiple extractors)
- `FromRequest<S, M>`: consumes the body (only the LAST extractor may do this)

The handler macro enforces this at compile time — all arguments except the last must implement `FromRequestParts`. **[axum: Insight 1, axum-core/src/extract/mod.rs]**

### Phantom Type Coherence Workaround

`FromRequest<S, M = private::ViaRequest>` uses phantom marker `M` to prevent blanket impl conflicts. When a type implements `FromRequestParts`, a blanket impl provides `FromRequest<S, ViaParts>`. The marker resolves coherence without user-visible complexity. **[axum: Insight 2, axum/src/handler/mod.rs:129-144]**

### Missing State as Type Tracking

`Router<S>` means a router MISSING state of type `S`, not one that HAS it. Only `Router<()>` (nothing missing) can become a `Service`. This replaced runtime `Extension` panics with compile-time state extraction via `FromRef<T>`. **[axum: Insight 4, PR #1532]**

### Security-Aware Extractors

Spoofable header values (`Host`, `Scheme`) should force acknowledgment at the call site. Design options explored: `SpoofableValue` wrapper, `Spoofable<E>` wrapper with private trait, parameterized trust levels. The principle: make security implications visible, don't hide or block them. **[axum: Issue #2998]**

### Transferable Principle for x.uma

x.uma policy extractors should follow the Parts/Body split. Policy-relevant metadata (headers, path, method, source IP) extracts from request parts without consuming the body. Body inspection (payload scanning, content validation) consumes the body and must be the terminal extraction step. Use phantom types to enforce this ordering at compile time. Track "what policy data is still owed" as a type parameter (the Missing State pattern).

---

## 5. Error Handling Across Middleware Layers

**The question for x.uma:** How should policy enforcement errors compose across middleware?

### The Fundamental Tension

tower's nested error types produce unreadable errors in production:
```
Inner(Inner(Inner(B(Service(Inner(A(B(Error { kind: Connect }))))))))
```

Three approaches debated for 6+ years without resolution:
1. Nested generics — precise but fragile when middleware order changes
2. `Box<dyn Error>` — readable but erases non-Error traits (Linkerd needs them)
3. `Into<Box<dyn Error>>` bound — the compromise adopted by tower middleware

**[tower: Issue #131, 13 comments]**

### axum's Resolution: Errors ARE Responses

axum forces all services to `Error = Infallible`. Errors that reach hyper kill the connection. Therefore, axum converts every error into a response via `HandleErrorLayer`. The `match err {}` pattern on `Infallible` is a type-level proof of unreachability. **[axum: Insight 5, axum/src/routing/route.rs:228-237]**

### Buffer Error Propagation Order

tower's Buffer worker propagates errors in strict order: (1) expose error in shared state, (2) close receive channel, (3) store for pending requests. This ordering prevents a race where a sender could miss the error entirely. The compiler won't enforce this ordering — it must be documented. **[tower: Insight 5, worker.rs]**

### Transferable Principle for x.uma

x.uma should follow axum's pattern: policy denials are responses, not errors. A rate-limit rejection is HTTP 429, an auth failure is HTTP 401/403 — these are well-formed responses, not middleware errors. Reserve the error path for infrastructure failures (connection drops, OOM). Use `Box<dyn Error>` for infrastructure errors at the FFI boundary (precision is not needed there). When propagating errors in concurrent contexts, document the ordering invariant explicitly.

---

## 6. Compile-Time vs Runtime Type Trade-offs

**The question for x.uma:** Where should x.uma erase types, and where preserve them?

### BoxedIntoRoute — Early Erasure Strategy

axum type-erases handlers at registration time via `BoxedIntoRoute`. Without this, a router with 50 routes generates enormous monomorphization IR. The erasure happens once per method, not once per handler. **[axum: Insight 6, axum/src/boxed.rs]**

### Body Type Erasure at the Boundary

axum removed the body type parameter `B` from `Router<S, B>`. At the Service boundary, any `B: HttpBody` is immediately erased to `Body` via `req.map(Body::new)`. Performance impact: negligible. DX impact: transformative. The principle: when a type parameter is useful to < 5% of users but costs 100% in complexity, remove it. **[axum: Insight 3, PR #1751]**

### The BoxedIntoFactory Pattern for Registries

axum's erasure pattern transfers directly to x.uma's extension registry. Instead of N factory structs (C++/JVM pattern), use trait-on-type + generic builder erasure:

```rust
let registry = Registry::<HttpMessage>::builder()
    .input::<HeaderInput>("xuma.http.v1.HeaderInput")  // erased here
    .input::<PathInput>("xuma.http.v1.PathInput")
    .build();  // frozen after this
```

Monomorphization generates a specialized closure at compile time. After the builder method, the concrete type is erased into `Box<dyn Fn(&[u8]) -> Result<Box<dyn DataInput>>>`. Zero factory boilerplate. **[axum: Derived Pattern, detailed in axum-mastery]**

### Deferred Bounds for Compile Time

Remove trait bounds from router/builder methods and defer checking to when the service is actually used. Lazier type checking = faster compilation. **[axum: Issue #200, PR #404]**

### The Generic Request Trade-off

tower moved from `Service { type Request; }` (associated type) to `Service<Request>` (generic parameter). Generic parameters carry ergonomic cost. Put bounds on constructors (for error messages) but not on accessors (for flexibility). **[tower: Issue #99, 34 comments]**

### Transferable Principle for x.uma

x.uma should erase types at three boundaries: (1) policy rule registration (BoxedIntoFactory pattern — erase rule types into trait objects at builder time), (2) the HTTP body (erase at gateway entry, like axum), (3) middleware composition (erase policy middleware into `BoxCloneSyncService` equivalents for the router). Keep concrete types within each policy module for compile-time safety. Defer bounds to use-site for faster compilation of the registry builder.

---

## 7. Body Handling and Backpressure

**The question for x.uma:** How should x.uma handle request/response bodies, especially for policy inspection?

### Zero-Capacity Channels as Physical Backpressure

hyper uses `mpsc::channel(0)` for body streaming. Zero capacity means the sender physically cannot push data until the receiver has polled. This IS the backpressure — not a protocol, a physical constraint. Any buffer capacity > 0 means the producer can outrun the consumer. **[hyper: Insight 3, body/incoming.rs:114-137]**

### Zero-Capacity Is Not Degenerate

hyper uses both `futures_channel::mpsc` and `tokio::sync::mpsc` because `tokio::sync::mpsc` does not support zero capacity (minimum is 1). Benchmarks confirmed different performance profiles for different access patterns. Unifying without benchmarking degraded performance. **[hyper: Issue #3309]**

### The Want Channel

On top of the data channel, a `watch::channel` carries a WANT_PENDING/WANT_READY signal for `Expect: 100-continue`. The server shouldn't send the body until the client signals readiness. Ownership becomes protocol signaling: Sender drop = graceful close, `abort()` = abnormal close. **[hyper: body/incoming.rs:80-88]**

### Body as Frames (Forward-Compatible Trait)

hyper unified `poll_data` + `poll_trailers` into a single `poll_frame` returning `Frame<Data>`. New frame types (HTTP/2 PRIORITY, PUSH_PROMISE) become new `Frame` variants — semver-compatible. The cost: explicit `data_done` state tracking moves inside the library rather than the trait. **[hyper: PR #3020, Issue #2840]**

### No Body vs Empty Body

In HTTP, absence of `Content-Length` + no body means "no body." `Content-Length: 0` means "body exists but is empty." hyper's `body_type` logic distinguishes `None` (no body at all) from `Some(BodyLength::Known(0))`. HEAD responses, 204, 304 MUST NOT include `Content-Length: 0`. **[hyper: Commit 3705a7e4]**

### Transferable Principle for x.uma

For policy inspection that requires reading the body (payload scanning, content-type validation): use a zero-buffered channel to stream body chunks through the policy engine without buffering the entire payload in memory. For streaming xDS config updates, zero-capacity channels prevent the control plane from outrunning x.uma's config application. Design x.uma's body/frame trait with a discriminated union (`poll_frame` pattern) for forward-compatibility with future protocol extensions.

---

## Cross-Cutting Concerns

### Own Your IO Boundary

hyper defines its own `Read`/`Write`/`Executor`/`Timer` traits for io_uring forward-compatibility and 3-year stability. The DX cost (wrapper per runtime) is paid once per runtime, saved across the ecosystem lifetime. x.uma should own its async IO boundary for the same reason. **[hyper: Insight 5, rt/io.rs]**

### Dependency Audit

hyper replaced 17K-line `futures-util` with 150 lines of hand-written polyfills. Audit dependencies by what you import, not what the crate provides. For a foundation crate like x.uma, external churn is an existential risk. **[hyper: PR #3890]**

### Two-Layer Trait Pattern

When you need both API stability and ecosystem extensibility: public open trait + internal sealed trait with blanket impl. Users implement the public trait; x.uma evolves the internal trait freely. **[hyper: Issue #2051, Insight 10]**

### Timeout and Config Provenance

Track whether a configuration value was set as a default or explicitly by the user. Default: degrade gracefully (warn + disable). Explicit: fail loudly (panic). **[hyper: Insight 19, commit f3308c04]**

### Security Defaults as Operational Decisions

DoS-relevant defaults (max concurrent streams, rate limits) should be documented as unstable operational tuning, not as API stability guarantees. Reserve semver promises for interfaces. **[hyper: Insight 24, commit dd638b5b]**

---

## Source Traceability

| Concern | tower | axum | hyper |
|---------|-------|------|-------|
| Service composition | Service + Layer traits, Issue #99 | Handler blanket impls, boxed.rs | Service &self, HttpService sealed bridge |
| Backpressure | poll_ready protocol, Buffer/PollSender, Issue #626 | — | channel(0), want channel, poll_frame |
| State machines | — | — | conn.rs Reading x Writing x KA, dispatch.rs |
| Extraction | — | FromRequestParts/FromRequest, phantom coherence | — |
| Error handling | Issue #131, worker.rs ordering | Infallible, HandleErrorLayer | maybe_panic!, InfallibleRouteFuture |
| Type erasure | BoxService, Issue #663 | BoxedIntoRoute, body erasure, PR #1751 | Own traits, dependency audit |
| Body handling | — | Body::new erasure | channel(0), poll_frame, Kind enum |
