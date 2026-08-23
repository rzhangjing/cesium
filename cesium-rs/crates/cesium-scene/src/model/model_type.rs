//! Ported from `packages/engine/Source/Scene/ModelType.js`.

/// The type of a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
    /// A 3D Tiles model.
    Tiles3D,
    /// A glTF model.
    Gltf,
}
