# Capability-Led Connectivity — Architecture Reference

The connectivity model that geist-edge implements. Read this before making architectural decisions about geist-edge — it provides the mental model for why processors, adapters, policies, and capabilities exist as separate concerns.

## 1. The Connectivity Model

### What a Connectivity Model Is

A connectivity model answers three questions:
1. **What is the unit of connectivity?** What do entities connect TO?
2. **How is connectivity organized?** What structure governs connections?
3. **What does the organizing principle make possible — and impossible?**

| Model | Unit | Organizing Principle | Enables | Limits |
|-------|------|---------------------|---------|--------|
| Point-to-point | Address (host:port) | Direct coupling | Simplicity | N*M spaghetti |
| ESB | Message channel | Centralized bus | Decoupling | SPOF, bus becomes bottleneck |
| API-led | API endpoint (URL) | Three-layer hierarchy (Experience/Process/System) | Reusable API layers | Endpoint coupling, runtime lock-in |
| Service mesh | Service identity | Sidecar interception + mTLS | Zero-trust networking | Policy coupled to proxy extension model |
| **Capability-led** | **Capability (logical function)** | **Processing contract portable across runtimes** | **Runtime portability, invisible decomposition, policy-processor separation** | Model complexity; requires registry + control plane |

### What "Led" Means

"Led" means the organizing principle that determines how connections form. In API-led connectivity, APIs lead — you decompose into three layers of APIs. The APIs ARE the connectivity. You don't connect systems; you connect APIs.

In capability-led connectivity, capabilities lead — you decompose into portable processing capabilities. The capability is the invariant; the endpoint, the runtime, the protocol, the deployment topology — all variables that can change without breaking connectivity.

### The Three Shifts

**Shift 1: Endpoint → Capability.** Clients depend on logical capability IDs, not URLs. Decomposition (splitting a service into microservices) becomes a deployment operation, not a migration project.

**Shift 2: Reimplementation → Composition.** Processing logic (auth, rate limiting, transformation) is portable across runtimes via the ProcessingRequest/ProcessingResponse contract. Same capability runs on axum, pingora, envoy, embedded. N + M, not N * M.

**Shift 3: Coupled → Separated configuration.** Policies (what) separated from processors (how). The platform can swap the rate-limiting implementation without changing any tenant's policy configuration. Tenants can change their policies without touching the implementation.

These separations stack and multiply — that multiplication IS composability.

## 2. CapabilityServer and CapabilityClient

### The Naming Arc

Three names, three conceptual levels — each decouples from one more variable:

**APIServer / APIClient** — Unit: API endpoint. Clients coupled to address, protocol, runtime. Decompose your service → every client breaks.

**ProtocolServer / ProtocolClient** — Unit: protocol. Decouples from specific endpoints — speak HTTP, gRPC, MCP. But clients must still know which protocol and where.

**CapabilityServer / CapabilityClient** — Unit: capability (logical function). Decouples from both endpoint AND protocol. Client says "I need payments." Infrastructure resolves protocol, address, runtime. The capability is the contract; everything else is variable.

The codebase currently uses ProtocolServer/ProtocolClient. The conceptual model uses CapabilityServer/CapabilityClient. Implementation stays concrete (HTTP-first, Path B) until a second protocol forces the abstraction.

### CapabilityServer Affordances

The inbound edge. Exposes capabilities, not endpoints:

- **Register capabilities** — "I provide payments, version 2" — not "I listen on port 8080 at /api/v2/payments"
- **Declare processing requirements** — "inbound traffic must go through auth, rate-limiting, telemetry" — the pipeline is part of the capability declaration
- **Invisible decomposition** — split into three services behind the same capability ID. Clients never know. Control plane updates routing.
- **Protocol flexibility** — same capability reachable over HTTP, gRPC, MCP. Adapters handle translation. Capability is invariant.

### CapabilityClient Affordances

The outbound edge. Consumes capabilities, not endpoints:

