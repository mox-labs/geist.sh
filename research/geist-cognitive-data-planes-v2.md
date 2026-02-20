# From isolated agents to coordinated cognitive data planes

**The next generation of AI agent infrastructure will not be built on containers.** A convergence of WebAssembly 3.0's Component Model, eBPF-based kernel enforcement, and capability-driven architectures now makes it possible to construct "cognitive data planes" — governed, observable runtime fabrics where agents coordinate through typed interfaces rather than ad-hoc API calls. This white paper presents the Geist.sh thesis: that Wasm components, monitored by Rust eBPF sentinels via Aya-rs, represent a fundamentally superior substrate for agentic workloads compared to the microVM and container-based approaches dominating production today. The case rests on three measurable advantages — **sub-millisecond cold starts** (vs. 100–300ms for microVMs), **capability-scoped isolation** eliminating entire classes of prompt-injection-to-host attacks, and **kernel-level contract enforcement** via eBPF LSM programs that bind WIT interfaces to syscall policy. What follows is a rigorous, source-grounded analysis of the 2026 stack landscape, sandbox patterns, observability layers, and an implementation roadmap for building this architecture.

---

## 1. Executive summary: the Geist.sh thesis

AI agent deployments quadrupled from 11% to **42% of enterprises** between Q2 and Q3 2025. Every major cloud provider — AWS with Bedrock AgentCore, Google with Agent Sandbox on GKE, Microsoft with Agent 365 — now ships dedicated agent isolation primitives. Yet the dominant isolation strategies (Firecracker microVMs, gVisor user-space kernels, Kata Containers) were designed for general-purpose serverless and container workloads, not for the unique characteristics of agentic execution: non-deterministic behavior, multi-turn stateful sessions, dynamic tool invocation, and inter-agent coordination.

The Geist.sh thesis posits that **Wasm Components + eBPF** form a tighter, more appropriate abstraction. Wasm's capability-based model means an agent literally cannot access a resource unless its WIT interface declares it. eBPF LSM hooks provide a second enforcement layer at the kernel boundary, validating that runtime behavior conforms to declared contracts. Together, they create what we term a **coordinated cognitive data plane**: a runtime fabric where agent isolation, inter-agent state sharing, observability, and policy enforcement are unified into a single, standards-based architecture rather than bolted on as separate infrastructure layers.

This is not hypothetical. **Wasmtime v41.0.2** (released February 2026) ships production-ready Component Model support with async primitives. **Wasm 3.0**, released by the W3C in September 2025, standardized Memory64, garbage collection, and tail calls. **WASI 0.3** — adding native `stream<T>` and `future<T>` async types — is expected to stabilize in early 2026. **Microsoft's Wassette** (August 2025) already bridges Wasm Components to the Model Context Protocol, enabling agents to discover and execute sandboxed tools from OCI registries. On the enforcement side, **Aya-rs** provides 22 eBPF program types in pure Rust, including LSM hooks that can enforce per-module access policies with deep kernel state inspection. The pieces exist. This paper maps how they fit together.

---

## 2. The 2026 isolation landscape: Wasm is 1,000× faster at cold start

The performance delta between WebAssembly and microVM isolation is not incremental — it is architectural. Wasm runtimes eliminate the boot sequence entirely: there is no guest kernel to initialize, no virtual hardware to emulate, no init process to launch.

**Wasmtime standalone cold starts measure under 30 microseconds.** Fermyon's Spin framework demonstrates **0.52ms** cold starts in production on Akamai's edge network, serving 75 million requests per second. By contrast, Firecracker's official specification guarantees boot-to-init in **≤125ms** on bare metal, with practical measurements on EC2 i3.metal instances showing VMM startup at ~12ms plus kernel boot. Kata Containers, adding orchestration layers atop Cloud Hypervisor (~200ms boot), land in the **150–300ms** range. gVisor achieves **50–100ms** by avoiding VM boot entirely, but its syscall-interception architecture introduces 10× per-syscall overhead (~800ns vs. ~70ns native) and severe I/O penalties — disk operations measured up to 216× slower in microbenchmarks.

