# Proxyless gRPC with xDS: Comprehensive Reference

> Service mesh without the sidecar. gRPC clients speak xDS natively, connecting
> directly to the control plane for configuration, load balancing, and security.

**Last updated**: 2026-02-20
**Status**: Reference document for architectural decisions on service mesh topology.

---

## Table of Contents

1. [What Proxyless Means](#1-what-proxyless-means)
2. [xDS in gRPC](#2-xds-in-grpc)
3. [Architecture](#3-architecture)
4. [Language Support](#4-language-support)
5. [Traffic Management](#5-traffic-management)
6. [Security](#6-security)
7. [When to Use Proxyless vs Sidecar](#7-when-to-use-proxyless-vs-sidecar)
8. [Istio + Proxyless gRPC](#8-istio--proxyless-grpc)
9. [Limitations](#9-limitations)
10. [Decision Framework](#10-decision-framework)

---

## 1. What Proxyless Means

### The Core Idea

In a traditional service mesh (Istio, Linkerd), every pod gets a sidecar proxy (Envoy). All traffic flows through this proxy, which handles load balancing, retries, mTLS, observability, and policy enforcement.

**Proxyless gRPC eliminates the sidecar.** The gRPC library itself implements the xDS client protocol, connecting directly to the mesh control plane to receive configuration. The gRPC client/server then enforces that configuration natively -- load balancing, mTLS, retries -- without any intermediate proxy.

```
SIDECAR MODEL                          PROXYLESS MODEL

+----------+    +-------+              +----------+
| gRPC     |--->| Envoy |---> network  | gRPC     |---> network
| client   |    | proxy |              | client   |
+----------+    +-------+              | (xDS     |
                    ^                  |  built-in)|
                    |                  +----------+
               Control Plane               ^
               (Istiod)                    |
                                      Control Plane
                                      (Istiod)
```

### What Gets Eliminated

| Component | Sidecar | Proxyless |
|-----------|---------|-----------|
| Envoy sidecar per pod | Yes | No |
| Extra network hop (client->proxy) | Yes | No |
| Extra network hop (proxy->server) | Yes | No |
| iptables / netfilter rules | Yes | No |
| Proxy memory (~50-100 MiB) | Yes | No |
| Proxy CPU overhead | Yes | No |

### What Remains

Even in proxyless mode, a lightweight **istio-agent** (pilot-agent) still runs as a sidecar container. It does NOT proxy traffic. It only:

- Communicates with the control plane (Istiod) on behalf of the gRPC app.
- Generates and writes the xDS bootstrap configuration file.
- Manages certificate rotation (SDS).
- Uses less than 0.1% vCPU and ~25 MiB memory (less than half of Envoy).

```
PROXYLESS ARCHITECTURE (detailed)

+--------------------------------------------+
| Pod                                        |
|                                            |
|  +------------------+  +-----------------+ |
|  | gRPC Application |  | istio-agent     | |
|  |                  |  | (pilot-agent)   | |
|  | xDS client  <-------+ bootstrap.json  | |
|  | (built-in)       |  |                 | |
|  | - LB decisions   |  | - cert rotation | |
|  | - mTLS handshake  |  | - xDS bootstrap | |
|  | - retry logic     |  | - health check  | |
|  | - RBAC enforce    |  |                 | |
|  +--------|---------+  +--------+--------+ |
|           |                      |          |
+-----------+----------------------+----------+
            |                      |
            v                      v
     +------+------+       +------+------+
     | Peer gRPC   |       | Istiod      |
     | Service     |       | (xDS server)|
     +-------------+       +-------------+
```

---

## 2. xDS in gRPC

### The xDS Resource Chain

xDS (x Discovery Service) is the protocol Envoy uses to receive configuration from a control plane. gRPC implements a subset of this protocol natively. The resource chain flows top-down:

```
LDS (Listener Discovery Service)
 |
 +---> RDS (Route Discovery Service)
        |
        +---> CDS (Cluster Discovery Service)
               |
               +---> EDS (Endpoint Discovery Service)
```

### Resource Descriptions

**LDS -- Listener Discovery Service**
- Entry point for client configuration.
- Specifies which HTTP filters to apply (fault injection, RBAC).
- Points to an RDS route configuration.
- For xDS-enabled gRPC servers: configures server-side TLS and HTTP filters.

**RDS -- Route Discovery Service**
- Provides route configuration: virtual hosts, route rules, match predicates.
- Determines which cluster handles a request based on method name, headers, etc.
- Supports weighted clusters for traffic splitting (canary, A/B).
- Points to one or more CDS clusters.

**CDS -- Cluster Discovery Service**
- Configures a logical cluster of endpoints.
- Specifies load balancing policy (round_robin, ring_hash).
- Configures circuit breaking thresholds.
- Configures outlier detection (ejection).
- Points to EDS for endpoint resolution.

**EDS -- Endpoint Discovery Service**
- Returns the set of actual backend addresses (IP:port) for a cluster.
- Includes locality information (region/zone/subzone) for locality-aware routing.
- Includes endpoint weights and health status.
- Can instruct clients to drop a percentage of traffic.

### Additional xDS Resources in gRPC

| Resource | Purpose | Status |
|----------|---------|--------|
| **LDS** | Listener config, filter chains | Supported |
| **RDS** | Route matching, traffic splitting | Supported |
| **CDS** | Cluster config, LB policy | Supported |
| **EDS** | Endpoint addresses, weights | Supported |
| **SDS** | Secret (certificate) discovery | NOT supported (uses certificate providers instead) |
| **LEDS** | Locality endpoint discovery | Not supported |
| **SRDS** | Scoped route discovery | Not supported |

**Critical note on SDS**: gRPC does **not** support Envoy's SDS (Secret Discovery Service) for certificate management. If `validation_context_sds_secret_config` is set in CDS, gRPC will **NACK** the entire update. Instead, gRPC uses its own **certificate provider plugin framework** (see Section 6).

### ADS (Aggregated Discovery Service)

gRPC uses ADS -- a single gRPC stream to the control plane that multiplexes all xDS resource types. This simplifies connection management and ensures ordering guarantees across resource types.

---

## 3. Architecture

### Bootstrap Configuration

Before any xDS resource can be fetched, the gRPC client/server must be initialized with a **bootstrap configuration**. This is provided via:

- Environment variable `GRPC_XDS_BOOTSTRAP` -- path to a JSON file.
- Environment variable `GRPC_XDS_BOOTSTRAP_CONFIG` -- inline JSON string.

#### Complete Bootstrap File Structure

```json
{
  "xds_servers": [
    {
      "server_uri": "xds-control-plane.example.com:18000",
      "channel_creds": [
        {
          "type": "google_default"
        }
      ],
      "server_features": ["xds_v3"]
    }
  ],

  "node": {
    "id": "sidecar~10.244.0.5~my-service-7b8c9d-abc12.default~default.svc.cluster.local",
    "metadata": {
      "INSTANCE_IPS": "10.244.0.5",
      "NAMESPACE": "default",
      "CLUSTER_ID": "Kubernetes"
    },
    "locality": {
      "region": "us-central1",
      "zone": "us-central1-a"
    }
  },

  "certificate_providers": {
    "default": {
      "plugin_name": "file_watcher",
      "config": {
        "certificate_file": "/var/run/secrets/workload-spiffe-credentials/certificates.pem",
        "private_key_file": "/var/run/secrets/workload-spiffe-credentials/private_key.pem",
        "ca_certificate_file": "/var/run/secrets/workload-spiffe-credentials/ca_certificates.pem",
        "refresh_interval": "600s"
      }
    }
  },

  "server_listener_resource_name_template": "grpc/server?xds.resource.listening_address=%s",

  "authorities": {
    "my-authority.example.com": {
      "xds_servers": [
        {
          "server_uri": "other-control-plane:18000",
          "channel_creds": [{ "type": "insecure" }],
          "server_features": ["xds_v3"]
        }
      ],
      "client_listener_resource_name_template": "xdstp://my-authority.example.com/envoy.config.listener.v3.Listener/%s"
    }
  }
}
```

#### Bootstrap Fields Reference

| Field | Required | Description |
|-------|----------|-------------|
| `xds_servers` | Yes | Array of control plane servers. First reachable one is used. |
| `xds_servers[].server_uri` | Yes | Address of the xDS server (host:port). |
| `xds_servers[].channel_creds` | Yes | Credentials for the xDS channel. Types: `google_default`, `insecure`, `tls`. |
| `xds_servers[].server_features` | Yes | Must include `"xds_v3"` for v3 xDS transport. |
| `node` | No | Node identity sent in xDS requests. Used by control plane for config selection. |
| `node.id` | No | Unique identifier. Istio format: `sidecar~IP~POD.NS~NS.svc.cluster.local`. |
| `node.metadata` | No | Arbitrary key-value pairs. Used for config selection on the control plane. |
| `node.locality` | No | Region/zone/subzone for locality-aware routing. |
| `certificate_providers` | No | Map of named certificate provider instances. Required for mTLS. |
| `server_listener_resource_name_template` | No* | Template for server Listener resource name. `%s` replaced with `IP:port`. Required for xDS-enabled servers. |
| `authorities` | No | Map of authority name to config. Enables federated xDS (multiple control planes). |

#### Channel Credential Types

| Type | Description |
|------|-------------|
| `google_default` | Google Application Default Credentials. For GCP Traffic Director. |
| `insecure` | No TLS on the xDS channel itself. For local/development control planes. |
| `tls` | TLS to the xDS server. Requires `root_certs` in config. |

### The xds:/// Resolver Scheme

gRPC clients use the `xds:///` URI scheme to trigger xDS-based name resolution instead of DNS:

```go
// Standard DNS resolution
conn, _ := grpc.Dial("dns:///my-service:8080")

// xDS resolution -- fetches config from control plane
conn, _ := grpc.Dial("xds:///my-service:8080")
```

When a channel targets `xds:///service-name`:

1. The **xds resolver** starts an ADS stream to the control plane (from bootstrap config).
2. Subscribes to the **LDS** resource for `service-name`.
3. LDS response points to an **RDS** resource (or contains inline route config).
4. RDS response maps routes to **CDS** clusters.
5. CDS response for each cluster points to an **EDS** resource.
6. EDS response provides the actual backend endpoints.
7. The resolver configures the channel's load balancer with endpoints and policies.

```
APPLICATION CODE
       |
       | grpc.Dial("xds:///my-service:8080")
       v
+------+--------+
| xds resolver   |
| (name resolver)|------> ADS stream ------> Control Plane
+------+--------+                              |
       |                                       |
       | service config                        | LDS, RDS
       | (LB policy, endpoints)                | CDS, EDS
       v                                       |
+------+--------+                              |
| LB policy      |<------- xDS config ---------+
| (priority /    |
|  weighted /    |
|  round_robin)  |
+------+--------+
       |
       | pick endpoint
       v
  +----+-----+
  | SubChannel|---> Backend Pod
  +----------+
```

### Load Balancer Hierarchy

gRPC's xDS integration constructs a tree of load balancing policies:

```
xds_cluster_resolver (top-level)
  |
  +--- priority_lb
        |
        +--- weighted_target (per priority level)
              |
              +--- round_robin or ring_hash (leaf)
                    |
                    +--- actual subchannels to endpoints
```

- **xds_cluster_resolver**: Receives CDS/EDS data, converts to addresses with locality attributes.
- **priority_lb**: Handles failover between priority levels (e.g., local zone > remote zone).
- **weighted_target**: Distributes traffic across localities by weight.
- **round_robin / ring_hash**: Final endpoint selection within a locality.

---

## 4. Language Support

### Feature Parity Matrix

gRPC's xDS support varies by language implementation. The C-core languages (C++, Python, Ruby, PHP) share an implementation. Java and Go have independent implementations.

| Feature | C++ / Python / Ruby | Java | Go | Node.js |
|---------|-------------------|------|-----|---------|
| **xDS v3 transport** | v1.36.0+ | v1.36.0+ | v1.36.0+ | v1.4.0+ |
| **LDS** | v1.30.0+ | v1.30.0+ | v1.30.0+ | v1.4.0+ |
| **RDS** | v1.30.0+ | v1.30.0+ | v1.30.0+ | v1.4.0+ |
| **CDS** | v1.30.0+ | v1.30.0+ | v1.30.0+ | v1.4.0+ |
| **EDS** | v1.30.0+ | v1.30.0+ | v1.30.0+ | v1.4.0+ |
| **Round Robin LB** | v1.30.0+ | v1.30.0+ | v1.30.0+ | v1.4.0+ |
| **Ring Hash LB** | v1.40.0+ | v1.40.0+ | v1.40.0+ | v1.5.0+ |
| **Weighted clusters** | v1.34.0+ | v1.34.0+ | v1.34.0+ | v1.4.0+ |
| **Header-based routing** | v1.34.0+ | v1.34.0+ | v1.34.0+ | v1.4.0+ |
| **Retry** | v1.40.0+ | v1.40.0+ | v1.40.0+ | v1.5.0+ |
| **Circuit breaking** | v1.40.0+ | v1.40.0+ | v1.40.0+ | Partial |
| **Fault injection** | v1.37.0+ | v1.37.0+ | v1.37.0+ | v1.4.0+ |
| **mTLS (cert providers)** | v1.36.0+ | v1.36.0+ | v1.36.0+ | Partial |
| **RBAC authorization** | v1.42.0+ | v1.42.0+ | v1.42.0+ | Not yet |
| **Outlier detection** | v1.49.0+ | v1.49.0+ | v1.49.0+ | Not yet |
| **xDS server (serving)** | v1.36.0+ | v1.36.0+ | v1.36.0+ | Not yet |
| **Custom LB policies** | v1.52.0+ | v1.52.0+ | v1.52.0+ | Not yet |

### Maturity Tiers

**Tier 1 -- Production Ready**: Go, Java, C++
- Full xDS feature set including server-side xDS.
- Extensive testing, used in production at Google and others.
- Go and Java are independently implemented (not C-core wrappers).

**Tier 2 -- Production Capable**: Python
- Shares C-core implementation with C++, inherits same feature set.
- Performance characteristics differ from C++ (Python overhead).
- Suitable for gRPC servers that are not on the hot path.

**Tier 3 -- Experimental / Partial**: Node.js, Ruby, PHP
- Node.js has independent implementation, catching up on features.
- Ruby and PHP share C-core, but xDS testing is less thorough.
- Not recommended for production xDS workloads without careful validation.

### Language-Specific Setup

**Go**: Import the xDS package to register resolvers and balancers:
```go
import (
    _ "google.golang.org/grpc/xds" // Register xDS resolver and balancers
)

// Then dial with xds:/// scheme
conn, err := grpc.NewClient("xds:///my-service:8080",
    grpc.WithTransportCredentials(insecure.NewCredentials()),
)
```

**Java**: Add the `grpc-xds` dependency and initialize:
```java
// Dependency: io.grpc:grpc-xds
// Initialize xDS -- must be called before creating channels
XdsChannelCredentials xdsCreds = XdsChannelCredentials.create(
    InsecureChannelCredentials.create());

ManagedChannel channel = Grpc.newChannelBuilder(
    "xds:///my-service:8080", xdsCreds).build();
```

**C++ / Python**: Controlled via environment variables:
```bash
export GRPC_XDS_BOOTSTRAP=/path/to/bootstrap.json
# Or inline:
export GRPC_XDS_BOOTSTRAP_CONFIG='{"xds_servers": [...]}'
```

```python
import grpc
# xDS credentials for mTLS
creds = grpc.xds_channel_credentials(grpc.local_channel_credentials())
channel = grpc.secure_channel("xds:///my-service:8080", creds)
```

---

## 5. Traffic Management

### Client-Side Load Balancing

All load balancing decisions happen **inside the gRPC client**. No external proxy makes routing decisions.

**Supported LB Policies via xDS**:

| Policy | Description | Use Case |
|--------|-------------|----------|
| `ROUND_ROBIN` | Equal distribution across healthy endpoints | General purpose, default |
| `RING_HASH` | Consistent hashing based on request attributes | Session affinity, caching |

**Not supported**: `LEAST_REQUEST`, `RANDOM`, `MAGLEV` -- gRPC will NACK if control plane sends these.

**Locality-Aware Routing**: EDS provides locality labels (region/zone/subzone) with weights. gRPC prefers local endpoints, falling back to remote with configurable weights:

```
Priority 0 (local zone):     us-central1-a (weight: 100)
Priority 1 (same region):    us-central1-b (weight: 50), us-central1-c (weight: 50)
Priority 2 (failover region): us-east1-b (weight: 100)
```

### Retry Policies

Configured via RDS route configuration. gRPC converts Envoy retry policy format:

```
Route Configuration:
  VirtualHost:
    retry_policy:
      retry_on: "cancelled,deadline-exceeded,internal,resource-exhausted,unavailable"
      num_retries: 3
      retry_back_off:
        base_interval: "0.025s"
        max_interval: "1s"
```

**Supported retry conditions** (mapped from Envoy HTTP status codes to gRPC codes):

| Envoy `retry_on` | gRPC Status Code |
|-------------------|------------------|
| `cancelled` | CANCELLED |
| `deadline-exceeded` | DEADLINE_EXCEEDED |
| `internal` | INTERNAL |
| `resource-exhausted` | RESOURCE_EXHAUSTED |
| `unavailable` | UNAVAILABLE |

**Retry hierarchy**: Route-level retry_policy overrides VirtualHost-level, which overrides cluster-level.

**Retry budget**: gRPC implements a retry throttle per server name. Maximum active retries defaults to channel concurrency limit. This prevents retry storms.

### Circuit Breaking

Configured via CDS cluster configuration:

```
Cluster:
  circuit_breakers:
    thresholds:
      - priority: DEFAULT
        max_requests: 1024
```

gRPC supports `max_requests` from the `DEFAULT` priority threshold. This limits the number of concurrent requests to a cluster. When the threshold is reached, new requests fail immediately with UNAVAILABLE.

**Not supported**: `max_connections`, `max_pending_requests`, `max_retries` thresholds, and non-DEFAULT priority levels.

### Fault Injection

Configured via LDS HTTP filters. Two types:

**Delay fault**: Injects latency before processing:
```
http_filters:
  - name: envoy.fault
    typed_config:
      delay:
        fixed_delay: "5s"
        percentage:
          numerator: 50
          denominator: HUNDRED
```

**Abort fault**: Returns an error without calling the backend:
```
http_filters:
  - name: envoy.fault
    typed_config:
      abort:
        grpc_status: 14  # UNAVAILABLE
        percentage:
          numerator: 10
          denominator: HUNDRED
```

Fault injection can be scoped per virtual host, per route, or per weighted cluster. On the client side, it was initially gated behind the `GRPC_XDS_EXPERIMENTAL_FAULT_INJECTION` environment variable (now enabled by default in recent versions).

### Traffic Splitting / Weighted Routing

Configured via RDS with weighted clusters for canary deployments:

```
Route:
  match:
    prefix: "/"
  route:
    weighted_clusters:
      clusters:
        - name: "my-service-v1"
          weight: 90
        - name: "my-service-v2"
          weight: 10
```

### Header-Based Routing

Route matching supports custom metadata (gRPC headers):

```
Route:
  match:
    prefix: "/my.package.MyService/"
    headers:
      - name: "x-canary"
        exact_match: "true"
  route:
    cluster: "canary-cluster"
```

**Important caveat**: gRPC matches only custom metadata (application headers), not HTTP/2 pseudo-headers (`:method`, `:path`, `:authority`). This differs from Envoy, which can match all headers. The reason: routing decisions happen above the transport layer in gRPC.

### Outlier Detection

Configured via CDS. Detects unhealthy endpoints and ejects them temporarily:

```
Cluster:
  outlier_detection:
    interval: "10s"
    base_ejection_time: "30s"
    max_ejection_percent: 50
    success_rate_stdev_factor: 1900
    enforcing_success_rate: 100
    success_rate_minimum_hosts: 5
    success_rate_request_volume: 100
```

**Supported ejection algorithms**:
- Success rate based (frequency of successful calls).

**Note**: Initially gated behind `GRPC_EXPERIMENTAL_ENABLE_OUTLIER_DETECTION` environment variable.

---

## 6. Security

### mTLS via Certificate Providers

gRPC does **not** use Envoy's SDS (Secret Discovery Service). Instead, it uses a **certificate provider plugin framework** defined in gRPC proposal A29.

The control plane signals TLS configuration through:
- **DownstreamTlsContext** (in LDS) -- server-side TLS config.
- **UpstreamTlsContext** (in CDS) -- client-side TLS config.

These reference `certificate_provider_instance` entries from the bootstrap config, not SDS resources.

#### The file_watcher Provider

The only built-in certificate provider implementation. Reads certificates from local filesystem and watches for changes:

```json
{
  "certificate_providers": {
    "default": {
      "plugin_name": "file_watcher",
      "config": {
        "certificate_file": "/certs/cert-chain.pem",
        "private_key_file": "/certs/key.pem",
        "ca_certificate_file": "/certs/root-cert.pem",
        "refresh_interval": "600s"
      }
    }
  }
}
```

| Field | Description |
|-------|-------------|
| `certificate_file` | Path to the identity certificate or certificate chain (PEM). |
| `private_key_file` | Path to the private key (PEM). |
| `ca_certificate_file` | Path to the root CA certificates / trust bundle (PEM). |
| `refresh_interval` | How often to re-read files. Default: 600s. |

#### How mTLS Works End-to-End

```
1. Bootstrap config defines certificate_providers["default"]
2. Control plane sends CDS with UpstreamTlsContext referencing "default"
3. gRPC client reads certs via file_watcher plugin
4. Client initiates TLS handshake with peer using mesh certificates
5. Peer validates client cert against its trust bundle
6. Mutual authentication complete -- both sides verified
```

In Istio, the istio-agent handles certificate issuance and rotation via Citadel. It writes certs to a shared volume that the gRPC app reads through file_watcher.

#### system_root_certs (New in 2025)

A new boolean field in xDS allows the control plane to instruct gRPC to use the operating system's default root trust store instead of a private mesh CA. This enables proxyless gRPC to connect to public endpoints (like Cloud Run) without mesh CA configuration.

### Authorization via RBAC

gRPC implements the xDS RBAC (Role-Based Access Control) HTTP filter for server-side authorization. Defined in gRPC proposal A41.

**Scope**: Service-to-service authorization only. Resource-level authorization is out of scope.

#### How RBAC Works

The RBAC filter is delivered via LDS as an HTTP filter on the server side:

```
Listener:
  filter_chains:
    - filters:
        - name: "envoy.http_connection_manager"
          typed_config:
            http_filters:
              - name: "envoy.filters.http.rbac"
                typed_config:
                  rules:
                    action: ALLOW
                    policies:
                      "allow-frontend":
                        permissions:
                          - url_path:
                              path:
                                prefix: "/my.package.MyService/"
                        principals:
                          - authenticated:
                              principal_name:
                                exact: "spiffe://cluster.local/ns/default/sa/frontend"
```

**Supported match fields**:
- Source principal (SPIFFE ID from mTLS).
- Destination port.
- URL path (gRPC method).
- Request headers (custom metadata).

**Evaluation logic**: `ChainEngine` processes a chain of RBAC engines. Each engine has an action (ALLOW or DENY). Deny rules are evaluated first. If no rule matches, the default is to deny.

---

## 7. When to Use Proxyless vs Sidecar

### Decision Matrix

| Factor | Proxyless | Sidecar Proxy |
|--------|-----------|---------------|
| **Latency sensitivity** | Superior -- no extra hops | Adds ~1-2ms per hop |
| **Resource overhead** | ~25 MiB agent only | ~50-100 MiB Envoy per pod |
| **Protocol support** | gRPC only | Any L4/L7 protocol |
| **Filter ecosystem** | Limited (fault, RBAC, retry) | Full Envoy filter chain |
| **L7 observability** | Application must emit | Proxy captures automatically |
| **Language support** | Go, Java, C++ (mature) | Language-agnostic |
| **Operational complexity** | Coupled to app runtime | Decoupled sidecar lifecycle |
| **Feature parity with Envoy** | Subset | Full |
| **mTLS mode** | STRICT only | STRICT + PERMISSIVE |
| **HTTP/REST traffic** | Not applicable | Full support |
| **Polyglot mesh** | Requires xDS support per lang | Works with any language |

### When Proxyless Wins

1. **Pure gRPC microservices** -- All services speak gRPC, no REST/HTTP mixed traffic.
2. **Latency-critical paths** -- Sub-millisecond matters. Eliminating proxy hops delivers ~10x improvement in request duration overhead (per benchmarks).
3. **Resource-constrained environments** -- Hundreds of pods where Envoy memory adds up. Proxyless saves ~25-75 MiB per pod.
4. **High-throughput services** -- Eliminating proxy CPU overhead for serialization/deserialization.
5. **Simple traffic policies** -- Round robin, retries, mTLS are sufficient. No need for advanced Envoy filters.

### When Sidecar Wins

1. **Mixed protocol environment** -- HTTP, gRPC, TCP services coexist.
2. **Advanced L7 features needed** -- Rate limiting, external authorization, custom WASM filters, header manipulation.
3. **L7 observability required** -- Automatic distributed tracing, access logging, metrics without app changes.
4. **Polyglot environment** -- Services in languages without mature xDS support (Ruby, PHP, .NET).
5. **PERMISSIVE mTLS needed** -- Gradual mesh adoption where some clients don't have mTLS.
6. **Decoupled security updates** -- Proxy can be patched independently of application code.

### Hybrid Approach

You can mix proxyless and sidecar within the same mesh. This is the recommended approach for most organizations:

```
+------------------+          +------------------+
| gRPC Service A   |          | HTTP Service B   |
| (proxyless)      |<-------->| (Envoy sidecar)  |
| - latency-critical           | - REST API       |
| - Go / Java      |          | - Node.js        |
+------------------+          +------------------+
         \                          /
          \                        /
           +------+------+--------+
                  |  Istiod  |
                  | (shared  |
                  |  control |
                  |  plane)  |
                  +----------+
```

Both deployment models share the same control plane and mesh identity. A proxyless gRPC client can communicate with an Envoy-proxied gRPC server and vice versa.

---

## 8. Istio + Proxyless gRPC

### Configuration

#### Step 1: Enable Sidecar Injection (for the agent, not Envoy)

Label the namespace for injection:
```bash
kubectl label namespace default istio-injection=enabled
```

#### Step 2: Annotate the Deployment

Use the `grpc-agent` injection template instead of the default sidecar template:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-grpc-service
  namespace: default
spec:
  replicas: 3
  selector:
    matchLabels:
      app: my-grpc-service
  template:
    metadata:
      labels:
        app: my-grpc-service
      annotations:
        inject.istio.io/templates: grpc-agent
        proxy.istio.io/config: '{"holdApplicationUntilProxyStarts": true}'
    spec:
      containers:
        - name: app
          image: my-grpc-service:latest
          ports:
            - containerPort: 50051
              name: grpc
          env:
            - name: GRPC_XDS_BOOTSTRAP
              value: "/var/lib/istio/data/grpc-bootstrap.json"
```

**Key annotations**:
- `inject.istio.io/templates: grpc-agent` -- Injects pilot-agent without Envoy.
- `proxy.istio.io/config: '{"holdApplicationUntilProxyStarts": true}'` -- Ensures bootstrap file is ready before gRPC starts.

#### Step 3: Expose the Service

```yaml
apiVersion: v1
kind: Service
metadata:
  name: my-grpc-service
spec:
  selector:
    app: my-grpc-service
  ports:
    - name: grpc
      port: 50051
      targetPort: 50051
```

Port naming matters: use `grpc` or `grpc-*` prefix so Istio identifies the protocol.

### PeerAuthentication for mTLS

**STRICT mode only** -- proxyless gRPC does not support PERMISSIVE mode:

```yaml
apiVersion: security.istio.io/v1beta1
kind: PeerAuthentication
metadata:
  name: default
  namespace: default
spec:
  mtls:
    mode: STRICT
```

If the server is in STRICT mode, all clients must use mTLS. For proxyless clients, this means they must have `ISTIO_MUTUAL` configured (handled automatically when using `xds:///` resolver with xDS credentials).

**Critical constraint**: You cannot gradually roll out mTLS with proxyless. PERMISSIVE mode (accept both plaintext and mTLS) is not supported. This means you must coordinate the migration -- all clients connecting to a proxyless server must support mTLS before enabling STRICT.

### AuthorizationPolicy

```yaml
apiVersion: security.istio.io/v1beta1
kind: AuthorizationPolicy
metadata:
  name: allow-frontend
  namespace: default
spec:
  selector:
    matchLabels:
      app: my-grpc-service
  action: ALLOW
  rules:
    - from:
        - source:
            principals: ["cluster.local/ns/default/sa/frontend"]
      to:
        - operation:
            methods: ["*"]
            paths: ["/my.package.MyService/*"]
```

Istio translates this to xDS RBAC filter configuration delivered via LDS.

### VirtualService for Traffic Management

```yaml
apiVersion: networking.istio.io/v1beta1
kind: VirtualService
metadata:
  name: my-grpc-service
spec:
  hosts:
    - my-grpc-service
  http:
    - match:
        - headers:
            x-version:
              exact: "v2"
      route:
        - destination:
            host: my-grpc-service
            subset: v2
    - route:
        - destination:
            host: my-grpc-service
            subset: v1
          weight: 90
        - destination:
            host: my-grpc-service
            subset: v2
          weight: 10
```

**Supported VirtualService features with proxyless gRPC**:
- Route matching by headers (custom metadata only).
- Weighted routing / traffic splitting.
- Retries.
- Timeouts (max stream duration).

**NOT supported via VirtualService with proxyless gRPC** (as of Istio 1.24+):
- Fault injection (requires direct xDS config or future Istio support).
- Request mirroring.
- URL rewrite.
- CORS policy.

### Control Plane Options

| Control Plane | Proxyless Support | Notes |
|---------------|-------------------|-------|
| **Istio (Istiod)** | Yes (experimental since 1.11, maturing) | Open source, Kubernetes-native |
| **Google Cloud Service Mesh (Traffic Director)** | Yes (GA) | GCP only, most mature implementation |
| **Cilium** | Under development | CFP for proxyless gRPC xDS support (issue #30189) |
| **Linkerd** | No | Different architecture, no xDS support |
| **Consul Connect** | No | Uses its own protocol |

---

## 9. Limitations

### Fundamental Constraints

**1. gRPC-only protocol support**
Proxyless xDS works exclusively with gRPC. HTTP/REST, TCP, WebSocket, and other protocols require a sidecar proxy. There is no workaround.

**2. No PERMISSIVE mTLS**
You cannot simultaneously accept plaintext and mTLS connections. The server is either STRICT (mTLS required) or has no mesh security. This makes gradual migration harder.

**3. Limited L7 observability**
Without a proxy intercepting traffic, you lose automatic:
- Distributed tracing span generation.
- Access logging.
- L7 metrics (request count, latency histograms by route).

The application must emit these signals itself using gRPC interceptors/middleware and OpenTelemetry. gRPC does provide some built-in metrics (via OpenCensus/OpenTelemetry), but topological mesh attributes are missing from `grpc.client.attempt.started` and `grpc.server.call.started` metrics.

**4. Subset of Envoy filter functionality**
gRPC implements only a few xDS HTTP filters:
- Fault injection.
- RBAC.
- Router.
- (Limited) Custom filters via interceptor mapping.

Missing: rate limiting, external authorization (ext_authz), WASM filters, Lua filters, header manipulation, CORS, and the long tail of Envoy's filter ecosystem.

**5. NACK-all-or-nothing semantics**
If the control plane sends an unsupported field or value, gRPC NACKs the **entire** xDS response, not just the unsupported part. This means one misconfigured resource can break configuration for all services on that client. This is particularly dangerous in shared control planes.

### Load Balancing Gaps

| Feature | Envoy | Proxyless gRPC |
|---------|-------|----------------|
| Round Robin | Yes | Yes |
| Ring Hash | Yes | Yes |
| Least Request | Yes | No (NACK) |
| Random | Yes | No (NACK) |
| Maglev | Yes | No (NACK) |
| Weighted Round Robin | Yes | Partial (via weighted clusters) |

### Traffic Management Gaps

| Feature | Envoy | Proxyless gRPC |
|---------|-------|----------------|
| Header-based routing | All headers | Custom metadata only |
| URL rewrite | Yes | No |
| Request mirroring | Yes | No |
| Rate limiting | Yes (local + global) | No |
| External authorization | Yes (ext_authz) | No |
| CORS | Yes | No |
| Compression | Yes | No (via xDS) |

### Operational Gaps

| Feature | Sidecar Model | Proxyless Model |
|---------|---------------|-----------------|
| Hot restart / upgrade | Proxy restarts independently | App restart required |
| Debug / admin endpoint | Envoy admin interface | No equivalent |
| Traffic capture / replay | Via proxy tap filter | Not available |
| Canary proxy version | Yes | N/A |
| Protocol detection | Automatic | Must be declared |

### Known Vulnerabilities (from Production Experience)

1. **agent_id spoofing**: Without proxy mediation, the client self-reports its identity. Certificate-based identity (SPIFFE) mitigates this, but is only available with mTLS.

2. **Path traversal in resource names**: xDS resource names containing `../` could potentially reference unintended resources. Control plane must canonicalize paths.

3. **TOCTOU (Time-of-Check-Time-of-Use)**: Configuration fetched from the control plane could change between check and use. ADS ordering helps but does not fully eliminate this.

---

## 10. Decision Framework

### Quick Decision Tree

```
Is the service gRPC?
  |
  +-- No --> Use sidecar proxy. Proxyless is gRPC-only.
  |
  +-- Yes
       |
       Is latency / resource overhead the primary concern?
         |
         +-- Yes --> Strong candidate for proxyless.
         |
         +-- No
              |
              Do you need advanced Envoy filters (rate limiting, ext_authz, WASM)?
                |
                +-- Yes --> Use sidecar proxy.
                |
                +-- No
                     |
                     Do you need PERMISSIVE mTLS for gradual rollout?
                       |
                       +-- Yes --> Use sidecar proxy.
                       |
                       +-- No
                            |
                            Is the language Go, Java, or C++?
                              |
                              +-- No --> Use sidecar proxy. Tier 2/3 xDS support.
                              |
                              +-- Yes --> Use proxyless.
```

### Quantitative Comparison

| Metric | Sidecar (Envoy) | Proxyless gRPC | Delta |
|--------|-----------------|----------------|-------|
| Added latency per hop | ~1-2ms | ~0.1ms | ~10x improvement |
| Memory per pod | 50-100 MiB | 25 MiB (agent) | ~2-4x savings |
| CPU per pod (proxy) | 0.1-0.5 vCPU | ~0.001 vCPU (agent) | ~100x savings |
| Startup time | Envoy init + iptables | Agent + bootstrap only | Faster |
| Feature coverage | 100% Envoy features | ~30-40% of Envoy features | Significant gap |
| Protocol coverage | Any L4/L7 | gRPC only | Narrow |

### Recommendation for geist.sh

For the geist-gateway (Rust, HTTP adapter) in the geist.sh architecture:

**Proxyless gRPC is not directly applicable** because:
1. The gateway is Rust, and gRPC Rust does not yet have xDS support (though the 2025 gRPC roadmap mentions native xDS for Rust as a future initiative).
2. The gateway handles HTTP (Axum), not exclusively gRPC.
3. geist-policy (the PDP) is a library crate, not a networked service.

**Where proxyless gRPC becomes relevant for geist.sh**:
- If geist agents (Python, Claude Agent SDK) communicate via gRPC, proxyless could govern agent-to-service calls with the control plane being the shell's policy layer.
- The xDS protocol itself (LDS/RDS/CDS/EDS) is a reference model for how the geist-gateway could distribute policy decisions -- the "configuration plane" pattern.
- The RBAC filter model (SPIFFE-based identity, deny-first evaluation, chain of engines) maps well to geist-policy's existing PolicyEvaluator pattern.

---

## Sources

- [gRPC xDS Features Matrix](https://grpc.github.io/grpc/core/md_doc_grpc_xds_features.html)
- [gRPC xDS Bootstrap File Format](https://grpc.github.io/grpc/core/md_doc_grpc_xds_bootstrap_format.html)
- [gRPC Proposal A27: xDS Global Load Balancing](https://github.com/grpc/proposal/blob/master/A27-xds-global-load-balancing.md)
- [gRPC Proposal A28: xDS Traffic Splitting and Routing](https://github.com/grpc/proposal/blob/master/A28-xds-traffic-splitting-and-routing.md)
- [gRPC Proposal A29: xDS TLS Security](https://github.com/grpc/proposal/blob/master/A29-xds-tls-security.md)
- [gRPC Proposal A33: Fault Injection](https://github.com/grpc/proposal/blob/master/A33-Fault-Injection.md)
- [gRPC Proposal A39: xDS HTTP Filters](https://github.com/grpc/proposal/blob/master/A39-xds-http-filters.md)
- [gRPC Proposal A41: xDS RBAC](https://github.com/grpc/proposal/blob/master/A41-xds-rbac.md)
- [gRPC Proposal A42: xDS Ring Hash LB Policy](https://github.com/grpc/proposal/blob/master/A42-xds-ring-hash-lb-policy.md)
- [gRPC Proposal A44: xDS Retry](https://github.com/grpc/proposal/blob/master/A44-xds-retry.md)
- [gRPC Proposal A50: xDS Outlier Detection](https://github.com/grpc/proposal/blob/master/A50-xds-outlier-detection.md)
- [gRPC Proposal A65: xDS mTLS Creds in Bootstrap](https://github.com/grpc/proposal/blob/master/A65-xds-mtls-creds-in-bootstrap.md)
- [Istio: gRPC Proxyless Service Mesh](https://istio.io/latest/blog/2021/proxyless-grpc/)
- [Google Cloud: Proxyless gRPC Overview](https://docs.cloud.google.com/service-mesh/docs/service-routing/proxyless-overview)
- [Google Cloud: Proxyless gRPC Limitations](https://cloud.google.com/traffic-director/docs/limitations-proxyless)
- [Google Cloud: Proxyless gRPC Security Setup](https://cloud.google.com/traffic-director/docs/security-proxyless-setup)
- [Google Cloud Blog: Traffic Director Supports Proxyless gRPC](https://cloud.google.com/blog/products/networking/traffic-director-supports-proxyless-grpc)
- [The New Stack: gRPC Delivers on the Promise of a Proxyless Service Mesh](https://thenewstack.io/grpc-delivers-on-the-promise-of-a-proxyless-service-mesh/)
- [gRPConf India 2025: How Proxyless gRPC Works in a Service Mesh](https://tldrecap.tech/posts/2025/grpconf-india/proxyless-grpc-xds-service-mesh/)
- [gRPConf India 2025: What's New in gRPC (Rust, Proxyless)](https://tldrecap.tech/posts/2025/grpconf-india/grpc-ai-rust-proxyless/)
- [Envoy xDS Protocol Documentation](https://www.envoyproxy.io/docs/envoy/latest/api-docs/xds_protocol)
- [Cilium: Enable Proxyless gRPC Connections to xDS (Issue #30189)](https://github.com/cilium/cilium/issues/30189)
- [Comparative Analysis: Sidecar, Ambient, and Proxyless Models](https://aimjournals.com/index.php/ijmcsit/article/view/330)
- [Tetrate: Which Data Plane -- Sidecar, Ambient, Cilium, or gRPC?](https://tetrate.io/blog/ambient-vs-sidecar)
- [grpc-java xDS Service Mesh Integration (DeepWiki)](https://deepwiki.com/grpc/grpc-java/8-xds-service-mesh-integration)
- [grpc-go xDS Example](https://github.com/grpc/grpc-go/blob/master/examples/features/xds/README.md)
