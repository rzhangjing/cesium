//! Ported from `packages/engine/Source/Scene/PostProcessStageComposite.js`.
//!
//! A composite of multiple post-process stages.

use crate::frame_state::FrameState;
use crate::post_process_stage::PostProcessStage;

/// A composite post-process stage that applies multiple stages in sequence.
///
/// Mirrors CesiumJS `PostProcessStageComposite` (345 lines).
pub struct PostProcessStageComposite {
    /// The unique name of this composite.
    pub name: String,
    /// The stages in this composite.
    stages: Vec<PostProcessStage>,
    /// Whether this composite is enabled.
    pub enabled: bool,
    /// Whether this composite is ready.
    ready: bool,
}

impl PostProcessStageComposite {
    /// Creates a new PostProcessStageComposite.
    pub fn new() -> Self {
        Self {
            name: String::new(),
            stages: Vec::new(),
            enabled: true,
            ready: false,
        }
    }

    /// Creates a composite from a list of stages.
    pub fn from_stages(name: &str, stages: Vec<PostProcessStage>) -> Self {
        Self {
            name: name.to_string(),
            stages,
            enabled: true,
            ready: true,
        }
    }

    /// Returns the number of stages.
    pub fn len(&self) -> usize {
        self.stages.len()
    }

    /// Returns whether the composite is empty.
    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }

    /// Gets a stage by index.
    pub fn get(&self, index: usize) -> Option<&PostProcessStage> {
        self.stages.get(index)
    }

    /// Returns whether this composite is ready.
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Updates all stages for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        // DEVIATION: Requires sequential render pass execution
    }
}

impl Default for PostProcessStageComposite {
    fn default() -> Self { Self::new() }
}
