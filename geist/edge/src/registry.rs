//! Processor extension registration — bridges [`IntoProcessor`] to slickit's
//! [`TypedRegistry`](slick::TypedRegistry).
//!
//! Extension crates self-register via [`register_processor!`] (wraps
//! `inventory::submit!`). The binary collects all registrations via
//! [`collect_processor_extensions`] and gets back a
//! [`TypedRegistryBuilder`](slick::TypedRegistryBuilder) ready to `.build()`.
//!
//! Adding a new processor extension requires zero code changes to
//! geist-edge or the binary — only a Cargo dependency and a config entry.
//!
//! # Pattern
//!
//! ```text
//! Extension crate:  register_processor!("mox.geist.processors.v1.Foo", FooProcessor)
//! Binary:           let registry = collect_processor_extensions().build();
//! Config:           [{ "type_url": "mox.geist.processors.v1.Foo", "config": {...} }]
//! ```

use std::sync::Arc;

use serde::de::DeserializeOwned;

use crate::processor::{Processor, ProcessorError};

// Re-export slickit types — single source of truth.
pub use slick::{TypedConfig, TypedRegistry, TypedRegistryBuilder};

/// Processor registry — slickit's TypedRegistry specialized for processors.
pub type ProcessorRegistry = TypedRegistry<Arc<dyn Processor>, ProcessorError>;

/// Factory trait for processor extensions.
///
/// Each processor defines its `Config` type (policy + runtime config combined).
/// The trait is monomorphized at registration time and type-erased at runtime
/// via the closure captured in [`register_processor!`].
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
    fn from_config(config: Self::Config) -> Result<Arc<dyn Processor>, ProcessorError>;
}

/// A self-contained processor registration for distributed static collection.
///
/// Extension crates submit these via [`register_processor!`] (preferred) or
/// raw `inventory::submit!` (advanced). The binary collects them via
/// [`collect_processor_extensions`].
///
/// # Example
///
/// Preferred — via macro:
///
/// ```ignore
/// geist_edge::register_processor!(
///     "mox.geist.processors.v1.AccessControl",
///     AccessControlProcessor
/// );
/// ```
///
/// Advanced — raw registration:
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
/// Bridges the [`IntoProcessor`] trait to distributed static registration.
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

/// Collect all processor extensions registered via `inventory::submit!`.
///
/// Returns a [`TypedRegistryBuilder`] seeded with all discovered extensions.
/// Call `.build()` to freeze it into an immutable [`ProcessorRegistry`].
///
/// ```ignore
/// let registry = collect_processor_extensions().build();
/// ```
///
/// # Panics
///
/// Panics if two extensions register the same type URL.
pub fn collect_processor_extensions(
) -> TypedRegistryBuilder<Arc<dyn Processor>, ProcessorError> {
    let mut builder = TypedRegistryBuilder::new();
    for reg in inventory::iter::<ProcessorRegistration> {
        builder = builder.register_unique(reg.type_url, reg.factory);
    }
    builder
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase::PhaseResult;
    use crate::processor::BoxFuture;
    use rumi_http::HttpMessage;

    /// Helper to extract error since Arc<dyn Processor> doesn't impl Debug.
    fn unwrap_err(
        result: Result<Arc<dyn Processor>, slick::RegistryError<ProcessorError>>,
    ) -> String {
        match result {
            Err(e) => e.to_string(),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    /// Test helper: register an IntoProcessor type on a builder.
    fn with_processor<T: IntoProcessor>(
        builder: TypedRegistryBuilder<Arc<dyn Processor>, ProcessorError>,
        type_url: &str,
    ) -> TypedRegistryBuilder<Arc<dyn Processor>, ProcessorError> {
        let url = type_url.to_owned();
        builder.register(type_url, move |value: &serde_json::Value| {
            let config: T::Config = serde_json::from_value(value.clone()).map_err(|e| {
                ProcessorError::new(&url, format!("config deserialization failed: {e}"))
            })?;
            T::from_config(config)
        })
    }

    // -- Test processors --

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
    fn builder_registers_factory() {
        let registry = with_processor::<EchoProcessor>(TypedRegistryBuilder::new(), "test.echo.v1")
            .build();

        assert_eq!(registry.len(), 1);
        assert_eq!(registry.type_urls(), vec!["test.echo.v1"]);
    }

    #[test]
    fn create_instantiates_processor() {
        let registry = with_processor::<EchoProcessor>(TypedRegistryBuilder::new(), "test.echo.v1")
            .build();

        let config = serde_json::json!({ "greeting": "hello" });
        let processor = registry.create("test.echo.v1", &config).unwrap();
        assert_eq!(processor.name(), "echo");
    }

    #[test]
    fn create_unknown_type_url_errors() {
        let registry = with_processor::<EchoProcessor>(TypedRegistryBuilder::new(), "test.echo.v1")
            .build();

        let config = serde_json::json!({});
        let err = unwrap_err(registry.create("test.missing.v1", &config));
        assert!(err.contains("test.missing.v1"));
        assert!(err.contains("test.echo.v1"));
    }

    #[test]
    fn create_bad_config_errors() {
        let registry = with_processor::<EchoProcessor>(TypedRegistryBuilder::new(), "test.echo.v1")
            .build();

        let config = serde_json::json!({ "wrong_field": 42 });
        let err = unwrap_err(registry.create("test.echo.v1", &config));
        assert!(err.contains("config deserialization failed"));
    }

    #[test]
    fn multiple_registrations() {
        let builder = TypedRegistryBuilder::new();
        let builder = with_processor::<EchoProcessor>(builder, "test.echo.v1");
        let builder = with_processor::<NoopProcessor>(builder, "test.noop.v1");
        let registry = builder.build();

        assert_eq!(registry.len(), 2);
        assert_eq!(registry.type_urls(), vec!["test.echo.v1", "test.noop.v1"]);
    }

    #[test]
    fn create_pipeline_from_configs() {
        let builder = TypedRegistryBuilder::new();
        let builder = with_processor::<EchoProcessor>(builder, "test.echo.v1");
        let builder = with_processor::<NoopProcessor>(builder, "test.noop.v1");
        let registry = builder.build();

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
        let registry = with_processor::<EchoProcessor>(TypedRegistryBuilder::new(), "test.echo.v1")
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

        let err = registry.create_pipeline(&configs);
        assert!(err.is_err());
    }

    #[test]
    fn empty_registry() {
        let registry: ProcessorRegistry =
            TypedRegistryBuilder::<Arc<dyn Processor>, ProcessorError>::new().build();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.type_urls().is_empty());
    }

    #[test]
    fn collect_extensions_runs() {
        let registry = collect_processor_extensions().build();
        let _ = registry.len();
    }

    #[test]
    fn last_registration_wins_on_duplicate_type_url() {
        let builder = TypedRegistryBuilder::new();
        let builder = with_processor::<EchoProcessor>(builder, "test.dup.v1");
        let builder = with_processor::<NoopProcessor>(builder, "test.dup.v1");
        let registry = builder.build();

        assert_eq!(registry.len(), 1);

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
