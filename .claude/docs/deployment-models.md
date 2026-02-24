# geist-edge: Deployment Models & Enforcement Architecture

## Overview

geist-edge is a composable data plane runtime. The core insight: **processors are portable, adapters are swappable**. The same access control, rate limiting, and auth processors run identically regardless of whether they're enforcing in a Tauri desktop app, a Linux server, or an in-process Agent SDK hook.

This document covers:
1. How agents actually execute tools (verified against Claude Agent SDK)
2. The five enforcement layers and what they govern
3. Deployment models and how layers compose per platform
4. The sandbox-runtime integration (Anthropic's OS-level sandboxing)

---

## How Agent Tool Execution Actually Works

### Claude Agent SDK (Python)

The Python Agent SDK does NOT execute tools itself. It spawns the **Claude Code CLI** (a bundled Node.js binary) as a subprocess. The CLI runs the agentic loop and executes all tools locally.

```
┌──────────────────────────────────────────────────┐
│              Python Application                  │
│           (your agent, Claude SDK)               │
│                                                  │
│  ┌────────────────────────────────────────────┐  │
│  │        SubprocessCLITransport              │  │
│  │  - Spawns Claude Code CLI                  │  │
│  │  - JSON lines over stdin/stdout            │  │
│  │  - Control protocol handshake              │  │
│  │  - Hook dispatch (PreToolUse, etc.)        │  │
│  └───────────────────┬────────────────────────┘  │
└──────────────────────┼───────────────────────────┘
                       │ stdin/stdout (JSON lines)
┌──────────────────────┼───────────────────────────┐
│        Claude Code CLI (Node.js)                 │
│                      │                           │
│  ┌───────────────────┴────────────────────┐      │
│  │          Agentic Loop                  │      │
│  │  while (stop_reason == "tool_use"):    │      │
│  │    1. Check permissions                │      │
│  │    2. Fire PreToolUse hooks → SDK      │      │
│  │    3. Sandbox wraps command (if on)    │      │
│  │    4. Execute tool locally             │      │
│  │    5. Fire PostToolUse hooks → SDK     │      │
│  │    6. Send tool_result to API          │      │
│  └───────────────────┬────────────────────┘      │
│                      │                           │
│  ┌───────────────────┴────────────────────┐      │
│  │        Built-in Tool Executors         │      │
│  │  Bash    → spawns shell subprocess     │      │
│  │  Read    → local fs read               │      │
│  │  Write   → local fs write              │      │
│  │  Edit    → string replace + write      │      │
│  │  Glob    → file pattern matching       │      │
│  │  Grep    → ripgrep subprocess          │      │
│  │  WebFetch → outbound HTTP              │      │
│  └────────────────────────────────────────┘      │
│                      │                           │
│           Anthropic Claude API                   │
│         (HTTP, messages.create)                  │
└──────────────────────────────────────────────────┘
```

**Key facts:**
- Built-in tools (Bash, Read, Write, Edit, Glob, Grep) execute **locally** in the CLI process. No network.
- Only WebSearch/WebFetch make outbound HTTP calls.
- The Claude API never sees or executes tools — it only receives results.
- Custom `@tool` functions run **in-process** in the Python SDK via an MCP server bridge.

### Interception Points

The execution chain within the CLI:

```
API returns tool_use block
  → PreToolUse hooks fire (can block/allow/modify input)
  → Permission system evaluates
  → Sandbox wraps command (if sandbox enabled)
  → Tool executes locally
  → PostToolUse hooks fire (can provide feedback to Claude)
```

The SDK `PreToolUse` hook decision schema maps to the ext_proc processing model:
- `"allow"` → `PhaseResult::Continue`
- `"deny"` → `PhaseResult::Respond(ImmediateResponse { status: 403 })`
- `"allow"` + `updatedInput` → `PhaseResult::Mutate(...)` (input rewriting)

### MCP Tool Calls

For tools exposed as MCP servers, the agent makes HTTP calls through geist-edge:

```
Agent → geist-edge → Bash MCP server (localhost)
Agent → geist-edge → Filesystem MCP server (localhost)
```

The edge sees an HTTP POST with an MCP JSON-RPC body:
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "bash",
    "arguments": { "command": "rm -rf /" }
  }
}
```

Tool name, command, and file path are in the **request body** — not headers. Access control for MCP tool calls is a **body-processing** concern (ext_proc body phase).

Two enforcement levels:
1. **Headers-only** (routing) — can this agent reach the Bash capability at all?
2. **Body inspection** — can this agent run *this specific command*?

---

## The Five Enforcement Layers

The EPA pattern: same processor pipeline, different adapters for different enforcement contexts. Five layers, composing from application level down to OS level.

```
┌───────────────────────────────────────────────────────────┐
│                    Policy Layer                           │
│  (AccessControlPolicy, RateLimitPolicy, AuthPolicy)      │
│  User-defined configs — the WHAT                         │
└─────────────────────────┬─────────────────────────────────┘
                          │ compiled to
