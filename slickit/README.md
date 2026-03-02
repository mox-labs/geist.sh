# slickit

Semantic, LLM-Interpretable Component Kit.

Map type URLs to factories, instantiate typed instances from JSON config. Two type parameters (`T` for target instance, `E` for domain error), zero opinions about what you register.

```
cargo add slickit
```

## Runtime layer

Register factories by type URL, build a frozen registry, create instances from config.

```rust
use slick::{TypedConfig, TypedRegistryBuilder};

let registry = TypedRegistryBuilder::<String, String>::new()
    .register("example.echo.v1", |value| {
        serde_json::from_value::<String>(value.clone())
            .map_err(|e| e.to_string())
    })
    .build();

let instance = registry
    .create("example.echo.v1", &serde_json::json!("hello"))
    .unwrap();
assert_eq!(instance, "hello");
```

| Type | Role |
|------|------|
| `TypedConfig` | Config envelope: `{ type_url, config }` |
| `TypedRegistryBuilder<T, E>` | Mutable builder. `register()`, `register_unique()`, `build()` |
| `TypedRegistry<T, E>` | Frozen registry. `create()`, `create_pipeline()`. Thread-safe via `Arc` |
| `RegistryError<E>` | `UnknownTypeUrl` or `Factory` error, with diagnostics |

### Domain wrapping

slick is generic. Domains wrap it with their own types:

```rust
use std::sync::Arc;
use slick::{TypedRegistryBuilder, TypedRegistry};

trait Processor: Send + Sync {
    fn name(&self) -> &str;
}

struct ProcessorError { message: String }

// Wrap the generic registry
struct ProcessorRegistry {
    inner: TypedRegistry<Arc<dyn Processor>, ProcessorError>,
}
```

Add domain methods on the wrapper (`collect_extensions()`, `with::<T>()`), flatten `RegistryError<ProcessorError>` into your domain error at the boundary.

## Authoring layer

Behind the `manifest` feature flag. Describes components at authoring time — what they are, what they consume/produce, declared resource bounds.

```
cargo add slickit --features manifest
```

```rust
use slick::manifest::*;

let manifest = ComponentManifest {
    kind: ComponentKind::Capability,
    type_url: "mox.geist.processors.v1.AccessControl".into(),
    description: "Deny-first access control".into(),
    contract: ComponentContract {
        consumes: vec![],
        produces: Some("mox.geist.v1.AuthResult".into()),
        assertions: vec!["Denies by default when no rules match".into()],
        boundaries: vec!["Does not handle authentication".into()],
    },
    envelope: BehavioralEnvelope::default(),
};
```

| Type | Role |
|------|------|
| `ComponentKind` | Agent, Capability, Skill, Flow |
| `ComponentManifest` | Kind, type URL, description, contract, envelope |
| `ComponentContract` | What a component consumes and produces (DAG composition) |
| `BehavioralEnvelope` | Declared resource bounds and degradation modes |

## Bridge

`type_url` joins the two layers. Manifests describe, the registry instantiates.

```
Authoring (manifest)             Runtime (default)
─────────────────────────────    ──────────────────────────
ComponentManifest                TypedConfig
  kind: ComponentKind              type_url ──────┐
  type_url ───────────────────── = type_url       │
  description                    config (JSON)    │
  contract (consumes/produces)                    │
  envelope (bounds/degradation)  TypedRegistry    │
                                   create() ◄────┘
                                   → T instance
```

## Cross-surface

Rust is canonical. Crusts (compiled bindings) expose the same types to Python and TypeScript:

| Surface | Import | Package | Build |
|---------|--------|---------|-------|
| Rust | `use slick::` | `slickit` | cargo |
| Python | `import slickpy` | `slickpy` | maturin |
| TypeScript | `import { ... } from 'slick'` | `slick` | wasm-pack |

## Dependencies

`serde` + `serde_json`. Nothing else.

## License

MIT OR Apache-2.0
