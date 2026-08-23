//! Ported from `packages/engine/Source/Core/PinBuilder.js`.

/// Creates pin markers for billboards.
pub struct PinBuilder {
    _private: (),
}

impl PinBuilder {
    /// Creates a new PinBuilder.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PinBuilder {
    fn default() -> Self { Self::new() }
}
