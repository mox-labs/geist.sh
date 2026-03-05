# geist.sh

Governed runtime for AI agents. Composable processor pipeline that intercepts, evaluates, and controls agent tool calls before they execute.

## Crates

| Crate | Path | Purpose |
|-------|------|---------|
| **geist-edge** | `geist/edge` | Core runtime — Processor trait, compositor, registry, built-in processors |
| **geist-sh** | `geist/bin` | Binary — composition root, collects extensions, starts runtime |
| **slickit** | `slickit/` | Generic typed extension registry. `TypedConfig` → `TypedRegistry<T, E>` |

## Status

| Phase | What | State |
|-------|------|-------|
| P1 | Core — Processor trait, Sequence compositor, PhaseResult | Done |
| P1.5 | Extension registry + access control processor | Done |
| P2 | axum HTTP adapter | Next |
| P3 | Composer (intent → capability selection) | |
| P4 | CLI demo | |
| P5 | Desktop shell (Tauri + SvelteKit) | |

## License

[BSL 1.1](LICENSE) — Licensor: Mox Labs. Converts to Apache-2.0 on 2030-03-03.
