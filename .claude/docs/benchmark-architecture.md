# Benchmark: geist-edge vs Spring Cloud Gateway vs Envoy

## Thesis

The fundamental design difference between inline Rust processors, JVM filter chains, and external gRPC processors produces measurable overhead differences — especially when governance features (ACL, retry) interact with body handling.

**The killer insight:** When SCG enables retry, it MUST buffer the entire request body (`CacheRequestBody` + `EnableBodyCachingEvent`) even when the ACL filter only reads headers. Envoy sends only headers to ext_proc (configurable `processing_mode`). geist-edge with `ProcessingMode::HEADERS_ONLY` never touches the body. Same governance, radically different overhead.

## Benchmark Matrix

One proxy at a time, all on `:8090` via Docker Compose profiles. No noisy neighbors.

| Profile | Service | ACL | Retry | Body buffered? | Async log |
|---------|---------|-----|-------|----------------|-----------|
| — | Upstream (`:8081`, always up) | — | — | — | — |
| `scg` | SCG | Yes | No | No | logback AsyncAppender |
| `scg-retry` | SCG + retry | Yes | **Yes** | **Yes** (the tax) | logback AsyncAppender |
| `envoy` | Envoy + ext_proc | Yes | Yes | No (headers-only mode) | Envoy flush thread |
| `geist` | geist-edge | No | No | No | tracing-appender NonBlocking |
| `geist-acl` | geist-edge + ACL | Yes | No | No (HEADERS_ONLY) | tracing-appender NonBlocking |

### Tests

| Test | What it measures | Script |
|------|-----------------|--------|
| **T1: Baseline** | Pure proxy overhead + ~3KB file access logging | `k6/baseline.js` |
| **T2: ACL + Retry** | Governance overhead + body buffering tax (20KB bodies) | `k6/acl-retry.js` |

### Key comparisons

- **8082 vs 8086** — SCG no-retry vs SCG retry = exact cost of `EnableBodyCachingEvent` (body buffering tax)
- **8082 vs 8085** — SCG vs geist-edge (both with ACL, no retry) = JVM vs Rust runtime overhead
- **8083 vs 8085** — Envoy ext_proc vs geist-edge (both with ACL) = gRPC round-trip vs inline processor
- **8086 vs 8085** — SCG retry vs geist-edge (both with ACL) = worst case vs best case

## Architecture Per Variant

### geist-edge (Rust, inline processors)

```
client → axum → translate → Sequence(request_headers) → forward → upstream
                                 ↓ inline &self call
                            AccessLogProcessor (channel send → background thread → file)
                            AccessControlProcessor (compiled rumi matchers, headers-only)
```

- Processors are `Arc<dyn Processor>` called inline via `&self`
- No serialization, no IPC, no body touching for headers-only processors
- Async log: `tracing-appender::NonBlocking` (crossbeam channel + background thread)

### Spring Cloud Gateway (JVM, Reactor filter chain)

```
client → Netty → Reactor chain → AclFilter → [RetryFilter] → forward → upstream
                                      ↓
                                 AccessLogFilter (MDC → logback AsyncAppender → file)
```

- Filters are Reactor `Mono<Void>` chains with allocations per filter
- RetryGatewayFilter publishes `EnableBodyCachingEvent` → `ModifyRequestBodyGatewayFilterFactory` reads and caches entire request body into `PooledDataBuffer`
- AclFilter only reads headers but body is already buffered by the time it runs
- Async log: logback `AsyncAppender` (`ArrayBlockingQueue` + background thread)

### Envoy + ext_proc (C++ proxy, external gRPC processor)

```
client → Envoy → ext_proc filter → gRPC stream → Kotlin ext_proc server
                                                       ↓
                                                  ACL evaluation (headers only)
                                                       ↓
                                              ProcessingResponse (allow/deny)
              Envoy ← ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
                  ↓
              forward → upstream
```

- ext_proc `processing_mode`: `request_body_mode: NONE` — body never leaves Envoy
- gRPC bidirectional stream per request (serialization + network hop even on localhost)
- Retry handled natively by Envoy (no body forwarding to ext_proc)
- Async log: Envoy file access logger (dedicated flush thread)

## Load Profile

- **Payload**: 20KB JSON bodies (request and response)
- **k6 ramp**: 0 → 100 VUs (30s) → sustained 100 VUs (3m) → ramp down (30s)
- **Traffic mix** (T2): 80% allowed, 20% denied
- **Log entry**: ~3KB structured JSON per request (all three variants)

## Metrics Collected

### HTTP (k6)
- **Latency**: avg, min, median, max, p90, p95, p99, p99.9
- **Throughput**: requests/sec (total and per-VU)
- **Error rate**: HTTP failures, check failures
- **Data transfer**: bytes sent/received

Output: `results/<variant>/<test>-k6.json` (full timeseries) + `<test>-summary.json` (aggregates)

### Container Resources (docker stats, 1s polling)
- **CPU %**: per-container utilization
- **Memory**: usage bytes, limit bytes, utilization %
- **Network I/O**: bytes received, bytes transmitted
- **PIDs**: process count (thread pressure indicator)

Output: `results/<variant>/<test>-stats.csv`

### Resource Limits (Docker)
All proxies: 512MB memory, 2 CPUs. ext_proc: 256MB, 1 CPU. Same limits ensure fair comparison.

## Running

### Automated (recommended)

```bash
cd bench

# Run all variants
./run.sh

# Run specific variants
./run.sh geist scg

# Available: geist, geist-acl, scg, scg-retry, envoy
```

`run.sh` handles: build → start upstream → run each variant in isolation (start proxy → warm up → k6 + stats collection → cool down → tear down) → summary.

