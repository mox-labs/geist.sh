# Schools of Rust

Choosing which school of Rust design to draw from for geist.sh `shell/` decisions.
From 20 elite codebases, 428 insights, ~175k words of mining.

## Table of Contents

1. [The Schools](#the-schools)
2. [The Meta-Pattern (20/20)](#the-meta-pattern-2020)
3. [Where Schools Disagree](#where-schools-disagree)
4. [The Oscillation Signal](#the-oscillation-signal)
5. [Choosing a School for geist.sh](#choosing-a-school-for-geistsh)

---

## The Schools

### tokio-rs — Protocol Correctness and Composition

**Leader**: Carl Lerche. **Codebases**: tower, bytes, tokio, hyper, tracing.

Composable protocol machines. Every abstraction is a producer-consumer contract.
Composition is algebraic: `Service -> Service`, `Layer -> Layer`.

**When to apply**: Middleware stacks, async runtime, backpressure, observability.

**Signature move**: The reservation pattern. `poll_ready` before `call` (tower).
`channel(0)` as physical backpressure (hyper). Cooperative budget (tokio).
Interest caching (tracing). Every resource has a protocol for acquiring it.

### Tolnay — API Perfection and Decade Stability

**Leader**: David Tolnay. **Codebases**: serde, proc-macro2, quote, syn.

The API IS the product. Compile-time performance is user experience. Stability
over innovation — serde has never had a breaking release. Generated code has
different optimization needs than handwritten code.

**When to apply**: Public trait hierarchies, proc macros, serialization, stable APIs.

**Signature move**: 29-type data model (serde). Dual-mode wrapper/fallback
(proc-macro2). Frequency-ordered rules (quote). `#[non_exhaustive]` + `Verbatim`
for grammar evolution (syn). Minimize cost imposed on downstream.

### CrabNebula — Security as Architecture

**Leader**: Lucas Nogueira, Fabian-Lars. **Codebases**: tauri.

Security is the architecture, not a feature. Deny-first ACL where deny is
absolute. Compile-time config embedding = binary IS the policy. Cryptographic
isolation within same process. Cross-platform = every platform quirk is yours.

**When to apply**: Desktop IPC, permissions, webview security, plugin trust, globs.

**Signature move**: `denied_commands` before `allowed_commands` (absolute deny).
AES-256-GCM isolation. Origin-based IPC. `require_literal_separator: true` on
globs after security advisory.

### aya — Dual-World Privilege Enforcement

**Leader**: Volo, Tamir Duberstein. **Codebases**: aya.

When your abstraction boundary is a privilege boundary, enforce at the type level.
The kernel is a different world. Pod guards the crossing. Feature detection by
error code because the kernel doesn't give nice capability APIs.

**When to apply**: eBPF (srt), kernel/user data, syscall boundaries, capability probing.

**Signature move**: `Pod` trait as kernel/user boundary. TC dual-path (TCX vs
Netlink) by kernel version. MockableFd for rootless testing. OwnedFd/BorrowedFd RAII.

### Mozilla — FFI Bridging via Code Generation

**Leader**: Ben Dean-Kawamura. **Codebases**: uniffi-rs.

When the code generator is the authority, safety shifts from type system to
generation pipeline. Generated code: hand-monomorphize, double catch_unwind,
tolerate redundancy. ADR-driven deliberation: oscillation in documents, not code.

**When to apply**: geist (Python) <-> shell (Rust) FFI, async FFI, resource lifecycle.

**Signature move**: Lower (infallible) / Lift (fallible) = trust asymmetry.
Two-phase argument lifting. Handle LSB bit-stealing. 9 formal ADRs. Send+Sync
mandate from Firefox Android production bug (ADR-0004).

### Other Schools (Not Primary for geist.sh)

| School | Philosophy | Draw From When... |
|--------|-----------|-------------------|
| **BurntSushi** | Measure on real workloads, default to boring | Performance-sensitive x.uma paths |
| **matklad** | Incremental computation, IDE-first | Demand-driven analysis |
| **Glavina** | Formal concurrency, epoch GC | Lock-free data structures in srt |
| **Leptos/Dioxus** | Type system for DX, reactive UI | If HUD becomes reactive Rust |
| **Embassy** | Async without alloc | Constrained/embedded targets for srt |
| **rustls** | `forbid(unsafe_code)`, compiler-enforced | TLS layer, security-critical paths |

---

## The Meta-Pattern (20/20)

> **Protocol obligations over convenience optimizations.**

Confirmed 20/20. Correctness constraints are **obligations** that consumers MUST
fulfill. When someone optimizes around the obligation, the system breaks.

### All 20 Manifestations

| # | Codebase | Obligation |
|---|----------|-----------|
| 1 | **tower** | `poll_ready` before `call` (social contract — panic permitted) |
| 2 | **bytes** | `advance()` reduces capacity (reverted optimization that violated this) |
| 3 | **leptos** | Track inside reactive context (diagnostic warning with examples) |
| 4 | **crossbeam** | Stamp garbage with global epoch (local optimization → use-after-free) |
| 5 | **ripgrep** | Pre-filter must never produce false negatives |
| 6 | **serde** | 29-type data model IS the contract (cannot extend without breaking) |
| 7 | **axum** | Body extractors MUST be last; rejection errors MUST be `Infallible` |
| 8 | **rust-analyzer** | salsa's incremental model IS the contract (demand-driven, not eager) |
| 9 | **embassy** | TxToken IS reservation; SpawnToken MUST be spawned or leaked |
| 10 | **aya** | Pod guards kernel/user boundary; verifier enforces reserve/submit |
| 11 | **tokio** | Cooperative budget MUST be committed; RUNNING bit MUST be acquired; dual close bits for shutdown |
| 12 | **hyper** | Body `channel(0)` blocks sender until receiver polls; state machine guards reject invalid transitions |
| 13 | **rustls** | Verification markers MUST exist to reach traffic state |
| 14 | **tracing** | Interest caching IS the protocol; per-layer bitmap coordination; cooperative close ordering |
| 15 | **uniffi-rs** | unsafe traits encode trust boundary; two-phase lifting; hand-monomorphized scaffolding; ADR-driven deliberation over code-level oscillation |
| 16 | **dioxus** | Unconditional hooks (index protocol); suspense-scope diffing even when suspended; synchronous prevent_default; height-ordered parent-before-child diffing; generation check on every signal access |
| 17 | **tauri** | `denied_commands` before `allowed_commands` (deny is absolute); RuntimeAuthority check before every command; Channel drop sends `{ end: true }`; CSP injection mandatory; glob `require_literal_separator` after advisory; path canonicalization before scope check; origin check per IPC call |
| 18 | **proc-macro2** | Fallback parser validates before compiler delegation; mismatch panic enforces variant consistency; DeferredTokenStream must flush before iteration; PhantomData<Rc<()>> forces !Send/!Sync matching compiler; negative literals split to match compiler behavior |
| 19 | **quote** | Transposition protocol (7 offsets, 3 tokens context) is rigid structural obligation; HasIterator MUST be true for repetitions; RepInterp shadow binding prevents double-advance; `while true` over `loop` for diagnostic hygiene; frequency-ordered rules as performance obligation |
| 20 | **syn** | Parse trait contract (peek before parse, advance or error); `#[non_exhaustive]`+Verbatim (every enum MUST handle unknown variants); Attribute not implementing Parse (forced context choice); Punctuated not implementing Parse (trailing ambiguity obligation); Unexpected token detection via Drop; `fork()` MUST be validated by `advance()`; FixupContext 10-flag correctness protocol |

### For geist.sh

- **L1 (srt)**: Pod for kernel/user data. Feature probing before attach.
- **L2 (x.uma)**: Deny before allow. Privileged channel for policy. Validate before route.
- **L4 (geist<->shell)**: Lift validates every Python->Rust crossing. Drop cancels across FFI.

---

## Where Schools Disagree

These are where the deepest judgment lives. No universal right answer.

### 1. `&self` vs `&mut self` on Service Traits

| School | Position | Rationale |
|--------|----------|-----------|
| **tower** | `&mut self` on `call` | Enables `poll_ready` backpressure protocol |
| **hyper** | `&self` on `call` (dropped `poll_ready`) | Enables `Arc<S>`, `&S`, `Box<S>` wrapper algebra |
| **axum** | Inherits hyper's `&self` | DX — handlers are Fn, not FnMut |

hyper PR #3607 proved `&self` enables full wrapper algebra. tower kept `&mut self`
for backpressure. Both correct for their context.

**geist.sh**: x.uma policy checks are pure validation (no backpressure). Use `&self`.

### 2. Compile-Time vs Runtime Configuration

| School | Position | Rationale |
|--------|----------|-----------|
| **tauri** | Compile-time freeze | Binary IS the policy; webview can't rewrite |
| **aya** | Runtime loading | eBPF programs loaded at runtime, kernel varies |
| **tokio-rs** | Mix — types compile-time, values runtime | Trait bounds are static; configuration is dynamic |

**geist.sh**: Enforcement mechanism compiles in. Policies are runtime-configurable
through privileged channel. See SKILL.md § "Security Architecture."

### 3. Type-System Encoding vs Social Contract

| School | Position | Rationale |
|--------|----------|-----------|
| **axum** | Encode in type system (body extractor ordering) | Compile error > runtime error |
| **tower** | Social contract (poll_ready before call) | Can't encode without GATs / session types |
| **rustls** | Encode in type system (handshake typestate) | Security demands it |
| **syn** | Social contract (peek before parse, fork+advance) | Parsing is too dynamic for full type encoding |

**geist.sh**: Type-system encoding for security boundaries (L1, L2). Social
contracts for internal coordination where type-system cost is prohibitive.

### 4. Generated Code vs Handwritten Code

| School | Position | Rationale |
|--------|----------|-----------|
| **uniffi-rs** | Generated code should be redundant and safe | Different optimization profile; maintainability doesn't matter |
| **Tolnay** | Generated code should be optimal and minimal | Compile time IS user experience |
| **syn** | Self-bootstrapping codegen from schema | Generate the boilerplate, hand-write the logic |

**geist.sh**: FFI boundary = uniffi-rs (safety over elegance). Proc macros = Tolnay
(minimize compile time).

### 5. Error Strategy

| School | Position | Rationale |
|--------|----------|-----------|
| **axum** | Rich rejection types, 5 approaches explored | DX — clear error messages for API users |
| **rustls** | `dangerous()` API — friction by design | Security — wrong path should feel wrong |
| **serde** | Error messages as crafted artifacts | Usability — errors are the primary teaching tool |
| **hyper** | `maybe_panic!` — debug panic, release error | Internal bugs should never reach users |

**geist.sh**: Policy violations (L2) = `dangerous()`-style friction. FFI errors
(L4) = serde-quality crafted messages. Internal bugs = hyper's `maybe_panic!`.

### 6. Async Trait Strategy

| School | Position | Rationale |
|--------|----------|-----------|
| **tower** | Manual poll-based traits | Need `poll_ready`, Send bounds, dyn-safety |
| **hyper** | Manual poll-based traits | Same 3 blockers as tower |
| **uniffi-rs** | Hand-rolled RustFuture with custom waker | Can't use generic futures across FFI |
| **embassy** | `#[embassy_executor::task]` | No alloc, static dispatch only |

**geist.sh**: x.uma = tower-style poll traits. FFI async = uniffi-rs completion-callback.
eBPF = synchronous.

---

## The Oscillation Signal

> try -> revert -> settle

Oscillations in git history are the strongest signal of hard-won wisdom. Two types:

| Type | Duration | Signal |
|------|----------|--------|
| **Architectural** | Days to weeks | Genuine trade-off — the "settled" encodes *why* alternatives failed |
| **Constraint violation** | Hours to same-day | Fundamental invariant — quick revert means non-negotiable |

### Oscillations Relevant to geist.sh

| Codebase | Oscillation | Lesson for shell/ |
|----------|------------|-------------------|
| tower | `&mut self` on `call` — kept for backpressure | x.uma's Service trait design |
| bytes | Capacity reuse after advance — reverted | Data buffer contracts in IPC |
| hyper | futures 0.2 — massive revert | Foundation crates migrate LAST |
| tracing | Thread-local dispatch — eliminated (3 bugs) | Use `&'static dyn` / Arc dispatch |
| tauri | CSP isolation — add/revert/re-add | CSP handling for webview |
| tauri | Sharun default — reverted | Conservative defaults for packaging |
| uniffi-rs | Modulemap — reverted | FFI serves primary consumer (geist) |
| syn | Parser paradigm — 3 attempts to settle | Parsing infra needs iteration |

### How to Read Oscillations

1. **Revert commit message** explains why the attempt failed.
2. **What came after** — the third way is the insight, not the revert.
3. **Duration** — same-day = hard constraint. Weeks = design trade-off.
4. **Who reverted** — original author = decisive. Someone else = ongoing debate.

---

## Choosing a School for geist.sh

### By Layer

| Layer | Primary School | Secondary |
|-------|---------------|-----------|
| L1 (srt / eBPF) | aya | Embassy (no_std), crossbeam (lock-free) |
| L2 (x.uma gateway) | tokio-rs (tower, hyper, axum, tracing) | CrabNebula (deny-first), rustls (TLS) |
| L3 (application) | Tolnay (API stability, serde) | BurntSushi (perf measurement) |
| L4 (geist<->shell FFI) | Mozilla (uniffi-rs) | Tolnay (proc macros) |
| Cross-cutting | tokio-rs (tracing) | — |

### Quick Reference

| Decision | School to Consult | Key Insight |
|----------|------------------|-------------|
| Trait design for middleware | tower | Reservation protocol, Layer composition |
| HTTP connection handling | hyper | State machines beat futures by ~40% |
| Route extraction DX | axum | FromRequestParts/FromRequest split |
| Permission model | tauri | Deny-first, plugin ACL namespacing |
| eBPF program structure | aya | Pod trait, feature detection, TC dual-path |
| Python FFI boundary | uniffi-rs | Lower/Lift trust asymmetry, two-phase lift |
| Serialization formats | serde | 29-type data model, never break the contract |
| TLS and crypto | rustls | `forbid(unsafe_code)`, typestate handshake |
| Async runtime choices | tokio | Work-stealing, cooperative budget, task vtable |
| Structured logging | tracing | Layer/Subscriber split, interest caching |
| Performance tuning | ripgrep | Measure first, boring defaults, own the stack |
| Proc macros for shell/ | quote + syn | Transposition strategy, ParseStream, #[non_exhaustive] |

### When Schools Conflict

The school whose primary domain matches your layer wins. tower's `&mut self`
is right for middleware; hyper's `&self` is right for HTTP. The question is
always: *which domain are you in?*

At layer boundaries, write an ADR (per uniffi-rs practice). Oscillation in
documents is cheaper than oscillation in code.