- **Address capabilities, not URLs** — "connect to the payments capability" — control plane resolves via xDS
- **Protocol-transparent consumption** — client doesn't know if payments is HTTP or gRPC
- **Governed egress** — every outbound call goes through the processing pipeline. Governance IS the connectivity path, not a layer you can bypass.
- **Composition** — compose capabilities by addressing them. Infrastructure handles the rest.

### Same Code, Three Deployment Contexts

CapabilityServer and CapabilityClient are ACES components — identical processor pipeline code in:
- **Proxyless** (library, embedded in application process)
- **Runtime** (sidecar, standalone process per service)
- **Gateway** (shared infrastructure, edge proxy)

The adapter changes. The processors don't.

## 3. Processing Pipeline Flow

### End-to-End Topology

Validated against Envoy's upstream/downstream filter architecture:

```
CapabilityServer [protocol adapter: inbound]
  → downstream processors [about the requester: auth, policy, rate limit]
    → Capability Router [select capability to route to]
      → upstream processors [about the target: request adaptation, circuit breaking]
        → CapabilityClient [capability ID → target addresses via xDS]
          → upstream target

Response reverses both chains.
```

**Downstream processors** concern the requester — identity, authorization, rate limiting. They run before routing because they don't need to know the target.

**Upstream processors** concern the target — request adaptation, circuit breaking, retry policy. They run after routing because they need to know which capability was selected.

The **Capability Router** is the boundary between downstream and upstream. It selects a capability based on request attributes (path, headers, tool name) and resolved routes.

### Mapping to Existing Code

| Concept | Code | Path |
|---------|------|------|
| Processor | `trait Processor` (`&self`, 4 phase methods, `BoxFuture`) | `geist/edge/src/processor.rs` |
| Phase result | `enum PhaseResult` (Continue, Mutate, Respond) | `geist/edge/src/phase.rs` |
| Processing mode | `struct ProcessingMode` (request_body, response_body flags) | `geist/edge/src/phase.rs` |
| Pipeline compositor | `struct Sequence` (linear, ordered execution) | `geist/edge/src/compositor/sequence.rs` |
| Pipeline outcome | `enum SequenceOutcome` (Continue, Respond, Error) | `geist/edge/src/compositor/sequence.rs` |
| Failure handling | `enum FailureMode` (FailClosed, FailOpen) | `geist/edge/src/compositor/sequence.rs` |
| Extension factory | `trait IntoProcessor` (associated `Config` type) | `geist/edge/src/registry.rs` |
| Extension registry | `ProcessorRegistryBuilder` → `ProcessorRegistry` | `geist/edge/src/registry.rs` |
| Self-registration | `register_processor!` macro + `ProcessorRegistration` | `geist/edge/src/registry.rs` |
| Pipeline config | `TypedConfig { type_url, config }` | `geist/edge/src/registry.rs` |
| Access control | `AccessControlProcessor` (deny-first, rumi matchers) | `geist/acl/src/processor.rs` |

The Capability Router and upstream/downstream split are Phase 6+ (enterprise). Current implementation uses a single Sequence compositor for the full pipeline. The compositor abstraction can support multiple topologies (Sequence, Graph/DAG) — Sequence is the first, covering linear pipeline needs.

### Phase Model

Four phases per request, matching ext_proc's processing lifecycle:

```
request_headers  →  request_body  →  [upstream]  →  response_headers  →  response_body
```

- **request_headers**: Always processed. All processors participate. This is where auth, policy, and routing decisions happen.
- **request_body**: Only processed if any processor opts in via `ProcessingMode`. Body is buffered by the adapter.
- **response_headers**: Always processed. Processors can mutate response headers (add CORS, security headers).
- **response_body**: Only processed if any processor opts in. Used for response transformation.

The aggregate mode is the union (most-permissive) of all processor modes — if any processor needs the body, the adapter buffers it for everyone.

### PhaseResult Vocabulary

Each processor returns one of three results per phase:

