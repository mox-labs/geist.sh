# geist.sh

Desktop AI collaborator for knowledge workers. A packaged app where users set up their collaborator(s), backed by a governed runtime that makes composition safe.

**Domain**: gestalt.mox.nexus
**Stack**: Rust (edge runtime), Python (agent / Claude Agent SDK), SvelteKit (experience), Tauri (desktop shell)

## What geist.sh Is

A desktop application (Tauri) for knowledge workers — not just developers. Users set up their collaborator(s) with pre-installed skills, and the agent operates within a governed runtime (geist-edge). Think OpenClaw but for knowledge workers, with capability-led connectivity as the runtime model.

- **geist-edge** = the composable data plane runtime. Implements the Extension Protocol Adapter pattern — portable processing capabilities across runtimes (axum, pingora, ext_proc). Same building blocks at edge and per-service.
- **geist-shell** = Tauri desktop container. SvelteKit frontend in webview.
- **geist-policy** = Policy Decision Point. Deny-first evaluation, already built (48 tests).

All agent traffic flows through geist-edge. The edge governs, observes, and routes.

## Architecture

geist-edge follows the **capability-led connectivity** model: portable processing logic via the ext_proc unified contract (`ProcessingRequest`/`ProcessingResponse` from Envoy protos, available via `envoy-grpc-ext-proc` crate and re-exported by `rumi-http`), with adapters for different runtimes.

```
Tauri (geist-shell)
  └─ geist-edge (axum adapter)
      ├─ ProtocolServer: inbound → processors → agent
      ├─ ProtocolClient: agent → processors → external
      ├─ Processors: auth, policy, telemetry, routing
      └─ Metadata: type-safe inter-processor communication
          └─ Agent runtime (Claude SDK) — all calls through edge
```

**ACES** (Adaptable, Composable, Extensible Software) is the architectural quality:
- **Adaptable**: swap adapter (axum → pingora → ext_proc) without touching processors
- **Composable**: processors compose through pipeline + metadata, not direct coupling
- **Extensible**: add processor = add capability, no core or adapter changes

See `scratch/geist-edge-context-2026-02-22.md` for full architecture with blueprint source citations.

## Implementation Plan

| Phase | What | Status |
|-------|------|--------|
| **P1** | geist-policy PDP — workspace, naming, PolicyError, #[non_exhaustive], Send+Sync | **Done** |
| **P2** | geist-edge core — Processor trait, compositors (Sequence/Router), MetadataMap (ProcessingRequest/Response come from ext_proc protos, not hand-built) | Next |
| **P3** | Adapters — axum adapter (first), pingora adapter | |
| **P4** | Telemetry — OTel processor baked into pipeline | |
| **P5** | geist-shell — Tauri desktop, SvelteKit frontend | |
| **P6+** | Agent runtime, skills, AgentPolicy CRDs, Escalate variant, control plane | Later |

**Note:** Claude Code hook enforcement (originally P2) moved to x.uma as the rumi CLI. See `x.uma/scratch/rumi-cli-claude-hooks-2026-02-22.md`.

## Crate Layout

| Crate | Path | Role |
|-------|------|------|
| `geist-policy` | `geist/policy` | PDP domain core (48 tests) |
| `geist-edge` | `geist/edge` | Composable data plane runtime (stub) |
| `geist` | `geist/bin` | Binary (stub) |

## Key Documents

1. `.claude/docs/deployment-models.md` — **Deployment models & enforcement architecture.** Five enforcement layers (axum, hook, tauri, sandbox-runtime, eBPF), three deployment models, Agent SDK tool execution research, sandbox-runtime integration.
2. `scratch/geist-edge-context-2026-02-22.md` — geist-edge architecture. Full capability-led connectivity model with blueprint source citations.
2. `scratch/handoff-2026-02-21.md` — Session handoff covering P1 completion, naming resolution, guild outputs.
3. `scratch/architecture-session-2026-02-20.md` — Gateway API extension, deployment modes, ECDS.
4. `scratch/guild-deliberation-2026-02-19.md` — Guild record (10 members, 22 validation criteria).
5. `scratch/act-synthesis-2026-02-19.md` — ACT research synthesis (90+ sources).

### Blueprint Sources (External)

