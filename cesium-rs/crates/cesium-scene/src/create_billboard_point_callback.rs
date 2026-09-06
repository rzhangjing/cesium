//! Ported from `packages/engine/Source/Scene/CreateBillboardPointCallback.js`.

/// Callback for creating billboard points.
///
/// Creates billboard point instances during primitive rendering.
pub struct CreateBillboardPointCallback {
    /// Whether the callback is active.
    pub active: bool,
}

impl CreateBillboardPointCallback {
    /// Creates a new CreateBillboardPointCallback.
    pub fn new() -> Self { Self { active: false } }
}

impl Default for CreateBillboardPointCallback {
    fn default() -> Self { Self::new() }
}
