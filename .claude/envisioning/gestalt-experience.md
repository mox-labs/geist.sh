# gestalt.mox.nexus — Experience Vision

## What It Is

The product site for geist.sh. Where people encounter the project, understand what it does, and decide if it's for them. Not documentation — an experience.

**Domain**: gestalt.mox.nexus
**Stack**: SvelteKit, Cloudflare Pages, static prerender
**Location**: `docs/experience/`

## Aesthetic: The Terminal

The whole site is the terminal. Not a website with terminal-styled elements — a terminal that happens to be a website.

**Design system**: `--hud-*` CSS custom properties (`src/lib/tokens/hud.css`)
**Typeface**: JetBrains Mono (400, 500, 600) via bunny.net
**Background**: Void black `#000000` with CRT scanline overlay

### Three-Color Language

From the sigil, inherited by everything:

| Color | Hex | Role | Meaning |
|-------|-----|------|---------|
| Green | `#00FF41` | Accent, primary | Organic, agent, life, operational |
| Red | `#d45555` | Structure | Machine, enforcement, error |
| Blue | `#4da6ff` | Spark | Energy, information, spark |

Green dominates. Red appears for structure (heptagon, borders, error states). Blue is rare — reserved for energy and ignition.

### Typography Scale

All monospace. 10px–28px. Base 14px. Three line heights: tight (1.2), normal (1.5), relaxed (1.7). Spacing on a 4px grid.

## Current State

### Done

- **Landing page** — 5-section hero: full-viewport sigil, "the problem" terminal block, "the equation" (geist + shell = gestalt), four-layer stack diagram, "enter" CTA
- **Animated sigil** — 11-second cinematic boot sequence (spark → rays → star → heptagon → triskelion arms → assembly pulse → orbit), then looping orbital state with entity expansion. RAF-driven, composable Svelte components
- **HUD design tokens** — complete token system in CSS custom properties
- **Shell layout** — sidebar nav + main area, mobile drawer, responsive breakpoints
- **Content system** — content-collections schema (docs + decisions), Prose renderer with GFM, no content written yet
- **CI** — GitHub Actions workflow triggers on `docs/experience/**` changes, deploys to Cloudflare Pages

### Empty

- Navigation sections (defined in code, array is `[]`)
- Content markdown (`docs/content/` directory doesn't exist)
- Cloudflare Pages project (workflow ready, project TBD)

## Vision: Architecture Diagram

Inspired by Baseten's isometric infrastructure animation — but terminal-native, not corporate-clean.

### What We're Showing

The geist-edge processing pipeline as a living diagram:

```
Request enters →
  CapabilityServer [inbound adapter]
    → Processor pipeline [ACL, auth, rate limit — green flowing through]
      → Capability Router [decision point]
        → Upstream processors [request adaptation]
          → CapabilityClient [resolves capability → target]
            → Agent runtime
```

Elements orbit around the pipeline. Extensions self-register (appear from edges, dock into the pipeline). Policy decisions flash red/green. Capabilities resolve with blue spark lines.

### How: Code-Driven SVG

Not Lottie (After Effects dependency, opaque JSON blobs). Code-driven SVG with Svelte, matching the existing sigil approach:

- **SVG paths** for isometric shapes (cubes, cylinders, connecting lines)
- **Svelte components** for each element (like the sigil's `Heptagon.svelte`, `Orbit.svelte`)
- **RAF loop** for continuous animation (established pattern in `Sigil.svelte`)
- **CSS transforms** for isometric projection (`matrix()` skew)
- **Three-color language** for semantic meaning (green=flowing, red=denied, blue=resolving)

Composable: each piece is a Svelte component with its own animation state. The whole diagram is assembled from parts, just like the sigil.

### Isometric Grid

```
transform: rotateX(60deg) rotateZ(-45deg);
```

Or via SVG matrix transforms (as Baseten does). The grid creates depth without actual 3D. All shapes are flat SVG paths — cubes are three parallelograms, cylinders are ellipses + rectangles.

## Content Roadmap

### Pages Needed

| Page | Section | What |
|------|---------|------|
| Why | Overview | The problem (200k agents, no governance) and the approach |
| Architecture | Architecture | Capability-led connectivity, processing pipeline, ACES |
| Stack | Architecture | Rust + Python + SvelteKit, why each choice |
| Getting Started | Reference | First steps with geist-sh |
| ADR-001 | Decisions | Rust + PyO3 for agent runtime |
| ADR-002 | Decisions | SLICK architecture |

### Content Format

Markdown with content-collections. Schema already defined:
- `docs`: title, description, order, section (overview/architecture/reference)
- `decisions`: title, date, status (proposed/accepted/superseded/deprecated)

## Future Possibilities

- **Interactive pipeline demo** — click a processor, see its config, watch it evaluate a request
- **Live stats** — real telemetry from a running geist-sh instance
- **Extension catalog** — browse available processors, see their type URLs and config schemas
- **Agent playground** — configure an agent, see its capability surface computed in real-time