┌─────────────────────────┴─────────────────────────────────┐
│                  Processor Pipeline                       │
│  (AccessControlProcessor, RateLimiterProcessor, etc.)     │
│  Enforcement implementations — the HOW                   │
└────┬────────┬──────────┬──────────┬──────────┬────────────┘
     │        │          │          │          │
┌────┴───┐┌───┴───┐┌────┴────┐┌────┴────┐┌────┴─────────┐
│  axum  ││ hook  ││  tauri  ││sandbox- ││    eBPF      │
│adapter ││adapter││ adapter ││runtime  ││   adapter    │
│        ││       ││         ││ adapter ││              │
└────┬───┘└───┬───┘└────┬────┘└────┬────┘└──────┬───────┘
     │        │         │          │             │
  L7 HTTP   in-proc   app-level  OS-level     kernel
  MCP/API   tool call  IPC/scope sandbox     syscall
```

### Layer 1: axum Adapter (HTTP/L7)

**Governs:** All HTTP traffic — MCP server calls, external API calls, outbound requests.

**How:** Reverse proxy (axum). Intercepts HTTP requests, translates to `ProcessingRequest`, runs pipeline, translates `PhaseResult` back to HTTP response or forwards upstream.

**Level:** L7 — full HTTP semantics. Headers, path, method, body (buffered).

**Platform:** All (Rust, cross-platform).

**When:** Agent makes any HTTP call through the edge — to MCP servers, external APIs, other services.

### Layer 2: Hook Adapter (Agent SDK In-Process)

**Governs:** Built-in agent tool execution (Bash, Read, Write, Edit, Grep, Glob) — tools that execute locally in the Claude Code CLI process.

**How:** Registers as a `PreToolUse` hook callback in the Agent SDK. When the CLI is about to execute a tool, the hook fires. The adapter translates hook input (`tool_name`, `tool_input`) into `ProcessingRequest`, runs pipeline, translates `PhaseResult` back to hook response (`allow`/`deny`/`modify`).

**Level:** Tool — tool name, arguments, file paths, commands. Full input visibility.

**Platform:** All (Python in-process callback, or shell command on stdin/stdout).

**When:** Agent's built-in tools (not MCP, not HTTP). These never hit the network — the hook is the only software interception point.

### Layer 3: Tauri Adapter (Desktop Application)

**Governs:** Application-level operations — shell command execution, filesystem access, IPC between webview and backend.

**How:** Integrates with Tauri v2's security model:
- **Capabilities/ACL** — per-window permissions, deny-by-default
- **Shell plugin scopes** — command whitelist + argument regex validation
- **FS plugin scopes** — allow/deny path globs, path traversal prevention
- **IPC permissions** — which webview can call which Rust command

Maps geist-edge policies to Tauri scope configurations. Enforcement in Tauri's Rust backend — webview cannot bypass it.

**Level:** Application — scoped command execution, scoped filesystem access, IPC control.

**Platform:** macOS, Windows, Linux (desktop). Cross-platform via Tauri.

**When:** Desktop deployment. The Tauri shell IS the governed container.

### Layer 4: Sandbox Runtime Adapter (OS-Level Sandboxing)

**Governs:** OS-level filesystem and network isolation — what the agent process can read, write, and connect to.

**How:** Integrates with Anthropic's `sandbox-runtime` (`@anthropic-ai/sandbox-runtime`, source at `~/oss/sandbox-runtime/`). Uses platform-specific OS primitives:

| Platform | Filesystem | Network | Process |
|----------|-----------|---------|---------|
| **macOS** | `sandbox-exec` + dynamically generated Seatbelt profiles | HTTP + SOCKS proxies on localhost | Single process wrapping |
| **Linux** | `bubblewrap` bind-mounts + ripgrep dangerous file scanning | HTTP + SOCKS proxies via Unix sockets | Full namespace isolation (PID, network, IPC) |

**Isolation model (dual — both required):**

Filesystem:
- **Reads** — deny-only (permissive by default, block specific paths like `~/.ssh`, `.env`)
- **Writes** — allow-only (restrictive by default, explicitly permit paths)
- **Mandatory denies** — shell configs, git hooks, IDE dirs, Claude config (can't override)

Network:
- **Allow-only** — explicitly allow domains (`github.com`, `*.npmjs.org`)
- **Deny takes precedence** — `deniedDomains` checked before `allowedDomains`
- **All traffic through proxies** — HTTP proxy + SOCKS5 proxy enforce domain allowlist

**Level:** OS — kernel-enforced filesystem and network restrictions. Process cannot bypass.

**Platform:** macOS (Seatbelt) and Linux (bubblewrap + seccomp BPF). Not yet Windows.

**When:** Any deployment where OS-level isolation is needed. Already used by Claude Code. Can run standalone (`srt <command>`) or as a library.

**Key architecture detail:** sandbox-runtime wraps individual commands — it generates a sandboxed command string that the caller executes:
```
Input:  "npm install"
Output: "sandbox-exec -f /tmp/profile.sb npm install"  (macOS)
Output: "bwrap ... apply-seccomp ... npm install"       (Linux)
```

This means the adapter can wrap any tool execution with policy-derived sandbox configurations.

### Layer 5: eBPF Adapter (Linux Kernel — Future)

**Governs:** Syscall-level operations — file access (`open`, `read`, `write`), process execution (`execve`), network calls (`connect`, `sendto`).

**How:** eBPF programs attached to kernel tracepoints/LSM hooks.

**Level:** Kernel — catches everything, including processes that escape all other layers.

**Platform:** **Linux only.**

**When:** Maximum defense-in-depth on Linux. Goes beyond sandbox-runtime by intercepting at the syscall level rather than through process wrapping. Future investment — sandbox-runtime covers the immediate need.

**Note:** sandbox-runtime already provides strong OS-level enforcement on both macOS and Linux. eBPF is additive — it catches edge cases where a process escapes the bubblewrap sandbox or bypasses proxy environment variables.

---

## Layer Comparison

| Layer | Governs | Level | macOS | Linux | Windows |
|-------|---------|-------|-------|-------|---------|
| **axum** | HTTP/MCP traffic | L7 | yes | yes | yes |
| **hook** | Agent SDK built-in tools | Tool | yes | yes | yes |
| **Tauri** | Shell/FS/IPC scopes | Application | yes | yes | yes |
| **sandbox-runtime** | FS + network isolation | OS | yes (Seatbelt) | yes (bwrap) | no* |
| **eBPF** | Syscalls | Kernel | no | yes | no |

*Windows support for sandbox-runtime is not yet available.

---

## Deployment Models

Two deployment models. Same composable data plane, different contexts.

### Model 1: Desktop — Personal CIS (geist-shell)

A personal Collaborative Intelligence System. Users set up their collaborator(s) in the Tauri shell — agents with pre-installed skills that operate within the governed runtime. Intent-driven composition: the user expresses intent, the system composes capabilities to fulfill it.

Multiple collaborators possible — each governed by the same edge.

```
┌──────────────────────────────────────────────────────────┐
│            Tauri Shell (container + microgateway)         │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │              geist-edge (embedded)                 │  │
│  │                                                    │  │
│  │  ┌────────┐ ┌──────┐ ┌───────┐ ┌──────────────┐   │  │
│  │  │  axum  │ │ hook │ │ tauri │ │  sandbox-    │   │  │
│  │  │adapter │ │adapt.│ │adapt. │ │  runtime     │   │  │
│  │  └───┬────┘ └──┬───┘ └──┬────┘ └──────┬───────┘   │  │
│  │      │         │        │             │            │  │
│  │  ┌───┴─────────┴────────┴─────────────┴─────────┐  │  │
│  │  │            Processor Pipeline                │  │  │
│  │  │   access control, rate limit, auth           │  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌───────────────────────────────────────────────────┐   │
│  │  Collaborator(s) (Claude Agent SDK apps)          │   │
│  │  ┌──────────────┐  ┌──────────────┐               │   │
│  │  │ Collaborator │  │ Collaborator │  ...          │   │
│  │  │ (skills, MCP)│  │ (skills, MCP)│               │   │
│  │  └──────┬───────┘  └──────┬───────┘               │   │
│  │         │                 │                        │   │
│  │         │  tool calls  → hook adapter              │   │
│  │         │  MCP calls   → axum adapter              │   │
│  │         │  shell/fs    → tauri adapter              │   │
│  │         │  OS sandbox  → sandbox-runtime            │   │
│  └───────────────────────────────────────────────────┘   │
│                                                          │
│  ┌───────────────────────────────────────────────────┐   │
│  │  MCP Servers (capability components)              │   │
│  │  bash, fs, browser, search, custom tools, ...     │   │
│  └───────────────────────────────────────────────────┘   │
│                                                          │
│  ┌───────────────────────────────────────────────────┐   │
│  │          SvelteKit Frontend (webview)             │   │
│  └───────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────┘
```

**Enforcement stack (macOS):**
1. Hook adapter — governs built-in tool execution (PreToolUse)
2. axum adapter — governs HTTP/MCP traffic
3. Tauri adapter — governs shell scopes, FS scopes, IPC
4. sandbox-runtime — OS-level FS + network isolation (Seatbelt)

**Enforcement stack (Linux desktop):**
- Same as macOS, but sandbox-runtime uses bubblewrap + seccomp
- Optional: eBPF adapter for kernel-level defense-in-depth

### Model 2: Cloud — Services + Operator Agent

Deploy agent apps, API services, and backend services to the cloud. geist-edge runs as the composable data plane (sidecar/microgateway). A Claude Agent SDK app acts as the operator — maintaining, monitoring, and operating the running service.

```
┌──────────────────────────────────────────────────────────┐
│               Cloud / Server Environment                 │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │       geist-edge (sidecar / microgateway)         │  │
│  │                                                    │  │
│  │  ┌────────┐ ┌──────┐ ┌──────────────┐             │  │
│  │  │  axum  │ │ hook │ │  sandbox-    │             │  │
│  │  │adapter │ │adapt.│ │  runtime     │             │  │
│  │  └───┬────┘ └──┬───┘ └──────┬───────┘             │  │
│  │      │         │            │                      │  │
│  │  ┌───┴─────────┴────────────┴───────────────────┐  │  │
│  │  │            Processor Pipeline                │  │  │
│  │  │  access control, rate limit, auth,           │  │  │
│  │  │  telemetry, routing                          │  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │            Deployed Services                       │  │
│  │  (agent apps, APIs, databases, workers, etc.)      │  │
│  │  All inbound/outbound traffic through edge         │  │
│  └────────────────────────┬───────────────────────────┘  │
│                           │                              │
│  ┌────────────────────────┴───────────────────────────┐  │
│  │        Operator Agent (Claude Agent SDK)           │  │
│  │                                                    │  │
│  │  - Monitors service health and metrics             │  │
│  │  - Handles incidents and alerts                    │  │
│  │  - Performs maintenance (deploys, migrations)      │  │
│  │  - Scales resources                                │  │
│  │  - Debugs production issues                        │  │
│  │  - All actions governed through edge               │  │
│  └────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘
```

**Enforcement stack:**
1. axum adapter — all HTTP traffic (service ↔ external, agent ↔ service)
2. hook adapter — governs operator agent's built-in tool calls (PreToolUse)
3. sandbox-runtime — wraps shell/CLI operations the agent performs on the host
4. Optional: eBPF adapter on Linux for kernel-level defense-in-depth

**Key difference from Desktop:** No Tauri adapter (no shell). The operator agent runs alongside the service, governed through edge. Policies define what the agent can do: "can restart service, can read logs, cannot drop database, cannot modify credentials."

---

## Sandbox Runtime Details

Source: `~/oss/sandbox-runtime/` (Anthropic, Apache 2.0)

### Configuration

```typescript
{
  network: {
    allowedDomains: ["github.com", "*.npmjs.org"],
    deniedDomains: ["*.internal.corp"],  // checked first
    allowLocalBinding: false,
    allowAllUnixSockets: false,
  },
  filesystem: {
    denyRead: ["~/.ssh", "~/.aws", ".env"],       // deny-only
    allowWrite: [".", "/tmp"],                      // allow-only
    denyWrite: [".env", ".git/hooks"],             // exceptions
  }
}
```

### Mandatory Deny Paths (Auto-Protected)

These are always blocked regardless of configuration:
- Shell configs: `.bashrc`, `.zshrc`, `.profile`
- Git hooks: `.git/hooks/**`
- IDE dirs: `.vscode/`, `.idea/`
- Claude config: `.claude/commands/`, `.claude/agents/`
- MCP config: `.mcp.json`

### Integration Pattern

sandbox-runtime wraps commands — it generates a sandboxed command string:

```
Input:  "npm install"

macOS:  sandbox-exec -f /tmp/seatbelt-profile.sb npm install
Linux:  bwrap --ro-bind / / --bind ./src ./src ... apply-seccomp npm install
```

For geist-edge, the adapter translates processor pipeline decisions into sandbox configurations:
- AccessControlProcessor denies write to `/etc/` → sandbox `denyWrite: ["/etc/"]`
- Network policy allows only `github.com` → sandbox `allowedDomains: ["github.com"]`

### Library API

Can be used standalone (no Claude Code required):

```typescript
import { SandboxManager } from '@anthropic-ai/sandbox-runtime'

await SandboxManager.initialize(config)
const wrappedCmd = await SandboxManager.wrapWithSandbox('npm install')
// Execute wrappedCmd as a child process — sandbox enforced by OS
```

---

## Naming Corrections

Based on architectural review (2026-02-23):

| Correction | Status | Notes |
|------------|--------|-------|
| `DenyAllowPolicy` → `AccessControlPolicy` | Done | Name describes what, not how |
| `PolicyEvaluator` removed — each processor owns its policy | Done | No generic PDP; processors compile policy → rumi matchers directly |
| `x-geist-*` synthetic headers → real HTTP + MCP body | Pending | Current processor uses provisional header convention; will evolve with adapter layer |
| sandbox-runtime as primary OS-level story (not eBPF) | Done (docs) | eBPF is Linux-only additive layer |

---

## References

### Agent SDK & Claude Code
- [Claude Agent SDK — Python reference](https://platform.claude.com/docs/en/agent-sdk/python)
- [Claude Agent SDK — Hooks](https://platform.claude.com/docs/en/agent-sdk/hooks)
- [How Claude Code Works](https://code.claude.com/docs/en/how-claude-code-works)
- [Claude Code Hooks](https://code.claude.com/docs/en/hooks)
- [Claude Code Sandboxing](https://code.claude.com/docs/en/sandboxing)
- [Sandbox Runtime (GitHub)](https://github.com/anthropic-experimental/sandbox-runtime)

### Tauri v2
- [Tauri v2 Permissions](https://v2.tauri.app/security/permissions/)
- [Tauri v2 Capabilities](https://v2.tauri.app/security/capabilities/)
- [Tauri v2 Command Scopes](https://v2.tauri.app/security/scope/)
- [Tauri v2 Shell Plugin](https://v2.tauri.app/plugin/shell/)
- [Tauri v2 FS Plugin](https://v2.tauri.app/plugin/file-system/)

### Envoy Reference Patterns
- Envoy RBAC filter: `~/oss/envoy/source/extensions/filters/http/rbac/` — policy/engine/filter separation
- Envoy OAuth2 filter: `~/oss/envoy/source/extensions/filters/http/oauth2/` — auth flow state machine
- Envoy ext_proc: `~/oss/envoy/api/envoy/service/ext_proc/v3/` — ProcessingRequest/Response contract
- RBAC proto: `~/oss/envoy/api/envoy/config/rbac/v3/rbac.proto` — Principal + Permission model

### Local Source
- Sandbox runtime: `~/oss/sandbox-runtime/`
- Envoy: `~/oss/envoy/`
- xDS: `~/oss/xds/`