- **`Continue`** — No mutation. Pass to next processor.
- **`Mutate(HeaderMutation)`** — Apply header mutations, continue pipeline. Mutations accumulate across processors. Processors see the original message, not prior mutations.
- **`Respond(ImmediateResponse)`** — Short-circuit. Send this response directly to the client. Terminates all subsequent processors in the current phase AND all subsequent phases.

### Cross-Phase Termination (Dijkstra I2)

If any phase returns `Respond`, all subsequent phases are skipped. The adapter constructs the response directly from the `ImmediateResponse`. No upstream forwarding, no response phases.

This is how policy denial works: `AccessControlProcessor` returns `PhaseResult::Respond(ImmediateResponse { status: 403, body: "denied" })` during `request_headers`. The request never reaches the upstream target.

### Failure Modes

- **FailClosed** (default): On processor error, generate error response and stop. Deny by default. This is the safe default for security-sensitive pipelines.
- **FailOpen**: On processor error, log and continue. Used for non-critical processors (telemetry, optional enrichment) where availability matters more than enforcement.

## 4. Protocol Mechanics

### The Invariant and the Variable

The capability is the invariant. The protocol is the variable.

An agent's "read file" capability might be:
- A tool call over MCP (`tool_name: "read"`, `arguments: {path: "/foo"}`)
- An HTTP GET to a REST API (`GET /files/foo`)
- A gRPC unary call (`FileService.Read(ReadRequest{path: "/foo"})`)

The edge's role is three-fold:

**1. Encode protocol-specific mechanics into a common processing surface.** Each protocol has its own `DataInput` impls for the rumi matcher engine. HTTP has `PathInput`, `MethodInput`, `HeaderInput`. MCP would have `ToolNameInput`, `ArgumentInput`. These are the "protocol contracts as mechanics" — concrete mechanics formalized as matchable inputs.

**2. Run protocol-agnostic processing over that surface.** The processor pipeline doesn't know if it's processing an HTTP request or an MCP tool call. An `AccessControlProcessor` evaluates deny/allow rules against whatever inputs are registered for that context type. The rules are protocol-aware (they match on protocol-specific fields); the evaluation engine is protocol-agnostic.

**3. Translate back.** A `PhaseResult::Respond(ImmediateResponse)` means "deny this" regardless of protocol. The adapter translates into HTTP 403, MCP error response, or A2A task failure.

### Extension Protocol Adapter Pattern

The adapter translates between protocol-specific mechanics and the universal processing contract:

```
Protocol-specific input (HTTP headers, gRPC metadata, MCP tool call)
  → ProcessingRequest (universal contract, from ext_proc protos)
    → Processor pipeline (protocol-agnostic)
      → ProcessingResponse / PhaseResult
        → Protocol-specific output (HTTP response, gRPC status, MCP result)
```

The `ProcessingRequest`/`ProcessingResponse` contract is the seam. On one side: protocol mechanics. On the other: protocol-agnostic processing. The adapter is the translator.

### Capability as Invariant, Protocol as Variable

A capability provider can expose the same capability over multiple protocols:

```
Capability: "payments.charge"
  ├─ HTTP: POST /payments/charge (REST adapter)
  ├─ gRPC: PaymentService.Charge (gRPC adapter)
  └─ MCP: tool "charge-payment" (MCP adapter)
```

One capability, multiple protocol projections. The processing pipeline runs once per request regardless of protocol. In API-led, each protocol exposure is a separate API. In capability-led, it's one capability with derived protocol projections.

### Current Scope

The ext_proc contract is HTTP-native. Current implementation translates HTTP mechanics to HTTP processing via `HttpMessage` from `rumi-http`. Cross-protocol capability resolution (MCP client → HTTP provider) is a future capability the model predicts but the implementation doesn't yet support.

## 5. Boundary and Encapsulation

### Governance as Intrinsic

In API-led connectivity, governance is bolted on. APIs exist independently of the governance layer. You can call an API directly, bypassing the gateway.

