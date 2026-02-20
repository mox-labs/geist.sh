# The Agentic Data Plane: Architecture, Composition, and Security

> **Date**: February 13, 2026
> **Scope**: Technical landscape analysis for geist.sh — what exists, what's missing, and where geist.sh fits
> **Sources**: 10 documents (3 deep research reports, 2 compass artifacts, 1 prior synthesis, 1 guild verdict, 3 critique/working documents). Full source list in appendix.
> **Evidence policy**: Every claim labeled **Strong** / **Moderate** / **Weak** / **Speculative**. Unverifiable citations flagged. Numbers trace to specific papers.

---

## 1. The Problem: Agents Need a Runtime, Not Just a Protocol

AI agent deployments quadrupled from 11% to 42% of enterprises between Q2 and Q3 2025. Every major cloud provider ships agent isolation primitives — AWS Bedrock AgentCore, Google Agent Sandbox on GKE, Microsoft Agent 365. MCP has 97M monthly SDK downloads. A2A has 150+ supporting organizations.

Yet the dominant infrastructure pattern is **bolted-on security over general-purpose compute**: Firecracker microVMs for isolation, then gVisor for defense-in-depth, then seccomp for syscall filtering, then OPA for admission control, then Tetragon for runtime enforcement. Each layer compensates for gaps left by the one below it. The result is operationally complex and architecturally accidental.

Three problems remain unsolved:

1. **Safe composition.** Multi-agent systems amplify errors 17.2x in unstructured topologies (DeepMind, 180 configurations). No production system enforces coordination topology. **[Strong — Kim et al. 2025, arXiv:2512.08296]**

2. **Behavioral contract enforcement.** LLMs follow strict constraints only 39-66% of the time (CIFE, 14 models, 1,000 tasks). Constraining the vocabulary to 18 DSL operations raises accuracy from 60% to 95.8% (Anka). The mechanism works. No production runtime implements it for agent tool composition. **[Strong — arXiv:2512.17387, arXiv:2512.23214]**

3. **Intent-action observability.** When an agent takes an action, no production system correlates *what the LLM was asked to do* with *what the process actually did at the kernel level*. AgentSight (arXiv:2508.02736) demonstrates the approach academically. No one ships it. **[Moderate — research prototype, <3% overhead claimed but not validated at scale]**

These three gaps define the design space for geist.sh.

---

## 2. The Runtime Landscape

### 2.1 Isolation: WASM vs. Containers vs. Hybrid

The performance delta between WebAssembly and microVM isolation is architectural, not incremental.

| Runtime | Cold start | Memory overhead | Isolation model | Production evidence |
|---------|-----------|----------------|-----------------|---------------------|
| Wasmtime (standalone) | <0.03ms (instantiation) | Few KB per instance | Capability-based, linear memory | Bytecode Alliance LTS |
| Fermyon Spin | **0.52ms** | ~10-15 MB per binary | WASI capability scope | 75M RPS on Akamai **[Strong]** |
| Hyperlight (Wasm+KVM) | **1-2ms** | Minimal | Double-layer: Wasm + KVM | CNCF Sandbox **[Strong]** |
| gVisor | 50-100ms | Low | User-space kernel | GKE default **[Strong]** |
| Firecracker | 100-125ms | <5 MiB VMM | KVM hardware virtualization | AWS Lambda **[Strong]** |
| Kata Containers | 150-300ms | 128-256 MiB | KVM via configurable VMM | OpenStack/K8s **[Strong]** |

