# Kubernetes Gateway API Reference

Comprehensive architectural reference for the Kubernetes Gateway API. Covers the resource model, policy attachment patterns, extension points, conformance system, and implementation landscape as of v1.4.0 (October 2025).

---

## Table of Contents

1. [Design Philosophy](#design-philosophy)
2. [Resource Model](#resource-model)
3. [GatewayClass](#gatewayclass)
4. [Gateway](#gateway)
5. [Route Resources](#route-resources)
6. [ReferenceGrant](#referencegrant)
7. [Policy Attachment (GEP-713)](#policy-attachment-gep-713)
8. [BackendTLSPolicy](#backendtlspolicy)
9. [Extension Points](#extension-points)
10. [Implementation Landscape](#implementation-landscape)
11. [Conformance and Versioning](#conformance-and-versioning)
12. [Gateway API Inference Extension](#gateway-api-inference-extension)
13. [Key Design Decisions](#key-design-decisions)

---

## Design Philosophy

### Role-Oriented Architecture

Gateway API is designed around three organizational personas, each with distinct RBAC boundaries:

| Persona | Role | Resources Owned | Analogy |
|---------|------|-----------------|---------|
| **Infrastructure Provider** (Ian) | Manages underlying infrastructure, defines controller behavior | GatewayClass | StorageClass provider |
| **Cluster Operator** (Chihiro) | Administers the cluster, provisions Gateways, sets policy boundaries | Gateway, policies | Platform team |
| **Application Developer** (Ana) | Exposes applications via Routes within operator-defined boundaries | HTTPRoute, GRPCRoute, etc. | App team |

This separation is the core architectural insight. Unlike Ingress (where a single resource conflates infrastructure and application concerns), Gateway API distributes responsibility across organizational boundaries. A platform team provisions and constrains Gateways; application teams independently attach Routes within those constraints.

### Why Not Ingress

| Limitation | Ingress | Gateway API |
|-----------|---------|-------------|
| Protocol support | HTTP/HTTPS only | HTTP, HTTPS, gRPC, TCP, UDP, TLS |
| Extensibility | Annotations (non-portable) | Typed CRDs, policy attachment |
| Multi-tenancy | None (single resource) | Namespace isolation, RBAC per persona |
| Role separation | Flat | Infrastructure / Operator / Developer |
| TLS termination | Gateway-only | Terminate, Passthrough, backend TLS |
| Traffic splitting | Not native | Weighted backendRefs |
| Cross-namespace | Not standardized | ReferenceGrant security model |
| Conformance | Informal | Formal profiles, test suite, reports |

---

## Resource Model

The four core resource types form a directed acyclic graph (DAG):

```
GatewayClass          (cluster-scoped, defines controller)
    |
    v
Gateway               (namespace-scoped, binds listeners to addresses)
    |
    v
*Route                (namespace-scoped, maps traffic to backends)
    |
    v
Service / Backend     (namespace-scoped, the destination)
```

Policy resources attach laterally to any node in this DAG. ReferenceGrant controls cross-namespace edges.

### Binding Model

Route attachment is bidirectional:

1. The Route declares a `parentRef` pointing to a Gateway
2. The Gateway listener controls which Routes may attach via `allowedRoutes`

This two-sided handshake prevents unauthorized route attachment.

---

## GatewayClass

Cluster-scoped. Defines a class of Gateways managed by a specific controller. Analogous to `StorageClass` or `IngressClass`.

```yaml
apiVersion: gateway.networking.k8s.io/v1
kind: GatewayClass
metadata:
  name: envoy-gateway
spec:
  controllerName: gateway.envoyproxy.io/gatewayclass-controller
  parametersRef:              # optional: implementation-specific config
    group: gateway.envoyproxy.io
    kind: EnvoyProxy
    name: proxy-config
    namespace: envoy-system
```

### Status (v1.4+)

Implementations must populate `supportedFeatures` in status before or simultaneously with accepting the GatewayClass:

```yaml
status:
  conditions:
  - type: Accepted
    status: "True"
    reason: Accepted
    message: "Handled by envoy-gateway controller"
  supportedFeatures:
  - HTTPRoute
  - HTTPRouteHostRewrite
  - HTTPRoutePortRedirect
  - HTTPRouteQueryParamMatching
  - GRPCRoute
  - TLSRoute
  - BackendTLSPolicy
```

This creates a verifiable link between declared capabilities and conformance test results. The conformance test suite automatically runs tests based on declared features, eliminating the need for `--supported-features` flags.

### Key Fields

| Field | Description |
|-------|-------------|
| `controllerName` | Domain-prefixed string identifying the controller (immutable) |
| `parametersRef` | Optional reference to implementation-specific configuration |
| `description` | Human-readable description |
| `status.supportedFeatures` | (v1.4+) Features this implementation supports |

---

## Gateway

Namespace-scoped. Represents a request for traffic-handling infrastructure. Binds network addresses to listeners.

```yaml
apiVersion: gateway.networking.k8s.io/v1
kind: Gateway
metadata:
  name: production-gateway
  namespace: gateway-infra
spec:
  gatewayClassName: envoy-gateway
  addresses:
  - type: IPAddress
    value: 10.0.0.1
  infrastructure:
    labels:
      env: production
    annotations:
      service.beta.kubernetes.io/aws-load-balancer-type: nlb
    parametersRef:
      group: ""
      kind: ConfigMap
      name: gateway-options
  listeners:
  - name: http
    protocol: HTTP
    port: 80
    hostname: "*.example.com"
    allowedRoutes:
      namespaces:
        from: Selector
        selector:
          matchLabels:
            gateway-access: "true"
      kinds:
      - kind: HTTPRoute
  - name: https
    protocol: HTTPS
    port: 443
    hostname: "*.example.com"
    tls:
      mode: Terminate
      certificateRefs:
      - kind: Secret
        name: wildcard-cert
    allowedRoutes:
      namespaces:
        from: All
  - name: tcp-postgres
    protocol: TCP
    port: 5432
    allowedRoutes:
      kinds:
      - kind: TCPRoute
  - name: tls-passthrough
    protocol: TLS
    port: 8443
    tls:
      mode: Passthrough
    allowedRoutes:
      kinds:
      - kind: TLSRoute
```

### Listener Distinctiveness Rules

Listeners on the same Gateway must be distinct to ensure traffic matches only one listener:

| Protocol | Distinct On |
|----------|-------------|
| TCP/UDP | protocol + port |
| TLS | protocol + port + hostname (SNI) |
| HTTP | protocol + port + hostname |
| HTTPS | protocol + port + hostname + valid TLS config |

### Key Fields

| Field | Description |
|-------|-------------|
| `gatewayClassName` | References the GatewayClass (required) |
| `listeners[]` | Port/protocol/hostname/TLS configuration |
| `listeners[].allowedRoutes` | Controls which namespaces and Route kinds may attach |
| `addresses[]` | Requested network addresses (IP or Hostname) |
| `infrastructure` | Labels, annotations, parametersRef for generated resources |

### allowedRoutes.namespaces

| Value | Meaning |
|-------|---------|
| `from: Same` | Only Routes in the Gateway's namespace |
| `from: All` | Routes from any namespace |
| `from: Selector` | Routes from namespaces matching label selector |

---

## Route Resources

### HTTPRoute (Standard, GA since v1.0)

The primary L7 routing resource.

```yaml
apiVersion: gateway.networking.k8s.io/v1
kind: HTTPRoute
metadata:
  name: app-routes
  namespace: app-team
spec:
  parentRefs:
  - name: production-gateway
    namespace: gateway-infra
    sectionName: https       # attach to specific listener
  hostnames:
  - "app.example.com"
  rules:
  - name: api-v2             # named rule (GA in v1.4)
    matches:
    - path:
        type: PathPrefix
        value: /api/v2
      headers:
      - type: Exact
        name: x-api-version
        value: "2"
      method: GET
    filters:
    - type: RequestHeaderModifier
      requestHeaderModifier:
        add:
        - name: x-routed-by
          value: gateway-api
    backendRefs:
    - name: api-v2-svc
      port: 8080
      weight: 90
    - name: api-v2-canary
      port: 8080
      weight: 10
    timeouts:
      request: 30s
      backendRequest: 5s
  - name: legacy-redirect
    matches:
    - path:
        type: PathPrefix
        value: /api/v1
    filters:
    - type: RequestRedirect
      requestRedirect:
        scheme: https
        path:
          type: ReplaceFullPath
          replaceFullPath: /api/v2
        statusCode: 301
```

#### Match Fields

| Field | Values | Description |
|-------|--------|-------------|
| `path.type` | `Exact`, `PathPrefix`, `RegularExpression` | Path matching strategy |
| `headers[].type` | `Exact`, `RegularExpression` | Header matching strategy |
| `queryParams[].type` | `Exact`, `RegularExpression` | Query parameter matching |
| `method` | HTTP method string | Method matching |

Multiple matches within a rule use OR logic. Multiple conditions within a single match use AND logic.

#### Core Filters

| Filter | Description |
|--------|-------------|
| `RequestHeaderModifier` | Add/set/remove request headers |
| `ResponseHeaderModifier` | Add/set/remove response headers |
| `RequestRedirect` | Redirect the request |
| `URLRewrite` | Rewrite the URL |
| `RequestMirror` | Mirror requests to another backend |
| `ExtensionRef` | Implementation-specific filter |
| `externalAuth` | (v1.4, experimental) External authentication |

Constraint: `URLRewrite` and `RequestRedirect` cannot be combined on the same rule.

#### Timeouts (Standard since v1.2)

| Field | Description |
|-------|-------------|
| `request` | Total client request-response duration |
| `backendRequest` | Single gateway-to-backend duration (must be <= `request`) |

### GRPCRoute (Standard, GA since v1.1)

gRPC-specific routing with method-level matching.

```yaml
apiVersion: gateway.networking.k8s.io/v1
kind: GRPCRoute
metadata:
  name: grpc-routes
spec:
  parentRefs:
  - name: production-gateway
    sectionName: https
  hostnames:
  - "grpc.example.com"
  rules:
  - name: user-service
    matches:
    - method:
        service: com.example.UserService
        method: GetUser
    - method:
        service: com.example.UserService
        method: ListUsers
    filters:
    - type: RequestHeaderModifier
      requestHeaderModifier:
        add:
        - name: x-service-mesh
          value: "true"
    backendRefs:
    - name: user-svc
      port: 50051
      weight: 90
    - name: user-svc-canary
      port: 50051
      weight: 10
  - name: catch-all
    backendRefs:
    - name: default-grpc-svc
      port: 50051
```

Use GRPCRoute over HTTPRoute when you need gRPC-specific behavior: gRPC-aware retries on status codes, gRPC-oriented metrics, and method-level matching semantics.

### TCPRoute (Experimental)

L4 TCP routing. No content-based matching -- routes map ports to backends.

```yaml
apiVersion: gateway.networking.k8s.io/v1alpha2
kind: TCPRoute
metadata:
  name: postgres-route
spec:
  parentRefs:
  - name: production-gateway
    sectionName: tcp-postgres
  rules:
  - backendRefs:
    - name: postgres-primary
      port: 5432
```

TCPRoute supports only one rule. Each listener can have only one TCPRoute attached (no multiplexing at L4 without SNI).

### TLSRoute (Experimental)

TLS passthrough routing discriminated by SNI. The Gateway does NOT terminate TLS; the encrypted byte stream passes through to the backend.

```yaml
apiVersion: gateway.networking.k8s.io/v1alpha2
kind: TLSRoute
metadata:
  name: tls-passthrough-route
spec:
  hostnames:
  - "secure.example.com"
  parentRefs:
  - name: production-gateway
    sectionName: tls-passthrough
  rules:
  - backendRefs:
    - name: secure-backend
      port: 8443
```

TLSRoute requires the parent listener to have `tls.mode: Passthrough`. The backend is responsible for TLS termination.

### UDPRoute (Experimental)

L4 UDP routing. Same constraints as TCPRoute -- one rule, port-to-backend mapping.

```yaml
apiVersion: gateway.networking.k8s.io/v1alpha2
kind: UDPRoute
metadata:
  name: dns-route
spec:
  parentRefs:
  - name: udp-gateway
    sectionName: dns
  rules:
  - backendRefs:
    - name: coredns
      port: 53
```

### Route Type Summary

| Route | Protocol | OSI Layer | Discriminator | Channel |
|-------|----------|-----------|---------------|---------|
| HTTPRoute | HTTP/HTTPS | L7 | Path, headers, query, method | Standard (v1.0) |
| GRPCRoute | gRPC over HTTP/2 | L7 | Service, method, headers | Standard (v1.1) |
| TLSRoute | TLS | L4-7 | SNI hostname | Experimental |
| TCPRoute | TCP | L4 | None (port only) | Experimental |
| UDPRoute | UDP | L4 | None (port only) | Experimental |

---

## ReferenceGrant

Namespace-scoped security primitive. Authorizes cross-namespace references. Without a ReferenceGrant, cross-namespace references are invalid and implementations MUST reject them.

```yaml
# In namespace: backend-team
# Allows HTTPRoutes in namespace "app-team" to reference Services here
apiVersion: gateway.networking.k8s.io/v1beta1
kind: ReferenceGrant
metadata:
  name: allow-app-team-routes
  namespace: backend-team
spec:
  from:
  - group: gateway.networking.k8s.io
    kind: HTTPRoute
    namespace: app-team
  to:
  - group: ""
    kind: Service
```

```yaml
# In namespace: cert-store
# Allows Gateways in namespace "gateway-infra" to reference Secrets here
apiVersion: gateway.networking.k8s.io/v1beta1
kind: ReferenceGrant
metadata:
  name: allow-gateway-certs
  namespace: cert-store
spec:
  from:
  - group: gateway.networking.k8s.io
    kind: Gateway
    namespace: gateway-infra
  to:
  - group: ""
    kind: Secret
```

### Design Properties

- **Destination-side authorization**: The ReferenceGrant lives in the target namespace (the namespace being referenced into)
- **No resource names in `from`**: Resource names in the `from` section are intentionally excluded -- they rarely provide meaningful protection
- **Trust is explicit**: Each trust relationship requires its own ReferenceGrant
- **Revocation is immediate**: Removing a ReferenceGrant must revoke access in the same reconciliation cycle

---

## Policy Attachment (GEP-713)

GEP-713 defines the standard pattern for extending Gateway API resources with additional configuration that does not belong in the core spec. It introduces the concept of **metaresources** -- resources that augment the behavior of another resource without modifying its definition.

### Two Classes of Policy

| Property | Direct Policy | Inherited Policy |
|----------|--------------|-----------------|
| **Scope** | Affects ONLY the targeted object | Affects the target AND objects below it in the hierarchy |
| **Hierarchy** | No hierarchy traversal | Flows down the DAG (GatewayClass > Gateway > Route > Service) |
| **Merge strategy** | None (oldest wins on conflict) | Defaults (bottom-up) and Overrides (top-down) |
| **CRD label** | `gateway.networking.k8s.io/policy: direct` | `gateway.networking.k8s.io/policy: inherited` |
| **Example** | BackendTLSPolicy | TimeoutPolicy, RetryPolicy |

### PolicyTargetReference Structs

Three variants exist depending on the scope of the reference:

#### LocalPolicyTargetReference (same namespace)

```go
type LocalPolicyTargetReference struct {
    Group Group   `json:"group"`
    Kind  Kind    `json:"kind"`
    Name  string  `json:"name"`
}
```

#### NamespacedPolicyTargetReference (cross-namespace, requires ReferenceGrant)

```go
type NamespacedPolicyTargetReference struct {
    Group     Group   `json:"group"`
    Kind      Kind    `json:"kind"`
    Name      string  `json:"name"`
    Namespace *string `json:"namespace,omitempty"`
}
```

#### LocalPolicyTargetReferenceWithSectionName (targeting subsections)

```go
type LocalPolicyTargetReferenceWithSectionName struct {
    Group       Group   `json:"group"`
    Kind        Kind    `json:"kind"`
    Name        string  `json:"name"`
    SectionName *string `json:"sectionName,omitempty"`
}
```

The `sectionName` field (enhanced in v1.4) can target:
- Gateway listener names
- Service port names
- HTTPRoute rule names (v1.4+, since named rules are now Standard)

### Single vs Multiple Targets

Policies may use either:
- `targetRef` (singular) -- one target
- `targetRefs` (list) -- up to 16 targets

CEL validation ensures only one field is set. Migration from singular to list requires careful API versioning.

### Naming Conventions

| Requirement | Rule |
|-------------|------|
| Kind suffix | MUST end with `Policy` (e.g., `TimeoutPolicy`, `RateLimitPolicy`) |
| Resource name | SHOULD use `policies` (e.g., `timeoutpolicies`) |
| CRD label | MUST include `gateway.networking.k8s.io/policy: direct` or `inherited` |

### Conflict Resolution

When multiple policies of the same type target the same object:

1. **Oldest creation timestamp wins** (the "established" policy)
2. On identical timestamps, **alphabetical by `{namespace}/{name}`** wins
3. Losing policies get `Accepted: False, Reason: Conflicted`

### Status Conditions

Every policy MUST report these conditions:

| Condition | Reasons | Description |
|-----------|---------|-------------|
| `Accepted` | `Accepted`, `Conflicted`, `Invalid`, `TargetNotFound` | Whether the policy was accepted by the controller |
| `Programmed` | `Programmed`, `PartiallyProgrammed`, `Reconciling`, `Overridden` | Whether the policy is actively enforced |

#### PolicyAncestorStatus

For policies that may be implemented by multiple controllers or target resources with multiple ancestors, the `PolicyAncestorStatus` struct enables per-ancestor condition tracking:

```go
type PolicyAncestorStatus struct {
    AncestorRef ParentReference       `json:"ancestorRef"`
    ControllerName string             `json:"controllerName"`
    Conditions []metav1.Condition     `json:"conditions"`
}
```

### Discoverability

Policies SHOULD add conditions to affected target objects following the pattern `<domain>/<PolicyKindAffected>`. This addresses the "discoverability problem" -- users need to know which policies affect their resources.

### Direct Policy Example

```yaml
apiVersion: networking.example.io/v1alpha1
kind: TLSMinimumVersionPolicy
metadata:
  name: require-tls-12
  namespace: gateway-infra
  labels:
    gateway.networking.k8s.io/policy: direct
spec:
  targetRef:
    group: gateway.networking.k8s.io
    kind: Gateway
    name: production-gateway
  minimumTLSVersion: "1.2"
status:
  conditions:
  - type: Accepted
    status: "True"
    reason: Accepted
    observedGeneration: 1
```

### Inherited Policy Example (Defaults + Overrides)

```yaml
apiVersion: networking.example.io/v1alpha1
kind: TimeoutPolicy
metadata:
  name: platform-timeouts
  namespace: gateway-infra
  labels:
    gateway.networking.k8s.io/policy: inherited
spec:
  targetRef:
    group: gateway.networking.k8s.io
    kind: Gateway
    name: production-gateway
  defaults:
    request: 30s               # app teams get 30s unless they specify otherwise
    backendRequest: 5s
  overrides:
    request: 120s              # platform team caps at 120s, no exceptions
```

### Merge Strategies

| Strategy | Direction | Behavior |
|----------|-----------|----------|
| **None** | N/A | Oldest policy wins; conflicts rejected |
| **Atomic Defaults** | Bottom-up | More-specific completely replaces less-specific |
| **Atomic Overrides** | Top-down | Less-specific completely replaces more-specific |
| **Patch Defaults** | Bottom-up | JSON Merge Patch (RFC 7386), challenger patches established |
| **Patch Overrides** | Top-down | JSON Merge Patch, established patches challenger |
| **Custom** | Implementation-defined | Implementation-specific algorithm |

Precedence rules:
- **Overrides flow top-down**: GatewayClass override > Gateway override > Route override
- **Defaults flow bottom-up**: Route default > Gateway default > GatewayClass default

---

## BackendTLSPolicy

The first policy resource in the Gateway API spec itself. GA since v1.4.0. A Direct Policy that configures TLS from the Gateway to backend pods.

```yaml
apiVersion: gateway.networking.k8s.io/v1
kind: BackendTLSPolicy
metadata:
  name: tls-to-auth-service
  namespace: backend-team
spec:
  targetRefs:
  - group: ""
    kind: Service
    name: auth-service
    sectionName: https          # target specific port
  validation:
    caCertificateRefs:
    - group: ""
      kind: ConfigMap
      name: auth-ca-cert
    hostname: auth.internal.example.com
    subjectAltNames:            # v1.2+, experimental
    - type: Hostname
      hostname: auth.internal.example.com
    - type: URI
      uri: spiffe://cluster.local/ns/backend-team/sa/auth
```

### Using System Certificates (Development)

```yaml
apiVersion: gateway.networking.k8s.io/v1
kind: BackendTLSPolicy
metadata:
  name: tls-dev-backend
spec:
  targetRefs:
  - group: ""
    kind: Service
    name: dev-service
  validation:
    wellKnownCACertificates: System
    hostname: dev.example.com
```

### Key Constraints

- BackendTLSPolicy and target Service MUST be in the same namespace (no cross-namespace)
- Must use exactly one of `caCertificateRefs` or `wellKnownCACertificates`
- When `subjectAltNames` are specified, `hostname` is used only for SNI; authentication uses SANs
- Up to 8 CA certificate bundles allowed
- IP addresses and wildcards are NOT permitted in hostname

### Validation Fields

| Field | Description |
|-------|-------------|
| `hostname` | SNI hostname for backend connection (FQDN required) |
| `caCertificateRefs` | PEM-encoded CA certificate references (ConfigMap or Secret) |
| `wellKnownCACertificates` | Use system/default certificates (`System`) |
| `subjectAltNames` | (experimental) 1-5 SANs for certificate verification |
| `options` | (experimental) Implementation-specific TLS settings |

---

## Extension Points

Gateway API provides five categories of extension points:

### 1. Policy Attachment (GEP-713)

The primary extension mechanism. Custom CRDs targeting Gateway API resources via `targetRef`. See [Policy Attachment](#policy-attachment-gep-713) above.

### 2. Custom BackendRefs

Routes can forward traffic to non-Service backends:

```yaml
rules:
- backendRefs:
  - group: gateway.example.io
    kind: S3Bucket
    name: static-assets
```

Enables routing to cloud services, serverless functions, storage backends, or any custom resource.

### 3. HTTPRoute ExtensionRef Filters

Implementation-specific filters via `ExtensionRef`:

```yaml
rules:
- filters:
  - type: ExtensionRef
    extensionRef:
      group: gateway.envoyproxy.io
      kind: RateLimitFilter
      name: api-rate-limit
```

### 4. GatewayClass parametersRef

Implementation-specific GatewayClass configuration:

```yaml
spec:
  controllerName: gateway.envoyproxy.io/gatewayclass-controller
  parametersRef:
    group: gateway.envoyproxy.io
    kind: EnvoyProxy
    name: custom-proxy-config
```

### 5. Gateway infrastructure.parametersRef

Implementation-specific Gateway provisioning configuration:

```yaml
spec:
  infrastructure:
    parametersRef:
      group: ""
      kind: ConfigMap
      name: gateway-provisioning-options
```

---

## Implementation Landscape

### Envoy Gateway

Envoy Gateway extends Gateway API with five policy CRDs, all following the GEP-713 pattern:

#### ClientTrafficPolicy (Direct)

Controls downstream client-to-proxy behavior. Targets Gateway resources.

```yaml
apiVersion: gateway.envoyproxy.io/v1alpha1
kind: ClientTrafficPolicy
metadata:
  name: client-settings
spec:
  targetRefs:
  - group: gateway.networking.k8s.io
    kind: Gateway
    name: production-gateway
  timeout:
    http:
      requestReceivedTimeout: 2s
  http1:
    preserveHeaderCase: true
  tcpKeepalive:
    probes: 3
    idleTime: 20m
    interval: 60s
```

#### BackendTrafficPolicy (Direct)

Controls proxy-to-backend behavior. Targets Gateway or HTTPRoute.

```yaml
apiVersion: gateway.envoyproxy.io/v1alpha1
kind: BackendTrafficPolicy
metadata:
  name: backend-resilience
spec:
  targetRefs:
  - group: gateway.networking.k8s.io
    kind: HTTPRoute
    name: api-routes
  rateLimit:
    type: Global
    global:
      rules:
      - limit:
          requests: 100
          unit: Second
  circuitBreaker:
    maxConnections: 1024
    maxPendingRequests: 128
    maxRetries: 3
  retry:
    numRetries: 2
    retryOn:
    - gateway-error
    - reset
    perRetry:
      timeout: 500ms
  loadBalancer:
    type: LeastRequest
  timeout:
    tcp:
      connectTimeout: 5s
```

#### SecurityPolicy (Direct)

Access control targeting Gateway or HTTPRoute. Supports CORS, JWT, OIDC, basic auth, IP access lists, external auth.

```yaml
apiVersion: gateway.envoyproxy.io/v1alpha1
kind: SecurityPolicy
metadata:
  name: jwt-auth
spec:
  targetRefs:
  - group: gateway.networking.k8s.io
    kind: HTTPRoute
    name: api-routes
  jwt:
    providers:
    - name: auth0
      issuer: https://auth.example.com/
      audiences:
      - api.example.com
      remoteJWKS:
        uri: https://auth.example.com/.well-known/jwks.json
      claimToHeaders:
      - claim: sub
        header: x-user-id
```

#### EnvoyExtensionPolicy

Configures Envoy-specific extensibility (ext_proc, ext_authz, Wasm filters).

#### EnvoyPatchPolicy

Direct xDS patch for advanced use cases that cannot be expressed through higher-level policies.

#### Precedence

When the same policy type targets both a Gateway and an HTTPRoute attached to that Gateway, the HTTPRoute-level policy wins (more specific takes precedence).

### Istio

Istio implements Gateway API natively and uses it as the primary API for ambient mesh:

- **Waypoint proxies** are deployed as Gateway resources with `gatewayClassName: istio-waypoint`
- Policy attachment via `targetRefs` on `AuthorizationPolicy` and other Istio CRDs
- VirtualService is Alpha in ambient mode; Gateway API is the only GA option for waypoint configuration
- Supports the Gateway API Inference Extension for AI-aware routing

```yaml
# Istio waypoint proxy as a Gateway
apiVersion: gateway.networking.k8s.io/v1
kind: Gateway
metadata:
  name: waypoint
  namespace: app-ns
  labels:
    istio.io/waypoint-for: service
spec:
  gatewayClassName: istio-waypoint
  listeners:
  - name: mesh
    port: 15008
    protocol: HBONE
```

### kgateway (Solo.io)

kgateway uses three custom policy CRDs:

| CRD | Scope | Description |
|-----|-------|-------------|
| `TrafficPolicy` | Route-level | Rate limiting, transformations, fault injection, retries |
| `HTTPListenerPolicy` | Listener-level | Applies to all HTTP/HTTPS listeners |
| `DirectResponse` | Route-level | Static responses without backend |

Key features:
- Deep merge for `extAuth` and `extProc` policies
- `kgateway.dev/policy-weight` annotation for merge priority
- Global namespace feature for cross-namespace policy attachment via label selectors
- Can serve as a pluggable waypoint proxy for Istio ambient mesh

---

## Conformance and Versioning

### Release Channels

| Channel | Stability | Content | Release Cadence |
|---------|-----------|---------|-----------------|
| **Standard** | GA / production-ready | Graduated resources and fields only | Every ~4 months |
| **Experimental** | Alpha / breaking changes possible | Standard + alpha resources and new fields | Monthly (`monthly-YYYY-MM`) |

### Graduation Criteria (Experimental to Standard)

All of these must be met:
1. Full conformance test coverage
2. Multiple conformant implementations
3. Widespread implementation and usage
4. At least 6 months as alpha API
5. No significant changes for 1 minor release AND 3 months
6. Approval from subproject owners + KEP reviewers

### API Version Mapping

Gateway API uses two stability levels (not three like core Kubernetes):

| API Version | Channel | Stability |
|-------------|---------|-----------|
| `v1` | Standard | GA |
| `v1beta1` | Standard | Beta (legacy, being phased out) |
| `v1alpha2` | Experimental | Alpha |

Resources graduate directly from `v1alpha2` to `v1` (skipping beta) when moving to Standard.

### Conformance Profiles

| Profile | Resources Tested | Description |
|---------|-----------------|-------------|
| Gateway | GatewayClass, Gateway, HTTPRoute, ReferenceGrant | Ingress/north-south traffic |
| Mesh | Service, HTTPRoute | Service mesh/east-west traffic |

### Support Levels Within Profiles

| Level | Description |
|-------|-------------|
| **Core** | Must be supported by all conformant implementations |
| **Extended** | Portable features, not universally required. Implementations supporting them must behave consistently |
| **Implementation-Specific** | Vendor-specific, not tested, not standardized |

### Current Resource Channel Status (v1.4)

| Resource | Channel | API Version |
|----------|---------|-------------|
| GatewayClass | Standard | v1 |
| Gateway | Standard | v1 |
| HTTPRoute | Standard | v1 |
| GRPCRoute | Standard | v1 |
| ReferenceGrant | Standard | v1beta1 |
| BackendTLSPolicy | Standard | v1 |
| TCPRoute | Experimental | v1alpha2 |
| TLSRoute | Experimental | v1alpha2 |
| UDPRoute | Experimental | v1alpha2 |
| Mesh | Experimental | v1alpha1 |

---

## Gateway API Inference Extension

A separate SIG project (not part of core Gateway API) that adds AI/ML-aware routing. Introduces two CRDs:

### InferencePool (Platform Admin)

Defines a pool of model-serving pods on shared compute (GPU nodes).

```yaml
apiVersion: inference.networking.x-k8s.io/v1alpha2
kind: InferencePool
metadata:
  name: llm-pool
spec:
  targetPortNumber: 8000
  selector:
    app: vllm
  extensionRef:
    name: endpoint-picker
```

### InferenceModel (AI/ML Owner)

Maps a public model name to an actual model in a pool.

```yaml
apiVersion: inference.networking.x-k8s.io/v1alpha2
kind: InferenceModel
metadata:
  name: gpt4-chat
spec:
  modelName: gpt-4-chat
  criticality: Critical
  poolRef:
    name: llm-pool
  targetModels:
  - name: llama-2-70b
    weight: 90
  - name: llama-2-70b-v2
    weight: 10
```

### Why It Matters

LLM inference is fundamentally different from HTTP serving:
- Sessions are long-running and resource-intensive
- GPU memory is finite and shared across sessions
- LoRA adapters need locality-aware routing
- Criticality-based scheduling (interactive chat vs. batch) is essential
- Standard round-robin load balancing causes GPU memory saturation

The Endpoint Selection Extension (ESE) replaces traditional load balancing with inference-aware scheduling that considers queue depth, memory usage, loaded adapters, and model-specific metrics.

Supported by: Envoy Gateway, Istio, NGINX Gateway Fabric, kgateway, GKE.

---

## Key Design Decisions

### 1. Role-Oriented, Not Resource-Oriented

The fundamental decision: Gateway API models organizational roles, not just technical resources. This is why GatewayClass, Gateway, and Route are separate resources even though Ingress combined them. The separation enables RBAC boundaries that match real team structures.

### 2. Typed Extension Over Annotations

Ingress relied on annotations for extension, producing a non-portable ecosystem where every implementation had incompatible annotations. Gateway API uses typed CRDs and the policy attachment pattern, creating a portable extension mechanism with structural validation.

### 3. Deny-by-Default Cross-Namespace

All cross-namespace references require explicit ReferenceGrant authorization from the target namespace. This is a security-first design: the owner of the referenced resource must consent.

### 4. Conformance as a Contract

The conformance test suite, profiles, and `supportedFeatures` in GatewayClass status create a verifiable contract between implementations and consumers. This is architectural -- it prevents the Ingress problem where "supporting Gateway API" had no testable meaning.

### 5. Bidirectional Binding

Route-to-Gateway attachment requires both sides to agree (Route `parentRef` + Gateway `allowedRoutes`). This prevents unauthorized route attachment, a security property that Ingress lacked.

### 6. Policy as Metaresource

Instead of adding every possible configuration to core resource specs, Gateway API uses the metaresource pattern: separate CRDs that augment behavior via `targetRef`. This keeps the core API surface minimal and extensible.

### 7. Standard + Experimental Channels

The two-channel system lets the project iterate quickly (monthly experimental releases) while providing strong stability guarantees (4-month standard releases with graduation criteria). This solves the tension between innovation speed and production reliability.

### 8. Listener-Level Isolation

Each listener is a separate attachment point with its own namespace/kind restrictions. This enables multi-tenant Gateways where different teams attach to different listeners on shared infrastructure.

---

## Version History

| Version | Date | Key Additions |
|---------|------|---------------|
| v1.0 | Oct 2023 | HTTPRoute, Gateway, GatewayClass graduate to GA |
| v1.1 | May 2024 | GRPCRoute GA, service mesh support, conformance profiles |
| v1.2 | Nov 2024 | Timeouts GA, experimental retries, SubjectAltNames |
| v1.3 | Jan 2025 | Scoping/stabilization release |
| v1.4 | Oct 2025 | BackendTLSPolicy GA, supportedFeatures, named rules GA, Mesh resource (experimental), default gateways (experimental), externalAuth filter (experimental) |

---

## Source References

- [Gateway API Official Documentation](https://gateway-api.sigs.k8s.io/)
- [GEP-713: Metaresources and Policy Attachment](https://gateway-api.sigs.k8s.io/geps/gep-713/)
- [GEP-2648: Direct Policy Attachment](https://gateway-api.sigs.k8s.io/geps/gep-2648/) (declined, merged back to GEP-713)
- [GEP-2649: Inherited Policy Attachment](https://gateway-api.sigs.k8s.io/geps/gep-2649/) (declined, merged back to GEP-713)
- [Gateway API v1.4 Release Blog](https://kubernetes.io/blog/2025/11/06/gateway-api-v1-4/)
- [Gateway API v1.2 Release Blog](https://kubernetes.io/blog/2024/11/21/gateway-api-v1-2/)
- [Gateway API v1.1 Release Blog](https://kubernetes.io/blog/2024/05/09/gateway-api-v1-1/)
- [API Overview](https://gateway-api.sigs.k8s.io/concepts/api-overview/)
- [Conformance](https://gateway-api.sigs.k8s.io/concepts/conformance/)
- [Versioning](https://gateway-api.sigs.k8s.io/concepts/versioning/)
- [BackendTLSPolicy](https://gateway-api.sigs.k8s.io/api-types/backendtlspolicy/)
- [HTTPRoute](https://gateway-api.sigs.k8s.io/api-types/httproute/)
- [GRPCRoute](https://gateway-api.sigs.k8s.io/api-types/grpcroute/)
- [TCP Routing Guide](https://gateway-api.sigs.k8s.io/guides/tcp/)
- [TLS Guide](https://gateway-api.sigs.k8s.io/guides/tls/)
- [ReferenceGrant](https://gateway-api.sigs.k8s.io/api-types/referencegrant/)
- [Envoy Gateway Extensions](https://gateway.envoyproxy.io/docs/concepts/gateway_api_extensions/)
- [kgateway Policy Attachments](https://kgateway.dev/blog/policy-attachments/)
- [Gateway API Inference Extension](https://gateway-api-inference-extension.sigs.k8s.io/)
- [Introducing Gateway API Inference Extension (Kubernetes Blog)](https://kubernetes.io/blog/2025/06/05/introducing-gateway-api-inference-extension/)
- [GitHub: kubernetes-sigs/gateway-api](https://github.com/kubernetes-sigs/gateway-api)
