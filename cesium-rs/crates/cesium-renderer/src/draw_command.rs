//! Ported from `packages/engine/Source/Renderer/DrawCommand.js`.
//!
//! Represents a command to the renderer for drawing. In CesiumJS, this is a
//! lightweight command object that holds references to shader programs, vertex arrays,
//! render state, and uniforms. The actual execution is done by the Context.

use std::any::Any;
use cesium_core::bounding_rectangle::BoundingRectangle;
use crate::render_state::RenderState;
use crate::shader_program::ShaderProgram;

/// Flags for draw command behavior (mirrors CesiumJS Flags enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawCommandFlags(u32);

impl DrawCommandFlags {
    pub const CULL: u32 = 1;
    pub const OCCLUDE: u32 = 2;
    pub const EXECUTE_IN_CLOSEST_FRUSTUM: u32 = 4;
    pub const DEBUG_SHOW_BOUNDING_VOLUME: u32 = 8;
    pub const CAST_SHADOWS: u32 = 16;
    pub const RECEIVE_SHADOWS: u32 = 32;
    pub const PICK_ONLY: u32 = 64;
    pub const DEPTH_FOR_TRANSLUCENT_CLASSIFICATION: u32 = 128;

    pub fn new() -> Self { Self(0) }

    pub fn has(&self, flag: u32) -> bool {
        (self.0 & flag) != 0
    }

    pub fn set(&mut self, flag: u32, value: bool) {
        if value {
            self.0 |= flag;
        } else {
            self.0 &= !flag;
        }
    }
}

impl Default for DrawCommandFlags {
    fn default() -> Self { Self::new() }
}

/// A command to draw primitives.
///
/// Mirrors the CesiumJS `DrawCommand` class which holds all the information
/// needed to draw a primitive, including shader program, vertex array,
/// render state, and uniform map.
pub struct DrawCommand {
    /// The bounding volume of the geometry in world space.
    pub bounding_volume: Option<Box<dyn Any + Send + Sync>>,
    /// The oriented bounding box for plane intersection testing.
    pub oriented_bounding_box: Option<Box<dyn Any + Send + Sync>>,
    /// The model matrix for transforming from model to world space.
    pub model_matrix: Option<Box<dyn Any + Send + Sync>>,
    /// The primitive type (triangles, lines, points, etc.).
    pub primitive_type: u32,
    /// The vertex array to draw.
    pub vertex_array: Option<Box<dyn Any + Send + Sync>>,
    /// The number of vertices or indices to draw.
    pub count: Option<u32>,
    /// The offset into the vertex array or index buffer.
    pub offset: u32,
    /// The number of instances to draw (for instanced rendering).
    pub instance_count: u32,
    /// The shader program to use.
    pub shader_program: Option<ShaderProgram>,
    /// The uniform map function for setting uniform values.
    pub uniform_map: Option<Box<dyn Fn() + Send + Sync>>,
    /// The render state for this command.
    pub render_state: RenderState,
    /// The framebuffer to render to (None = default framebuffer).
    pub framebuffer: Option<Box<dyn Any + Send + Sync>>,
    /// The pass this command belongs to.
    pub pass: Option<u32>,
    /// The owner of this command (for debugging).
    pub owner: Option<Box<dyn Any + Send + Sync>>,
    /// The pick ID for this command.
    pub pick_id: Option<String>,
    /// Whether pick metadata is allowed.
    pub pick_metadata_allowed: bool,
    /// Behavior flags.
    pub flags: DrawCommandFlags,
    /// Whether this command has been modified since last execution.
    pub dirty: bool,
    /// The last time this command was marked dirty.
    pub last_dirty_time: u64,
}

impl DrawCommand {
    /// Creates a new draw command with default values.
    pub fn new() -> Self {
        let mut flags = DrawCommandFlags::new();
        flags.set(DrawCommandFlags::CULL, true);
        flags.set(DrawCommandFlags::OCCLUDE, true);

        Self {
            bounding_volume: None,
            oriented_bounding_box: None,
            model_matrix: None,
            primitive_type: 4, // TRIANGLES
            vertex_array: None,
            count: None,
            offset: 0,
            instance_count: 0,
            shader_program: None,
            uniform_map: None,
            render_state: RenderState::default(),
            framebuffer: None,
            pass: None,
            owner: None,
            pick_id: None,
            pick_metadata_allowed: false,
            flags,
            dirty: true,
            last_dirty_time: 0,
        }
    }

    // ---- Flag accessors ----

    /// Whether this command should be frustum/horizon culled.
    pub fn cull(&self) -> bool {
        self.flags.has(DrawCommandFlags::CULL)
    }