**Key correction**: Report 1 claimed "microsecond-level cold starts" while its own data showed 16.9ms. Standalone Wasmtime instantiation can be microseconds; production cold starts (Spin) are 0.52ms. The 1000x advantage over microVMs holds for Spin-vs-Kata but not for standalone benchmarks. **[Strong — Report 1's own data contradicts its headline]**

**The security trade-off is real.** MicroVMs provide hardware-enforced isolation via KVM — requiring both a VMM escape and a KVM bypass. Wasm isolation is enforced by the runtime's compiler and validator; an exploit requires finding a bug in Wasmtime itself. Hyperlight bridges this with double-layer isolation at 1-2ms. CVE-2023-41880, cited by Report 1 as a sandbox escape, is explicitly described by NVD as "not an escape from the WebAssembly sandbox" — it was a JIT miscompilation producing incorrect results within the sandbox. **[Strong — NVD entry]**

### 2.2 The Hybrid Runtime Pattern: Physically Ephemeral, Logically Persistent

Pure ephemeral runtimes (Lambda) fail for agents due to session statefulness and context reconstruction costs. Pure persistent runtimes create single points of failure. The industry converges on a hybrid.

| System | Pattern | Cold/Reactivation | State | Composition Model |
|--------|---------|-------------------|-------|-------------------|
| Cloudflare Durable Objects | Single-threaded actor + co-located SQLite | <50ms reactivation | 10 GB per DO, sub-ms reads | Static (Workers routing) |
| Restate Virtual Objects | Keyed entities, single-writer, durable execution | 15ms median (3-step) | Isolated K/V per object | Pre-defined workflows |
| Temporal | Workflow/activity model, durable execution | Higher per-step than Restate | 2 MB payload limit | Pre-defined activities |
| wasmCloud Lattice | Auction-based scheduling, wRPC over NATS | Microsecond component start | Distributed via NATS JetStream | **Closest to dynamic composition** |

**What "assembled per intent" would look like**: User intent arrives → intent analysis selects required capabilities → capabilities are composed into a session-scoped runtime → runtime executes with exactly the authority needed → runtime dismantled after task completes.

**No production system fully implements this.** wasmCloud's lattice comes closest — its auction-based scheduling broadcasts constraints, hosts respond with capability matches, and components launch dynamically via wRPC. But it lacks the intent analysis layer that maps user intent to component selection. This is the confirmed gap. **[Strong — verified across all sources; every system examined provides partial capabilities but none implements the complete pattern]**

### 2.3 Protocols: MCP + A2A as the Foundation

| Protocol | Role | Adoption | Maturity |
|----------|------|----------|----------|
| MCP | Agent-to-tool | 97M monthly SDK downloads, 10,000+ servers, Linux Foundation (AAIF) | De facto standard **[Strong]** |
| A2A | Agent-to-agent | 150+ organizations, v0.3 with gRPC + signed security cards | Emerging standard **[Strong]** |
| wRPC | Component-to-component | Bytecode Alliance, wasmCloud | Niche but technically strongest for typed composition **[Moderate]** |
| ANP | Semantic interop | Only protocol with JSON-LD / Schema.org vocabularies | Early **[Weak]** |

**MCP is instrumental, not constitutive.** MCP's architecture is inherently client-server with no server-to-server communication channel. Tools are instruments wielded by an orchestrator. Elicitation is a server→client→user→client→server round-trip for user input. Roots are advisory filesystem scope boundaries. Neither enables inter-tool state sharing. **[Strong — MCP spec]**

**A2A deliberately keeps agents opaque.** Agents collaborate "without needing to share their internal state, memory, or tools." This is by design — it enables heterogeneous agent composition but precludes constitutive integration. **[Strong — A2A spec]**

**wRPC is the constitutive composition primitive.** Wasm components subscribe on NATS subjects corresponding to their WIT-defined exports, and any component can invoke another's functions directly through typed interfaces. While the wasmCloud host mediates link establishment, individual function calls are peer-to-peer via wRPC over NATS. WIT provides rich structural types (records, variants, enums, resources with lifetimes), and wRPC is transport-agnostic (TCP, NATS, QUIC, UDP). This is the closest existing analog to constitutive composition. **[Strong — Bytecode Alliance project, production in wasmCloud]**

---

## 3. Multi-Agent Scaling Constraints

Google DeepMind's "Towards a Science of Scaling Agent Systems" (Kim et al., December 2025) tested 180 agent configurations across four benchmarks using mixed-effects regression (cross-validated R² = 0.513).

### The 45% Threshold

When single-agent baseline accuracy exceeds ~45%, adding agents yields diminishing or negative returns. The capability saturation coefficient was β = -0.408 (p < 0.001). The model correctly predicted optimal coordination strategy for 87% of held-out configurations.

**Caveats**: R² = 0.513 leaves ~49% of variance unexplained. Only four benchmarks tested. The threshold is empirical, not theoretically derived. As base models improve, fewer tasks will sit below this threshold. **[Moderate — significant study, but limited benchmark coverage]**

### Error Amplification by Topology

| Topology | Error amplification | Why |
|----------|-------------------|-----|
| Unstructured ("bag of agents") | **17.2x** | Errors multiply via unchecked propagation; contradictory outputs cascade rather than resolve |
| Centralized (hub-and-spoke) | **4.4x** | Orchestrator acts as circuit breaker, detects contradictions, requests re-execution |
| Sequential reasoning under any multi-agent approach | **39-70% degradation** | Multi-turn reasoning chains cannot be parallelized |

### Domain-Specific Results

| Task type | Best approach | Evidence |
|-----------|--------------|----------|
| Parallelizable (Finance-Agent) | Centralized MAS: +80.9% over single agent | **[Moderate]** |
| Dynamic search (BrowseComp-Plus) | Decentralized: +9.2% | **[Moderate]** |
| Sequential reasoning (PlanCraft) | Single agent — all multi-agent variants degraded 39-70% | **[Moderate]** |

**Design constraint for geist.sh**: The data plane must enforce explicit coordination topology. Default to hub-and-spoke. Keep sequential reasoning within single extensions. Each additional tool consumes "cognitive bandwidth" (token budget) — with 16+ tools, agents spend their budget on coordination rather than problem-solving. **[Moderate — extrapolated from DeepMind data to the geist.sh context]**

---

## 4. Capability-Based Security

### 4.1 The Formal Lineage: 60 Years of Capability Theory

The capability model — authority travels as an unforgeable token bundled with resource designation — traces from Dennis & Van Horn (1966, MIT) through Hardy's Confused Deputy (1988, Tymshare) to Miller's object-capability model (2006, Johns Hopkins). The formal properties are:

- **No ambient authority** — a subject must present a specific capability for any operation
- **Attenuation without amplification** — rights flow downward only (monotonic restriction)
- **POLA by construction** — new subjects start with only the capabilities explicitly provided
- **Only connectivity begets connectivity** — capabilities can only be obtained through initial conditions, parenthood, endowment, or introduction

**[Strong — foundational CS research with machine-checked proofs (seL4)]**

### 4.2 The Spectrum of Assurance

| System | Default posture | Enforcement | Formal assurance |
|--------|----------------|-------------|-----------------|
| seL4 | Zero authority | Kernel trap on every capability invocation | **Machine-checked proofs** (Isabelle/HOL, functional correctness + information flow) |
| Capsicum (FreeBSD) | Ambient → voluntary `cap_enter()` | Kernel-level, ~60 capability rights | Design-level argument, model checking |
| Zircon (Fuchsia) | Zero ambient authority (no POSIX) | Handle-based, kernel-enforced | Design-level + hardware mitigations |
| WASI/Component Model | Zero authority (architectural absence) | Compile-time (WIT imports) + runtime (host handles) | Formally specified, no end-to-end security proof |
| Linux (seccomp + LSMs + namespaces) | Full ambient authority | Composition of deny mechanisms | **Approximation** — new resource types accessible by default until denied |

**The critical distinction**: In ACL systems, the agent tries to access the filesystem and is blocked. In capability systems, the agent cannot express a filesystem access because it lacks the capability. This is not merely semantic — blocked access attempts leak information (resource exists, path resolves, timing metadata). Capability systems make the operation absent from the agent's universe of discourse. **[Strong — formally proven property, Dennis & Van Horn through Miller]**

### 4.3 WASI: CloudABI's Inheritance, WASM's Clean Slate

WASI inherited Capsicum's principles via CloudABI but starts from a stronger position:

- Capsicum: ambient authority process must voluntarily enter capability mode via `cap_enter()`
- CloudABI: no unsandboxed mode — 49 system calls, compile-time enforcement
- WASI: Wasm has **no syscall instructions, no I/O ports** — authority enters only through host-controlled imports

**Two-layer capability control** in the Component Model:
1. **Link-time capabilities**: WIT world declaration specifies imports. Missing import = operation doesn't exist in address space.
2. **Runtime capabilities**: Unforgeable handles passed as arguments. No fabrication from integers or strings.

**Current limitations** (WASI 0.2): No capability revocation. Preopened directories are all-or-nothing for subtrees. Network restrictions by IP only, not domain. Node.js WASI implementation does not properly restrict by default. **[Strong — documented in WASI spec and Gohman's writings]**

### 4.4 Wassette: Capabilities Meet Agent Tools

Microsoft's Wassette (Azure Core Upstream, v0.3.4, August 2025) bridges Wasm Components to MCP. Components are fetched from OCI registries, sandboxed in Wasmtime, and exposed as MCP tools. Deny-by-default: zero access to filesystem, network, or stdio unless explicitly granted.

**Not production-ready.** Explicitly labeled as such. Nascent ecosystem of Wasm-compiled MCP servers. WASI lacks thread support. Performance overhead from Wasmtime is measurable versus native. **[Strong — verified from Microsoft GitHub; production readiness caveat from maintainers themselves]**

---

## 5. Composition Models: Instrumental vs. Constitutive

### The Fundamental Distinction

| Property | Instrumental composition | Constitutive composition |
|----------|------------------------|-------------------------|
| Control flow | Orchestrator invokes tools | Components communicate directly |
| State sharing | Passed through orchestrator or graph edges | Shared mutable context, peer-to-peer |
| Protocol | MCP (client-server), LangGraph (supervisor) | wRPC (typed peer-to-peer via WIT) |
| Agent role | Instrument wielded by LLM | Peer participant in unified process |
| Production examples | LangGraph, CrewAI, AWS Bedrock Agents | wasmCloud lattice (closest), none complete |

**LangGraph** provides centralized shared state (TypedDict/Pydantic) where all nodes read/write through a single structure. Reducer functions handle concurrent merge. But execution order is always controlled by graph topology, typically LLM-driven. This is instrumental. **[Strong — documented behavior]**

**wasmCloud wRPC** enables something fundamentally different — components communicate through shared typed interfaces rather than through a central LLM orchestrator. This is the closest existing analog to constitutive composition. **[Strong — Bytecode Alliance project]**

### The Shared Ontology Problem

No major agent protocol includes a shared ontology for semantically interoperable knowledge exchange:
- A2A deliberately keeps agents opaque
- MCP has no server-to-server communication
- Only ANP adopts JSON-LD with Schema.org vocabularies
- WIT provides structural interoperability (data shapes) but no semantic layer (data meaning)

**Minimum viable shared representation** requires three layers:
1. **Typed interface contracts** (WIT) — exists
2. **Shared mutable context** accessible without LLM mediation — does not exist as a standard
3. **Semantic annotations** mapping types to cognitive vocabulary — does not exist

The critical missing piece is Layer 2. Current systems force a choice: rich centralized state (LangGraph) OR direct peer communication without shared state (wRPC). **[Moderate — analysis across all sources converges on this gap]**

---

## 6. The Variance Reduction Thesis

The information-theoretic argument is verified from three independent angles:

| Paper | Finding | Design | Evidence Level |
|-------|---------|--------|---------------|
| **Anka DSL** (arXiv:2512.23214) | 60% → 95.8% accuracy via 18-operation constrained vocabulary | Controlled experiment | **Strong** (100% on multi-step pipeline tasks; 95.8% overall) |
| **CIFE** (arXiv:2512.17387) | 39-66% strict constraint adherence across 14 models, 1,000 tasks | Benchmark, 13 constraint categories | **Strong** |
| **IFEval++** (arXiv:2512.14754) | Up to 61.8% performance drop from synonym rephrasing | 46 LLMs tested | **Strong** |
| **CtxBugGen** (arXiv:2601.06497) | 55.93% Pass@1 for context-sensitive tasks; nearly half of generated code has subtle semantic defects | 3,683 bugs, Kimi-K2 best model | **Strong** |

**The mechanism**: Constraining the generation space — replacing "generate arbitrary code" with "select from verified components" — reduces the entropy of the decision. This is the same mechanism at every layer: DSL vocabulary constrains language generation, behavioral contracts constrain component selection, capability scoping constrains resource access. **[Strong — converging evidence from independent teams]**

---

## 7. Where geist.sh Fits

### 7.1 What Exists vs. What's Missing

| Capability | Exists? | Where? | Gap for agents |
|-----------|---------|--------|----------------|
| Sub-millisecond agent isolation | Yes | Fermyon Spin, Wasmtime | Not purpose-built for agent lifecycle |
| Agent-to-tool protocol | Yes | MCP (97M SDK downloads) | No behavioral contract enforcement |
| Agent-to-agent protocol | Yes | A2A (150+ orgs) | Deliberately opaque — no shared state |
| Typed component interfaces | Yes | WIT, Component Model | No semantic layer for cognitive operations |
| Kernel-level enforcement | Yes | BPF-LSM, Tetragon, Aya-rs | No WIT-contract-aware enforcement exists |
| Durable agent state | Yes | Durable Objects, Restate, Temporal | Not integrated with capability-scoped isolation |
| Intent-action correlation | Research | AgentSight (arXiv) | No production implementation |
| Assembled-per-intent runtime | No | — | **Confirmed gap across all sources** |
| Behavioral contracts for AI components | No | — | **SLICK-style contracts have no production implementation** |
| WIT → BPF-LSM policy translation | No | — | **Novel R&D territory** |

### 7.2 The geist.sh Value Proposition

geist.sh fills a specific, verified gap: **no existing system combines safe composition, behavioral contract enforcement, and intent-action observability in a single runtime**.

Individual pieces exist:
- Wasmtime/Spin provide capability-based isolation
- MCP/A2A provide protocol foundations
- Aya-rs/Tetragon provide kernel-level enforcement
- Restate/Temporal provide durable execution
- wasmCloud provides distributed composition

What doesn't exist:
- A runtime that reads behavioral contracts and generates enforcement policies
- A system that correlates LLM intent with kernel-level actions
- An assembly mechanism that composes capabilities per-intent with exactly the authority needed

### 7.3 The Genuinely Novel Contributions

**1. eBPF Sentinel Layer** — Intent-action correlation at the kernel level. uprobes on SSL_read/write for intent extraction, kprobes on execve/openat2 for action monitoring, BPF-LSM for enforcement. The individual techniques are proven; the correlation engine and WIT-to-BPF-LSM translation are novel. **[Moderate — proven primitives, novel integration]**

**2. Behavioral contracts as enforcement input** — SLICK-style safety_level/postconditions/invariants as Agent Skills frontmatter extensions, consumed by the runtime to generate policies. No existing system does this. **[Moderate — sound design, no production validation]**

**3. Per-intent capability assembly** — Composing a runtime with exactly the capabilities needed for a specific task, then dismantling it. The closest approach (wasmCloud lattice) provides the infrastructure but not the intent-analysis-to-composition pipeline. **[Speculative — no implementation exists anywhere]**

---

## 8. Risks and Honest Assessment

### What Could Kill This

1. **WASM ecosystem matures faster than expected.** If componentize-py becomes production-ready and WASI 0.3 ships on time, the Rust+PyO3 choice loses its primary advantage (full Python ecosystem) while missing WASM's strongest property (compile-time capability enforcement).

2. **MCP absorbs behavioral contracts.** If the Agentic AI Foundation adds contract semantics to MCP, geist.sh's differentiator narrows to the eBPF enforcement layer.

3. **The correlation engine proves impractical.** Intent-action correlation across async operations with natural language intents is an unsolved research problem. If false positive rates are too high, the Sentinel Layer is reduced to standard syscall monitoring.

4. **wasmCloud ships the intent layer.** wasmCloud already has dynamic composition via wRPC/NATS. If they add intent analysis, geist.sh's per-intent assembly story weakens.

5. **DeepMind's threshold rises.** As base models improve, the 45% accuracy threshold suggests fewer tasks benefit from multi-agent approaches — potentially reducing demand for multi-agent coordination infrastructure.

### What We're Confident About

| Claim | Confidence | Why |
|-------|-----------|-----|
| Constrained composition reduces variance | **95%** | Three independent papers verify from different angles |
| eBPF enforcement primitives are production-ready | **90%** | Aya-rs + Tetragon verified at multiple organizations |
| No complete assembled-per-intent runtime exists | **90%** | Exhaustive search across all sources confirms gap |
| MCP + A2A are the protocol foundation | **85%** | Linux Foundation backing, adoption numbers verified |
| WIT→BPF-LSM translation is feasible | **60%** | Architecturally sound; no implementation; security-critical |
| Intent-action correlation at scale is achievable | **50%** | AgentSight proves concept; production viability unknown |

---

## Appendix: Source Documents

| Document | Type | Key contribution to this synthesis |
|----------|------|-----------------------------------|
| compass-52c0 (Capability Architectures) | Deep research | Formal capability theory lineage, Capsicum → WASI |
| compass-dfbd (Corrections and Gap Fills) | Deep research | Hybrid runtimes, wRPC, DeepMind scaling, Wassette details |
| agentic_data_plane_synthesis.docx | Deep research | Architectural affordances, protocol convergence, 10 design patterns |
| geist-architectural-analysis-wasm-agents (Report 1) | Deep research | Sandbox benchmarks, Sentinel Layer proposal (treat with skepticism) |
| geist-cognitive-data-planes-v2 (Report 3) | Deep research | Traceability matrix, dual-tool eBPF strategy, best technical accuracy |
| slick-component-architecture-analysis (Report 2) | Deep research | Variance reduction evidence, Design-by-Contract, hexagonal architecture |
| geist-research-synthesis | Prior synthesis | Verified claims, corrections, confidence levels |
| slick-guild-synthesis | Guild verdict | SLICK → Agent Skills extensions, three-move sequence |
| architectural-analysis-critique | Critique | Sentinel Layer reality check, WIT→LSM gap, engineering estimates |
| research-synthesis-2026-02-13 | Working document | Earlier partial synthesis (superseded by this document) |
