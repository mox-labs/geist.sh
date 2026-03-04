# Synthesis: The Governed Composition Runtime

> **Scope**: This document integrates and supersedes `geist-research-synthesis.md` (claim verification), `synthesis-agentic-data-plane.md` (landscape analysis), and `synthesis-geist-adr.md` (architectural decisions). Those documents remain in `rnd/` as detailed references. For SLICK manifest-to-enforcement pipeline detail not covered here, see `geist-research-synthesis.md` section "SLICK Integration."

> AI agents need more than protocols — they need a runtime that makes composition safe. geist-edge applies patterns proven in production data plane engineering to the agentic domain: portable processors, typed extension registries, capability-led connectivity, and deny-first policy evaluation. The thesis: **constrained, contract-enforced composition is strictly superior to unconstrained generation for agentic workloads**. Data plane architecture — refined through years of building production gateways, API management platforms, and service frameworks — provides the structural model for enforcing this.

---

## 1. The Problem Space

### 1.1 Agents Fail at Composition, Not Capability

The gap in AI agent infrastructure is not intelligence — it is governance. Individual LLM capabilities continue to improve rapidly. What does not improve automatically is the reliability of multi-step, multi-tool agent workflows where the LLM must select, compose, and coordinate capabilities under real-world constraints.

Three bodies of evidence define this problem:

**Error amplification in multi-agent systems.** Kim et al. (Google/MIT) tested 180 agent configurations across four benchmarks using mixed-effects regression (cross-validated R² = 0.524). Unstructured multi-agent topologies amplify errors 17.2x. Centralized coordination contains amplification to 4.4x. Sequential reasoning degrades 39-70% under any multi-agent variant. When single-agent baseline accuracy exceeds approximately 45%, adding agents yields diminishing or negative returns (capability saturation coefficient beta = -0.404, p < 0.001). **[Strong — arXiv:2512.08296]**

**Variance in constraint adherence.** Multiple independent studies converge on the same finding: LLMs are unreliable at following specific constraints during generation.

| Study | Finding | Evidence |
|-------|---------|----------|
| CIFE | 39-66% strict constraint adherence across 14 models, 1,000 tasks | **[Strong — arXiv:2512.17387]** |
| IFEval++ | Up to 61.8% performance drop from synonym rephrasing of identical prompts | **[Strong — arXiv:2512.14754]** |
| CtxBugGen | 55.93% Pass@1 for context-sensitive tasks; nearly half of generated code has subtle semantic defects | **[Strong — arXiv:2601.06497]** |
| Anka DSL | Constraining to 18 DSL operations raises accuracy from 60% to 95.8% overall (100% on multi-step pipeline tasks) | **[Strong — arXiv:2512.23214]** |

The mechanism is information-theoretic: constraining the generation space — replacing "generate arbitrary code" with "select from verified components" — reduces the entropy of the decision. This holds at every layer: DSL vocabulary constrains language generation, behavioral contracts constrain component selection, capability scoping constrains resource access.

**Prompt injection as fundamental threat.** CaMeL (Debenedetti et al.) demonstrates that prompt injection is not an edge case but a fundamental vulnerability requiring architectural defense. Their information flow control approach — treating security as a system property, not a model property — solves 77% of AgentDojo tasks with provable security guarantees. The insight: security must be enforced by the runtime, not the LLM. **[Strong — arXiv:2503.18813]**

### 1.2 The Bolted-On Security Anti-Pattern

The dominant infrastructure pattern for AI agents is **bolted-on security over general-purpose compute**: container isolation, then gVisor for defense-in-depth, then seccomp for syscall filtering, then OPA for admission control, then runtime enforcement via Tetragon. Each layer compensates for gaps left by the one below it.

This is the same anti-pattern observed in production data plane engineering: gateway proliferation where multiple independent systems each implement the same cross-cutting concerns (authentication, rate limiting, observability, circuit breaking) independently. Prior production experience at scale (9B+ requests/day, 75,000+ filter instances across multiple gateway technologies) demonstrated that this fragmentation produces:

- Inconsistent policy enforcement across boundaries
- N implementations of the same capability (measured: 4+ engineer-years of duplicated work)
- Coordination overhead consuming 25%+ of platform team time
- Configuration drift that increases compliance risk

**[Validated — prior production experience, not publicly reproducible across 15+ gateway deployments]**

The resolution in both domains is the same: portable processors, typed configuration, and a unified processing model that separates *what* should happen (policy) from *how* it happens (processor implementation).

