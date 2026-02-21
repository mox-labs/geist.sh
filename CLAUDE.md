# geist.sh

Self-composing AI assistant. ghost.shell — the agent that builds itself within a governed runtime.

**Domain**: gestalt.mox.nexus (gestalt = emergence from geist + shell)
**Stack**: Rust (shell runtime), Python (agent / Claude Agent SDK), SvelteKit (docs/experience), eBPF (kernel enforcement via Aya-rs)

## What geist.sh Is

geist.sh is two things:

1. **Self-composing AI assistant** — an autonomous agent (geist) running inside a governed runtime (shell). The agent composes its own capabilities through the ACES progression (cix → slick → treya → mox). The shell ensures composition happens within policy boundaries.

2. **Deployable runtime with on-app-instance SRE** — deploy your app with geist.sh and get an autonomous site reliability engineer on every instance. The geist observes, diagnoses, and heals your application — governed by policy so it can't go rogue.

The shell is the governed runtime. The geist is the operational intelligence. Policy enforcement is the governance layer that makes both safe — not the product itself.

## Architecture

The shell IS the runtime — the agent runs INSIDE it, all syscalls and network calls mediated.

```
L4: Agentic    (geist)  — the self-composing agent (Claude Agent SDK, Python)
L3: Application (yours)  — business logic, policy configuration (Python)
L2: Gateway    (shell)  — protocol routing, policy enforcement (Rust)
L1: Kernel     (shell)  — eBPF, sandbox enforcement (Rust, future)
```

- **geist** (L4) = the autonomous, self-composing agent
- **shell** (L2/L1) = the governed runtime it lives in
- Together = geist.sh — the assistant that composes itself safely

## Current State

Rust workspace at repo root. Three crates: `geist/policy` (PDP), `geist/edge` (edge adapter stub), `geist/bin` (binary stub). SvelteKit experience at `docs/experience/`, deployed to `gestalt.mox.nexus`. Guild deliberation (10 members) completed 2026-02-19.

## Key Documents (Read in This Order)

1. `scratch/handoff-2026-02-21.md` — **START HERE**. Self-contained session handoff: current state, P2 spec, what exists vs needs building, all settled decisions.
2. `scratch/architecture-session-2026-02-20.md` — Full architecture (Gateway API extension, deployment modes, capability abstraction, ECDS).
3. `scratch/guild-deliberation-2026-02-19.md` — Full guild record (10 members, 22 validation criteria, phased plan).
4. `scratch/act-synthesis-2026-02-19.md` — ACT research synthesis (90+ sources).

## Implementation Plan (Guild-Approved)

| Phase | What | Status |
|-------|------|--------|
| **P1** | Rename + restructure (geist/policy, geist/edge, geist/bin). #[non_exhaustive], PolicyError, Send+Sync | Done |
| **P2** | act-cli for Claude Code PreToolUse hook + catch_unwind + path canonicalization | Pending |
| **P3** | Escalate variant (Deny > Escalate > Allow lattice) + AgentPolicy CRD types | Pending |
| **P4** | geist-telemetry + shared kernel extraction decision | Pending |
| **P5+** | geist-edge HTTP (APIServer/APIClient + capability registry), eBPF, MCP adapter | Future |

## Crate Layout

| Crate | Path | Role |
|-------|------|------|
| `geist-policy` | `geist/policy` | PDP domain core |
| `geist-edge` | `geist/edge` | Edge adapter (stub) |
| `geist` | `geist/bin` | Binary + orchestrator |

## Proto API Naming Convention

Follows xDS type URL format: `{org}.{product}.{domain}.{version}.{Type}`

```
apis/proto/
└── mox/geist/
    ├── agent/v1/       ← package mox.geist.agent.v1 (agent tool governance)
    │   ├── agent_op.proto
    │   ├── policy.proto
    │   └── inputs.proto
    └── edge/v1/        ← package mox.geist.edge.v1 (edge config)
        └── edge.proto
```

| Domain | Package | Types |
|--------|---------|-------|
| Agent governance | `mox.geist.agent.v1` | AgentOp, AgentOpMatch, DenyAllowPolicy, PolicyDecision |
| Edge config | `mox.geist.edge.v1` | EdgeConfig, Capability |
| Traffic governance | `mox.geist.traffic.v1` | (future — HTTP/gRPC via rumi-http) |

Registry type URLs match: `mox.geist.agent.v1.AgentIdInput`, `mox.geist.agent.v1.ToolNameInput`, etc.

### Control Plane Architecture

```
Gateway API CRDs ──→ Control Plane ──→ xDS (ECDS) ──→ geist-edge ──→ rumi.evaluate()
AgentPolicy      ──→ translates to ──→ TypedExtensionConfig ──→ agent policy enforcement
HTTPRoute        ──→ translates to ──→ RDS/LDS              ──→ traffic routing
```

Agent policies are delivered as **typed extension configs via ECDS** — they're not listeners, routes, or clusters. This allows dynamic policy updates without edge restart, and coexistence with standard Envoy xDS resources.

## Settled Decisions (Not Open for Debate)

