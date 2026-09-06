//! Ported from `packages/engine/Source/Scene/FrustumCommands.js`.

use cesium_renderer::draw_command::DrawCommand;
use cesium_renderer::pass::Pass;

/// Defines a list of commands whose geometry are bound by near and far
/// distances from the camera.
pub struct FrustumCommands {
    /// The lower bound or closest distance from the camera.
    pub near: f64,
    /// The upper bound or farthest distance from the camera.
    pub far: f64,
    /// Per-pass command lists.
    pub commands: Vec<Vec<DrawCommand>>,
    /// Per-pass index counters.
    pub indices: Vec<usize>,
}

impl FrustumCommands {
    /// Creates a new `FrustumCommands`.
    pub fn new(near: Option<f64>, far: Option<f64>) -> Self {
        let n = Pass::NumberOfPasses as usize;
        Self {
            near: near.unwrap_or(0.0),
            far: far.unwrap_or(0.0),
            commands: (0..n).map(|_| Vec::new()).collect(),
            indices: vec![0; n],
        }
    }
}

impl Default for FrustumCommands {
    fn default() -> Self {
        Self::new(None, None)
    }
}
