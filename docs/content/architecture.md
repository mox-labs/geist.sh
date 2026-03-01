---
title: Architecture
description: The composable data plane
order: 2
section: overview
---

## Overview

geist-edge is a composable data plane runtime. It implements the Extension Protocol Adapter pattern — portable processing capabilities that work identically whether hosted in an axum server, a pingora proxy, or an Envoy ext_proc sidecar.

```
Tauri (geist-shell)
  └─ geist-edge (axum adapter)
      ├─ ProtocolServer: inbound → processors → agent
      ├─ ProtocolClient: agent → processors → external
      ├─ Processors: registered via typed extension registry
      └─ Metadata: type-safe inter-processor communication
          └─ Agent runtime (Claude SDK)
```

## Three Composable Layers

The same building blocks compose at every scale:

| Layer | Role | Example |
|-------|------|---------|
| L1 Network/Proxy | Transport, TLS, connection management | axum, pingora, Envoy |
| L2 Gateway | Processor pipeline — the extension layer | Access control, rate limiting, logging |
| L3 Business Logic | Domain-specific processors | Agent governance, capability routing |

L2 and L3 are **identical** regardless of which L1 hosts them. This is the core portability guarantee.

## Extension Protocol Adapter

The EPA sits between the hosting runtime and the processor pipeline. It translates runtime-specific types into the universal processing contract (`ProcessingRequest`/`ProcessingResponse`), runs the pipeline, and translates back.

```
Runtime Request                    Runtime Response
      │                                  ▲
      ▼                                  │
┌─────────────────────────────────────────────┐
│           Extension Protocol Adapter         │
│                                             │
│  translate → pipeline → translate           │
│                                             │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐    │
│  │ Proc A  │→ │ Proc B  │→ │ Proc C  │    │
│  └─────────┘  └─────────┘  └─────────┘    │
└─────────────────────────────────────────────┘
```

The adapter is the only component that knows about the hosting runtime. Processors never see HTTP types, gRPC frames, or MCP messages directly — they work with the universal contract.

## Processor Model

A processor is the unit of composable logic. Each processor:

- Implements the `Processor` trait (`&self` — immutable, shareable via `Arc`)
- Owns its configuration/policy type (no generic evaluator)
- Compiles policy into efficient matchers at construction time
- Communicates with other processors via typed metadata, never direct coupling

```rust
#[async_trait]
pub trait Processor: Send + Sync {
    async fn process_request_headers(
        &self, req: &mut ProcessingRequest, meta: &mut Metadata
    ) -> PhaseResult { PhaseResult::Continue }

    async fn process_response_headers(
        &self, resp: &mut ProcessingResponse, meta: &mut Metadata
    ) -> PhaseResult { PhaseResult::Continue }
}
```

## Pipeline Composition

Processors compose into pipelines via the Sequence compositor. The pipeline runs each processor in order. Any processor can:

- **Continue** — pass to the next processor
- **Mutate** — modify the request/response and continue
- **Respond** — short-circuit with an immediate response (deny, redirect, etc.)

This is the same filter chain model proven by Envoy, Spring Cloud Gateway, and every serious proxy — but portable across runtimes.

## Typed Extension Registry

New processors register without modifying core code:

1. Implement the processor + factory (`IntoProcessor` trait)
2. Self-register via `inventory::submit!`
3. Add Cargo dependency to the composition root

That's it. The core binary calls `collect_extensions()` once. It never changes.

See [Capability-Led Connectivity](/docs/capability-led-connectivity) for the deeper connectivity model.
