//! Ported from `packages/engine/Source/Scene/createBillboardPointCallback.js`.

/// Creates a billboard point callback.
pub struct CreateBillboardPointCallback {
    _private: (),
}

impl CreateBillboardPointCallback {
    /// Creates a new CreateBillboardPointCallback.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CreateBillboardPointCallback {
    fn default() -> Self { Self::new() }
}
