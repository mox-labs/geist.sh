//! Generic typed extension registry.
//!
//! Maps type URL → factory closure → typed instance. Builder pattern
//! ensures the registry is immutable after construction.

use std::collections::HashMap;
use std::fmt;

/// A typed config entry from pipeline configuration.
///
/// Same shape as Envoy's `TypedExtensionConfig`: a type URL that identifies
/// the factory, and an opaque config blob deserialized by the factory.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypedConfig {
    /// Factory lookup key (e.g. `mox.geist.processors.v1.AccessControl`).
    pub type_url: String,
    /// Opaque config — each factory deserializes into its own type.
    pub config: serde_json::Value,
}

/// Error returned by [`TypedRegistry::create`] and [`TypedRegistry::create_pipeline`].
#[derive(Debug)]
pub enum RegistryError<E> {
    /// No factory registered for this type URL.
    UnknownTypeUrl {
        type_url: String,
        available: Vec<String>,
    },
    /// Factory returned an error during instantiation.
    Factory { type_url: String, source: E },
}

impl<E: fmt::Display> fmt::Display for RegistryError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTypeUrl {
                type_url,
                available,
            } => write!(
                f,
                "unknown type URL '{}'. registered: [{}]",
                type_url,
                available.join(", ")
            ),
            Self::Factory { type_url, source } => {
                write!(f, "factory error for '{}': {}", type_url, source)
            }
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for RegistryError<E> {}

// Type-erased factory stored in the registry.
type BoxedFactory<T, E> =
    Box<dyn Fn(&serde_json::Value) -> Result<T, E> + Send + Sync>;

/// Mutable builder for [`TypedRegistry`]. Immutable after [`build()`](Self::build).
pub struct TypedRegistryBuilder<T, E> {
    factories: HashMap<String, BoxedFactory<T, E>>,
}

impl<T, E> TypedRegistryBuilder<T, E> {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    /// Register a factory for a type URL. Last registration wins on duplicate.
    #[must_use]
    pub fn register(
        mut self,
        type_url: &str,
        factory: impl Fn(&serde_json::Value) -> Result<T, E> + Send + Sync + 'static,
    ) -> Self {
        self.factories
            .insert(type_url.to_owned(), Box::new(factory));
        self
    }

    /// Register a factory, panicking if the type URL is already registered.
    ///
    /// Use this for distributed registration (e.g. `inventory::submit!`)
    /// where duplicates indicate a configuration error.
    #[must_use]
    pub fn register_unique(
        mut self,
        type_url: &str,
        factory: impl Fn(&serde_json::Value) -> Result<T, E> + Send + Sync + 'static,
    ) -> Self {
        if self.factories.contains_key(type_url) {
            panic!(
                "duplicate type URL '{}' — each type URL must be registered exactly once.",
                type_url
            );
        }
        self.factories
            .insert(type_url.to_owned(), Box::new(factory));
        self
    }

    /// Returns true if a factory is registered for this type URL.
    pub fn contains(&self, type_url: &str) -> bool {
        self.factories.contains_key(type_url)
    }

    /// Freeze the registry. No further registrations possible.
    pub fn build(self) -> TypedRegistry<T, E> {
        TypedRegistry {
            factories: self.factories,
        }
    }
}

impl<T, E> Default for TypedRegistryBuilder<T, E> {
    fn default() -> Self {
        Self::new()
    }
}

/// Immutable typed registry. Maps type URL → factory.
///
/// Created via [`TypedRegistryBuilder::build`]. Thread-safe and
/// shareable via `Arc`.
pub struct TypedRegistry<T, E> {
    factories: HashMap<String, BoxedFactory<T, E>>,
}

