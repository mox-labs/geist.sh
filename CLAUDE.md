# geist.sh

Desktop AI collaborator for knowledge workers. A packaged app where users set up their collaborator(s), backed by a governed runtime that makes composition safe.

**Domain**: gestalt.mox.nexus
**Stack**: Rust (edge runtime), Python (agent / Claude Agent SDK), SvelteKit (experience), Tauri (desktop shell)

## What geist.sh Is

A desktop application (Tauri) for knowledge workers — not just developers. Users set up their collaborator(s) with pre-installed skills, and the agent operates within a governed runtime (geist-edge). Capability-led connectivity as the runtime model.

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

See `scratch/capability-led-connectivity.md` for the full processing model.

## Implementation Plan

| Phase | What | Status |
|-------|------|--------|
| **P1** | geist-edge core — Processor trait, Sequence compositor, PhaseResult, ProcessingMode | **Done** (67 tests) |
| **P1.5** | Typed extension registry + access control processor extension | **Done** (40 tests, 3 PRs: #8 registry, #9 access-control, #10 composition root) |
| **P2** | Processor trait refactor (`http::` types) + axum adapter + OTel + Docker | **Done** (48 tests) |
| **P3** | Composer (intent → capability selection from cix catalog) | |
| **P4** | CLI demo (compose → configure → edge → agent session) | |
| **P5** | geist-shell — Tauri desktop, SvelteKit frontend | |
| **P6+** | Enterprise: Router compositor, OTel, pingora, xDS transport, control plane | Growth |

## Crate Layout

| Crate | Path | Role |
|-------|------|------|
| `geist-edge` | `geist/edge` | Composable data plane runtime — Processor trait, typed extension registry, compositors, built-in processors (incl. access control) |
| `geist-sh` | `geist/bin` | Binary — composition root, links extensions, starts adapter + agent runtime |

**External dependency**: [`slickit`](https://github.com/mox-labs/slick) (Apache-2.0) — generic typed extension registry (`TypedConfig`, `TypedRegistry<T, E>`). geist-edge uses it via type alias `ProcessorRegistry = TypedRegistry<Arc<dyn Processor>, ProcessorError>`.

## Key Documents

1. `scratch/typed-extension-registry.md` — **Typed extension registry pattern.** IntoProcessor trait, type URL factories, `collect_processor_extensions()`. Reference patterns from Envoy FactoryRegistry and rumi RegistryBuilder.
2. `scratch/deployment-models.md` — **Deployment models & enforcement architecture.** Five enforcement layers (axum, hook, tauri, sandbox-runtime, eBPF), deployment models, Agent SDK tool execution research.
3. `scratch/capability-led-connectivity.md` — **Capability-led connectivity model.** Processing pipeline, protocol mechanics, CapabilityServer/Client, boundary/encapsulation, Envoy mapping, decision framework.

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
- **Hexagonal architecture** — edge core uses `http::` types as protocol vocabulary (like serde), adapters are feature-gated
- **Typed extension registry** — processors register via type URL + factory (same pattern as rumi RegistryBuilder and Envoy FactoryRegistry). Each processor owns its policy/config type. No generic PolicyEvaluator.
- **Policy-processor separation** — each processor owns its config/policy type and compiles to rumi matchers at construction. No generic PolicyEvaluator, no AgentOp intermediary.
- **Tower in HTTP adapter only** — never in domain core
- **xDS namespace: `mox.geist.{domain}.v1`** — follows Envoy convention
- **"Agentic Control Theory" is research framing only** — not in code or wire format
- **ECDS for processor config distribution** — dynamic updates without edge restart (TypedExtensionConfig envelope)
- **ACES as system quality** — not a specific pattern, achieved through implementation choices
- **Capability-led connectivity** — processors portable across runtimes via unified contract
- **Open source runtime, service-based business** — gestalt.mox.nexus is the managed platform
- **`http::` types as processor input** — `&http::request::Parts` / `&http::response::Parts`. Own `HeaderMutations` and `ImmediateResponse` using `http::` vocabulary. ext_proc types only in ext_proc adapter (P6+).

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
# All tests (core + adapter)
cargo test --workspace

# Run locally
cargo run -p geist-sh
# curl http://localhost:3000/

# Docker (from mox/ parent directory)
docker compose -f geist.sh/docker-compose.yml up
# Jaeger UI: http://localhost:16686
```
