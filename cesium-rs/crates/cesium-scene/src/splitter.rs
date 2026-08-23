//! Ported from `packages/engine/Source/Scene/Splitter.js`.

/// Controls the split-screen rendering mode.
pub struct Splitter {
    /// Whether the splitter is active.
    pub active: bool,
    /// The position of the split (0.0 to 1.0).
    pub position: f64,
}

impl Splitter {
    /// Creates a new splitter.
    pub fn new() -> Self {
        Self { active: false, position: 0.5 }
    }
}

impl Default for Splitter {
    fn default() -> Self { Self::new() }
}
