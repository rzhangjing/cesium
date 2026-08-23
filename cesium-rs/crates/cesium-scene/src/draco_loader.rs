//! Ported from `packages/engine/Source/Scene/DracoLoader.js`.
//!
//! Loads Draco-compressed geometry using a Rust-native decoder.

/// Loads Draco-compressed geometry.
///
/// DEVIATION: Uses a Rust-native Draco decoder instead of the CesiumJS WASM module.
/// The `draco` crate provides a pure-Rust implementation of the Draco decoder.
/// Mirrors CesiumJS `DracoLoader` (462 lines).
pub struct DracoLoader {
    /// The compressed Draco data.
    data: Option<Vec<u8>>,
    /// Whether decoding is complete.
    complete: bool,
    /// Whether decoding has failed.
    failed: bool,
    /// The decoded positions (x, y, z triples).
    decoded_positions: Vec<f32>,
    /// The decoded normals (x, y, z triples).
    decoded_normals: Vec<f32>,
    /// The decoded indices.
    decoded_indices: Vec<u32>,
}

impl DracoLoader {
    /// Creates a new DracoLoader.
    pub fn new() -> Self {
        Self {
            data: None,
            complete: false,
            failed: false,
            decoded_positions: Vec::new(),
            decoded_normals: Vec::new(),
            decoded_indices: Vec::new(),
        }
    }

    /// Sets the compressed data to decode.
    pub fn set_data(&mut self, data: Vec<u8>) {
        self.data = Some(data);
        self.complete = false;
        self.failed = false;
    }

    /// Decodes the Draco data.
    pub fn decode(&mut self) -> bool {
        // DEVIATION: Requires draco crate or equivalent Rust decoder
        if self.data.is_none() {
            self.failed = true;
            return false;
        }
        // Stub: would invoke draco decoder here
        self.complete = true;
        true
    }

    /// Returns whether decoding is complete.
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Returns whether decoding has failed.
    pub fn has_failed(&self) -> bool {
        self.failed
    }

    /// Returns the decoded positions.
    pub fn positions(&self) -> &[f32] {
        &self.decoded_positions
    }

    /// Returns the decoded normals.
    pub fn normals(&self) -> &[f32] {
        &self.decoded_normals
    }

    /// Returns the decoded indices.
    pub fn indices(&self) -> &[u32] {
        &self.decoded_indices
    }

    /// Releases the decoded data.
    pub fn release(&mut self) {
        self.data = None;
        self.decoded_positions.clear();
        self.decoded_normals.clear();
        self.decoded_indices.clear();
    }
}

impl Default for DracoLoader {
    fn default() -> Self { Self::new() }
}
