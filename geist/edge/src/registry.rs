//! Typed extension registry for processor extensions.
//!
//! Extension crates self-register via [`inventory::submit!`] with a
//! [`ProcessorRegistration`]. The binary collects all registrations
//! via [`ProcessorRegistryBuilder::collect_extensions`].
//!
//! Adding a new processor extension requires zero code changes to
//! geist-edge or the binary — only a Cargo dependency and a config entry.
//!
//! # Pattern
//!
//! Same pattern as Envoy's `FactoryRegistry` + `REGISTER_FACTORY` and
//! rumi's `RegistryBuilder` + `IntoDataInput`:
//!
//! ```text
//! Extension crate:  inventory::submit!(ProcessorRegistration { ... })
//! Binary:           ProcessorRegistryBuilder::new().collect_extensions().build()
//! Config:           [{ "type_url": "mox.geist.processors.v1.Foo", "config": {...} }]
//! ```

use std::sync::Arc;

use serde::de::DeserializeOwned;
use slick::RegistryError;

use crate::processor::{Processor, ProcessorError};

// Re-export TypedConfig from slick — same type, single source of truth.
pub use slick::TypedConfig;

/// Factory trait for processor extensions.
///
/// Each processor defines its `Config` type (policy + runtime config combined).
/// The trait is monomorphized at registration time and type-erased at runtime
/// via the closure captured in [`ProcessorRegistryBuilder::with`].
///
/// # Example
///
/// ```ignore
/// impl IntoProcessor for AccessControlProcessor {
///     type Config = AccessControlPolicy;
///
///     fn from_config(config: Self::Config) -> Result<Arc<dyn Processor>, ProcessorError> {
///         Ok(Arc::new(AccessControlProcessor::new(config)?))
///     }
/// }
/// ```
pub trait IntoProcessor: Send + Sync + 'static {
    /// The configuration type this processor is built from.
    type Config: DeserializeOwned + Send + Sync;

    /// Create a processor instance from deserialized config.
    ///
    /// Called once per pipeline config entry. The returned `Arc<dyn Processor>`
    /// is shared across all requests (immutable via `&self`).
    fn from_config(config: Self::Config) -> Result<Arc<dyn Processor>, ProcessorError>;
}

/// A self-contained processor registration for distributed static collection.
///
/// Extension crates submit these via [`register_processor!`] (preferred) or
/// raw `inventory::submit!` (advanced). The binary collects them via
/// [`ProcessorRegistryBuilder::collect_extensions`].
///
/// # Example
///
/// Preferred — via macro (handles deserialization + error wrapping):
///
/// ```ignore
/// geist_edge::register_processor!(
///     "mox.geist.processors.v1.AccessControl",
///     AccessControlProcessor
/// );
/// ```
///
/// Advanced — raw registration (custom deserialization logic):
///
/// ```ignore
/// inventory::submit! {
///     ProcessorRegistration {
///         type_url: "mox.geist.processors.v1.AccessControl",
///         factory: |value| {
///             let config: AccessControlPolicy = serde_json::from_value(value.clone())
///                 .map_err(|e| ProcessorError::new(
///                     "mox.geist.processors.v1.AccessControl",
///                     format!("config deserialization failed: {e}"),
///                 ))?;
///             AccessControlProcessor::from_config(config)
///         },
///     }
/// }
/// ```
pub struct ProcessorRegistration {
    /// xDS type URL — the factory lookup key.
    pub type_url: &'static str,
    /// Factory function: deserializes config JSON and constructs the processor.
    pub factory: fn(config: &serde_json::Value) -> Result<Arc<dyn Processor>, ProcessorError>,
}

inventory::collect!(ProcessorRegistration);

