# geist.sh

Governed runtime for AI agents. Composable processor pipeline that intercepts, evaluates, and controls agent tool calls before they execute.

Built on the [ext_proc](https://www.envoyproxy.io/docs/envoy/latest/api-v3/service/ext_proc/v3/external_processor.proto) processing model — same `ProcessingRequest`/`ProcessingResponse` contract used by Envoy's external processing filter, adapted for in-process agent governance.

## What It Does

Agent tool calls (file reads, shell commands, API calls) flow through a processor pipeline. Each processor can inspect, mutate, or deny the request before it reaches the tool. Deny-first by default.

```
Agent request
  → Processor pipeline (access control, rate limiting, audit, ...)
    → Tool execution (or denial)
      → Response pipeline
        → Agent
```

Processors are stateless (`&self`), composable, and registered via type URL. Add a processor = add a crate dependency + a config entry. No changes to core.

## Architecture

```
geist-edge (core)
├── Processor trait         — &self, phase-native (headers/body), Send + Sync
├── Sequence compositor     — linear pipeline, fail-open/closed, cross-phase termination
├── Typed extension registry — type URL → factory, inventory-based self-registration
└── PhaseResult             — Continue | Mutate(headers) | Respond(immediate)
```

### Processing Model

Four phases per request lifecycle: request headers, request body, response headers, response body. Processors declare which phases they participate in via `ProcessingMode`. Headers are always processed; body is opt-in.

`PhaseResult::Respond` short-circuits — no subsequent processors run, no upstream forwarding.

## Crates

| Crate | Path | Purpose |
|-------|------|---------|
| **geist-edge** | `geist/edge` | Core runtime — Processor trait, compositor, registry, built-in processors |
| **geist-sh** | `geist/bin` | Binary — composition root, collects extensions, starts runtime |
| **slickit** | `slickit/` | Generic typed extension registry. `TypedConfig` → `TypedRegistry<T, E>`. Used by geist-edge, usable standalone. |

## Writing a Processor Extension

1. Define your config type (derives `Deserialize`):

```rust
#[derive(serde::Deserialize)]
struct MyPolicy {
    blocked_tools: Vec<String>,
}
```

2. Implement `Processor` — override the phases you care about:

```rust
impl Processor for MyProcessor {
    fn name(&self) -> &str { "my-processor" }

    fn process_request_headers(
        &self,
        msg: &HttpMessage,
    ) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>> {
        let tool = msg.header("x-geist-tool-name").unwrap_or("unknown");
        Box::pin(async move {
            if self.policy.blocked_tools.contains(&tool.to_string()) {
                Ok(PhaseResult::Respond(ImmediateResponse {
                    status: Some(HttpStatus { code: 403 }),
                    body: b"blocked".to_vec(),
                    ..Default::default()
                }))
            } else {
                Ok(PhaseResult::Continue)
            }
        })
    }
}
```

3. Implement `IntoProcessor` — bridge config → instance:

```rust
impl IntoProcessor for MyProcessor {
    type Config = MyPolicy;

    fn from_config(config: Self::Config) -> Result<Arc<dyn Processor>, ProcessorError> {
        Ok(Arc::new(MyProcessor { policy: config }))
    }
}
```

4. Self-register — one line, picked up automatically:

```rust
geist_edge::register_processor!("com.example.processors.v1.MyProcessor", MyProcessor);
```

5. Configure in pipeline JSON:

```json
[
  {
    "type_url": "com.example.processors.v1.MyProcessor",
    "config": { "blocked_tools": ["Bash", "Write"] }
  }
]
```

The binary's composition root calls `ProcessorRegistryBuilder::new().collect_extensions().build()` — it discovers your processor via `inventory` without any code changes.

## Built-in Processors

**Access Control** (`mox.geist.processors.v1.AccessControl`)

Deny-first access control on agent operations. Evaluates deny rules first, then allow rules, then default deny. Policy compiles to matchers at construction time.

```json
{
  "deny": [{ "reason": "No shell access", "matches": [{ "tool_name": { "Exact": "Bash" } }] }],
  "allow": [{ "matches": [{ "agent_id": { "Exact": "claude-main" } }] }]
}
```

## Status

| Phase | What | State |
|-------|------|-------|
| P1 | Core — Processor trait, Sequence compositor, PhaseResult | Done |
| P1.5 | Extension registry + access control processor | Done |
| P2 | axum HTTP adapter | Next |
| P3 | Composer (intent → capability selection) | |
| P4 | CLI demo | |
| P5 | Desktop shell (Tauri + SvelteKit) | |

## Development

```bash
# Run tests (core crates)
cargo test -p geist-edge -p slickit -p geist-sh

# Run all default members
cargo test
```

Requires path dependencies on [`rumi`](https://github.com/mox-labs/x.uma) (matcher engine) at `../x.uma/rumi/`.

## License

[BSL 1.1](LICENSE) — Licensor: Mox Labs. Converts to Apache-2.0 on 2030-03-03.