### 1.3 What Exists vs. What Is Missing

| Capability | Exists? | Where? | Gap for agents |
|-----------|---------|--------|----------------|
| Sub-millisecond isolation | Yes | Fermyon Spin (0.52ms), Hyperlight (1-2ms) | Not purpose-built for agent lifecycle |
| Agent-to-tool protocol | Yes | MCP (97M SDK downloads, Linux Foundation) | No behavioral contract enforcement |
| Agent-to-agent protocol | Yes | A2A (150+ orgs, Linux Foundation) | Deliberately opaque — no shared state |
| Kernel-level enforcement | Yes | BPF-LSM, Tetragon, Aya-rs | Not contract-aware |
| Durable agent state | Yes | Durable Objects, Restate, Temporal | Not integrated with capability-scoped isolation |
| Typed component interfaces | Yes | WIT, Component Model | No semantic layer for cognitive operations |
| Assembled-per-intent runtime | No | — | **Confirmed gap across all sources** |
| Behavioral contracts for AI components | No | — | **No production implementation** |

**[Strong — exhaustive search across all sources confirms gap]**

---

## 2. The Architecture

### 2.1 Capability-Led Connectivity

geist-edge follows the **capability-led connectivity** model: portable processing logic via a unified contract, with adapters for different runtimes. This model was developed and validated through production data plane engineering before being applied to the agentic domain.

The core insight from production experience: wrap entire plugin pipelines with a single protocol adapter rather than rewriting plugins. Translation happens at the pipeline boundary, not per-plugin.

**Production validation**: 59 production plugins migrated to a new runtime in 6-8 weeks with 2-3 engineers, versus an estimated 12-18 month rewrite with 10-15 engineers. Zero plugins rewritten. **[Validated — prior production experience, not publicly reproducible]**

In geist-edge, this pattern manifests as:

```
Tauri (geist-shell)
  └─ geist-edge (axum adapter)
      ├─ ProtocolServer: inbound → processors → agent
      ├─ ProtocolClient: agent → processors → external
      ├─ Processors: registered via typed extension registry
      └─ Metadata: type-safe inter-processor communication
          └─ Agent runtime — all calls through edge
```

**Agent-edge interaction**: When the Claude SDK agent makes a tool call, the request flows through the ProtocolServer's processor pipeline before reaching the external tool. For example: the agent requests a file write → the access control processor receives the `ProcessingRequest`, checks the operation against configured policy (deny-first), and either permits it (returning `Continue`) or denies it (returning `ImmediateResponse` with a denial). The agent receives back either the tool result or the denial — it never bypasses the pipeline. Outbound calls from the agent to external services flow through ProtocolClient's pipeline symmetrically.

The `ProcessingRequest`/`ProcessingResponse` types come from the ext_proc protocol (Envoy external processing), not hand-rolled. This means the same processor implementation can run:

- Inline via an axum adapter (current)
- As an ext_proc server behind Envoy
- Via a pingora adapter
- Embedded in a service framework

**[Validated — prior production experience, not publicly reproducible]**

### 2.2 Extension Protocol Adapter Pattern

The Extension Protocol Adapter is the pattern that makes runtime portability practical. It originated in prior data plane engineering and applies directly to geist-edge.

**Traditional migration approach** (per-plugin adaptation):
```
Runtime → Plugin 1 Adapter [translate] → Process → [translate back]
         Plugin 2 Adapter [translate] → Process → [translate back]
         ...
         Plugin N Adapter [translate] → Process → [translate back]
```

**Extension Protocol Adapter** (pipeline-level adaptation):
```
Runtime → [TRANSLATE ONCE at pipeline boundary]
            → All N processors run natively
          [TRANSLATE ONCE back]
```

Single translation boundary at the pipeline level, not per-processor.

**Benefits for geist-edge:**
- Processors written once, run across axum, pingora, ext_proc
- Clean separation: extension logic (Rust processors) vs. runtime (adapter)
- Incremental optimization: migrate individual processors to native runtime as needed
- Future runtime swaps without touching processor code

**[Validated — prior production experience, not publicly reproducible]**

### 2.3 Typed Extension Registry

Processors register via type URL and factory function (`IntoProcessor` trait with associated `Config` type). Pipeline configuration is a list of `TypedConfig { type_url, config }` entries. The registry is immutable after build (builder pattern to frozen registry).

This follows the same pattern as Envoy's `FactoryRegistry` — processors are the unit of extension, type URLs are the unit of identity, and the registry is the composition root.