### Manual

**One proxy at a time.** All proxies bind to `:8090` via Docker Compose profiles.
Upstream is always running. Swap the profile, run k6, tear down, repeat.

## Expected Results (Hypothesis)

| Metric | SCG | SCG + retry | Envoy + ext_proc | geist-edge |
|--------|-----|-------------|-------------------|------------|
| T1 p99 latency | Highest (JVM, GC) | — | Medium (C++, but proxy overhead) | Lowest (Rust, zero-copy) |
| T2 p99 latency | Medium | Much higher (body buffering) | Medium (gRPC round-trip) | Lowest (inline Rust, headers-only) |
| Memory (steady) | ~150-256MB (JVM heap) | Higher (body cache pool) | ~30MB + ext_proc JVM | ~5-10MB (Rust, no GC) |
| T2 memory delta | Small | Large (20KB × concurrent requests) | Small (body stays in Envoy) | None (headers-only) |

## Directory Layout

```
bench/
├── run.sh                      ← orchestration: build → run all variants → collect
├── collect-stats.sh            ← polls docker stats at 1s intervals → CSV
├── docker-compose.yml          ← profiles: one proxy at a time on :8090
├── upstream/
│   ├── Dockerfile              ← envoyproxy/envoy image
│   ├── envoy.yaml              ← direct_response, 20KB JSON body
│   └── response.json           ← 20KB payload
├── scg/
│   ├── Dockerfile              ← gradle build → JRE alpine
│   ├── build.gradle.kts        ← Spring Cloud Gateway + Kotlin
│   ├── settings.gradle.kts
│   └── src/main/
│       ├── kotlin/mox/geist/bench/scg/
│       │   ├── Application.kt
│       │   ├── AclFilter.kt       ← deny-first ACL (same logic as geist-acl)
│       │   └── AccessLogFilter.kt  ← MDC + structured log
│       └── resources/
│           ├── application.yml          ← default: no retry
│           ├── application-retry.yml    ← retry profile (triggers body buffering)
│           └── logback-spring.xml       ← AsyncAppender → file
├── envoy/
│   ├── Dockerfile              ← envoyproxy/envoy image
│   ├── envoy.yaml              ← ext_proc filter, retry, ~3KB access log
│   └── ext-proc/
│       ├── Dockerfile          ← gradle build → JRE alpine
│       ├── build.gradle.kts    ← grpc-kotlin + coroutines
│       ├── settings.gradle.kts
│       └── src/main/
│           ├── kotlin/mox/geist/bench/extproc/
│           │   └── ExtProcServer.kt    ← ACL via gRPC (headers-only)
│           └── proto/envoy/            ← minimal wire-compatible protos
├── geist/
│   ├── Dockerfile              ← multi-stage rust build → debian slim
│   ├── config.json             ← T1: access log only
│   └── config-acl.json         ← T2: ACL + access log
├── k6/
│   ├── common.js               ← 20KB payload, header presets
│   ├── baseline.js             ← T1: proxy overhead
│   └── acl-retry.js            ← T2: ACL + retry + body buffering tax
├── results/
│   └── <variant>/              ← per-variant: k6 JSON + summary + stats CSV + log
└── logs/                       ← mounted volumes for access logs
```

## Running

**One proxy at a time.** All proxies bind to `:8090` via Docker Compose profiles.
Upstream is always running. Swap the profile, run k6, tear down, repeat.

```bash
cd bench

# Build everything once
docker compose build

# Start upstream (stays up for all runs)
docker compose up -d upstream

# ─── Run each variant in isolation ──────────────────────────────────────

# SCG (no retry)
docker compose --profile scg up -d
k6 run --out json=results/t1-scg.json -e TARGET_URL=http://localhost:8090 k6/baseline.js
k6 run --out json=results/t2-scg.json -e TARGET_URL=http://localhost:8090 k6/acl-retry.js
docker compose --profile scg down

# SCG (retry — body buffering tax)
docker compose --profile scg-retry up -d
k6 run --out json=results/t1-scg-retry.json -e TARGET_URL=http://localhost:8090 k6/baseline.js
k6 run --out json=results/t2-scg-retry.json -e TARGET_URL=http://localhost:8090 k6/acl-retry.js
docker compose --profile scg-retry down

# Envoy + ext_proc
docker compose --profile envoy up -d
k6 run --out json=results/t1-envoy.json -e TARGET_URL=http://localhost:8090 k6/baseline.js
k6 run --out json=results/t2-envoy.json -e TARGET_URL=http://localhost:8090 k6/acl-retry.js
docker compose --profile envoy down

# geist-edge (baseline — access log only)
docker compose --profile geist up -d
k6 run --out json=results/t1-geist.json -e TARGET_URL=http://localhost:8090 k6/baseline.js
docker compose --profile geist down

# geist-edge (ACL + access log)
docker compose --profile geist-acl up -d
k6 run --out json=results/t2-geist-acl.json -e TARGET_URL=http://localhost:8090 k6/acl-retry.js
docker compose --profile geist-acl down

# Tear down upstream
docker compose down
```

## Crate: geist-log

Access log processor extension at `geist/log/`. Zero changes to geist-edge core. Binary needs Cargo dep + `use geist_log as _;` (linking only).

- Type URL: `mox.geist.processors.v1.AccessLog`
- Async I/O: `tracing-appender::NonBlocking` (crossbeam channel + background thread)
- Entry size: ~3KB structured JSON (padded for fair I/O comparison)
- Config: `{ "path": "/var/log/geist/access.log" }` (or `"stdout"`)
