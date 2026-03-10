# Architecture Overview

> **Status: Experimental / Alpha.** All APIs are volatile.

## The Invariant

Everything transits the mediation plane. No exceptions.

geist-run cannot reach capabilities directly. It has no addresses to route to — the routing table lives in geist-edge. This is structural, not policy.

## System Architecture

```mermaid
graph TB
    subgraph edge["geist-edge (Rust)"]
        CS["Capability Server<br/><i>accepts connections</i>"]
        CR["Capability Resolution<br/><i>xuma matcher API</i>"]
        CC["Capability Client<br/><i>authenticated upstream call</i>"]
        PP["Processor Pipeline<br/><i>evaluate, mutate, deny</i>"]
    end

    subgraph run["geist-run (Python)"]
        A["Agent runtime<br/><i>claude-agent-sdk</i>"]
    end

    U[User] --> CS
    CS --> PP
    PP --> A
    A -->|capability call| PP
    PP --> CR
    CR --> CC
    CC --> CAP[Capability Implementation]

    style edge fill:#2d3748,stroke:#e2e8f0,color:#e2e8f0
    style run fill:#2d3748,stroke:#718096,color:#e2e8f0
```

geist-edge does three things:

1. **Capability server** — accepts inbound connections from users and capability calls from geist-run
2. **Capability resolution** — xuma (unified matcher API). Given a capability name, find the implementation. Deterministic, not LLM-dependent.
3. **Capability client** — executes the authenticated call to the upstream implementation

## Hexagonal Boundary

geist-edge core has zero runtime dependencies. Runtime adapters are feature-gated.

```mermaid
graph TB
    subgraph core["geist-edge core (zero runtime deps)"]
        V["Protocol vocabulary: http::, bytes, serde"]
        D["Domain: Processor, Sequence, Registry"]
        T["Types: PhaseResult, ProcessingMode"]
    end

    subgraph adapters["Adapter boundary (feature-gated)"]
        AX["axum<br/>tower · hyper · tokio"]
        PI["pingora<br/>pingora-core"]
        EX["ext_proc<br/>tonic · rumi-http"]
    end

    core --> adapters

    style core fill:#2d3748,stroke:#e2e8f0,color:#e2e8f0
    style adapters fill:#1a202c,stroke:#4a5568,color:#e2e8f0
```

`http::` is protocol vocabulary — zero runtime dependencies, lives in core. Same as `serde` for serialization.

## Security Model

Three layers of defense in depth:

| Layer | Mechanism | Scope |
|-------|-----------|-------|
| Architectural | geist-run has no capability addresses | Sufficient for local-first |
| Network | NetworkPolicy restricts geist-run egress to edge only | Cloud-native |
| Identity | mTLS — capabilities only accept edge's certificate | Production |

Each layer is independently sufficient. All three apply in production.

## Crate Layout

| Crate | Path | Role |
|-------|------|------|
| `geist-edge` | `geist/edge` | Mediation plane — Processor trait, compositor, registry |
| `geist-sh` | `geist/bin` | Binary — composition root, starts adapter |

External: [**slickit**](https://github.com/mox-labs/slick) (Apache-2.0) — typed extension registry (`TypedConfig`, `TypedRegistry<T, E>`).

## The Wire

geist-edge ↔ geist-run: bidi streaming gRPC over HTTP/2, following the Envoy ext_proc pattern.

| Property | Why |
|----------|-----|
| Process isolation | Blocked LLM call doesn't take down the gateway |
| Typed messages | Structured, interceptable, full metadata |
| mTLS native | Security at the transport layer |
| Swappable | The proto is the contract — swap the Python runtime, keep the proto |
