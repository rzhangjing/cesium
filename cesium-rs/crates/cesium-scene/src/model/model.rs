//! Ported from `packages/engine/Source/Scene/Model/Model.js`.
//!
//! A 3D model based on glTF.

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;
use cesium_core::event::Event;
use cesium_core::matrix4::Matrix4;

use crate::frame_state::FrameState;
use crate::shadow_mode::ShadowMode;

/// A 3D model based on glTF, the runtime asset format for WebGL, OpenGL ES, and OpenGL.
///
/// Use `Model::from_gltf_async` to construct. Do not call the constructor directly.
/// Mirrors CesiumJS `Model` (3376 lines).
pub struct Model {
    // ---- identity ----
    /// A user-defined ID for this model.
    pub id: Option<String>,
    /// The type of model (GLTF, B3DM, I3DM, PNTS, GEOJSON).
    pub model_type: ModelType,

    // ---- transform ----
    /// The 4x4 transformation matrix from model to world coordinates.
    pub model_matrix: Matrix4,
    /// A uniform scale applied to this model.
    pub scale: f64,
    /// The minimum pixel size of the model regardless of zoom.
    pub minimum_pixel_size: f64,
    /// The maximum scale size of the model.
    pub maximum_scale: Option<f64>,

    // ---- appearance ----
    /// Whether the model is shown.
    pub show: bool,
    /// The color to blend with the model's base color.
    pub color: Color,
    /// The color blend mode.
    pub color_blend_mode: ColorBlendMode,
    /// The color blend amount (0.0 to 1.0).
    pub color_blend_amount: f64,
    /// The silhouette color.
    pub silhouette_color: Color,
    /// The silhouette size.
    pub silhouette_size: f64,
    /// The shadow mode.
    pub shadows: ShadowMode,
    /// The split direction.
    pub split_direction: SplitDirection,
    /// Whether the model has a custom shader.
    pub has_custom_shader: bool,

    // ---- lighting ----
    /// Whether lighting is enabled.
    pub enable_lighting: bool,
    /// The image-based lighting intensity.
    pub image_based_lighting_intensity: f64,
    /// Whether to use image-based lighting.
    pub use_image_based_lighting: bool,
    /// Whether to use specular environment maps.
    pub use_specular_environment_maps: bool,
    /// Whether to use diffuse environment maps.
    pub use_diffuse_environment_maps: bool,

    // ---- point cloud ----
    /// The point cloud shading attenuation distance.
    pub point_cloud_shading_attenuation: bool,

    // ---- clipping ----
    /// Whether back-face culling is enabled.
    pub back_face_culling: bool,
    /// Whether to show debug wireframe.
    pub debug_wireframe: bool,
    /// Whether to show debug bounding volume.
    pub debug_bounding_volume: bool,

    // ---- state ----
    /// Whether the model is ready (loaded and processed).
    pub ready: bool,
    /// Whether the model has been destroyed.
    is_destroyed: bool,
    /// The bounding sphere of the model.
    pub bounding_sphere: BoundingSphere,
    /// The active time for animations.
    pub active_time: f64,
    /// Whether vertical exaggeration is enabled.
    pub enable_vertical_exaggeration: bool,

    // ---- events ----
    /// Event raised when the model is ready.
    pub ready_event: Event,
}

/// The type of a 3D model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
    /// A standard glTF model.
    Gltf,
    /// Batched 3D Model.
    B3dm,
    /// Instanced 3D Model.
    I3dm,
    /// Point Cloud.
    Pnts,
    /// GeoJSON vector tile.
    GeoJson,
}

/// The color blend mode for a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorBlendMode {
    /// Highlight: blend between original and highlight color.
    Highlight,
    /// Replace: replace original color entirely.
    Replace,
    /// Mix: mix original and highlight color.
    Mix,
}

/// The split direction for a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    /// Render on the left side.
    Left,
    /// Render on both sides.
    None,
    /// Render on the right side.
    Right,
}

impl Model {
    /// Creates a new Model with default values.
    pub fn new() -> Self {
        Self {
            id: None,
            model_type: ModelType::Gltf,
            model_matrix: Matrix4::IDENTITY,
            scale: 1.0,
            minimum_pixel_size: 0.0,
            maximum_scale: None,
            show: true,
            color: Color::new(1.0, 1.0, 1.0, 1.0),
            color_blend_mode: ColorBlendMode::Highlight,
            color_blend_amount: 0.0,
            silhouette_color: Color::new(1.0, 1.0, 1.0, 1.0),
            silhouette_size: 0.0,
            shadows: ShadowMode::Enabled,
            split_direction: SplitDirection::None,
            has_custom_shader: false,
            enable_lighting: true,
            image_based_lighting_intensity: 1.0,
            use_image_based_lighting: true,
            use_specular_environment_maps: false,
            use_diffuse_environment_maps: false,
            point_cloud_shading_attenuation: true,
            back_face_culling: false,
            debug_wireframe: false,
            debug_bounding_volume: false,
            ready: false,
            is_destroyed: false,
            bounding_sphere: BoundingSphere::new(Cartesian3::ZERO, 0.0),
            active_time: 0.0,
            enable_vertical_exaggeration: true,
            ready_event: Event::new(),
        }
    }

    /// Updates the model for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        // DEVIATION: Requires full glTF pipeline processing, animation update,
        // scene graph traversal, and draw command generation
        if !self.ready {
            // DEVIATION: In real implementation, would trigger async loading
        }
    }

    /// Returns whether this model has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this model and releases GPU resources.
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
    }

    /// Gets a node by name.
    pub fn get_node(&self, _name: &str) -> Option<()> {
        // DEVIATION: Requires scene graph
        None
    }

    /// Gets a mesh by name.
    pub fn get_mesh(&self, _name: &str) -> Option<()> {
        // DEVIATION: Requires mesh registry
        None
    }

    /// Gets a material by name.
    pub fn get_material(&self, _name: &str) -> Option<()> {
        // DEVIATION: Requires material registry
        None
    }
}

impl Default for Model {
    fn default() -> Self { Self::new() }
}
