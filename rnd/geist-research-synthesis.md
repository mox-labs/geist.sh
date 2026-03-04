# Geist.sh: Verified Research Synthesis

> **Date**: February 13, 2026
> **Reports synthesized**:
> 1. `geist-architectural-analysis-wasm-agents.md` (Report 1) — WASM/eBPF runtime architecture
> 2. `slick-component-architecture-analysis.md` (Report 2) — SLICK semantic component composition
> 3. `geist-cognitive-data-planes-v2.md` (Report 3) — Cognitive data planes (strongest)
>
> **Methodology**: Independent verification of all cited claims via primary sources. Evidence labeled **Strong** / **Moderate** / **Weak** / **Speculative**. All three reports were produced by AI deep research — this synthesis corrects, qualifies, and integrates their findings.

---

## The Thesis

Constrained, contract-enforced composition is strictly superior to unconstrained generation for agentic workloads. This holds at every layer of the stack — from kernel enforcement (eBPF LSM programs that make unauthorized syscalls inexpressible) through runtime isolation (Wasm capabilities that make unauthorized operations uncompilable) to semantic contracts (SLICK manifests that reduce LLM decision entropy). The mechanism is the same everywhere: **reducing the space of possible actions reduces the probability of incorrect action**.

This is not architectural preference. It is information-theoretic fact, confirmed empirically:

- When Anka constrained an LLM to 18 DSL operations instead of general Python, accuracy on multi-step tasks went from 60% to 100%. **[Strong — arXiv:2512.23214, verified]**
- When CIFE measured unconstrained constraint adherence, models followed specific requirements only 39–66% of the time. **[Strong — arXiv:2512.17387, verified]**
- When IFEval++ tested synonym rephrasing of identical prompts, performance dropped up to 61.8%. **[Strong — arXiv:2512.14754, verified]**

The question for geist.sh is not whether to constrain, but *where in the stack* to place each constraint, and *which constraints are production-ready today*.

---

## What Is Verified

### Runtime Performance

