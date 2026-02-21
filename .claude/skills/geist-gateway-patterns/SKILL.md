# Gateway Patterns

Architectural judgment for gateway infrastructure: Gateway API, xDS, proxyless gRPC, control planes. Grounded in geist.sh's architecture where rumi compiles both HTTP routes and agent policies through the same matcher engine.

## Contents

- [Core Mental Model](#core-mental-model)
- [Gateway API](#gateway-api)
- [xDS Protocol](#xds-protocol)
- [Proxyless gRPC](#proxyless-grpc)
- [Control Plane Patterns](#control-plane-patterns)
- [geist.sh Mapping](#geistsh-mapping)
- [Decision Frameworks](#decision-frameworks)

---

## Core Mental Model

geist.sh is a Gateway API implementation that governs two domains through one enforcement point:

```
                    geist-edge
                    ┌──────────────────────────────┐
HTTP/gRPC traffic ─→│  rumi + HttpMessage inputs    │─→ upstream
                    │                              │
Agent tool calls ──→│  rumi + AgentOp inputs        │─→ tool execution
                    └──────────────────────────────┘
                    Policy: Gateway API CRDs for both
```

Same rumi matcher engine. Same `DataInput` + `FieldMatcher` + `MatcherTree` core. Different domain inputs registered:

| Domain | Context Type | Inputs | Compiles From |
|--------|-------------|--------|---------------|
| HTTP | `HttpMessage` | path, method, header, query, scheme, authority | `HttpRouteMatch` |
| Agent | `AgentOp` | agent_id, tool_name, resource, operation, session_id, metadata | `AgentOpMatch` |

The positioning: **MCP = socket, A2A = network, geist.sh = firewall.**

---

## Gateway API

Role-oriented resource model for network infrastructure.

### Resource Hierarchy

```
GatewayClass (cluster) ─ defines controller
    └─ Gateway (namespace) ─ binds listeners to addresses
        └─ *Route (namespace) ─ maps traffic to backends
            └─ Service/Backend ─ upstream targets
```

### Policy Attachment (GEP-713)

The extension mechanism. Custom CRDs that attach policy to any resource:

```yaml
apiVersion: your.domain/v1alpha1
kind: SomethingPolicy              # MUST end in "Policy"
metadata:
  labels:
    gateway.networking.k8s.io/policy: "direct"
spec:
  targetRef:
    group: gateway.networking.k8s.io
    kind: Gateway
    name: my-gateway
    sectionName: my-listener       # optional granularity
status:
  conditions:
    - type: Accepted
      reason: Accepted             # or Conflicted, Invalid, TargetNotFound
```

| Type | Scope | Use |
|------|-------|-----|
| **Direct** | Affects only targeted object | Rate limits, auth on a specific route |
| **Inherited** | Cascades down hierarchy | Default TLS settings on a gateway |

### Key Design Decisions

- **Deny-by-default cross-namespace** — `ReferenceGrant` required for cross-namespace refs
- **Conformance profiles** — testable contract, not just docs
- **Typed extension over annotations** — why Ingress failed
- **Role separation** — infra admin (Gateway) vs app dev (Route)

> **Reference**: `references/gateway-api.md` — full resource YAML, all policy merge strategies, implementation landscape

---

## xDS Protocol

Discovery service for distributing config from control plane to data plane.

### Resource Types

| Service | Discovers | Type URL |
|---------|-----------|----------|
| LDS | Listeners | `envoy.config.listener.v3.Listener` |
| RDS | Routes | `envoy.config.route.v3.RouteConfiguration` |
| CDS | Clusters | `envoy.config.cluster.v3.Cluster` |
| EDS | Endpoints | `envoy.config.endpoint.v3.ClusterLoadAssignment` |
| SDS | Secrets/certs | `envoy.extensions.transport_sockets.tls.v3.Secret` |
| ECDS | Extension configs | `envoy.config.core.v3.TypedExtensionConfig` |

### Dependency Ordering (non-negotiable)

```
CDS ─→ EDS ─→ LDS ─→ RDS
```

CDS before EDS (need cluster to add endpoints). LDS before RDS (need listener to add routes). Violation causes dangling references.

### Transport Variants

| Variant | Best For |
|---------|----------|
| **SotW (State of the World)** | Simplicity, < 1K resources |
| **Delta/Incremental** | Scale (10K+ endpoints), bandwidth efficiency |
| **ADS (Aggregated)** | Cross-type consistency on single stream |

### ACK/NACK Flow

```
Control Plane              Data Plane
     │                         │
     │──── DiscoveryResponse ─→│  (version: "v1", nonce: "abc")
     │                         │
     │←── DiscoveryRequest ────│  ACK: version_info="v1", nonce="abc"
     │                         │       (no error_detail)
     │──── DiscoveryResponse ─→│  (version: "v2", nonce: "def")
     │                         │
     │←── DiscoveryRequest ────│  NACK: version_info="v1", nonce="def"
     │                         │        error_detail={...}
```

NACK = "I reject v2, still on v1, here's why." Control plane MUST handle gracefully.

> **Reference**: `references/xds-protocol.md` — proto definitions, go-control-plane interfaces, SnapshotCache patterns

---

## Proxyless gRPC

gRPC client embeds xDS client directly — no sidecar proxy.

### When to Use

| Proxyless | Sidecar Proxy |
|-----------|---------------|
| Pure gRPC services | Mixed protocols (HTTP + gRPC + TCP) |
| Latency-critical (~0.2ms vs ~2ms) | Advanced L7 filters needed |
| Resource-constrained | Full observability required |
| Go / Java services | Polyglot environment |

### What gRPC Supports via xDS

| Feature | Supported |
|---------|-----------|
| Client-side LB (round_robin, ring_hash) | Yes |
| Retry policies | Yes |
| Circuit breaking (max_requests only) | Partial |
| mTLS via certificate providers | Yes |
| RBAC authorization | Yes |
| Fault injection | Yes |
| Rate limiting, Wasm, ext_proc | No |
| L7 metrics/tracing | No (need proxy) |

### Architecture

```
┌─────────────────────────┐
│  gRPC Application       │
│  ┌───────────────────┐  │
│  │ xDS Client        │  │  xds:///service-name
│  │ (built into gRPC) │──┼──────────────────────→ Control Plane
│  └───────────────────┘  │                         (istiod, etc.)
│  ┌───────────────────┐  │
│  │ istio-agent       │  │  Certificate rotation
│  │ (~25 MiB)         │──┼──────────────────────→ CA (Citadel)
│  └───────────────────┘  │
└─────────────────────────┘
```

> **Reference**: `references/proxyless-grpc.md` — bootstrap config, language support matrix, Istio integration

---

## Control Plane Patterns

### Universal Responsibilities

Every control plane does four things:

```
Ingest ─→ Translate ─→ Distribute ─→ Validate
(CRDs)    (to xDS)     (to proxies)   (status)
```

### Implementation Landscape

| Control Plane | xDS Server | Key Pattern | Scale Approach |
|---------------|-----------|-------------|----------------|
| **Istio** | Custom (istiod) | PushContext + debounce | Per-proxy generation |
| **Envoy Gateway** | go-control-plane SnapshotCache | IR translation pipeline | Snapshot versioning |
| **Cilium** | Per-node xDS over UDS | eBPF + CiliumEnvoyConfig | Node-local, no central bottleneck |
| **Linkerd** | gRPC streaming (no xDS) | Rust proxy, minimal config | Destination service |
| **Consul** | Custom (Raft-backed) | Blocking queries | Datacenter federation |

### State Management

- **Eventual consistency** — control planes are eventually consistent by design
- **Level-triggered** — reconciliation loops are idempotent, not event-driven
- **Coalescing** — batch changes, debounce (Istio: 100ms min, 10s max)

### Extensibility Spectrum

```
Least invasive ────────────────────────── Most invasive
ext_authz → ext_proc → Wasm → Native C++ filter
```

`ext_authz` = external authorization callout. **This is geist-policy's analog** — the PDP is an ext_authz-style decision point.

> **Reference**: `references/control-plane-patterns.md` — architecture diagrams, deployment topologies, agent governance mapping

---

## geist.sh Mapping

The service mesh → agent governance analogy:

| Service Mesh | geist.sh | Role |
|-------------|----------|------|
| Envoy proxy | Agent runtime (shell) | Data plane — enforces policy |
| xDS config | PolicyRuleset | Distributed configuration |
| ext_authz callout | geist-policy PDP | Per-request policy decision |
| `HttpRouteMatch` | `AgentOpMatch` | Request/operation matching |
| SecurityPolicy CRD | AgentPolicy CRD | Policy declaration |
| Control plane (istiod) | geist control plane | Config translation + distribution |
| Sidecar / ambient | Shell runtime | Interception mode |

### AgentPolicy as Gateway API Extension

```yaml
apiVersion: geist.sh/v1alpha1
kind: AgentPolicy
metadata:
  name: restrict-tools
  labels:
    gateway.networking.k8s.io/policy: "direct"
spec:
  targetRef:
    group: geist.sh
    kind: Agent
    name: my-agent
  deny:
    - reason: "No raw shell access"
      matches:
        - toolName: { exact: "Bash" }
  allow:
    - matches:
        - toolName: { exact: "Read" }
        - toolName: { exact: "Grep" }
```

### Where Agent Governance Exceeds Mesh Patterns

1. **Three-valued lattice** — Deny > Escalate > Allow (mesh is binary allow/deny)
2. **Temporal composition** — individually-valid actions composing into harmful sequences
3. **Self-composition** — agent modifying its own policy (no mesh proxy does this)
4. **Session context** — stateful across a conversation, not per-request

### Deployment Topologies

| Mode | Analog | When |
|------|--------|------|
| **Library** (geist-policy in-process) | Proxyless gRPC | Lowest latency, Claude Code hook |
| **Sidecar** (geist-edge per-agent) | Envoy sidecar | Per-agent isolation |
| **Gateway** (shared geist-edge) | Ingress gateway | Multi-agent, central policy |

---

## Decision Frameworks

### Policy Distribution: When to Use What

| Scale | Approach | Implementation |
|-------|----------|----------------|
| Single agent, local | In-process library | `geist-policy` crate directly |
| Few agents, one host | gRPC streaming | Custom, no xDS overhead |
| Fleet, Kubernetes | xDS via go-control-plane | Full Gateway API conformance |

### Protocol Selection

| Need | Choice |
|------|--------|
| Agent-to-agent gRPC, latency-critical | Proxyless gRPC |
| Mixed protocols, full observability | Sidecar (geist-edge) |
| External traffic ingress | Gateway mode |
| Agent tool governance only | In-process PDP (no proxy needed) |

### Gateway API Extension Checklist

When creating a new policy CRD for geist.sh:

- [ ] Kind ends with `Policy`
- [ ] Has `targetRef` with group/kind/name
- [ ] Status includes `Accepted` condition with standard reasons
- [ ] Label: `gateway.networking.k8s.io/policy: "direct"` or `"inherited"`
- [ ] Deny-by-default for cross-namespace references
- [ ] Compiles to rumi matcher config (same engine, different inputs)

---

## References

| Topic | Load |
|-------|------|
| Gateway API resource model, policy attachment, implementations | [gateway-api.md](references/gateway-api.md) |
| xDS protocol family, transport, go-control-plane | [xds-protocol.md](references/xds-protocol.md) |
| Proxyless gRPC architecture, language support, limitations | [proxyless-grpc.md](references/proxyless-grpc.md) |
| Control plane architectures, state management, agent mapping | [control-plane-patterns.md](references/control-plane-patterns.md) |

## Cross-References

- **geist-rust-mastery** — Rust architectural judgment for implementing gateway/PDP
- **rumi-http** (`x.uma/rumi/ext/http/`) — HTTP domain inputs, Gateway API HttpRouteMatch compiler
- **geist-policy** (`geist/policy/`) — Agent domain inputs, AgentOp policy evaluator
