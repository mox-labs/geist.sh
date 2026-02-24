# geist.sh

Desktop AI collaborator for knowledge workers. A packaged app where users set up their collaborator(s), backed by a governed runtime that makes composition safe.

**Domain**: gestalt.mox.nexus
**Stack**: Rust (edge runtime), Python (agent / Claude Agent SDK), SvelteKit (experience), Tauri (desktop shell)

## What geist.sh Is

A desktop application (Tauri) for knowledge workers — not just developers. Users set up their collaborator(s) with pre-installed skills, and the agent operates within a governed runtime (geist-edge). Think OpenClaw but for knowledge workers, with capability-led connectivity as the runtime model.

- **geist-edge** = the composable data plane runtime. Implements the Extension Protocol Adapter pattern — portable processing capabilities across runtimes (axum, pingora, ext_proc). Same building blocks at edge and per-service. Includes typed extension registry for processor pluggability.
- **geist-shell** = Tauri desktop container. SvelteKit frontend in webview.

All agent traffic flows through geist-edge. The edge governs, observes, and routes.

## Architecture

geist-edge follows the **capability-led connectivity** model: portable processing logic via the ext_proc unified contract (`ProcessingRequest`/`ProcessingResponse` from Envoy protos, available via `envoy-grpc-ext-proc` crate and re-exported by `rumi-http`), with adapters for different runtimes.

```
Tauri (geist-shell)
  └─ geist-edge (axum adapter)
      ├─ ProtocolServer: inbound → processors → agent
      ├─ ProtocolClient: agent → processors → external
      ├─ Processors: registered via typed extension registry (type URL → factory)
      └─ Metadata: type-safe inter-processor communication
          └─ Agent runtime (Claude SDK) — all calls through edge
```

**Typed Extension Registry** (same pattern as rumi `RegistryBuilder` and Envoy `FactoryRegistry`):
- Processors register via type URL + factory (`IntoProcessor` trait with associated `Config` type)
- Pipeline config is a list of `TypedConfig { type_url, config }` entries
- Each processor owns its policy type — no generic PolicyEvaluator
- Registry is immutable after build (builder pattern → frozen registry)
- Extension crates self-register via `inventory::submit!` — zero code changes to core or binary

**ACES** (Adaptable, Composable, Extensible Software) is the architectural quality:
- **Adaptable**: swap adapter (axum → pingora → ext_proc) without touching processors
- **Composable**: processors compose through pipeline + metadata, not direct coupling
- **Extensible**: add processor = register extension + config entry, no core or adapter changes

See `scratch/geist-edge-context-2026-02-22.md` for full architecture with blueprint source citations.

## Implementation Plan

| Phase | What | Status |
|-------|------|--------|
| **P1** | geist-edge core — Processor trait, Sequence compositor, PhaseResult, ProcessingMode | **Done** (67 tests) |
| **P1.5** | Typed extension registry + access control processor extension | **Done** (40 tests, 3 PRs: #8 registry, #9 access-control, #10 composition root) |
| **P2** | axum adapter + governed proxy binary | |
| **P3** | Composer (intent → capability selection from cix catalog) | |
| **P4** | CLI demo (compose → configure → edge → agent session) | |
| **P5** | geist-shell — Tauri desktop, SvelteKit frontend | |
| **P6+** | Enterprise: Router compositor, OTel, pingora, xDS transport, control plane | Growth |

**Note:** Claude Code hook enforcement moved to x.uma as the rumi CLI.

## Crate Layout

| Crate | Path | Role |
|-------|------|------|
| `geist-edge` | `geist/edge` | Composable data plane runtime — Processor trait, typed extension registry, compositors, adapters |
| `geist` | `geist/bin` | Binary — composition root, wires extensions + adapter |
| `geist-access-control` | `geist/access-control` | Access control processor extension (P1.5) |

## Key Documents

1. `.claude/docs/typed-extension-registry.md` — **Typed extension registry pattern.** IntoProcessor trait, ProcessorRegistryBuilder, type URL factories. Reference patterns from Envoy FactoryRegistry and rumi RegistryBuilder.
2. `.claude/docs/deployment-models.md` — **Deployment models & enforcement architecture.** Five enforcement layers (axum, hook, tauri, sandbox-runtime, eBPF), deployment models, Agent SDK tool execution research.
3. `.claude/docs/capability-led-connectivity.md` — **Capability-led connectivity model.** Processing pipeline, protocol mechanics, CapabilityServer/Client, boundary/encapsulation, Envoy mapping, decision framework.
4. `scratch/geist-edge-context-2026-02-22.md` — geist-edge architecture. Full capability-led connectivity model with blueprint source citations.
5. `scratch/handoff-2026-02-21.md` — Session handoff covering P1 completion, naming resolution, guild outputs.
6. `scratch/architecture-session-2026-02-20.md` — Gateway API extension, deployment modes, ECDS.
7. `scratch/guild-deliberation-2026-02-19.md` — Guild record (10 members, 22 validation criteria).
8. `scratch/act-synthesis-2026-02-19.md` — ACT research synthesis (90+ sources).

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
- **`&self` Processor trait** — enables Arc sharing, hyper school pattern
- **Hexagonal architecture** — edge core has zero HTTP types without adapter features
- **Typed extension registry** — processors register via type URL + factory (same pattern as rumi RegistryBuilder and Envoy FactoryRegistry). Each processor owns its policy/config type. No generic PolicyEvaluator.
- **Policy-processor separation** — each processor owns its config/policy type and compiles to rumi matchers at construction. No generic PolicyEvaluator, no AgentOp intermediary.
- **Tower in HTTP adapter only** — never in domain core
- **xDS namespace: `mox.geist.{domain}.v1`** — follows Envoy convention
- **"Agentic Control Theory" is research framing only** — not in code or wire format
- **ECDS for processor config distribution** — dynamic updates without edge restart (TypedExtensionConfig envelope)
- **ACES as system quality** — not a specific pattern, achieved through implementation choices
- **Capability-led connectivity** — processors portable across runtimes via unified contract
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
