//! SLICK component manifest — authoring-layer types (CRD equivalent).
//!
//! These types describe components at authoring time: what kind they are,
//! what they consume and produce, their behavioral envelope. The Composer
//! uses manifests to plan pipelines. The runtime layer ([`super::TypedConfig`],
//! [`super::TypedRegistry`]) instantiates them.
//!
//! # CRD/xDS Layering
//!
//! | Layer | Types | Purpose |
//! |-------|-------|---------|
//! | Authoring (CRD) | [`ComponentManifest`], [`ComponentKind`] | Describe + compose |
//! | Runtime (xDS) | [`super::TypedConfig`], [`super::TypedRegistry`] | Instantiate + run |
//!
//! Bridge: `ComponentManifest.type_url` = `TypedConfig.type_url`.
//!
//! # Cross-Surface
//!
//! These types are surface-agnostic. The same [`ComponentKind::Capability`]
//! describes a Rust processor, a Svelte panel, or a Python function.
//! Crusts (PyO3, wasm-bindgen) expose identical types to Python and TypeScript.

use serde::{Deserialize, Serialize};
use std::fmt;

/// The 4 core component kinds. Each implies a different runtime contract.
///
/// | Kind | Contract | Example |
/// |------|----------|---------|
/// | [`Agent`](Self::Agent) | Autonomous reasoning, session-based | Code review agent, HudAdapter |
/// | [`Capability`](Self::Capability) | Stateless function, single invocation | AccessControl processor, NavPanel |
/// | [`Skill`](Self::Skill) | Knowledge/context, no execution | Design tokens, domain framework |
/// | [`Flow`](Self::Flow) | Orchestrated DAG, artifact ledger | CI pipeline, wizard flow |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentKind {
    /// Autonomous reasoning with session state. Perceive → reason → act loop.
    Agent,
    /// Stateless, single invocation. Input → output, no session.
    Capability,
    /// Knowledge and context. Loaded into working memory, not executed.
    Skill,
    /// Orchestrated multi-step process. DAG with consumes/produces contracts.
    Flow,
}

impl fmt::Display for ComponentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Agent => write!(f, "agent"),
            Self::Capability => write!(f, "capability"),
            Self::Skill => write!(f, "skill"),
            Self::Flow => write!(f, "flow"),
        }
    }
}

/// Authoring-layer component manifest (CRD equivalent).
///
/// Produced by bodhi/hades (slick plugin), consumed by the Composer.
/// The `type_url` bridges to the runtime layer — it's the lookup key
/// in [`super::TypedRegistry`].
///
/// # Example
///
/// ```
/// use slick_types::manifest::{ComponentManifest, ComponentKind, ComponentContract, BehavioralEnvelope};
///
/// let manifest = ComponentManifest {
///     kind: ComponentKind::Capability,
///     type_url: "mox.geist.processors.v1.AccessControl".into(),
///     description: "Deny-first access control processor".into(),
///     contract: ComponentContract {
///         consumes: vec![],
///         produces: Some("mox.geist.v1.AuthResult".into()),
///         assertions: vec!["Denies by default when no rules match".into()],
///         boundaries: vec!["Does not handle authentication".into()],
///     },
///     envelope: BehavioralEnvelope::default(),
/// };
///
/// assert_eq!(manifest.kind, ComponentKind::Capability);
/// assert_eq!(manifest.type_url, "mox.geist.processors.v1.AccessControl");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentManifest {
    /// Behavioral contract category.
    pub kind: ComponentKind,
    /// Globally unique identity. Bridges to runtime layer.
    pub type_url: String,
    /// Human-readable description.
    pub description: String,
    /// What this component consumes, produces, asserts, and excludes.
    pub contract: ComponentContract,
    /// Declared behavioral variance.
    pub envelope: BehavioralEnvelope,
}

/// What a component consumes and produces (for DAG composition).
///
/// `consumes` and `produces` enable the Composer to verify pipelines are
/// well-formed before runtime: every consumed type URL has a producer,
/// and the graph is a DAG (no cycles).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentContract {
    /// Input type URLs this component requires.
    #[serde(default)]
    pub consumes: Vec<String>,
    /// Output type URL this component produces. `None` for Skills (context only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub produces: Option<String>,
    /// Testable statements the component must satisfy.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assertions: Vec<String>,
    /// What this component explicitly does NOT do.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boundaries: Vec<String>,
}

/// Declared behavioral variance (Skalse NeurIPS 2022: proxies degrade —
/// make imperfection legible).
///
/// Envelopes are informational at authoring time. Enforcement is eval's job.
/// 5 components at 95% reliability → 77.4% pipeline reliability.
/// Envelopes make composition risk legible.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BehavioralEnvelope {
    /// Resource consumption bounds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_bounds: Option<ResourceBounds>,
    /// Known failure modes and degradation behavior.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub degradation: Vec<String>,
}

