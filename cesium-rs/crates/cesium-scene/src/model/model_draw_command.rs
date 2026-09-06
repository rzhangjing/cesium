//! Ported from `packages/engine/Source/Scene/Model/ModelDrawCommand.js`.
//!
//! Wraps a draw command with per-model state: shadow mode, back-face culling,
//! derived commands for silhouette/edge/LOD, and 2D mode support.

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::matrix4::Matrix4;

use crate::cull_face::CullFace;

/// A derived draw command variant (e.g. translucent, silhouette, edge).
#[derive(Debug, Clone)]
pub struct ModelDerivedCommand {
    /// Whether shadow mode should be updated for this command.
    pub update_shadows: bool,
    /// Whether back-face culling should be updated.
    pub update_back_face_culling: bool,
    /// Whether cull face should be updated.
    pub update_cull_face: bool,
    /// Whether debug bounding volume should be updated.
    pub update_debug_show_bounding_volume: bool,
    /// Whether this is a 2D mode command.
    pub is_2d: bool,
    /// The corresponding 2D derived command, if generated.
    pub derived_command_2d: Option<Box<ModelDerivedCommand>>,
}

impl ModelDerivedCommand {
    /// Creates a new 3D derived command.
    pub fn new() -> Self {
        Self {
            update_shadows: true,
            update_back_face_culling: true,
            update_cull_face: true,
            update_debug_show_bounding_volume: true,
            is_2d: false,
            derived_command_2d: None,
        }
    }
}

impl Default for ModelDerivedCommand {
    fn default() -> Self { Self::new() }
}

/// A draw command for a model.
///
/// Wraps the base draw command with model-specific state: shadow mode,
/// back-face culling, derived commands for silhouette/edge/LOD, and
/// optional 2D-mode commands.
/// Mirrors CesiumJS `ModelDrawCommand` (~450 lines).
pub struct ModelDrawCommand {
    /// The model matrix for 3D mode.
    pub model_matrix: Matrix4,
    /// The bounding sphere in world coordinates.
    pub bounding_volume: BoundingSphere,
    /// The model matrix for 2D mode.
    pub model_matrix_2d: Matrix4,
    /// The bounding sphere in 2D coordinates.
    pub bounding_volume_2d: BoundingSphere,
    /// Whether the 2D model matrix needs recomputation.
    pub model_matrix_2d_dirty: bool,
    /// Whether back-face culling is enabled.
    pub back_face_culling: bool,
    /// Which face to cull.
    pub cull_face: CullFace,
    /// Whether to show the debug bounding volume.
    pub debug_show_bounding_volume: bool,
    /// Whether this command uses back-face culling.
    pub uses_back_face_culling: bool,
    /// Whether a translucent derived command is needed.
    pub needs_translucent_command: bool,
    /// Whether silhouette derived commands are needed.
    pub needs_silhouette_commands: bool,
    /// Whether edge derived commands are needed.
    pub needs_edge_commands: bool,
    /// The original (opaque) derived command.
    pub original_command: Option<ModelDerivedCommand>,
    /// The translucent derived command.
    pub translucent_command: Option<ModelDerivedCommand>,
    /// All derived commands for LOD/skip-level rendering.
    pub derived_commands: Vec<ModelDerivedCommand>,
    /// Whether 2D commands have been generated.
    pub has_2d_commands: bool,
}

impl ModelDrawCommand {
    /// Creates a new `ModelDrawCommand` with default state.
    pub fn new() -> Self {
        Self {
            model_matrix: Matrix4::IDENTITY,
            bounding_volume: BoundingSphere::default(),
            model_matrix_2d: Matrix4::IDENTITY,
            bounding_volume_2d: BoundingSphere::default(),
            model_matrix_2d_dirty: true,
            back_face_culling: true,
            cull_face: CullFace::Back,
            debug_show_bounding_volume: false,
            uses_back_face_culling: true,
            needs_translucent_command: false,
            needs_silhouette_commands: false,
            needs_edge_commands: false,
            original_command: Some(ModelDerivedCommand::new()),
            translucent_command: None,
            derived_commands: Vec::new(),
            has_2d_commands: false,
        }
    }

    /// Returns the total number of derived commands (including original/translucent).
    pub fn derived_command_count(&self) -> usize {
        let mut count = self.derived_commands.len();
        if self.original_command.is_some() {
            count += 1;
        }
        if self.translucent_command.is_some() {
            count += 1;
        }
        count
    }

    /// Marks the 2D model matrix as dirty, forcing recomputation.
    pub fn mark_2d_dirty(&mut self) {
        self.model_matrix_2d_dirty = true;
        self.has_2d_commands = false;
    }
}

impl Default for ModelDrawCommand {
    fn default() -> Self { Self::new() }
}