    pub fn set_cull(&mut self, value: bool) {
        if self.flags.has(DrawCommandFlags::CULL) != value {
            self.flags.set(DrawCommandFlags::CULL, value);
            self.dirty = true;
        }
    }

    /// Whether this command should be horizon culled.
    pub fn occlude(&self) -> bool {
        self.flags.has(DrawCommandFlags::OCCLUDE)
    }

    pub fn set_occlude(&mut self, value: bool) {
        if self.flags.has(DrawCommandFlags::OCCLUDE) != value {
            self.flags.set(DrawCommandFlags::OCCLUDE, value);
            self.dirty = true;
        }
    }

    /// Whether to execute in the closest frustum only.
    pub fn execute_in_closest_frustum(&self) -> bool {
        self.flags.has(DrawCommandFlags::EXECUTE_IN_CLOSEST_FRUSTUM)
    }

    pub fn set_execute_in_closest_frustum(&mut self, value: bool) {
        if self.flags.has(DrawCommandFlags::EXECUTE_IN_CLOSEST_FRUSTUM) != value {
            self.flags.set(DrawCommandFlags::EXECUTE_IN_CLOSEST_FRUSTUM, value);
            self.dirty = true;
        }
    }

    /// Whether to debug show the bounding volume.
    pub fn debug_show_bounding_volume(&self) -> bool {
        self.flags.has(DrawCommandFlags::DEBUG_SHOW_BOUNDING_VOLUME)
    }

    pub fn set_debug_show_bounding_volume(&mut self, value: bool) {
        if self.flags.has(DrawCommandFlags::DEBUG_SHOW_BOUNDING_VOLUME) != value {
            self.flags.set(DrawCommandFlags::DEBUG_SHOW_BOUNDING_VOLUME, value);
            self.dirty = true;
        }
    }

    /// Whether this command casts shadows.
    pub fn cast_shadows(&self) -> bool {
        self.flags.has(DrawCommandFlags::CAST_SHADOWS)
    }

    pub fn set_cast_shadows(&mut self, value: bool) {
        if self.flags.has(DrawCommandFlags::CAST_SHADOWS) != value {
            self.flags.set(DrawCommandFlags::CAST_SHADOWS, value);
            self.dirty = true;
        }
    }

    /// Whether this command receives shadows.
    pub fn receive_shadows(&self) -> bool {
        self.flags.has(DrawCommandFlags::RECEIVE_SHADOWS)
    }

    pub fn set_receive_shadows(&mut self, value: bool) {
        if self.flags.has(DrawCommandFlags::RECEIVE_SHADOWS) != value {
            self.flags.set(DrawCommandFlags::RECEIVE_SHADOWS, value);
            self.dirty = true;
        }
    }

    /// Whether this command is for picking only (not rendered to color buffer).
    pub fn pick_only(&self) -> bool {
        self.flags.has(DrawCommandFlags::PICK_ONLY)
    }

    pub fn set_pick_only(&mut self, value: bool) {
        if self.flags.has(DrawCommandFlags::PICK_ONLY) != value {
            self.flags.set(DrawCommandFlags::PICK_ONLY, value);
            self.dirty = true;
        }
    }

    /// Whether this command is for depth testing for translucent classification.
    pub fn depth_for_translucent_classification(&self) -> bool {
        self.flags.has(DrawCommandFlags::DEPTH_FOR_TRANSLUCENT_CLASSIFICATION)
    }

    pub fn set_depth_for_translucent_classification(&mut self, value: bool) {
        if self.flags.has(DrawCommandFlags::DEPTH_FOR_TRANSLUCENT_CLASSIFICATION) != value {
            self.flags.set(DrawCommandFlags::DEPTH_FOR_TRANSLUCENT_CLASSIFICATION, value);
            self.dirty = true;
        }
    }

    /// Creates a derived command for picking.
    ///
    /// Mirrors `DrawCommand.shallowClone()` for creating pick variants.
    pub fn shallow_clone(&self) -> Self {
        Self {
            bounding_volume: None, // Derived commands don't copy bounding volume
            oriented_bounding_box: None,
            model_matrix: None,
            primitive_type: self.primitive_type,
            vertex_array: None,
            count: self.count,
            offset: self.offset,
            instance_count: self.instance_count,
            shader_program: None,
            uniform_map: None,
            render_state: self.render_state.clone(),
            framebuffer: None,
            pass: self.pass,
            owner: None,
            pick_id: self.pick_id.clone(),
            pick_metadata_allowed: self.pick_metadata_allowed,
            flags: self.flags,
            dirty: true,
            last_dirty_time: 0,
        }
    }
}

impl Default for DrawCommand {
    fn default() -> Self { Self::new() }
}
