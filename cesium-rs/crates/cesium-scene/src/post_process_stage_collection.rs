//! Ported from `packages/engine/Source/Scene/PostProcessStageCollection.js`.
//!
//! A collection of post-process stages.

use crate::frame_state::FrameState;
use crate::post_process_stage::PostProcessStage;

/// A collection of post-process stages applied to the scene output.
///
/// Stages are applied in order. Each stage can reference the output of the
/// previous stage as its input texture.
/// Mirrors CesiumJS `PostProcessStageCollection` (583 lines).
pub struct PostProcessStageCollection {
    /// The stages in this collection.
    stages: Vec<PostProcessStage>,
    /// Whether this collection is ready.
    ready: bool,
}

impl PostProcessStageCollection {
    /// Creates a new PostProcessStageCollection.
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            ready: true,
        }
    }

    /// Adds a stage to the collection.
    pub fn add(&mut self, stage: PostProcessStage) -> usize {
        let index = self.stages.len();
        self.stages.push(stage);
        index
    }

    /// Removes a stage by index.
    pub fn remove(&mut self, index: usize) -> bool {
        if index < self.stages.len() {
            self.stages.remove(index);
            true
        } else {
            false
        }
    }

    /// Removes all stages.
    pub fn remove_all(&mut self) {
        self.stages.clear();
    }

    /// Gets a stage by index.
    pub fn get(&self, index: usize) -> Option<&PostProcessStage> {
        self.stages.get(index)
    }

    /// Returns the number of stages.
    pub fn len(&self) -> usize {
        self.stages.len()
    }

    /// Returns whether the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }

    /// Returns whether this collection is ready.
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Updates all stages for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        // DEVIATION: Requires sequential render pass execution
    }
}

impl Default for PostProcessStageCollection {
    fn default() -> Self { Self::new() }
}
