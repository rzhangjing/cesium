//! Ported from `packages/engine/Source/Scene/Model/VaryingType.js`.

/// The type of a shader varying.
pub struct VaryingType {
    _private: (),
}

impl VaryingType {
    /// Creates a new VaryingType.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for VaryingType {
    fn default() -> Self { Self::new() }
}
