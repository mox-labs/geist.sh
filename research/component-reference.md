# geist.sh Component Reference

> **Date**: February 13, 2026
> **Purpose**: What each technology in the geist.sh stack actually is, who makes it, what's verified, what's not
> **Rule**: Every claim here must trace to a specific source. If unverified, it says so.

---

## Kernel / OS Enforcement

### seccomp BPF

**What it is**: Linux kernel feature that restricts which system calls a process can make. A process installs a BPF (Berkeley Packet Filter) bytecode program that inspects each syscall and returns allow/deny/kill. Once installed, filters cannot be removed or relaxed — only tightened.

**Origin**: seccomp added to Linux 2.6.12 (2005) by Andrea Arcangeli. Filter mode (seccomp-BPF, SECCOMP_MODE_FILTER) added in Linux 3.5 (2012), reusing the BPF bytecode VM originally designed for network packet filtering (1992, tcpdump).

**Who uses it**: Chrome/Chromium, Docker, systemd, Android (since 8.0 Oreo), OpenSSH, QEMU. Anthropic's sandbox-runtime ships pre-compiled seccomp BPF filters for x64 and arm64.

**In geist.sh context**: srt already uses seccomp BPF on Linux to block unix socket creation. This is the *static* layer — binary allow/deny per syscall, no state, no observation. It's the floor, not the ceiling.

**Key limitation**: Classic BPF can only inspect syscall number and register values. Cannot inspect pointer arguments (file paths, buffer contents). Cannot maintain state across calls. Cannot be modified after loading. Programs limited to 4,096 BPF instructions with forward-only branches (guaranteed termination). This is why eBPF exists.

**Notable**: `SECCOMP_RET_USER_NOTIF` (Linux 5.0, 2019) added user-space notification — the filter can defer decisions to a supervisor process that *can* read `/proc/PID/mem` to inspect pointer arguments. This partly closes the "can't see file paths" gap but at the cost of a context switch to userspace. Kees Cook (Google) has been the primary steward of seccomp development.