/// Register a processor extension via `inventory::submit!`.
///
/// Bridges the [`IntoProcessor`] trait to distributed static registration,
/// eliminating the boilerplate of manual deserialization and error wrapping.
///
/// # Example
///
/// ```ignore
/// use geist_edge::register_processor;
///
/// register_processor!(
///     "mox.geist.processors.v1.AccessControl",
///     AccessControlProcessor
/// );
/// ```
///
/// Expands to an `inventory::submit!` with correct deserialization,
/// error wrapping (using the type URL as processor name), and
/// `IntoProcessor::from_config` call.
#[macro_export]
macro_rules! register_processor {
    ($type_url:expr, $processor_type:ty) => {
        ::inventory::submit! {
            $crate::registry::ProcessorRegistration {
                type_url: $type_url,
                factory: |value| {
                    let config: <$processor_type as $crate::registry::IntoProcessor>::Config =
                        ::serde_json::from_value(value.clone()).map_err(|e| {
                            $crate::processor::ProcessorError::new(
                                $type_url,
                                format!("config deserialization failed: {e}"),
                            )
                        })?;
                    <$processor_type as $crate::registry::IntoProcessor>::from_config(config)
                },
            }
        }
    };
}

/// Builder for [`ProcessorRegistry`]. Immutable after [`build()`](Self::build).
///
/// Two registration paths:
/// - [`collect_extensions`](Self::collect_extensions) — production. Gathers all
///   `inventory::submit!` registrations. Zero code changes when extensions change.
/// - [`with`](Self::with) — tests/explicit. Fluent builder for direct registration.
pub struct ProcessorRegistryBuilder {
    inner: slick::TypedRegistryBuilder<Arc<dyn Processor>, ProcessorError>,
}

impl ProcessorRegistryBuilder {
    pub fn new() -> Self {
        Self {
            inner: slick::TypedRegistryBuilder::new(),
        }
    }

    /// Collect all extensions registered via `inventory::submit!`.
    ///
    /// Call once in the composition root. This never changes regardless
    /// of how many extensions exist — adding an extension only requires
    /// a Cargo dependency and a config entry.
    ///
    /// # Panics
    ///
    /// Panics if two extensions register the same type URL. Duplicate
    /// registrations are a configuration error — silent override would
    /// be non-deterministic (depends on platform-specific static init
    /// ordering).
    #[must_use]
    pub fn collect_extensions(mut self) -> Self {
        for reg in inventory::iter::<ProcessorRegistration> {
            self.inner = self.inner.register_unique(reg.type_url, reg.factory);
        }
        self
    }

    /// Register a processor factory explicitly via the [`IntoProcessor`] trait.
    ///
    /// Monomorphizes `T::Config` deserialization into a type-erased closure.
    /// Useful for tests or when you want explicit control over registration.
    #[must_use]
    pub fn with<T: IntoProcessor>(mut self, type_url: &str) -> Self {
        let url_for_closure = type_url.to_owned();
        self.inner = self.inner.register(type_url, move |value: &serde_json::Value| {
            let config: T::Config = serde_json::from_value(value.clone()).map_err(|e| {
                ProcessorError::new(
                    &url_for_closure,
                    format!("config deserialization failed: {e}"),
                )
            })?;
            T::from_config(config)
        });
        self
    }

    /// Freeze the registry. No further registrations possible.
    pub fn build(self) -> ProcessorRegistry {
        ProcessorRegistry {
            inner: self.inner.build(),
        }
    }
}

impl Default for ProcessorRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Flatten `RegistryError<ProcessorError>` → `ProcessorError`.
fn flatten_error(e: RegistryError<ProcessorError>) -> ProcessorError {
    match e {
        RegistryError::UnknownTypeUrl {
            type_url,
            available,
        } => ProcessorError::new(
            &type_url,
            format!(
                "unknown type URL '{}'. registered: [{}]",
                type_url,
                available.join(", ")
            ),
        ),
        RegistryError::Factory { source, .. } => source,
    }
}

/// Immutable processor registry. Maps type URL → factory.
///
/// Created via [`ProcessorRegistryBuilder::build`]. Thread-safe and
/// shareable via `Arc`.
pub struct ProcessorRegistry {
    inner: slick::TypedRegistry<Arc<dyn Processor>, ProcessorError>,
}

