//! slickit — Semantic, LLM-Interpretable Component Kit.
//!
//! Two layers, both in-memory:
//!
//! | Layer | Feature | Types | Consumer |
//! |-------|---------|-------|----------|
//! | Runtime (xDS) | default | [`TypedConfig`], [`TypedRegistry`], [`RegistryError`] | geist-edge |
//! | Authoring (CRD) | `manifest` | [`ComponentKind`], [`ComponentManifest`], etc. | Composer, slick plugin |
//!
//! Bridge: `ComponentManifest.type_url` = `TypedConfig.type_url`.
//!
//! Cross-surface: Rust is canonical. Crusts (PyO3, wasm-bindgen) expose
//! identical types to Python and TypeScript.
//!
//! # Example (runtime layer)
//!
//! ```
//! use slickit::{TypedConfig, TypedRegistryBuilder};
//!
//! let registry = TypedRegistryBuilder::<String, String>::new()
//!     .register("example.v1", |value| {
//!         let name: String = serde_json::from_value(value.clone())
//!             .map_err(|e| e.to_string())?;
//!         Ok(name)
//!     })
//!     .build();
//!
//! let config = serde_json::json!("hello");
//! let instance = registry.create("example.v1", &config).unwrap();
//! assert_eq!(instance, "hello");
//! ```

mod registry;

pub use registry::{RegistryError, TypedConfig, TypedRegistry, TypedRegistryBuilder};

#[cfg(feature = "manifest")]
pub mod manifest;

#[cfg(feature = "manifest")]
pub use manifest::{
    BehavioralEnvelope, ComponentContract, ComponentKind, ComponentManifest, ResourceBounds,
};