In capability-led connectivity, governance is intrinsic. The CapabilityClient can only reach capabilities through the processing pipeline. There is no "direct call" that bypasses governance because capability resolution itself goes through the infrastructure. The pipeline isn't a layer between client and server — it IS the capability channel.

### What Encapsulation Means

An agent inside the geist-edge boundary cannot:
- **Make a raw network call.** All egress goes through CapabilityClient → processing pipeline.
- **Call a tool without governance.** Tool calls are capabilities, routed through the pipeline.
- **Observe the protocol.** Protocol is an implementation detail resolved by infrastructure.
- **Know physical addresses.** Addressing is logical (capability ID), not physical (URL).

This is information hiding at the connectivity level. The agent knows WHAT capabilities it can address. The boundary determines the agent's **capability surface**.

### The Capability Surface

An agent's capability surface — the set of capabilities available to it — is determined by:

1. **Registration** (CapabilityServer) — which capabilities exist in the system
2. **Policy** (AccessControlProcessor) — which capabilities are allowed for this agent
3. **Resolution** (control plane / xDS) — which implementations are currently healthy

The surface is dynamic. It changes at runtime (ECDS delivers new policy, a provider goes down, a new capability is registered). The agent doesn't manage this — the infrastructure does.

### Two Enforcement Layers

**Proactive scoping (capability surface construction):**
Before the agent acts, the system constructs its capability surface. Capabilities not in the surface don't exist — they're absent, not denied. The agent can't even formulate the request because the affordance isn't present. This is the DNS analogy: you can't query what doesn't resolve.

**Reactive enforcement (request-time evaluation):**
Even with a scoped surface, runtime evaluation is necessary because:
- **Arguments matter.** "Read file" is in the surface, but "read /etc/shadow" might be denied.
- **Temporal context matters.** The third file write in a session might be denied.
- **Dynamic context matters.** Rate limits, budget constraints change at runtime.

The deny-first evaluation in `AccessControlProcessor` is the reactive layer. Capability surface construction is the proactive layer. Both are necessary.

### When a Capability Isn't Available

The answer depends on WHY:

| Reason | Layer | Agent Experience |
|--------|-------|-----------------|
| **Absent** | Proactive scoping | Affordance doesn't exist. Agent cannot formulate the request. |
| **Not registered** | Resolution | Capability ID unknown. Resolution failure. |
| **Policy denied** | Reactive enforcement | Agent attempted, was refused with reason. |
| **No healthy impl** | Resolution | Capability exists but all providers are down. |
| **Escalation** | Reactive enforcement | Request suspended pending human approval. |

### Object-Capability Isomorphism

Capability-led connectivity has an isomorphism with the object-capability model (ocap). In ocap, an entity can only access resources for which it holds an unforgeable capability token. Access control is intrinsic to the reference.

In capability-led connectivity, an agent can only reach capabilities in its surface. The surface IS the accessible universe — not because access is checked at a gateway, but because capability resolution IS addressing. You can't address what isn't in your surface.

WASI does the same thing: a WebAssembly module can only access filesystem paths, sockets, env vars explicitly granted as capabilities. The runtime IS the boundary. geist-edge IS the boundary. Same structure, different substrate.

## 6. Envoy Mapping

The processing pipeline maps directly to Envoy's architecture. This mapping validates the topology and provides a shared vocabulary with the broader proxy ecosystem.

| Envoy Concept | geist-edge Concept | Notes |
|---------------|-------------------|-------|
| Listener | CapabilityServer | Inbound edge. Accepts connections, runs downstream processing. |
| Downstream filters | Downstream processors | About the requester. Auth, policy, rate limiting. |
| Router filter | Capability Router | Boundary between downstream and upstream. Selects target. |
| Upstream filters | Upstream processors | About the target. Request adaptation, circuit breaking. |
| Cluster (CDS) | Capability | Logical grouping of endpoints providing the same function. |
| Endpoint (EDS) | CapabilityClient resolution | Physical address of a capability provider. |
| Filter chain | Sequence compositor | Linear processor pipeline with short-circuit semantics. |
| `ProcessingRequest`/`ProcessingResponse` | Same (ext_proc protos) | geist-edge uses the actual ext_proc types, not hand-rolled equivalents. |
| `TypedExtensionConfig` (ECDS) | `TypedConfig` | Processor config distribution. Type URL → factory. |
| `FactoryRegistry` | `ProcessorRegistry` | Maps type URL → processor factory. Immutable after build. |
| Filter factory | `IntoProcessor` | Associated Config type + `from_config()`. |