impl<T, E> TypedRegistry<T, E> {
    /// Instantiate a value from a type URL and config.
    pub fn create(
        &self,
        type_url: &str,
        config: &serde_json::Value,
    ) -> Result<T, RegistryError<E>> {
        let factory = self.factories.get(type_url).ok_or_else(|| {
            RegistryError::UnknownTypeUrl {
                type_url: type_url.to_owned(),
                available: self.type_urls_owned(),
            }
        })?;
        factory(config).map_err(|source| RegistryError::Factory {
            type_url: type_url.to_owned(),
            source,
        })
    }

    /// Instantiate values from a list of typed config entries.
    pub fn create_pipeline(
        &self,
        configs: &[TypedConfig],
    ) -> Result<Vec<T>, RegistryError<E>> {
        configs
            .iter()
            .map(|tc| self.create(&tc.type_url, &tc.config))
            .collect()
    }

    /// List all registered type URLs, sorted (for diagnostics).
    pub fn type_urls(&self) -> Vec<&str> {
        let mut urls: Vec<&str> = self.factories.keys().map(|s| s.as_str()).collect();
        urls.sort_unstable();
        urls
    }

    /// Returns the number of registered factories.
    pub fn len(&self) -> usize {
        self.factories.len()
    }

    /// Returns true if no factories are registered.
    pub fn is_empty(&self) -> bool {
        self.factories.is_empty()
    }

