//! Ported from `packages/engine/Source/Scene/ITwinData.js`.

/// iTwin data.
pub struct ITwinData {
    _private: (),
}

impl ITwinData {
    /// Creates a new ITwinData.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ITwinData {
    fn default() -> Self { Self::new() }
}
