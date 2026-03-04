# geist.sh Architectural Decision Record

> **Date**: February 13, 2026
> **Status**: Research-complete, pre-implementation
> **Scope**: Decisions made, evidence supporting them, what's verified, what's not, and what remains open
> **Sources**: Same corpus as `synthesis-agentic-data-plane.md`

---

## ADR-001: Rust + PyO3 + eBPF over WASM for MVP

### Decision

Build the MVP runtime in Rust with embedded Python via PyO3. Use eBPF (Aya-rs) for kernel-level observability and enforcement. Do not depend on WASM or the Component Model for MVP.

### Context

Three research reports arrived at different conclusions:
- Report 1: Full WASM (Wasmtime), eBPF as Phase 4
- Report 2: Pure Python with lightweight runtime, protocol-agnostic
- Report 3: WASM Components + eBPF, most technically rigorous

### Evidence For

| Factor | Evidence | Level |
|--------|----------|-------|
| Full Python ecosystem immediately | PyO3 is mature (13k+ GitHub stars), GIL release, zero-copy numpy interop | **Strong** |
| No WASI 0.3 dependency risk | WASI 0.3 "expected" ~Feb 2026, not shipped. Building MVP on "expected" standards is a dependency risk | **Strong** |
| componentize-py has real limitations | No threading, partial C extensions, no sys.exit, no production evidence for AI agent workloads | **Strong** |
| eBPF works regardless of runtime | uprobes/kprobes attach to processes, not to Wasm runtimes specifically | **Strong** |
| Aya-rs production-ready | 22 eBPF program types, pure Rust, BTF/CO-RE. Production at Deepfence, Red Hat (bpfman), Exein (Pulsar) | **Strong** |

### Evidence Against

| Factor | Evidence | Level |
|--------|----------|-------|
| No compile-time capability enforcement | WASM's strongest security property — architectural absence makes unauthorized operations inexpressible | **Strong** |
| Process-level isolation is weaker | Requires eBPF/seccomp to compensate for lack of Wasm SFI | **Strong** |
| Misses wasmCloud distributed composition | wRPC/NATS lattice is the closest thing to constitutive composition | **Moderate** |
| Doesn't participate in Wasm Component ecosystem | Wassette, SpinKube, wasi-cloud-core pass us by | **Moderate** |

### Migration Path

**Phase 1 (MVP)**: Rust + PyO3 + eBPF. Ship a working data plane. Use behavioral contracts for semantic enforcement. eBPF provides observability and policy enforcement.

**Phase 2 (Evaluation)**: When WASI 0.3 ships and componentize-py matures, evaluate Wasm Components as a *guest format* within the Rust runtime. WIT interface definitions from Phase 1 should map cleanly to Wasm Component interfaces — this is free if interfaces are typed and capability-scoped from the start.

**Phase 3 (Distribution)**: If Wasm Components prove viable, adopt wasmCloud's lattice model for distributed composition. The Rust runtime becomes a wasmCloud host with custom capability providers.

### Risk

The biggest risk is that WASM matures faster than expected. If componentize-py becomes production-ready and WASI 0.3 ships on time, the Rust+PyO3 choice loses its primary advantage while missing WASM's strongest property. The mitigation is designing WIT-compatible interfaces from day one.

### Confidence: **80%**

---

## ADR-002: SLICK Behavioral Contracts as Agent Skills Extensions

### Decision

SLICK's behavioral contract concepts (Design-by-Contract for AI components) become `slick:` namespace extensions in Agent Skills frontmatter. SLICK does not ship as a standalone spec or project. Enforcement lives in geist.sh runtime, not in the description format.

### Context

SLICK was proposed as a standalone semantic component kit. Internal review (unanimous CONCERN) concluded: SLICK's insight is validated, SLICK as a standalone project is not.

### What's Dead

| Cut | Why |
|-----|-----|
| SLICK as standalone spec | Competing with MCP (97M) + Agent Skills (27+ adopters) is resource suicide |
| 7-kind taxonomy as developer-facing choice | Start with one kind (Component). Let runtime infer the rest |
| YAML manifest as primary authoring surface | Generate from code. Never ask humans to write it |
| Dijkstra runtime inside SLICK spec | Layer violation. Runtime belongs in geist.sh, not in a description spec |
| SLICK as MCP extension | MCP is a wire protocol. Executable contracts are a runtime concern. Mixing violates Interface Segregation |

### What Survives (as Agent Skills extensions)

```yaml
# Standard Agent Skills frontmatter
name: document-summarizer
description: "Compress text. Use when: summarizing documents."

# SLICK behavioral extensions
slick:
  safety_level: read-only
  idempotent: true
  postcondition: "len(output) < len(input)"
  invariants:
    - "input.language == output.language"
```

