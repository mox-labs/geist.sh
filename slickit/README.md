# slickit

Semantic, LLM-Interpretable Component Kit.

Generic typed extension registry. Map type URLs to factories, instantiate typed instances from JSON config. Same pattern as Envoy's `TypedExtensionConfig` + `FactoryRegistry`.

Two type parameters (`T` for target instance, `E` for domain error), zero opinions about what you register.

## Usage

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

## Dependencies

`serde` + `serde_json`. Nothing else.

## License

BSL 1.1 — see [LICENSE](../LICENSE) for details. Converts to Apache-2.0 on 2030-03-03.