/// Resource consumption bounds for a component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBounds {
    /// Maximum token consumption per invocation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    /// Maximum latency in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_latency_ms: Option<u64>,
    /// Maximum cost in USD per invocation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_cost_usd: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_kind_display() {
        assert_eq!(ComponentKind::Agent.to_string(), "agent");
        assert_eq!(ComponentKind::Capability.to_string(), "capability");
        assert_eq!(ComponentKind::Skill.to_string(), "skill");
        assert_eq!(ComponentKind::Flow.to_string(), "flow");
    }

    #[test]
    fn component_kind_serde_roundtrip() {
        for kind in [
            ComponentKind::Agent,
            ComponentKind::Capability,
            ComponentKind::Skill,
            ComponentKind::Flow,
        ] {
            let json = serde_json::to_string(&kind).unwrap();
            let back: ComponentKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, back);
        }
    }

    #[test]
    fn component_kind_deserializes_lowercase() {
        assert_eq!(
            serde_json::from_str::<ComponentKind>(r#""agent""#).unwrap(),
            ComponentKind::Agent
        );
        assert_eq!(
            serde_json::from_str::<ComponentKind>(r#""capability""#).unwrap(),
            ComponentKind::Capability
        );
        assert_eq!(
            serde_json::from_str::<ComponentKind>(r#""skill""#).unwrap(),
            ComponentKind::Skill
        );
        assert_eq!(
            serde_json::from_str::<ComponentKind>(r#""flow""#).unwrap(),
            ComponentKind::Flow
        );
    }

    #[test]
    fn manifest_serializes_roundtrip() {
        let manifest = ComponentManifest {
            kind: ComponentKind::Capability,
            type_url: "mox.geist.processors.v1.AccessControl".into(),
            description: "Deny-first access control".into(),
            contract: ComponentContract {
                consumes: vec![],
                produces: Some("mox.geist.v1.AuthResult".into()),
                assertions: vec!["Denies by default".into()],
                boundaries: vec!["Does not authenticate".into()],
            },
            envelope: BehavioralEnvelope {
                resource_bounds: Some(ResourceBounds {
                    max_tokens: None,
                    max_latency_ms: Some(5),
                    max_cost_usd: None,
                }),
                degradation: vec!["Fails closed on error".into()],
            },
        };

        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let back: ComponentManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind, ComponentKind::Capability);
        assert_eq!(back.type_url, "mox.geist.processors.v1.AccessControl");
        assert_eq!(back.contract.produces.as_deref(), Some("mox.geist.v1.AuthResult"));
        assert_eq!(back.envelope.resource_bounds.as_ref().unwrap().max_latency_ms, Some(5));
    }

    #[test]
    fn manifest_minimal_fields() {
        let json = r#"{
            "kind": "skill",
            "type_url": "slick.v1.RustMastery",
            "description": "Rust architectural judgment",
            "contract": {},
            "envelope": {}
        }"#;
        let manifest: ComponentManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.kind, ComponentKind::Skill);
        assert!(manifest.contract.consumes.is_empty());
        assert!(manifest.contract.produces.is_none());
        assert!(manifest.envelope.resource_bounds.is_none());
    }

    #[test]
    fn agent_manifest_with_session() {
        let manifest = ComponentManifest {
            kind: ComponentKind::Agent,
            type_url: "mox.hud.adapters.v1.Cloudflare".into(),
            description: "Cloudflare runtime adapter".into(),
            contract: ComponentContract {
                consumes: vec![],
                produces: Some("mox.hud.v1.Plane".into()),
                assertions: vec!["Lists all Cloudflare resources".into()],
                boundaries: vec!["Read-only, does not modify resources".into()],
            },
            envelope: BehavioralEnvelope::default(),
        };
        assert_eq!(manifest.kind, ComponentKind::Agent);
    }

    #[test]
    fn flow_manifest_with_consumes_produces() {
        let manifest = ComponentManifest {
            kind: ComponentKind::Flow,
            type_url: "ix.v1.ExperimentFlow".into(),
            description: "Probe → trial → sensor → reading".into(),
            contract: ComponentContract {
                consumes: vec!["ix.v1.Probes".into(), "ix.v1.Subject".into()],
                produces: Some("ix.v1.Readings".into()),
                assertions: vec!["Executes all probes against subject".into()],
                boundaries: vec![],
            },
            envelope: BehavioralEnvelope {
                resource_bounds: Some(ResourceBounds {
                    max_tokens: Some(50_000),
                    max_latency_ms: Some(30_000),
                    max_cost_usd: Some(0.50),
                }),
                degradation: vec![
                    "Partial results on timeout".into(),
                    "Skips failed probes".into(),
                ],
            },
        };

        assert_eq!(manifest.contract.consumes.len(), 2);
        assert_eq!(manifest.envelope.degradation.len(), 2);
    }

    #[test]
    fn svelte_panel_as_capability() {
        let manifest = ComponentManifest {
            kind: ComponentKind::Capability,
            type_url: "mox.hud.panels.v1.NavPanel".into(),
            description: "File tree explorer panel".into(),
            contract: ComponentContract {
                consumes: vec!["mox.hud.v1.Plane".into()],
                produces: Some("mox.hud.v1.PanelRender".into()),
                assertions: vec!["Renders file tree for current plane".into()],
                boundaries: vec!["Does not modify files".into()],
            },
            envelope: BehavioralEnvelope {
                resource_bounds: Some(ResourceBounds {
                    max_tokens: None,
                    max_latency_ms: Some(16), // 60fps
                    max_cost_usd: None,
                }),
                degradation: vec![],
            },
        };

        assert_eq!(manifest.kind, ComponentKind::Capability);
        assert_eq!(
            manifest.envelope.resource_bounds.as_ref().unwrap().max_latency_ms,
            Some(16)
        );
    }

    #[test]
    fn cross_surface_type_url_is_bridge() {
        // Authoring layer: manifest
        let manifest = ComponentManifest {
            kind: ComponentKind::Capability,
            type_url: "mox.geist.processors.v1.AccessControl".into(),
            description: "Access control".into(),
            contract: ComponentContract::default(),
            envelope: BehavioralEnvelope::default(),
        };

        // Runtime layer: typed config
        let config = crate::TypedConfig {
            type_url: "mox.geist.processors.v1.AccessControl".into(),
            config: serde_json::json!({"default_action": "deny"}),
        };

        // The type_url is the join key
        assert_eq!(manifest.type_url, config.type_url);
    }
}