Key properties:
- **Each processor owns its policy type** — no generic PolicyEvaluator
- **Self-registration** via `inventory::submit!` — extension crates register themselves, zero code changes to core or binary
- **Policy-processor separation** — each processor compiles its policy/config to matchers at construction time

**[Strong — Envoy FactoryRegistry pattern (production at every major cloud provider); validated in prior production data plane work]**

### 2.4 The 4D Architecture Framework

The system maps to four operational layers:

| Layer | Role | geist-edge Mapping |
|-------|------|--------------------|
| **Decision** | Policy definition and control logic | Typed extension registry + policy compilation |
| **Dissemination** | Configuration distribution | ECDS (xDS) for dynamic processor config updates |
| **Discovery** | Service and topology discovery | Capability IDs, registry lookup |
| **Data** | Policy enforcement (request processing) | Processor pipeline |

Production experience showed that fragmentation occurs when each component builds its own complete 4D stack. The solution is shared infrastructure for dissemination and discovery, with specialized data planes preserving domain-specific excellence.

**[Validated — framework developed from prior production experience, not publicly reproducible]**

### 2.5 ACES as System Quality

ACES (Adaptable, Composable, Extensible Software) is not a specific pattern but a system quality achieved through implementation choices:

- **Adaptable**: Swap adapter (axum → pingora → ext_proc) without touching processors. Same processor implementation works across deployment contexts.
- **Composable**: Processors compose through pipeline + metadata, not direct coupling. Capabilities compose via control plane. Configuration cascades (platform defaults → workspace → service-specific).
- **Extensible**: Add processor = register extension + config entry. `inventory::submit!` enables zero-code-change registration. No core modifications needed.

The core thesis of ACES: **best-in-class is a moving target. Systems should be designed to adapt to excellence, not locked into today's implementation of it.**

Production evidence across four applications of ACES:

| Application | Domain | ACES Expression | Outcome |
|-------------|--------|-----------------|---------|
| API Management Platform | Multi-tenant gateway | Processor portability, policy-processor separation | $700K savings, 60% latency reduction |
| Unified Edge Platform | Gateway consolidation (15+ → unified) | Extension Protocol Adapter, capability abstraction | 59 plugins on new runtime in 6-8 weeks |
| Code Intelligence Engine | Agent-based analysis | Deterministic foundation + agent reasoning | 67% accuracy improvement |
| Experimentation Framework | QoS experiments | Composable components, adaptable runtime | Domain-agnostic DAG orchestration |

**[Validated — measured across four prior production applications, not publicly reproducible]**

### 2.6 Deny-First Policy Evaluation

All policy evaluation in geist-edge uses deny-first semantics. This is not a design preference — it is consensus across Cedar (AWS), Security Content Automation Protocol (NIST), and OWASP.

In the context of agent governance: an agent has zero capabilities by default. Capabilities are explicitly granted through processor configuration. A processor that is not registered cannot be invoked. A capability that is not configured does not exist in the agent's action space.

This aligns with the capability security lineage: Dennis & Van Horn (1966, MIT) through Hardy's Confused Deputy (1988) to Miller's object-capability model (2006, Johns Hopkins). The formal properties:

- **No ambient authority** — capabilities must be explicitly provided
- **Attenuation without amplification** — rights flow downward only
- **POLA by construction** — new subjects start with only the capabilities explicitly granted

**[Strong — foundational CS research with machine-checked proofs (seL4)]**

---

## 3. Prior Art & Patterns

These six patterns were selected from a broader data plane pattern catalog based on one criterion: **they directly shaped geist-edge's design**. Other patterns (circuit breaker, sidecar, gateway aggregation, service mesh) are well-documented elsewhere and are not geist-edge-specific. The patterns below are the ones where production experience with data plane engineering directly informed the Processor trait, the typed extension registry, and the capability-led connectivity model.

### 3.1 Unified Processing Model

**Pattern**: Single processor implementation runs across all platforms via adapters. Write once, deploy everywhere. Same testing, same behavior.

**Production origin**: In a production environment with heterogeneous gateway platforms (JVM-based edge proxy, Spring Cloud Gateway, Envoy), each platform had its own plugin model. Common capabilities (authentication, rate limiting, observability) were implemented 3-6 times across the data plane.

The Unified Processing Model solves this through a ProcessingAdaptor — one adapter per platform, N portable processors.

| Before | After |
|--------|-------|
| Auth in edge gateway (Java, proprietary API) | Auth processor (portable) |
| Auth in API gateway (Java, Spring model) | Same auth processor |
| Auth in service framework (Java, gRPC model) | Same auth processor |
| N implementations | 1 implementation |