### Key Insight

Envoy separates downstream filters (about the requester) from upstream filters (about the target), with the Router as the boundary. geist-edge follows the same architecture: downstream processors run before routing, upstream processors run after. The router is where "who is asking" transitions to "what are they asking for."

## 7. Decision Framework

When making architectural decisions about geist-edge, use these questions:

| Question | Answer | Examples |
|----------|--------|----------|
| **Is it about WHAT should happen?** | **Policy.** Deny/allow/escalate rules. User-defined configuration. | AccessControlPolicy, RateLimitPolicy, temporal rules |
| **Is it about HOW it happens?** | **Processor.** Implements the enforcement. Compiles policy → runtime logic. | AccessControlProcessor, RateLimiterProcessor, AuthProcessor |
| **Is it about WHERE it runs?** | **Adapter.** Translates runtime types ↔ processing contract. | axum adapter, pingora adapter, ext_proc adapter |
| **Is it about WHAT can be reached?** | **Capability.** Registered, discovered, invoked via logical ID. | file.read, payments.charge, agent.deploy |
| **Is it about WHO can reach it?** | **Capability surface.** Computed from policy + registration + health. | Per-agent tool list, filtered API catalog |
| **Is it about WHEN it can be done?** | **Temporal policy.** Session-aware, stateful, cross-invocation. | "max 5 file writes per session", "test before deploy" |

### Separation Diagnostic

If a concern touches multiple rows, it may need decomposition:

- "Rate limiting that depends on the caller's identity" → **Policy** (rules) + **Processor** (enforcement). Not one blob.
- "Auth that varies by deployment" → **Processor** (logic) + **Adapter** (runtime integration). Not a processor that knows about axum.
- "Deny Bash access for this agent" → **Policy** (AccessControlPolicy deny rule) applied by **Processor** (AccessControlProcessor) scoping the **Capability surface** (agent can't see Bash tool).

### The Separations Stack

Each separation creates a different kind of freedom:

| Separation | What vs What | Freedom Created |
|------------|-------------|-----------------|
| Policy / Processor | What should happen / How it happens | Change rules without touching code. Change implementation without touching config. |
| Adapter / Processor | Runtime specifics / Processing logic | Port to new runtime without touching processors. |
| Capability / Endpoint | Logical function / Physical address | Decompose services without breaking clients. |
| Surface / Enforcement | Proactive scoping / Reactive evaluation | Shape what agents can see separately from what they can do. |

Stacked together, these multiply into composability — not designed in, but falling out of correctly-placed separations.

## Sources

| Source | What It Contributed |
|--------|-------------------|
| `scratch/kavi-capability-led-connectivity.md` | Connectivity model definition, three shifts, naming arc, affordances, protocol mechanics, ocap isomorphism |
| `scratch/kavi-capability-led-connectivity-2.md` | Proactive/reactive enforcement distinction, capability surface construction, absence vs denial |
| `geist/edge/src/processor.rs` | Processor trait, BoxFuture, ProcessorError |
| `geist/edge/src/phase.rs` | PhaseResult, ProcessingMode |
| `geist/edge/src/compositor/sequence.rs` | Sequence, SequenceOutcome, FailureMode, SequenceBuilder |
| `.claude/docs/typed-extension-registry.md` | IntoProcessor, ProcessorRegistry, TypedConfig, Envoy/rumi mapping |
| Envoy codebase (conversation) | Upstream/downstream filter separation, Router as boundary, codec→filter chain→router→upstream |
