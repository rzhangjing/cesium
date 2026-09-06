//! Ported from `packages/engine/Source/Scene/Vector3DTileBatch.js`.

/// A batch of features within a vector 3D tile.
///
/// Groups features by rendering properties for efficient draw calls.
pub struct Vector3DTileBatch {
    /// The number of features in this batch.
    pub features_length: u32,
    /// The batch indices.
    pub batch_ids: Vec<u32>,
}

impl Vector3DTileBatch {
    /// Creates a new Vector3DTileBatch.
    pub fn new() -> Self { Self { features_length: 0, batch_ids: Vec::new() } }
}

impl Default for Vector3DTileBatch {
    fn default() -> Self { Self::new() }
}
