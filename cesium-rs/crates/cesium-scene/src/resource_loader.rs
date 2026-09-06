//! Ported from `packages/engine/Source/Scene/ResourceLoader.js`.

/// Resource loader.
///
/// Loads and manages external resources (textures, models, etc.).
pub struct ResourceLoader {
    /// Whether the loader is active.
    pub active: bool,
    /// The number of pending requests.
    pub pending_count: u32,
}

impl ResourceLoader {
    /// Creates a new ResourceLoader.
    pub fn new() -> Self { Self { active: false, pending_count: 0 } }
}

impl Default for ResourceLoader {
    fn default() -> Self { Self::new() }
}
