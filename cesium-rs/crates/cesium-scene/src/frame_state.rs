//! Ported from `packages/engine/Source/Scene/FrameState.js`.
//!
//! Per-frame state passed through the update/render pipeline.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::julian_date::JulianDate;
use cesium_core::matrix4::Matrix4;
use crate::scene_mode::SceneMode;

/// The state of the current frame, passed through the update/render pipeline.
pub struct FrameState {
    /// The current scene mode.
    pub mode: SceneMode,
    /// The morph time (0.0 = 2D/CV, 1.0 = 3D).
    pub morph_time: f64,
    /// The current simulation time.
    pub time: JulianDate,
    /// The view matrix.
    pub view_matrix: Matrix4,
    /// The projection matrix.
    pub projection_matrix: Matrix4,
    /// The view-projection matrix.
    pub view_projection_matrix: Matrix4,
    /// The inverse view matrix.
    pub inverse_view_matrix: Matrix4,
    /// The inverse projection matrix.
    pub inverse_projection_matrix: Matrix4,
    /// The camera position in world coordinates.
    pub camera_position: Cartesian3,
    /// The camera direction in world coordinates.
    pub camera_direction: Cartesian3,
    /// The camera up vector in world coordinates.
    pub camera_up: Cartesian3,
    /// The camera right vector in world coordinates.
    pub camera_right: Cartesian3,
    /// The drawing buffer width.
    pub drawing_buffer_width: u32,
    /// The drawing buffer height.
    pub drawing_buffer_height: u32,
    /// The camera frustum's SSE denominator (`2 * tan(fov / 2)` for a
    /// perspective frustum), mirroring CesiumJS `frustum.sseDenominator`.
    pub sse_denominator: f64,
    /// The current frame number.
    pub frame_number: u64,
    /// The current context (if available).
    pub context_ready: bool,
    /// Whether the scene is rendering a pick pass.
    pub pick_objects: bool,
    /// Whether the scene is rendering a shadow map.
    pub shadow_maps_enabled: bool,
    /// The passes being rendered.
    pub passes: FramePasses,
}

/// Describes which rendering passes are active.
pub struct FramePasses {
    /// Whether the main color pass is active.
    pub main: bool,
    /// Whether the reflection pass is active.
    pub reflection: bool,
    /// Whether a pick pass is active.
    pub pick: bool,
    /// Whether a shadow pass is active.
    pub shadow: bool,
}

impl FramePasses {
    /// Creates a new default frame passes.
    pub fn new() -> Self {
        Self { main: true, reflection: false, pick: false, shadow: false }
    }
}

impl Default for FramePasses {
    fn default() -> Self { Self::new() }
}

impl FrameState {
    /// Creates a new default frame state.
    pub fn new() -> Self {
        Self {
            mode: SceneMode::Scene3D,
            morph_time: 1.0,
            time: JulianDate::now(),
            view_matrix: Matrix4::IDENTITY,
            projection_matrix: Matrix4::IDENTITY,
            view_projection_matrix: Matrix4::IDENTITY,
            inverse_view_matrix: Matrix4::IDENTITY,
            inverse_projection_matrix: Matrix4::IDENTITY,
            camera_position: Cartesian3::default(),
            camera_direction: Cartesian3::default(),
            camera_up: Cartesian3::default(),
            camera_right: Cartesian3::default(),
            drawing_buffer_width: 0,
            drawing_buffer_height: 0,
            sse_denominator: 2.0 * (std::f64::consts::FRAC_PI_3 * 0.5).tan(),
            frame_number: 0,
            context_ready: false,
            pick_objects: false,
            shadow_maps_enabled: false,
            passes: FramePasses::default(),
        }
    }
}

impl Default for FrameState {
    fn default() -> Self { Self::new() }
}
