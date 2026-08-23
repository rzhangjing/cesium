//! Ported from `packages/engine/Source/Scene/hasExtension.js`.

/// Checks for extension.
pub struct HasExtension {
    _private: (),
}

impl HasExtension {
    /// Creates a new HasExtension.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for HasExtension {
    fn default() -> Self { Self::new() }
}
