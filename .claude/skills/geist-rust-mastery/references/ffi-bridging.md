# geist (Python) <-> shell (Rust) FFI Boundary Reference

Synthesized from uniffi-rs, tokio, and bytes mastery extracts.
Applicable to PyO3-based bridging between Python (geist/Matrix/Agent SDK) and Rust (shell/x.uma).

---

## Table of Contents

1. [Trust Asymmetry](#1-trust-asymmetry)
2. [Handle Management](#2-handle-management)
3. [Async FFI](#3-async-ffi)
4. [Error Handling Across the Boundary](#4-error-handling-across-the-boundary)
5. [Resource Lifecycle](#5-resource-lifecycle)
6. [Data Passing](#6-data-passing)
7. [ADR-Driven FFI Design](#7-adr-driven-ffi-design)

---

## 1. Trust Asymmetry

**Core principle:** The FFI boundary is asymmetric. Rust validates; Python is validated.

UniFFI encodes this in its trait hierarchy:
- **Lower** (Rust -> Python): infallible. Rust controls the output; the data is well-formed by construction.
- **Lift** (Python -> Rust): fallible (`try_lift`). Python inputs cannot be verified by the Rust compiler.

```
Lower: fn lower(v: Self) -> Self::FfiType;          // infallible
Lift:  fn try_lift(v: Self::FfiType) -> Result<Self>; // fallible
```

**Application to geist<->shell:** Every PyO3 `#[pyfunction]` or `#[pymethods]` entry point into shell/x.uma is a Lift boundary. All arguments arriving from Python must be validated before entering Rust logic. Return values (Lower direction) need no validation -- Rust's type system guarantees well-formedness.

**The mandate:** All types crossing the boundary must be `Send + Sync`. UniFFI's ADR-0004 removed support for non-thread-safe interfaces entirely after a Firefox Android production bug where hidden `Mutex` wrappers caused unexplained blocking. The lesson: if your FFI framework silently adds mutexes, users cannot reason about performance. Force thread-safety to be explicit.

**Implication for PyO3:** PyO3's `#[pyclass]` requires `Send` by default (for the GIL-release case). This aligns with the uniffi mandate. Never weaken this -- if a type cannot be `Send + Sync`, it should not cross the boundary.

---

## 2. Handle Management

**The evolution (from uniffi-rs):**

| Stage | Mechanism | Trade-off |
|-------|-----------|-----------|
| 1. HandleMap | Safe index-into-array | Cannot support nested objects or return values |
| 2. Arc pointers | Raw `Arc<T>` via `Arc::into_raw` | Requires generated code to uphold safety |
| 3. u64 handles | Uniform `u64` regardless of platform | Eliminates 32/64-bit branching |
| 4. LSB bit-stealing | Odd = foreign handle, even = Rust handle | Minimum viable abstraction |

```rust
#[repr(transparent)]
pub struct Handle(u64);

impl Handle {
    pub fn is_foreign(&self) -> bool { (self.0 & 1) == 1 }

    pub fn from_arc<T>(arc: Arc<T>) -> Self {
        Self(Arc::into_raw(arc) as u64)  // Even -- pointer alignment
    }

    pub unsafe fn into_arc<T>(self) -> Arc<T> {
        Arc::from_raw(self.0 as *const T)
    }
}
```

**Application to geist<->shell:** When Python holds a reference to a Rust policy engine or matcher, it holds a `u64` handle. The handle is opaque to Python. Reconstruction (`into_arc`) happens only on the Rust side, inside a validated Lift path.

**Key insight (ADR-0005):** Generated/macro code has different safety requirements than hand-written code. When PyO3 proc-macros are the single source of truth for the bridge, raw `Arc` pointers are acceptable -- the macro upholds the contract, not the programmer. But never expose raw handle manipulation to hand-written code.

**Never reuse handles.** UniFFI learned this the hard way -- handle reuse breaks trait interfaces where the same Rust object might be wrapped by different Python objects.

---

## 3. Async FFI

**The problem:** Rust futures must be `Send` (they move between tokio worker threads). But lifting raw pointers from Python is not `Send`. These two requirements conflict.

**The solution -- two-phase argument lifting:**

```rust
// Phase 1: Lift (not Send -- deals with raw pointers from Python)
let args = try_lift_all_args(raw_args)?;

// Phase 2: Create Send future from lifted Rust-native args
let future = async move {
    policy_engine.evaluate(args).await  // args are Rust types, Send
};
```

Lift arguments outside the future (synchronous, on the calling thread), then create the async future from safe Rust types. This is the FFI equivalent of tokio's "poll_ready before call" protocol.

**RustFuture design patterns (from uniffi-rs + tokio):**

- **Two separate mutexes:** One for future state (poll execution), one for scheduling state (wake/poll state machine). Combining them creates unnecessary contention. Same separation as tokio's task header.
- **Hand-rolled waker vtable:** Both uniffi-rs and tokio use `RawWakerVTable` with explicit Arc-based reference management rather than `futures::task::ArcWake`. Eliminates one pointer chase per wake -- and waking is the hot path.
- **`AssertUnwindSafe` justified by never-poll-after-panic:** If `poll` panics, transition to terminal error state. The invalid post-panic future is never observed because it is dropped immediately. Document this proof in a comment.
- **Cooperative scheduling budget:** If shell exposes async operations to geist, those operations must participate in tokio's cooperative budget (`poll_proceed`). Otherwise a long-running policy evaluation could starve other tasks on the same runtime.

**Application to geist<->shell:** When Python `await`s a Rust async operation:
1. Python calls into Rust (GIL released via `pyo3_asyncio` or manual `allow_threads`)
2. Arguments are lifted synchronously (Phase 1)
3. A `Send` future is created and spawned on the tokio runtime (Phase 2)
4. Completion is signaled back to Python via callback or channel

---

## 4. Error Handling Across the Boundary

**Double `catch_unwind` -- the belt-and-suspenders pattern:**

```rust
let result = std::panic::catch_unwind(|| {
    // Call into policy evaluation logic
});

match result {
    Ok(v) => v,
    Err(e) => {
        // Second catch_unwind: formatting the panic message might itself panic
        let message = std::panic::catch_unwind(|| {
            format_panic_message(e)
        }).unwrap_or_else(|_| "secondary panic".to_string());
        // Return error to Python
    }
}
```

**Why two levels:** A panic crossing the FFI boundary is undefined behavior. The first `catch_unwind` prevents this. But the panic payload's `Display` impl might itself panic (accessing invalidated state). The second `catch_unwind` costs nothing in the happy path but prevents UB when error reporting fails.

**The error protocol (from uniffi-rs `RustCallStatus`):**

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Expected error (function returned `Err`) |
| 2 | Unexpected error (panic caught by `catch_unwind`) |
| 3 | Cancelled (async cancellation) |

**Application to geist<->shell:** Every PyO3 entry point wraps the inner call in `catch_unwind`. Expected errors become Python exceptions (via PyO3's `PyErr`). Panics become a generic `RuntimeError` with the caught message. Never let a panic unwind through the FFI boundary.

**Double-completion prevention:** Guard completion callbacks with an atomic flag ensuring exactly-once invocation. A Firefox Android production bug showed that timing-dependent double-completion on older ARM devices caused double-free segfaults. The cost of an atomic check is negligible; the cost of a double-free on a user's device is catastrophic.

---

## 5. Resource Lifecycle

**Separate reference release from resource closing (ADR-0008).**

| Concern | Mechanism | Timing |
|---------|-----------|--------|
| Reference release | GC / ref-count drop | Non-deterministic (Python GC is fine) |
| Resource closing | Explicit `close()` / context manager | Deterministic (user controls) |

Never conflate these. Python's GC handles reference lifecycle. But if a Rust object holds a file handle, network connection, or policy engine lock, there must be an explicit `close()` that works even if other `Arc` references exist.

**Drop = Cancel for async operations (from uniffi-rs):**

```rust
impl Drop for ForeignFutureDroppedCallback {
    fn drop(&mut self) {
        // Signal the foreign side: this future was cancelled
        (self.callback)(self.handle, ForeignFutureResult { /* cancelled */ });
    }
}
```

Rust's Drop-as-cancel is stricter than Python's "just stop awaiting." The FFI protocol maps the strict model -- Python must support cancellation via the completion callback.

**Channel close protocols (from tokio):**

Shutdown requires dual close bits -- one on the work queue, one on the task registry. Close only one and tasks leak into the unclosed collection. For geist<->shell: when the shell runtime shuts down, both the command channel and the handle registry must be closed. Order matters -- close the command channel first (reject new work), then drain and close the handle registry (clean up existing resources).

---

## 6. Data Passing

**The spectrum:**

| Strategy | Cost | When to use |
|----------|------|-------------|
| Zero-copy (`Bytes`) | O(1) clone, refcount bump | Hot path: policy match data, streaming bytes |
| Serialization (byte buffer) | O(n) copy | Complex types, cross-version safety |
| `repr(C)` structs | Zero overhead | Simple scalar types only |

**UniFFI's evolution:** Started with serialization into byte buffers (ADR-0002) because it "strictly controls shared access to memory on each side of the boundary." Each side owns its memory; the buffer is the only shared artifact. Now migrating to direct pointer passing for performance.

**Zero-copy patterns from bytes:**

The `freeze/split/clone` trilogy enables write-once-read-many without copies:
```
BytesMut::with_capacity(1024)  // allocate once
  -> buf.put(data)              // write
  -> buf.split()                // O(1): shared ref, advance cursor
  -> split.freeze()             // O(1): immutable Bytes
  -> bytes.clone()              // O(1): refcount bump
```

**Application to geist<->shell:** When policy match data arrives as network bytes:
1. Receive into `BytesMut` (single allocation)
2. `freeze()` into `Bytes` (O(1))
3. Pass `Bytes` to the Rust policy engine (zero-copy, Send + Sync)
4. Return results to Python as serialized bytes or scalar handles

**Pitfall -- mmap + async = architectural mismatch:** Page faults are blocking operations that stall the tokio runtime. Production evidence (Moonfire NVR) shows that `memcpy` from mmap on a dedicated thread is faster than mmap in the async runtime. If shell uses memory-mapped policy files, read them on `spawn_blocking`, distribute as `Bytes`.

**Design for the weakest consumer's type system:** FFI types must accommodate Python's limitations. Use fixed-width integers (not `usize`). Use signed integers where Python or JVM consumers expect them. The FFI type is the lowest common denominator.

---

## 7. ADR-Driven FFI Design

**The correlation (from uniffi-rs):** 9 formal ADRs, only 5 code-level oscillations in 2,057 commits. The deliberation happens in documents, not in reverts.

**Why this matters for geist<->shell:** The FFI boundary between Python and Rust is foundational infrastructure. Breaking changes propagate across the entire agent SDK. ADRs force deliberation before code, reducing costly reverts.

**ADR template for FFI decisions:**

```markdown
# ADR-NNNN: [Decision Title]

## Status: [Proposed | Accepted | Deprecated | Superseded]

## Context
What is the force driving this decision?

## Decision
What was decided and why.

## Consequences
What becomes easier. What becomes harder.
Known technical debt (with explicit "acceptable for MVP" if applicable).

## Alternatives Considered
What was rejected and why.
```

**Decisions that need ADRs in geist<->shell:**
1. PyO3 vs uniffi-rs vs manual `extern "C"` -- the bridge mechanism
2. Serialization format for complex policy types (bytes vs protobuf vs repr(C))
3. Handle strategy for long-lived Rust objects held by Python
4. Async bridging protocol (pyo3-asyncio vs manual channel-based)
5. Error taxonomy -- which Rust errors become which Python exceptions
6. Version checking -- how to detect Python SDK / Rust runtime mismatch

**Key quote (ADR-0002):** "This choice comes with non-trivial performance costs, but that's acceptable for MVP." When the ADR explicitly acknowledges the debt, you have created a contract with your future self. The migration is the debt being repaid -- exactly as planned.

**The master principle:** The cost of an oscillation scales with the dependency graph. For foundational infrastructure, oscillate in design documents, not in code. ADRs are cheaper to revert than commits.

---

## Quick Reference: Protocol Obligations at the geist<->shell Boundary

| Obligation | Violation Consequence |
|------------|-----------------------|
| Validate all Python inputs (Lift is fallible) | Type confusion, UB |
| `Send + Sync` for all boundary types | Hidden contention, data races |
| Two-phase argument lifting for async | Non-Send future, compile error or UB |
| Double `catch_unwind` at every entry point | Panic crosses FFI = UB |
| Atomic guard on completion callbacks | Double-free on concurrent completion |
| Separate resource close from reference release | Resource leak or use-after-close |
| Explicit `close()` / context manager for resources | Deterministic cleanup lost to GC timing |
| Version checksum at init time | Silent corruption from version mismatch |
| ADR for every foundational FFI decision | Oscillation in code, costly reverts |

---

## Sources

- **uniffi-rs mastery extract** -- FfiConverter hierarchy, ADR-0002/0004/0005/0008, Handle evolution, double catch_unwind, two-phase lifting, RustFuture design, Drop=Cancel
- **tokio mastery extract** -- Cooperative scheduling budget, two-mutex pattern, hand-rolled waker vtable, block_in_place core migration, dual close bits for shutdown, AssertUnwindSafe + never-poll-after-panic
- **bytes mastery extract** -- Zero-copy freeze/split/clone, Bytes vtable polymorphism, mmap+async mismatch, BufMut unsafe trait (trust asymmetry for write side), pointer bit-stealing, provenance considerations