**Source**: [Linux kernel docs](https://docs.kernel.org/userspace-api/seccomp_filter.html), [Wikipedia](https://en.wikipedia.org/wiki/Seccomp), [LWN overview](https://lwn.net/Articles/656307/), [seccomp_unotify(2)](https://man7.org/linux/man-pages/man2/seccomp_unotify.2.html)

---

### eBPF

**What it is**: Extended Berkeley Packet Filter — a Linux kernel subsystem that lets userspace programs run sandboxed bytecode inside the kernel. Unlike classic BPF (static, fire-and-forget), eBPF programs are dynamically loaded/unloaded, can maintain state in maps (hash tables, ring buffers, etc.), and attach to a wide variety of kernel hooks (kprobes, tracepoints, LSM hooks, network hooks, etc.).

**Origin**: Introduced in Linux 3.18 (2014) by Alexei Starovoitov. Has since become one of the most actively developed kernel subsystems. The eBPF Foundation (Linux Foundation, founded 2021) governs the ecosystem. Founding members: Meta, Google, Isovalent (now Cisco), Microsoft, Netflix.

**Key capabilities for geist.sh**:
- **kprobes/kretprobes**: Attach to any kernel function. Observe execve, openat, connect, etc.
- **uprobes**: Attach to userspace function entries. Can probe SSL_read/SSL_write for network content observation.
- **BPF-LSM**: Attach to Linux Security Module hooks. Same hooks as AppArmor/SELinux but programmable. Can enforce policy *with* context (file paths, socket addresses, process identity) — unlike seccomp which only sees register values.
- **Maps**: Shared state between eBPF programs and userspace. This is how you correlate: store intent in a map from userspace, eBPF program checks intent against observed action at kernel level.
- **Ring buffers**: Efficient kernel→userspace event streaming.

**Maturity**: Production at Meta (every packet), Cloudflare (DDoS mitigation), Google (GKE Dataplane V2), Netflix, many others. Not experimental.

**Key limitations**:
- Linux-only. No macOS equivalent. Microsoft has a separate eBPF runtime for Windows, but it's a different project. For development on Darwin, a trait-based userspace fallback is needed (see ADR-003 in synthesis-geist-adr.md).
- 512-byte stack limit per BPF program (workarounds exist with per-CPU arrays/maps).
- Programs must pass the kernel verifier — constraints on loops, memory access patterns, and complexity.
- Requires root or `CAP_BPF` capability.

**Source**: [ebpf.io](https://ebpf.io), [eBPF Foundation](https://ebpf.foundation/), kernel docs, [eBPF ecosystem progress 2024-2025](https://eunomia.dev/blog/2025/02/12/ebpf-ecosystem-progress-in-20242025-a-technical-deep-dive/). Verified in our research synthesis.

---

### Aya-rs

**What it is**: Pure Rust library for writing, loading, and managing eBPF programs. No libbpf dependency, no C. Supports BTF (BPF Type Format) and CO-RE (Compile Once, Run Everywhere) for portability across kernel versions.

**Who maintains it**: Open source project at [github.com/aya-rs/aya](https://github.com/aya-rs/aya). Multiple contributors.

**Current version**: **0.13.1** on crates.io. Pre-1.0 — API still evolving. Primary developers include Dave Tucker (Microsoft) and Alessandro Decina.

**Maturity**: Supports 22 eBPF program types. Production use at Deepfence (eBPFGuard), Red Hat (bpfman), Exein (Pulsar), Kubernetes Gateway API SIG (Blixt load balancer). **[Verified — aya-rs.dev, GitHub repos]**

**In geist.sh context**: This is how geist.sh writes custom eBPF programs in Rust. For Phase 2 contract-aware enforcement: parse behavioral contracts at deploy time → generate eBPF maps with per-component policies → load LSM/kprobe programs that check actions against policy maps.

**Why not libbpf-rs?** Aya-rs is pure Rust (no C toolchain), aligns with our Rust-native approach. libbpf-rs wraps the C libbpf library. Both work; Aya-rs has more Rust-idiomatic ergonomics.

**Build requirement**: Requires **nightly Rust toolchain** for eBPF program compilation (the kernel-side `aya-ebpf` crate). The Rust eBPF target (`bpf-unknown-none`) is not yet stabilized in upstream rustc; Aya uses a custom `bpf-linker`.

**Source**: [aya-rs.dev](https://aya-rs.dev), [docs.rs/aya/0.13.1](https://docs.rs/aya/0.13.1/aya/), production users verified in research synthesis

---

### Cilium Tetragon

**What it is**: eBPF-based security observability and runtime enforcement tool. CNCF project under the Cilium umbrella. Define policies as TracingPolicy CRDs (YAML), Tetragon compiles them to eBPF programs and loads them. Kubernetes-aware: understands pods, namespaces, labels.

**Who maintains it**: Isovalent (now Cisco) + CNCF community. v1.6.0 at time of our research. **[Verified — tetragon.io]**

**In geist.sh context**: Phase 1 baseline security. Instead of writing raw eBPF, define standard security policies as Tetragon TracingPolicies. Block unauthorized egress, monitor file access, detect breakout attempts. Lower engineering cost than custom Aya-rs programs.

**The two-tool strategy**: Tetragon for standard enforcement (YAML policies, well-tested). Aya-rs for novel, contract-aware enforcement (custom programs). Tetragon handles the 80% case; Aya-rs handles the geist.sh-specific 20%.

**Key limitations**: Kubernetes-centric. Works outside K8s but designed for it. For local development (single machine, no K8s), some features may not apply directly. Requires kernel >= 4.19 (optimal >= 5.11 for ring buffer support). v1.6.0 defaults to non-root Operator (UID 65532) and uses BPF ring buffer from kernel 5.11+.

**Source**: [tetragon.io](https://tetragon.io), [GitHub releases](https://github.com/cilium/tetragon/releases/tag/v1.6.0), CNCF project listing. Verified.

---

### Deepfence eBPFGuard

**What it is**: Aya-rs-based LSM policy enforcement. Rust/YAML configurable policies for BPF-LSM hooks. Closest existing example to what geist.sh's contract-to-eBPF compiler would produce.

**In geist.sh context**: Reference implementation / starting point for contract-aware LSM enforcement. Not a dependency — a pattern to learn from.

**Source**: [github.com/deepfence/ebpfguard](https://github.com/deepfence/ebpfguard). Verified.

---

## OS Sandboxing (used by srt)

### bubblewrap (bwrap)

**What it is**: Minimal unprivileged sandboxing tool for Linux. Uses kernel namespaces (mount, user, PID, network, IPC) to create isolated environments. No root required.

**Origin**: Created by Alexander Larsson at Red Hat in 2016, extracted from Flatpak. Now maintained under the `containers` org on GitHub. v0.11.0 current. **[Verified — GitHub]**

**How srt uses it**: On Linux, srt wraps commands with `bwrap` to create filesystem namespaces. Allowed paths are bind-mounted in; everything else is invisible. Combined with seccomp BPF for syscall filtering.

**Key property**: Unlike Docker, bwrap doesn't require a daemon or root privileges. It creates a lightweight namespace around a single command. Fast to set up, minimal overhead. v0.11.0 added `--[ro-]bind-fd` to address CVE-2024-42472 (TOCTOU vulnerability in bind mounts).

**Also used by**: Flatpak (millions of users), GNOME Thumbnailer, WebKitGTK web process sandbox, OpenAI Codex (code execution sandboxing on Linux).

**Source**: [github.com/containers/bubblewrap](https://github.com/containers/bubblewrap), [Arch Wiki](https://wiki.archlinux.org/title/Bubblewrap)

---

### macOS sandbox-exec / seatbelt

**What it is**: Apple's application sandboxing mechanism. Uses Scheme-based sandbox profiles to define what a process can do. The kernel enforces the profile — there is no userspace bypass.

**How srt uses it**: Generates a complete sandbox profile per command with `(deny default)`, explicit allow rules for needed operations, and deny rules for protected paths. Wraps the command with `env ... sandbox-exec -p <profile> <shell> -c <command>`.

**Key properties**:
- Deny-default: everything blocked unless explicitly allowed
- Supports regex and glob matching for file paths
- Network rules: can restrict to specific localhost ports only (forces proxy routing)
- Violation logging via `log stream` with per-command tags for correlation

**Key limitations**:
- Apple considers `sandbox-exec` **deprecated** (man page says so since macOS 15 Sequoia). But the underlying kernel mechanism (`Sandbox.kext`, later integrated into the kernel) is still fully functional — Apple's own App Sandbox, system daemons, and XPC services depend on it.
- The SBPL (Sandbox Profile Language) is **undocumented**. Community knowledge is reverse-engineered from system profiles in `/System/Library/Sandbox/Profiles/`.
- Apple can change SBPL syntax between macOS versions without notice.

**Who else uses it**: Chromium (renderer + GPU process sandbox), Google's Gemini CLI, OpenAI's Codex CLI. It's the standard for CLI-level sandboxing on macOS despite the deprecation notice.

**Source**: srt source code (macos-sandbox-utils.ts), [Chromium seatbelt design](https://github.com/chromium/chromium/blob/main/sandbox/mac/seatbelt_sandbox_design.md), [The Apple Wiki - Seatbelt](https://theapplewiki.com/wiki/Dev:Seatbelt)

---

## Protocols

### MCP (Model Context Protocol)

**What it is**: Anthropic's open protocol for connecting AI agents to tools and data sources. Client-server architecture: an AI agent (MCP client) connects to MCP servers that expose tools, resources, and prompts via JSON-RPC over stdio or HTTP+SSE.

**Spec versions**:
- 2024-11-05: Initial release
- 2025-03-26: Added JSON-RPC batching
- 2025-06-18: Removed batching, added structured tool output, OAuth security, elicitation (server-initiated user interaction)
- 2025-11-25: Added Tasks (tracking server-side work), better context control

**Adoption**: 97M monthly SDK downloads, 10,000+ servers. Donated to the **Agentic AI Foundation (AAIF)** under the Linux Foundation in December 2025, co-founded by Anthropic, Block, and OpenAI. SDKs in TypeScript, Python, Java, C#, Rust, Go, Kotlin, Swift. **[Verified — multiple industry sources]**

**In geist.sh context**: MCP is how Claude Code talks to tools. geist.sh doesn't need to implement MCP — Claude Code already is an MCP client. geist.sh's job is configuring *which* MCP servers are available in a given session (per-intent assembly) and *observing* the MCP calls being made (intent-action correlation).

**Key architectural property**: MCP is *instrumental*, not *constitutive*. Tools are instruments wielded by an orchestrator. No server-to-server communication. No inter-tool state sharing. This is by design. **[Verified — MCP spec]**

**Source**: [modelcontextprotocol.io](https://modelcontextprotocol.io), spec versions verified via [blog](http://blog.modelcontextprotocol.io/posts/2025-11-25-first-mcp-anniversary/), [GitHub](https://github.com/modelcontextprotocol/modelcontextprotocol)

---

### A2A (Agent-to-Agent Protocol)

**What it is**: Google's open protocol for agent-to-agent communication. Designed for opaque agents — they collaborate "without needing to share their internal state, memory, or tools." HTTP-based, supports streaming via SSE and push notifications.

**Versions**: v0.1 (April 2025, initial launch) → v0.2 (stateless interaction, standardized auth) → **v0.3** (July 31, 2025, gRPC support, signed Agent Cards). No v0.4 verified as of our research. Originally Google, now donated to Linux Foundation. 150+ supporting organizations including Atlassian, Salesforce, SAP, ServiceNow, LangChain, MongoDB, PayPal. **[Verified — Google Cloud Blog, Linux Foundation]**

**In geist.sh context**: A2A is for *inter-agent* communication. If geist.sh sessions need to collaborate with external agents, A2A is the wire protocol. Not needed for MVP (single Claude Code session), but relevant for multi-agent scenarios.

**Key architectural property**: Deliberately keeps agents opaque. Precludes constitutive integration by design. This is the complement to MCP: MCP connects agent↔tool, A2A connects agent↔agent. **[Verified — A2A spec]**

**Source**: [a2a-protocol.org](https://a2a-protocol.org), [Google Developers Blog](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/)

---

### wRPC

**What it is**: WIT-based RPC protocol from the Bytecode Alliance. Components communicate through shared typed interfaces (WIT — WebAssembly Interface Types) rather than through a central orchestrator. Typically runs over NATS.

**In geist.sh context**: wRPC is the closest existing primitive for *constitutive* composition — direct peer-to-peer communication with type-safe interfaces. Used by wasmCloud's lattice. Not needed for MVP, but represents the path toward true component composition if geist.sh evolves beyond orchestrated tool-use.

**Current version**: `wrpc` crate 0.16.0, `wrpc-transport` 0.28.x (sub-crates versioned independently). Pre-1.0. [Protocol spec](https://github.com/bytecodealliance/wrpc/blob/main/SPEC.md) exists but is not yet finalized (v0.1.0 formalization in progress). Primary author: Roman Volosatovs. Transports: TCP, Unix Domain Sockets, QUIC, NATS (only NATS has seen significant production use).

**Maturity**: Niche but technically strongest for typed composition. **[Moderate confidence — Bytecode Alliance project, limited production evidence outside wasmCloud]**

**Source**: [github.com/bytecodealliance/wrpc](https://github.com/bytecodealliance/wrpc), [crates.io/crates/wrpc](https://crates.io/crates/wrpc), wasmCloud docs

---

## Language FFI

### PyO3

**What it is**: Rust bindings for Python. Lets you call Python from Rust and vice versa. Write Python modules in Rust, or embed a Python interpreter in a Rust application.

**Current version**: **0.28.1**. Supports Python 3.7-3.14, including free-threaded Python 3.14t (GIL-free). 13k+ GitHub stars. Minimum Rust **1.83** (raised in 0.28.x). Community-maintained; primary maintainer David Hewitt.

**Key features**: GIL release for parallel Rust execution, zero-copy numpy interop, `#[pymodule]` / `#[pyfunction]` / `#[pyclass]` procedural macros. Since 0.28, defaults to assuming modules are thread-safe for free-threaded Python (opt-out with `#[pymodule(gil_used = true)]`).

**Notable users**: pydantic-core (Pydantic v2), polars, ruff, cryptography, orjson, tokenizers (Hugging Face). Build tool: [maturin](https://github.com/PyO3/maturin).

**In geist.sh context**: Originally planned for embedding Python in the Rust runtime (ADR-001: Rust + PyO3 + eBPF). With the reframe to Claude Code as the agent, PyO3 is less critical for MVP — Claude Code is Node.js, not Python. PyO3 becomes relevant for Phase 2+ when custom Python-based agent workloads or component execution needs embedded Python.

**ADR-001 update**: See sandbox-runtime-analysis.md for the reconsidered role of PyO3 given the Claude Code reframe.

**Source**: [pyo3.rs](https://pyo3.rs), [GitHub](https://github.com/pyo3/pyo3)

---

## Anthropic's Stack (what already exists)

### sandbox-runtime (srt)

**What it is**: `@anthropic-ai/sandbox-runtime` — Anthropic's OS-level sandboxing library for Claude Code. TypeScript library + CLI. See `sandbox-runtime-analysis.md` for full analysis.

**Version**: 0.0.35, Apache-2.0.

**What it provides that geist.sh doesn't need to build**: Filesystem isolation, network domain filtering, seccomp BPF, macOS seatbelt profiles, violation monitoring, dynamic config update.

**What it doesn't provide that geist.sh adds**: Behavioral contracts, intent-action correlation, per-intent assembly, steering, eBPF observation.

**Source**: [github.com/anthropic-experimental/sandbox-runtime](https://github.com/anthropic-experimental/sandbox-runtime), our fork at github.com/yzavyas/sandbox-runtime

---

### Claude Code

**What it is**: Anthropic's CLI agent. Node.js binary. Spawns as `claude` or `claude -p` (headless). Uses MCP for tool access, has Agent Skills for skill loading, supports hooks for event-driven automation.

**In geist.sh context**: Claude Code *is* the agent. geist.sh is the shell it runs in. The ghost in the shell. Per-intent assembly = configured Claude Code session with `--skills` flags, MCP server configuration, and srt-enforced boundaries.

**Key integration points**:
- `claude -p` for headless execution with a prompt
- `--skills` for loading Agent Skills (where slick: contracts live)
- Hooks (PreToolUse, PostToolUse, etc.) for runtime observation
- MCP server configuration for tool access control

---

## The Stack (how it fits together)

```
┌─────────────────────────────────────────────────┐
│  Intent                                          │
│  "Deploy and monitor this webapp"               │
└──────────────────┬──────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────┐
│  geist.sh (Rust orchestrator)                    │
│  - Query cix registry for components             │
│  - Compile slick: contracts → SandboxRuntimeConfig│
│  - Configure Claude Code session                 │
│  - Attach eBPF observation (Linux)               │
│  - Monitor + steer                               │
└──────────────────┬──────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────┐
│  sandbox-runtime (srt)                           │
│  - Filesystem: seatbelt (macOS) / bwrap (Linux)  │
│  - Network: HTTP + SOCKS5 proxy                  │
│  - Syscalls: seccomp BPF (Linux)                 │
│  - Violations: SandboxViolationStore             │
└──────────────────┬──────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────┐
│  Claude Code (Node.js)                           │
│  - MCP client → tools                            │
│  - Agent Skills → behavioral contracts           │
│  - Hooks → runtime events                        │
└──────────────────┬──────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────┐
│  Kernel                                          │
│  - eBPF (observation + enforcement, Linux)        │
│  - seccomp BPF (static syscall deny)             │
│  - seatbelt (macOS sandbox profile)              │
│  - namespaces (Linux, via bwrap)                 │
└─────────────────────────────────────────────────┘
```

---

## Confidence Summary

| Component | Maturity | Our Confidence | Notes |
|-----------|----------|---------------|-------|
| seccomp BPF | Production (2012+) | **Verified** | Used by Chrome, Docker, Android, srt |
| eBPF | Production (2014+) | **Verified** | Meta, Cloudflare, Google, Netflix |
| Aya-rs | Production | **Verified** | Deepfence, Red Hat, Exein |
| Tetragon | Production (CNCF) | **Verified** | v1.6.0, Isovalent/Cisco |
| bubblewrap | Production | **Verified** | Flatpak, srt |
| macOS seatbelt | Shipping but deprecated | **Verified** | srt uses it; Apple may remove |
| MCP | De facto standard | **Verified** | 97M SDK downloads, Linux Foundation |
| A2A | Emerging standard | **Verified** | v0.3, 150+ orgs, Linux Foundation |
| wRPC | Niche | **Moderate** | Bytecode Alliance, wasmCloud only |
| PyO3 | Production | **Verified** | 13k+ stars, 0.28.x |
| srt | Shipping | **Verified** | Used by Claude Code in production |
| eBPFGuard | Reference impl | **Verified** | Deepfence, Aya-based |
| Contract→eBPF pipeline | Novel | **Speculative** | No production implementation exists |
| Intent-action correlation | Novel | **Speculative** | Research problem, not engineering |
