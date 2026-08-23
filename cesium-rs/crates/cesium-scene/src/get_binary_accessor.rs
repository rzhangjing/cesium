//! Ported from `packages/engine/Source/Scene/getBinaryAccessor.js`.

/// Gets a binary accessor.
pub struct GetBinaryAccessor {
    _private: (),
}

impl GetBinaryAccessor {
    /// Creates a new GetBinaryAccessor.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GetBinaryAccessor {
    fn default() -> Self { Self::new() }
}
