//! Ported from `packages/engine/Source/Scene/PostProcessStageSampleMode.js`.

/// The sampling mode for post-process stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PostProcessStageSampleMode {
    /// Nearest neighbor sampling.
    Nearest = 0,
    /// Linear filtering.
    Linear = 1,
}
