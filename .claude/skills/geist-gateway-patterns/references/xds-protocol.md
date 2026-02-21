# xDS Protocol Family — Comprehensive Reference

> For architectural judgment when designing control planes. Covers protocol mechanics,
> transport variants, resource model, ordering semantics, and production control plane patterns.

---

## Table of Contents

1. [Protocol Family Overview](#1-protocol-family-overview)
2. [Resource Type Registry](#2-resource-type-registry)
3. [Transport Protocol](#3-transport-protocol)
4. [Resource Model — Messages and Fields](#4-resource-model)
5. [ACK/NACK Flow](#5-acknack-flow)
6. [Delta/Incremental Protocol](#6-deltaincrementalprotocol)
7. [ADS — Aggregated Discovery Service](#7-ads)
8. [Resource Ordering and Warming](#8-resource-ordering-and-warming)
9. [Wildcard and Resource Naming](#9-wildcard-and-resource-naming)
10. [go-control-plane Reference Implementation](#10-go-control-plane)
11. [Control Plane Patterns in Production](#11-control-plane-patterns)
12. [Key Design Decisions](#12-key-design-decisions)
13. [Sources](#13-sources)

---

## 1. Protocol Family Overview

xDS is the dynamic configuration protocol family that Envoy (and increasingly gRPC, Cilium,
and other data planes) uses to discover configuration at runtime from a management server
(control plane). "xDS" is shorthand for "x Discovery Service" — where x is replaced by
the resource type.

The protocol was born inside Envoy but is now stewarded by the CNCF as a
[standalone specification](https://github.com/cncf/xds). It has become the de facto standard
for control-plane-to-data-plane communication in the service mesh and gateway ecosystem.

### Core Insight

xDS separates **what** is being configured (resource types) from **how** configuration is
delivered (transport protocol). Any resource type can be delivered over any transport variant.

```
                    +---------------------------------------------+
                    |            Management Server                 |
                    |          (Control Plane / xDS Server)        |
                    +-----+-------+-------+-------+-------+-------+
                          |       |       |       |       |
                         LDS     RDS     CDS     EDS     SDS
                          |       |       |       |       |
                    +-----+-------+-------+-------+-------+-------+
                    |            Envoy / Data Plane                |
                    |           (xDS Client)                       |
                    +---------------------------------------------+
```

---

## 2. Resource Type Registry

### Core Discovery Services

| Service | Abbreviation | Discovers | Type URL (v3) | Subscription |
|---------|-------------|-----------|---------------|-------------|
| Listener Discovery Service | **LDS** | Listeners (bind address, filter chains) | `type.googleapis.com/envoy.config.listener.v3.Listener` | Wildcard |
| Route Discovery Service | **RDS** | Route configurations (virtual hosts, routing rules, header modifications) | `type.googleapis.com/envoy.config.route.v3.RouteConfiguration` | Explicit |
| Cluster Discovery Service | **CDS** | Upstream clusters (load balancing, health checking, circuit breaking) | `type.googleapis.com/envoy.config.cluster.v3.Cluster` | Wildcard |
| Endpoint Discovery Service | **EDS** | Cluster members / endpoints (IP:port, weights, health) | `type.googleapis.com/envoy.config.endpoint.v3.ClusterLoadAssignment` | Explicit |
| Secret Discovery Service | **SDS** | TLS certificates, private keys, session ticket keys, trusted CA certs | `type.googleapis.com/envoy.extensions.transport_sockets.tls.v3.Secret` | Explicit |
| Extension Config Discovery Service | **ECDS** | Extension filter configurations (HTTP filters, network filters, listener filters) | `type.googleapis.com/envoy.config.core.v3.TypedExtensionConfig` | Explicit |

### Extended Discovery Services

| Service | Abbreviation | Discovers | Type URL (v3) | Notes |
|---------|-------------|-----------|---------------|-------|
| Scoped Route Discovery Service | **SRDS** | Route scopes for massive route tables | `type.googleapis.com/envoy.config.route.v3.ScopedRouteConfiguration` | Avoids linear search over huge route tables |
| Virtual Host Discovery Service | **VHDS** | Individual virtual hosts within a route config | `type.googleapis.com/envoy.config.route.v3.VirtualHost` | Delta-only, on-demand subscription |
| Runtime Discovery Service | **RTDS** | Runtime feature flags and config layers | `type.googleapis.com/envoy.service.runtime.v3.Runtime` | Augments file-based runtime layers |

### Dependency Graph

```
  LDS ──references──> RDS ──references──> CDS (via cluster names in routes)
                                            |
  CDS ──references──> EDS (via cluster type = EDS)
    |
    +──references──> SDS (via transport_socket TLS config)

  LDS ──references──> SDS (via listener TLS config)
  LDS ──references──> ECDS (via extension filter configs)
  LDS ──references──> SRDS (alternative to RDS for scoped routing)
  RDS ──contains───> VHDS (on-demand virtual host discovery)
```

### What Each Service Does

**LDS** — The entry point. Discovers entire listener configurations including bind address,
filter chains (network filters, HTTP connection manager), and TLS transport socket
configuration. Each listener may reference RDS for routes, SDS for TLS, and ECDS for
dynamically loaded filters.

**RDS** — Discovers HTTP route configurations. These include virtual host definitions,
routing rules (path/header/query matching), header manipulation, retry policies, and
rate limit configurations. Referenced by name from LDS HTTP connection manager config.

**CDS** — Discovers upstream cluster definitions. A cluster is a group of logically similar
upstream hosts. Configuration includes load balancing policy (round robin, least request,
ring hash, etc.), health checking, circuit breakers, outlier detection, and connection
pool settings. Wildcard — Envoy receives all clusters.

**EDS** — Discovers the actual endpoints (IP:port pairs) that belong to a cluster. Replaces
DNS-based service discovery with richer metadata: locality-aware routing, weighted
endpoints, health status per endpoint, and priority levels. Referenced by cluster name
when a cluster's type is `EDS`.

**SDS** — Discovers cryptographic material: TLS certificates + private keys, trusted root
CAs, certificate revocation lists. Enables hot-rotation of certificates without restart.
Envoy never needs the private key on disk — SDS streams it over a Unix domain socket
or gRPC channel. Critical for zero-downtime cert rotation (e.g., with SPIFFE/SPIRE).

**ECDS** — Discovers extension configurations independently from the listener. Allows a
WAF, fault injection, or auth filter to be updated without touching the listener config.
Decouples filter lifecycle from listener lifecycle.

---

## 3. Transport Protocol

xDS supports three transport variants, each with different tradeoffs:

### 3.1 State-of-the-World (SotW) gRPC Streaming

The original and most common variant. Bidirectional gRPC stream per resource type.

- **Every response contains the complete set** of resources of that type
- Absence of a previously-present resource implies deletion
- Simple mental model: "here is everything, right now"
- Separate bidirectional stream per type URL (unless ADS)

```
gRPC Service Definitions (v3):

  envoy.service.listener.v3.ListenerDiscoveryService
    rpc StreamListeners(stream DiscoveryRequest) returns (stream DiscoveryResponse);
    rpc FetchListeners(DiscoveryRequest) returns (DiscoveryResponse);        // REST

  envoy.service.route.v3.RouteDiscoveryService
    rpc StreamRoutes(stream DiscoveryRequest) returns (stream DiscoveryResponse);
    rpc FetchRoutes(DiscoveryRequest) returns (DiscoveryResponse);

  envoy.service.cluster.v3.ClusterDiscoveryService
    rpc StreamClusters(stream DiscoveryRequest) returns (stream DiscoveryResponse);
    rpc FetchClusters(DiscoveryRequest) returns (DiscoveryResponse);

  envoy.service.endpoint.v3.EndpointDiscoveryService
    rpc StreamEndpoints(stream DiscoveryRequest) returns (stream DiscoveryResponse);
    rpc FetchEndpoints(DiscoveryRequest) returns (DiscoveryResponse);

  envoy.service.secret.v3.SecretDiscoveryService
    rpc StreamSecrets(stream DiscoveryRequest) returns (stream DiscoveryResponse);
    rpc FetchSecrets(DiscoveryRequest) returns (DiscoveryResponse);

  envoy.service.discovery.v3.AggregatedDiscoveryService
    rpc StreamAggregatedResources(stream DiscoveryRequest) returns (stream DiscoveryResponse);
    rpc DeltaAggregatedResources(stream DeltaDiscoveryRequest) returns (stream DeltaDiscoveryResponse);
```

### 3.2 Delta/Incremental gRPC Streaming

Added in Envoy 1.12. Only available over gRPC (no REST fallback).

- **Responses contain only changed/added/removed resources** — not the full set
- Client explicitly subscribes/unsubscribes to individual resources
- Server tracks per-client resource state
- More efficient for large resource sets (thousands of endpoints)
- Uses different proto messages: `DeltaDiscoveryRequest` / `DeltaDiscoveryResponse`

```
Delta gRPC Methods (same services, different RPCs):

  rpc DeltaListeners(stream DeltaDiscoveryRequest) returns (stream DeltaDiscoveryResponse);
  rpc DeltaClusters(stream DeltaDiscoveryRequest) returns (stream DeltaDiscoveryResponse);
  rpc DeltaRoutes(stream DeltaDiscoveryRequest) returns (stream DeltaDiscoveryResponse);
  rpc DeltaEndpoints(stream DeltaDiscoveryRequest) returns (stream DeltaDiscoveryResponse);
```

### 3.3 REST-JSON (Polling)

Unary HTTP endpoint, client polls periodically.

- Uses the same `DiscoveryRequest`/`DiscoveryResponse` messages, JSON-encoded (proto3 canonical JSON)
- No streaming — client must poll
- REST endpoints follow pattern: `POST /v3/discovery:{resource_type}`
- Rarely used in production — exists primarily for simple/debugging scenarios

```
REST Endpoints:

  POST /v3/discovery:listeners    (LDS)
  POST /v3/discovery:routes       (RDS)
  POST /v3/discovery:clusters     (CDS)
  POST /v3/discovery:endpoints    (EDS)
```

### Transport Selection Matrix

| Property | SotW gRPC | Delta gRPC | REST-JSON |
|----------|-----------|------------|-----------|
| Streaming | Yes | Yes | No (poll) |
| Incremental updates | No (full state) | Yes (diffs) | No |
| ADS support | Yes | Yes | No |
| Deletion semantics | Implicit (absence) | Explicit (removed_resources) | Implicit |
| Server state tracking | Minimal | Per-client subscriptions | Stateless |
| Wire efficiency (large sets) | Poor | Good | Poor |
| Implementation complexity | Low | Medium-High | Low |
| Production adoption | High (legacy default) | Growing (Istio 1.22+ default) | Rare |

---

## 4. Resource Model

### 4.1 DiscoveryRequest (SotW)

```protobuf
// envoy.service.discovery.v3.DiscoveryRequest
message DiscoveryRequest {
  // Version of resources most recently ACK'd by the client.
  // Empty on first request. Set to version_info from the last
  // successfully processed DiscoveryResponse.
  string version_info = 1;

  // Node identifier for the Envoy instance.
  // Only guaranteed populated on the FIRST request of a stream.
  // Subsequent requests on the same stream may omit it.
  config.core.v3.Node node = 2;

  // List of resource names to subscribe to.
  // For LDS/CDS: empty = wildcard (all resources).
  // For RDS/EDS/SDS: explicit list of names.
  repeated string resource_names = 3;

  // Type URL of the requested resources.
  // Identifies which xDS API when multiplexed over ADS.
  // e.g., "type.googleapis.com/envoy.config.cluster.v3.Cluster"
  string type_url = 4;

  // Nonce from the most recent DiscoveryResponse.
  // Pairs this request with the specific response being ACK'd/NACK'd.
  string response_nonce = 5;

  // Populated on NACK only. Indicates the rejected config and why.
  google.rpc.Status error_detail = 6;
}
```

### 4.2 DiscoveryResponse (SotW)

```protobuf
// envoy.service.discovery.v3.DiscoveryResponse
message DiscoveryResponse {
  // Server-assigned version for this response.
  // Opaque to client — used for ACK/NACK.
  string version_info = 1;

  // The actual resources, wrapped in google.protobuf.Any.
  // For SotW: this is the COMPLETE set of resources.
  // Absence of a previously-present resource = deletion.
  repeated google.protobuf.Any resources = 2;

  // Determines if resources are Cluster, Listener, etc.
  // Redundant with the type_url inside each Any, but
  // required for efficient demuxing in ADS.
  string type_url = 4;

  // Server-generated nonce. Client must echo it back in
  // the next DiscoveryRequest to pair ACK/NACK.
  string nonce = 5;

  // [Deprecated] Was used for canary deployments.
  bool canary = 3;

  // Control plane instance identifier for debugging.
  config.core.v3.ControlPlane control_plane = 6;
}
```

### 4.3 Node Message

```protobuf
// envoy.config.core.v3.Node
message Node {
  // Opaque node identifier. Set in Envoy bootstrap config.
  // Often: "sidecar~<ip>~<pod>.<namespace>~<namespace>.svc.cluster.local"
  string id = 1;

  // Cluster the node belongs to. Used for grouping.
  string cluster = 2;

  // Opaque metadata. Control planes use this for per-node
  // configuration decisions (labels, annotations, etc).
  google.protobuf.Struct metadata = 3;

  // Locality: region, zone, sub_zone.
  // Used for locality-aware load balancing.
  Locality locality = 4;

  // User agent name (e.g., "envoy", "grpc-go").
  string user_agent_name = 6;

  // User agent version or build version.
  oneof user_agent_version_type {
    string user_agent_version = 7;
    BuildVersion user_agent_build_version = 8;
  }

  // Client feature capabilities (e.g., "envoy.lb.does_not_support_overprovisioning").
  repeated string client_features = 10;
}
```

### 4.4 DeltaDiscoveryRequest (Incremental)

```protobuf
message DeltaDiscoveryRequest {
  config.core.v3.Node node = 1;

  string type_url = 2;

  // Resources to ADD to the subscription.
  repeated string resource_names_subscribe = 3;

  // Resources to REMOVE from the subscription.
  repeated string resource_names_unsubscribe = 4;

  // On stream reconnect: map of resource name -> last known version.
  // Allows the server to compute a diff from the client's state.
  // Only populated on the FIRST request of a reconnected stream.
  // Empty on the very first stream of a session.
  map<string, string> initial_resource_versions = 5;

  // Nonce from most recent DeltaDiscoveryResponse (for ACK/NACK).
  string response_nonce = 6;

  // Populated on NACK.
  google.rpc.Status error_detail = 7;
}
```

### 4.5 DeltaDiscoveryResponse (Incremental)

```protobuf
message DeltaDiscoveryResponse {
  // System version (informational only).
  string system_version_info = 1;

  // Changed or new resources.
  repeated Resource resources = 2;

  string type_url = 4;

  // Names of resources REMOVED since last response.
  // Explicit deletion — no "absence implies deletion" ambiguity.
  repeated string removed_resources = 6;

  string nonce = 5;

  config.core.v3.ControlPlane control_plane = 7;
}

message Resource {
  // Resource name. Unique within a type URL.
  string name = 3;

  // Resource aliases (for VHDS: host header values).
  repeated string aliases = 4;

  // Resource version. Opaque, server-assigned.
  string version = 1;

  // The actual resource payload.
  google.protobuf.Any resource = 2;

  // Optional TTL. Resource expires after this duration
  // unless refreshed by a heartbeat response.
  google.protobuf.Duration ttl = 6;

  // Cache control metadata.
  CacheControl cache_control = 8;
}
```

---

## 5. ACK/NACK Flow

The xDS protocol uses an explicit acknowledgment mechanism. Every `DiscoveryResponse` from
the server requires a corresponding `DiscoveryRequest` from the client indicating whether
the configuration was accepted (ACK) or rejected (NACK).

### SotW ACK/NACK

```
  Client                                 Server
    |                                      |
    |--- DiscoveryRequest ----------------->|  (initial: version_info="", nonce="")
    |                                      |
    |<-- DiscoveryResponse ----------------|  (version_info="v1", nonce="A")
    |    {resources: [cluster-a, cluster-b]}|
    |                                      |
    |--- DiscoveryRequest ----------------->|  ACK: version_info="v1", nonce="A"
    |                                      |     (accepted v1)
    |                                      |
    |<-- DiscoveryResponse ----------------|  (version_info="v2", nonce="B")
    |    {resources: [cluster-a, cluster-c]}|  (cluster-b removed, cluster-c added)
    |                                      |
    |--- DiscoveryRequest ----------------->|  NACK: version_info="v1", nonce="B",
    |                                      |     error_detail={...}
    |                                      |     (rejected v2, still on v1)
    |                                      |
```

### ACK Rules

| Field | ACK | NACK |
|-------|-----|------|
| `version_info` | Set to `version_info` from accepted response | Set to `version_info` of **last accepted** response |
| `response_nonce` | Set to `nonce` from the response being ACK'd | Set to `nonce` from the response being NACK'd |
| `error_detail` | Absent / empty | Populated with rejection reason |

### Critical Protocol Rules

1. **No new request until ACK/NACK** — Client must not send a new request on a stream until it has processed and ACK'd or NACK'd the most recent response.

2. **Nonce prevents stale responses** — The server should not send a new `DiscoveryResponse` for a request that has a stale nonce. The nonce pairs requests to responses unambiguously.

3. **NACK keeps old config** — On NACK, the client continues using the last successfully applied configuration. The server may retry with corrected config.

4. **Node only on first request** — Only the first `DiscoveryRequest` on a stream is guaranteed to carry the `node` identifier. Subsequent requests may omit it. The server must associate the node with the stream.

5. **Resource names can change per request** — The client can modify its subscription set (the `resource_names` list) in any ACK/NACK request. This is how Envoy dynamically subscribes to new EDS clusters referenced by newly received CDS config.

### Delta ACK/NACK

The delta protocol ACK/NACK works similarly but uses `response_nonce` from `DeltaDiscoveryResponse`. The key difference: there is no `version_info` field. Instead, the server tracks per-resource versions internally.

```
  Client                                 Server
    |                                      |
    |--- DeltaDiscoveryRequest ------------>|  subscribe: ["cluster-a"]
    |                                      |
    |<-- DeltaDiscoveryResponse ------------|  resources: [{name:"cluster-a", version:"1"}]
    |                                      |  nonce: "X"
    |                                      |
    |--- DeltaDiscoveryRequest ------------>|  ACK: nonce="X"
    |    subscribe: ["cluster-b"]          |  (also subscribing to cluster-b)
    |                                      |
    |<-- DeltaDiscoveryResponse ------------|  resources: [{name:"cluster-b", version:"1"}]
    |                                      |  nonce: "Y"
    |                                      |
    |--- DeltaDiscoveryRequest ------------>|  ACK: nonce="Y"
    |                                      |
    |<-- DeltaDiscoveryResponse ------------|  removed_resources: ["cluster-a"]
    |                                      |  nonce: "Z"
    |                                      |
    |--- DeltaDiscoveryRequest ------------>|  ACK: nonce="Z"
    |                                      |
```

---

## 6. Delta/Incremental Protocol

### Why Delta Exists

SotW sends the **entire** resource set on every update. For a fleet with 10,000 endpoints,
even changing one endpoint means re-serializing and transmitting all 10,000. Delta solves
this with incremental updates.

### Key Differences from SotW

| Aspect | SotW | Delta |
|--------|------|-------|
| Response content | Complete resource set | Only changed/added/removed |
| Deletion signal | Absence from response | Explicit `removed_resources` list |
| Subscription | `resource_names` in request | `resource_names_subscribe` / `resource_names_unsubscribe` |
| Versioning | Single `version_info` string | Per-resource `version` + `initial_resource_versions` map |
| Stream reconnection | Server re-sends everything | Client sends `initial_resource_versions`; server computes diff |
| Transport | gRPC or REST | gRPC only |

### Stream Reconnection with initial_resource_versions

When a delta stream is disconnected and reconnected, the client sends its known resource
state in the first request:

```
DeltaDiscoveryRequest {
  node: { id: "envoy-1" }
  type_url: "type.googleapis.com/envoy.config.cluster.v3.Cluster"
  initial_resource_versions: {
    "cluster-a": "v3",
    "cluster-b": "v7",
    "cluster-c": "v2"
  }
}
```

The server compares this against its current state and sends only the diff — resources
that changed since those versions, plus any new resources, plus removals.

### Istio's Adoption of Delta xDS

As of Istio 1.22 (2024), delta xDS is enabled by default for all proxy communication.
This was motivated by:
- Reduced memory pressure on istiod (no need to serialize full state per push)
- Reduced network bandwidth in large meshes
- Faster propagation of endpoint changes

---

## 7. ADS — Aggregated Discovery Service

### The Problem ADS Solves

Without ADS, each resource type uses an independent gRPC stream, potentially to different
management servers. This means:
- CDS and EDS updates arrive independently — race conditions possible
- A new cluster might be referenced before its endpoints are known
- No ordering guarantee across types

### How ADS Works

ADS multiplexes **all** resource types onto a **single** gRPC stream to a **single** management
server. The `type_url` field in each request/response identifies which xDS API the message
belongs to.

```
  Client                                 Server (ADS)
    |                                      |
    |=== Single gRPC Stream ===============|
    |                                      |
    |--- DiscoveryRequest (type=CDS) ----->|
    |<-- DiscoveryResponse (type=CDS) -----|  {clusters: [c1, c2]}
    |--- DiscoveryRequest (ACK CDS) ------>|
    |                                      |
    |--- DiscoveryRequest (type=EDS) ----->|  (subscribing to c1, c2 endpoints)
    |<-- DiscoveryResponse (type=EDS) -----|  {endpoints for c1, c2}
    |--- DiscoveryRequest (ACK EDS) ------>|
    |                                      |
    |--- DiscoveryRequest (type=LDS) ----->|
    |<-- DiscoveryResponse (type=LDS) -----|  {listeners referencing routes}
    |--- DiscoveryRequest (ACK LDS) ------>|
    |                                      |
    |--- DiscoveryRequest (type=RDS) ----->|  (subscribing to route configs)
    |<-- DiscoveryResponse (type=RDS) -----|
    |--- DiscoveryRequest (ACK RDS) ------>|
    |                                      |
```

### ADS Ordering Guarantee

With ADS, the management server can enforce the canonical ordering:

```
  CDS → EDS → LDS → RDS
  (clusters first, then endpoints, then listeners, then routes)
```

The server controls the send order on the single stream, ensuring that:
1. CDS updates are delivered first (cluster definitions)
2. EDS updates arrive after their referenced CDS clusters
3. LDS updates arrive after CDS/EDS for clusters they reference
4. RDS updates arrive after LDS for listeners that reference them

### ADS vs Separate Streams

| Property | Separate Streams | ADS |
|----------|-----------------|-----|
| Ordering guarantee | None (eventually consistent) | Server-controlled |
| Number of streams | One per type URL | Single stream |
| Management servers | Can be different per type | Must be single server |
| Scalability | Better (parallel) | Bottleneck on single stream |
| Use case | Simple, few resources | Complex, ordering-sensitive |

---

## 8. Resource Ordering and Warming

### The Make-Before-Break Rule

xDS updates follow a **make-before-break** model to minimize traffic disruption:

```
  Correct ordering for adding a new service:

  1. CDS  — Add the new cluster definition
  2. EDS  — Provide endpoints for the new cluster
  3. LDS  — Add/update listener that will route to the new cluster
  4. RDS  — Add route rules pointing to the new cluster

  Correct ordering for removing a service:

  1. RDS  — Remove routes pointing to the cluster
  2. LDS  — Update/remove listeners
  3. EDS  — (stale, can be removed)
  4. CDS  — Remove the cluster definition
```

### Warming

Warming is the process by which Envoy ensures a resource is fully ready before it starts
serving traffic through it. Not all resources go through warming.

**Resources that ARE warmed:**
- **Clusters** — A new or updated cluster is warmed by waiting for its EDS response
  (if type=EDS). The cluster will not receive traffic until warming completes.
- **Listeners** — A new listener waits for its referenced RDS routes to arrive before
  being marked active. Envoy uses the previously known RouteConfiguration if available.

**Resources that are NOT warmed:**
- **Routes** — Route updates take effect immediately. The management plane must ensure
  that clusters referenced by a route already exist before pushing the route update.
  This is why CDS must precede RDS.

```
  Cluster Warming Flow:

  CDS response (new cluster "my-svc", type=EDS)
       |
       v
  Envoy creates cluster in WARMING state
       |
       v
  Envoy sends EDS request for "my-svc"
       |
       v
  EDS response with endpoints for "my-svc"
       |
       v
  Cluster transitions to ACTIVE state
       |
       v
  Traffic can now be routed to "my-svc"
```

### Warming and Initialization

During Envoy startup, the proxy will not mark itself as live/ready until:
- All CDS clusters have been warmed (EDS responses received)
- All LDS listeners have been warmed (RDS responses received)

If the management server fails to provide EDS/RDS responses, Envoy will not complete
initialization. This is intentional — it prevents serving traffic with incomplete
configuration.

### Eventually Consistent Reality

Despite ordering conventions, xDS is **eventually consistent** by design:

- There is no atomic update across all proxies in a fleet
- Each proxy applies updates independently
- Brief traffic drops are possible during updates (e.g., route references a cluster
  that hasn't arrived yet)
- The management server should minimize this window by respecting ordering, but
  cannot eliminate it entirely in disaggregated (non-ADS) mode

---

## 9. Wildcard and Resource Naming

### Subscription Modes

**Wildcard subscription** — Client subscribes to all resources of a type. Used for LDS
and CDS where Envoy needs to know about everything the control plane has.
- SotW: send `resource_names: []` (empty list)
- Delta: send `resource_names_subscribe: ["*"]`

**Explicit subscription** — Client names specific resources. Used for RDS, EDS, SDS,
ECDS where Envoy only needs specific resources referenced by other config.
- SotW: send `resource_names: ["route-config-1", "route-config-2"]`
- Delta: send `resource_names_subscribe: ["route-config-1"]`

### Resource Naming: xdstp:// Scheme

A structured naming scheme was introduced to support federated xDS, namespace delegation,
and caching:

```
xdstp://{authority}/{resource_type}/{id}?{context_params}

Examples:
  xdstp://my-control-plane.example.com/envoy.config.cluster.v3.Cluster/my-cluster
  xdstp:///envoy.config.listener.v3.Listener/my-listener    (empty authority = default)
```

### Glob Collections

For delta xDS, glob collections allow subscribing to a directory of resources:

```
xdstp://authority/envoy.config.cluster.v3.Cluster/prod/*

  This subscribes to all clusters under the "prod/" prefix.
  Supported for LDS, CDS, and SRDS over delta gRPC.
```

---

## 10. go-control-plane — Reference Implementation

The [envoyproxy/go-control-plane](https://github.com/envoyproxy/go-control-plane) library
is the canonical Go implementation for building xDS management servers. It provides the
gRPC server implementation, cache abstractions, and proto-generated types.

### Architecture Overview

```
  +-------------------+      +-------------------+      +------------------+
  |  Your Control     |      |  go-control-plane |      |  Envoy / gRPC    |
  |  Plane Logic      |      |  Cache + Server   |      |  Client          |
  |                   |      |                   |      |                  |
  |  Watches K8s,     | ---> |  SnapshotCache    | ---> |  Receives xDS    |
  |  computes config, |      |  LinearCache      |      |  responses via   |
  |  sets snapshots   |      |  MuxCache         |      |  gRPC streams    |
  +-------------------+      +-------------------+      +------------------+
```

### Key Interfaces

```go
// Cache is the top-level interface combining watch and fetch capabilities.
// This is what you pass to the xDS server.
type Cache interface {
    ConfigWatcher      // For streaming (gRPC)
    ConfigFetcher      // For unary (REST)
}

// ConfigWatcher is the streaming interface — the core of xDS.
// CreateWatch returns a channel that receives responses.
type ConfigWatcher interface {
    CreateWatch(request *Request, state stream.StreamState, responses chan Response) (cancel func())
    CreateDeltaWatch(request *DeltaRequest, state stream.StreamState, responses chan DeltaResponse) (cancel func())
}

// NodeHash computes a string key for grouping Envoy nodes.
// Nodes with the same hash get the same configuration.
type NodeHash interface {
    ID(node *core.Node) string
}

// Snapshot represents a versioned point-in-time configuration.
type Snapshot interface {
    GetVersion(typeURL string) string
    GetResources(typeURL string) map[string]types.Resource
    GetResourcesAndTTL(typeURL string) map[string]types.ResourceWithTTL
    ConstructVersionMap() error
}
```

### SnapshotCache — The Default Choice

SnapshotCache maintains a single versioned snapshot per node group. It is the simplest
and most commonly used cache implementation.

```go
// Create a snapshot cache.
// ADS flag: when true, delays responses until snapshot is consistent
// (all referenced RDS/EDS resources are present).
cache := cache.NewSnapshotCache(
    true,                    // ads: enforce consistency
    cache.IDHash{},          // NodeHash: use node.Id as the key
    logger,                  // optional logger
)

// Build a snapshot with all resource types.
snapshot, _ := cache.NewSnapshot(
    "v1",                    // version string
    map[resource.Type][]types.Resource{
        resource.ClusterType:  {makeCluster("my-cluster")},
        resource.EndpointType: {makeEndpoint("my-cluster", "10.0.0.1", 8080)},
        resource.ListenerType: {makeListener("my-listener")},
        resource.RouteType:    {makeRoute("my-route")},
        resource.SecretType:   {},  // no secrets
        resource.RuntimeType:  {},  // no runtime
    },
)

// Assign the snapshot to a node group.
cache.SetSnapshot(context.Background(), "node-group-1", snapshot)
```

**ADS mode behavior**: When the ADS flag is true, SnapshotCache delays responding to
EDS/RDS requests until all resources referenced by the snapshot's CDS/LDS responses
are named in the request. This ensures atomic, consistent updates.

### LinearCache — For High-Cardinality Types

LinearCache is designed for resource types with high churn and many entries (typically EDS).
It maintains a single collection indexed by resource name with internal version tracking.

```go
// Create a linear cache for endpoints.
lc := cache.NewLinearCache(
    resource.EndpointType,   // single type URL
    cache.WithInitialResources(initialEndpoints),
)

// Update a single resource — efficient, no full snapshot rebuild.
lc.UpdateResource("cluster-a", makeEndpoint("cluster-a", "10.0.0.2", 8080))

// Delete a resource.
lc.DeleteResource("cluster-b")
```

**Key property**: Eventually consistent. Maintains a linear version history and a version
vector. For each request, compares the request version against latest versions for the
requested resources and responds with any updates.

### MuxCache — Combining Caches

MuxCache routes requests to different cache implementations based on type URL.
Classic pattern: SnapshotCache for LDS/RDS/CDS, LinearCache for EDS.

```go
mux := &cache.MuxCache{
    Classify: func(req *cache.Request) string {
        return req.TypeUrl  // route by type URL
    },
    ClassifyDelta: func(req *cache.DeltaRequest) string {
        return req.TypeUrl
    },
    Caches: map[string]cache.Cache{
        resource.ListenerType: snapshotCache,
        resource.RouteType:    snapshotCache,
        resource.ClusterType:  snapshotCache,
        resource.EndpointType: linearCache,   // high-cardinality
    },
}
```

### Server Setup

```go
// Create the xDS server.
srv := server.NewServer(
    context.Background(),
    snapshotCache,           // or mux cache
    &callbacks{},            // lifecycle callbacks
)

// Register with a gRPC server.
grpcServer := grpc.NewServer()

// Register individual services (disaggregated mode):
discoverygrpc.RegisterAggregatedDiscoveryServiceServer(grpcServer, srv)
listenerservice.RegisterListenerDiscoveryServiceServer(grpcServer, srv)
clusterservice.RegisterClusterDiscoveryServiceServer(grpcServer, srv)
endpointservice.RegisterEndpointDiscoveryServiceServer(grpcServer, srv)
routeservice.RegisterRouteDiscoveryServiceServer(grpcServer, srv)
secretservice.RegisterSecretDiscoveryServiceServer(grpcServer, srv)

grpcServer.Serve(lis)
```

### Callbacks — Lifecycle Hooks

```go
type Callbacks interface {
    OnStreamOpen(ctx context.Context, streamID int64, typeURL string) error
    OnStreamClosed(streamID int64, node *core.Node)
    OnStreamRequest(streamID int64, request *discovery.DiscoveryRequest) error
    OnStreamResponse(ctx context.Context, streamID int64, request *discovery.DiscoveryRequest, response *discovery.DiscoveryResponse)
    OnDeltaStreamOpen(ctx context.Context, streamID int64, typeURL string) error
    OnDeltaStreamClosed(streamID int64, node *core.Node)
    OnStreamDeltaRequest(streamID int64, request *discovery.DeltaDiscoveryRequest) error
    OnStreamDeltaResponse(streamID int64, request *discovery.DeltaDiscoveryRequest, response *discovery.DeltaDiscoveryResponse)
}
```

Callbacks enable: access control, metrics, logging, rate limiting, and custom validation
at the xDS protocol level.

---

## 11. Control Plane Patterns in Production

### 11.1 Istio (istiod)

**Architecture**: Single binary (istiod) consolidating Pilot, Citadel, and Galley. Watches
Kubernetes API server for VirtualService, DestinationRule, Gateway, ServiceEntry, and
other Istio CRDs plus native Service/Endpoint resources.

**xDS Implementation**: Custom xDS server (not using go-control-plane's SnapshotCache).
Istio implements its own `DiscoveryServer` in `pilot/pkg/xds/discovery.go`.

**Key patterns**:
- **PushContext** — A cached, pre-computed state object containing all information needed
  to generate xDS config for any proxy. Rebuilt on mesh state changes. Acts as the
  "snapshot" equivalent but is shared across all proxies (with per-proxy filtering at
  generation time).
- **Debouncing** — Config changes are debounced (default 100ms, max 10s) before triggering
  a push. Multiple rapid changes are batched into a single push cycle.
- **Generator pattern** — Type-specific generators (CDS generator, EDS generator, etc.)
  produce xDS resources from PushContext. Generators are selected based on proxy type
  (sidecar vs gateway) and requested type URL.
- **Delta xDS (default since 1.22)** — Istio switched to delta xDS as the default transport,
  reducing memory and bandwidth for large meshes.
- **Per-proxy filtering** — Unlike go-control-plane's SnapshotCache (which serves the
  same snapshot to a node group), Istio generates config per-proxy, filtering to only
  relevant clusters/routes/listeners for that specific sidecar.

```
  Kubernetes API Server
         |
    (watch events)
         |
         v
  +------------------+
  |     istiod        |
  |                  |
  |  Config Store    |  <-- VirtualService, DestinationRule, Gateway, etc.
  |  Service Registry|  <-- Service, Endpoints, Pods
  |                  |
  |  debounce (100ms)|
  |       |          |
  |  PushContext     |  <-- Pre-computed mesh state
  |       |          |
  |  Generators      |  <-- CDS/EDS/LDS/RDS per proxy type
  |       |          |
  |  xDS Server      |  <-- gRPC streams to all proxies
  +--------+---------+
           |
    (Delta xDS / ADS)
           |
     +-----+-----+
     |     |     |
   proxy proxy proxy
```

### 11.2 Envoy Gateway

**Architecture**: Kubernetes-native gateway using the Gateway API. Translates Gateway API
resources (GatewayClass, Gateway, HTTPRoute, etc.) into Envoy configuration.

**xDS Implementation**: Uses go-control-plane's server and cache directly. Delta xDS
protocol is the transport.

**Key patterns**:
- **Intermediate Representation (IR)** — Two-stage translation: Gateway API resources are
  first translated into an xDS IR (intermediate representation), then the xDS IR is
  translated into actual Envoy xDS resources. This decouples the input API from xDS
  specifics.
- **xDS Translator** — Converts xDS IR into go-control-plane resource types, then pushes
  them into the snapshot cache.
- **Extension hooks** — Hooks at Cluster, VirtualHost, and HTTPListener stages of translation
  allow external gRPC servers to modify xDS config before it reaches Envoy.

```
  Gateway API Resources (GatewayClass, Gateway, HTTPRoute)
         |
    (Kubernetes watch)
         |
         v
  +------------------+
  |  Envoy Gateway   |
  |                  |
  |  Resource Watcher|
  |       |          |
  |  xDS IR          |  <-- Intermediate Representation
  |       |          |
  |  xDS Translator  |  <-- IR -> Envoy xDS resources
  |       |          |
  |  Extension Hooks |  <-- External gRPC callouts
  |       |          |
  |  go-control-plane|  <-- SnapshotCache + Delta Server
  +--------+---------+
           |
     (Delta xDS)
           |
     Envoy Proxy fleet
```

### 11.3 Cilium

**Architecture**: eBPF-first networking with Envoy as the L7 proxy. Cilium agent manages
an embedded Envoy instance per node using xDS.

**xDS Implementation**: Custom xDS server (`pkg/envoy/xds_server.go`) — not go-control-plane
SnapshotCache. Directly manages resource state and pushes to the local Envoy instance.

**Key patterns**:
- **CiliumEnvoyConfig CRD** — Kubernetes CRD that declaratively specifies Envoy
  configuration (listeners, routes, clusters, filters). Cilium agent watches these
  and programs local Envoy via xDS.
- **Per-node xDS** — Unlike Istio (one control plane, many proxies), Cilium runs an xDS
  server per node for its local Envoy. Tighter coupling, lower latency.
- **Custom filters** — Cilium extends Envoy with custom filters (`cilium:bpf_metadata`,
  `cilium:network_filter`, `cilium:l7policy`) that integrate with the BPF datapath.
- **Policy-driven** — L7 network policies trigger xDS config generation. The xDS server
  translates CiliumNetworkPolicy into Envoy filter chain configuration.

```
  CiliumEnvoyConfig CRD        CiliumNetworkPolicy
         |                            |
    (K8s watch)                  (K8s watch)
         |                            |
         v                            v
  +------+----------------------------+------+
  |              Cilium Agent                |
  |                                          |
  |  XDSServer (per-node)                    |
  |       |                                  |
  |  xDS resources (LDS, RDS, CDS, etc.)     |
  +--------+---------------------------------+
           |
     (xDS over UDS)
           |
     Envoy (per-node)
       + cilium custom filters
       + BPF datapath integration
```

---

## 12. Key Design Decisions

### 12.1 Eventually Consistent by Design

xDS deliberately chose eventual consistency over strong consistency. Implications:

- **No atomic fleet-wide updates** — Each proxy applies updates independently
- **Brief inconsistency windows** — Proxies may have different views of configuration
  for short periods
- **Simplicity and scalability** — No distributed consensus needed in the control plane
- **Traffic drops are possible** — During config transitions, a route may reference a
  cluster that hasn't been delivered yet

This matches the reality of distributed systems: perfect consistency across a fleet of
proxies is impractical at scale.

### 12.2 Type Ordering Dependencies

The dependency graph creates ordering constraints:

```
  CDS must arrive before EDS  (cluster must exist before endpoints populate it)
  CDS must arrive before RDS  (routes reference clusters by name)
  LDS must arrive after CDS   (listener filter chains may reference clusters)
  RDS must arrive after LDS   (routes are referenced by listeners)
  SDS should arrive before    (TLS needed before listener can accept connections)
    LDS/CDS that reference it
```

**Without ADS**: These are best-effort. Separate streams have no ordering. The management
server should send CDS first, but can't guarantee arrival order.

**With ADS**: The server controls message ordering on a single stream, making these
guarantees enforceable.

### 12.3 Warming Prevents Premature Traffic

Warming is the protocol's answer to the "reference before define" problem. By holding new
clusters and listeners in a warming state until their dependencies arrive, Envoy avoids
routing traffic to incomplete configurations.

**Cost**: Warming adds latency to configuration updates. A new cluster isn't usable until
its EDS response arrives. If the management server is slow or partitioned, warming can
block indefinitely (with timeout controls).

### 12.4 SotW vs Delta: When to Choose

**Use SotW when**:
- Resource count per type is small (< hundreds)
- Implementation simplicity matters
- You want implicit deletion semantics (simple)
- REST fallback is needed

**Use Delta when**:
- Resource count is high (thousands of endpoints)
- Bandwidth efficiency matters (large meshes)
- You want explicit deletion (safer, auditable)
- You need on-demand / lazy-loading (VHDS)
- Stream reconnection should be efficient (only send diffs)

### 12.5 SnapshotCache vs Custom Implementation

**Use SnapshotCache (go-control-plane) when**:
- All nodes in a group get identical configuration
- Configuration changes are infrequent relative to number of connected proxies
- You want the simplest possible control plane
- ADS consistency guarantees are important

**Build a custom cache when**:
- Each proxy needs different configuration (Istio's per-proxy generation)
- Resource cardinality is extremely high (LinearCache for EDS)
- You need fine-grained push control (debouncing, partial pushes)
- You need custom grouping/filtering logic beyond simple node hashing

### 12.6 Domain-Specific API Above xDS

Every production control plane interposes a **domain-specific configuration API** between
users and raw xDS:

| Control Plane | User-Facing API | Internal xDS |
|---------------|----------------|-------------|
| Istio | VirtualService, DestinationRule | LDS, RDS, CDS, EDS |
| Envoy Gateway | Gateway API (HTTPRoute, Gateway) | LDS, RDS, CDS, EDS |
| Cilium | CiliumEnvoyConfig, CiliumNetworkPolicy | LDS, RDS, CDS, EDS |

Users never write raw xDS. The control plane translates domain concepts into xDS resources.
This translation layer is where most of the control plane complexity lives.

### 12.7 The Control Plane is the Hard Part

The xDS protocol itself is well-specified and relatively simple. The hard part is:

1. **What to generate** — Translating high-level intent into correct xDS configuration
2. **When to push** — Debouncing, batching, prioritizing updates
3. **To whom** — Per-proxy filtering, node grouping, scope limiting
4. **Ordering** — Ensuring CDS arrives before EDS in practice, not just in theory
5. **Performance** — Handling thousands of proxies, millions of endpoints
6. **Correctness** — Ensuring no dangling references, no stale config, no traffic black holes

---

## 13. Sources

### Primary Documentation
- [xDS REST and gRPC Protocol](https://www.envoyproxy.io/docs/envoy/latest/api-docs/xds_protocol) — Canonical protocol specification
- [xDS Configuration API Overview](https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/operations/dynamic_configuration) — Architecture overview
- [xDS API Endpoints](https://www.envoyproxy.io/docs/envoy/latest/configuration/overview/xds_api) — Endpoint reference
- [Common Discovery API Components (Proto)](https://www.envoyproxy.io/docs/envoy/latest/api-v3/service/discovery/v3/discovery.proto) — Proto definitions
- [Extension Configuration (ECDS)](https://www.envoyproxy.io/docs/envoy/latest/configuration/overview/extension) — ECDS documentation
- [Secret Discovery Service (SDS)](https://www.envoyproxy.io/docs/envoy/latest/configuration/security/secret) — SDS documentation
- [Virtual Host Discovery Service (VHDS)](https://www.envoyproxy.io/docs/envoy/latest/configuration/http/http_conn_man/vhds) — VHDS documentation

### Reference Implementations
- [envoyproxy/go-control-plane](https://github.com/envoyproxy/go-control-plane) — Go reference implementation
- [go-control-plane cache package](https://pkg.go.dev/github.com/envoyproxy/go-control-plane/pkg/cache/v3) — Cache interfaces and implementations
- [go-control-plane server package](https://pkg.go.dev/github.com/envoyproxy/go-control-plane/pkg/server/v3) — Server implementation
- [go-control-plane example](https://github.com/envoyproxy/go-control-plane/tree/main/internal/example) — Example xDS server
- [CNCF xDS](https://github.com/cncf/xds) — CNCF-hosted xDS specification

### Control Plane Implementations
- [Istio DiscoveryServer](https://github.com/istio/istio/blob/master/pilot/pkg/xds/discovery.go) — Istio's xDS server
- [Istio xDS package](https://pkg.go.dev/istio.io/istio/pilot/pkg/xds) — Istio xDS Go package
- [Istio Delta xDS (Default in 1.22)](https://tetrate.io/blog/istio-service-mesh-delta-xds) — Delta adoption
- [Introducing istiod](https://istio.io/latest/blog/2020/istiod/) — Control plane consolidation
- [Envoy Gateway System Design](https://gateway.envoyproxy.io/contributions/design/system-design/) — EG architecture
- [Envoy Gateway Control Plane](https://aigateway.envoyproxy.io/docs/concepts/architecture/control-plane/) — EG control plane concepts
- [Cilium Envoy Integration](https://docs.cilium.io/en/latest/security/network/proxy/envoy/) — Cilium xDS integration
- [Cilium xds_server.go](https://github.com/cilium/cilium/blob/v1.15.4/pkg/envoy/xds_server.go) — Cilium xDS server source
- [Scaling Cilium with xDS (Solo.io)](https://www.solo.io/blog/scaling-cilium-to-new-heights-with-xds) — Cilium xDS scaling

### Guidance and Analysis
- [Guidance for Building a Control Plane for Envoy](https://blog.christianposta.com/envoy/guidance-for-building-a-control-plane-to-manage-envoy-proxy-based-infrastructure/) — Christian Posta's series (Part 1)
- [Part 2: Identify Components](https://blog.christianposta.com/envoy/guidance-for-building-a-control-plane-for-envoy-identify-components/)
- [Part 3: Domain-Specific Configuration API](https://blog.christianposta.com/envoy/guidance-for-building-a-control-plane-for-envoy-domain-specific-configuration-api/)
- [Part 4: Build for Extensibility](https://blog.christianposta.com/guidance-for-building-a-control-plane-for-envoy-build-for-pluggability/)
- [Part 5: Deployment Tradeoffs](https://blog.christianposta.com/guidance-for-building-a-control-plane-for-envoy-deployment-tradeoffs/)
- [envoyproxy/data-plane-api DeepWiki](https://deepwiki.com/envoyproxy/data-plane-api/8-api-discovery-services-(xds)) — API analysis
- [Introduction to xDS (Medium)](https://medium.com/@rajithacharith/introduction-to-envoys-dynamic-resource-discovery-xds-protocol-d340032a63b4) — Protocol introduction
- [Improving Istio Propagation Delay (Airbnb)](https://medium.com/airbnb-engineering/improving-istio-propagation-delay-d4da9b5b9f90) — Real-world optimization

### Rust Ecosystem
- [envoy-types crate (docs.rs)](https://docs.rs/envoy-types/latest/i686-pc-windows-msvc/envoy_types/pb/envoy/service/discovery/v3/struct.DiscoveryRequest.html) — Rust xDS proto types
