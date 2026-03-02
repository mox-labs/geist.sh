//! slick-ts — TypeScript bindings for slickit via wasm-bindgen.
//!
//! Exposes SLICK's component type system to TypeScript/Svelte: ComponentKind,
//! ComponentManifest, ComponentContract, BehavioralEnvelope, TypedConfig.
//!
//! Built with `wasm-pack build --target web`. Produces `.d.ts` type definitions
//! alongside the WASM binary.

use wasm_bindgen::prelude::*;

// ═══════════════════════════════════════════════════════════════════════
// ComponentKind
// ═══════════════════════════════════════════════════════════════════════

/// The 4 core component kinds.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    Agent = 0,
    Capability = 1,
    Skill = 2,
    Flow = 3,
}

impl From<ComponentKind> for slick::ComponentKind {
    fn from(kind: ComponentKind) -> Self {
        match kind {
            ComponentKind::Agent => Self::Agent,
            ComponentKind::Capability => Self::Capability,
            ComponentKind::Skill => Self::Skill,
            ComponentKind::Flow => Self::Flow,
        }
    }
}

impl From<slick::ComponentKind> for ComponentKind {
    fn from(kind: slick::ComponentKind) -> Self {
        match kind {
            slick::ComponentKind::Agent => Self::Agent,
            slick::ComponentKind::Capability => Self::Capability,
            slick::ComponentKind::Skill => Self::Skill,
            slick::ComponentKind::Flow => Self::Flow,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ComponentManifest (opaque, JSON bridge)
// ═══════════════════════════════════════════════════════════════════════

/// Authoring-layer component manifest.
///
/// Constructed from and serialized to JS objects via serde-wasm-bindgen.
#[wasm_bindgen]
pub struct ComponentManifest {
    inner: slick::ComponentManifest,
}

#[wasm_bindgen]
impl ComponentManifest {
    /// Create a ComponentManifest from a JS object.
    ///
    /// Expected shape:
    /// ```js
    /// {
    ///   kind: "agent" | "capability" | "skill" | "flow",
    ///   type_url: "mox.geist.processors.v1.AccessControl",
    ///   description: "...",
    ///   contract: { consumes: [], produces: "...", assertions: [], boundaries: [] },
    ///   envelope: { resource_bounds: { max_latency_ms: 16 }, degradation: [] }
    /// }
    /// ```
    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(obj: JsValue) -> Result<ComponentManifest, JsValue> {
        let inner: slick::ComponentManifest = serde_wasm_bindgen::from_value(obj)
            .map_err(|e| JsValue::from_str(&format!("invalid manifest: {e}")))?;
        Ok(Self { inner })
    }

    /// Serialize to a JS object.
    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("serialization failed: {e}")))
    }

    /// Create from a JSON string.
    #[wasm_bindgen(js_name = "fromJson")]
    pub fn from_json(json: &str) -> Result<ComponentManifest, JsValue> {
        let inner: slick::ComponentManifest = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("invalid JSON: {e}")))?;
        Ok(Self { inner })
    }

    /// Serialize to a JSON string.
    #[wasm_bindgen(js_name = "toJson")]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("serialization failed: {e}")))
    }

    /// Get the component kind.
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> ComponentKind {
        self.inner.kind.into()
    }

    /// Get the type URL.
    #[wasm_bindgen(getter, js_name = "typeUrl")]
    pub fn type_url(&self) -> String {
        self.inner.type_url.clone()
    }

    /// Get the description.
    #[wasm_bindgen(getter)]
    pub fn description(&self) -> String {
        self.inner.description.clone()
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TypedConfig
// ═══════════════════════════════════════════════════════════════════════

/// Config envelope: type URL + opaque config.
#[wasm_bindgen]
pub struct TypedConfig {
    inner: slick::TypedConfig,
}

#[wasm_bindgen]
impl TypedConfig {
    /// Create from a JS object { type_url: string, config: any }.
    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(obj: JsValue) -> Result<TypedConfig, JsValue> {
        let inner: slick::TypedConfig = serde_wasm_bindgen::from_value(obj)
            .map_err(|e| JsValue::from_str(&format!("invalid config: {e}")))?;
        Ok(Self { inner })
    }

    /// Serialize to a JS object.
    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("serialization failed: {e}")))
    }

    /// Get the type URL.
    #[wasm_bindgen(getter, js_name = "typeUrl")]
    pub fn type_url(&self) -> String {
        self.inner.type_url.clone()
    }
}