    /// Sorted owned type URLs (for error messages).
    fn type_urls_owned(&self) -> Vec<String> {
        let mut urls: Vec<String> = self.factories.keys().cloned().collect();
        urls.sort_unstable();
        urls
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Helpers --

    fn echo_factory(value: &serde_json::Value) -> Result<String, String> {
        serde_json::from_value::<String>(value.clone()).map_err(|e| e.to_string())
    }

    fn int_factory(value: &serde_json::Value) -> Result<String, String> {
        let n: i64 =
            serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        Ok(format!("int:{n}"))
    }

    fn failing_factory(_value: &serde_json::Value) -> Result<String, String> {
        Err("construction failed".to_owned())
    }

    // -- Builder tests --

    #[test]
    fn register_adds_factory() {
        let registry = TypedRegistryBuilder::<String, String>::new()
            .register("test.echo.v1", echo_factory)
            .build();

        assert_eq!(registry.len(), 1);
        assert_eq!(registry.type_urls(), vec!["test.echo.v1"]);
    }

    #[test]
    fn register_multiple() {
        let registry = TypedRegistryBuilder::<String, String>::new()
            .register("test.echo.v1", echo_factory)
            .register("test.int.v1", int_factory)
            .build();

        assert_eq!(registry.len(), 2);
        assert_eq!(
            registry.type_urls(),
            vec!["test.echo.v1", "test.int.v1"]
        );
    }

    #[test]
    fn register_duplicate_last_wins() {
        let registry = TypedRegistryBuilder::<String, String>::new()
            .register("test.v1", echo_factory)
            .register("test.v1", int_factory)
            .build();

        assert_eq!(registry.len(), 1);
        let result = registry
            .create("test.v1", &serde_json::json!(42))
            .unwrap();
        assert_eq!(result, "int:42");
    }

    #[test]
    #[should_panic(expected = "duplicate type URL")]
    fn register_unique_panics_on_duplicate() {
        let _ = TypedRegistryBuilder::<String, String>::new()
            .register_unique("test.v1", echo_factory)
            .register_unique("test.v1", int_factory)
            .build();
    }

    #[test]
    fn contains_checks_registration() {
        let builder = TypedRegistryBuilder::<String, String>::new()
            .register("test.echo.v1", echo_factory);

        assert!(builder.contains("test.echo.v1"));
        assert!(!builder.contains("test.missing.v1"));
    }

    // -- Create tests --

    #[test]
    fn create_returns_instance() {
        let registry = TypedRegistryBuilder::<String, String>::new()
            .register("test.echo.v1", echo_factory)
            .build();

        let result = registry
            .create("test.echo.v1", &serde_json::json!("hello"))
            .unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn create_unknown_type_url() {
        let registry = TypedRegistryBuilder::<String, String>::new()
            .register("test.echo.v1", echo_factory)
            .build();

        let err = registry
            .create("test.missing.v1", &serde_json::json!("x"))
            .unwrap_err();
        match err {
            RegistryError::UnknownTypeUrl {
                type_url,
                available,
            } => {
                assert_eq!(type_url, "test.missing.v1");
                assert_eq!(available, vec!["test.echo.v1"]);
            }
            _ => panic!("expected UnknownTypeUrl"),
        }
    }

    #[test]
    fn create_factory_error() {
        let registry = TypedRegistryBuilder::<String, String>::new()
            .register("test.fail.v1", failing_factory)
            .build();

        let err = registry
            .create("test.fail.v1", &serde_json::json!(null))
            .unwrap_err();
        match err {
            RegistryError::Factory { type_url, source } => {
                assert_eq!(type_url, "test.fail.v1");
                assert_eq!(source, "construction failed");
            }
            _ => panic!("expected Factory error"),
        }
    }

    // -- Pipeline tests --

    #[test]
    fn create_pipeline_success() {
        let registry = TypedRegistryBuilder::<String, String>::new()
            .register("test.echo.v1", echo_factory)
            .register("test.int.v1", int_factory)
            .build();

        let configs = vec![
            TypedConfig {
                type_url: "test.echo.v1".into(),
                config: serde_json::json!("hi"),
            },
            TypedConfig {
                type_url: "test.int.v1".into(),
                config: serde_json::json!(42),
            },
        ];

        let pipeline = registry.create_pipeline(&configs).unwrap();
        assert_eq!(pipeline, vec!["hi", "int:42"]);
    }

    #[test]
    fn create_pipeline_fails_on_unknown() {
        let registry = TypedRegistryBuilder::<String, String>::new()
            .register("test.echo.v1", echo_factory)
            .build();

        let configs = vec![
            TypedConfig {
                type_url: "test.echo.v1".into(),
                config: serde_json::json!("hi"),
            },
            TypedConfig {
                type_url: "test.missing.v1".into(),
                config: serde_json::json!(null),
            },
        ];

        let err = registry.create_pipeline(&configs).unwrap_err();
        assert!(matches!(err, RegistryError::UnknownTypeUrl { .. }));
    }

    // -- Empty registry --

    #[test]
    fn empty_registry() {
        let registry = TypedRegistryBuilder::<String, String>::new().build();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.type_urls().is_empty());
    }

    // -- TypedConfig deserialization --

    #[test]
    fn typed_config_deserializes() {
        let json =
            r#"{"type_url": "mox.geist.processors.v1.Test", "config": {"key": "value"}}"#;
        let tc: TypedConfig = serde_json::from_str(json).unwrap();
        assert_eq!(tc.type_url, "mox.geist.processors.v1.Test");
        assert_eq!(tc.config["key"], "value");
    }

    #[test]
    fn typed_config_serializes_roundtrip() {
        let tc = TypedConfig {
            type_url: "test.v1".into(),
            config: serde_json::json!({"a": 1}),
        };
        let json = serde_json::to_string(&tc).unwrap();
        let tc2: TypedConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(tc2.type_url, "test.v1");
        assert_eq!(tc2.config, serde_json::json!({"a": 1}));
    }

    // -- Display --

    #[test]
    fn registry_error_display() {
        let err: RegistryError<String> = RegistryError::UnknownTypeUrl {
            type_url: "x.v1".into(),
            available: vec!["a.v1".into(), "b.v1".into()],
        };
        assert_eq!(
            err.to_string(),
            "unknown type URL 'x.v1'. registered: [a.v1, b.v1]"
        );

        let err: RegistryError<String> = RegistryError::Factory {
            type_url: "x.v1".into(),
            source: "boom".into(),
        };
        assert_eq!(err.to_string(), "factory error for 'x.v1': boom");
    }
}
