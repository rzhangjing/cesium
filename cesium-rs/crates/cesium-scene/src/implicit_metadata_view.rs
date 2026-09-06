//! Ported from `packages/engine/Source/Scene/ImplicitMetadataView.js`.

/// Implicit metadata view.
///
/// Provides a view into implicit tile metadata.
pub struct ImplicitMetadataView {
    /// The number of properties.
    pub property_count: u32,
}

impl ImplicitMetadataView {
    /// Creates a new ImplicitMetadataView.
    pub fn new() -> Self { Self { property_count: 0 } }
}

impl Default for ImplicitMetadataView {
    fn default() -> Self { Self::new() }
}