- **Deny-first evaluation** — consensus across Cedar, SCT, OWASP
- **`&self` PolicyEvaluator** — enables Arc sharing, matches hyper school pattern
- **Hexagonal architecture** — domain crate has zero HTTP types
- **AgentOp as domain context** — 6 fields: agent_id, tool_name, resource, operation, session_id, metadata
- **Tower in HTTP adapter only** — never in domain core
- **rumi-act and rumi-claude are SEPARATE crates** — domain PDP vs Claude-specific adapter
- **Keep 3-crate structure** — don't premature-split until coupling evidence manifests
- **AgentPolicy as Gateway API extension** — same policy attachment pattern (GEP-713) as SecurityPolicy/AuthorizationPolicy. rumi compiles both `HttpRouteMatch` (HTTP) and `AgentOpMatch` (agent) through the same engine
- **xDS namespace: `mox.geist.{domain}.v1`** — follows Envoy convention (`{org}.{product}.{domain}.{version}.{Type}`)
- **"Agentic Control Theory" is research framing only** — lives in papers/docs, not in crate names or wire format

## Research Corpus

### ACT Research (in scratch/)
- `scratch/act-synthesis-2026-02-19.md` — Unified synthesis (90+ sources, 10 high-concordance claims)
- `scratch/act-research-control-theory.md` — SCT, cybernetics, Ashby, composition problem
- `scratch/act-research-transparency.md` — Microscope, CoT fragility, FIDES
- `scratch/act-research-scaffolds.md` — Cedar, OPA, AgentSpec, enforcement taxonomy
- `scratch/act-research-naming-standards.md` — 5 ACT collisions, firewall analogy, standards landscape

### Source Papers (in research/)
- `dao-agentic-design-patterns.md` — Dao et al. (arXiv:2601.19752) — five-subsystem decomposition, 12 design patterns
- `miehling-systems-theory.md` — Miehling et al. (arXiv:2503.00237) — IBM FAST: "Agentic AI Needs a Systems Theory"
- `allegrini-formal-properties.md` — Allegrini et al. (arXiv:2510.14133) — temporal logic, 31 formal properties
- `camel-info-flow-control.md` — DeepMind (arXiv:2503.18813) — CaMeL: defeating prompt injection by design
- `kim-scaling-agent-systems.md` — Kim et al. (arXiv:2512.08296) — scaling agent systems

### Earlier Research (in research/)
- `geist-architectural-analysis-wasm-agents.md` — WASM agent architecture
- `geist-research-synthesis.md` — Earlier research synthesis
- `synthesis-agentic-data-plane.md` — Agentic data plane synthesis
- `synthesis-geist-adr.md` — Architecture decision records
- `sandbox-runtime-analysis.md` — Sandbox/SRT analysis

### Unsolved Frontier: Agent Composition
The deepest open problem. Per-action policy is necessary but not sufficient — agents can take individually-valid actions that compose into harmful sequences. Multi-agent interaction makes this worse (emergent behavior unpredictable from individual properties). IBM FAST Workshop, Allegrini temporal logic, and DeepMind CaMeL are the closest efforts. Nobody has solved it. See `scratch/act-research-control-theory.md` Section 6.4.

## Git Workflow

Inherits mox conventions (conventional commits, fetch+rebase, squash merge). Project-specific additions:

### Branching

- Branch per feature/fix: `feat/description`, `fix/description`, `refactor/description`
- Branch from origin/main: `git checkout -b feat/thing origin/main`
- Delete after merge

### Commits

- **Conventional commits**: `type(scope): message`
  - Scopes: `pdp`, `gateway`, `edge`, `experience`, `docs`, `ci`
  - Examples: `feat(pdp): add #[non_exhaustive] to PolicyDecision`, `fix(experience): sigil mobile scaling`
- **Atomic** — one logical change per commit
- **Co-author**: `Co-Authored-By: Claude <noreply@anthropic.com>`

### Pull Requests

- Short title (< 70 chars), detail in body
- Always include `## Test plan` section
- Squash merge to main — clean linear history
- Delete branch after merge

### Syncing

```bash
# NEVER git pull
git fetch origin
git rebase origin/main
```

### Deployment

- Push to `main` with changes in `docs/experience/` auto-deploys to Cloudflare Pages (`gestalt.mox.nexus`)
- Workflow: `.github/workflows/deploy-gestalt.yml`
- Manual deploy: `cd docs/experience && bun run deploy`
- Secrets: `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID` (GitHub Actions secrets)

## Playwright Screenshots

Always save screenshots to `.playwright-mcp/` (gitignored). Use relative paths:

```
filename: ".playwright-mcp/description.png"
```

Never save screenshots to the project root.

## Skills

- `.claude/skills/geist-rust-mastery/` — Rust architectural judgment (13 codebases)
- `.claude/skills/geist-gateway-patterns/` — Gateway API, xDS, proxyless gRPC, control plane patterns. 174k of reference material mapping service mesh → agent governance

## Quick Verification

```bash
cargo test --workspace
# Expected: 48 tests passing, 0 failures
```
