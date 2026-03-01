---
title: "ADR-002: Typed Extension Registry"
date: "2026-02-22"
status: accepted
---

## Context

geist-edge needs a mechanism for processors to register themselves with the runtime without modifying core code. The system must support:

- Adding new processor types without changing the core crate or binary
- Configuration-driven pipeline assembly (JSON/YAML config → running pipeline)
- Type-safe construction from untyped configuration
- Zero runtime registration — the registry must be immutable after build

## Decision

**Typed extension registry with self-registration via `inventory`.** The pattern is:

1. Each processor implements the `IntoProcessor` factory trait with an associated `Config` type
2. Extension crates self-register via `inventory::submit!` — a static registration macro
3. The binary calls `collect_extensions()` once at startup, building an immutable registry
4. Pipeline config references processors by type URL (e.g., `mox.geist.edge.v1.AccessControl`)
5. The registry resolves type URL → factory → `Arc<dyn Processor>`

```rust
pub trait IntoProcessor: Send + Sync + 'static {
    type Config: DeserializeOwned;

    fn type_url(&self) -> &'static str;
    fn from_config(&self, config: Self::Config) -> Arc<dyn Processor>;
}
```

## Precedent

This pattern is proven in two production systems:

- **Envoy FactoryRegistry** — `REGISTER_FACTORY` macro, `TypedExtensionConfig` proto envelope, ECDS distribution
- **rumi RegistryBuilder** — `IntoDataInput` trait, explicit registration, immutable after build

The key insight from both: monomorphize at registration time (the factory knows its concrete `Config` type), type-erase at runtime (the registry stores `Box<dyn ...>`). This gives type safety during development and flexibility during execution.

## Consequences

**Positive:**
- Adding a processor = new crate + Cargo dependency + `use crate as _;` in binary. Zero code changes to core
- Config-driven pipeline assembly — no recompilation to change pipeline order
- Type URLs follow xDS convention (`mox.geist.{domain}.v1.{Type}`) — compatible with ECDS dynamic config distribution
- The registry is immutable after build — no synchronization needed at request time

**Negative:**
- Depends on `inventory` crate (dtolnay) for static registration — link-time magic
- Type URLs must be globally unique — requires namespace discipline
- Factory errors surface at startup (config parsing), not at compile time — a tradeoff of dynamic assembly

**Accepted tradeoff:** The link-time magic of `inventory` is well-understood (used by `tracing`, `linkme`, and other foundational crates). The alternative — manual registration in the binary — would require code changes for every new processor, violating the extensibility guarantee.
