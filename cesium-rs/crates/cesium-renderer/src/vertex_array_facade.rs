//! Ported from `packages/engine/Source/Renderer/VertexArrayFacade.js`.
//!
//! A facade over multiple vertex arrays for handling large geometry.
//! When geometry exceeds the maximum index value (e.g., 65535 for
//! UNSIGNED_SHORT), it is split across multiple vertex arrays.

use crate::vertex_array::VertexArray;

/// A facade over multiple vertex arrays for handling large geometry.
///
/// Mirrors the CesiumJS `VertexArrayFacade` which manages multiple vertex
/// arrays when geometry exceeds the maximum index buffer size.
pub struct VertexArrayFacade {
    /// The vertex arrays in this facade.
    vas: Vec<VertexArray>,
    /// The current index into the vas array (for round-robin rendering).
    current_index: usize,
    is_destroyed: bool,
}

impl VertexArrayFacade {
    /// Creates a new vertex array facade.
    pub fn new() -> Self {
        Self {
            vas: Vec::new(),
            current_index: 0,
            is_destroyed: false,
        }
    }

    /// Creates a vertex array facade with the given vertex arrays.
    pub fn with_arrays(vas: Vec<VertexArray>) -> Self {
        Self {
            vas,
            current_index: 0,
            is_destroyed: false,
        }
    }

    /// Returns the number of vertex arrays in this facade.
    pub fn count(&self) -> usize {
        self.vas.len()
    }

    /// Returns the vertex arrays.
    pub fn vertex_arrays(&self) -> &[VertexArray] {
        &self.vas
    }

    /// Returns a mutable reference to the vertex arrays.
    pub fn vertex_arrays_mut(&mut self) -> &mut Vec<VertexArray> {
        &mut self.vas
    }

    /// Returns the current vertex array (for round-robin rendering).
    pub fn current(&self) -> Option<&VertexArray> {
        self.vas.get(self.current_index)
    }

    /// Advances to the next vertex array (wraps around).
    pub fn next(&mut self) {
        if !self.vas.is_empty() {
            self.current_index = (self.current_index + 1) % self.vas.len();
        }
    }

    /// Resets to the first vertex array.
    pub fn reset(&mut self) {
        self.current_index = 0;
    }

    /// Returns whether this facade has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys all vertex arrays in this facade.
    pub fn destroy(&mut self) {
        for va in &mut self.vas {
            va.destroy();
        }
        self.vas.clear();
        self.is_destroyed = true;
    }
}

impl Default for VertexArrayFacade {
    fn default() -> Self { Self::new() }
}