Prior art from previous work — concepts apply, documents need sanitization before any public use.

**geist.sh's twist:** Previous work was HTTP/gRPC focused and didn't reach Capability Edge. geist.sh applies these ideas with protocol-agnostic capability management (HTTP, gRPC, MCP, agent protocols) via protocol adapters.

**Research & synthesis** (`blueprints/scratch/`):

| Document | Concepts |
|----------|----------|
| `edge-api-era-understanding.md` | 4D framework, 29 principles, capability abstraction, ECDS |
| `proxyless_api_management_strategy.md` | API Mesh, proxyless viability, golden path |
| `composable-federated-data-plane-eval.md` | Three-layer architecture, latency analysis, migration economics |
| `composable_data_plane_context.md` | ACES framework, composable data plane |
| `data-plane-metadata-architecture.md` | Type-safe hierarchical metadata |
| `research-capability-led-connectivity.md` | API-led → capability-led evolution, policy-processor separation |
| `research-capability-led-connectivity-ext-proc.md` | ext_proc protocol lifecycle, unified processing model, Extension Protocol Adapter |

**Case studies** (`blueprints/sources/case-studies/`):

| Document | Concepts |
|----------|----------|
| `capability-connectivity-model/mox-apiserver-proxyless.md` | APIServer/APIClient pattern, logical capability IDs, xDS discovery, transparent decomposition |
| `anatomy-of-a-data-plane/edge-api-platform-era.md` | Enterprise Edge, Extension Protocol Adapter (59 plugins, 6-8 weeks), perimeter vs protocol layers |
| `anatomy-of-a-data-plane/unified-edge-compute-cluster.md` | Capability Edge, Enterprise Edge, two-boundary architecture, embedded vs sidecar |
| `engineering-for-sustainable-excellence/aces-unified-processing-model.md` | Unified Processing Model, ProcessingAdaptor, deployment topologies |
| `engineering-for-sustainable-excellence/aces-framework.md` | ACES framework, boundary-abstraction-implementation pattern |

## Proto API Naming Convention

Follows xDS type URL format: `{org}.{product}.{domain}.{version}.{Type}`

```
apis/proto/mox/geist/
├── agent/v1/       ← mox.geist.agent.v1 (agent tool governance)
└── edge/v1/        ← mox.geist.edge.v1 (edge config)
```

## Settled Decisions (Not Open for Debate)

- **Deny-first evaluation** — consensus across Cedar, SCT, OWASP
- **`&self` PolicyEvaluator** — enables Arc sharing, hyper school pattern
- **Hexagonal architecture** — domain crate has zero HTTP types
- **AgentOp as domain context** — 6 fields: agent_id, tool_name, resource, operation, session_id, metadata
- **Tower in HTTP adapter only** — never in domain core
- **xDS namespace: `mox.geist.{domain}.v1`** — follows Envoy convention
- **"Agentic Control Theory" is research framing only** — not in code or wire format
- **AgentPolicy as Gateway API extension** — GEP-713 pattern
- **ECDS for agent policy distribution** — dynamic updates without edge restart
- **ACES as system quality** — not a specific pattern, achieved through implementation choices
- **Capability-led connectivity** — processors portable across runtimes via unified contract
- **Policy-processor separation** — policies (what) decoupled from processors (how)
- **Open source runtime, service-based business** — gestalt.mox.nexus is the managed platform
- **ProcessingRequest/Response from ext_proc protos** — not hand-rolled. Via `envoy-grpc-ext-proc`, re-exported by `rumi-http`. `HttpMessage` (indexed view) from `rumi-http`.

## Git Workflow

Inherits mox conventions (conventional commits, fetch+rebase). Project-specific:

- **Scopes**: `policy`, `edge`, `shell`, `experience`, `docs`, `ci`
- **Rebase merge for stacked PRs** (squash merge breaks stacked PR chains)
- Cloudflare Pages auto-deploy on push to main (when `docs/experience/` changes)

## Skills

- `.claude/skills/geist-rust-mastery/` — Rust architectural judgment (13 codebases)
- `.claude/skills/geist-gateway-patterns/` — Gateway API, xDS, proxyless gRPC, control plane patterns

## Quick Verification

```bash
cargo test --workspace
```
