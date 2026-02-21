# Control Plane Architecture Patterns

**Purpose**: Comprehensive reference for control plane architecture patterns across canonical service mesh implementations. For architectural judgment when building policy distribution systems in geist.sh.

**Sources**: Istio (istiod), Envoy Gateway, Cilium, Linkerd, Consul Connect, Envoy xDS protocol, go-control-plane reference implementation, Christian Posta's control plane guidance series.

---

## Table of Contents

1. [Core Concepts](#1-core-concepts)
2. [The Five Canonical Control Planes](#2-the-five-canonical-control-planes)
3. [Architecture Comparison](#3-architecture-comparison)
4. [Control Plane Responsibilities](#4-control-plane-responsibilities)
5. [Data Plane Integration (xDS)](#5-data-plane-integration-xds)
6. [State Management](#6-state-management)
7. [Multi-Tenancy](#7-multi-tenancy)
8. [Extensibility](#8-extensibility)
9. [Operational Patterns](#9-operational-patterns)
10. [The Control Plane for Agents Analogy](#10-the-control-plane-for-agents-analogy)

---

## 1. Core Concepts

### What a Control Plane Does

A control plane translates operator intent into data plane configuration. It is the "brain" that takes high-level declarative policy and pushes concrete, per-instance configuration to every enforcement point.

```
  Operator Intent          Control Plane           Data Plane
  (declarative)        (translation engine)      (enforcement)

  +-----------+        +------------------+      +----------+
  | Policy    |------->| Ingest           |      | Proxy A  |
  | CRDs      |        | Translate        |----->| Proxy B  |
  | API calls |        | Distribute       |      | Proxy C  |
  | Config    |        | Validate         |      | ...      |
  +-----------+        +------------------+      +----------+
```

### The Four Functions

Every control plane, regardless of implementation, performs these four functions:

1. **Ingest** -- Watch external sources (K8s API, config files, APIs) for desired state
2. **Translate** -- Convert high-level config into data-plane-specific configuration
3. **Distribute** -- Push translated config to all connected data plane instances
4. **Validate** -- Ensure config is well-formed before and after application

### Control Plane vs. Data Plane

| Concern | Control Plane | Data Plane |
|---------|--------------|------------|
| Path | Off the request path | On the request path |
| Latency | Seconds acceptable | Microseconds required |
| Consistency | Eventual | Per-request enforcement |
| Failure mode | Config stale, mesh continues | Requests fail |
| State | Global view | Local view (own config) |
| Scaling | Scale for config volume | Scale for traffic volume |

---

## 2. The Five Canonical Control Planes

### 2.1 Istio (istiod)

**Architecture**: Monolithic control plane binary consolidating Pilot, Citadel, and Galley.

**Key characteristics**:
- Single `istiod` binary handles config translation, certificate issuance, and xDS serving
- Watches Kubernetes API server for VirtualService, DestinationRule, Gateway, ServiceEntry CRDs
- Translates to four xDS resource types: CDS, LDS, RDS, EDS
- Maintains PushContext -- an in-memory representation of the entire mesh state
- Delta xDS enabled by default since Istio 1.22

**Ambient mode evolution** (Istio 1.22+):
- Splits data plane into two layers: ztunnel (L3/L4, Rust) and waypoint proxies (L7, Envoy)
- ztunnel deployed as DaemonSet per node -- handles mTLS, L4 auth, basic telemetry
- Waypoint proxies are optional, shared per namespace/service account
- Control plane (istiod) serves xDS to both ztunnel and waypoint proxies

```
                     istiod
            +---------------------+
            | K8s Watcher         |
            | (Informers)         |
            +---------+-----------+
                      |
            +---------v-----------+
            | Config Translation  |
            | (PushContext)        |
            +---------+-----------+
                      |
            +---------v-----------+
            | Discovery Server    |
            | (xDS gRPC)          |
            +---------+-----------+
                      |
          +-----------+-----------+
          |                       |
    +-----v------+        +------v-----+
    | ztunnel    |        | Waypoint   |
    | (per-node) |        | (per-ns)   |
    | L3/L4 Rust |        | L7 Envoy   |
    +------------+        +------------+
```

**Config sync states**: SYNCED (proxy acked latest), NOT SENT (nothing to send), STALE (sent but no ack -- indicates network issue or bug).

### 2.2 Envoy Gateway

**Architecture**: Multi-stage translation pipeline with Intermediate Representation (IR).

**Key characteristics**:
- Implements Kubernetes Gateway API (not Istio CRDs)
- Explicit separation: Resource Watcher -> Translator -> IR -> xDS Translator -> xDS Server
- Two IRs: Infra IR (managed infrastructure) and xDS IR (proxy configuration)
- xDS Server built on go-control-plane, implementing Delta xDS
- ExtensionManager provides gRPC hooks for custom control plane extensions

```
  +----------------+     +------------------+     +----------+
  | K8s Gateway API|---->| Resource Watcher |---->| GW API   |
  | (GatewayClass, |     | (provider-       |     |Translator|
  |  Gateway,      |     |  specific)       |     +----+-----+
  |  HTTPRoute)    |     +------------------+          |
  +----------------+                              +----v-----+
                                                  | IR       |
                                                  | (Infra + |
                                                  |  xDS)    |
                                                  +----+-----+
                                                       |
                                          +------------+------------+
                                          |                         |
                                    +-----v------+          +------v------+
                                    | xDS        |          | Infra       |
                                    | Translator |          | Manager     |
                                    +-----+------+          +------+------+
                                          |                        |
                                    +-----v------+          +------v------+
                                    | xDS Server |          | Envoy Pods  |
                                    | (gRPC)     |--------->| (managed)   |
                                    +------------+          +-------------+
```

**Status management**: The Gateway API Translator calculates status conditions during translation and publishes them over a message bus. A Status Manager subscribes and updates resource status via the provider.

### 2.3 Cilium

**Architecture**: Per-node agent with eBPF datapath -- sidecar-free.

**Key characteristics**:
- `cilium-agent` DaemonSet on every node is the central orchestrator
- `cilium-operator` Deployment handles cluster-wide operations (IPAM, GC)
- Policy compiled directly to eBPF bytecode and loaded into kernel
- No sidecar proxies for L3/L4 -- enforcement happens in kernel TC hooks
- Per-node Envoy proxy only for L7 features (CiliumEnvoyConfig)
- Identity-based policy: security identities assigned to workloads, enforced via eBPF maps

```
  +-------------------+        +-------------------+
  | K8s API Server    |        | cilium-operator   |
  | (CiliumNetwork-   |        | (cluster-wide)    |
  |  Policy, etc.)    |        +-------------------+
  +---------+---------+
            |
  +---------v---------+
  | cilium-agent      |  <-- per node
  | +---------------+ |
  | | K8s Watcher   | |
  | +-------+-------+ |
  |         |          |
  | +-------v-------+ |
  | | Policy Engine | |
  | +-------+-------+ |
  |         |          |
  | +-------v-------+ |       +------------------+
  | | BPF Compiler  +-------->| Kernel (TC hooks)|
  | +---------------+ |       | eBPF maps:       |
  |                    |       | - identity       |
  | +---------------+ |       | - policy         |
  | | Envoy (L7)    | |       | - conntrack      |
  | | (optional)    | |       +------------------+
  | +---------------+ |
  +-------------------+
```

**Performance advantage**: Eliminates two proxy hops per request. Single agent per node instead of per-pod sidecar. Policy enforcement at kernel level -- no userspace crossing for L3/L4.

### 2.4 Linkerd

**Architecture**: Minimal control plane with purpose-built Rust proxy.

**Key characteristics**:
- Control plane runs in `linkerd` namespace: destination service, identity service, proxy injector
- linkerd2-proxy is a purpose-built Rust proxy (not Envoy)
- Destination service provides: service discovery, policy distribution, service profiles
- Identity service acts as TLS CA, issuing 24-hour certificates with automatic rotation
- Policy controller provides validating admission webhook
- gRPC streaming: `destination.Get()` keeps stream open, sends initial full state, then reactive updates on watches

```
  +-------------------+
  | K8s API Server    |
  +---------+---------+
            |
  +---------v---------+     +-----------------+
  | Destination Svc   |     | Identity Svc    |
  | - Service discovery|     | - TLS CA        |
  | - Policy distrib.  |     | - Cert issuance |
  | - Service profiles |     | - 24h rotation  |
  +--------+----------+     +--------+--------+
           |                          |
  +--------v--------------------------v--------+
  | linkerd2-proxy (Rust, per-pod sidecar)     |
  | - gRPC streaming from destination          |
  | - mTLS with identity certs                 |
  | - L7 routing, retries, timeouts            |
  +--------------------------------------------+
```

**Policy CRDs**: Server (selects pod/port, deny-by-default when present), AuthorizationPolicy (references Server + authentication), MeshTLSAuthentication, NetworkAuthentication. Admission controller prevents overlapping Server resources.

**Watch pattern**: Destination service reacts to K8s watches. When a Pod comes online, an event triggers the EndpointTranslator, which enriches raw K8s data (cross-references IPs for identities, identifies availability zones).

### 2.5 Consul Connect

**Architecture**: Distributed control plane with Raft consensus, xDS via gRPC.

**Key characteristics**:
- Consul servers form a Raft cluster for consistent state
- Consul Dataplane replaces node-level client agents (K8s model)
- Three-layer service mesh: CA (SPIFFE X.509), Proxy Config, XDS Server
- Intentions are authorization policies -- one intention per service pair
- Intentions cached locally on agents and proxies via continuous blocking queries
- Multi-datacenter: primary_datacenter authoritative for intentions, automatic replication to secondaries
- Rate limiting: `update_max_per_second` controls xDS update rate across all streams

```
  +---------------------+
  | Consul Servers      |
  | (Raft Cluster)      |
  | +--CA (SPIFFE)----+ |
  | +--Config Gen-----+ |
  | +--XDS Server-----+ |
  +----------+----------+
             |
             | gRPC (xDS)
             |
  +----------v----------+
  | Consul Dataplane    |  <-- per-pod sidecar
  | +--Envoy Proxy----+ |
  | +--Bootstrap------+ |
  | +--Health Check---+ |
  +---------+-----------+
            |
            | proxied traffic
            v
```

**Key distinction**: Unlike Istio/Linkerd which use K8s informers, Consul uses its own consensus protocol (Raft) for state and blocking queries (long-poll) for change notification to agents/dataplanes.

---

## 3. Architecture Comparison

### Structural Comparison

```
                ISTIO           ENVOY GW        CILIUM         LINKERD        CONSUL

Control      +--------+      +--------+      +--------+     +--------+     +--------+
Plane        | istiod |      | EG     |      | agent  |     | dest   |     | server |
             | (mono- |      | (pipe- |      | (per-  |     | svc    |     | (Raft  |
             |  lith) |      |  line) |      |  node) |     | +ident |     |cluster)|
             +---+----+      +---+----+      +---+----+     +---+----+     +---+----+
                 |               |               |               |               |
Protocol     xDS gRPC       xDS gRPC        eBPF maps       gRPC stream    xDS gRPC
                 |               |            + xDS              |          + blocking
                 |               |               |               |            queries
                 |               |               |               |               |
Data         +---v----+      +---v----+      +---v----+     +---v----+     +---v----+
Plane        | Envoy  |      | Envoy  |      | kernel |     | link-  |     | Envoy  |
             | sidecar|      | (manag-|      | BPF +  |     | erd2-  |     | sidecar|
             | or     |      |  ed)   |      | Envoy  |     | proxy  |     |        |
             | ztunnel|      |        |      | (opt.) |     | (Rust) |     |        |
             +--------+      +--------+      +--------+     +--------+     +--------+

Topology     Per-pod*        Per-gateway     Per-node        Per-pod        Per-pod

Config       K8s CRDs        Gateway API     K8s CRDs        K8s CRDs      Consul API
Source       (Istio)         (standard)      (Cilium)        + Gateway API  + K8s CRDs

State        In-memory       In-memory       In-memory       In-memory      Raft
Model        (PushContext)   (Snapshot        (eBPF maps)    (watch-based)  consensus
                              Cache)

* Ambient mode: per-node ztunnel + optional per-namespace waypoint
```

### Design Philosophy Spectrum

```
  Minimal                                                    Feature-rich
  Simple                                                     Comprehensive
    |                                                              |
    |    Linkerd         Cilium        Consul       Envoy GW   Istio
    |      |               |             |             |         |
    +------+---------------+-------------+-------------+---------+

  Purpose-built proxy   eBPF-first     Raft consensus  Pipeline    Monolith
  Rust data plane      No sidecar(L4)  Multi-DC        IR-based    xDS native
  gRPC streaming       Kernel enforce  Intentions      Gateway API Full mesh
```

### Key Differentiators

| Dimension | Istio | Envoy GW | Cilium | Linkerd | Consul |
|-----------|-------|----------|--------|---------|--------|
| **Proxy** | Envoy (C++) / ztunnel (Rust) | Envoy (managed) | eBPF + Envoy (opt.) | linkerd2-proxy (Rust) | Envoy |
| **Config API** | Istio CRDs | Gateway API | Cilium CRDs | Gateway API + CRDs | Consul API + CRDs |
| **Distribution** | xDS (Delta) | xDS (Delta) | eBPF maps + xDS | gRPC streaming | xDS + blocking queries |
| **Identity** | SPIFFE (Citadel) | Delegated | Security identities (eBPF) | SPIFFE (Identity svc) | SPIFFE (CA) |
| **Multi-cluster** | Multi-primary/remote | Single cluster | ClusterMesh | Multi-cluster | Multi-datacenter (Raft) |
| **L4 enforcement** | Envoy / ztunnel | Envoy | Kernel (eBPF) | linkerd2-proxy | Envoy |
| **L7 enforcement** | Envoy / waypoint | Envoy | Per-node Envoy | linkerd2-proxy | Envoy |

---

## 4. Control Plane Responsibilities

### 4.1 Config Ingestion

All control planes watch external state sources. The mechanism varies by ecosystem:

**Kubernetes Informers** (Istio, Envoy Gateway, Cilium, Linkerd):
- Reflector watches K8s API server, pushes objects to DeltaFIFO queue
- Informer reads from queue, indexes objects, dispatches to controller
- Handler enqueues a key (namespace/name) into WorkQueue
- Same key added multiple times is processed only once (deduplication)
- Cache keeps local copy -- all reads hit cache, not API server

```
  K8s API Server
       |
       | watch stream
       v
  +-----------+
  | Reflector |
  +-----+-----+
        |
  +-----v--------+
  | DeltaFIFO    |
  | (dedup queue)|
  +-----+--------+
        |
  +-----v-----+     +----------+
  | Informer  +---->| Indexer  |
  +-----+-----+     | (cache) |
        |            +----------+
  +-----v-----+
  | Handler   |
  +-----+-----+
        |
  +-----v--------+
  | WorkQueue    |
  | (coalesced)  |
  +-----+--------+
        |
  +-----v---------+
  | Reconciler    |
  | (idempotent)  |
  +--------------+
```

**Consul blocking queries**:
- Long-poll HTTP requests with index-based versioning
- Agent maintains local cache of intentions
- Changes propagated near-instantly via continuous blocking query
- No K8s dependency -- works with any service registry

**Key principle**: The reconciliation loop is idempotent. Running the same reconciliation with the same input produces the same result. This is what enables eventual consistency.

### 4.2 Translation

Translation is the core intellectual work of a control plane. It converts domain-specific high-level config into data-plane-specific low-level config.

**Istio's translation**:
```
  VirtualService + DestinationRule + Gateway
            |
            v
  PushContext (in-memory mesh state)
            |
            v
  Per-proxy config generation
  (filtered by proxy's scope)
            |
            v
  xDS resources: CDS, LDS, RDS, EDS
```

**Envoy Gateway's IR-based translation**:
```
  Gateway API resources (GatewayClass, Gateway, HTTPRoute)
            |
            v
  GW API Translator
            |
            v
  IR (Infra IR + xDS IR)
            |
            +---> xDS Translator ---> xDS resources
            |
            +---> Infra Manager ---> Envoy Pods/Deployments
```

The IR pattern is significant: it decouples the control plane from both the input format (Gateway API today, something else tomorrow) and the output format (xDS today, eBPF maps theoretically). This is hexagonal architecture applied to control planes.

**Cilium's eBPF compilation**:
```
  CiliumNetworkPolicy + CiliumEnvoyConfig
            |
            v
  Policy Engine (identity-based)
            |
            +---> BPF Compiler ---> Kernel TC hooks (eBPF bytecode)
            |
            +---> CiliumEnvoyConfig ---> Per-node Envoy (L7 only)
```

### 4.3 Distribution

How translated config reaches the data plane.

**xDS protocol** (Istio, Envoy Gateway, Consul):

Four protocol variants:
| Variant | Stream | Granularity |
|---------|--------|-------------|
| SotW (State of the World) | Separate per type | Full snapshot |
| Incremental (Delta) | Separate per type | Only changed resources |
| ADS (Aggregated Discovery Service) | Single stream, all types | Full snapshot |
| Incremental ADS | Single stream, all types | Only changed resources |

Resource types and their dependencies:
```
  LDS (Listeners) ----references----> RDS (Routes)
       |                                    |
       |                                    references
       |                                    |
  CDS (Clusters) ----references----> EDS (Endpoints)

  ECDS (Extensions) -- independent, hot-loadable
  SDS (Secrets/Certs) -- independent
```

**Ordering constraint**: RDS updates for new listeners must arrive AFTER CDS/EDS/LDS. ADS solves this by serializing all types on a single stream.

**Delta xDS** (default in Istio 1.22+):
- Client subscribes/unsubscribes to specific resource names
- Server sends only changed resources
- At scale: instead of re-sending 100k clusters when one changes, send only the modified cluster
- Request type: `DeltaDiscoveryRequest`, Response type: `DeltaDiscoveryResponse`

**gRPC streaming** (Linkerd):
- `destination.Get()` opens persistent stream
- Server sends initial full state, then incremental updates on watch events
- No xDS -- custom protobuf API (linkerd2-proxy-api)
- Lighter weight than xDS but less standardized

**eBPF map updates** (Cilium):
- Agent updates BPF maps (identity, policy, conntrack) directly
- No network protocol needed -- kernel memory operations
- Atomic map updates for consistency
- Fastest distribution mechanism (no serialization, no network hop)

### 4.4 Health Checking and Status

Control planes must know whether config was successfully applied:

**Istio**: Three sync states per proxy -- SYNCED (acked), NOT SENT (nothing to send), STALE (sent, no ack). Status visible via `istioctl proxy-status`.

**Envoy Gateway**: Status Manager subscribes to translation events and updates Gateway API status conditions on resources.

**Linkerd**: Destination service maintains stream health per proxy connection. Policy controller reports status via CRD conditions.

**Consul**: Health checks at service level (HTTP, TCP, script, gRPC). xDS connection state tracked per dataplane instance. Intention application confirmed via proxy config cache.

---

## 5. Data Plane Integration (xDS)

### 5.1 The xDS Resource Model

xDS is the universal control plane protocol, originated by Envoy but now used across multiple projects.

```
  +-------+                              +-------+
  | CP    |  DeltaDiscoveryResponse      | Proxy |
  |       |----------------------------->|       |
  |       |                              |       |
  |       |  DeltaDiscoveryRequest       |       |
  |       |<-----------------------------|       |
  |       |  (subscribe/unsubscribe/ack) |       |
  +-------+                              +-------+
```

**Resource type hierarchy**:

| Type URL | Short | Purpose | References |
|----------|-------|---------|------------|
| `type.googleapis.com/envoy.config.listener.v3.Listener` | LDS | Listener config, filter chains | RDS |
| `type.googleapis.com/envoy.config.route.v3.RouteConfiguration` | RDS | Route matching, weighted clusters | CDS |
| `type.googleapis.com/envoy.config.cluster.v3.Cluster` | CDS | Upstream cluster config | EDS |
| `type.googleapis.com/envoy.config.endpoint.v3.ClusterLoadAssignment` | EDS | Endpoint addresses, weights | -- |
| `type.googleapis.com/envoy.extensions.transport_sockets.tls.v3.Secret` | SDS | TLS certs, keys | -- |
| `type.googleapis.com/envoy.config.core.v3.TypedExtensionConfig` | ECDS | Extension configs (filters) | -- |

### 5.2 go-control-plane Reference Implementation

The canonical xDS server implementation in Go provides two cache strategies:

**SnapshotCache**:
- Maintains a single versioned snapshot per node group
- `SetSnapshot(nodeID, snapshot)` -- atomically replaces all config for a node
- On set, compares versions with all open watches and triggers responses for changes
- In ADS mode: holds EDS/RDS responses until all referenced resources are requested (enables atomic updates)
- Keyed by node hash function (node ID, cluster, metadata)

**LinearCache**:
- Single collection indexed by resource name
- Manages versions internally
- Suited for uniform config across fleet (e.g., all EDS entries same for all proxies)
- Simpler but less flexible than SnapshotCache

```
  SnapshotCache                          LinearCache
  +---------------------------+          +---------------------------+
  | node_id -> Snapshot       |          | resource_name -> Resource |
  |                           |          |                           |
  | Snapshot {                |          | Internally versioned      |
  |   version: "v42"         |          | Single type URL           |
  |   resources: {           |          | Uniform across fleet      |
  |     CDS: [...],          |          +---------------------------+
  |     LDS: [...],          |
  |     RDS: [...],          |
  |     EDS: [...],          |
  |   }                      |
  |   version_map: {         |
  |     resource_name -> hash|
  |   }                      |
  | }                        |
  +---------------------------+
```

**Consistency guarantee**: SnapshotCache enforces that in ADS mode, the CDS response names all EDS clusters, and the LDS response names all RDS routes. This ensures the proxy receives a consistent view.

### 5.3 Per-Proxy Configuration

Most control planes generate per-proxy config, not global config:

**Istio**: PushContext generates config scoped to each proxy's namespace and sidecar resource configuration. Discovery selectors filter which namespaces the control plane even considers.

**Envoy Gateway**: Config generated per-gateway (not per-pod). Each Gateway resource maps to a managed Envoy deployment.

**Linkerd**: Destination service returns endpoints filtered by the requesting proxy's authorization context.

**Consul**: xDS server generates config based on the service registration of the connecting dataplane.

### 5.4 Incremental Updates and Connection Draining

**Delta xDS workflow**:
1. Proxy connects, subscribes to resource names it needs
2. Server sends initial resources (matching subscriptions)
3. On config change, server sends only modified resources
4. Proxy ACKs or NACKs each response
5. On NACK, server can retry or log -- proxy continues with last good config

**Connection draining** (Envoy-specific):
- When a listener is removed, existing connections are drained gracefully
- Drain timeout configurable per listener
- New connections rejected immediately, existing connections allowed to complete
- Cluster warming: new clusters are health-checked before receiving traffic

---

## 6. State Management

### 6.1 In-Memory State Models

| Control Plane | State Structure | Consistency Model |
|--------------|-----------------|-------------------|
| **Istio** | PushContext (rebuilt on config change) | Eventual (per-proxy convergence) |
| **Envoy GW** | IR (Infra + xDS), SnapshotCache | Eventual (snapshot-based) |
| **Cilium** | eBPF maps (identity, policy, conntrack) | Atomic (per-map update) |
| **Linkerd** | Watch-based endpoint/policy cache | Eventual (stream-based) |
| **Consul** | Raft log + agent cache | Strong (Raft) + Eventual (agent cache) |

### 6.2 The Eventual Consistency Contract

Service mesh control planes embrace eventual consistency as a fundamental design principle:

1. **Config is eventually consistent** -- different proxies may have different config versions at any instant
2. **The system converges** -- given no new changes, all proxies will eventually have the same config
3. **Proxies are ephemeral** -- they can restart and re-fetch full state from the control plane
4. **Stale config is safe** -- the data plane continues to function with last-known-good config if the control plane is temporarily unavailable
5. **Health checking compensates** -- since service discovery is eventually consistent, active health checking at the proxy provides a real-time correctness backstop

**Istio's PushContext pattern**:
```
  Config change detected
         |
         v
  Rebuild PushContext (full mesh state snapshot)
         |
         v
  For each connected proxy:
    Generate per-proxy xDS config
    Compare with last-sent version
    If different: push via xDS stream
    Track: SYNCED / STALE status
```

### 6.3 Snapshot vs. Streaming

Two dominant state distribution patterns:

**Snapshot-based** (Istio, Envoy Gateway, Consul):
- Control plane builds complete state snapshot
- On change, compute diff and push
- Proxy receives atomic snapshot or delta
- Version tracking enables ACK/NACK protocol
- Simpler correctness reasoning -- each snapshot is a consistent point-in-time

**Stream-based** (Linkerd, Cilium agent):
- Persistent connection with initial state dump
- Changes pushed as they happen (reactive)
- No explicit versioning -- stream ordering provides consistency
- Lower latency for individual changes
- More complex reconnection logic (need to re-sync full state)

### 6.4 Reconciliation Loop Properties

All K8s-native control planes use the reconciliation pattern with these properties:

- **Idempotent**: Same input always produces same output
- **Level-triggered, not edge-triggered**: Reconciler acts on current state, not on the event that triggered it
- **Coalescing**: Multiple events for the same resource are batched into one reconciliation
- **Retry with backoff**: Failed reconciliations are requeued with exponential backoff
- **Status reporting**: Reconciler updates resource status conditions to reflect actual state

---

## 7. Multi-Tenancy

### 7.1 Namespace Isolation Patterns

**Istio Configuration Scoping**:
1. **Discovery Selectors**: Control plane ignores namespaces not matching criteria
2. **Sidecar Resource**: Per-namespace sidecar configuration restricts which namespaces a proxy receives config for
3. **exportTo field**: ServiceEntry, DestinationRule, VirtualService can specify which namespaces they export to
4. Default: all config visible to all proxies (must opt-in to restriction)

**Cilium Namespace Isolation**:
- CiliumNetworkPolicy is namespace-scoped by default
- CiliumClusterwideNetworkPolicy for cross-namespace rules
- Identity-based: each namespace gets distinct security identities
- eBPF enforcement is per-pod, naturally namespace-aware

**Linkerd Server Isolation**:
- Server resources are namespace-scoped, select pods in same namespace only
- Admission controller prevents overlapping Server resources
- When Server exists: all traffic denied unless explicitly authorized
- AuthorizationPolicy references are namespace-scoped

**Consul Namespace Isolation**:
- Consul Enterprise: native namespace support with ACL tokens scoped per namespace
- Intentions scoped per service pair within or across namespaces
- Admin partitions for hard multi-tenancy boundaries
- Cross-partition references require explicit peering

### 7.2 Cross-Namespace Reference Patterns

A recurring challenge: how does config in one namespace reference resources in another?

| Pattern | Used By | Mechanism |
|---------|---------|-----------|
| **exportTo** | Istio | Explicit list of target namespaces |
| **ReferenceGrant** | Gateway API (Envoy GW, Linkerd) | Target namespace explicitly grants access |
| **ClusterPolicy** | Cilium | Cluster-scoped policy applies across namespaces |
| **Admin Partitions** | Consul | Hard boundaries with explicit peering |

The Gateway API `ReferenceGrant` pattern is the emerging standard: the referenced namespace must explicitly grant permission for cross-namespace references. This is deny-by-default for cross-namespace -- aligns with least privilege.

---

## 8. Extensibility

### 8.1 Envoy Extensibility Mechanisms

Four mechanisms, from least to most invasive:

**ext_authz (External Authorization)**:
- Filter calls external gRPC/HTTP service for authorization decisions
- Request metadata sent to external service, receives allow/deny
- Central deployment -- single authz service for many proxies
- Cannot modify request/response body
- Latency: one additional network hop per request

**ext_proc (External Processing)**:
- Can process both requests AND responses
- Can opt-in to receiving request/response bodies
- Must be deployed per-Envoy instance (latency-sensitive)
- Full request/response mutation capability
- Higher complexity than ext_authz

**Wasm Filters**:
- Code compiled to WebAssembly, loaded at runtime via Proxy-Wasm ABI
- SDKs in Rust, Go, C++, AssemblyScript
- Runs in-process (no network hop)
- Sandboxed execution (memory-safe)
- Hot-reloadable via ECDS

**ECDS (Extension Config Discovery Service)**:
- Extension configs served independently from listener config
- Enables separate control plane for extensions (e.g., WAF team manages WAF config)
- Hot-reload without listener restart
- Decouples extension lifecycle from core proxy lifecycle

```
  Extensibility Spectrum

  Least invasive                              Most invasive
  Lowest latency                              Highest flexibility
       |                                            |
       | ext_authz   ext_proc   Wasm     Native C++ |
       |    |           |        |          |       |
       +----+-----------+--------+----------+-------+

  Network hop    Network hop   In-process  In-process
  Auth only      Full mutate   Sandboxed   No sandbox
  Central        Per-proxy     Hot-reload  Recompile
```

### 8.2 Control Plane Extension Points

**Envoy Gateway ExtensionManager**:
- gRPC hooks executed before xDS generation (modify IR) and after (modify xDS resources)
- Enables custom policy injection without forking the control plane

**Istio EnvoyFilter**:
- Direct Envoy config manipulation (patch operations on listeners, routes, clusters)
- Powerful but fragile -- tightly coupled to Envoy internals
- Being deprecated in favor of Gateway API extensibility

**Cilium CiliumEnvoyConfig**:
- Kubernetes CRD that maps to Envoy listener/filter/route/cluster config
- Agent translates CRD to Envoy config for the per-node proxy
- Bridges Cilium's eBPF world with Envoy's L7 world

**Linkerd Policy Attachment**:
- Gateway API policy attachment pattern
- Policies are generic over target types (implementation detail of controller)
- Composable: multiple policies can attach to same target

### 8.3 The ext_authz Pattern (Most Relevant to geist.sh)

ext_authz is the closest service mesh analog to geist.sh's policy evaluation:

```
  Service Mesh (ext_authz)              geist.sh (policy evaluation)

  +--------+    +----------+           +--------+    +-------------+
  | Envoy  |--->| ext_authz|           | Agent  |--->| geist-policy|
  | Proxy  |    | Service  |           | (L4)   |    | (PDP)       |
  +--------+    +----------+           +--------+    +-------------+
       |              |                     |               |
  Request         Allow/Deny           AgentOp         PolicyDecision
  metadata        + headers            context         (Allow/Deny)

  Both:
  - Sit on the request path
  - Receive structured context (request metadata / AgentOp)
  - Return binary decision (allow/deny) with optional metadata
  - Are stateless per-evaluation
  - Policy config comes from control plane / config layer
```

---

## 9. Operational Patterns

### 9.1 Config Validation

Three validation stages across control planes:

**Admission-time validation** (before config enters the system):
- Istio: ValidatingAdmissionWebhook (`istio-validator-*`)
- Linkerd: Policy controller validating webhook (prevents overlapping Servers)
- Envoy Gateway: Gateway API webhook validation
- Cilium: CiliumNetworkPolicy webhook validation

**Translation-time validation** (during config processing):
- Istio: PushContext build validates internal consistency
- Envoy Gateway: IR translation validates cross-resource references
- Errors surface as status conditions on source resources

**Static analysis** (offline, before apply):
- Istio: `istioctl analyze` -- validates config against mesh state without applying
- Cilium: `cilium policy validate` -- checks policy syntax and semantics
- Envoy: config can be validated with `--mode validate` flag

### 9.2 Canary Config Rollout

**Istio Control Plane Revisions**:
- Install new istiod alongside existing one with a revision tag
- Label namespaces with new revision to migrate workloads gradually
- Both control planes serve xDS simultaneously
- Rollback: relabel namespaces to old revision

```
  Namespace: app-a              Namespace: app-b
  label: istio.io/rev=stable    label: istio.io/rev=canary
        |                             |
        v                             v
  +----------+                  +----------+
  | istiod   |                  | istiod   |
  | (stable) |                  | (canary) |
  +----------+                  +----------+
```

**Cilium**: Rolling DaemonSet update of cilium-agent. eBPF programs replaced atomically per-node. Existing connections maintained through connection tracking.

**Consul**: Primary datacenter controls intention replication. Config changes can be rolled out per-datacenter.

**General pattern**: Config rollout follows the same canary pattern as code deployment -- percentage-based, with monitoring and automatic rollback on error signals.

### 9.3 Dry-Run and Shadow Mode

**Istio**: `istioctl analyze` performs offline validation. Ambient mode ztunnel can be deployed in permissive mode (log but don't enforce).

**Linkerd**: Server resources support `accessPolicy: audit` mode -- logs policy violations without denying traffic.

**Cilium**: CiliumNetworkPolicy supports `audit` mode in policyAuditMode -- packets that would be dropped are logged instead.

**Consul**: Intentions can be tested with `consul intention check` before applying.

**Pattern**: Every mature control plane provides a way to observe what policy WOULD do before it does it. This is essential for safe rollout.

### 9.4 Status Reporting

All control planes report config application status back to operators:

| Control Plane | Status Mechanism | Granularity |
|--------------|-----------------|-------------|
| Istio | `istioctl proxy-status`, CRD `.status` | Per-proxy, per-resource |
| Envoy GW | Gateway API status conditions | Per-resource |
| Cilium | `cilium status`, CRD `.status` | Per-node, per-policy |
| Linkerd | CRD `.status`, `linkerd check` | Per-resource |
| Consul | Health checks, `consul members` | Per-service, per-agent |

---

## 10. The Control Plane for Agents Analogy

### 10.1 Mapping Service Mesh to Agent Governance

The core insight: geist.sh's policy distribution architecture is structurally isomorphic to a service mesh control plane. The domain objects differ, but the architecture is the same.

```
  SERVICE MESH                        GEIST.SH
  ============                        ========

  Operator writes                     Operator writes
  VirtualService/Policy CRDs          Policy rules (YAML/TOML)
       |                                   |
       v                                   v
  Control plane ingests              Shell ingests policy config
  (K8s informers / API)             (file watch / API)
       |                                   |
       v                                   v
  Translates to xDS config          Translates to PolicyRuleset
  (per-proxy config)                (per-agent config)
       |                                   |
       v                                   v
  Distributes to proxies            Distributes to enforcement points
  (xDS push / gRPC stream)         (in-process / gRPC)
       |                                   |
       v                                   v
  Proxy enforces per-request        Shell enforces per-AgentOp
  (allow/deny + routing)            (Allow/Deny + audit)
```

### 10.2 Concept Mapping

| Service Mesh Concept | geist.sh Equivalent | Notes |
|---------------------|---------------------|-------|
| **Envoy Proxy** | Agent runtime (L4 inside shell) | The "data plane" being configured |
| **xDS Config** | PolicyRuleset | The translated, enforceable config |
| **Listener** | Tool/capability endpoint | What the agent can invoke |
| **Route** | Policy rule match | How requests are classified |
| **Cluster** | Upstream resource/service | What the agent accesses |
| **ext_authz** | geist-policy (PDP) | Per-request authorization |
| **Service identity (SPIFFE)** | agent_id | Who is making the request |
| **Request context** | AgentOp | Structured context for policy evaluation |
| **PolicyDecision** | Allow/Deny/Escalate | Three-valued lattice (richer than mesh) |
| **VirtualService CRD** | Policy config file | Operator-authored intent |
| **istiod / control plane** | geist-gateway (L2) | Translation + distribution |
| **Sidecar resource** | Config scoping per agent | Namespace isolation analog |
| **ECDS** | Hot-reloadable policy extensions | Independent lifecycle |
| **Wasm filter** | Policy plugin (future) | In-process, sandboxed extension |

### 10.3 Architectural Lessons for geist.sh

**From Istio**:
- PushContext pattern -- rebuild full state on change, generate per-agent config from it
- Delta distribution matters at scale (don't re-send entire policy when one rule changes)
- Status tracking per enforcement point (SYNCED/STALE) is essential for operational confidence
- Configuration scoping prevents blast radius (discovery selectors, sidecar resources)

**From Envoy Gateway**:
- IR decouples input format from output format -- enables hexagonal architecture
- Translation pipeline with explicit stages is easier to reason about and test
- Status reporting during translation (not just after) catches errors early
- ExtensionManager pattern for pluggable policy injection without forking

**From Cilium**:
- Per-node agent instead of per-pod sidecar reduces overhead dramatically
- eBPF maps for L3/L4 enforcement -- kernel-level speed, no userspace crossing
- Identity-based policy scales better than IP-based
- Audit mode (log but don't enforce) is critical for safe rollout

**From Linkerd**:
- Purpose-built proxy in Rust (linkerd2-proxy) -- smaller, faster, more secure than general-purpose Envoy
- gRPC streaming with initial full state + incremental updates is simpler than full xDS
- Deny-by-default when policy resources exist (Server CRD)
- Admission webhooks prevent invalid policy combinations

**From Consul**:
- Raft consensus for strong consistency where it matters (intention source of truth)
- Blocking queries for lightweight change notification
- Multi-datacenter intention replication with authoritative primary
- Rate limiting config distribution (`update_max_per_second`)

### 10.4 The Three Deployment Modes as Control Plane Topologies

geist.sh's three deployment modes map directly to service mesh topologies:

```
  1. EMBEDDABLE LIBRARY (crate mode)
     = In-process policy evaluation
     ~ Envoy compiled-in filter

     +----------------------------+
     | Your Rust Application      |
     |                            |
     |  +---------------------+  |
     |  | geist-policy (crate)|  |
     |  | evaluate(AgentOp)   |  |
     |  +---------------------+  |
     +----------------------------+

  2. STANDALONE MICROGATEWAY (sidecar mode)
     = Sidecar proxy pattern
     ~ Envoy sidecar with ext_authz

     +------------------+    +------------------+
     | geist-gateway    |--->| Your Agent       |
     | (L2 enforcement) |<---| (any runtime)    |
     +------------------+    +------------------+

  3. FULL SHELL (runtime mode)
     = Ambient mesh (agent runs inside)
     ~ Istio ambient: ztunnel hosts workload

     +------------------------------------+
     | Shell (Rust Runtime)               |
     |                                    |
     |  +-----------------------------+  |
     |  | Agent (Python, L4)          |  |
     |  +-------------+---------------+  |
     |                | all calls         |
     |  +-------------v---------------+  |
     |  | Gateway (L2) + Kernel (L1)  |  |
     |  +-----------------------------+  |
     +------------------------------------+
```

### 10.5 What geist.sh Can Do That Meshes Cannot

The agent governance domain has capabilities that exceed service mesh patterns:

**Three-valued policy lattice**: Meshes are binary (allow/deny). geist.sh adds Escalate -- "I can't decide, ask a human." The lattice Deny > Escalate > Allow enables graceful degradation.

**Temporal composition awareness**: Per-request policy is necessary but not sufficient. Agents can take individually-valid actions that compose into harmful sequences. This is the unsolved frontier (see ACT research on composition problem). Service meshes don't face this because HTTP requests don't compose the same way tool invocations do.

**Self-composition**: The agent modifies its own capabilities. There is no service mesh analog -- proxies don't modify their own filter chains. This requires the control plane to reason about meta-policy: "Is the agent allowed to change what it's allowed to do?"

**Session context**: AgentOp carries session_id and metadata. Policy can be stateful across a session -- something meshes explicitly avoid (stateless per-request evaluation). This is a deliberate architectural choice for agent governance.

---

## Appendix A: xDS Resource Type Quick Reference

```
  LDS (Listener Discovery Service)
  ├── Defines: listeners, filter chains, transport sockets
  ├── References: RDS (route_config_name in HttpConnectionManager)
  └── Warming: Yes (new listeners warmed before accepting traffic)

  RDS (Route Discovery Service)
  ├── Defines: route configurations, virtual hosts, routes
  ├── References: CDS (cluster names in route actions)
  └── Warming: No (control plane must ensure referenced clusters exist)

  CDS (Cluster Discovery Service)
  ├── Defines: upstream clusters, load balancing, health checks
  ├── References: EDS (for EDS-type clusters)
  └── Warming: Yes (new clusters health-checked before receiving traffic)

  EDS (Endpoint Discovery Service)
  ├── Defines: endpoints per cluster, weights, health status
  ├── References: None
  └── Warming: No

  SDS (Secret Discovery Service)
  ├── Defines: TLS certificates, private keys, validation contexts
  ├── References: None
  └── Warming: N/A

  ECDS (Extension Config Discovery Service)
  ├── Defines: extension filter configurations
  ├── References: Varies by extension
  └── Warming: Yes (extension loaded before traffic routed through it)
```

## Appendix B: Decision Framework for Policy Distribution

When designing geist.sh's policy distribution, evaluate against these axes:

| Axis | Options | Trade-off |
|------|---------|-----------|
| **Distribution protocol** | In-process / gRPC / xDS | Latency vs. decoupling |
| **State model** | Snapshot / Streaming / Polling | Consistency vs. complexity |
| **Config granularity** | Per-agent / Per-session / Global | Flexibility vs. overhead |
| **Update strategy** | Full push / Delta / Lazy | Bandwidth vs. convergence time |
| **Validation timing** | Admission / Translation / Runtime | Safety vs. latency |
| **Rollout strategy** | Atomic / Canary / Shadow | Speed vs. risk |
| **Extensibility point** | In-process plugin / RPC / Compiled | Performance vs. decoupling |

**Recommended starting point for geist.sh** (based on patterns above):

Phase 1-2 (current): In-process evaluation (`geist-policy` crate, embeddable library mode). No distribution needed -- policy is compiled in. This is the Cilium eBPF-map equivalent: zero latency, zero network.

Phase 3-4: gRPC streaming for sidecar/runtime modes. Stream initial ruleset on connect, push deltas on policy change. This is the Linkerd destination-service pattern: simpler than xDS, purpose-built for the domain.

Phase 5+: Consider xDS compatibility if interoperability with existing mesh infrastructure becomes a requirement. The IR pattern from Envoy Gateway enables this without coupling the core to xDS.

## Appendix C: Sources

### Primary Documentation
- [Istio: Introducing istiod](https://istio.io/latest/blog/2020/istiod/)
- [Istio: Ambient Control Plane](https://istio.io/latest/docs/ambient/architecture/control-plane/)
- [Istio: Configuration Scoping](https://istio.io/latest/docs/ops/configuration/mesh/configuration-scoping/)
- [Envoy Gateway: System Design](https://gateway.envoyproxy.io/contributions/design/system-design/)
- [Envoy Gateway: Gateway API Translator Design](https://gateway.envoyproxy.io/contributions/design/gatewayapi-translator/)
- [Envoy: xDS Protocol Specification](https://www.envoyproxy.io/docs/envoy/latest/api-docs/xds_protocol)
- [Envoy: Dynamic Configuration Overview](https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/operations/dynamic_configuration)
- [Cilium: eBPF Datapath](https://docs.cilium.io/en/stable/network/ebpf/index.html)
- [Cilium: Service Mesh](https://docs.cilium.io/en/stable/network/servicemesh/index.html)
- [Linkerd: Architecture](https://linkerd.io/2-edge/reference/architecture/)
- [Linkerd: Authorization Policy](https://linkerd.io/2-edge/reference/authorization-policy/)
- [Consul: Control Plane Architecture](https://developer.hashicorp.com/consul/docs/architecture/control-plane)
- [Consul: Dataplane Architecture](https://developer.hashicorp.com/consul/docs/architecture/control-plane/dataplane)
- [Consul: Intentions](https://developer.hashicorp.com/consul/docs/secure-mesh/intention)
- [go-control-plane: Cache Package](https://pkg.go.dev/github.com/envoyproxy/go-control-plane/pkg/cache/v3)

### Technical Deep Dives
- [Tetrate: Delta xDS in Istio 1.22](https://tetrate.io/blog/istio-service-mesh-delta-xds)
- [Tetrate: 4 Envoy Extensibility Mechanisms](https://tetrate.io/blog/4-envoy-extensibility-mechanisms-how-to-boost-envoy-gateway-performance-and-functionality)
- [Beza: Deep Dive into linkerd-destination](https://medium.com/@bezarsnba/deep-dive-the-linkerd-destination-service-en-19f6efd1b308)
- [Buoyant: Designing Linkerd's Policy CRD](https://www.buoyant.io/media/what-we-learned-from-the-gateway-api-designing-linkerds-new-policy-crd)
- [Linkerd: Polixy Design](https://github.com/linkerd/polixy/blob/main/DESIGN.md)
- [Cilium DeepWiki](https://deepwiki.com/cilium/cilium)
- [Istio DeepWiki: Control Plane](https://deepwiki.com/higress-group/istio/2-control-plane-(istiod))

### Architecture Guidance
- [Christian Posta: Building a Control Plane for Envoy (Part 1)](https://blog.christianposta.com/envoy/guidance-for-building-a-control-plane-to-manage-envoy-proxy-based-infrastructure/)
- [Christian Posta: Identify Components (Part 2)](https://blog.christianposta.com/envoy/guidance-for-building-a-control-plane-for-envoy-identify-components/)
- [Christian Posta: Domain-Specific Configuration (Part 3)](https://blog.christianposta.com/envoy/guidance-for-building-a-control-plane-for-envoy-domain-specific-configuration-api/)
- [Christian Posta: Build for Extensibility (Part 4)](https://blog.christianposta.com/guidance-for-building-a-control-plane-for-envoy-build-for-pluggability/)
- [Christian Posta: Deployment Tradeoffs (Part 5)](https://blog.christianposta.com/guidance-for-building-a-control-plane-for-envoy-deployment-tradeoffs/)
- [Matt Klein: Data Plane vs Control Plane](https://blog.envoyproxy.io/service-mesh-data-plane-vs-control-plane-2774e720f7fc)
- [Solo.io: Istio Architecture](https://www.solo.io/topics/istio/istio-architecture)
- [Envoy Gateway Extensions Design](https://gateway.envoyproxy.io/contributions/design/extending-envoy-gateway/)