| Runtime | Cold start | Memory overhead | Isolation model | Security boundary |
|---------|-----------|----------------|-----------------|-------------------|
| **Wasmtime (standalone)** | **<0.03ms** | Few KB per instance | Capability-based, linear memory | Wasm sandbox (compile-time + runtime) |
| **Fermyon Spin** | **0.52ms** | ~10–15 MB per binary | WASI capability scope | Wasm sandbox + WASI permissions |
| **Hyperlight (Wasm+microVM)** | **1–2ms** | Minimal (no guest OS) | Double-layer: Wasm + KVM | VM escape AND Wasm exploit required |
| **gVisor** | **50–100ms** | Low (Sentry + Gofer) | User-space kernel, Systrap | Process-level + seccomp |
| **Firecracker** | **100–125ms** | <5 MiB VMM per VM | KVM hardware virtualization | VM escape required + Jailer defense-in-depth |
| **Cloud Hypervisor** | **~200ms** | Slightly > Firecracker | KVM + more virtual devices | VM escape required |
| **Kata Containers** | **150–300ms** | 128–256 MiB min guest | KVM via configurable VMM | Full VM isolation + OCI compatibility |

The security trade-off is real: microVMs provide hardware-enforced isolation via KVM, requiring both a VMM escape and a KVM bypass — an extremely rare attack chain. Wasm isolation is enforced by the runtime's compiler and validator; an exploit requires finding a bug in Wasmtime itself. **Wasmtime's response** is rigorous fuzzing, formal verification efforts, and its promotion to the Bytecode Alliance's first "Core Project" with dedicated security governance. The hybrid approach — exemplified by Microsoft's Hyperlight (CNCF Sandbox), which wraps Wasm in a micro-VM for **1–2ms** cold starts with double-layer isolation — represents an intermediate point on the security-performance curve.

For agentic workloads specifically, the cold-start advantage compounds: agents frequently spawn sub-agents, invoke tool-execution sandboxes, and scale to zero between requests. At **150,000 agent invocations per second**, the difference between 0.5ms and 125ms startup is the difference between a responsive system and one bottlenecked on sandbox provisioning.

### The snapshot escape hatch

Production microVM platforms mitigate cold-start costs via snapshot/restore. AWS Lambda SnapStart, Firecracker's snapshot mechanism, and the new Kubernetes Pod Snapshots in Google's Agent Sandbox all reduce effective warm-start times to single-digit milliseconds. ETH Zürich's Dandelion system (SOSP '25) demonstrated that snapshot-based Firecracker achieves 8.6× shorter latency than conventional serverless. This narrows the gap considerably — but introduces state management complexity (snapshot freshness, memory overhead of pre-warmed pools) that Wasm's instant-start model avoids entirely.

---

## 3. Wasm Components as capabilities, not services

The **Wasm Component Model** — Phase 1 in W3C standardization but enabled by default in Wasmtime since v37 — fundamentally restructures how software modules compose. Unlike containers (which share a kernel and communicate via network), or microservices (which serialize data across HTTP boundaries), Wasm Components interact through **typed, language-neutral interfaces defined in WIT (WebAssembly Interface Types)**. Each component declares exactly what it imports and exports; the runtime enforces these contracts at instantiation time.

This maps directly to a capability-based security model. A component that declares `import wasi:filesystem/types` can access the filesystem; one that does not literally lacks the ability to attempt a file operation. There is no ambient authority — **the absence of a capability declaration is the enforcement mechanism itself**.

**wasmCloud** (CNCF Incubating) has operated on this model since 2019, and its architecture previews what a cognitive data plane looks like in practice. Components containing business logic are connected to **capability providers** (HTTP servers, key-value stores, message brokers) at runtime via WIT-defined links. The **lattice** — a self-forming mesh powered by CNCF NATS — enables distributed composition via **wRPC (WIT over RPC)**, where components on different nodes compose as if co-located. Cosmonic's enterprise deployment demonstrates microsecond-scale cold starts with zero-to-zero scaling on VMware vSphere Kubernetes clusters.

