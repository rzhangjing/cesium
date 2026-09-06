//! Ported from `packages/engine/Source/Scene/GetClippingFunction.js`.

/// Gets the clipping function.
///
/// Returns the appropriate clipping shader function.
pub struct GetClippingFunction {
    /// Whether the function is resolved.
    pub resolved: bool,
}

impl GetClippingFunction {
    /// Creates a new GetClippingFunction.
    pub fn new() -> Self { Self { resolved: false } }
}

impl Default for GetClippingFunction {
    fn default() -> Self { Self::new() }
}
