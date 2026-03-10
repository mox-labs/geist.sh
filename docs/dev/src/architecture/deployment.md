# Deployment Models

> **Status: Experimental / Alpha.** All APIs are volatile.

geist-edge supports three deployment models. The same processors work in all three — only the adapter changes.

## Library (axum)

geist-edge runs in-process, embedded in the host application.

```mermaid
graph TB
    subgraph host["Host application (e.g. Tauri)"]
        subgraph edge["geist-edge (axum)"]
            PP["Processor pipeline"]
        end
        SDK["Agent runtime<br/>Claude SDK"]
    end

    style host fill:#1a202c,stroke:#4a5568,color:#e2e8f0
    style edge fill:#2d3748,stroke:#718096,color:#e2e8f0
    style SDK fill:#2d3748,stroke:#718096,color:#e2e8f0
```

**When:** Desktop app (geist-shell), Claude Code hook, single-agent setups. Lowest latency — no network hop.

## Standalone Proxy (pingora)

geist-edge runs as a standalone daemon, proxying between clients and upstream services.

```mermaid
graph LR
    C[Clients] --> GE["geist-edge<br/>(pingora)"]
    GE --> AP[Agent pool]

    style GE fill:#2d3748,stroke:#e2e8f0,color:#e2e8f0
```

Properties: hot restart · connection pooling · graceful shutdown

**When:** Multi-agent gateway, production deployment, hot restart needed.

## Remote Processor (ext_proc)

geist-edge processors run as a remote ext_proc server, called by an Envoy proxy or geist-run.

```mermaid
sequenceDiagram
    participant E as Envoy / geist-run
    participant G as geist-edge (ext_proc)

    E->>G: ProcessingRequest (gRPC bidi stream)
    G->>G: Run processor pipeline
    G->>E: ProcessingResponse
```

**When:** Distributed deployment, Envoy sidecar, geist-run agent orchestration.

## Independent Deployment

geist-edge (governed proxy) and geist-run (agent orchestration) deploy independently:

| Component | Responsibility | Adapter |
|-----------|---------------|---------|
| geist-edge | Governs — intercepts, evaluates, controls | axum or pingora |
| geist-run | Orchestrates — manages agent lifecycle | ext_proc client |

They communicate via ext_proc gRPC when remote, or share a pipeline when co-located.
