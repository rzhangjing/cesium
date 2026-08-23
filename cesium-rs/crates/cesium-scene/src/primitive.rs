//! Ported from `packages/engine/Source/Scene/Primitive.js`.
//!
//! A renderable primitive composed of geometry instances.

use cesium_core::color::Color;
use cesium_core::geometry_instance::GeometryInstance;

use crate::frame_state::FrameState;
use crate::shadow_mode::ShadowMode;

/// A renderable primitive composed of geometry instances.
///
/// Primitives are the main way to render geometry in CesiumJS. Each primitive
/// holds one or more GeometryInstances, an Appearance, and manages the GPU
/// pipeline for rendering.
pub struct Primitive {
    /// The geometry instances to render.
    pub geometry_instances: Vec<GeometryInstance>,
    /// Whether this primitive is shown.
    pub show: bool,
    /// The model matrix applied to all geometry instances.
    pub model_matrix: cesium_core::matrix4::Matrix4,
    /// Whether to enable vertex shader-based depth clipping.
    pub depth_fail_material: Option<crate::material::Material>,
    /// Whether to allow picking of individual instances.
    pub allow_picking: bool,
    /// Whether to enable compression of geometry attributes.
    pub compress: bool,
    /// Whether to keep the geometry's local coordinate frame.
    pub release_geometry_instances: bool,
    /// Whether this primitive is translucent.
    pub translucent: bool,
    /// The shadow mode for this primitive.
    pub shadows: ShadowMode,
    /// The classification type (if this is a classification primitive).
    pub classification_type: Option<()>, // Placeholder for ClassificationType
    /// Whether this primitive has been destroyed.
    is_destroyed: bool,
    /// Whether this primitive is ready for rendering.
    ready: bool,
}

impl Primitive {
    /// Creates a new Primitive.
    pub fn new() -> Self {
        Self {
            geometry_instances: Vec::new(),
            show: true,
            model_matrix: cesium_core::matrix4::Matrix4::IDENTITY,
            depth_fail_material: None,
            allow_picking: true,
            compress: true,
            release_geometry_instances: true,
            translucent: false,
            shadows: ShadowMode::Disabled,
            classification_type: None,
            is_destroyed: false,
            ready: false,
        }
    }

    /// Adds a geometry instance to this primitive.
    pub fn add_instance(&mut self, instance: GeometryInstance) {
        self.geometry_instances.push(instance);
    }

    /// Returns the number of geometry instances.
    pub fn instance_count(&self) -> usize {
        self.geometry_instances.len()
    }

    /// Returns whether this primitive is ready for rendering.
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Updates this primitive for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        if !self.show {
            return;
        }
        // In full port:
        // 1. Process geometry instances through GeometryPipeline
        // 2. Create GPU buffers for vertex data
        // 3. Build render commands
        // 4. Submit draw commands
    }

    /// Returns true if this object was destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys the WebGL resources held by this object.
    pub fn destroy(&mut self) {
        self.geometry_instances.clear();
        self.is_destroyed = true;
    }
}

impl Default for Primitive {
    fn default() -> Self { Self::new() }
}
