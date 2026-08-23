//! Ported from `packages/engine/Source/Scene/getClipAndStyleCode.js`.

/// Gets clip and style code.
pub struct GetClipAndStyleCode {
    _private: (),
}

impl GetClipAndStyleCode {
    /// Creates a new GetClipAndStyleCode.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GetClipAndStyleCode {
    fn default() -> Self { Self::new() }
}
