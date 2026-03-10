# Streaming Composition Protocol

> From P2 guild review discussion, 2026-03-07. Crystallized insights toward the universal composition model.

## The Convergence

Three things that looked separate are the same thing:

1. **geist-edge processors** — process HTTP requests through a pipeline
2. **slick Components** — name, kind, consumes/produces, process(context) → Outcome
3. **matrix DAG** — orchestrate components with typed dependencies

Processors ARE Components. The Sequence compositor IS a DAG (currently a degenerate linear one). The pipeline IS a Construct flowing through the graph.

## Reactive Signal Model

This is NOT request/response. This is reactive signal processing.

- **Intent signals** (input stream) — what the client wants, plus ongoing steering
- **Outcome signals** (output stream) — what is happening/happened, streamed as the Construct is being built
- The **Construct** is the accumulating state as signals flow through the DAG
- Components are **reactive operators** — they transform intent signals into outcome signals, enriching the Construct as they go

Mono (single intent, single outcome) is the degenerate case — classic HTTP request/response. Flux (streaming intent, streaming outcome) is the real model — agentic, steerable, long-running.

This is Project Reactor's model freed from HTTP. From any interface to any system.

## IntentConstructProtocol

The application-level protocol that doesn't exist yet. Intent Driven Design needs this.

**Intent** → **Construct** (constructs the outcome) → **Outcome**

1. Intent signal arrives
2. Construct is created — the thing under construction
3. Components (reactive operators) process intent signals → outcome signals, enriching the Construct
4. The Construct accumulates enrichments — each component's contribution is structural
5. Outcome signals stream back as the Construct is built

The protocol defines:
- How intent signals flow in (initial + steering)
- How the Construct accumulates (enrichments from components)
- How outcome signals flow out (aggregated from DAG)
- Circuit establishment, mux/demux, backpressure

Wire transport: gRPC bidi streaming, WebSocket, or in-process (no wire at all). The protocol is transport-agnostic.

## Construct — The Universal Subject

Everything is I/O. The Construct carries the input, constructs the outcome.

Components don't see protocol. They see intent-level enrichments:
- An ACL component doesn't read `request.headers.get("x-geist-agent-id")` — it reads an `AgentContext` enrichment
- The adapter translates protocol-specific input into intent-level enrichments on the Construct

| Adapter | Protocol Input → | Intent Enrichments | → Protocol Output |
|---------|------------------|--------------------|-------------------|
| HTTP (axum, pingora) | request headers/body | AgentContext, Resource, etc. | response status/headers/body |
| CLI | args/stdin | same enrichments | result/stdout |
| In-process | function params | same enrichments | return value |
| gRPC bidi | stream messages | same enrichments | stream messages |

The adapter's job:
- **Inbound**: protocol-specific input → intent-level enrichments on Construct
- **Outbound**: outcome enrichments on Construct → protocol-specific output

A component written for HTTP proxy works unmodified in CLI — IF the adapter provides the same enrichments.

## DAG, Not Linear Chain

Production proxies (Envoy, SCG, Pingora) all use linear chains. That's their limitation, not a law of physics.

The actual constraint: **phases are sequential** (request headers before request body before upstream before response — protocol semantics). Within a phase, there's no reason components can't run in parallel if they don't depend on each other.

```
Phase: request_headers
  ┌─ ACL (consumes: nothing) ──────────┐
  │                                     ├─ join → aggregated mutations
  └─ AuthN (consumes: nothing) ────────┘
                                        │
                                        ▼
  ┌─ RateLimit (consumes: AuthIdentity) ┐
  │                                     ├─ join
  └─ Routing (consumes: AuthIdentity) ──┘

Phase: request_body (sequential protocol constraint)
  ...
```

### Design-Time Validation

SCG hardcodes filter ordering in Java. Envoy lets you misconfigure and fail at runtime. Both are worse.

`ComponentManifest` has `consumes: Vec<String>` and `produces: Option<String>`. A DAG with typed edges validates at composition time:

- Component A produces `AuthIdentity` (type URL)
- Component B consumes `AuthIdentity`, produces `RateQuota`
- Component C consumes nothing (ACL — reads enrichments directly)
- A and C run in parallel. B waits for A. **Validated before a single request arrives.**

## Steerability — Circuit Establishment

Traditional exchange: request in → processing → response out. No steerability during processing.

When components are agentic (autonomous, intelligent), you need to steer them DURING processing. Not fire-and-forget.

1. Initial intent signal traverses the DAG — not demuxed. Each component registers their in/out streams as it passes through.
2. Outcome signal streams from components get aggregated back to client (mux).
3. After circuit construction: subsequent intent signals on the input stream get demuxed to specific components (steering).

```
Client                    Edge                     Components
  │                        │                          │
  │──intent──────────────▶│── traverses DAG ────────▶│
  │                        │   (circuit setup)        │
  │                        │   components register    │
  │                        │   in/out streams         │
  │                        │                          │
  │◀──outcome stream──────│◀──mux────────────────────│
  │──steering signal──────▶│──demux──▶ Component B    │
  │                        │                          │
```

The edge IS the protocol server. It owns:
- **Input routing**: demux steering signals to components after circuit setup
- **Output aggregation**: mux component outcome streams into one stream to client

The client sees one intent stream in, one outcome stream out. Doesn't know about internal components.

## The Component Trait

One trait. Reactive operator over signals. Protocol-agnostic.

```rust
trait Component {
    fn name(&self) -> &str;
    fn consumes(&self) -> &[TypeUrl];
    fn produces(&self) -> Option<TypeUrl>;
    fn process(&self, construct: &mut Construct) -> BoxFuture<'_, Result<Outcome, ComponentError>>;
}
```

Phase is in the Construct. Protocol is in the adapter. The component just processes.

The current P2 `Processor` trait (4 phase methods, `&http::request::Parts`) is the wrong cut — it leaks HTTP into the component and encodes phase in the trait. The Component trait replaces it.

## Structural Observability

The Construct provides observability for free. Every component's contribution is an enrichment. You can reconstruct exactly what happened, who produced what, in what order. That's the trace — not bolted-on OTel spans, but structural observability from the composition model itself.

## Open Questions

- What does `Construct` look like in Rust? Generic over subject, with typed enrichments?
- How does the streaming model interact with the Construct? Is each stream message an enrichment?
- How do phase constraints (HTTP sequential phases) express as DAG constraints?
- What's the circuit establishment lifecycle? Registration, streaming, teardown?
- How does backpressure propagate through the DAG?
- Relationship to ext_proc bidi streaming — same pattern, different scope? ext_proc is platform-agnostic (protobuf) but the serialization cost is high for in-process. Can ext_proc be the wire protocol for remote components while Construct stays the in-process model?
- Mono/Flux unification — how does one Component trait handle both single-shot and streaming?
