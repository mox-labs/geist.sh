---
title: Capability-Led Connectivity
description: From endpoint-coupled to capability-driven
order: 3
section: architecture
---

## Three Shifts

The capability-led connectivity model represents three fundamental shifts from traditional API management:

| From | To | What Changes |
|------|----|-------------|
| Endpoint-coupled | Capability-driven | Consumers bind to logical capability IDs, not network addresses |
| Reimplementation | Composition | Cross-cutting concerns compose via pipeline, not per-service code |
| Coupled config | Separated config | Policy and routing decouple from processor implementation |

## The Naming Arc

The evolution of the core abstraction tells the story:

**APIServer** → **ProtocolServer** → **CapabilityServer**

- *APIServer*: Address-coupled. Knows where things live. Tightly bound to HTTP.
- *ProtocolServer*: Protocol-aware. Understands HTTP, gRPC, MCP as first-class citizens. But still thinks in terms of endpoints.
- *CapabilityServer*: Capability-driven. Consumers declare what capability they need. The runtime resolves where it lives, how to reach it, and what governance applies.

## Processing Pipeline

The pipeline processes requests through four phases:

```
request_headers → request_body → [upstream] → response_headers → response_body
```

Each phase runs the full processor chain. Processors declare which phases they participate in via `ProcessingMode` — a headers-only processor (like access control) skips body phases entirely.

### PhaseResult Vocabulary

Every processor phase returns one of:

- **Continue** — no opinion, pass through
- **Mutate** — modify headers/body, then continue
- **Respond** — short-circuit the pipeline with an immediate response

This vocabulary maps directly to Envoy's ext_proc `ProcessingResponse` variants, ensuring wire-level compatibility.

## Boundary and Encapsulation

Governance is intrinsic, not bolted on. The capability surface — what an agent can reach — is determined by three factors:

1. **Registration** — what capabilities exist in the catalog
2. **Policy** — what policies apply to this consumer for this capability
3. **Health** — whether the backing service is available

An agent can only invoke capabilities present in its surface. This follows the object-capability security model: authority comes from possession of a reference, not from identity checks after the fact.

### Two Enforcement Layers

- **Proactive scoping**: Construct the capability surface at session start. Absent capabilities are unreachable — the agent never even sees them.
- **Reactive enforcement**: Evaluate policy at request time for capabilities that are present. Deny-first evaluation ensures unknown states fail closed.

## Extension Protocol Adapter

The EPA is the key enabler of portability. It provides a single processing contract regardless of hosting runtime:

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│    axum      │     │   pingora    │     │  ext_proc    │
│   adapter    │     │   adapter    │     │   adapter    │
└──────┬───────┘     └──────┬───────┘     └──────┬───────┘
       │                    │                    │
       ▼                    ▼                    ▼
┌─────────────────────────────────────────────────────────┐
│              ProcessingRequest / Response                │
│                 (universal contract)                     │
├─────────────────────────────────────────────────────────┤
│                  Processor Pipeline                      │
│           [Access Control] → [Logging] → ...            │
└─────────────────────────────────────────────────────────┘
```

The contract uses `ProcessingRequest`/`ProcessingResponse` from the Envoy ext_proc protocol definitions — not hand-rolled types. This means a processor written for geist-edge can participate in an Envoy ext_proc deployment with zero code changes.

## Decision Framework

When placing a new concern, ask six questions:

| Question | If Yes → |
|----------|----------|
| Does it enforce rules on requests? | **Processor** |
| Does it translate between runtime and pipeline? | **Adapter** |
| Does it define what can be reached? | **Capability surface** |
| Does it determine how something is reached? | **Routing / cluster config** |
| Is it user-defined configuration? | **Policy** (owned by a processor) |
| Does it change over time without restart? | **ECDS distribution** |