impl ProcessorRegistry {
    /// Instantiate a processor from a type URL and config.
    ///
    /// Looks up the factory by `type_url`, deserializes the config
    /// into the factory's `Config` type, and returns `Arc<dyn Processor>`.
    pub fn create(
        &self,
        type_url: &str,
        config: &serde_json::Value,
    ) -> Result<Arc<dyn Processor>, ProcessorError> {
        self.inner.create(type_url, config).map_err(flatten_error)
    }

    /// Instantiate processors from a list of typed config entries.
    ///
    /// Convenience method for building a pipeline from config.
    pub fn create_pipeline(
        &self,
        configs: &[TypedConfig],
    ) -> Result<Vec<Arc<dyn Processor>>, ProcessorError> {
        configs
            .iter()
            .map(|tc| self.create(&tc.type_url, &tc.config))
            .collect()
    }

    /// List all registered type URLs (for diagnostics).
    pub fn type_urls(&self) -> Vec<&str> {
        self.inner.type_urls()
    }

    /// Returns the number of registered factories.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns true if no factories are registered.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase::PhaseResult;
    use crate::processor::BoxFuture;
    use rumi_http::HttpMessage;

    /// Helper to extract error from Result<Arc<dyn Processor>, ProcessorError>
    /// since Arc<dyn Processor> doesn't impl Debug.
    fn unwrap_err(result: Result<Arc<dyn Processor>, ProcessorError>) -> ProcessorError {
        match result {
            Err(e) => e,
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    fn unwrap_err_vec(
        result: Result<Vec<Arc<dyn Processor>>, ProcessorError>,
    ) -> ProcessorError {
        match result {
            Err(e) => e,
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    // -- Test processor --

    #[allow(dead_code)]
    struct EchoProcessor {
        greeting: String,
    }

    impl Processor for EchoProcessor {
        fn name(&self) -> &str {
            "echo"
        }

        fn process_request_headers(
            &self,
            _msg: &HttpMessage,
        ) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>> {
            Box::pin(async { Ok(PhaseResult::Continue) })
        }
    }

    #[derive(serde::Deserialize)]
    struct EchoConfig {
        greeting: String,
    }

    impl IntoProcessor for EchoProcessor {
        type Config = EchoConfig;

        fn from_config(config: Self::Config) -> Result<Arc<dyn Processor>, ProcessorError> {
            Ok(Arc::new(EchoProcessor {
                greeting: config.greeting,
            }))
        }
    }

    // -- Test processor 2 (for multi-registration) --

    struct NoopProcessor;

    impl Processor for NoopProcessor {
        fn name(&self) -> &str {
            "noop"
        }
    }

    #[derive(serde::Deserialize)]
    struct NoopConfig {}

    impl IntoProcessor for NoopProcessor {
        type Config = NoopConfig;

        fn from_config(_config: Self::Config) -> Result<Arc<dyn Processor>, ProcessorError> {
            Ok(Arc::new(NoopProcessor))
        }
    }

    // -- Tests --

    #[test]
    fn builder_with_registers_factory() {
        let registry = ProcessorRegistryBuilder::new()
            .with::<EchoProcessor>("test.echo.v1")
            .build();

        assert_eq!(registry.len(), 1);
        assert_eq!(registry.type_urls(), vec!["test.echo.v1"]);
    }

    #[test]
    fn create_instantiates_processor() {
        let registry = ProcessorRegistryBuilder::new()
            .with::<EchoProcessor>("test.echo.v1")
            .build();

        let config = serde_json::json!({ "greeting": "hello" });
        let processor = registry.create("test.echo.v1", &config).unwrap();
        assert_eq!(processor.name(), "echo");
    }

    #[test]
    fn create_unknown_type_url_errors() {
        let registry = ProcessorRegistryBuilder::new()
            .with::<EchoProcessor>("test.echo.v1")
            .build();

        let config = serde_json::json!({});
        let err = unwrap_err(registry.create("test.missing.v1", &config));
        assert!(err.message.contains("unknown type URL"));
        assert!(err.message.contains("test.echo.v1"));
    }

    #[test]
    fn create_bad_config_errors() {
        let registry = ProcessorRegistryBuilder::new()
            .with::<EchoProcessor>("test.echo.v1")
            .build();

        let config = serde_json::json!({ "wrong_field": 42 });
        let err = unwrap_err(registry.create("test.echo.v1", &config));
        assert!(err.message.contains("config deserialization failed"));
    }

    #[test]
    fn multiple_registrations() {
        let registry = ProcessorRegistryBuilder::new()
            .with::<EchoProcessor>("test.echo.v1")
            .with::<NoopProcessor>("test.noop.v1")
            .build();

        assert_eq!(registry.len(), 2);
        assert_eq!(registry.type_urls(), vec!["test.echo.v1", "test.noop.v1"]);
    }

    #[test]
    fn create_pipeline_from_configs() {
        let registry = ProcessorRegistryBuilder::new()
            .with::<EchoProcessor>("test.echo.v1")
            .with::<NoopProcessor>("test.noop.v1")
            .build();

        let configs = vec![
            TypedConfig {
                type_url: "test.echo.v1".into(),
                config: serde_json::json!({ "greeting": "hi" }),
            },
            TypedConfig {
                type_url: "test.noop.v1".into(),
                config: serde_json::json!({}),
            },
        ];

        let pipeline = registry.create_pipeline(&configs).unwrap();
        assert_eq!(pipeline.len(), 2);
        assert_eq!(pipeline[0].name(), "echo");
        assert_eq!(pipeline[1].name(), "noop");
    }

    #[test]
    fn create_pipeline_fails_on_bad_type_url() {
        let registry = ProcessorRegistryBuilder::new()
            .with::<EchoProcessor>("test.echo.v1")
            .build();

        let configs = vec![
            TypedConfig {
                type_url: "test.echo.v1".into(),
                config: serde_json::json!({ "greeting": "hi" }),
            },
            TypedConfig {
                type_url: "test.missing.v1".into(),
                config: serde_json::json!({}),
            },
        ];

        let err = unwrap_err_vec(registry.create_pipeline(&configs));
        assert!(err.message.contains("unknown type URL"));
    }

    #[test]
    fn empty_registry() {
        let registry = ProcessorRegistryBuilder::new().build();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.type_urls().is_empty());
    }

    #[test]
    fn collect_extensions_picks_up_inventory() {
        // inventory::submit! happens at static init, so this test verifies
        // that collect_extensions() at least runs without panicking.
        // Actual cross-crate inventory collection is tested in integration tests.
        let registry = ProcessorRegistryBuilder::new()
            .collect_extensions()
            .build();

        // No extensions registered in this crate's tests, so should be empty
        // (unless other test crates have inventory::submit! — which they don't yet).
        // The point: collect_extensions() doesn't panic.
        let _ = registry.len();
    }

    #[test]
    fn last_registration_wins_on_duplicate_type_url() {
        let registry = ProcessorRegistryBuilder::new()
            .with::<EchoProcessor>("test.dup.v1")
            .with::<NoopProcessor>("test.dup.v1")
            .build();

        assert_eq!(registry.len(), 1);

        // NoopProcessor was registered last — it should win.
        let config = serde_json::json!({});
        let processor = registry.create("test.dup.v1", &config).unwrap();
        assert_eq!(processor.name(), "noop");
    }

    #[test]
    fn typed_config_deserializes_from_json() {
        let json = r#"{"type_url": "mox.geist.processors.v1.Test", "config": {"key": "value"}}"#;
        let tc: TypedConfig = serde_json::from_str(json).unwrap();
        assert_eq!(tc.type_url, "mox.geist.processors.v1.Test");
        assert_eq!(tc.config["key"], "value");
    }
}
