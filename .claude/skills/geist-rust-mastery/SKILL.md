# geist-rust-mastery

Rust architectural judgment for geist.sh. Use when: designing x.uma gateway or matcher,
Tauri desktop shell patterns, PyO3 FFI boundary, eBPF programs for srt, concurrency and
lock-free patterns, observability with tracing, choosing `&self` vs `&mut self`, or making
compile-time vs runtime enforcement decisions. For judgment, not syntax.

## Contents

- [The Meta-Pattern](#the-meta-pattern)
- [What Claude Gets Wrong](#what-claude-gets-wrong)
- [Security: Freeze vs Configure](#security-freeze-vs-configure)
- [Schools of Rust](#schools-of-rust)
- [Gateway Design Tensions](#gateway-design-tensions-xuma)
- [FFI Boundary](#ffi-boundary-geist--shell)
- [eBPF Specifics](#ebpf-specifics-srt--aya)
- [Anti-Patterns With Evidence](#anti-patterns-with-evidence)
- [References](#references)

---

## The Meta-Pattern

> **Protocol obligations over convenience optimizations.** (Confirmed 20/20 codebases)

Correctness constraints are obligations that consumers MUST fulfill, even when convenience
would relax them.

| Layer | Manifestation |
|-------|--------------|
| L1 (srt/eBPF) | aya Pod guards kernel/user boundary; verifier enforces reserve/submit |
| L2 (x.uma) | tower `poll_ready` before `call`; deny checked before allow |
| L2 (HTTP) | hyper body `channel(0)` = physical backpressure, not protocol |
| L4 (FFI) | uniffi-rs Lower (infallible) / Lift (fallible) = trust asymmetry |

---

## What Claude Gets Wrong

These are the specific gaps where default Claude reasoning produces wrong answers.
The skill exists to correct these.

| Default assumption | Correction | Evidence |
|-------------------|------------|----------|
| "Freeze config at compile time" | Freeze the *model*, configure the *decisions* at runtime | geist.sh threat model: agents, not webview JS |
| `&mut self` for stateful traits | `&self` + interior mutability for composability | hyper PR #3607 — `&mut self` kills `Arc<S>` algebra |
| Symmetric FFI validation | Directional: Lower (infallible) / Lift (fallible) | uniffi-rs trust hierarchy |
| Lift arguments inside async future | Two-phase: lift OUTSIDE future (non-Send), then create Send future | uniffi-rs async FFI pattern |
| Migrate foundation crates first | Migrate foundations LAST | hyper futures 0.2 massive revert |
| Test eBPF requires root/kernel | aya MockableFd enables userspace testing | aya testing infrastructure |
| Standard glob matching | `require_literal_separator: true` for security | Tauri scope matching |
| One-size IPC for all messages | <8KB direct, larger via queue | Tauri measured 2x difference |
| "Idiomatic Rust" (one school) | 7 distinct schools with different design philosophies | See [Schools of Rust](#schools-of-rust) |
| AFIT (async fn in trait) is ready | Can't add Send bounds, not dyn-safe | tower + hyper BOTH declined |

---

## Security: Freeze vs Configure

**Principle: the enforcement mechanism must be outside the attacker's reach.**

Tauri freezes config at compile time because the attacker is JS in a webview. geist.sh
needs runtime policy updates because the attacker is an autonomous agent — and agents
register and evolve. The OS kernel analogy: nobody recompiles the kernel to add a user.

| Compile into binary | Configure at runtime (privileged channel) |
|--------------------|------------------------------------------|
| x.uma matcher engine | Matcher rules and policy files |
| Deny-first evaluation order | Which commands are denied/allowed |
| IPC protocol + crypto primitives | ACL configurations per agent |
| eBPF program loader (srt) | eBPF programs themselves |
| The security *model* | The security *decisions* |

**Critical constraint**: the policy loading channel must be privileged and authenticated.
Agents cannot modify their own policies. Only the control plane (HUD) pushes policy
changes; x.uma validates them before they take effect.

### Tauri Patterns That Transfer

| Pattern | Mapping to shell/ |
|---------|------------------|
| Deny-first ACL (`denied_commands` before `allowed_commands`) | x.uma evaluation order — deny is absolute, never overrideable |
| Plugin ACL namespacing (`plugin:<name>\|<command>`) | Agent capability namespacing — prevents shadow attacks |
| Origin-based IPC (Local vs Remote) | Agent identity verification per request |
| Channel Drop sends `{ end: true }` | Resource lifecycle across geist-shell boundary |
| Size-based IPC routing (<8KB direct, large via queue) | Optimize geist-shell message passing |
| `require_literal_separator: true` on globs | Scope matching in x.uma |
| Path canonicalization before scope check | File access scoping in srt |

### Tauri Patterns That DON'T Transfer

| Pattern | Why |
|---------|-----|
| Compile-time config embedding | Agents need runtime policy updates |
| Single-binary distribution | shell/ has Rust + Python + eBPF components |
| Webview as primary UI | HUD may be terminal-first |

---

## Schools of Rust

Each school has a design philosophy. Applying the wrong school to a problem produces
subtly wrong architecture. x.uma must compose multiple schools.

| School | Philosophy | When to apply in shell/ |
|--------|-----------|------------------------|
| **tokio-rs** | Service protocol, algebraic composition | x.uma middleware stack, policy layer composition |
| **Tolnay** | Minimal API surface, maximum compile-time guarantees | Public API design, serde derives |
| **BurntSushi** | O(n) or better, evidence over theory, exhaustive tests | Matcher engine (x.uma/rumi), pattern matching |
| **matklad** | Understandable code > clever code, boring technology | Internal plumbing, non-critical paths |
| **CrabNebula/Tauri** | Deny-first security, compile-time policy | ACL framework, permission model |
| **Embassy** | Zero-cost async, embedded-first, `#![no_std]` when possible | srt/eBPF kernel programs |
| **Mozilla/uniffi** | ADR-driven, oscillation in docs not code | FFI boundary decisions |

**The decision**: when designing a component, identify which school's philosophy applies.
x.uma's matcher (BurntSushi school) has different design norms than x.uma's middleware
stack (tokio-rs school) or x.uma's permission model (Tauri school).

See [schools-of-rust.md](references/schools-of-rust.md) for per-school deep dive with
examples and traceability.

---

## Gateway Design Tensions (x.uma)

x.uma must resolve a specific tension between tower and hyper:

- **tower**: `poll_ready(&mut self)` — explicit backpressure via mutable state
- **hyper**: `call(&self)` — dropped poll_ready in favor of honest interior mutability

hyper's Sean McArthur explicitly moved away from tower's protocol in PR #3607 because
`&mut self` prevents wrapping the service in `Arc`, breaking concurrent handler patterns.

**For x.uma**: Use `&self` + interior mutability (hyper school) for request handlers that
need `Arc` sharing. Use `poll_ready` (tower school) only for rate-limiting/backpressure
layers where mutable state IS the point.

Other non-obvious gateway patterns:
- hyper H1 connection: 5x4 state machine (Reading x Writing) with guards
- hyper dispatcher: 16-iteration poll_loop with `yield_now` (counted, not timed)
- axum rejection DX: 5 approaches explored (#1116) — compile errors > runtime errors
- `BoxedIntoRoute` trades type complexity for compile time

See [middleware-gateway.md](references/middleware-gateway.md) for tower + axum + hyper synthesis.

---

## FFI Boundary: geist <-> shell

The counterintuitive patterns Claude misses:

**Trust is directional.** Lower (Rust->Python) is infallible — Rust's type system guarantees
well-formedness. Lift (Python->Rust) is fallible — all Python inputs must be validated.

**Two-phase argument lifting for async.** Lift arguments OUTSIDE the future (synchronous,
non-Send). Then create the Send future from safe Rust types. Lifting inside the future fails
because raw FFI pointers are not Send.

**Send+Sync is non-negotiable.** uniffi-rs ADR-0004 removed non-thread-safe interface support
entirely after a Firefox Android production bug where hidden Mutex wrappers caused unexplained
blocking. Never weaken this requirement for PyO3 `#[pyclass]`.

**Generated code has different safety rules.** ADR-0005: when PyO3 proc-macros are the single
source of truth, raw Arc pointers are acceptable. But never expose raw handle manipulation
to hand-written code.

**Drop = Cancel across FFI.** Rust's Drop trait maps to cancellation protocol. Python must
support cancellation via completion callback when Rust drops a foreign future.

**ADR-driven design.** uniffi-rs has 9 formal ADRs and only 5 code oscillations in 2,057
commits. The lesson: oscillate in documents, not in code. FFI decisions need ADRs because
breaking changes propagate across the entire agent SDK.

See [ffi-bridging.md](references/ffi-bridging.md) for full protocol details.

---

## eBPF Specifics (srt / aya)

Patterns specific to aya that differ from general eBPF knowledge:

| Pattern | Why it matters for srt |
|---------|----------------------|
| **Pod trait** guards kernel/user boundary | Type-safe shared data — not just `repr(C)` |
| **Feature detection by error code** | Probe capabilities before attaching, not feature flags |
| **TC dual-path** (TCX vs Netlink) | Kernel version detection at attach time, not build time |
| **MockableFd** | Test eBPF programs without root/kernel access |
| **OwnedFd/BorrowedFd** (RAII) | Fd lifecycle management — no manual close |
| **BTF CO-RE via PhantomData** markers | Compile-once run-everywhere, not per-kernel builds |

See [security-architecture.md](references/security-architecture.md) for aya + rustls + tauri
security deep dive.

---

## Anti-Patterns With Evidence

Every entry has production evidence. This is not opinion — these are mistakes that were
made, measured, and reversed.

| Don't | Why | Evidence |
|-------|-----|---------|
| Allow-only security | No "never this" | Tauri v1->v2 rewrite (#8428) |
| Freeze policies at compile time | Agents need runtime updates | geist.sh threat model |
| `&mut self` on shared traits | Kills `Arc<S>` wrapper algebra | hyper PR #3607 |
| One-size-fits-all IPC | Slow for both small and large | Tauri measured 2x |
| Foundation crates migrate first | Should migrate LAST | hyper futures 0.2 revert |
| AFIT (async fn in trait) | Can't add Send bounds, not dyn-safe | tower + hyper declined |
| Optimizing fork/clone early | LLM latency dominates | deepcopy ~0.5ms vs LLM 1-30s |
| Global mutable state | Thread-safety, testing friction | Tauri PR #14668 |

---

## References

### From 20 mined codebases (424 insights)

- [security-architecture.md](references/security-architecture.md) — rustls + tauri + aya security patterns
- [middleware-gateway.md](references/middleware-gateway.md) — tower + axum + hyper for x.uma
- [ffi-bridging.md](references/ffi-bridging.md) — uniffi-rs + tokio + bytes for geist-shell boundary
- [schools-of-rust.md](references/schools-of-rust.md) — 7 schools, when to apply which
- [desktop-shell-patterns.md](references/desktop-shell-patterns.md) — Tauri IPC, plugins, events, state
- [concurrency-patterns.md](references/concurrency-patterns.md) — crossbeam + tokio + bytes
- [observability.md](references/observability.md) — tracing patterns for multi-layer systems
- [matcher-patterns.md](references/matcher-patterns.md) — ripgrep + serde for x.uma/rumi

### Not yet mined (future work)

- **PyO3** — GIL management, pyo3-asyncio, `#[pyclass]` patterns. Currently using uniffi-rs
  patterns applied to PyO3; a dedicated PyO3 mining run would add PyO3-specific judgment.
- **wasmtime** — component model, WASI, sandboxed plugin execution. Relevant if agents or
  plugins run in WebAssembly sandboxes.

**Full extracts**: `~/oss/research/*-mastery.md` (424 insights with commit traceability)
