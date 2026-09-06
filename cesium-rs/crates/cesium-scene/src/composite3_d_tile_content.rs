//! Ported from `packages/engine/Source/Scene/Composite3DTileContent.js`.

/// Content for composite 3D tiles.
///
/// Manages multiple child contents within a single composite tile.
pub struct Composite3DTileContent {
    /// The number of child contents.
    pub inner_contents_length: u32,
    /// Whether all inner contents are ready.
    pub ready: bool,
}

impl Composite3DTileContent {
    /// Creates a new Composite3DTileContent.
    pub fn new() -> Self { Self { inner_contents_length: 0, ready: false } }

    /// Returns the number of inner contents.
    pub fn inner_contents_length(&self) -> u32 { self.inner_contents_length }

    /// Returns true if all contents are ready.
    pub fn is_ready(&self) -> bool { self.ready }
}

impl Default for Composite3DTileContent {
    fn default() -> Self { Self::new() }
}
