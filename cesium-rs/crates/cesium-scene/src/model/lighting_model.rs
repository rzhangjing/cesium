//! Ported from `packages/engine/Source/Scene/LightingModel.js`.

/// The lighting model used by a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightingModel {
    /// Unlit (no lighting calculations).
    Unlit,
    /// Physically-based rendering (metallic-roughness).
    PbrMetallicRoughness,
    /// Physically-based rendering (specular-glossiness).
    PbrSpecularGlossiness,
}
