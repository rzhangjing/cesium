//! Ported from `packages/engine/Source/Scene/GltfDracoLoader.js`.
//!
//! Loads Draco-compressed glTF primitives.

use crate::draco_loader::DracoLoader;

/// Loads Draco-compressed glTF primitives.
///
/// Wraps [`DracoLoader`] to handle glTF-specific decompression,
/// including attribute decoding and index reconstruction.
/// Mirrors CesiumJS `GltfDracoLoader` (276 lines).
pub struct GltfDracoLoader {
    /// The underlying Draco decoder.
    draco_loader: DracoLoader,
    /// Whether this loader has been destroyed.
    is_destroyed: bool,
}

impl GltfDracoLoader {
    /// Creates a new GltfDracoLoader.
    pub fn new() -> Self {
        Self {
            draco_loader: DracoLoader::new(),
            is_destroyed: false,
        }
    }

    /// Sets the compressed data to decode.
    pub fn set_data(&mut self, data: Vec<u8>) {
        self.draco_loader.set_data(data);
    }

    /// Decodes the Draco-compressed glTF data.
    pub fn decode(&mut self) -> bool {
        self.draco_loader.decode()
    }

    /// Returns whether decoding is complete.
    pub fn is_complete(&self) -> bool {
        self.draco_loader.is_complete()
    }

    /// Returns the decoded positions.
    pub fn positions(&self) -> &[f32] {
        self.draco_loader.positions()
    }

    /// Returns the decoded normals.
    pub fn normals(&self) -> &[f32] {
        self.draco_loader.normals()
    }

    /// Returns the decoded indices.
    pub fn indices(&self) -> &[u32] {
        self.draco_loader.indices()
    }

    /// Returns whether this loader has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this loader and releases resources.
    pub fn destroy(&mut self) {
        self.draco_loader.release();
        self.is_destroyed = true;
    }
}

impl Default for GltfDracoLoader {
    fn default() -> Self { Self::new() }
}