**Application to geist-edge**: The `Processor` trait is the unified processing model for agent governance. An access control processor, a rate limiting processor, an observability processor — each is written once and runs across all deployment contexts (axum, ext_proc, pingora).

**[Validated — prior production experience, not publicly reproducible]**

### 3.2 Configuration Hierarchy (Progressive Disclosure)

**Pattern**: CSS-like specificity for configuration. Users get working defaults immediately. Advanced users customize only specific layers.

```
Platform Defaults (built-in, zero-config)
  → Workspace Defaults (override only what differs)
    → Service-Specific Configuration (final overrides)
      → Resolved Configuration
```

**Production origin**: API management platform serving 250K TPS. Zero-config path: define an API contract, point it at a backend, publish through a gateway — three resources, platform defaults for auth, rate limiting, and observability. Teams that need custom behavior override only what they need.

**Key design insight**: Convention over configuration, not configuration over convention. The first experience must be minimal. Complexity is revealed progressively, not imposed upfront.

**Application to geist-edge**: Processor pipeline configuration follows the same cascade. Default processors provide baseline governance. Workspace configuration overrides specific policies. Per-session configuration refines further.

**[Validated — prior production experience, not publicly reproducible]**

### 3.3 APIServer/APIClient (Logical Capability IDs)

**Pattern**: Services depend on logical capability IDs, not physical endpoints. Control plane resolves capabilities to implementations. Service decomposition does not break clients.

```
Before decomposition:
  "payments" capability → payments-service:8080

After decomposition:
  "payments" capability → [payment-auth, payment-processing, payment-settlement]

Clients: unchanged. Not a line of code.
```

**Production origin**: Inspired by proxyless gRPC (xDS-based). Designed and prototyped for a production environment with 1,000+ services. The capability abstraction layer eliminates infrastructure coupling — clients depend on *what* a service does, not *where* it lives.

**Validation**: A production consolidation collapsed five self-managed gateways into one multi-tenant platform. 80+ client applications. Zero configuration changes. Clients never noticed. **[Validated — prior production experience, not publicly reproducible]**

**Application to geist-edge**: Agent capabilities are logical, not physical. The composer (P3) maps intent to capability selection from the extension catalog. Infrastructure changes (scaling, redeployment, decomposition) are invisible to the agent session.

### 3.4 Three-Layer Composable Architecture

**Pattern**: Decompose the data plane into three independently swappable layers:

| Layer | Role | Change Frequency | geist-edge Mapping |
|-------|------|------------------|--------------------|
| **Kernel** | Packet filtering, DDoS, basic routing | Years | OS-level enforcement (eBPF on Linux, seatbelt on macOS) |
| **Proxy** | Connection management, protocol handling | Months | Runtime adapter (axum, pingora, ext_proc) |
| **Extensions** | Business logic, policies, governance | Days-Weeks | Processor pipeline |

Each layer can evolve independently. Swapping the proxy runtime does not require rewriting extensions. Changing an extension does not require proxy changes. This is what makes the ACES "continuously adopt excellence" thesis practical.

**Production evidence**: Benchmarks comparing runtimes showed 5x throughput, 5x lower latency, and 27x compute efficiency when moving from a JVM-based proxy to a native proxy — with zero extension rewrites via the Extension Protocol Adapter. **[Validated — prior production benchmarks, not publicly reproducible]**

### 3.5 First Principles Constraints

Immutable constraints from physics, mathematics, and human cognition that shape architectural decisions:

| Domain | Constraint | Implication |
|--------|-----------|-------------|
| **Physics** | 9.8ms RTT per 1,000km (speed of light in fiber) | Geography is destiny for latency floors |
| **Mathematics** | N services → N(N-1)/2 potential connections | Governance becomes impossible without platform mediation at scale |
| **Mathematics** | 10 services at 99.9% = 99.0% system reliability | Every synchronous dependency reduces reliability multiplicatively |
| **Cognition** | 4 plus or minus 1 working memory chunks | Developers maintain deep understanding of 3-5 APIs simultaneously |
| **Cognition** | 23 minutes to regain focus after interruption | Minimizing context switches through integrated tooling improves productivity |

These justify the typed extension registry (reduces N-squared coordination), processor pipeline (minimizes hops), and unified processing model (respects cognitive limits by providing one pattern, not N platform-specific patterns).

**[Strong — grounded in foundational research: Miller 1956, Cowan 2001, Conway 1968, reliability engineering fundamentals]**