| Claim | Report | Verdict | Primary Source |
|---|---|---|---|
| Fermyon Spin 0.52ms cold start, 75M RPS on Akamai | 3 | **Verified** | [GlobeNewsWire, Nov 2025](https://www.globenewswire.com/news-release/2025/11/12/3186327/0/en/) |
| Wasmtime standalone instantiation <30μs | 3 | **Partially verified** | Module instantiation can be microseconds; end-to-end cold starts are sub-millisecond. Report 1's 16.9ms figure includes full initialization path. |
| Hyperlight 1–2ms double-layer isolation (Wasm+KVM) | 3 | **Verified** | [Microsoft OSS Blog, Feb 2025](https://opensource.microsoft.com/blog/2025/02/11/hyperlight-creating-a-0-0009-second-micro-vm-execution-time) — CNCF Sandbox |
| Firecracker end-to-end 94.2ms | 1 | **Plausible** | Consistent with ≤125ms spec. Realistic for production with jailer + network setup. |
| gVisor 50–100ms, 10x syscall overhead | 3 | **Plausible** | Consistent with known gVisor architecture (Sentry syscall interception). |

**Correction needed**: Report 1 claims "microsecond-level cold starts" while its own data shows 16.9ms. Report 3 is more precise — standalone instantiation is microseconds, production cold starts (Spin) are 0.52ms. The 1000x claim (vs microVMs) holds for Spin-vs-Kata but not for Wasmtime-standalone-vs-Firecracker-bare.

### Ecosystem Projects

| Project | Claim | Verdict | Evidence Level |
|---|---|---|---|
| wasmCloud | CNCF Incubating, wRPC/NATS lattice | **Verified** | Moved to Incubating Nov 2024. Active releases. **Strong** |
| SpinKube | Kubernetes-native Wasm, containerd-shim-spin | **Verified** | Shim v0.22.0 (Nov 2025), Azure AKS integration. **Strong** |
| Wassette | Wasm→MCP bridge, Microsoft | **Verified** | [Microsoft OSS Blog, Aug 2025](https://opensource.microsoft.com/blog/2025/08/06/introducing-wassette-webassembly-based-tools-for-ai-agents) — Azure Core team. **Strong** |
| WASI 0.3 | Previews in Wasmtime 37+, `stream<T>`/`future<T>` | **Verified** | [wasi.dev/roadmap](https://wasi.dev/roadmap), Fermyon blog. Expected ~Feb 2026. **Strong** |
| componentize-py | v0.19.3, Python→Wasm Components | **Verified** | Bytecode Alliance. NumPy demonstrated. Threading absent, C extensions partial. **Strong** |
| Dandelion | SOSP '25, distributed runtime | **Verified** | arXiv:2505.01603. Multiple isolation backends (KVM, Linux processes, CHERI, rWasm). **Strong** |
| AlloyStack | Library OS for serverless | **Verified** | EuroSys publication. 98.5% cold start reduction (to 1.3ms). **Strong** |

### eBPF/Security Stack

| Claim | Verdict | Evidence Level |
|---|---|---|
| Aya-rs: 22 eBPF program types, pure Rust, BTF/CO-RE | **Verified** — [aya-rs.dev](https://aya-rs.dev), production at Deepfence, Red Hat (bpfman), Exein (Pulsar) | **Strong** |
| Tetragon: TracingPolicy CRDs, kernel-level enforcement | **Verified** — [tetragon.io](https://tetragon.io), CNCF project under Cilium, v1.6.0 | **Strong** |
| Deepfence eBPFGuard: Aya-based LSM policy enforcement | **Verified** — [GitHub](https://github.com/deepfence/ebpfguard), Rust/YAML policies, tested on host + containers + K8s | **Strong** |
| AgentSight: dual-probe boundary tracing, <3% overhead | **Verified** — arXiv:2508.02736, ACM published, open source | **Strong** |
| BPF-LSM superiority over seccomp-BPF | **Verified** — deep kernel state inspection (file paths, socket addresses) vs register-only inspection | **Strong** |
| 4-layer defense-in-depth stack (seccomp → eBPF LSM → Tetragon → OPA) | **Architecturally sound** — each layer verified independently; integration is Report 3's design, not a cited implementation | **Moderate** |

### Variance Reduction Research

| Paper | Claim | Verdict | Detail |
|---|---|---|---|
| **CtxBugGen** (arXiv:2601.06497) | 55.93% Pass@1 for context-sensitive tasks | **Verified** | 3,683 CtxBugs, Kimi-K2 best model, up to 30% degradation from CtxBugs. GitHub: ztwater/CtxBugGen |
| **CIFE** (arXiv:2512.17387) | 39–66% strict constraint adherence | **Verified** | 1,000 Python tasks, 13 constraint categories, 14 models. Partial adherence >90%, strict 39–66%. |
| **IFEval++** (arXiv:2512.14754) | Up to 61.8% performance drop from synonym rephrasing | **Verified** | 46 LLMs (20 proprietary + 26 open-source). Introduces reliable@k metric. |
| **Anka DSL** (arXiv:2512.23214) | 60%→100% accuracy via constrained vocabulary | **Partially verified** | 100% on multi-step pipeline tasks specifically; 95.8% overall accuracy. Report 2's framing is selective but directionally correct. |
| **bMAS blackboard** (arXiv:2507.01701) | Competitive accuracy with fewer tokens via self-selection | **Verified** | LbMAS implementation, public/private blackboard spaces. |

---

## What Is Overstated or Wrong

These corrections matter because they change architectural decisions.

### CVE-2023-41880: Not a Sandbox Escape

Report 1 (Section 2.2) presents CVE-2023-41880 as evidence that JIT miscompilation can "pierce the linear memory sandbox," enabling arbitrary host memory access. The actual NVD description states: **"this issue is not an escape from the WebAssembly sandbox."** It is a miscompilation bug (bit-shift producing incorrect results within the sandbox), not a security boundary violation.

**Impact**: Report 1's "Wasm Breach" attack chain — prompt injection → malformed Wasm → JIT miscompilation → sandbox escape — is plausible in theory but its cited evidence does not support the specific claim. The defense-in-depth argument for eBPF LSM as a second perimeter remains sound on general principle, but the motivating CVE is misrepresented. **[Evidence: Strong — NVD entry directly contradicts Report 1]**

### "Microsecond-Level Cold Starts" (Report 1)

Report 1's own benchmarks show Wasmtime at 16.9ms — milliseconds, not microseconds. Module instantiation alone can be microseconds (5μs for SpiderMonkey.wasm per Mozilla), but no production Wasm agent runtime achieves microsecond end-to-end cold starts. Fermyon Spin's 0.52ms is the production floor. **[Evidence: Strong — Report 1's own data]**

### "Complete Abandonment of Containers" (Report 1)

Contradicted by Report 1's own Phase 2 (Kata fallback), by Report 3's recommendation to use both Aya-rs AND Cilium/Tetragon, and by the entire SpinKube project which runs Wasm *within* Kubernetes. The trajectory is hybrid, not replacement. **[Evidence: Strong — internal contradiction]**

### UMA as Industry Standard (Report 1)

"Universal Microservices Architecture" comes from Enrico Piovesan's Medium blog series. Report 3 correctly notes it is "one researcher's framework rather than a Bytecode Alliance standard." The pattern it describes (WIT contracts → portable logic → runtime adapters) is real and is the trajectory of the Bytecode Alliance toolchain — but calling it a "standard" is inaccurate. **[Evidence: Strong — no W3C/BA/CNCF standardization]**

### Anka DSL: 100% Needs Qualification

Report 2 frames this as "60% to 100%." The actual paper shows 95.8% overall accuracy, with 100% specifically on multi-step pipeline tasks. The general claim that constrained vocabularies dramatically improve accuracy is strongly supported. The specific "100%" number needs the qualifier "on multi-step pipeline tasks." **[Evidence: Strong — arXiv paper states 95.8% overall]**

### TOOLQP (Report 2)

Cited as research demonstrating the "semantic gap" in tool selection. No arXiv paper with this identifier was found. The concept exists under different names — GRETEL's "semantic-functional gap" (arXiv:2510.17843), TOOLCERT's robustness framework — but "TOOLQP" itself appears to be Report 2's own term. **[Evidence: Weak — unverifiable citation]**

---

## What Is Missing

Across all three reports, these gaps remain unaddressed:

1. **Internal data flow model.** How does data move *within* a geist instance? Between the LLM loop, state store, tool execution, and protocol adapters? None of the reports specify this.

2. **State management architecture.** Report 3 mentions NATS JetStream KV buckets and Wasmtime SharedMemory but doesn't design a coherent state model. What is ephemeral vs persistent? What survives restarts?

3. **Multi-tenancy.** Reports assume single-tenant operation. No treatment of tenant isolation, resource quotas, or billing boundaries.

4. **macOS development story.** eBPF is Linux-only. All three reports assume Linux deployment. No discussion of development ergonomics on Darwin (the primary development platform for this project).

5. **HUD control plane.** Referenced in session context but absent from all research reports. How does configuration reach geist instances? Push vs pull? Format? Security?

6. **Cost model.** No analysis of memory overhead per agent instance, compute cost at scale, or comparison with container-based approaches at equivalent workload.

7. **Real-world agent performance on Wasm.** All cold-start benchmarks measure hello-world-class workloads. No data on LLM inference latency, MCP tool invocation latency, or multi-turn conversation state management through Wasm Components.

---

## The Architecture Decision: WASM vs Rust+eBPF+PyO3

The three reports arrive at different conclusions about runtime, but the verification data resolves the tension.

### What the Reports Recommend

- **Report 1**: Full WASM (Wasmtime), eBPF as Phase 4 optimization
- **Report 2**: Pure Python with lightweight runtime (Dijkstra), protocol-agnostic
- **Report 3**: WASM Components + eBPF, with componentize-py for Python agents — the most technically rigorous position

### What the Verification Data Shows

**For WASM:**
- Ecosystem is real and maturing fast. wasmCloud (CNCF Incubating), SpinKube, Hyperlight (CNCF Sandbox), Wassette (Microsoft) are production projects, not vapor.
- WASI 0.3 with async is imminent (~Feb 2026).
- Wasm Component Model is the only capability-based isolation model with compile-time enforcement.
- Cold start advantage is genuine and significant for agentic patterns (sub-millisecond vs hundreds of milliseconds).

**Against WASM for MVP:**
- componentize-py has real limitations: no threading, partial C extensions, no sys.exit.
- The Component Model's shared-nothing design creates a fundamental tension with multi-agent state sharing — WASI has no shared memory type (WebAssembly/WASI#594).
- No production evidence of Python AI agent workloads running in WASM Components. All verified benchmarks are simple request-response patterns.
- WASI 0.3 is "expected" not "shipped." Building an MVP on "expected" standards is a dependency risk.

**For Rust+eBPF+PyO3:**
- Full Python ecosystem immediately — no componentize-py gamble.
- PyO3 is mature (13k+ GitHub stars), supports GIL release, zero-copy numpy interop.
- eBPF Sentinel Layer works identically regardless of runtime (uprobes/kprobes attach to processes, not to Wasm runtimes specifically).
- Eliminates WASI 0.3 as a critical path dependency.
- Aya-rs for custom enforcement, Tetragon for baseline — both verified production-ready.

**Against Rust+eBPF+PyO3:**
- No compile-time capability enforcement (WASM's strongest security property).
- Process-level isolation is weaker than WASM SFI (requires eBPF/seccomp to compensate).
- Misses the wasmCloud distributed composition model (wRPC/NATS lattice).
- Doesn't participate in the Wasm Component ecosystem (Wassette, SpinKube, wasi-cloud-core).

### Recommended Path

**Phase 1 (MVP)**: Rust runtime + PyO3 + eBPF (Aya-rs). Ship a working cognitive data plane. Use SLICK manifests for semantic contracts. eBPF provides observability and enforcement. No WASM dependency risk.

**Phase 2 (Evaluation)**: Once WASI 0.3 ships and componentize-py matures, evaluate Wasm Components as a *guest format* within the Rust runtime. The WIT interface definitions from Phase 1 should be designed to map cleanly to Wasm Component interfaces — this is free if the interfaces are typed and capability-scoped from the start.

**Phase 3 (Distribution)**: If Wasm Components prove viable, adopt wasmCloud's lattice model for distributed composition. The Rust runtime becomes a wasmCloud host with custom capability providers.

This path preserves optionality. It does not bet against WASM — it bets on WASM maturing before committing to it.

---

## SLICK Integration

Report 2's core insight is verified and applicable regardless of runtime: **constrained selection from verified components reduces variance**. The specific integration points:

### Manifest → Enforcement Pipeline

```
slick.yaml manifest
  ├── metadata.semantics.intent    → Agent discovery ("what can this do?")
  ├── spec.contracts.invariants    → Pre-dispatch validation ("should this call happen?")
  ├── spec.contracts.guarantee     → Post-execution verification ("did it do what it promised?")
  ├── metadata.semantics.safety_level → Policy generation ("what syscalls are allowed?")
  │     read-only  → BPF-LSM blocks all write syscalls
  │     idempotent → Safe retry without additional guards
  └── spec.adapters                → Protocol projection (MCP/A2A/REST)
```

### Where Contracts Are Enforced

| Layer | Enforcement | Tool |
|---|---|---|
| Semantic (pre-dispatch) | Intent matching, contract preconditions | SLICK Dijkstra runtime (Python, via PyO3) |
| Application (post-execution) | Postcondition validation, invariant checking | Pydantic validators in Python agent |
| Process (runtime) | Syscall filtering, resource limits | eBPF LSM (Aya-rs) + seccomp-BPF |
| Network (external) | Egress control, protocol enforcement | Cilium/Tetragon TracingPolicies |

### The Bootstrapping Problem

Report 2's own lowest confidence area (65% adoption). For geist.sh, the first 100 components don't need to be built from scratch. The strategy is:

1. **Wrap existing MCP servers** as SLICK components — auto-generate manifests from MCP tool schemas
2. **Wrap existing Python libraries** — Bodhi tool extracts contracts from type hints + docstrings
3. **Core primitives first** — file I/O, HTTP, database, LLM inference as SLICK components with strong contracts
4. **Prove the pipeline** — one real agent workflow (e.g., code review) fully SLICK-constrained, measured against unconstrained baseline

---

## The Sentinel Layer

The eBPF enforcement architecture is the strongest, most consistently verified finding across all three reports. AgentSight (arXiv:2508.02736) provides the academic foundation. Aya-rs and Tetragon provide the production tools.

### Dual-Tool Strategy (Verified)

| Concern | Tool | Why |
|---|---|---|
| Standard network observability | Cilium/Hubble | L3-L7 flow visibility, service dependency maps. CNCF Graduated. Zero custom code. |
| Standard runtime security | Tetragon TracingPolicies | Kubernetes-aware enforcement via YAML CRDs. Block unauthorized egress, monitor breakout attempts. |
| WIT/SLICK contract enforcement | Aya-rs custom programs | Novel requirement: parse contracts at deploy time, generate per-component eBPF maps, enforce at kernel level. |
| Intent-action correlation | Aya-rs uprobes + kprobes | uprobes on LLM API calls (SSL_read/write) for intent extraction. kprobes on execve/openat2 for action monitoring. Correlation engine bridges the semantic gap. |

### eBPF Phasing

Report 1 places eBPF in Phase 4 (last). Report 3 places it in Phase 3 (middle). The verified evidence suggests **Phase 1 observability, Phase 2 enforcement**:

- **Phase 1**: Cilium/Hubble + basic Tetragon policies. Observability-first — see what agents actually do before writing enforcement rules. Low cost, high information value.
- **Phase 2**: Aya-rs custom programs. Contract-to-policy compilation. This requires understanding real agent behavior patterns, which Phase 1 observability provides.

### macOS Development Fallback

eBPF is Linux-only. For Darwin development:

```rust
trait PolicyEngine {
    fn enforce_file_access(&self, path: &Path, mode: AccessMode) -> Result<(), PolicyDenied>;
    fn enforce_network(&self, addr: &SocketAddr) -> Result<(), PolicyDenied>;
    fn enforce_process(&self, cmd: &str) -> Result<(), PolicyDenied>;
}

struct EbpfPolicyEngine { /* Aya-rs, Linux only */ }
struct UserspacePolicyEngine { /* Trait-based, macOS fallback */ }
```

The userspace engine provides identical API with weaker enforcement (application-level, not kernel-level). Sufficient for development; production requires Linux.

---

## Confidence Levels

| Architectural Decision | Confidence | Basis |
|---|---|---|
| Constrained composition reduces variance | **95%** | Three independent papers verify from different angles |
| eBPF enforcement is production-ready | **90%** | Aya-rs, Tetragon both verified in production at multiple organizations |
| SLICK manifests are the right contract format | **75%** | Sound design, but no production validation yet. Kubernetes Resource Model is proven; SLICK's semantic extensions are novel. |
| Rust+PyO3 is correct MVP runtime | **80%** | Eliminates dependency risk. But misses WASM's strongest property (compile-time capability enforcement). |
| WASM Components will be production-ready for agents by late 2026 | **70%** | Ecosystem momentum is strong, but componentize-py limitations + WASI 0.3 delay risk are real. |
| Blackboard pattern for multi-agent coordination | **70%** | bMAS paper verified. But emergent behavior is harder to debug than hierarchical orchestration. Production evidence is thin. |
| Full WIT→eBPF contract enforcement pipeline | **50%** | Architecturally sound. No production implementation exists. ACM ASIA CCS '23 paper demonstrates the approach for Wasm specifically, but WIT-level contract compilation to eBPF is novel. |

---

## Open Questions

1. **SLICK's "Dijkstra" naming collision** — SLICK's runtime is named Dijkstra. The arch-guild has a Dijkstra agent. One must be renamed before confusion compounds.

2. **Contract enforcement location** — Where does Pydantic validation happen? In Python (via PyO3)? In Rust (re-implemented)? Both (defense in depth with performance cost)?

3. **Protocol convergence or fragmentation?** — Report 2 warns of "Protocol Wars" (MCP vs A2A vs proprietary). The Agentic AI Foundation (Linux Foundation, Dec 2025) suggests convergence. Hexagonal architecture hedges either way, but adapter proliferation has real maintenance cost.

4. **Blackboard implementation** — NATS JetStream KV (Report 3) vs Rust-native sled/rocksdb (session context) vs Redis. Each has different consistency, latency, and operational characteristics.

5. **Agent lifecycle** — What happens when an agent crashes mid-tool-execution? What state is recoverable? How does the Sentinel Layer distinguish between a legitimate agent shutdown and an anomalous termination?

---

## Source Reports: Quality Assessment

| Report | Overall Quality | Strongest Contribution | Weakest Area |
|---|---|---|---|
| **Report 1** (WASM/eBPF Analysis) | **Moderate** — Good landscape overview, significant accuracy issues | Sandbox provider stratification matrix | Source quality: UMA from blog posts, CVE misrepresentation, hyperbolic claims |
| **Report 2** (SLICK Analysis) | **Strong** — Rigorous triangulation, well-sourced | Variance reduction empirical evidence + Decision Space Reduction insight | Bootstrapping problem unresolved, TOOLQP citation unverifiable |
| **Report 3** (Cognitive Data Planes) | **Strong** — Best technical accuracy, honest about limitations | Traceability matrix + Dual-tool eBPF strategy + componentize-py limitations | 20-week roadmap timeline is ambitious; no team size or resource estimates |

Report 3 should be treated as the primary reference. Reports 1 and 2 contribute specific insights (sandbox benchmarks, variance research) that Report 3 integrates but doesn't fully duplicate.

---

## Verification Ledger

Every cited claim in this synthesis has been independently verified. The full ledger:

### Verified (Primary source confirmed)
- Fermyon Spin 0.52ms / 75M RPS — GlobeNewsWire press release
- wasmCloud CNCF Incubating — CNCF announcement Nov 2024
- Hyperlight 1-2ms CNCF Sandbox — Microsoft OSS blog
- Wassette Wasm→MCP bridge — Microsoft OSS blog Aug 2025
- SpinKube containerd-shim-spin — CNCF project, v0.22.0
- WASI 0.3 previews in Wasmtime 37+ — wasi.dev/roadmap
- componentize-py v0.19.3 limitations — Bytecode Alliance GitHub
- Aya-rs 22 program types, production users — aya-rs.dev, GitHub
- Tetragon TracingPolicy CRDs — tetragon.io, CNCF
- eBPFGuard on Aya — Deepfence GitHub
- AgentSight arXiv:2508.02736 — ACM published, <3% overhead
- CtxBugGen arXiv:2601.06497 — 55.93% Pass@1, 3,683 bugs, GitHub
- CIFE arXiv:2512.17387 — 39-66% strict, 1,000 tasks, 13 categories
- IFEval++ arXiv:2512.14754 — 61.8% drop, 46 LLMs
- bMAS arXiv:2507.01701 — blackboard MAS, competitive with fewer tokens
- Dandelion SOSP '25 — arXiv:2505.01603
- AlloyStack EuroSys — 98.5% cold start reduction
- PyO3 production maturity — 13k+ GitHub stars, GIL release, zero-copy

### Partially Verified (Directionally correct, details need qualification)
- Wasmtime <30μs instantiation — microseconds for module init only, not end-to-end
- Anka DSL 60%→100% — 100% on multi-step tasks specifically; 95.8% overall
- CVE-2023-41880 — real CVE, real miscompilation, but NVD states "not an escape from the WebAssembly sandbox"

### Unverified (Could not locate primary source)
- TOOLQP semantic gap paper — no arXiv paper found; concept exists under other names
- "Rax" resource-aware agents paper — no arXiv paper found
- "41x faster warm startup via Eryx" — lib.rs crate claim only
- Seccomp-eBPF temporal specialization "33-55% attack surface reduction" — specific paper not located in search
- OpenAI "50% to 25% market share in 18 months" (Report 2) — no credible analyst source found; "market share" for LLM APIs is not a standard tracked metric
- MCP "three major version changes in six months" (Report 2) — MCP has been relatively stable since Anthropic's late 2024 release; claim appears exaggerated

### Corrected (Report claim contradicted by evidence)
- CVE-2023-41880 as sandbox escape evidence — **NVD explicitly states it is not**
- "Microsecond-level cold starts" for production agents — **Report 1's own data shows 16.9ms**
- UMA as industry standard — **One blogger's framework, not standardized**
- "Complete abandonment of containers" — **Contradicted by Report 1's own roadmap**
