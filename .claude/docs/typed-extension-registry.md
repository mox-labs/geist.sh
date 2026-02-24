# Typed Extension Registry — Pattern Reference

## The Invariant

Adding a processor extension to geist-edge requires **zero code changes** to core or binary. An extension crate:

1. Implements `IntoProcessor` (factory trait)
2. Calls `inventory::submit!` (self-registration)
3. Adds a `TypedConfig` entry to pipeline config (JSON)

The binary's composition root calls `collect_extensions()` once — this never changes regardless of how many extensions exist.

**Three implementations of this pattern:**

| System | Registration | Factory Trait | Config Envelope |
|--------|-------------|---------------|-----------------|
| **Envoy** | `REGISTER_FACTORY` (static global, before `main()`) | `FactoryBase<Proto>` | `TypedExtensionConfig` proto |
| **rumi** | `RegistryBuilder::input::<T>()` (explicit) | `IntoDataInput` | `TypedConfig` |
| **geist-edge** | `inventory::submit!` (distributed static) | `IntoProcessor` | `TypedConfig` |

geist-edge's approach is closest to Envoy's: distributed static registration where each extension self-registers without touching core. The difference: Envoy uses C++ static initializers; geist-edge uses `inventory` (dtolnay's `ctor`-based distributed collection).

## Core Types

### IntoProcessor — the factory trait

```rust
/// Factory trait for processor extensions.
/// Each processor defines its Config type (policy + runtime config combined).
/// Monomorphized at registration, type-erased at runtime.
pub trait IntoProcessor: Send + Sync + 'static {
    type Config: DeserializeOwned + Send + Sync;
    fn from_config(config: Self::Config) -> Result<Arc<dyn Processor>, ProcessorError>;
}
```

### ProcessorRegistration — the inventory item

```rust
/// A self-contained processor registration.
/// Extension crates submit these via `inventory::submit!`.
/// The binary collects them via `inventory::iter::<ProcessorRegistration>`.
pub struct ProcessorRegistration {
    /// xDS type URL — the factory lookup key.
    /// Format: mox.geist.processors.v1.{TypeName}
    pub type_url: &'static str,

    /// Factory function: deserializes config JSON → Arc<dyn Processor>.
    /// This is the monomorphized closure — type-safe at registration,
    /// type-erased at runtime.
    pub factory: fn(&serde_json::Value) -> Result<Arc<dyn Processor>, ProcessorError>,
}

inventory::collect!(ProcessorRegistration);
```

### ProcessorRegistry — immutable after build

```rust
type BoxedProcessorFactory =
    Box<dyn Fn(&serde_json::Value) -> Result<Arc<dyn Processor>, ProcessorError> + Send + Sync>;

pub struct ProcessorRegistryBuilder {
    factories: HashMap<String, BoxedProcessorFactory>,
}

impl ProcessorRegistryBuilder {
    pub fn new() -> Self { ... }

    /// Collect all extensions registered via `inventory::submit!`.
    /// Call once in the composition root. Never changes.
    #[must_use]
    pub fn collect_extensions(mut self) -> Self {
        for reg in inventory::iter::<ProcessorRegistration> {
            self.factories.insert(
                reg.type_url.to_owned(),
                Box::new(reg.factory),
            );
        }
        self
    }

    /// Register a processor factory explicitly (for tests or one-offs).
    #[must_use]
    pub fn with<T: IntoProcessor>(mut self, type_url: &str) -> Self {
        self.factories.insert(
            type_url.to_owned(),
            Box::new(|value: &serde_json::Value| {
                let config: T::Config = serde_json::from_value(value.clone())?;
                T::from_config(config)
            }),
        );
        self
    }

    pub fn build(self) -> ProcessorRegistry { ... }
}

pub struct ProcessorRegistry {
    factories: HashMap<String, BoxedProcessorFactory>,
}

impl ProcessorRegistry {
    pub fn create(
        &self,
        type_url: &str,
        config: &serde_json::Value,
    ) -> Result<Arc<dyn Processor>, ProcessorError> {
        let factory = self.factories.get(type_url)
            .ok_or_else(|| ProcessorError::unknown_type_url(type_url, self.type_urls()))?;
        factory(config)
    }

    pub fn type_urls(&self) -> Vec<&str> { ... }
}
```

Two registration paths:
- **`collect_extensions()`** — production path. Gathers all `inventory::submit!` registrations. Zero code changes when extensions are added/removed.
- **`.with::<T>(type_url)`** — test/explicit path. Fluent builder for tests or when you want explicit control.

## Extension Contributor Experience

### Writing a processor extension (geist-acl example)

An extension crate depends on `geist-edge` and provides three things:

**1. The processor** — implements `Processor` trait:

```rust
// geist-acl/src/processor.rs (behind `processor` feature gate)
pub struct AccessControlProcessor {
    evaluator: PolicyEvaluator,  // compiled rumi matchers
}

impl Processor for AccessControlProcessor {
    fn processing_mode(&self) -> ProcessingMode { ProcessingMode::HEADERS_ONLY }

    fn on_request_headers<'a>(&'a self, msg: &'a HttpMessage) -> BoxFuture<'a, Result<PhaseResult, ProcessorError>> {
        Box::pin(async move {
            // extract agent context from x-geist-* headers
            // evaluate deny/allow rules via compiled matchers
            // return Continue, Mutate, or Respond(ImmediateResponse)
        })
    }
}
```

**2. The factory** — implements `IntoProcessor`:

```rust
// geist-acl/src/processor.rs
impl IntoProcessor for AccessControlProcessor {
    type Config = AccessControlPolicy;  // deny rules + allow rules

    fn from_config(config: Self::Config) -> Result<Arc<dyn Processor>, ProcessorError> {
        // Compile deny/allow rules → rumi matchers at construction time.
        // The compiled matchers live in the Arc, shared across all requests.
        Ok(Arc::new(AccessControlProcessor::new(config)?))
    }
}
```

**3. Self-registration** — `inventory::submit!`:

```rust
// geist-acl/src/processor.rs
inventory::submit! {
    ProcessorRegistration {
        type_url: "mox.geist.processors.v1.AccessControl",
        factory: |value| {
            let config: AccessControlPolicy = serde_json::from_value(value.clone())
                .map_err(|e| ProcessorError::new("access-control", e.to_string()))?;
            AccessControlProcessor::from_config(config)
        },
    }
}
```

**Cargo.toml**:

```toml
[dependencies]
geist-edge = { path = "../edge" }
inventory = "0.3"
rumi = { ... }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### Extension crate structure

```
geist-acl/
├── src/
│   ├── lib.rs          ← pub mod + inventory::submit!
│   ├── processor.rs    ← AccessControlProcessor: impl Processor + impl IntoProcessor
│   ├── config.rs       ← AccessControlPolicy (deny/allow rules)
│   └── evaluator.rs    ← Compiled rumi matchers, deny-first evaluation
└── Cargo.toml          ← depends on geist-edge, rumi, inventory
```

Same principle as Envoy: self-contained extension module. Everything in one crate. The `inventory::submit!` call is the Rust equivalent of `REGISTER_FACTORY`.

### Composition root (geist binary) — NEVER changes

```rust
// geist/bin/src/main.rs — this code is written once
let registry = ProcessorRegistryBuilder::new()
    .collect_extensions()  // gathers all inventory registrations
    .build();

// Load pipeline from config file
let processors: Vec<Arc<dyn Processor>> = config.processors.iter()
    .map(|tc| registry.create(&tc.type_url, &tc.config))
    .collect::<Result<_, _>>()?;

let sequence = SequenceBuilder::new()
    .fail_closed()
    .processors(processors)
    .build();
```

Adding `geist-auth`, `geist-ratelimit`, or any new extension:
1. Add the crate as a dependency in `geist/bin/Cargo.toml`
2. Add a `TypedConfig` entry in the pipeline config JSON
3. **Zero code changes.** The `collect_extensions()` call picks it up automatically.

## Pipeline Config (JSON)

```json
{
  "processors": [
    {
      "type_url": "mox.geist.processors.v1.AccessControl",
      "config": {
        "deny": [{ "matches": [{ "tool_name": { "Exact": "Bash" } }], "reason": "No Bash" }],
        "allow": [{ "matches": [{ "agent_id": { "Exact": "claude-main" } }] }]
      }
    },
    {
      "type_url": "mox.geist.processors.v1.RateLimit",
      "config": {
        "store": "redis://localhost:6379",
        "rules": [{ "limit": 100, "window": "1m" }]
      }
    }
  ]
}
```

Each entry is a `TypedConfig { type_url, config }` — same shape as Envoy's `TypedExtensionConfig`. The `type_url` is the factory lookup key. The `config` is opaque JSON deserialized by the factory into the processor's own `Config` type.

## How `inventory` Works

[`inventory`](https://crates.io/crates/inventory) by dtolnay uses the `ctor` crate to run initialization code when a shared library is loaded. Each `inventory::submit!` creates a static constructor that appends to a global linked list. `inventory::iter` walks that list.

**Why not `linkme`?** The `linkme` crate uses linker sections (`distributed_slice`) which silently drop entries from dependency crates due to [rust-lang/rust#67209](https://github.com/rust-lang/rust/issues/67209). `inventory`'s `ctor` mechanism doesn't have this issue — it uses platform init functions (`.init_array` on Linux, `__mod_init_func` on macOS) which are reliably called for all linked objects.

**Safety**: `inventory` runs constructors at load time (before `main()`). The constructors only append to a global linked list — no complex initialization, no ordering dependencies. The pattern is identical to Envoy's `REGISTER_FACTORY` (C++ static global constructors).

**Cross-crate**: Works correctly across crate boundaries. An `inventory::submit!` in `geist-acl` is collected by `inventory::iter` in `geist-bin`, as long as `geist-acl` is a dependency (direct or transitive).

## The Two-Level Lifecycle

```
JSON config → Arc<dyn Processor>
```

Envoy has three levels: proto → compiled config (shared) → per-request filter.
geist-edge collapses to two because `&self` on the `Processor` trait makes the processor immutable and shared via `Arc`. No per-request allocation needed.

| Envoy Level | geist-edge Equivalent | Lifetime |
|-------------|----------------------|----------|
| Proto config (from xDS) | `serde_json::Value` (from pipeline config) | Transient — consumed during construction |
| Compiled config (shared) | `Arc<dyn Processor>` | Pipeline lifetime — shared across all requests |
| Per-request filter | *(eliminated)* | `&self` means no per-request state |

The `Arc<dyn Processor>` IS the compiled config. The factory (`IntoProcessor::from_config`) compiles rules → matchers, allocates caches, builds engines — all at construction. Every subsequent request invokes `&self` methods on the shared instance.

**Trade-off**: processors can't maintain per-request mutable state across phases. If needed in the future, a per-request context map passed through the pipeline would solve this without changing the Processor trait.

## Envoy Reference (from ~/oss/envoy/)

### TypedExtensionConfig proto

```protobuf
// api/envoy/config/core/v3/extension.proto
message TypedExtensionConfig {
  string name = 1;                    // opaque identifier (cosmetic)
  google.protobuf.Any typed_config = 2; // type_url = factory lookup key
}
```

### REGISTER_FACTORY (the Envoy equivalent of inventory::submit!)

```cpp
// envoy/registry/registry.h — creates static global, registers before main()
#define REGISTER_FACTORY(FACTORY, BASE)                            \
  ABSL_ATTRIBUTE_UNUSED void forceRegister##FACTORY() {}          \
  static Envoy::Registry::RegisterFactory<FACTORY, BASE>           \
      FACTORY##_registered

// Constructor runs at static init, registers factory by name + type URL
RegisterFactory() {
  FactoryRegistry<Base>::registerFactory(instance_, instance_.name());
  FactoryCategoryRegistry::registerCategory(instance_.category(), ...);
}
```

### Factory type hierarchy

```
UntypedFactory                              ← name(), category(), configTypes()
  └─ TypedFactory                           ← createEmptyConfigProto()
       └─ HttpFilterConfigFactoryBase       ← category() = "envoy.filters.http"
            └─ NamedHttpFilterConfigFactory ← createFilterFactoryFromProto()
                 └─ FactoryBase<Proto>      ← boilerplate removal template
                      └─ RBACFactory        ← the actual extension
```

### FactoryRegistry (dual key, lazily built type URL map)

```cpp
template <class Base> class FactoryRegistry {
  static absl::flat_hash_map<std::string, Base*>& factories();        // by NAME
  static absl::flat_hash_map<std::string, Base*>& factoriesByType();  // by TYPE URL

  // Type URL map built lazily from factory.configTypes()
};
```

geist-edge only needs type URL lookup (no legacy name compatibility), so `ProcessorRegistry` uses a single `HashMap<String, Factory>`.

### ECDS flow

```
xDS server sends TypedExtensionConfig
  → extract type_url from Any
  → FactoryRegistry::getFactoryByType(type_url)
  → factory.createFilterFactoryFromProto(message)
  → closure captures compiled config
  → TLS distributes to worker threads
  → next request uses new config
```

Same model works for geist-edge: ECDS delivers new configs to existing factories. The registry is frozen; only configs change at runtime.

## rumi Reference (from x.uma/rumi/)

### Trait pattern (same shape as IntoProcessor)

```rust
pub trait IntoDataInput<Ctx: 'static>: Send + Sync + 'static {
    type Config: DeserializeOwned + Send + Sync;
    fn from_config(config: Self::Config) -> Result<Box<dyn DataInput<Ctx>>, MatcherError>;
}
```

### Type erasure via closures (same technique)

```rust
pub fn input<T: IntoDataInput<Ctx>>(mut self, type_url: &str) -> Self {
    self.input_factories.insert(
        type_url.to_owned(),
        Box::new(|value: &serde_json::Value| {
            let config: T::Config = serde_json::from_value(value.clone())?;
            T::from_config(config)
        }),
    );
    self
}
```

rumi uses explicit `register()` functions rather than `inventory` because rumi's use case is library-to-library (the caller always knows which inputs to register). geist-edge uses `inventory` because extensions should be addable without touching the binary.

## Key Differences from Service Locator Anti-Pattern

- **Distributed static registration** (not runtime service discovery)
- **Compile-time knowledge** (monomorphized types captured in closures via `inventory::submit!`)
- **Stateless factories** (fn pointers, no global mutable state)
- **Type safety** (Config types checked at compile time within the factory)
- **Immutable after build** (builder → frozen registry, no runtime registration)
- **Explicit dependency** (extension must be in Cargo.toml — no classpath scanning magic)

## Type URL Convention

Format: `mox.geist.processors.v1.{TypeName}`

| Processor | Type URL |
|-----------|----------|
| Access Control | `mox.geist.processors.v1.AccessControl` |
| Rate Limit | `mox.geist.processors.v1.RateLimit` |
| Auth (JWT) | `mox.geist.processors.v1.JwtAuth` |
| Telemetry | `mox.geist.processors.v1.Telemetry` |

Follows xDS convention: `{org}.{product}.{domain}.{version}.{Type}`. Aligns with the project's proto namespace `mox.geist.edge.v1`.

## Source Files

| File | What |
|------|------|
| `~/oss/envoy/envoy/registry/registry.h` | FactoryRegistry, RegisterFactory macro, dual-key lookup |
| `~/oss/envoy/envoy/config/typed_config.h` | UntypedFactory, TypedFactory base classes |
| `~/oss/envoy/api/envoy/config/core/v3/extension.proto` | TypedExtensionConfig proto |
| `~/oss/envoy/source/extensions/filters/http/rbac/config.h` | RBAC factory (complete extension example) |
| `~/oss/envoy/source/extensions/filters/http/common/factory_base.h` | FactoryBase template |
| `x.uma/rumi/core/src/registry.rs` | rumi RegistryBuilder |
| `x.uma/rumi/ext/http/src/lib.rs` | rumi-http register() |
| `scratch/envoy-extension-system-2026-02-24.md` | Full Envoy extension system analysis |
