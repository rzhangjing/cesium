//! Ported from `packages/engine/Source/Scene/Model/extensions/gpm/`.

/// SPDCF extension data.
pub struct Spdcf {
    _private: (),
}

impl Spdcf {
    /// Creates a new Spdcf.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Spdcf {
    fn default() -> Self { Self::new() }
}
