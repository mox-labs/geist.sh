# Security Architecture Reference for geist.sh Shell Layer

> Synthesized from: tauri (desktop ACL/IPC), rustls (TLS/crypto), aya (eBPF/kernel enforcement)
> Purpose: Architectural judgment for geist.sh's shell layer (L1/L2)
> Scope: Deny-first policy, trust boundaries, compile-time security, permission namespacing,
> path security, cryptographic isolation, eBPF enforcement

---

## Table of Contents

1. [Deny-First Policy Enforcement (x.uma)](#1-deny-first-policy-enforcement-xuma)
2. [Trust Boundaries (agent-shell, shell-kernel)](#2-trust-boundaries)
3. [Compile-Time vs Runtime Security Trade-offs](#3-compile-time-vs-runtime-security-trade-offs)
4. [Permission Namespacing for Agents/Plugins](#4-permission-namespacing-for-agentsplugins)
5. [Path/Scope Security](#5-pathscope-security)
6. [Cryptographic Isolation](#6-cryptographic-isolation)
7. [eBPF Enforcement Patterns](#7-ebpf-enforcement-patterns)
8. [Cross-Cutting Principles](#8-cross-cutting-principles)

---

## 1. Deny-First Policy Enforcement (x.uma)

The central invariant: **denied operations are checked before allowed operations, and a deny is absolute.** This must hold across every permission boundary in geist.sh -- command ACL, filesystem scopes, network policy, agent capabilities.

### The Deny-First Invariant

Tauri's `RuntimeAuthority` checks `denied_commands` before `allowed_commands`. In the filesystem scope, `forbidden_patterns` are matched before `allowed_patterns`. This ordering is not configurable. (tauri insight #3)

```
resolve_access(command):
  if command in denied_commands -> REJECT (absolute, no override)
  if command in allowed_commands -> check origin, scope, context
  else -> REJECT (default deny)
```

Rustls enforces an analogous pattern: the `ConfigBuilder` typestate makes it impossible to create a config without specifying a verifier. The "default" is not "allow all" -- it is "you must decide." The only way to weaken verification is through the `dangerous()` API, which requires four separate acts of acknowledgment (method name, module name, type name, trait implementation). (rustls insight #8)

**Transferable principle for x.uma:** Deny-first is not a check ordering -- it is a composition guarantee. When multiple policies compose (agent policy + plugin policy + system policy), any deny from any source must be final. This makes policy composition safe: adding a deny can never be overridden by an allow from a different source.

### Allowlist-to-ACL Evolution

Tauri's v1 used a boolean allowlist per API. This broke when the model needed: multi-origin (remote URLs vs local app), multi-plugin (each plugin needs its own permission space), and scoped access (path-specific filesystem grants). The migration to full ACL (PR #8428) was forced by composition requirements, not complexity preference. (tauri insight #2)

**Transferable principle for x.uma:** Start with the ACL model, not the allowlist. geist.sh will have multi-agent, multi-plugin, and scoped access from day one. A boolean allowlist will not survive first contact with real agent policies.

### Verification as Capability Types

Rustls encodes verification completion as zero-sized, non-copyable types (`HandshakeSignatureValid`, `PeerVerified`, `FinishedMessageVerified`). The terminal traffic state requires values of all three types. If any verification step is skipped, the state cannot be constructed -- this is a compile-time guarantee. (rustls insight #2)

**Transferable principle for x.uma:** Policy evaluation results should be capability types, not booleans. A `PolicyApproved` zero-sized type produced only by the policy engine, consumed by the execution gate, makes policy bypass structurally impossible. A boolean `is_allowed: bool` can be set to `true` anywhere.

---

## 2. Trust Boundaries

geist.sh has three trust boundaries: agent-to-shell (L4-L2), shell-to-kernel (L2-L1), and agent-to-agent (lateral). Each boundary requires different enforcement mechanisms.

### Agent-to-Shell: IPC with Authority Checks

Tauri's IPC runs every command through `RuntimeAuthority` before execution. Every call carries its `Origin` (Local vs Remote). The authority check is in the framework's IPC protocol, not delegated to the webview's same-origin policy. (tauri insights #3, #10)

Tauri's Channel sends `{ end: true }` on drop, ensuring the JavaScript side knows the Rust resource is gone. Resource lifecycle spans the IPC boundary. (tauri insight #14)

**Transferable principle:** The shell must enforce its own authority on every agent IPC call, independent of any transport-level security. Origin tracking (which agent, which plugin, which scope) travels with every request. Resource cleanup must be protocol-obligated, not optional.

### Shell-to-Kernel: The Dual-World Problem

aya's architecture splits cleanly between userspace (aya/) and kernel (aya-ebpf/). They share data through BPF maps and file descriptors but never share code. The `unsafe trait Pod` guards every byte that crosses this boundary -- non-Pod types (containing references, vtable pointers, padding) would produce undefined behavior. (aya core abstractions: Pod)

The kernel boundary is also a privilege boundary. MockableFd exists because kernel resources (file descriptors) require special handling -- Rust 1.80 aborts the process if you close an invalid fd. Real kernel interaction cannot be mocked by simply not calling syscalls; you must intercept at the syscall dispatch layer. (aya insight #3)

**Transferable principle:** The shell-to-kernel boundary is a serialization boundary (Pod), a privilege boundary (CAP_BPF), and a lifecycle boundary (OwnedFd). Types that cross it must satisfy all three constraints simultaneously.

### Lateral: Agent-to-Agent Isolation

Tauri's event system has two privilege levels: scoped listeners (`listen()`/`once()`) and sniffers (`listen_any()`/`once_any()`). Sniffing all events is a privileged operation through the Manager trait. (tauri insight #19)

**Transferable principle:** Agents should not be able to observe each other's policy evaluations, IPC traffic, or resource handles unless explicitly granted sniffer-level access. Default is isolation; cross-agent visibility is a capability.

---

## 3. Compile-Time vs Runtime Security Trade-offs

This is the most consequential architectural decision for geist.sh: what is frozen at compile time versus evaluated at runtime.

### Compile-Time: Configuration as Binary Artifact

Tauri's `tauri.conf.json` is parsed at compile time by `tauri-codegen`. Assets are embedded. The Config struct is generated. There is no runtime config loading. Rationale: if an attacker gains code execution in the agent (webview), they cannot modify the security policy because it is baked into the binary. (tauri insight #12)

Rustls's `#![forbid(unsafe_code)]` is the extreme compile-time position: the entire crate delegates all unsafe operations to external crates, audited separately. This constraint created the `CryptoProvider` abstraction, which accidentally solved the FIPS pluggability problem four years later. (rustls insights #1, #21)

**Trade-off:** Compile-time config means slower iteration during development. Tauri resolves this with a permissive dev mode. geist.sh should similarly distinguish dev-mode (relaxed policy evaluation) from production (frozen policy).

### Runtime: Feature Detection and Capability Negotiation

aya's feature detection probes the running kernel by attempting syscalls and interpreting error codes. This is necessary because kernel version numbers lie -- distributions backport features, embedded kernels strip them. The only ground truth is empirical: "did the kernel accept this specific operation?" (aya insight #4)

Rustls's FIPS status is computed at runtime as the minimum across all component statuses. Each crypto component reports its own status; the aggregate is the weakest link. This allows mixed configurations to report accurately. (rustls insight #10)

**Transferable principle for geist.sh:**
- **Policy rules**: compile-time (frozen into the shell binary or a signed policy artifact)
- **Kernel capabilities**: runtime (probe what the kernel supports via eBPF feature detection)
- **Agent capabilities**: runtime (negotiated at agent registration, evaluated against frozen policy)
- **Crypto provider**: runtime selection from compile-time-verified options (rustls model)

### The Spectrum

| Security property | Compile-time | Runtime | geist.sh recommendation |
|---|---|---|---|
| Deny rules | Frozen | -- | Compile-time (cannot be weakened) |
| Allow rules | Default set frozen | Agent-specific grants at registration | Hybrid |
| Kernel enforcement | eBPF programs compiled | Feature probing at load | Runtime detection |
| Crypto provider | Available providers compiled in | Selection at startup | Runtime from compiled set |
| Policy schema | Type-checked at build | -- | Compile-time |

---

## 4. Permission Namespacing for Agents/Plugins

### Tauri's Plugin ACL Namespacing

Each plugin command is namespaced as `plugin:<name>|<command>`. The `__app-acl__` key is reserved for application-level commands. This prevents: plugins shadowing each other's permissions, a malicious plugin declaring permissions that affect other plugins, and namespace collisions between plugin and application commands. (tauri insight #11)

### Scoped Permissions

Tauri's ACL goes beyond command-level: each permission has an associated scope. A filesystem permission isn't just "allow fs read" -- it specifies which paths, with deny patterns. The `ScopeManager` tracks per-command scopes resolved at compile time. (tauri insight #2)

### Rustls's Sealed Types as Namespace Enforcement

Rustls seals its `ConfigSide` trait so only `ClientSide` and `ServerSide` can implement it. This prevents user-defined sides that might bypass verification. The pattern uses a private `sealed::Sealed` trait. (rustls insight #19)

**Transferable principle for geist.sh:**

Permission namespacing for agents:
```
agent:<agent-id>|<capability>
plugin:<plugin-id>|<capability>
system:<shell-capability>
```

Each namespace is a closed world. An agent cannot declare permissions in another agent's namespace. The shell's system namespace is sealed (like rustls's ConfigSide) -- only the shell binary can grant system capabilities.

Scoped permissions are essential from the start. "agent:writer|fs:write" must include the path scope. Without scopes, the permission is either too broad (allow all writes) or too narrow (allow nothing).

---

## 5. Path/Scope Security

Path handling is a persistent security surface. Three codebases, three sets of battle scars.

### Glob Pattern Semantics Are a Security Surface

Tauri's GHSA-6mv3-wm7j-h4w5: without `require_literal_separator: true`, the pattern `/dir/*` matches `/dir/subdir/secret.txt`. A single boolean controls whether a glob scope is directory-bounded or tree-bounded. (tauri insight #4)

**Transferable principle:** Every glob pattern in geist.sh's scope system must use `require_literal_separator: true`. This is non-negotiable. Document it as a security invariant, not a configuration option.

### Path Canonicalization vs Glob Patterns

Tauri discovered that path canonicalization libraries (like `dunce`) assume valid paths. Glob patterns contain `*`, which is not valid in Windows paths. The intersection -- canonicalizing a scope pattern -- requires manual handling. (tauri insight #5)

Symlinks must be dereferenced before scope matching. Non-existent paths must be allowed (they are glob patterns). Existing paths must be fully canonicalized. Three behaviors for three input categories. (tauri insight #15)

**Transferable principle:** geist.sh's scope matching must handle three path categories:
1. **Symlinks**: dereference before matching (prevents escape via symlink creation)
2. **Glob patterns**: allow non-existent paths, enforce `require_literal_separator`
3. **Real paths**: full canonicalization (resolve `.` and `..`)

Missing any category is a scope escape vulnerability.

### Cross-Platform Path Normalization

Tauri handles Windows UNC paths, Android symlink chains (`/data/user/0/` -> `/data/data/`), and macOS case sensitivity. Each platform has its own path normalization rules. (tauri insights #5, #6)

**Transferable principle:** geist.sh's shell layer runs on Linux (primary target for eBPF). Path normalization is simpler but not trivial -- `/proc/self/` resolution, mount namespace awareness, and overlay filesystem handling are the Linux-specific concerns.

---

## 6. Cryptographic Isolation

### In-Process Trust Domains

Tauri's isolation mode creates a cryptographic barrier within a single process. The isolation iframe encrypts all IPC messages with AES-256-GCM. The key is per-window, generated from CSPRNG, embedded at build time, and the script removes itself from DOM after execution. (tauri insight #9)

**Transferable principle:** When geist.sh cannot trust agent-produced content (agents can be compromised), add a cryptographic barrier at the IPC boundary. This is not about network encryption -- it is about in-process trust domain separation. The shell's IPC handler should verify message integrity before policy evaluation.

### Automated Cryptographic Hygiene

Rustls automates sequence number tracking and key rotation. At `SEQ_SOFT_LIMIT`, the library sends a key update (TLS 1.3) or close (TLS 1.2). At `SEQ_HARD_LIMIT`, it refuses to encrypt rather than reuse a key. Applications are not trusted to manage key rotation. (rustls insight #6)

Rustls defaults to post-quantum key exchange (X25519MLKEM768) because the cost is low and the "harvest now, decrypt later" threat is real. The secure choice is the default; the user must actively choose less security. (rustls insight #12)

**Transferable principle:** geist.sh should automate cryptographic lifecycle management for agent sessions. Key rotation, session ticket management, and nonce tracking should be shell responsibilities, not agent responsibilities. Default to the strongest available crypto; make weakening explicit.

### Pluggable Crypto Without Mega-Traits

Rustls's CryptoProvider evolved from hardcoded ring, to a trait (0.22), to a struct (0.23). The struct approach enables partial customization via struct update syntax -- override one field without reimplementing everything. (rustls insight #3)

**Transferable principle:** If geist.sh needs pluggable crypto (e.g., for FIPS environments), use a struct of trait objects, not a single CryptoProvider trait. This allows operators to swap just the key provider (e.g., HSM) without touching cipher suite selection.

---

## 7. eBPF Enforcement Patterns

These patterns are specific to geist.sh's L1 (kernel) layer, where eBPF programs enforce policy below the application layer.

### The Loading Ceremony

aya's EbpfLoader orchestrates seven steps: parse ELF, patch globals, fixup BTF, relocate BTF, create maps, relocate maps, load programs. Each step has preconditions that the previous step establishes. (aya core abstractions: EbpfLoader)

**Transferable principle:** geist.sh's eBPF program loading should be a single, validated ceremony -- not a sequence of independent steps that callers might reorder or skip. The `EbpfLoader` builder pattern is the right model.

### Feature Detection by Probing

aya detects kernel capabilities by attempting syscalls and interpreting error codes. `EINVAL` means "type unknown," `EBADF` means "type recognized, BTF lookup failed" (supported!), `524` (ENOTSUPP) means "reached type-specific validation" (supported!). (aya insight #4)

The arm64 LSM trap: some arm64 kernels can load LSM programs but cannot attach them. Detection requires probing both load and attach. (aya insight #4)

**Transferable principle:** geist.sh must probe every eBPF capability it intends to use. The probe results should be cached (aya uses `LazyLock<Features>`) and consulted before any policy enforcement program is loaded. A capability matrix, not a version check, determines what enforcement is available.

### Pod Trait: The Boundary Serialization Contract

`unsafe trait Pod: Copy + 'static` guards every byte crossing the kernel/userspace boundary. The `Copy` bound ensures no destructors; `'static` ensures no lifetime dependencies. Non-Pod types (references, vtable pointers, uninitialized padding) produce UB when reinterpreted across the boundary. (aya core abstractions: Pod)

`repr(transparent)` is required for Pod soundness on newtype wrappers -- without it, the compiler may add padding or change layout. (aya insight #26: RFC 1758)

**Transferable principle:** All types shared between geist.sh's userspace shell and kernel eBPF programs must implement Pod. This is not optional. Define a geist.sh-specific Pod trait (or use aya's) and audit every implementing type for padding, pointers, and alignment.

### Map Shared State: Kernel Memory vs Rust Aliasing

aya discovered that `HashMap::get_mut() -> &mut V` is unsound for pre-allocated BPF maps because the kernel can mutate values in-place concurrently. The fix: remove `get_mut`, expose only `get() -> Option<&V>` (still unsafe due to tearing) and `get_ptr_mut() -> Option<*mut V>` (honest raw pointer). (aya PR #290)

**Transferable principle:** BPF maps shared between geist.sh's shell and eBPF programs must use raw pointer access, not Rust references. The kernel's RCU memory model does not provide Rust's aliasing guarantees. Raw pointers are the honest API for cross-boundary shared state.

### Ring Buffer Protocol

The kernel-side `RingBufEntry` carries `#[must_use]` because the eBPF verifier requires every `reserve` to be matched by `submit` or `discard`. Userspace consumption uses `SeqCst` ordering paired with the kernel's `xchg` on commit. Every memory ordering decision cites the specific kernel source line it pairs with. (aya insight #6)

**Transferable principle:** geist.sh's policy event channel (eBPF -> userspace) should use RingBuf, not PerfEventArray, for new kernel targets (>= 5.8). The lock-free protocol requires precise memory ordering -- copy aya's commenting discipline of citing kernel source lines for every ordering choice.

### Verifier Compatibility

The eBPF verifier is a static analyzer with specific pattern recognition. Standard Rust comparisons may compile to instruction sequences the verifier rejects. aya provides `check_bounds_signed` with hand-written BPF assembly in the exact conditional form the verifier understands. (aya insight #9)

**Transferable principle:** geist.sh's eBPF programs must be tested against the verifier on every target kernel version. Bounds checks in security-critical paths should use verifier-compatible patterns, not rely on the Rust compiler's instruction selection.

### Drop-as-Detach Lifecycle

When aya's `Ebpf` object is dropped, all programs are detached. Persistence requires explicit `.pin()` to bpffs. RAII cleanup is the default; persistence is opt-in. (aya insight #15)

**Transferable principle:** geist.sh's enforcement programs should detach on shell shutdown by default. Persistent enforcement (surviving shell restart) requires explicit pinning to bpffs. This prevents orphaned enforcement programs with stale policy.

---

## 8. Cross-Cutting Principles

These principles emerge from all three codebases and apply broadly to geist.sh's architecture.

### Protocol Obligations Over Convenience

All three codebases confirm this pattern:
- **Tauri:** Deny is absolute. IPC requires authority. Channel drop notifies JS. (tauri cross-codebase pattern)
- **Rustls:** Verification markers must exist to reach traffic state. Builder requires verifier selection. `forbid(unsafe_code)` is crate-wide. (rustls cross-codebase pattern)
- **aya:** Pod trait for boundary types. Seven-step loading ceremony. Ring buffer reserve/submit obligation. (aya cross-codebase pattern)

**For geist.sh:** Every security-critical operation should be a protocol obligation, not a convenience check. Policy evaluation is not "call this function and check the bool" -- it is "produce a capability type that the execution gate consumes."

### Make the Secure Path the Easy Path

Rustls's `dangerous()` API requires four separate acknowledgments to weaken security. The normal API path cannot skip verification. (rustls insight #8)

Tauri's dev mode is permissive, but production mode is deny-first by default. The transition from dev to production tightens security automatically. (tauri insight #12)

**For geist.sh:** The default agent policy should be deny-all. Granting capabilities should require explicit, auditable configuration. The shell's API should make it impossible to accidentally run an agent without policy evaluation.

### Type-Level Guarantees Reduce Audit Surface

Rustls's Cure53 audit found zero exploitable vulnerabilities in 30 person-days. The auditors specifically cited typestate patterns and static encoding of properties as reducing the reviewable surface area. (rustls insight #13)

**For geist.sh:** Every type-level guarantee (capability types, Pod trait, sealed permission namespaces) reduces the surface area that security reviewers must manually verify. The goal is: "check that the types are correctly defined" rather than "check all execution paths."

### State Cardinality Reduction

aya systematically eliminates impossible states: `Option<(A, B)>` instead of `(Option<A>, Option<B>)`, factory patterns that return owned values instead of `Option<fd>`, typed syscall returns instead of `c_long`. (aya insights #20, #19, #22)

**For geist.sh:** Every `Option` in a security-relevant type should be audited. Can the None case actually occur? If not, eliminate it. Fewer states means fewer branches means fewer places for security bugs to hide.

### Empirical Capability Detection

aya probes the kernel. rustls negotiates TLS versions. tauri detects platform capabilities per-API.

**For geist.sh:** The shell must know what enforcement is available before making policy decisions. If an eBPF program type is not supported on the running kernel, the shell must either fall back to a weaker enforcement mechanism or refuse to start. Silent degradation of enforcement is a security vulnerability.

---

## Source Traceability Index

| Tag | Source | Insight |
|-----|--------|---------|
| tauri #1 | tauri-mastery.md | Security was PR #10 -- architecture, not feature |
| tauri #2 | tauri-mastery.md | Allowlist to ACL: simple models don't scale |
| tauri #3 | tauri-mastery.md | Deny takes precedence -- always |
| tauri #4 | tauri-mastery.md | Glob pattern security (GHSA-6mv3-wm7j-h4w5) |
| tauri #5 | tauri-mastery.md | Windows path normalization / canonicalization |
| tauri #6 | tauri-mastery.md | Android path canonicalization walks the tree |
| tauri #8 | tauri-mastery.md | CSP isolation oscillation |
| tauri #9 | tauri-mastery.md | AES-256-GCM isolation -- in-process trust boundary |
| tauri #10 | tauri-mastery.md | Origin-based IPC |
| tauri #11 | tauri-mastery.md | Plugin ACL namespacing |
| tauri #12 | tauri-mastery.md | Compile-time configuration as security choice |
| tauri #14 | tauri-mastery.md | Channel drop = protocol obligation |
| tauri #15 | tauri-mastery.md | Symlink resolution as security boundary |
| tauri #19 | tauri-mastery.md | Event system broadcast vs sniffer privilege |
| rustls #1 | rustls-mastery.md | `forbid(unsafe_code)` as architectural decision |
| rustls #2 | rustls-mastery.md | Verification obligations as capability types |
| rustls #3 | rustls-mastery.md | CryptoProvider extraction arc (trait to struct) |
| rustls #6 | rustls-mastery.md | Sequence number exhaustion as automatic key rotation |
| rustls #8 | rustls-mastery.md | `dangerous()` API -- graduated escape hatches |
| rustls #10 | rustls-mastery.md | FIPS as runtime property, not compile-time flag |
| rustls #12 | rustls-mastery.md | Post-quantum key exchange as default |
| rustls #13 | rustls-mastery.md | Cure53 audit verdict -- type-level guarantees reduce audit surface |
| rustls #19 | rustls-mastery.md | Sealed traits for typestate security (RFC 3323) |
| rustls #21 | rustls-mastery.md | `forbid(unsafe_code)` creates architectural pressure |
| aya #3 | aya-mastery.md | MockableFd evolution -- testing at syscall boundary |
| aya #4 | aya-mastery.md | Feature probing by syscall error code |
| aya #6 | aya-mastery.md | RingBuf memory ordering contract |
| aya #9 | aya-mastery.md | check_bounds_signed -- verifier compatibility |
| aya #15 | aya-mastery.md | Drop-as-detach lifecycle |
| aya #19 | aya-mastery.md | OwnedFd migration -- seven PRs to RAII |
| aya #20 | aya-mastery.md | State cardinality reduction |
| aya #22 | aya-mastery.md | Encoding syscall contracts in types |
| aya #26 | aya-mastery.md | repr(transparent) for Pod soundness (RFC 1758) |
| aya PR #290 | aya-mastery.md | HashMap unsoundness -- kernel memory vs Rust aliasing |