The pattern that researcher Enrico Piovesan terms "Universal Microservices Architecture" (UMA) formalizes this: each service defines a contract (WIT), pure portable logic (Wasm), and runtime adapters for target platforms. While UMA itself is one researcher's framework rather than a Bytecode Alliance standard, it accurately describes the trajectory the ecosystem is on. **The Bytecode Alliance's own toolchain now embodies this pattern**: `cargo-component` for Rust, `componentize-py` for Python, `jco` for JavaScript, and `wit-bindgen` for cross-language binding generation.

### WASI 0.2 through 1.0: the standards roadmap

| Standard | Status | Key interfaces | Timeline |
|----------|--------|---------------|----------|
| **WASI 0.2** | Stable (Jan 2024) | wasi-cli, wasi-http, wasi-filesystem, wasi-sockets, wasi-clocks, wasi-random | Shipped |
| **WASI 0.3** | Previews in Wasmtime 37+ | Native async (`stream<T>`, `future<T>`), simplified wasi-http (5 vs. 11 resource types) | ~Feb 2026 |
| **WASI 0.3.x** | Planned point releases | Cancellation, stream forwarding/splicing, caller-supplied buffers (zero-copy), cooperative then preemptive threads | 2026 |
| **WASI 1.0** | Planned | Full stable standard intended for decades of use | Late 2026 / Early 2027 |

Additional WASI proposals at varying maturity: **wasi-keyvalue** (Phase 2), **wasi-messaging** (Phase 1–2), **wasi-nn** (active, ML inference), **wasi-config** (Phase 1–2). Together, these form the **wasi-cloud-core** bundle targeting common distributed application needs.

---

## 4. Sandbox patterns and agentic data planes

### The "safe-to-try" execution model

The dominant AI agent sandbox pattern follows a cycle drawn from sociocratic decision-making: **isolate → execute → validate → commit or discard**. An agent receives a sandboxed environment, attempts actions (code execution, file manipulation, API calls), and results are validated against policy before any effects escape the sandbox boundary. If validation fails, the entire sandbox is destroyed with zero side effects.

This pattern is now production-standard. E2B implements it via per-session Firecracker microVMs with 24-hour maximum sessions. Google's Agent Sandbox provides Kubernetes-native `SandboxTemplate` and `SandboxClaim` CRDs with WarmPools for sub-second provisioning. AWS Bedrock AgentCore creates per-session microVMs that auto-destroy on session termination.

**The Wasm advantage for safe-to-try is structural.** Because Wasm Components operate under capability-based permissions and linear memory isolation, the "validate" step is partially encoded at compile time. An agent compiled as a Wasm Component that lacks `wasi:filesystem` cannot attempt filesystem access regardless of what the LLM instructs — the sandbox prevents the attempt from being expressible, not merely from succeeding. This is a qualitatively different security property than syscall filtering, which allows the attempt and then blocks it.

### Preventing prompt-injection-to-host escapes

The OWASP LLM Top 10 (2025 edition) ranks prompt injection as the **#1 risk**. Known attack vectors include indirect injection via RAG data poisoning, tool/plugin exploitation for RCE, config file manipulation (.cursorrules, AGENT.md), and agent chain escalation in multi-agent systems.

A defense-in-depth approach layers three enforcement mechanisms:

- **Wasm capability scope** (compile-time): Component cannot express unauthorized operations
- **eBPF LSM hooks** (kernel-level): Runtime validation that syscalls from the Wasm host process conform to declared policies, with deep argument inspection (file paths, socket addresses, process credentials)
- **seccomp-BPF** (process-level): Baseline syscall allowlisting inherited by all child processes

