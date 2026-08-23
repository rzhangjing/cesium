//! Ported from `packages/engine/Source/Scene/ImplicitMetadataView.js`.

/// Implicit metadata view.
pub struct ImplicitMetadataView {
    _private: (),
}

impl ImplicitMetadataView {
    /// Creates a new ImplicitMetadataView.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ImplicitMetadataView {
    fn default() -> Self { Self::new() }
}
