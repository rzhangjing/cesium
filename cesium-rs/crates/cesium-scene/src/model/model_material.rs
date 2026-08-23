//! Ported from `packages/engine/Source/Scene/Model/ModelMaterial.js`.
//!
//! A material within a model.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;

/// A material within a [`Model`](super::model::Model).
///
/// Wraps a glTF material with PBR metallic-roughness properties.
/// Mirrors CesiumJS `ModelMaterial` (321 lines).
pub struct ModelMaterial {
    /// The name of this material.
    pub name: String,
    /// The material type name (e.g., "pbrMetallicRoughness").
    pub type_name: String,
    /// The base color factor.
    pub base_color: Color,
    /// The metallic factor (0.0 = dielectric, 1.0 = metal).
    pub metallic_factor: f64,
    /// The roughness factor (0.0 = smooth, 1.0 = rough).
    pub roughness_factor: f64,
    /// The emissive factor (RGB).
    pub emissive_factor: Cartesian3,
    /// The normal texture scale.
    pub normal_scale: f64,
    /// The occlusion texture strength.
    pub occlusion_strength: f64,
    /// Whether the material is double-sided.
    pub double_sided: bool,
    /// The alpha mode ("OPAQUE", "MASK", "BLEND").
    pub alpha_mode: String,
    /// The alpha cutoff value (for MASK mode).
    pub alpha_cutoff: f64,
}

impl ModelMaterial {
    /// Creates a new ModelMaterial with default PBR values.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            type_name: "pbrMetallicRoughness".to_string(),
            base_color: Color::new(1.0, 1.0, 1.0, 1.0),
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            emissive_factor: Cartesian3::ZERO,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            double_sided: false,
            alpha_mode: "OPAQUE".to_string(),
            alpha_cutoff: 0.5,
        }
    }
}