### 3.6 Policy-Processor Separation

**Pattern**: Decouple *what should happen* (policy/config) from *how it happens* (processor implementation). Tenant policies remain stable during processor upgrades. Platform evolves implementations without tenant reconfiguration.

```
Tenant Policy (what) → Processor Implementation (how)
  Rate Limit: 1000 req/min  → Redis Rate Limiter (can swap to local counter)
  Auth: OAuth2 + JWKS       → OAuth Validator (can swap implementation)
```

**Production origin**: API management platform where upgrading a rate limiter's backing store broke every tenant's configuration. The separation created three degrees of freedom: implementation (swap processors), extensibility (custom policy types in tenant-scoped namespaces), and portability (same policies across platforms).

**Application to geist-edge**: Each processor owns its config type. The typed extension registry maps type URLs to factories. No generic PolicyEvaluator — processors compile their policy to matchers at construction time.

**[Validated — prior production experience, not publicly reproducible]**

---

## 4. The Evidence Base

### 4.1 Runtime Isolation Benchmarks

| Runtime | Cold Start | Memory Overhead | Isolation Model | Evidence Level |
|---------|-----------|----------------|-----------------|---------------|
| Fermyon Spin | **0.52ms** | ~10-15 MB | WASI capability scope | **Verified** — GlobeNewsWire, 75M RPS on Akamai |
| Hyperlight (Wasm+KVM) | **1-2ms** | Minimal | Double-layer: Wasm + KVM | **Verified** — CNCF Sandbox |
| gVisor | 50-100ms | Low | User-space kernel | **Plausible** — consistent with architecture |
| Firecracker | 100-125ms | <5 MiB VMM | KVM hardware virtualization | **Verified** — AWS Lambda specification |
| Kata Containers | 150-300ms | 128-256 MiB | KVM via VMM | **Verified** — OpenStack/K8s |

**Correction**: The term "microsecond-level cold starts" appeared in early research and is misleading. Standalone Wasmtime module instantiation can be microseconds; production cold starts (Spin) are 0.52ms. The 1000x advantage over microVMs holds for Spin-vs-Kata but not for standalone benchmarks. **[Strong — corrected against primary sources]**

### 4.2 Ecosystem Projects

