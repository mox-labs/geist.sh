# Observability Patterns — Structured Diagnostics Reference

> Synthesized from tracing mastery extract (Eliza Weisman's `tracing` crate).
> Applicable to geist.sh's four-layer model: srt (eBPF) -> x.uma (gateway) -> app -> geist (agent).

---

## Table of Contents

1. [Structured Event Model](#1-structured-event-model)
2. [Subscriber Composition](#2-subscriber-composition)
3. [Per-Layer Filtering](#3-per-layer-filtering)
4. [Async-Aware Instrumentation](#4-async-aware-instrumentation)
5. [Performance-Sensitive Tracing](#5-performance-sensitive-tracing)
6. [Cross-Boundary Tracing](#6-cross-boundary-tracing)
7. [Dynamic Filtering and Reload](#7-dynamic-filtering-and-reload)
8. [Anti-Patterns With Evidence](#8-anti-patterns-with-evidence)

---

## 1. Structured Event Model

**What Claude gets wrong:** Treating tracing as "structured logging." It is a different data model -- context flows through the call graph via spans; events are points within that context; fields are typed via a `Visit` trait (metrics layers call `record_i64`, formatting layers call `record_debug`). **[tracing-core: field.rs:1-80]**

| Concept | What it is | Lifecycle |
|---------|-----------|-----------|
| Span | A period of time with context | Created -> Entered -> Exited -> Closed |
| Event | A point-in-time occurrence within a span | Instantaneous |
| Field | A typed key-value pair on a span or event | Set at creation or recorded later |

**geist.sh mapping:** A request span at L2 (x.uma) parents both an eBPF enforcement span at L1 (srt) and an agent activity span at L4 (geist). The span tree crosses layer boundaries; events remain layer-local. L1 srt spans = syscall interception windows. L2 x.uma spans = request lifecycle. L4 geist spans = agent task / tool invocation.

---

## 2. Subscriber Composition

**What Claude gets wrong:** Assuming multiple independent subscribers. There is ONE subscriber (the authority that assigns span IDs). Multiple observers compose as Layers on that single subscriber.

```
Registry (Subscriber)      <- ONE per app, owns span storage + IDs (sharded-slab)
  |-- Layer: AuditLayer    <- policy allow/deny for compliance
  |-- Layer: PerfLayer     <- request latency at gateway boundary
  |-- Layer: AgentLayer    <- which agent initiated each request
  |-- Layer: EbpfLayer     <- correlates kernel events to gateway spans
```

The `Registry` stores spans in `sharded-slab` (lock-free, pooled slots) and provides `LookupSpan` for layers to access span data. Layers store per-layer data in span `Extensions` -- the same type-map as `http::Extensions` in axum/tower. **[tracing-subscriber: registry/sharded.rs]**

**Newtype discipline is mandatory.** Two layers storing `String` in extensions will collide -- `Extensions::insert` panics on duplicate `TypeId`. Each layer must use a newtype: `struct XumaTimingData(Instant)`. **[registry/extensions.rs:70-88]**

---

## 3. Per-Layer Filtering

**What Claude gets wrong:** Using global `max_level_hint` when one layer needs TRACE. This floods ALL layers. Per-layer filtering solves this via a 64-bit bitmap -- each `Filtered` layer gets a `FilterId` bitmask position, and "was this span enabled for me?" is a single bitwise AND. Hard limit: 64 filters per Registry. **[PR #1523]**

**The critical inversion:** `Filtered::enabled()` returns `true` even when its filter says "no." Returning `false` from `Layer::enabled` is a **global disable** -- it short-circuits the entire stack. Per-layer filters record "no" in a thread-local bitmap; callbacks check it and skip silently. **This is the most common mistake.** **[filter/layer_filters/mod.rs:757-788]**

```rust
let registry = tracing_subscriber::registry()
    .with(audit_layer.with_filter(LevelFilter::WARN))       // compliance only
    .with(perf_layer.with_filter(LevelFilter::INFO))        // latency tracking
    .with(debug_layer.with_filter(                           // x.uma internals only
        Targets::new().with_target("xuma", Level::TRACE)
    ));
```

---

## 4. Async-Aware Instrumentation

**What Claude gets wrong:** Using `Span::enter()` in async code. The `Entered` guard creates silently incorrect traces when held across `.await` -- the span remains "entered" while the runtime switches tasks. In multi-threaded runtimes the future becomes `!Send` (caught). In single-threaded runtimes, the compiler allows it (invisible bug). This motivated RFC 3014 (`#[must_not_suspend]`), still unstable. **[span.rs:602-642]**

```rust
// WRONG: contaminates other tasks
async fn handle(req: Request) {
    let _guard = info_span!("handle").enter();  // held across .await
    db.query().await;
}

// RIGHT: #[instrument] (preferred) -- enters/exits around each .await
#[tracing::instrument(skip(body, policy_engine), fields(agent_id, decision))]
async fn evaluate_policy(agent_id: &str, req: &RequestParts,
    body: Body, policy_engine: &PolicyEngine) -> PolicyDecision { .. }

// RIGHT: .instrument() combinator
async { db.query().await }.instrument(info_span!("handle")).await;

// RIGHT: in_scope for sync blocks (span exited before .await)
let result = span.in_scope(|| compute_sync());
db.store(result).await;
```

**geist.sh rule:** x.uma handlers are async. Always `#[instrument]` or `.instrument()`, never `span.enter()`. Use `skip` to avoid recording large values (bodies, LLM responses) as span fields. Exception: PyO3 `#[pyfunction]` calls are synchronous from Rust's side -- `span.enter()` is correct there.

---

## 5. Performance-Sensitive Tracing

**What Claude gets wrong:** Assuming tracing is free when disabled. Near-zero-cost, but the mechanism matters.

**Interest caching:** Each callsite caches `always` (skip `enabled()` forever), `never` (skip callsite entirely), or `sometimes` (call `enabled()` every time). `always` requires ALL subscribers to agree (pessimistic consensus). Dynamic filters force `sometimes` on every callsite they touch. **[subscriber.rs:104-181, Issues #902, #927]**

**Code size:** Each macro generates code at the callsite. Microsoft PR #3398 achieved 25-57% reduction by redesigning `ValueSet`. The fix for bloat is structural (fewer fields, simpler spans), not inlining. **[PRs #91, #994, #2555, #3398]**

**Cargo feature trap:** `release_max_level_info` compiles out TRACE/DEBUG. But Cargo features are **additive and transitive** -- if ANY dependency enables it, ALL trace/debug is eliminated for EVERYONE. This broke rustc. Never set compile-time tracing levels from a library crate. Only the final binary controls this via cfg flags. **[Issue #2081]**

**geist.sh performance rules:**
1. **srt**: No tracing macros in eBPF programs. Emit via ring buffer, correlate in userspace.
2. **x.uma**: Use `Targets` filter -- TRACE only for `xuma::*`, INFO for everything else.
3. **Binary crate only**: Set `release_max_level_info` in the final binary's `Cargo.toml`, never in libraries.
4. **Field discipline**: Record large values as events, not span fields.

---

## 6. Cross-Boundary Tracing

**What Claude gets wrong:** Assuming span context propagates automatically across process/language/kernel boundaries. Each boundary requires explicit propagation.

| Boundary | Mechanism | Challenge |
|----------|----------|-----------|
| geist (Python) -> shell (Rust) | Trace ID in FFI call args | Two separate tracing systems (OTel vs Rust tracing) |
| shell (Rust) -> srt (eBPF) | Ring buffer events with correlation ID | No tracing in kernel space |
| shell (Rust) -> external | W3C Trace Context headers | Standard HTTP propagation |

**Python -> Rust:** Pass trace ID across PyO3 boundary as a field. `span.enter()` is correct here (sync FFI call).

```rust
#[pyfunction]
fn evaluate_policy(trace_id: &str, agent_id: &str, action: &str) -> PyResult<bool> {
    let span = info_span!("xuma.policy_eval", trace_id, agent_id);
    let _guard = span.enter();  // sync FFI, enter() is fine
    // ...
}
```

**Rust -> eBPF:** Consume ring buffer events in userspace, create correlated spans:

```rust
while let Some(event) = ring_buf.next() {
    info_span!("srt.enforcement", syscall = event.syscall,
        pid = event.pid, correlation_id = event.request_id)
        .in_scope(|| info!(latency_ns = event.latency_ns, "enforcement complete"));
}
```

**Thread spawn:** Dispatch is thread-local, not task-local. Child threads lose span context. Explicitly pass it:

```rust
let span = tracing::Span::current();
tokio::spawn(async move { work().instrument(span).await });
```

RFC 3642 (`thread_spawn_hook`) addresses this at the language level but is still unstable. **[dispatcher.rs, RFC 3642]**

---

## 7. Dynamic Filtering and Reload

**What Claude gets wrong:** Setting up tracing once and never changing it. Production needs runtime filter changes without restart.

```rust
let (filter, reload_handle) = reload::Layer::new(EnvFilter::new("xuma=info"));
let subscriber = registry.with(filter);

// Later, from HUD control plane:
reload_handle.modify(|f| *f = EnvFilter::new("xuma=trace,srt=debug"))
    .expect("subscriber still active");
```

**Security constraint:** The reload channel must be privileged. Agents must not modify their own tracing filters (same principle as policy loading). Only the HUD control plane pushes filter changes.

**Interest cache invalidation:** Reload forces `Interest::sometimes` on all callsites, causing runtime re-evaluation. Accept this performance cost -- correctness over speed. **[subscriber.rs:104-181]**

**Enables:** incident response (TRACE for one agent), performance profiling (temporary timing spans via HUD), compliance audit (dynamic audit logging), development (hot-reload filters).

---

## 8. Anti-Patterns With Evidence

| Don't | Why | Evidence |
|-------|-----|---------|
| `span.enter()` in async code | Contaminates other tasks on same thread | RFC 3014, span.rs:604 |
| Return `false` from `Layer::enabled` for per-layer filtering | Globally disables callsite for ALL layers | filter/layer_filters/mod.rs:757-788 |
| Store common types in span extensions | Two layers storing `String` -> panic | registry/extensions.rs:70-88 |
| Set `release_max_level_*` in library crates | Cargo feature additivity kills tracing for all dependents | Issue #2081 (broke rustc) |
| Cache thread-local dispatch of global state | Staleness bugs with multiple write paths | Issues #2050, #2587, #2436; PR #2593 |
| Return `Interest::never` from fan-out subscriber | Future children might be interested | subscriber.rs:104-181 |
| Add `impl Drop` to generic instrumentation wrappers | Changes drop-check rules, breaks downstream borrows | Issue #2578 |
| Use `Option::<Layer>::None` expecting invisibility | `max_level_hint` returns `Some(OFF)`, poisons entire stack | Issue #2265, PR #2321 |

---

## Source Traceability

| Concern | Source |
|---------|--------|
| Subscriber/Layer split | tracing PR #420, Issues #135, #136 |
| Per-layer filtering bitmap | tracing PR #1523, Issues #302, #597 |
| Thread-local dispatch bugs | tracing Issues #2050, #2587, #2436; PR #2593 |
| Interest caching consensus | tracing Issues #902, #927 |
| Async span contamination | tracing span.rs:602-642, RFC 3014 |
| Compile-time filter hazard | tracing Issue #2081 |
| Extensions type-map | tracing registry/extensions.rs, http::Extensions |
| ValueSet code size | tracing PRs #91, #994, #2555, #3398 |
| Thread spawn context loss | RFC 3642, RFC 0461 |
| Option layer poisoning | tracing Issue #2265, PR #2321 |

Full extract: `~/oss/research/tracing-mastery.md` (22 insights with commit traceability)
