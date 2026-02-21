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

Rust workspace at repo root. Three crates: `act/pdp` (PDP domain core), `geist/edge` (edge adapter stub), `geist/bin` (binary stub). SvelteKit experience at `docs/experience/`, deployed to `gestalt.mox.nexus`. Guild deliberation (10 members) completed 2026-02-19.

## Key Documents (Read in This Order)

1. `scratch/guild-deliberation-2026-02-19.md` — **START HERE** for architecture decisions, phased plan, guild outputs, validation criteria. Self-contained — resume work from this document alone.
2. `scratch/architecture-reframe-2026-02-19.md` — The "shell IS the runtime" reframe.
3. `scratch/project-status-2026-02-19.md` — Code inventory and test coverage.
4. `scratch/act-synthesis-2026-02-19.md` — ACT research synthesis (90+ sources).

## Implementation Plan (Guild-Approved)

| Phase | What | Status |
|-------|------|--------|
| **P1** | Rename → act/pdp, geist/edge, geist/bin. #[non_exhaustive], PolicyError | In progress |
| **P2** | act-cli for Claude Code PreToolUse hook | Pending |
| **P3** | Escalate variant (Deny > Escalate > Allow lattice) | Pending |
| **P4** | geist-telemetry + shared kernel extraction decision | Pending |
| **P5+** | Gateway HTTP, eBPF, MCP adapter | Future |

## Crate Layout

| Crate | Path | Role |
|-------|------|------|
| `act-pdp` | `act/pdp` | PDP domain core |
| `geist-edge` | `geist/edge` | Edge adapter (stub) |
| `geist` | `geist/bin` | Binary + orchestrator |

## Settled Decisions (Not Open for Debate)

- **Deny-first evaluation** — consensus across Cedar, SCT, OWASP
- **`&self` PolicyEvaluator** — enables Arc sharing, matches hyper school pattern
- **Hexagonal architecture** — domain crate has zero HTTP types
- **AgentOp as domain context** — 6 fields: agent_id, tool_name, resource, operation, session_id, metadata
- **Tower in HTTP adapter only** — never in domain core
- **rumi-act and rumi-claude are SEPARATE crates** — domain PDP vs Claude-specific adapter
- **Keep 3-crate structure** — don't premature-split until coupling evidence manifests

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

## Quick Verification

```bash
cargo test --workspace
# Expected: 5 doc-tests passing
```
