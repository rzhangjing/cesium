//! Ported from `packages/engine/Source/Core/buildModuleUrl.js`.

/// Builds a URL for a module.
pub struct BuildModuleUrl {
    _private: (),
}

impl BuildModuleUrl {
    /// Creates a new BuildModuleUrl.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BuildModuleUrl {
    fn default() -> Self { Self::new() }
}
