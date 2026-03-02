# slickit

Semantic, LLM-Interpretable Component Kit.

Generic typed extension registry + component manifest. Two type parameters (`T` for target instance, `E` for domain error), zero opinions about what you register.

## What's in the box

| Type | Role |
|------|------|
| `TypedConfig` | Config envelope: `{ type_url, config }`. Same shape as Envoy's `TypedExtensionConfig`. |
| `TypedRegistryBuilder<T, E>` | Mutable builder. `register()`, `register_unique()`, `build()`. |
| `TypedRegistry<T, E>` | Frozen registry. `create()`, `create_pipeline()`. Thread-safe, shareable via `Arc`. |
| `RegistryError<E>` | `UnknownTypeUrl` or `Factory` error, with diagnostics. |

Behind the `manifest` feature flag:

| Type | Role |
|------|------|
| `ComponentKind` | 4 kinds: Agent, Capability, Skill, Flow. |
| `ComponentManifest` | Authoring-layer metadata: kind, type_url, description, contract, envelope. |
| `ComponentContract` | What a component consumes and produces (for DAG composition). |
| `BehavioralEnvelope` | Declared resource bounds and degradation modes. |

## Usage

```rust
use slickit::{TypedConfig, TypedRegistryBuilder};

// 1. Build a registry — register factories by type URL
let registry = TypedRegistryBuilder::<String, String>::new()
    .register("example.echo.v1", |value| {
        serde_json::from_value::<String>(value.clone())
            .map_err(|e| e.to_string())
    })
    .build();

// 2. Create instances from config
let instance = registry
    .create("example.echo.v1", &serde_json::json!("hello"))
    .unwrap();
assert_eq!(instance, "hello");

// 3. Or create a whole pipeline from a config list
let pipeline = registry.create_pipeline(&[
    TypedConfig {
        type_url: "example.echo.v1".into(),
        config: serde_json::json!("first"),
    },
]).unwrap();
```

### Domain-specific wrapping

slickit is generic. Domains wrap it with their own types:

```rust
use std::sync::Arc;
use slickit::{TypedRegistryBuilder, TypedRegistry, RegistryError};

// Your domain trait
trait Processor: Send + Sync {
    fn name(&self) -> &str;
}

// Your domain error
struct ProcessorError { message: String }

// Wrap the generic registry
struct ProcessorRegistry {
    inner: TypedRegistry<Arc<dyn Processor>, ProcessorError>,
}
```

This is how [geist-edge](https://github.com/mox-labs/geist.sh) uses it — `ProcessorRegistryBuilder` wraps `TypedRegistryBuilder<Arc<dyn Processor>, ProcessorError>`, adds domain methods like `collect_extensions()` and `with::<T: IntoProcessor>()`, and flattens `RegistryError<ProcessorError>` into `ProcessorError` at the boundary.

### Manifests

```rust
use slickit::manifest::*;

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

// type_url bridges authoring → runtime
let config = slickit::TypedConfig {
    type_url: "mox.geist.processors.v1.AccessControl".into(),
    config: serde_json::json!({"deny": [], "allow": []}),
};
assert_eq!(manifest.type_url, config.type_url);
```

## Two layers

```
Authoring (manifest feature)     Runtime (default)
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

`type_url` is the join key. Manifests describe components at authoring time. The registry instantiates them at runtime.

## Cross-surface

Rust is canonical. Crusts compile the same types to other surfaces:

| Surface | Crate | Build tool |
|---------|-------|------------|
| Python | `slickit/crusts/python` | maturin |
| TypeScript | `slickit/crusts/wasm` | wasm-pack |

## Dependencies

`serde` + `serde_json`. Nothing else.

## License

MIT OR Apache-2.0