| Project | Status | Evidence |
|---------|--------|----------|
| wasmCloud | CNCF Incubating, wRPC/NATS lattice | **Verified** — moved to Incubating Nov 2024, active releases |
| SpinKube | Kubernetes-native Wasm, containerd-shim-spin | **Verified** — v0.22.0, Azure AKS integration |
| Wassette | Wasm-to-MCP bridge (Microsoft) | **Verified** — Azure Core team, Aug 2025. Not production-ready (maintainers' own assessment) |
| WASI 0.3 | Async primitives (stream, future) | **Verified** — previews in Wasmtime 37+, expected early 2026 |
| componentize-py | Python-to-Wasm Components | **Verified** — v0.19.3. Limitations: no threading, partial C extensions, no sys.exit |

### 4.3 eBPF/Security Stack

| Component | Status | Evidence |
|-----------|--------|----------|
| Aya-rs | 22 eBPF program types, pure Rust, BTF/CO-RE | **Verified** — production at Deepfence, Red Hat (bpfman), Exein (Pulsar) |
| Tetragon | TracingPolicy CRDs, v1.6.0 | **Verified** — CNCF project under Cilium |
| eBPFGuard | Aya-based LSM policy enforcement | **Verified** — Deepfence, Rust/YAML policies |
| AgentSight | Dual-probe boundary tracing, <3% overhead | **Verified** — arXiv:2508.02736, ACM published |
| BPF-LSM superiority over seccomp-BPF | Deep kernel state inspection vs. register-only | **Verified** — kernel documentation |

### 4.4 Variance Reduction Papers

| Paper | Finding | Evidence |
|-------|---------|----------|
| Anka DSL (arXiv:2512.23214) | 18-operation constrained vocabulary raises accuracy from 60% to 95.8% | **Verified** — 100% on multi-step pipeline tasks; 95.8% overall |
| CIFE (arXiv:2512.17387) | 39-66% strict constraint adherence across 14 models, 1,000 tasks, 13 constraint categories | **Verified** |
| IFEval++ (arXiv:2512.14754) | Up to 61.8% performance drop from synonym rephrasing across 46 LLMs | **Verified** |
| CtxBugGen (arXiv:2601.06497) | 55.93% Pass@1 for context-sensitive tasks; 3,683 bugs | **Verified** |
| bMAS (arXiv:2507.01701) | Blackboard MAS competitive with SOTA using fewer tokens via self-selection | **Verified** |
| Kim et al. (arXiv:2512.08296) | 180 configurations, 17.2x error amplification in unstructured topologies | **Verified** |

### 4.5 Formal Verification & Security Research

| Paper | Contribution | Evidence |
|-------|-------------|----------|
| CaMeL (arXiv:2503.18813) | Information flow control defense against prompt injection; 77% of AgentDojo tasks with provable security | **Verified** |
| Allegrini et al. (arXiv:2510.14133) | Formal verification framework for multi-agent systems; state machine models for MCP/A2A protocols | **Verified** |
| Dao et al. (arXiv:2601.19752) | 12 agentic design patterns from systems theory; 5 functional subsystems | **Verified** — NeurIPS 2025 LAW workshop |
| Miehling et al. (arXiv:2503.00237) | Position: agentic AI needs systems theory; emergent capabilities from interaction | **Verified** — IBM Research |

### 4.6 Corrections

These corrections matter because they change architectural decisions or calibrate confidence levels.

| Claim | Source | Correction |
|-------|--------|-----------|
| CVE-2023-41880 as Wasm sandbox escape | Research report | NVD explicitly states "not an escape from the WebAssembly sandbox." It is a JIT miscompilation producing incorrect results *within* the sandbox. |
| "Microsecond-level cold starts" for production agents | Research report | Report's own data shows 16.9ms. Spin achieves 0.52ms. Production floor is sub-millisecond, not sub-microsecond. |
| "Universal Microservices Architecture" (UMA) as standard | Research report | One researcher's blog framework (Medium). Not standardized by Bytecode Alliance, W3C, or CNCF. Describes a real trajectory but is not itself a standard. |
| "Complete abandonment of containers" | Research report | Contradicted by the same report's own roadmap (Kata fallback), by SpinKube (runs Wasm within Kubernetes), and by the hybrid trajectory the industry is actually on. |
| Anka DSL "60% to 100%" | Research report | 95.8% overall accuracy; 100% specifically on multi-step pipeline tasks. The general claim that constrained vocabularies improve accuracy is strongly supported. The specific "100%" needs qualification. |
| MCP "three major version changes in six months" | Research report | MCP has been relatively stable since Anthropic's late 2024 release. Claim appears exaggerated. |

### 4.7 Unverified Claims

| Claim | Status |
|-------|--------|
| TOOLQP semantic gap paper | No arXiv paper found. Concept exists under other names (GRETEL arXiv:2510.17843, TOOLCERT). |
| "Rax" resource-aware agents | No arXiv paper found |
| Seccomp-eBPF "33-55% attack surface reduction" | Specific paper not located |

---

## 5. Architectural Decisions

### ADR-001: Rust + eBPF over WASM for MVP

**Decision**: Build the runtime in Rust. Use eBPF (Aya-rs) for kernel-level observability and enforcement on Linux. Do not depend on WASM or the Component Model for MVP.

**Evidence For**:

| Factor | Evidence | Level |
|--------|----------|-------|
| No WASI 0.3 dependency risk | WASI 0.3 "expected" not shipped; building MVP on expected standards is dependency risk | **Strong** |
| componentize-py limitations | No threading, partial C extensions, no sys.exit, no production evidence for agent workloads | **Strong** |
| eBPF works regardless of runtime | uprobes/kprobes attach to processes, not to Wasm runtimes specifically | **Strong** |
| Aya-rs production-ready | 22 program types, pure Rust, BTF/CO-RE; production at Deepfence, Red Hat, Exein | **Strong** |

**Evidence Against**:

| Factor | Evidence | Level |
|--------|----------|-------|
| No compile-time capability enforcement | WASM's strongest security property — architectural absence makes unauthorized operations inexpressible | **Strong** |
| Process-level isolation is weaker | Requires eBPF/seccomp to compensate for lack of Wasm SFI | **Strong** |
| Misses wasmCloud distributed composition | wRPC/NATS lattice is the closest to constitutive composition | **Moderate** |

**Migration path**: Design WIT-compatible interfaces from day one. When WASI 0.3 ships and componentize-py matures, evaluate Wasm Components as a guest format within the Rust runtime. The typed extension registry's interface definitions should map cleanly to Wasm Component interfaces.

**Risk**: WASM matures faster than expected, making the Rust+eBPF choice lose its primary advantage while missing WASM's strongest property.

**Confidence: 80%** — eliminates dependency risk at the cost of missing compile-time capability enforcement.

### ADR-002: Behavioral Contracts as Runtime Concern

**Decision**: Behavioral contract concepts (Design-by-Contract for AI components) live in the runtime, not in a description format. Contracts are enforced by geist-edge processors, not by the agent protocol.

**Rationale**:

- MCP is a wire protocol. Executable contracts are a runtime concern. Mixing violates Interface Segregation.
- Every previous reusable-component initiative that required manual interface description died of friction before automation tooling arrived (CORBA, SOA/WSDL, EJB 2.x). **[Strong — historical analysis]**
- Key difference this time: the consumer is an LLM, not a human. LLMs derive more value from structured descriptions. But only if descriptions are generated from code, not hand-authored. **[Moderate]**

**Mechanism**: Contracts generate processor configurations. A `safety_level: read-only` annotation compiles to a processor that blocks write operations. A postcondition compiles to a validation processor. The manifest is a build artifact, not a source artifact.

**Confidence: 75%** — sound design, but contract shapes may need revision once real agents exercise them.

### ADR-003: eBPF with Darwin Fallback

**Decision**: Two-tool eBPF strategy on Linux (Cilium/Tetragon for standard security, Aya-rs for novel contract-aware enforcement). Trait-based userspace fallback on macOS for development.

**Architecture**:

| Concern | Tool | Status |
|---------|------|--------|
| Standard observability | Cilium/Hubble | **Production-ready** — CNCF Graduated |
| Standard runtime security | Tetragon TracingPolicies | **Production-ready** — YAML CRDs, in-kernel enforcement |
| Contract-aware enforcement | Aya-rs custom programs | **R&D required** — parse contracts, generate eBPF maps |
| Intent-action correlation | Aya-rs uprobes + kprobes | **R&D required** — correlation engine is the hard part |

**macOS development fallback**:

```rust
trait PolicyEngine {
    fn enforce_file_access(&self, path: &Path, mode: AccessMode) -> Result<(), PolicyDenied>;
    fn enforce_network(&self, addr: &SocketAddr) -> Result<(), PolicyDenied>;
    fn enforce_process(&self, cmd: &str) -> Result<(), PolicyDenied>;
}
```

Identical API, weaker enforcement (application-level, not kernel-level). Sufficient for development; production requires Linux for full eBPF enforcement.

**Phasing**: Phase 1 observability (Cilium/Hubble + basic Tetragon — see what agents actually do before writing enforcement rules), Phase 2 enforcement (Aya-rs custom programs — requires understanding real agent behavior patterns).

**Novel component assessment**:

| Component | Confidence |
|-----------|-----------|
| Cilium/Tetragon baseline | **90%** |
| Aya-rs custom enforcement | **70%** |
| Contract-to-BPF-LSM translation | **50%** — architecturally sound, no implementation exists |
| Intent-action correlation | **40%** — concept proven (AgentSight), production viability unknown |

---

## 6. Open Questions & Gaps

### Architecture

1. **Internal data flow model.** How does data move within a geist instance between the LLM loop, state store, tool execution, and protocol adapters? The processing pipeline model handles request/response flow but does not specify internal state management.

2. **Multi-tenancy.** No treatment of tenant isolation, resource quotas, or billing boundaries for the managed platform (gestalt). Prior production experience with platform multi-tenancy (teams-as-tenants configuring isolated processing pipelines on shared infrastructure) provides the pattern, but it has not been designed for geist-edge.

3. **State management.** What is ephemeral vs. persistent? What survives restarts? The hybrid pattern (physically ephemeral, logically persistent) from Durable Objects and Restate provides a model, but the specific state architecture for agent sessions is unspecified.

### Security

4. **Contract-to-LSM translation gap.** The path from behavioral contract to eBPF LSM program has known hard problems: path canonicalization (symlinks, `..` resolution), inode-to-path mapping (expensive, racy), policy compilation per manifest. This is security-critical custom infrastructure. **[Moderate — proven primitives, novel integration]**

5. **Intent-action correlation algorithm.** How does the correlation engine match LLM intent to kernel-level action across async operations where temporal separation may be large? What is the false positive rate when legitimate operations match suspicious patterns? No data exists. **[Weak — concept proven, production viability unknown]**

6. **macOS development completeness.** eBPF is Linux-only. The `PolicyEngine` trait abstraction provides identical API with weaker enforcement. Is application-level enforcement sufficient for the development workflow? Prior production experience with macOS seatbelt profiles (sandbox-exec) provides partial coverage.

### Ecosystem

7. **Protocol convergence or fragmentation.** The Agentic AI Foundation (Linux Foundation, Dec 2025) suggests convergence for MCP and A2A. Hexagonal architecture hedges either way. But adapter proliferation has real maintenance cost.

8. **WASM timing.** When does componentize-py become viable for agent workloads? When does WASI 0.3 ship? The Phase 2 evaluation (ADR-001) depends on these external timelines.

9. **Bootstrapping.** The first practical test of behavioral contracts requires real agent workflows exercising them. Strategy: start with the access control processor (P1.5, done), then add processors for common governance patterns (rate limiting, content filtering, tool access control), then validate that constrained workflows outperform unconstrained baselines.

### Scaling

10. **Cost model.** No analysis of memory overhead per agent instance, compute cost at scale, or comparison with container-based approaches at equivalent workload. Production data plane experience suggests that specialized runtimes (Rust-based, purpose-built) achieve 5-27x compute efficiency over general-purpose runtimes — but this has not been measured for agent workloads specifically.

---

## References

### Academic Papers

| ID | Authors | Title |
|----|---------|-------|
| arXiv:2512.08296 | Kim et al. | Towards a Science of Scaling Agent Systems |
| arXiv:2512.17387 | CIFE authors | Code Instruction-Following Evaluation |
| arXiv:2512.14754 | IFEval++ authors | IFEval++: Instruction Following Evaluation with Synonym Paraphrases |
| arXiv:2512.23214 | Anka authors | Anka: Domain-Specific Language for Reliable Agent Pipelines |
| arXiv:2601.06497 | CtxBugGen authors | CtxBugGen: Context-Sensitive Bug Generation |
| arXiv:2507.01701 | bMAS authors | Blackboard-based Multi-Agent Systems |
| arXiv:2508.02736 | AgentSight authors | AgentSight: Dual-Probe Boundary Tracing |
| arXiv:2503.18813 | Debenedetti et al. | CaMeL: Defeating Prompt Injections by Design |
| arXiv:2510.14133 | Allegrini et al. | Formalizing Safety Properties of Agentic AI Systems |
| arXiv:2601.19752 | Dao et al. | Agentic Design Patterns: A System-Theoretic Framework |
| arXiv:2503.00237 | Miehling et al. | Agentic AI Needs a Systems Theory |
| arXiv:2505.01603 | Dandelion authors | Dandelion: Distributed Runtime (SOSP '25) |

### Foundational Research

- Dennis, J.B. & Van Horn, E.C. "Programming Semantics for Multiprogrammed Computations." CACM 9(3), March 1966.
- Hardy, N. "The Confused Deputy." ACM SIGOPS 22(4), 1988.
- Miller, M.S. "Robust Composition: Towards a Unified Approach to Access Control and Concurrency Control." PhD thesis, Johns Hopkins, 2006.
- Klein, G. et al. "seL4: Formal Verification of an OS Kernel." ACM SOSP 2009 (Best Paper).
- Conway, M.E. "How Do Committees Invent?" Datamation 14(4), 1968.
- Cowan, N. "The Magical Number 4 in Short-Term Memory." Behavioral and Brain Sciences 24(1), 2001.
- Parnas, D.L. "On the Criteria to Be Used in Decomposing Systems into Modules." CACM 15(12), 1972.
- Wiener, N. *Cybernetics: Or Control and Communication in the Animal and the Machine*. 1948.

### Projects & Specifications

- Envoy proxy: envoyproxy.io (ext_proc protocol, xDS APIs)
- wasmCloud: wasmcloud.com (CNCF Incubating, wRPC/NATS lattice)
- Fermyon Spin: fermyon.com (0.52ms cold starts, 75M RPS)
- Hyperlight: github.com/hyperlight-dev/hyperlight (CNCF Sandbox, double-layer isolation)
- SpinKube: spinkube.dev (Kubernetes-native Wasm)
- Aya-rs: aya-rs.dev (pure Rust eBPF)
- Tetragon: tetragon.io (CNCF, eBPF runtime security)
- MCP: modelcontextprotocol.io (97M monthly SDK downloads, Linux Foundation)
- A2A: a2a-protocol.org (150+ organizations, Linux Foundation)
- WASI: wasi.dev (0.2 stable, 0.3 in preview)
- Wassette: github.com/aspect-build/wassette (Wasm-to-MCP bridge)
- Cedar: github.com/cedar-policy (AWS, deny-first policy language)
