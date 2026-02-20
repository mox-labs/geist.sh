# sandbox-runtime (srt) Analysis

> **Date**: February 13, 2026
> **Source**: `@anthropic-ai/sandbox-runtime` v0.0.35, Apache-2.0
> **Upstream**: `github.com/anthropic-experimental/sandbox-runtime`
> **Fork**: `github.com/yzavyas/sandbox-runtime` (1 commit ahead: CLI fix)
> **Local paths**: `~/oss/sandbox-runtime` (upstream), `~/Projects/sandbox-runtime` (fork)

---

## What srt Is

Anthropic's OS-level sandboxing library for Claude Code. A TypeScript library + CLI (`srt`) that wraps arbitrary shell commands with filesystem, network, and syscall restrictions. Runs **outside** the sandbox on the host machine, configuring the OS enforcement mechanisms.

It is not a container. It is not a VM. It uses the kernel's own isolation primitives directly.

---

## How It Works (by platform)

### macOS: seatbelt profiles via `sandbox-exec`

Generates a Scheme-based sandbox profile at runtime and passes it to `sandbox-exec`. The profile is a deny-default policy with explicit allow rules. Uses `log stream` to monitor violations in real time.

Key primitives:
- `(deny default)` — deny everything not explicitly allowed
- `(allow file-read* ...)` / `(deny file-read* ...)` — filesystem read rules
- `(allow file-write* ...)` / `(deny file-write* ...)` — filesystem write rules (allow-only pattern)
- `(allow network-outbound ...)` — network restricted to proxy ports only
- `(deny file-write-unlink ...)` — blocks mv/rename to prevent bypass

The profile is *generated per command* with a unique log tag (`CMD64_<base64>_END_<session>`) for correlating violations back to the originating command.

### Linux: bubblewrap + seccomp BPF

Uses `bwrap` (bubblewrap) for filesystem namespace isolation and pre-compiled seccomp BPF filters for syscall restriction. Network filtering uses a bridge namespace that routes through the proxy.

- bubblewrap: mount namespace isolation (bind mounts for allowed paths)
- seccomp BPF: static bytecode filters (pre-compiled for x64/arm64) that block specific syscalls (e.g., unix socket creation)
- Network bridge: socat forwarding from sandbox namespace to host proxy

### Network (both platforms)

Starts HTTP and SOCKS5 proxy servers on localhost. All network traffic from the sandboxed process routes through these proxies. The proxy checks each request against allow/deny domain lists.

