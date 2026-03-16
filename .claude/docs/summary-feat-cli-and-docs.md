# Branch: feat/cli-and-docs

Changes on this branch since diverging from main.

## Commits

1. `55423ea` **feat(cli): add geist-cli + envisioning docs + dev mdbook**
2. `97ef1cb` **feat(hud): add @geist/hud — Lit Web Component library for agent projection**
3. `f653584` **chore(deps): switch slickit to crates.io 0.1.0**

## What's Here

### geist-cli (`geist/cli/`)
CLI tool for geist.sh. Added to workspace.

### @geist/hud (`geist/hud/`)
Lit Web Component library for visualizing agent state. 18 source files, 1,289 lines, 27KB bundle (6.9KB gzip).

**Components:**
- `<geist-projection>` — composition root, provides context
- `<geist-breath>` — RAF sinusoidal ambient glow, color + rate from phase
- `<geist-dot>` — phase-colored status indicator (10px, pulses when active)
- `<geist-phase>` — phase label + resource path
- `<geist-trace>` — streaming event feed with three-layer disclosure (summary → detail → deep)
- `<geist-sparkline>` — SVG polyline mini chart for latency history
- `<geist-pipeline>` — SVG processor flow diagram with dashed edges

**Transport:** WebSocket (exponential backoff reconnect) + mock state machine for dev.

**CSS:** Custom property cascade — `var(--hud-cyan, #00d4ff)` pattern inherits host tokens.

**Usage:** `<geist-projection mock></geist-projection>` or `<geist-projection ws="ws://..."></geist-projection>`

**Hosts:** geist-shell (Tauri webview), mox.hud (SvelteKit panel), docs (static embed).

### slickit dep switch
`Cargo.toml` workspace dep changed from `path = "../slick"` to `"0.1.0"` (crates.io). 48 tests pass unchanged.

## Open Questions

- **Container model**: current `<geist-projection>` is a panel. Should be strip + overlay for ambient agent presence on HUD chrome.
- **Multi-agent visibility**: how to show which stacks/dirs have active agents.
- **Faceplate/steering**: SP/PV display, force gradient, bumpless transfer — not yet implemented.
- **Wire protocol gaps**: missing confidence scores, mode toggle, circuit breaker events.

## Design Research

See `scratch/cis-interface-synthesis.md` (design argument) and `scratch/cis-interface-research-synthesis.md` (18 patterns, 10 principles, component spec).