Research from ACM ASIA CCS '23 demonstrated this layering practically: eBPF programs replaced WASI runtime security checks, enabling **fine-grained per-module policies** for filesystem access that go beyond WASI's coarse preopened-directory model. The eBPF programs attach via uprobes to the Wasm runtime, correlating thread IDs with specific module instances to enforce module-specific policies.

### Blackboard vs. hierarchical orchestration for multi-agent coordination

Two architectural patterns dominate multi-agent coordination, each with distinct trade-offs for cognitive data plane design:

| Dimension | Blackboard pattern | Hierarchical orchestration |
|-----------|-------------------|---------------------------|
| **Control flow** | Emergent — agents self-select based on shared state | Top-down — coordinator assigns tasks |
| **State sharing** | Central shared knowledge base (read/write by all) | Passed through coordinator or graph edges |
| **Token efficiency** | Better — only relevant agents participate | Variable — coordinator consumes overhead tokens |
| **Predictability** | Lower — emergent behavior harder to debug | Higher — explicit control flow |
| **Failure mode** | Graceful degradation (other agents continue) | Single point of failure at coordinator |
| **Best for** | Unknown workflows, creative problem-solving | Well-defined decomposable tasks |
| **Production examples** | AWS Strands Arbiter pattern, Confluent/Kafka event-driven agents | LangGraph supervisor nodes, CrewAI role-based crews |

Recent academic work validates the blackboard approach for LLM-based systems specifically: an arXiv paper on blackboard-based MAS (bMAS) showed results **competitive with state-of-the-art multi-agent systems while spending fewer tokens**, because agents self-select for relevance rather than being uniformly invoked.

