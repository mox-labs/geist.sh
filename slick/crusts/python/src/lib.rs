//! slick — Python bindings for slick-types via PyO3.
//!
//! Exposes SLICK's component type system to Python: ComponentKind,
//! ComponentManifest, ComponentContract, BehavioralEnvelope, TypedConfig.
//!
//! Same types as the Rust canonical definitions, compiled into a native
//! Python extension. No hand-maintained Python types — single source of truth.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

// ═══════════════════════════════════════════════════════════════════════
// ComponentKind
// ═══════════════════════════════════════════════════════════════════════

/// The 4 core component kinds. Each implies a different runtime contract.
#[pyclass(frozen, eq, hash)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentKind {
    /// Autonomous reasoning, session-based.
    Agent,
    /// Stateless function, single invocation.
    Capability,
    /// Knowledge/context, no execution.
    Skill,
    /// Orchestrated DAG, artifact ledger.
    Flow,
}

#[pymethods]
impl ComponentKind {
    fn __repr__(&self) -> &'static str {
        match self {
            Self::Agent => "ComponentKind.Agent",
            Self::Capability => "ComponentKind.Capability",
            Self::Skill => "ComponentKind.Skill",
            Self::Flow => "ComponentKind.Flow",
        }
    }

    fn __str__(&self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Capability => "capability",
            Self::Skill => "skill",
            Self::Flow => "flow",
        }
    }
}

impl From<ComponentKind> for slick_types::ComponentKind {
    fn from(kind: ComponentKind) -> Self {
        match kind {
            ComponentKind::Agent => Self::Agent,
            ComponentKind::Capability => Self::Capability,
            ComponentKind::Skill => Self::Skill,
            ComponentKind::Flow => Self::Flow,
        }
    }
}

