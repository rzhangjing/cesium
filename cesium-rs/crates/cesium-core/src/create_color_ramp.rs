//! Ported from `packages/engine/Source/Core/createColorRamp.js`.

/// Creates a color ramp texture.
pub struct CreateColorRamp {
    _private: (),
}

impl CreateColorRamp {
    /// Creates a new CreateColorRamp.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CreateColorRamp {
    fn default() -> Self { Self::new() }
}
