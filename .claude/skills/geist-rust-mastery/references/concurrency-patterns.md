# Concurrency Patterns — Lock-Free, Async, and Zero-Copy

> Synthesized from crossbeam, tokio, and bytes mastery extracts.
> Organized by architectural concern for x.uma gateway concurrency judgment.

---

## Table of Contents

1. [Lock-Free Data Structures](#1-lock-free-data-structures)
2. [Work-Stealing Scheduler](#2-work-stealing-scheduler)
3. [Zero-Copy Data Passing](#3-zero-copy-data-passing)
4. [Cooperative Scheduling](#4-cooperative-scheduling)
5. [Channel Patterns](#5-channel-patterns)
6. [Shutdown Coordination](#6-shutdown-coordination)
7. [Verification and Testing](#7-verification-and-testing)

---

## 1. Lock-Free Data Structures

**The question for x.uma:** When should policy stores and routing tables use lock-free structures vs locks?

### Epoch-Based Reclamation — The Consensus Problem

Lock-free reads require solving memory reclamation: when can freed memory be reused? crossbeam's answer is epoch-based reclamation (EBR). Readers "pin" a global epoch counter; writers defer cleanup until all pinned readers have advanced past the garbage's epoch. Two epoch advancements guarantee safety — mathematically sufficient, a three-epoch variant was tried and reverted as unnecessary overhead. **[crossbeam: internal.rs:148-152, commit 389a60b0]**

The core tension: performance demands local operation, correctness demands global coordination. crossbeam resolves this with a two-tier garbage system — thread-local bags (64 items, no synchronization) flush to a global queue (one atomic per 64 items). This amortizes synchronization rather than eliminating it. **[crossbeam: internal.rs:55-60]**

### What Claude Gets Wrong: Local vs Global Epoch

Garbage MUST be stamped with the global epoch, never a thread-local snapshot. Local snapshots can be stale by one advancement — enough for use-after-free. This bug was introduced and fixed within one week (commits `208bf821` and `cf4d4378`, 2016). The optimization (local stamping) seems obviously correct but violates the fundamental invariant: other threads may hold references from an epoch ahead of your local view. **[crossbeam: Insight 1]**

### Scalability Ceiling — Know Before You Build

EBR has a hard ceiling acknowledged by its own authors. Epoch advancement is O(N) where N = active threads. The global garbage queue becomes a contention point at 48+ cores. Long-lived pinned references (caches, cursors) block advancement indefinitely. A hybrid EBR + hazard pointer approach was prototyped but never merged. **[crossbeam: Issue #134, Issue #221]**

### The `CachePadded` Requirement

Hot shared atomics (epoch counters, head/tail positions) MUST be cache-line padded. False sharing is a performance cliff, not gradual degradation — if the epoch shares a cache line with other data, every update to nearby data invalidates every thread's epoch cache line. 64 bytes on x86, 128 on ARM. **[crossbeam: CachePadded in Global struct]**

### Double-Free in Concurrent Teardown

The recurring crossbeam bug class (Issues #972, #1084, #1187): every read-and-free in concurrent cleanup must be an atomic swap-to-null, not a read-then-free. Multiple teardown paths (Drop, disconnect, discard_all_messages) can reach the same pointer. The commit message for the 2025 fix is a masterclass: "it is critical that whenever `head.block` is freed it must also be set to a null pointer so that it is freed exactly once." **[crossbeam: commit 596df785]**

### Transferable Principle for x.uma

Use EBR for the policy store: concurrent policy reads while policies update, no locks on the read path. Pin during request evaluation, unpin after. But know the ceiling — if x.uma runs on 48+ core machines or holds policy references across multiple request evaluations, EBR will stall. For hot shared counters (rate limit state, connection counts), always `CachePadded`. For concurrent teardown of routing tables, atomic swap-to-null before free.

---

## 2. Work-Stealing Scheduler

**The question for x.uma:** How does tokio schedule x.uma's concurrent request handlers, and what can go wrong?

### The Scheduling Trilemma

An async runtime must simultaneously optimize for throughput (process many tasks), latency (respond quickly), and fairness (prevent starvation). These goals fundamentally conflict. tokio resolves this through layered mechanisms:

- **LIFO slot** (1 task) — latency via locality for message-passing patterns
- **Local queue** (256 slots) — throughput via lock-free single-producer access
- **Global inject queue** — fairness via periodic batch drain
- **Peer stealing** — load balancing by stealing half of a random peer's queue

The task priority order: LIFO slot, local queue, every N ticks drain global queue, steal from peers, park. **[tokio: scheduler/multi_thread/worker.rs]**

### What Claude Gets Wrong: LIFO, Not FIFO

tokio's LIFO slot means the most recently spawned task runs first. This is optimal for ping-pong message passing (task A wakes B, B runs immediately on the same core) but surprising if you expect FIFO ordering. The LIFO slot is capped at 3 polls per tick (`MAX_LIFO_POLLS_PER_TICK`) to prevent starvation. The `disable_lifo_slot` config exists for workloads that need FIFO guarantees. **[tokio: Insight 1]**

### The Two-Head Trick

The work-stealing queue packs two values into a single atomic: `steal` (where a thief is working) and `real` (the actual head). When they differ, a steal is in progress, blocking other stealers. This eliminates external locks during stealing. Wider-than-needed indices (u32 on 64-bit platforms for a 256-slot buffer) serve as free ABA generation counters. **[tokio: queue.rs:12-26, Issue #5041]**

### Core Migration via block_in_place

When `block_in_place` is called, the worker's scheduling state (local queue, LIFO slot, stats) migrates to a new thread. The original thread can now block safely. The LIFO slot is drained first — a task there is invisible to other workers. The unit of scheduling is the core, not the thread. **[tokio: worker.rs:345, Insight 5]**

### NUMA Breaks Work-Stealing

Work-stealing assumes uniform memory access. On NUMA systems (AMD EPYC, multi-socket), cross-node CAS operations dominate runtime. The fix is not "better stealing" but "don't steal across NUMA boundaries" — pin runtimes to NUMA nodes with `SO_REUSEPORT`. **[tokio: Issue #5076]**

### Transferable Principle for x.uma

x.uma runs inside Tauri's async runtime (tokio). Understand that request handling follows LIFO, not FIFO — recently spawned policy evaluation tasks run before older ones. For Tauri desktop, NUMA is irrelevant (single-socket). For future server deployment, plan per-NUMA-node runtime instances. Use `block_in_place` (not `spawn_blocking`) when policy evaluation must call synchronous Rust code that holds scheduling state.

---

## 3. Zero-Copy Data Passing

**The question for x.uma:** How should x.uma pass request data between policy stages without copying?

### The freeze/split/clone Lifecycle

bytes enables a write-once-read-many lifecycle with zero copies after the initial allocation:

```
BytesMut::with_capacity(1024)   // allocate once
    .put(data)                   // write
    .split()                     // O(1): shared ref, advances cursor
    .freeze()                    // O(1): mutable -> immutable
    .clone()                     // O(1): refcount bump
    .slice(10..20)               // O(1): ptr + len adjustment
```

Every operation after allocation is O(1). The alternative — `Vec<u8>` with clone — copies on every distribution. **[bytes: Insight 7]**

### What Claude Gets Wrong: Lazy Promotion

When `Bytes` is created from `Vec<u8>`, it does NOT immediately allocate a reference-counted `Shared` struct. It uses a `PROMOTABLE` vtable. Only the first `clone()` triggers promotion via CAS (two threads racing to clone both try to promote; the CAS ensures exactly one allocation). If the `Bytes` is never cloned (common in linear request pipelines), the `Shared` allocation never happens. **[bytes: Insight 3, PROMOTABLE_EVEN/ODD vtables]**

### advance() Is Not clear()

`Buf::advance()` moves the cursor forward, shrinking the view. It does NOT preserve capacity for reuse. This was tried, reverted, and codified with a test: "ensure BytesMut::advance reduces capacity." The trap: optimizing advance to reset the buffer when all data is consumed violates the `Buf` contract — callers depend on capacity decreasing. **[bytes: commit f488be4, Insight 4]**

### The Hand-Rolled Vtable Pattern

`Bytes` uses a manual `&'static Vtable` (5 function pointers: clone, into_vec, into_mut, is_unique, drop) instead of `dyn Trait`. This avoids trait object indirection while providing polymorphism across four backing strategies (static, promotable-even, promotable-odd, shared). tokio's task system uses the same pattern for the same reason. When you need precise control over layout and allocation, `dyn` is too restrictive. **[bytes: Bytes struct, tokio: Insight 2]**

### from_owner: Always Copies on into_mut

`Bytes::from_owner()` wraps external memory without copying, but `owned_is_unique()` always returns false. Converting an owner-backed `Bytes` to `BytesMut` always copies. This is documented but easily missed in generated code. **[bytes: Anti-pattern 6]**

### Transferable Principle for x.uma

Use `Bytes` directly for request data flowing through policy stages. Build into `BytesMut`, `freeze()` at the policy enforcement boundary, distribute `Bytes` clones to concurrent policy evaluators — zero copies after freeze. For x.uma's xDS protocol parsing, consume `impl Buf` (not `&[u8]`) for composable, infallible cursor operations. Never wrap mmap'd data directly in async context — page faults stall the runtime. Copy from mmap on a dedicated thread, distribute as `Bytes`. **[bytes: Issue #359, scottlamb's Moonfire NVR evidence]**

---

## 4. Cooperative Scheduling

**The question for x.uma:** How does tokio prevent a runaway policy evaluation from starving other requests?

### Budget-Based Yielding

Rust has no preemption. tokio uses a cooperative budget — every leaf I/O operation decrements a per-task counter (initial value: 128). When budget hits zero, the task yields. The `RestoreOnPending` RAII guard restores budget if no progress was made — preventing false budget consumption when a task returns `Pending` for unrelated reasons. **[tokio: task/coop/mod.rs, Insight 6]**

### What Claude Gets Wrong: Budget Is a Two-Phase Commit

Budget consumption is not decrement-and-check. It is claim-then-commit:

```rust
let coop = ready!(poll_proceed(cx));  // Phase 1: claim budget
// ... do I/O work ...
coop.made_progress();                  // Phase 2: commit
// If dropped without made_progress(), budget is restored
```

Claiming budget and making progress are separate events. If the guard is dropped without committing, the budget reverts to its pre-claim value. This prevents budget drain when a task polls speculatively. **[tokio: RestoreOnPending docs]**

### Opting Out

`task::unconstrained()` disables cooperative scheduling entirely. Use this only for tasks where yielding would cause correctness issues (e.g., atomic state machine transitions that must complete without interruption).

### Global Queue Interval with Hysteresis

The global inject queue is checked every N ticks. N is tuned dynamically but only updated when the change exceeds a threshold of 2 — hysteresis that prevents oscillation from workload jitter. **[tokio: worker.rs:1244, Insight 7]**

### Transferable Principle for x.uma

Policy evaluation in x.uma should respect cooperative scheduling — avoid long-running synchronous computation in async context. If a policy rule requires expensive matching (regex compilation, certificate validation), use `spawn_blocking` or `block_in_place`. For x.uma's policy hot path, budget is consumed by I/O leaf operations (reading request data, consulting external auth), not by CPU-bound pattern matching. Ensure policy evaluators call `poll_proceed` if they implement custom `Future` impls.

---

## 5. Channel Patterns

**The question for x.uma:** Which channel flavor for which communication pattern in the gateway?

### Three Architectures, One API

crossbeam implements three completely independent channel backends behind runtime dispatch:

| Constructor | Backing | Use Case |
|-------------|---------|----------|
| `unbounded()` | Block-allocated linked list (31 msgs/block) | Event streams where dropping is unacceptable |
| `bounded(n)` | Fixed ring buffer (Vyukov MPMC) | Backpressure between producer and consumer |
| `bounded(0)` | Rendezvous/exchanger | Synchronous handoff (request-response) |

The block size of 31 (not 32) reserves one slot for an end-of-block sentinel. Per-slot state machines (WRITE/READ/DESTROY) allow concurrent readers and the block destructor to cooperate without locks. **[crossbeam: Channel RFC, list.rs]**

### What Claude Gets Wrong: Zero-Capacity Is Physical Backpressure

hyper uses `mpsc::channel(0)` for body streaming. Zero capacity means the sender physically cannot push data until the receiver has polled. This IS backpressure — not a protocol, a physical constraint. Any buffer capacity > 0 allows the producer to outrun the consumer. tokio's `mpsc` does not support zero capacity (minimum 1), which is why hyper uses `futures_channel::mpsc` for body streams. **[hyper: body/incoming.rs:114-137]**

### Bounded Channels for Backpressure

For rate-limiting between pipeline stages, `bounded(n)` creates natural backpressure — senders block when the buffer is full. The bound should reflect the acceptable latency budget, not the expected throughput. A bound of 1 maximizes backpressure sensitivity; larger bounds amortize scheduling overhead.

### "Disconnected" Not "Closed"

crossbeam channels disconnect implicitly when all senders or receivers are dropped. There is no explicit `close()` method. "Disconnected" signals a structural state change; "closed" would imply an action. The wrong name teaches the wrong mental model. The naming was debated, reverted same-day, and never changed. **[crossbeam: Insight 3, commit 1cd5f176]**

### Transferable Principle for x.uma

Use bounded channels between x.uma's gateway stages (request parsing, policy evaluation, upstream forwarding) with bounds reflecting the latency budget. Use zero-capacity channels for body streaming where backpressure must be physical, not advisory. Use unbounded channels only for audit logging where dropping events is worse than memory growth — and pair with a drain mechanism. For request-response patterns between geist (agent) and shell (gateway), zero-capacity rendezvous channels enforce synchronous handoff.

---

## 6. Shutdown Coordination

**The question for x.uma:** How does x.uma cleanly shut down concurrent policy evaluators and in-flight requests?

### Dual Close Bits — Both Collections Must Signal

tokio's shutdown requires close bits on both the inject queue AND `OwnedTasks`:

- Close bit on only inject queue: a task bound to `OwnedTasks` could persist long after shutdown
- Close bit on only `OwnedTasks`: a notification pushed to inject queue after drain creates a ref-count cycle and memory leak

Spawning during shutdown creates a race between the spawn path and the shutdown path. Dual close bits ensure that regardless of ordering, either the task cleans up or the spawn fails immediately. **[tokio: worker.rs:1-57, Insight 8]**

### What Claude Gets Wrong: Drop Ordering in Concurrent Contexts

Drop ordering matters more in concurrent code than sequential code. A memory leak in bytes' `owned_to_vec` was caused by incorrect Drop ordering — the fix required reordering destructors. In crossbeam, `discard_all_messages` and `Channel::drop` can race to free the same block — the fix is atomic swap-to-null before every free. **[bytes: commit 3667543, crossbeam: commit 596df785]**

### The Three Reference Counts

tokio creates three ref-counts per spawned task: one for `OwnedTasks`, one for the `Notified` handle, one for the `JoinHandle`. The `Unowned` variant (blocking tasks) holds two. Shutdown must account for all three — dropping only the JoinHandle doesn't cancel the task (detach-by-default, matching `std::thread::JoinHandle`). **[tokio: Insight 13, task/mod.rs]**

### CancellationToken Over Drop-Based Cancellation

tokio deliberately rejected cancel-on-drop for `JoinHandle`. `mem::forget` is safe in Rust, making any safety invariant that depends on `Drop` running unsound in async contexts. Use `CancellationToken` (tokio-util) for explicit, cooperative cancellation propagation. **[tokio: Issue #1830, Issue #3162, Insight 15]**

### Transferable Principle for x.uma

x.uma shutdown must close both the request intake (stop accepting new connections) and the policy evaluator registry (stop spawning new evaluation tasks). Use `CancellationToken` for cooperative shutdown propagation — when the Tauri shell signals shutdown, propagate through the token tree. Do not rely on `Drop` for shutdown correctness — a leaked future can prevent `Drop` from running. For in-flight requests, give a grace period (drain timeout), then force-cancel remaining tasks via `JoinSet::abort_all`.

---

## 7. Verification and Testing

**The question for x.uma:** How should x.uma verify concurrent correctness?

### Two Levels: Loom + Miri

Lock-free data structures need two verification tools that catch different bug classes:

- **Loom** — exhaustively explores thread interleavings (logical correctness). Under Loom, queue capacity shrinks from 256 to 4 to make exploration tractable.
- **Miri** — models the C11 memory model with weak ordering semantics (memory model correctness). Catches UB invisible to Loom.

tokio's work-stealing queue passed all Loom tests for years while containing a data race that Miri only detected after its weak memory model was improved in 2025. **[tokio: Issue #7712, Insight 20]**

### The Primitive Abstraction Layer

Both crossbeam and tokio wrap all atomics behind a module that swaps between real atomics and Loom. The entire codebase uses `crate::loom::sync::atomic::AtomicUsize`, never `std::sync::atomic::AtomicUsize` directly. This abstraction must be baked in from the start — retrofitting it requires touching every atomic access. **[tokio: loom/, crossbeam: Insight 5]**

### What Claude Gets Wrong: Memory Ordering Is Not a Performance Knob

Relaxing memory orderings to improve performance is almost always a bug in concurrent synchronization code. crossbeam reverted a Chase-Lev deque ordering weakening (commit `5cdc8d68`). tokio's threadpool deadlocked because a spinning optimization weakened Acquire to Relaxed in park/unpark (Issue #525) — it worked on x86 (strong memory model) but failed on ARM/macOS. Start with SeqCst, only weaken with formal proof or model-checker validation. **[crossbeam: Anti-pattern 4, tokio: Insight 16]**

### Spinning Optimizations Require Diverse Contention Testing

crossbeam reverted a spinning optimization same-day (commits `71206d9c` and `31c2e7c5`, 2018-09-17). Spin strategies that improve throughput benchmarks can cause starvation under high contention. Test under diverse contention patterns, not just throughput microbenchmarks. **[crossbeam: Insight 2]**

### Transferable Principle for x.uma

If x.uma implements any lock-free data structures (policy store, routing table), wrap all atomics behind a Loom-compatible abstraction from day one. Run both Loom and Miri in CI. For policy hot paths using crossbeam or tokio primitives directly, the verification burden falls on those crates — but integration tests under Loom can still catch composition bugs. Never weaken memory orderings in x.uma's synchronization code without model-checker validation.

---

## Source Traceability

| Concern | crossbeam | tokio | bytes |
|---------|-----------|-------|-------|
| Lock-free structures | Epoch GC, CachePadded, swap-to-null teardown | — | — |
| Work-stealing | Chase-Lev deque, Worker/Stealer/Injector | Multi-thread scheduler, LIFO slot, two-head trick | — |
| Zero-copy | — | — | freeze/split/clone, PROMOTABLE vtable, hand-rolled vtable |
| Cooperative scheduling | — | Budget (128), RestoreOnPending, unconstrained | — |
| Channels | array/list/zero flavors, disconnected naming | — (uses crossbeam internally) | — |
| Shutdown | Concurrent Drop double-free | Dual close bits, 3 ref-counts, CancellationToken | Drop ordering in owned_to_vec |
| Verification | Loom primitive layer, FIXME compiler_fence | Loom layer, Miri weak-memory catches, Issue #7712 | Miri provenance dual path |
| Memory ordering | SeqCst default, ordering revert in deque | Lost wakeups from Relaxed (Issue #525) | AcqRel for promotion CAS |
| Backpressure | bounded(n), zero-capacity rendezvous | — | advance() contract enforcement |
