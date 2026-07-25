//! Draw command generation and render pass management.
//!
//! Maps to CesiumJS `Renderer/DrawCommand.js` and `Scene/Pass.js`

use glam::DMat4;
use serde::{Deserialize, Serialize};

/// Render pass types.
///
/// Maps to CesiumJS `Scene/Pass.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub enum RenderPass {
    /// Environment pass (sky, atmosphere).
    Environment = 0,
    /// 3D Tiles and terrain.
    Cesium3DTile = 1,
    /// Opaque primitives.
    #[default]
    Opaque = 2,
    /// Translucent primitives.
    Translucent = 3,
    /// Overlay pass (labels, polylines).
    Overlay = 4,
}

/// Blend state for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BlendState {
    /// No blending (opaque).
    #[default]
    Opaque,
    /// Alpha blending.
    AlphaBlend,
    /// Additive blending.
    Additive,
    /// Premultiplied alpha.
    PremultipliedAlpha,
}

/// Depth test state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepthState {
    /// Whether depth testing is enabled.
    pub enabled: bool,
    /// Whether depth writing is enabled.
    pub write_enabled: bool,
}

impl Default for DepthState {
    fn default() -> Self {
        Self {
            enabled: true,
            write_enabled: true,
        }
    }
}

/// A draw command representing a single render operation.
///
/// Maps to CesiumJS `Renderer/DrawCommand.js`
#[derive(Debug, Clone)]
pub struct DrawCommand {
    /// The render pass this command belongs to.
    pub pass: RenderPass,

    /// Model matrix (local to world).
    pub model_matrix: DMat4,

    /// Mesh/geometry asset ID.
    pub geometry_id: u64,

    /// Material/shader program ID.
    pub material_id: u64,

    /// Optional texture IDs.
    pub texture_ids: Vec<u64>,

    /// Blend state.
    pub blend_state: BlendState,

    /// Depth state.
    pub depth_state: DepthState,

    /// Whether to cull back faces.
    pub cull_face: bool,

    /// Sort key for ordering (distance or custom).
    pub sort_key: f64,

    /// Instance count (for instanced rendering).
    pub instance_count: u32,

    /// Whether this command casts shadows.
    pub casts_shadows: bool,

    /// Whether this command receives shadows.
    pub receives_shadows: bool,

    /// Picking ID for object selection.
    pub pick_id: Option<u64>,
}

impl Default for DrawCommand {
    fn default() -> Self {
        Self {
            pass: RenderPass::Opaque,
            model_matrix: DMat4::IDENTITY,
            geometry_id: 0,
            material_id: 0,
            texture_ids: Vec::new(),
            blend_state: BlendState::Opaque,
            depth_state: DepthState::default(),
            cull_face: true,
            sort_key: 0.0,
            instance_count: 1,
            casts_shadows: true,
            receives_shadows: true,
            pick_id: None,
        }
    }
}

impl DrawCommand {
    /// Creates a new draw command with the given geometry and material.
    pub fn new(geometry_id: u64, material_id: u64) -> Self {
        Self {
            geometry_id,
            material_id,
            ..Default::default()
        }
    }

    /// Sets the model matrix.
    pub fn with_model_matrix(mut self, matrix: DMat4) -> Self {
        self.model_matrix = matrix;
        self
    }

    /// Sets the render pass.
    pub fn with_pass(mut self, pass: RenderPass) -> Self {
        self.pass = pass;
        self
    }

    /// Sets the blend state.
    pub fn with_blend_state(mut self, blend: BlendState) -> Self {
        self.blend_state = blend;
        self
    }

    /// Sets the sort key.
    pub fn with_sort_key(mut self, key: f64) -> Self {
        self.sort_key = key;
        self
    }

    /// Sets the pick ID.
    pub fn with_pick_id(mut self, id: u64) -> Self {
        self.pick_id = Some(id);
        self
    }

    /// Returns true if this is a transparent command.
    pub fn is_transparent(&self) -> bool {
        !matches!(self.blend_state, BlendState::Opaque)
    }
}

/// A collection of draw commands organized by render pass.
#[derive(Debug, Default)]
pub struct RenderCommandList {
    /// Commands organized by pass.
    passes: std::collections::BTreeMap<RenderPass, Vec<DrawCommand>>,
}

impl RenderCommandList {
    /// Creates a new empty command list.
    pub fn new() -> Self {
        Self {
            passes: std::collections::BTreeMap::new(),
        }
    }

    /// Adds a command to the list.
    pub fn push(&mut self, command: DrawCommand) {
        self.passes.entry(command.pass).or_default().push(command);
    }

    /// Returns the total number of commands.
    pub fn len(&self) -> usize {
        self.passes.values().map(|v| v.len()).sum()
    }

