# Research Knowledge Base

Reading guide to the geist.sh research corpus. 13 documents covering agentic systems, formal verification, runtime isolation, and data plane design.

## Source Inventory

### Academic Papers (syntheses from arXiv PDFs)

| File | Paper | arXiv | Scope |
|------|-------|-------|-------|
| `allegrini-formal-properties.md` | Allegrini et al. — Formalizing Safety, Security, and Functional Properties of Agentic AI Systems | [2510.14133](https://arxiv.org/abs/2510.14133) | Formal verification framework for multi-agent systems. State machine models for MCP/A2A protocols. Safety and security properties as verifiable invariants. |
| `camel-info-flow-control.md` | Debenedetti et al. — Defeating Prompt Injections by Design (CaMeL) | [2503.18813](https://arxiv.org/abs/2503.18813) | Information flow control defense against prompt injection. Capability-based security for LLM agents. Custom Python interpreter enforces security policies without modifying the LLM. Solves 77% of AgentDojo tasks with provable security. |
| `dao-agentic-design-patterns.md` | Dao et al. — Agentic Design Patterns: A System-Theoretic Framework | [2601.19752](https://arxiv.org/abs/2601.19752) | 12 design patterns for agent systems derived from systems theory. Five functional subsystems (Reasoning, Perception, Action, Learning, Communication). NeurIPS 2025 LAW workshop. |
| `kim-scaling-agent-systems.md` | Kim et al. (Google/MIT) — Towards a Science of Scaling Agent Systems | [2512.08296](https://arxiv.org/abs/2512.08296) | Quantitative scaling laws for multi-agent systems. 180 configurations, 5 architectures, 3 LLM families. Key findings: coordination hurts sequential tasks (-39-70%), independent agents amplify errors 17.2x, centralized coordination contains to 4.4x. |
| `miehling-systems-theory.md` | Miehling et al. (IBM) — Agentic AI Needs a Systems Theory | [2503.00237](https://arxiv.org/abs/2503.00237) | Position paper arguing for systems-theoretic perspective on agentic AI. Emergent capabilities from agent interaction. Mechanisms for enhanced cognition, causal reasoning, metacognitive awareness. |

### Project Syntheses

| File | Scope |
|------|-------|
| `geist-research-synthesis.md` | **Start here.** Cross-validated synthesis of 3 deep research reports. Verifies every cited claim against primary sources. Separates what's proven from what's overstated. Includes corrections (CVE-2023-41880 is not a sandbox escape, "microsecond cold starts" contradicted by own data). |
| `synthesis-agentic-data-plane.md` | Technical landscape analysis. Runtime isolation benchmarks, protocol comparison (MCP/A2A/wRPC), multi-agent scaling constraints, defense-in-depth architecture. Evidence-labeled throughout. |
| `synthesis-geist-adr.md` | Architectural decision records. ADR-001: Rust+PyO3+eBPF over WASM for MVP. ADR-002: SLICK contracts as Agent Skills extensions. ADR-003: eBPF with Darwin fallback. Migration paths and risk analysis. |

### Deep Research Reports

| File | Scope |
|------|-------|
| `geist-cognitive-data-planes-v2.md` | White paper on cognitive data planes. Wasm Components + eBPF thesis. Isolation benchmarks, capability-based security, blackboard vs. hierarchical orchestration, WASI roadmap. The most technically rigorous of the three reports. |
| `geist-architectural-analysis-wasm-agents.md` | WASM/eBPF runtime architecture analysis. Sandbox provider comparison (E2B, Northflank, Koyeb). UMA framework. Contains some overstated claims corrected in `geist-research-synthesis.md`. |
| `slick-component-architecture-analysis.md` | SLICK semantic component composition. Variance reduction through constrained generation. Triangulated analysis (Skeptic/Architect/Practitioner). Hexagonal architecture recommendation. |

### Reference Documents

| File | Scope |
|------|-------|
| `component-reference.md` | Technology reference. What each technology in the stack actually is — seccomp, eBPF, Aya-rs, Tetragon, srt (Anthropic sandbox-runtime), MCP, A2A, Wasm Component Model. Every claim sourced. |
| `sandbox-runtime-analysis.md` | Analysis of Anthropic's `sandbox-runtime` (srt). How it works on macOS (seatbelt) and Linux (bubblewrap + seccomp). API surface, network proxy architecture, violation monitoring. |

## Reading Order

1. **`synthesis-comprehensive.md`** — start here. Integrates and supersedes the three synthesis files below. Problem space, architecture, prior art, evidence base, ADRs, open questions.
2. **`component-reference.md`** — if you need to understand a specific technology
3. Academic papers — for primary source verification
4. **`geist-research-synthesis.md`** — detailed claim verification and corrections (superseded by `synthesis-comprehensive.md`, kept as reference)
5. **`synthesis-agentic-data-plane.md`** — detailed landscape analysis (superseded by `synthesis-comprehensive.md`, kept as reference)
6. **`synthesis-geist-adr.md`** — detailed ADRs (superseded by `synthesis-comprehensive.md`, kept as reference)

## Clusters

### Variance Reduction (strongest evidence)

Constraining LLM action space improves reliability. Converging evidence from multiple independent studies:

- Anka DSL: 60% → 95.8% accuracy via 18-operation vocabulary ([arXiv:2512.23214](https://arxiv.org/abs/2512.23214))
- CIFE: 39-66% strict constraint adherence across 14 models ([arXiv:2512.17387](https://arxiv.org/abs/2512.17387))
- IFEval++: up to 61.8% performance drop from synonym rephrasing ([arXiv:2512.14754](https://arxiv.org/abs/2512.14754))
- CtxBugGen: 55.93% Pass@1 for context-sensitive tasks ([arXiv:2601.06497](https://arxiv.org/abs/2601.06497))

**Relevance to geist.sh**: This is the theoretical foundation for typed extension registry + behavioral contracts. Processors with typed configs reduce the LLM's decision space.

### Multi-Agent Coordination (strong evidence)

Scaling laws from Kim et al. (180 configurations, cross-validated R²=0.524):

- Tool-heavy tasks suffer disproportionately from multi-agent overhead
- Coordination yields diminishing/negative returns once single-agent baseline exceeds ~45%
- Independent agents amplify errors 17.2x; centralized coordination contains to 4.4x
- Sequential reasoning tasks degrade 39-70% with any multi-agent variant

**Relevance to geist.sh**: Governs compositor design. Not all tasks benefit from multi-agent. The runtime must select topology based on task properties.

### Runtime Isolation (strong evidence, well-benchmarked)

Wasm vs. microVM performance delta is architectural:

- Fermyon Spin: 0.52ms cold start, 75M RPS (verified)
- Hyperlight: 1-2ms with double-layer Wasm+KVM isolation (verified, CNCF Sandbox)
- Firecracker: 100-125ms (verified, AWS Lambda)
- gVisor: 50-100ms, 10x syscall overhead (plausible)

**Correction**: "microsecond cold starts" was overstated. Production floor is 0.52ms (Spin).

### Formal Verification (emerging)

Allegrini's framework formalizes MCP/A2A interactions as state machines with verifiable safety properties. CaMeL demonstrates provable security against prompt injection via information flow control. Both are academic — no production implementations yet.

### Security Defense-in-Depth (architecturally sound)

Four-layer stack: seccomp → eBPF LSM → Tetragon → application policy. Each layer verified independently. The integrated stack is geist.sh's design, not a cited implementation.

## Architectural Prior Art

Patterns extracted from production data plane engineering experience. These are the concepts geist.sh builds on — validated through real deployments, sanitized for public reference.

### Extension Protocol Adapter

Wrap entire plugin pipelines with a single protocol adapter rather than rewriting plugins. Translation happens at the pipeline boundary, not per-plugin. Enables runtime portability: same processors work on Envoy (via ext_proc), on an embedded gateway, or as standalone services.

**Production validation**: 59 production plugins migrated to a new runtime in 6-8 weeks with 2-3 engineers, versus 12-18 month rewrite with 10-15 engineers. Zero plugins rewritten.

**In geist.sh**: The Processor trait + ext_proc protocol implements this pattern. Future adapter swaps (axum → pingora → ext_proc) are low-risk because processors are portable.

### Policy-Processor Separation

Decouple "what should happen" (policy/config) from "how it happens" (processor implementation). Tenant policies remain stable during processor upgrades. Platform evolves implementations without tenant reconfiguration.

**In geist.sh**: Each processor owns its config type. Typed extension registry maps type URLs to factories. No generic PolicyEvaluator — processors compile their policy to matchers at construction time.

### 4D Architecture Framework

Understand systems through four operational layers:

| Layer | Role | geist.sh mapping |
|-------|------|-----------------|
| **Decision** | Policy definition and control logic | Typed extension registry + policy compilation |
| **Dissemination** | Configuration distribution | ECDS (xDS) for dynamic updates |
| **Discovery** | Service and topology discovery | Capability IDs, registry lookup |
| **Data** | Policy enforcement (request processing) | Processor pipeline |

### ACES (Adaptable, Composable, Extensible Software)

Not a pattern but a system quality — achieved through implementation choices:

- **Adaptable**: Swap adapter (axum → pingora → ext_proc) without touching processors
- **Composable**: Processors compose through pipeline + metadata, not direct coupling
- **Extensible**: Add processor = register extension + config entry. `inventory::submit!` enables zero-code-change registration

### Unified Processing Model

Single processor implementation runs across all platforms via adapters. Write once, deploy everywhere. Same testing, same behavior. The processor doesn't know or care what runtime hosts it — only the adapter does.

### Configuration Hierarchy (Progressive Disclosure)

CSS-like specificity for configuration:

```
Platform Defaults (built-in, zero-config)
  → Team/Workspace Defaults (override only what differs)
    → Service-Specific Configuration (final overrides)
      → Resolved Configuration
```

Users get working defaults immediately. Advanced users customize only specific layers.

### APIServer/APIClient with Logical Capability IDs

Services depend on logical capability IDs, not physical endpoints. Control plane resolves capabilities to implementations. Service decomposition doesn't break clients — infrastructure changes are invisible to consumers.

### First Principles Constraints

Immutable constraints that shape architectural decisions:

- **Physics**: 9.8ms RTT per 1000km (absolute floor)
- **Complexity**: N services → N×(N-1)/2 potential connections
- **Cognition**: 4±1 working memory chunks; deep understanding of 3-5 APIs simultaneously
- **Reliability**: 10 services at 99.9% = 99.0% system reliability (series composition)

These justify the typed registry (reduces N² coordination), processor pipeline (minimize hops), and extension interface (respects cognitive limits).

## Gaps

- **No production system implements intent-to-action observability** — AgentSight is research only
- **No production system enforces coordination topology** — Kim et al.'s scaling laws have no runtime implementation
- **WASI 0.3 not yet stable** — async primitives (`stream<T>`, `future<T>`) still in preview
- **componentize-py limitations** — no threading, partial C extensions, no production evidence for agent workloads
- **eBPF is Linux-only** — Darwin development requires trait-based fallback (ADR-003)
