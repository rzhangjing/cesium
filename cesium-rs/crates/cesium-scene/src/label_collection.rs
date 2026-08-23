//! Ported from `packages/engine/Source/Scene/LabelCollection.js`.
//!
//! A collection of labels.

use crate::frame_state::FrameState;
use crate::label::Label;

/// A collection of labels for efficient rendering of many text labels.
///
/// Mirrors CesiumJS `LabelCollection` (984 lines).
pub struct LabelCollection {
    /// Whether this collection is shown.
    pub show: bool,
    /// The model matrix for this collection.
    pub model_matrix: cesium_core::matrix4::Matrix4,
    /// Whether to enable depth testing for labels.
    pub depth_test_enabled: bool,
    /// The labels in this collection.
    labels: Vec<Label>,
    /// Whether this collection has been destroyed.
    is_destroyed: bool,
}

impl LabelCollection {
    /// Creates a new LabelCollection.
    pub fn new() -> Self {
        Self {
            show: true,
            model_matrix: cesium_core::matrix4::Matrix4::IDENTITY,
            depth_test_enabled: true,
            labels: Vec::new(),
            is_destroyed: false,
        }
    }

    /// Adds a label to the collection.
    pub fn add(&mut self, label: Label) -> usize {
        let index = self.labels.len();
        self.labels.push(label);
        index
    }

    /// Removes a label from the collection by index.
    pub fn remove(&mut self, index: usize) -> bool {
        if index < self.labels.len() {
            self.labels.remove(index);
            true
        } else {
            false
        }
    }

    /// Removes all labels from the collection.
    pub fn remove_all(&mut self) {
        self.labels.clear();
    }

    /// Gets a label by index.
    pub fn get(&self, index: usize) -> Option<&Label> {
        self.labels.get(index)
    }

    /// Gets a mutable reference to a label by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Label> {
        self.labels.get_mut(index)
    }

    /// Returns the number of labels.
    pub fn len(&self) -> usize {
        self.labels.len()
    }

    /// Returns whether the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }

    /// Updates the collection for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        // DEVIATION: Requires glyph atlas and SDF rendering
    }

    /// Returns whether this collection has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this collection.
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}

impl Default for LabelCollection {
    fn default() -> Self { Self::new() }
}
