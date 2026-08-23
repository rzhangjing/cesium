//! Ported from `packages/engine/Source/Scene/getClippingFunction.js`.

/// Gets a clipping function.
pub struct GetClippingFunction {
    _private: (),
}

impl GetClippingFunction {
    /// Creates a new GetClippingFunction.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GetClippingFunction {
    fn default() -> Self { Self::new() }
}