impl From<slick_types::ComponentKind> for ComponentKind {
    fn from(kind: slick_types::ComponentKind) -> Self {
        match kind {
            slick_types::ComponentKind::Agent => Self::Agent,
            slick_types::ComponentKind::Capability => Self::Capability,
            slick_types::ComponentKind::Skill => Self::Skill,
            slick_types::ComponentKind::Flow => Self::Flow,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ResourceBounds
// ═══════════════════════════════════════════════════════════════════════

/// Resource consumption bounds for a component.
#[pyclass(frozen, get_all)]
#[derive(Debug, Clone)]
pub struct ResourceBounds {
    pub max_tokens: Option<u64>,
    pub max_latency_ms: Option<u64>,
    pub max_cost_usd: Option<f64>,
}

#[pymethods]
impl ResourceBounds {
    #[new]
    #[pyo3(signature = (max_tokens=None, max_latency_ms=None, max_cost_usd=None))]
    fn new(
        max_tokens: Option<u64>,
        max_latency_ms: Option<u64>,
        max_cost_usd: Option<f64>,
    ) -> Self {
        Self {
            max_tokens,
            max_latency_ms,
            max_cost_usd,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ResourceBounds(max_tokens={:?}, max_latency_ms={:?}, max_cost_usd={:?})",
            self.max_tokens, self.max_latency_ms, self.max_cost_usd
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════
// BehavioralEnvelope
// ═══════════════════════════════════════════════════════════════════════

/// Declared behavioral variance.
#[pyclass(frozen, get_all)]
#[derive(Debug, Clone)]
pub struct BehavioralEnvelope {
    pub resource_bounds: Option<ResourceBounds>,
    pub degradation: Vec<String>,
}

#[pymethods]
impl BehavioralEnvelope {
    #[new]
    #[pyo3(signature = (resource_bounds=None, degradation=None))]
    fn new(resource_bounds: Option<ResourceBounds>, degradation: Option<Vec<String>>) -> Self {
        Self {
            resource_bounds,
            degradation: degradation.unwrap_or_default(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ComponentContract
// ═══════════════════════════════════════════════════════════════════════

/// What a component consumes and produces (for DAG composition).
#[pyclass(frozen, get_all)]
#[derive(Debug, Clone)]
pub struct ComponentContract {
    pub consumes: Vec<String>,
    pub produces: Option<String>,
    pub assertions: Vec<String>,
    pub boundaries: Vec<String>,
}

#[pymethods]
impl ComponentContract {
    #[new]
    #[pyo3(signature = (consumes=None, produces=None, assertions=None, boundaries=None))]
    fn new(
        consumes: Option<Vec<String>>,
        produces: Option<String>,
        assertions: Option<Vec<String>>,
        boundaries: Option<Vec<String>>,
    ) -> Self {
        Self {
            consumes: consumes.unwrap_or_default(),
            produces,
            assertions: assertions.unwrap_or_default(),
            boundaries: boundaries.unwrap_or_default(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ComponentManifest
// ═══════════════════════════════════════════════════════════════════════

/// Authoring-layer component manifest (CRD equivalent).
#[pyclass(frozen, get_all)]
#[derive(Debug, Clone)]
pub struct ComponentManifest {
    pub kind: ComponentKind,
    pub type_url: String,
    pub description: String,
    pub contract: ComponentContract,
    pub envelope: BehavioralEnvelope,
}

#[pymethods]
impl ComponentManifest {
    #[new]
    #[pyo3(signature = (kind, type_url, description, contract=None, envelope=None))]
    fn new(
        kind: ComponentKind,
        type_url: String,
        description: String,
        contract: Option<ComponentContract>,
        envelope: Option<BehavioralEnvelope>,
    ) -> Self {
        Self {
            kind,
            type_url,
            description,
            contract: contract.unwrap_or_else(|| ComponentContract::new(None, None, None, None)),
            envelope: envelope.unwrap_or_else(|| BehavioralEnvelope::new(None, None)),
        }
    }

    /// Serialize to JSON string.
    fn to_json(&self) -> PyResult<String> {
        let inner = to_inner_manifest(self);
        serde_json::to_string_pretty(&inner)
            .map_err(|e| PyValueError::new_err(format!("serialization failed: {e}")))
    }

    /// Deserialize from JSON string.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: slick_types::ComponentManifest =
            serde_json::from_str(json).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(from_inner_manifest(inner))
    }

    fn __repr__(&self) -> String {
        format!(
            "ComponentManifest(kind={}, type_url={:?})",
            self.kind.__str__(),
            self.type_url
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TypedConfig
// ═══════════════════════════════════════════════════════════════════════

/// Config envelope: type URL + opaque config JSON.
#[pyclass(frozen, get_all)]
#[derive(Debug, Clone)]
pub struct TypedConfig {
    pub type_url: String,
    pub config: String, // JSON string (opaque to Python)
}

#[pymethods]
impl TypedConfig {
    #[new]
    fn new(type_url: String, config: String) -> PyResult<Self> {
        // Validate config is valid JSON
        let _: serde_json::Value = serde_json::from_str(&config)
            .map_err(|e| PyValueError::new_err(format!("invalid JSON config: {e}")))?;
        Ok(Self { type_url, config })
    }

    fn __repr__(&self) -> String {
        format!("TypedConfig(type_url={:?})", self.type_url)
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Conversion helpers (PyO3 ↔ slick-types)
// ═══════════════════════════════════════════════════════════════════════

fn to_inner_manifest(m: &ComponentManifest) -> slick_types::ComponentManifest {
    slick_types::ComponentManifest {
        kind: m.kind.into(),
        type_url: m.type_url.clone(),
        description: m.description.clone(),
        contract: slick_types::ComponentContract {
            consumes: m.contract.consumes.clone(),
            produces: m.contract.produces.clone(),
            assertions: m.contract.assertions.clone(),
            boundaries: m.contract.boundaries.clone(),
        },
        envelope: slick_types::BehavioralEnvelope {
            resource_bounds: m.envelope.resource_bounds.as_ref().map(|rb| {
                slick_types::ResourceBounds {
                    max_tokens: rb.max_tokens,
                    max_latency_ms: rb.max_latency_ms,
                    max_cost_usd: rb.max_cost_usd,
                }
            }),
            degradation: m.envelope.degradation.clone(),
        },
    }
}

fn from_inner_manifest(inner: slick_types::ComponentManifest) -> ComponentManifest {
    ComponentManifest {
        kind: inner.kind.into(),
        type_url: inner.type_url,
        description: inner.description,
        contract: ComponentContract {
            consumes: inner.contract.consumes,
            produces: inner.contract.produces,
            assertions: inner.contract.assertions,
            boundaries: inner.contract.boundaries,
        },
        envelope: BehavioralEnvelope {
            resource_bounds: inner.envelope.resource_bounds.map(|rb| ResourceBounds {
                max_tokens: rb.max_tokens,
                max_latency_ms: rb.max_latency_ms,
                max_cost_usd: rb.max_cost_usd,
            }),
            degradation: inner.envelope.degradation,
        },
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Module
// ═══════════════════════════════════════════════════════════════════════

/// Python module: `slick`
#[pymodule]
fn slick(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ComponentKind>()?;
    m.add_class::<ComponentManifest>()?;
    m.add_class::<ComponentContract>()?;
    m.add_class::<BehavioralEnvelope>()?;
    m.add_class::<ResourceBounds>()?;
    m.add_class::<TypedConfig>()?;
    Ok(())
}