Supports:
- Wildcard domain patterns (`*.example.com`)
- MITM proxy forwarding (route specific domains through an upstream proxy via unix socket)
- External proxy ports (don't start local proxy, use an existing one)
- Dynamic user approval via `SandboxAskCallback` for unknown domains

---

## The API Surface

```typescript
// Initialize with config
SandboxManager.initialize(config: SandboxRuntimeConfig, askCallback?, enableLogMonitor?)

// Wrap a command string with sandbox
SandboxManager.wrapWithSandbox(command: string, shell?, customConfig?, abortSignal?): Promise<string>

// Update config at runtime (enables mid-flight policy changes)
SandboxManager.updateConfig(newConfig: SandboxRuntimeConfig): void

// Observe violations
SandboxManager.getSandboxViolationStore().subscribe(callback)

// Annotate stderr with violation context
SandboxManager.annotateStderrWithSandboxFailures(command, stderr): string

// Cleanup
SandboxManager.reset(): Promise<void>
```

### SandboxRuntimeConfig

```typescript
{
  network: {
    allowedDomains: string[]       // ["github.com", "*.npmjs.org"]
    deniedDomains: string[]        // checked first, before allowedDomains
    allowUnixSockets?: string[]    // macOS only: specific socket paths
    allowAllUnixSockets?: boolean  // disable socket blocking entirely
    allowLocalBinding?: boolean    // allow localhost port binding
    httpProxyPort?: number         // use external proxy instead of local
    socksProxyPort?: number        // use external SOCKS proxy
    mitmProxy?: {                  // route domains through upstream MITM
      socketPath: string
      domains: string[]
    }
  }
  filesystem: {
    denyRead: string[]             // paths denied for reading
    allowWrite: string[]           // paths allowed for writing (allow-only)
    denyWrite: string[]            // deny within allowed paths (precedence)
    allowGitConfig?: boolean       // allow .git/config writes (default: false)
  }
  ignoreViolations?: Record<string, string[]>  // command pattern → paths to ignore
  enableWeakerNestedSandbox?: boolean           // for Docker environments
  ripgrep?: { command: string; args?: string[] }
  mandatoryDenySearchDepth?: number             // Linux: depth for dangerous file scan
  allowPty?: boolean                            // macOS: allow pseudo-terminal
  seccomp?: { bpfPath?: string; applyPath?: string }  // Linux: custom seccomp paths
}
```

### Mandatory Protections (always enforced)

These paths are blocked for writing regardless of config:
- `.bashrc`, `.bash_profile`, `.zshrc`, `.profile` (shell configs)
- `.ssh/` (SSH keys and config)
- `.git/hooks/` (git hooks — code execution vector)
- `.env` files
- `.npmrc`, `.yarnrc` (package manager configs)

---

## BPF Lineage (relevant context)

```
BPF (1992, network packet filtering)
  → seccomp-BPF (2012, static syscall filtering)
    → eBPF (2014+, dynamic kernel programmability)
```

- **seccomp BPF** (what srt uses): Classic BPF bytecode. Static filters loaded once per process. Cannot be modified or removed after loading. Binary allow/deny per syscall. Pre-compiled `.bpf` files shipped in `vendor/seccomp/`.
- **eBPF** (what geist.sh would add): Extended BPF. Dynamic programs attached to kernel hooks. Maintain state in maps. Can observe without blocking. Loaded/unloaded at runtime. This is what Aya-rs provides.

seccomp BPF is a locked door. eBPF is a security camera that can also lock doors.

---

## What srt Does vs. What geist.sh Needs

### Already solved by srt

| Need | srt Implementation | Confidence |
|------|-------------------|------------|
| Filesystem isolation | macOS seatbelt + Linux bubblewrap | **Shipping** |
| Network domain filtering | HTTP + SOCKS5 proxy with allow/deny | **Shipping** |
| Syscall restriction | seccomp BPF (Linux), seatbelt (macOS) | **Shipping** |
| Violation monitoring | `SandboxViolationStore` with pub/sub | **Shipping** |
| Dangerous file protection | Mandatory deny for .ssh, .bashrc, .git/hooks, etc. | **Shipping** |
| Dynamic policy update | `updateConfig()` at runtime | **Shipping** |
| Per-command violation correlation | Base64-encoded command in log tags | **Shipping** |
| CLI wrapping | `srt wrap -- <command>` | **Shipping** |

### Not addressed by srt (geist.sh value-add)

| Need | Why srt Doesn't Do It | geist.sh Approach |
|------|----------------------|-------------------|
| **Behavioral contracts** | srt has raw allow/deny lists. No concept of agent intent or behavioral bounds. | slick: contracts in Agent Skills frontmatter → compiled to `SandboxRuntimeConfig` |
| **Intent-action observation** | srt monitors *denied* actions (violations). Doesn't observe *allowed* actions. | eBPF tracing (Linux) or structured tool-call logging to see what the agent *chose* to do within its allowed boundary |
| **Per-intent assembly** | srt configures one sandbox per session. | geist.sh queries registry per intent, merges component contracts into a single srt config, spawns configured `claude -p` |
| **Mid-flight steering** | srt can `updateConfig()` but has no intelligence about *when* to update. | Observe agent behavior → compare against contract expectations → throttle/redirect/halt |
| **Cross-component policy composition** | srt takes a flat config. No concept of composing policies from multiple sources. | Contract compiler merges slick: policies from N components into one `SandboxRuntimeConfig`, resolving conflicts |
| **eBPF observation (Linux)** | srt uses seccomp BPF for static blocking only. | Aya-rs eBPF programs for dynamic syscall/network tracing, maintaining state in maps for correlation |

---

## Architecture Implication

geist.sh sits **above** srt, not below it.

```
Intent arrives
  → geist.sh queries cix registry for components
  → each component has slick: behavioral contracts
  → contract compiler merges contracts into SandboxRuntimeConfig
  → geist.sh calls SandboxManager.initialize(config)
  → geist.sh calls SandboxManager.wrapWithSandbox("claude -p --skills ...")
  → geist.sh subscribes to violation store
  → (Linux) geist.sh attaches eBPF probes for deeper observation
  → geist.sh monitors, steers, and dismantles after task
```

srt is the enforcement floor. geist.sh is the contract-to-policy compiler and intent-aware observer.

The Rust runtime doesn't replace srt's sandboxing — it orchestrates it. The tracer bullet is a Rust process that:

1. Takes an intent + component manifest
2. Derives a `SandboxRuntimeConfig` from slick: contracts
3. Spawns Claude Code inside srt's sandbox
4. Observes via violation store + optionally eBPF
5. Steers via `updateConfig()` when contract boundaries are approached

---

## ADR-001 Update: PyO3 Reconsidered

Previous decision: Rust + PyO3 + eBPF. The PyO3 rationale was embedding Python for agent workloads.

With the reframe (Claude Code = the agent, not a Python script), PyO3 may not be needed for MVP. Claude Code is a Node.js binary. The Rust process doesn't need to embed Python — it needs to:
- Orchestrate srt (TypeScript library, but could call the CLI or use FFI)
- Compile contracts to config (pure Rust)
- Attach eBPF probes (Aya-rs, Rust-native)
- Manage process lifecycle (spawn/monitor/kill Claude Code)

PyO3 remains valuable for Phase 2+ when custom agent workloads or Python-based components need embedded execution. But the MVP path is simpler: Rust orchestrator → srt CLI/library → Claude Code.

---

## Open Questions

1. **Rust ↔ srt integration**: Call srt CLI from Rust? Use napi-rs to call the TypeScript library? Or reimplement the config → sandbox-exec/bwrap wrapping in Rust directly?
2. **macOS development**: srt works on macOS (seatbelt profiles). The eBPF layer is Linux-only. For development, the trait-based PolicyEngine fallback (ADR-003) maps cleanly: macOS uses srt directly, Linux adds eBPF on top.
3. **srt as dependency vs. fork**: Build on top of upstream srt? Or fork and extend? The fork is minimal (1 commit). Building on top preserves upstream improvements.
4. **Violation store limitations**: The store is in-memory, capped at 100 events, no persistence. For intent-action correlation, we may need a richer observation surface.
5. **`updateConfig()` latency**: How fast does `updateConfig()` take effect? For steering, we need sub-second policy changes. The proxy checks config on each request, so network policy changes are immediate. Filesystem policy changes require a new `wrapWithSandbox()` call (new process).

---

## Key Code Paths (for future reference)

| What | File |
|------|------|
| Main API | `src/sandbox/sandbox-manager.ts` |
| Config types + Zod schemas | `src/sandbox/sandbox-config.ts` |
| Restriction type interfaces | `src/sandbox/sandbox-schemas.ts` |
| macOS seatbelt profile generation | `src/sandbox/macos-sandbox-utils.ts` |
| macOS violation monitoring | `src/sandbox/macos-sandbox-utils.ts:startMacOSSandboxLogMonitor()` |
| Linux bubblewrap wrapping | `src/sandbox/linux-sandbox-utils.ts` |
| Violation store (pub/sub) | `src/sandbox/sandbox-violation-store.ts` |
| HTTP proxy (domain filtering) | `src/sandbox/http-proxy.ts` |
| SOCKS proxy (domain filtering) | `src/sandbox/socks-proxy.ts` |
| CLI entry point | `src/cli.ts` |
| Pre-compiled seccomp filters | `vendor/seccomp/` |