Any consumer that doesn't understand `slick:` ignores it. Existing Agent Skills tooling remains compatible.

### Three-Layer Architecture

```
SLICK (Behavioral Contracts) — extends frontmatter
Agent Skills (Portable Format) — the canonical format
cix (Distribution)            — the extension marketplace
```

### Evidence

- Constrained vocabularies reduce variance: Anka DSL 60% → 95.8% **[Strong — arXiv:2512.23214]**
- Unconstrained LLMs follow strict constraints only 39-66%: CIFE **[Strong — arXiv:2512.17387]**
- Semantic rephrasing causes up to 61.8% performance drops: IFEval++ **[Strong — arXiv:2512.14754]**
- Historical warning: every previous reusable-component initiative that required manual interface description died of friction before automation tooling arrived (CORBA, SOA/WSDL, EJB 2.x) **[Strong — Chesterton's analysis]**
- Key difference this time: the consumer is an LLM, not a human. LLMs derive more value from structured descriptions. But only if descriptions are generated from code, not hand-authored **[Moderate — reasoning by analogy with important caveat]**

### DX Requirement (Ace)

The first experience must be:

```python
@contract(
    post=lambda r, x: len(r) < len(x),
    safety="read-only",
    idempotent=True
)
def summarize(text: str) -> str:
    return llm.complete(f"Summarize: {text}")
```

The manifest is a **build artifact**, not a source artifact.

### Confidence: **75%**

Sound design, but no production validation. The contract shapes may need revision once real agents exercise them.

---

## ADR-003: eBPF Sentinel Layer Design

### Decision

Build a two-tool eBPF enforcement architecture: Cilium/Tetragon for standard observability and baseline security, Aya-rs for novel contract-aware enforcement. Phase observability before enforcement.

### Architecture

| Concern | Tool | Status |
|---------|------|--------|
| Standard network observability | Cilium/Hubble | **Production-ready.** L3-L7 flow visibility, service dependency maps. CNCF Graduated. Zero custom code. |
| Standard runtime security | Tetragon TracingPolicies | **Production-ready.** Kubernetes-aware enforcement via YAML CRDs. Block unauthorized egress, monitor breakout attempts. |
| Contract-aware enforcement | Aya-rs custom programs | **R&D required.** Parse contracts at deploy time, generate per-component eBPF maps, enforce at kernel level. |
| Intent-action correlation | Aya-rs uprobes + kprobes | **R&D required.** uprobes on SSL_read/write for intent, kprobes on execve/openat2 for actions. Correlation engine is the hard part. |

### The Novel Components (Honestly Assessed)

**WIT/SLICK → BPF-LSM Translation**

This is the core technical novelty. The mechanism:
1. Parse behavioral contracts (safety_level, postconditions) at deploy time
2. Generate eBPF LSM programs that attach to relevant kernel hooks
3. Store policies in eBPF maps keyed by module identity (thread ID or cgroup)
4. Enforce at kernel level: return -EPERM for syscalls that exceed contract scope

**What exists**: BPF-LSM can enforce syscall policies (proven). Aya-rs can deploy BPF-LSM programs (proven). WIT manifests describe capabilities (proven). Deepfence's eBPFGuard provides a practical starting point (Aya-based LSM hooks as Rust/YAML-configurable policies).

**What doesn't exist**: The translation layer. No citation to any project that implements contract-to-LSM translation. This is security-critical custom infrastructure.

**The semantic translation problem** — the critique document's most important finding:
- Contract: "can read files matching `/data/*.json`"
- LSM hook: intercepts `security_file_open()` with inode and path
- Gap: path canonicalization (symlinks, `..` resolution), inode-to-path mapping (expensive, racy), policy compilation per manifest, deployment mechanism to kernel

**Verdict**: Theoretical but plausible. Legitimate R&D territory. **Not off-the-shelf.** **[Moderate — proven primitives, novel integration]**

**Intent-Action Correlation Engine**

AgentSight (arXiv:2508.02736) demonstrates the approach: dual-probe boundary tracing with <3% overhead. But the correlation engine — temporally and semantically matching LLM intent parameters against actual syscall arguments — is hand-waved in all source documents.

Open questions:
- What's the algorithm for correlating intent to action across async operations?
- How does it handle cases where intent is expressed in natural language?
- What's the false positive rate when legitimate operations match malicious patterns?
- How does it scale with parallel tool invocations?

**Verdict**: Research prototype level. The <3% overhead claim is AgentSight's, not validated for geist.sh. **[Weak — concept proven, production viability unknown]**

### Hype Corrections

| Claim in sources | Reality |
|-----------------|---------|
| "Hardware-enforced kernel law" | Kernel-enforced, not hardware-enforced. BPF-LSM runs in kernel software, not TPM/SGX. |
| "Perfect workload isolation" | No security system is perfect. BPF-LSM can be bypassed via kernel vulnerabilities. TOCTOU races exist. |
| "Automatically translates WIT to LSM" | Translation layer doesn't exist. This is custom infrastructure development. |
| "Immutable policies" | eBPF programs can be unloaded. Not immutable in any strict sense. |

### Phasing

**Phase 1**: Cilium/Hubble + basic Tetragon policies. Observability-first — see what agents actually do before writing enforcement rules. Low cost, high information value.

**Phase 2**: Aya-rs custom programs. Contract-to-policy compilation. Requires understanding real agent behavior patterns, which Phase 1 observability provides.

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

Userspace engine provides identical API with weaker enforcement. Sufficient for development; production requires Linux.

### Engineering Reality

The architectural critique estimates **12-24 months with 3-5 senior engineers** (Rust, eBPF, distributed systems expertise) for the full Sentinel Layer. Breakdown:

- SSL interception: 1-2 months (proven technique)
- Correlation engine: 2-4 months (custom logic)
- WIT → LSM translation: 3-6 months (security-critical, needs formal verification)

### Confidence

| Component | Confidence |
|-----------|-----------|
| Cilium/Tetragon baseline | **90%** — production-ready, well-documented |
| Aya-rs custom enforcement | **70%** — proven library, novel application |
| WIT → BPF-LSM translation | **50%** — architecturally sound, no implementation exists |
| Intent-action correlation | **40%** — concept proven, production viability unknown |

---

## Verification Ledger

### Confirmed (primary source verified)

| Claim | Source | Verification |
|-------|--------|-------------|
| Fermyon Spin 0.52ms / 75M RPS on Akamai | Report 3 | GlobeNewsWire press release, Nov 2025 |
| wasmCloud CNCF Incubating | Reports 1, 3 | CNCF announcement Nov 2024 |
| Hyperlight 1-2ms, CNCF Sandbox | Report 3 | Microsoft OSS blog, Feb 2025 |
| Wassette Wasm→MCP bridge, v0.3.4 | compass-dfbd | Microsoft OSS blog, Aug 2025 |
| SpinKube containerd-shim-spin v0.22.0 | Report 3 | CNCF project, Nov 2025 |
| WASI 0.3 previews in Wasmtime 37+ | Report 3 | wasi.dev/roadmap |
| componentize-py v0.19.3, limitations documented | Report 3 | Bytecode Alliance GitHub |
| Aya-rs 22 program types, production users | Reports 1, 3 | aya-rs.dev, Deepfence, Red Hat |
| Tetragon TracingPolicy CRDs, v1.6.0 | Report 3 | tetragon.io, CNCF |
| eBPFGuard on Aya | Synthesis | Deepfence GitHub |
| AgentSight <3% overhead | Reports 1, 3 | arXiv:2508.02736, ACM published |
| CtxBugGen 55.93% Pass@1 | Report 2 | arXiv:2601.06497, 3,683 bugs |
| CIFE 39-66% strict constraint adherence | Report 2 | arXiv:2512.17387, 14 models |
| IFEval++ up to 61.8% drop from rephrasing | Report 2 | arXiv:2512.14754, 46 LLMs |
| Anka DSL 95.8% overall accuracy | Report 2 | arXiv:2512.23214 (100% on multi-step tasks only) |
| bMAS blackboard competitive with fewer tokens | Report 3 | arXiv:2507.01701 |
| MCP 97M monthly SDK downloads, Linux Foundation | compass-dfbd | Multiple industry sources |
| A2A 150+ organizations, v0.3 | compass-dfbd | Linux Foundation records |
| DeepMind 180 configurations, R²=0.513 | compass-dfbd | arXiv:2512.08296 |
| Dennis & Van Horn capability formalization (1966) | compass-52c0 | CACM 9(3), March 1966 |
| Hardy's Confused Deputy (1988) | compass-52c0 | ACM SIGOPS 22(4) |
| Miller's ocap model (2006) | compass-52c0 | Johns Hopkins PhD thesis |
| seL4 machine-checked proofs | compass-52c0 | ACM SOSP 2009, Best Paper |
| wRPC typed peer-to-peer via WIT over NATS | compass-dfbd | Bytecode Alliance project |
| PyO3 maturity | Synthesis | 13k+ GitHub stars, GIL release |

### Corrected (source claim contradicted by evidence)

| Claim | Source | Correction |
|-------|--------|-----------|
| CVE-2023-41880 as sandbox escape | Report 1 | NVD explicitly states "not an escape from the WebAssembly sandbox" |
| "Microsecond-level cold starts" for production agents | Report 1 | Report 1's own data shows 16.9ms. Spin achieves 0.52ms. |
| UMA as industry standard | Report 1 | One blogger's framework (Enrico Piovesan, Medium). Not standardized. |
| "Complete abandonment of containers" | Report 1 | Contradicted by Report 1's own Phase 2 (Kata fallback), SpinKube, hybrid trajectory |
| Anka DSL "60% to 100%" | Report 2 | 100% on multi-step pipeline tasks specifically; 95.8% overall |
| MCP "three major version changes in six months" | Report 2 | MCP has been relatively stable since late 2024. Claim appears exaggerated. |
| OpenAI "50% to 25% market share" | Report 2 | No credible analyst source found. "Market share" for LLM APIs is not a standard metric. |
| Blaurock β=0.507 as "process control" finding | compass-dfbd | β=0.507 measures effect of "strong" CI systems on perceived outcome responsibility, not process control as standalone variable |

### Unverified (primary source not located)

| Claim | Source | Status |
|-------|--------|--------|
| TOOLQP semantic gap paper | Report 2 | No arXiv paper found. Concept exists under other names (GRETEL, TOOLCERT). |
| "Rax" resource-aware agents paper | Report 1 | No arXiv paper found |
| "41x faster warm startup via Eryx" | Report 1 | lib.rs crate claim only |
| Seccomp-eBPF "33-55% attack surface reduction" | Report 3 | Specific paper not located |

---

## Open Questions

### Architecture

1. **Internal data flow model.** How does data move within a geist instance? Between the LLM loop, state store, tool execution, and protocol adapters? None of the reports specify this.

2. **State management architecture.** What is ephemeral vs persistent? What survives restarts? Durable Objects (10 GB SQLite), NATS JetStream KV, and Rust-native sled/rocksdb are all options with different consistency/latency/operational characteristics.

3. **Multi-tenancy.** No treatment of tenant isolation, resource quotas, or billing boundaries.

4. **macOS development story.** eBPF is Linux-only. The `PolicyEngine` trait abstraction provides identical API with weaker enforcement for development. Is this sufficient?

5. **HUD control plane.** How does configuration reach geist instances? Push vs pull? Format? Security?

6. **Cost model.** No analysis of memory overhead per agent instance or compute cost at scale.

### SLICK Integration

7. **Contract enforcement location.** Where does Pydantic validation happen? In Python (via PyO3)? In Rust (re-implemented)? Both (defense in depth with performance cost)?

8. **Bootstrapping.** Who builds the first 100 components? Strategy: wrap existing MCP servers → wrap existing Python libraries → core primitives first → prove the pipeline with one real workflow. But this is a plan, not evidence it works.

9. **Contract shapes.** Are safety_level/postcondition/invariant the right primitives? No production validation yet.

### Sentinel Layer

10. **Correlation algorithm.** How does the correlation engine match intent to action across async operations where temporal separation is large?

11. **False positive rate.** What happens when legitimate operations match "suspicious" patterns? No data on expected false positive rates.

12. **Policy update lifecycle.** How are BPF-LSM policies updated when contracts change? Hot-reload? Restart? What's the blast radius of a bad policy?

### Ecosystem

13. **Protocol convergence or fragmentation?** The Agentic AI Foundation suggests convergence. Hexagonal architecture hedges either way. But adapter proliferation has real maintenance cost.

14. **WASM timing.** When does componentize-py become viable? When does WASI 0.3 ship? The Phase 2 evaluation depends on these external timelines.

---

## Validation Criteria

These must be met before geist.sh can claim its value proposition is real.

### Must Demonstrate (MVP)

1. **A working runtime ships before Q3 2026** — a geist instance runs a real agent workflow end-to-end
2. **Behavioral contracts measurably reduce variance** — constrained workflow outperforms unconstrained baseline on the same task (replicating the Anka DSL result at the tool-composition level)
3. **eBPF observability provides actionable data** — Phase 1 Cilium/Hubble + Tetragon shows agent behavior patterns that inform enforcement rules
4. **SLICK-extended Agent Skills manifests remain consumable** by standard Agent Skills tooling (non-SLICK-aware consumers ignore `slick:` namespace)
5. **macOS development is productive** — the `PolicyEngine` trait abstraction doesn't block development velocity

### Should Demonstrate (Post-MVP)

6. **At least one workflow runs with behavioral contracts generating eBPF policies end-to-end** — safety_level → BPF-LSM policy → kernel enforcement
7. **Intent-action correlation works for at least one concrete scenario** — "agent was asked to read file X, kernel confirms it read file X and nothing else"
8. **WIT-compatible interface definitions** from Phase 1 map cleanly to Wasm Component interfaces when evaluated in Phase 2

### Would Be Remarkable

9. **External adoption** of `slick:` namespace by 3+ tools within 6 months of convention being published
10. **Measured false positive rate** for intent-action correlation below 5% on a representative workload
11. **Performance overhead** of full Sentinel Layer (observability + enforcement) below 5% on representative workload
