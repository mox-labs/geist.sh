---
title: What is geist.sh
description: A desktop AI collaborator with a governed runtime
order: 1
section: overview
---

## The Problem

200,000 developers running privileged autonomous agents. Shell access. Filesystem. Network. Browser.

No governance layer.

Agent frameworks give you capability. They don't give you control. Every tool call is a trust decision — and right now, that trust is implicit.

## The Equation

**geist** (agent) + **shell** (runtime) → **gestalt** (emergence)

geist.sh is a desktop application where knowledge workers — not just developers — set up their AI collaborator(s) with pre-installed skills, backed by a governed runtime that makes composition safe.

## Four-Layer Stack

| Layer | Name | Owner |
|-------|------|-------|
| L4 | Agentic | geist |
| L3 | Application | your code |
| L2 | Gateway | shell |
| L1 | Kernel | shell |

geist.sh provides L1 + L2 + L4. You bring L3.

## Architecture Qualities: ACES

The runtime is built around four architectural qualities:

- **Adaptable** — Swap the hosting runtime (axum, pingora, ext_proc sidecar) without touching any processor logic
- **Composable** — Processors compose through pipeline configuration and type-safe metadata, never through direct coupling
- **Extensible** — Adding a processor means registering an extension. No changes to core or adapter code
- **Software** — The "S" is deliberate. These aren't abstract principles — they're measurable properties of the implementation

## What This Is Not

geist.sh is **not** another agent framework. It's not a prompt router. It's not a chat UI.

It's a **governed runtime** — the layer between your agent and the capabilities it can reach. Every tool call, every external request, every filesystem operation flows through geist-edge. The runtime observes, governs, and routes.

## Current Status

geist.sh is in active research. The edge runtime core is implemented (P1 complete, 67 tests). The typed extension registry and access control processor are done (P1.5 complete, 40 tests). The axum HTTP adapter is next (P2).

See the [status page](/docs/status) for live phase tracking.