    /// Returns true if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.passes.values().all(|v| v.is_empty())
    }

    /// Returns commands for a specific pass.
    pub fn commands_for_pass(&self, pass: RenderPass) -> &[DrawCommand] {
        self.passes.get(&pass).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Sorts commands within each pass.
    ///
    /// Opaque passes are sorted front-to-back.
    /// Transparent passes are sorted back-to-front.
    pub fn sort(&mut self) {
        for (pass, commands) in self.passes.iter_mut() {
            match pass {
                RenderPass::Translucent | RenderPass::Overlay => {
                    // Back-to-front for transparency
                    commands.sort_by(|a, b| {
                        b.sort_key.partial_cmp(&a.sort_key).unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
                _ => {
                    // Front-to-back for opaque (early-z optimization)
                    commands.sort_by(|a, b| {
                        a.sort_key.partial_cmp(&b.sort_key).unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
            }
        }
    }

    /// Returns an iterator over all passes and their commands.
    pub fn iter(&self) -> impl Iterator<Item = (&RenderPass, &Vec<DrawCommand>)> {
        self.passes.iter()
    }

    /// Clears all commands.
    pub fn clear(&mut self) {
        self.passes.clear();
    }
}

/// Frame statistics for rendering.
#[derive(Debug, Clone, Default)]
pub struct FrameStatistics {
    /// Number of draw commands executed.
    pub draw_calls: usize,

    /// Number of triangles rendered.
    pub triangles: u64,

    /// Number of vertices processed.
    pub vertices: u64,

    /// Number of texture binds.
    pub texture_binds: usize,

    /// Number of shader switches.
    pub shader_switches: usize,

    /// Number of culled objects.
    pub culled_objects: usize,

    /// Frame time in milliseconds.
    pub frame_time_ms: f64,
}

impl FrameStatistics {
    /// Resets all statistics.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Merges another statistics object into this one.
    pub fn merge(&mut self, other: &FrameStatistics) {
        self.draw_calls += other.draw_calls;
        self.triangles += other.triangles;
        self.vertices += other.vertices;
        self.texture_binds += other.texture_binds;
        self.shader_switches += other.shader_switches;
        self.culled_objects += other.culled_objects;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_command_default() {
        let cmd = DrawCommand::default();
        assert_eq!(cmd.pass, RenderPass::Opaque);
        assert_eq!(cmd.blend_state, BlendState::Opaque);
        assert!(cmd.depth_state.enabled);
        assert!(cmd.cull_face);
        assert!(!cmd.is_transparent());
    }

    #[test]
    fn test_draw_command_builder() {
        let cmd = DrawCommand::new(1, 2)
            .with_pass(RenderPass::Translucent)
            .with_blend_state(BlendState::AlphaBlend)
            .with_sort_key(100.0)
            .with_pick_id(42);

        assert_eq!(cmd.geometry_id, 1);
        assert_eq!(cmd.material_id, 2);
        assert_eq!(cmd.pass, RenderPass::Translucent);
        assert_eq!(cmd.blend_state, BlendState::AlphaBlend);
        assert_eq!(cmd.sort_key, 100.0);
        assert_eq!(cmd.pick_id, Some(42));
        assert!(cmd.is_transparent());
    }

    #[test]
    fn test_render_command_list() {
        let mut list = RenderCommandList::new();

        list.push(DrawCommand::new(1, 1).with_pass(RenderPass::Opaque));
        list.push(DrawCommand::new(2, 2).with_pass(RenderPass::Translucent));
        list.push(DrawCommand::new(3, 3).with_pass(RenderPass::Opaque));

        assert_eq!(list.len(), 3);
        assert_eq!(list.commands_for_pass(RenderPass::Opaque).len(), 2);
        assert_eq!(list.commands_for_pass(RenderPass::Translucent).len(), 1);
    }

    #[test]
    fn test_render_command_list_sort() {
        let mut list = RenderCommandList::new();

        // Add opaque commands with different sort keys
        list.push(DrawCommand::new(1, 1).with_pass(RenderPass::Opaque).with_sort_key(100.0));
        list.push(DrawCommand::new(2, 2).with_pass(RenderPass::Opaque).with_sort_key(50.0));
        list.push(DrawCommand::new(3, 3).with_pass(RenderPass::Opaque).with_sort_key(200.0));

        list.sort();

        let opaque = list.commands_for_pass(RenderPass::Opaque);
        // Front-to-back: 50, 100, 200
        assert_eq!(opaque[0].geometry_id, 2);
        assert_eq!(opaque[1].geometry_id, 1);
        assert_eq!(opaque[2].geometry_id, 3);
    }

    #[test]
    fn test_translucent_sort_back_to_front() {
        let mut list = RenderCommandList::new();

        list.push(DrawCommand::new(1, 1).with_pass(RenderPass::Translucent).with_sort_key(100.0));
        list.push(DrawCommand::new(2, 2).with_pass(RenderPass::Translucent).with_sort_key(50.0));

        list.sort();

        let translucent = list.commands_for_pass(RenderPass::Translucent);
        // Back-to-front: 100, 50
        assert_eq!(translucent[0].geometry_id, 1);
        assert_eq!(translucent[1].geometry_id, 2);
    }

    #[test]
    fn test_render_pass_ordering() {
        assert!(RenderPass::Environment < RenderPass::Cesium3DTile);
        assert!(RenderPass::Cesium3DTile < RenderPass::Opaque);
        assert!(RenderPass::Opaque < RenderPass::Translucent);
        assert!(RenderPass::Translucent < RenderPass::Overlay);
    }

    #[test]
    fn test_frame_statistics() {
        let mut stats = FrameStatistics {
            draw_calls: 100,
            triangles: 50000,
            ..Default::default()
        };

        let other = FrameStatistics {
            draw_calls: 50,
            triangles: 25000,
            ..Default::default()
        };

        stats.merge(&other);

        assert_eq!(stats.draw_calls, 150);
        assert_eq!(stats.triangles, 75000);
    }

    #[test]
    fn test_command_list_clear() {
        let mut list = RenderCommandList::new();
        list.push(DrawCommand::new(1, 1));
        list.push(DrawCommand::new(2, 2));

        assert!(!list.is_empty());

        list.clear();

        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
    }
}