For a cognitive data plane architecture, the blackboard pattern maps naturally to a **shared Wasm linear memory region** or a **NATS-backed event stream** (as in wasmCloud's lattice). Each agent component reads from and writes to a typed shared state space; the runtime's WIT contracts ensure agents can only access state they're authorized to see.

### Zero-copy state transfer: Memory64 and SharedArrayBuffer

**Memory64** (Phase 4, merged into Wasm 3.0) extends linear memory addressing from 32-bit to 64-bit, enabling individual Wasm instances to address beyond 4 GiB. This is essential for agents working with large context windows, embedding vectors, or document corpora.

For inter-agent state sharing, the **shared-everything-threads proposal** (Phase 1, under active development) would enable Wasm modules to share functions, tables, globals, and GC references across threads — eliminating the `postMessage` copy overhead that currently makes inter-module communication expensive. However, this proposal is years from standardization.

Practical zero-copy approaches available today include:

- **WAMR Shared Heap**: Maps a region of the 32-bit address space to shared native memory; modules use `shared_malloc()`/`shared_free()` for zero-copy buffer sharing across instances
- **Roadrunner** (arXiv, 2025): A runtime shim enabling near-zero-copy, serialization-free data transfer between Wasm functions via shared memory regions, UNIX domain sockets, or memory-mapped files — while maintaining isolation by mediating all access through the shim
- **Wasmtime SharedMemory**: Supports concurrent access with atomic wait/notify operations, though the Component Model's shared-nothing design deliberately trades performance for safety

**The fundamental tension**: the Component Model enforces shared-nothing interoperability by design — values are copied across component boundaries to guarantee memory safety. WASI currently has no shared memory type (GitHub issue WebAssembly/WASI#594 tracks this gap). A cognitive data plane must therefore mediate between the Component Model's safety guarantees and the performance requirements of multi-agent state sharing, likely through runtime-managed shared regions with typed access controls.

---

## 5. Componentize-Py: Python agents in Wasm components

**Componentize-py v0.19.3** (stable, November 2025; canary v0.21.0 February 2026) compiles Python applications to Wasm Components by bundling CPython itself into the Wasm module alongside wasi-libc and native extensions. It generates type-annotated Python bindings from WIT definitions, enabling MyPy-compatible development.

The tool handles a remarkably complex compilation pipeline: it extends LLVM, Rust, wasi-sdk, and CPython, emulating `dlopen`/`dlsym` to allow CPython to load native extensions at runtime within the Wasm sandbox. **NumPy has been demonstrated working** via custom WASI builds.

Critical limitations for agentic workloads:

- **No threading support.** The Wasm Component Model is inherently single-threaded (shared-nothing architecture). Python's `threading` module cannot function within a Wasm component. This means CPU-bound parallel agent workloads must use multiple component instances rather than intra-component parallelism.
- **C extension compatibility is partial.** Packages requiring Fortran (SciPy), complex native builds (OpenBLAS), or system libraries lack WASI wheels. No standard cross-compilation path exists for arbitrary Python native extensions.
- **Import resolution constraints.** Runtime imports must be resolved at the top level; lazy submodule imports may fail (tracked as Issue #23).
- **No `sys.exit` support** (hangs indefinitely, Issue #178).
- **Async is constrained** by the single-threaded environment, though wasi-http supports concurrent I/O patterns.

For a Geist.sh implementation, this means Python-based agents can be compiled to Wasm Components for sandboxed execution, but **computationally intensive or multi-threaded agent logic should remain in Rust** (via `cargo-component`) while Python components handle orchestration, prompt construction, and tool invocation through WIT interfaces.

---

## 6. The Sentinel layer: Rust eBPF via Aya-rs

### Aya-rs custom probes vs. Cilium's standard mesh

**Aya** is a pure-Rust eBPF library (~4.1k GitHub stars) that compiles eBPF programs without depending on libbpf or BCC. It supports **22 program types** including the critical `BPF_PROG_TYPE_LSM` for security enforcement, plus kprobes, uprobes, tracepoints, XDP, TC classifiers, and cgroup hooks. BTF/CO-RE support enables compile-once-run-everywhere portability across kernel versions.

**Cilium** (CNCF Graduated) with **Hubble** provides production-ready L3–L7 network observability — flow visibility, service dependency graphs, DNS monitoring, HTTP/Kafka/gRPC protocol inspection — deployable via Helm with YAML configuration. **Tetragon** (v1.6.0) extends this with runtime security enforcement: TracingPolicy CRDs that define kprobe/tracepoint/LSM hooks with selectors (matchPIDs, matchArgs, matchBinaries) and actions (Sigkill, Override, Signal).

| Dimension | Aya-rs custom probes | Cilium / Hubble / Tetragon |
|-----------|---------------------|---------------------------|
| **Flexibility** | Maximum — any eBPF program type, any hook point | Structured — predefined CRDs, Hubble flow types |
| **Operational overhead** | High — requires Rust/eBPF expertise, custom deployment | Low — Helm install, kubectl, YAML policies |
| **Development cost** | Kernel + user-space code in Rust | Zero code — declarative policies only |
| **Customizability** | Arbitrary kernel inspection logic | Limited to supported selectors and actions |
| **K8s integration** | Manual (build own operators/CRDs) | Native (pod/namespace awareness, label selectors) |
| **Performance tuning** | Full control over filtering and aggregation | <1% overhead claimed; in-kernel filtering |
| **Production track record** | Deepfence, Red Hat (bpfman), Exein (Pulsar) | AWS, Azure, GCP default CNI; CiliumCon conferences |
| **Best fit** | Novel enforcement patterns (WIT contracts) | Standard network observability + security policies |

**For Geist.sh, the answer is both.** Cilium/Tetragon handles standard network observability and baseline security enforcement — detecting unauthorized egress, monitoring inter-agent traffic flows, enforcing pod-level security policies. **Aya-rs handles the novel requirement**: WIT contract enforcement at the kernel level, custom agent-to-agent communication monitoring, and policy-as-code integration specific to Wasm runtime behavior.

### Enforcing WIT contracts at the kernel level

No production system enforces WIT contracts via eBPF today, but the architectural pattern is clear from existing research. The ACM ASIA CCS '23 paper on eBPF-enhanced Wasm sandboxing demonstrated the approach:

1. **Parse WIT contracts at deploy time** to extract allowed host function signatures, resource access patterns, and capability requirements
2. **Generate eBPF LSM programs** that attach to relevant kernel hooks — `file_open` for filesystem WIT functions, `socket_connect` for networking WIT functions, `bprm_check_security` for process execution
3. **Attach uprobes to the Wasm runtime** (Wasmtime) to intercept hostcall invocations and validate conformance to WIT contracts
4. **Store policies in eBPF maps** keyed by module identity (thread ID or cgroup), updatable at runtime without program reload
5. **Enforce at kernel level**: return `-EPERM` for syscalls that exceed WIT contract scope

This is where Aya-rs becomes essential. Tetragon's TracingPolicy CRDs lack the expressiveness to encode WIT contract semantics. A custom Aya-rs program can parse WIT-derived policy from a BPF map, correlate the calling thread with a specific Wasm component instance, and make enforcement decisions based on the component's declared capability set — all within the kernel path, before the syscall completes.

**Deepfence's eBPFGuard** (built on Aya) provides a practical starting point: it exposes LSM hooks (`file_open`, `socket_bind`, `socket_connect`, `bprm_check_security`, `sb_mount`) as Rust/YAML-configurable policies without requiring direct eBPF programming. A Geist.sh Sentinel layer could extend eBPFGuard's model with WIT-aware policy generation.

### Policy-as-code for agentic syscall filtering

eBPF LSM programs are strictly superior to seccomp-BPF for agent sandbox enforcement because they can inspect **deep kernel state** — file paths, socket addresses, process credentials — whereas seccomp can only examine syscall numbers and register arguments without pointer dereferencing. A layered approach maximizes defense:

- **Layer 1 — seccomp-BPF**: Immutable baseline allowlist blocking ~44 dangerous syscalls (Docker default profile), inherited by all child processes
- **Layer 2 — eBPF LSM (Aya-rs)**: Fine-grained argument-level filtering (e.g., `open()` only on `/sandbox/*` paths; `connect()` only to approved endpoints)
- **Layer 3 — Tetragon TracingPolicies**: Kubernetes-aware enforcement scoped to namespaces, pods, and containers, with real-time SIEM integration
- **Layer 4 — OPA / admission control**: API-level policy gating at deployment time

Research on temporal syscall specialization (Seccomp-eBPF, 2023) shows this approach can reduce attack surface by **33–55%** by applying broader allowlists during initialization and restricted lists during serving.

---

## 7. Traceability matrix: architectural needs mapped to 2026 standards

| Architectural need | Technical standard | Implementation | Verification source |
|---|---|---|---|
| **Agent isolation** | WASI 0.2 capability model + Wasm Component Model | Wasmtime v41.0.2 (component model enabled by default) | Bytecode Alliance, Wasmtime stability docs |
| **Sub-ms cold start** | Wasm 3.0 (W3C, Sept 2025) | Fermyon Spin 3.5 on Akamai: 0.52ms measured | Fermyon Wasm Functions GA announcement, Nov 2025 |
| **64-bit memory addressing** | Memory64 (Phase 4, merged into Wasm 3.0) | Wasmtime, Chrome, Firefox ship support | WebAssembly/memory64 GitHub (Phase 4 merged) |
| **Async agent I/O** | WASI 0.3 (`stream<T>`, `future<T>`) | Wasmtime 37+ previews; Fermyon building on WASIp3 | wasi.dev/roadmap, Fermyon blog |
| **Typed inter-component interfaces** | WIT (WebAssembly Interface Types) | wit-bindgen, cargo-component, componentize-py | component-model.bytecodealliance.org |
| **Kernel-level enforcement** | eBPF LSM (Linux 5.7+, `BPF_PROG_TYPE_LSM`) | Aya-rs LSM program type; Tetragon TracingPolicy CRDs | docs.kernel.org/bpf/prog_lsm.html |
| **Wasm runtime syscall filtering** | eBPF uprobes + LSM hooks on Wasmtime | ACM ASIA CCS '23: per-module eBPF policies for Wasm | dl.acm.org/doi/10.1145/3579856.3592831 |
| **Network observability** | Cilium/Hubble eBPF mesh | L3-L7 flow visibility, protocol inspection | docs.cilium.io/en/stable/observability |
| **Runtime security enforcement** | Tetragon TracingPolicy CRDs | kprobes, LSM hooks, in-kernel Sigkill enforcement | tetragon.io/docs |
| **Multi-agent state sharing** | wRPC (WIT over RPC) on NATS | wasmCloud lattice distributed composition | wasmcloud.com/docs/concepts/lattice |
| **Agent-tool protocol** | MCP (Model Context Protocol) | 97M monthly SDK downloads; Wassette bridges Wasm→MCP | Agentic AI Foundation (Linux Foundation, Dec 2025) |
| **Agent-agent protocol** | A2A (Agent2Agent, Linux Foundation) | JSON-RPC 2.0 + SSE; Agent Cards for capability discovery | Google → Linux Foundation donation, June 2025 |
| **Python agent compilation** | componentize-py 0.19.3 | Python 3.10+ → Wasm Component via bundled CPython | github.com/bytecodealliance/componentize-py |
| **Distributed component composition** | wRPC + NATS JetStream | Runtime composition across wasmCloud lattice nodes | Bytecode Alliance wRPC project |
| **Container-Wasm bridge** | SpinKube (CNCF project) | Spin CRDs for Kubernetes; 1,000+ functions per 4-core node | Fermyon/SpinKube, GKE Autopilot integration |

### Grounding proofs for key architectural decisions

**Decision 1: Wasm over microVMs for agent isolation.** Fermyon's production deployment on Akamai demonstrates 0.52ms cold starts at 75M RPS with 99.9% reliability. Wasmtime's Component Model enforces capability-scoped isolation at compile time. Trade-off: weaker hardware isolation boundary — mitigated by Hyperlight's double-layer approach (1–2ms, CNCF Sandbox) or eBPF LSM enforcement at kernel level.

**Decision 2: Aya-rs for custom enforcement, Tetragon for baseline.** Aya-rs's LSM program support (22 program types, pure Rust, BTF/CO-RE portability) enables WIT-contract-aware enforcement that Tetragon's YAML CRDs cannot express. Tetragon provides Kubernetes-native observability with <1% overhead for standard security monitoring. Production validation: Deepfence uses Aya for eBPFGuard, Red Hat for bpfman, CoreWeave uses Tetragon for AI workloads.

**Decision 3: Blackboard pattern for multi-agent coordination.** Academic evidence shows blackboard-based MAS achieves competitive quality with fewer tokens through self-selection. Maps to NATS-backed shared state in wasmCloud's lattice model. WIT contracts define typed access to shared state regions, preventing unauthorized cross-agent data access.

**Decision 4: componentize-py for Python agent components.** Enables the dominant AI/ML ecosystem language to target Wasm Components, despite threading limitations. NumPy demonstrated working. Performance-critical paths route to Rust components; Python handles orchestration through WIT interfaces.

---

## 8. Implementation roadmap: from componentize-py to production Aya-rs cluster

### Phase 1: Agent component compilation (Weeks 1–4)

Define WIT interfaces for agent capabilities: tool invocation (`invoke-tool`), state access (`read-state`, `write-state`), inter-agent messaging (`send-message`, `receive-message`), and LLM inference (`complete`, `embed`). Compile Python agent logic using componentize-py v0.19.3 targeting these interfaces. Build Rust components via cargo-component for performance-critical paths (embedding similarity search, state serialization). Validate component composition using `wasm-tools compose` and test locally with Wasmtime v41.

### Phase 2: Lattice deployment with wasmCloud (Weeks 5–8)

Deploy wasmCloud on Kubernetes via Helm. Configure capability providers for HTTP (wasi-http), key-value state (wasi-keyvalue backed by Redis), and messaging (wasi-messaging backed by NATS). Define Wadm application manifests (OAM format) declaring component topology, link definitions, and scaling policies. Validate distributed composition via wRPC: agent components on different nodes composing through WIT interfaces over the NATS lattice. Implement MCP tool exposure via Wassette for agent-tool discovery.

### Phase 3: Sentinel layer deployment (Weeks 9–12)

Install Cilium as the CNI with Hubble enabled for L3–L7 flow observability across the wasmCloud lattice. Deploy Tetragon with baseline TracingPolicies: block unauthorized egress from agent pods, monitor `sys_mount`/`sys_unshare` for container breakout attempts, enforce read-only root filesystems. Build custom Aya-rs eBPF programs for WIT contract enforcement: parse WIT definitions at component deploy time, generate eBPF maps encoding per-component capability policies, attach LSM hooks (`file_open`, `socket_connect`) and uprobes on Wasmtime hostcall paths. Validate enforcement by attempting unauthorized operations from within agent components and confirming kernel-level denial.

### Phase 4: Cognitive data plane integration (Weeks 13–16)

Implement the blackboard coordination pattern via NATS JetStream key-value buckets, with typed access mediated through WIT interfaces. Configure zero-copy state transfer between co-located agent components using Wasmtime's SharedMemory with atomic wait/notify operations. Deploy the Agentic Gateway pattern: a policy-driven proxy (built as a Wasm Component) that routes inter-agent messages based on capability declarations, enforces per-agent rate limits, and logs all interactions to Hubble for observability. Instrument the full stack with OpenTelemetry traces flowing from Wasm component spans through eBPF kernel events to Hubble network flows, providing end-to-end visibility from agent decision to syscall execution.

### Phase 5: Production hardening (Weeks 17–20)

Enable WASI 0.3 async primitives for streaming agent I/O as Wasmtime stabilizes support. Implement temporal syscall specialization: broader capability sets during agent initialization (dependency loading, model warm-up), restricted sets during inference serving. Configure SpinKube for Kubernetes-native Wasm workload management with autoscaling policies. Establish WarmPools for frequently-used agent component types. Deploy cryptographic component signing via Notation/Cosign for supply chain integrity. Run adversarial testing: prompt injection attempts, sandbox escape vectors, inter-agent privilege escalation, and resource exhaustion attacks.

---

## Conclusion: the substrate determines the architecture

The 2026 agentic infrastructure landscape is bifurcating. The incumbent path — microVMs and containers with bolted-on security — scales by adding layers: Firecracker for isolation, then gVisor for defense-in-depth, then seccomp for syscall filtering, then OPA for admission control, then Tetragon for runtime enforcement. Each layer addresses a gap left by the one below it. The result is operationally complex and architecturally accidental.

The Wasm + eBPF path starts from a different premise: **isolation is a compile-time property, not a runtime addition.** A Wasm Component that lacks a capability cannot attempt the unauthorized action. An eBPF LSM program that enforces WIT contracts validates this property at the kernel boundary. The cognitive data plane — agents coordinating through typed interfaces over a capability-scoped lattice, observed and governed by kernel-level sentinels — is not a stack of compensating controls but an integrated architecture where security, observability, and coordination are the same system.

Three developments make this practical now rather than aspirational. First, **Wasm 3.0 and WASI 0.2/0.3** provide the stable standards foundation that enterprise adoption requires — Wasmtime's LTS releases with 2-year security support signal production readiness. Second, **the Agentic AI Foundation** (Anthropic, OpenAI, Block, under Linux Foundation) is standardizing agent protocols (MCP, A2A, AGENTS.md), creating the interoperability layer that cognitive data planes need. Third, **Aya-rs and Tetragon** have matured eBPF enforcement from research prototypes to production tools deployed at CoreWeave, Deepfence, and Red Hat.

The gap is the integration layer — the system that compiles WIT contracts into eBPF policies, that routes agent messages through capability-aware data planes, that provides end-to-end tracing from prompt to syscall. That gap is what Geist.sh is designed to fill. The technical primitives are proven. The standards are converging. What remains is the engineering to compose them into a coherent whole.