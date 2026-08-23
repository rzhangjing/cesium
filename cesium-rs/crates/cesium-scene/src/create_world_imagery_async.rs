//! Ported from `packages/engine/Source/Scene/createWorldImageryAsync.js`.

/// Creates world imagery asynchronously.
pub struct CreateWorldImageryAsync {
    _private: (),
}

impl CreateWorldImageryAsync {
    /// Creates a new CreateWorldImageryAsync.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CreateWorldImageryAsync {
    fn default() -> Self { Self::new() }
}
