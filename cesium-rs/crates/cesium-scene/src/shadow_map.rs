//! Ported from `packages/engine/Source/Scene/ShadowMap.js`.
//!
//! Shadow mapping for directional and point lights.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::matrix4::Matrix4;

use crate::frame_state::FrameState;

/// A shadow map used for rendering shadows.
///
/// Supports cascaded shadow maps for directional lights and cube map shadows
/// for point lights. Mirrors CesiumJS `ShadowMap` (1963 lines).
pub struct ShadowMap {
    // ---- configuration ----
    /// Whether the shadow map is enabled.
    pub enabled: bool,
    /// Whether soft shadows (PCF) are enabled.
    pub soft_shadows: bool,
    /// Whether normal offset bias is applied.
    pub normal_offset: bool,
    /// The shadow darkness (0 = no shadow, 1 = full black).
    pub darkness: f64,
    /// Whether shadows fade as the light approaches the horizon.
    pub fading_enabled: bool,
    /// Maximum distance for cascaded shadows.
    pub maximum_distance: f64,
    /// The size (width and height) of each shadow map in pixels.
    pub size: i32,
    /// Whether the light source is a point light (uses cube map).
    pub is_point_light: bool,
    /// Radius of the point light.
    pub point_light_radius: f64,
    /// Whether cascaded shadows are enabled.
    pub cascades_enabled: bool,
    /// Number of shadow cascades (1 or 4).
    pub number_of_cascades: i32,

    // ---- runtime state ----
    /// Whether the shadow map needs to be recomputed.
    pub dirty: bool,
    /// Whether the shadow map originates from a light source.
    pub from_light_source: bool,
    /// Whether the shadow map is out of view.
    out_of_view: bool,
    /// Whether the shadow map needs updating this frame.
    needs_update: bool,

    // ---- biases ----
    /// Polygon offset factor for terrain.
    terrain_polygon_offset_factor: f64,
    /// Polygon offset units for terrain.
    terrain_polygon_offset_units: f64,
    /// Polygon offset factor for primitives.
    primitive_polygon_offset_factor: f64,
    /// Polygon offset units for primitives.
    primitive_polygon_offset_units: f64,
    /// Depth bias for point lights.
    point_depth_bias: f64,

    // ---- GPU resources (stubs) ----
    /// The shadow map matrix (light space transform).
    shadow_map_matrix: Matrix4,
    /// The light direction in eye coordinates.
    light_direction_ec: Cartesian3,
    /// The distance from the light to the scene.
    distance: f64,
}

impl ShadowMap {
    /// Creates a new ShadowMap with default settings.
    pub fn new() -> Self {
        Self {
            enabled: true,
            soft_shadows: false,
            normal_offset: true,
            darkness: 0.3,
            fading_enabled: true,
            maximum_distance: 5000.0,
            size: 2048,
            is_point_light: false,
            point_light_radius: 100.0,
            cascades_enabled: true,
            number_of_cascades: 4,
            dirty: true,
            from_light_source: true,
            out_of_view: false,
            needs_update: true,
            terrain_polygon_offset_factor: 1.1,
            terrain_polygon_offset_units: 4.0,
            primitive_polygon_offset_factor: 1.1,
            primitive_polygon_offset_units: 4.0,
            point_depth_bias: 0.0005,
            shadow_map_matrix: Matrix4::IDENTITY,
            light_direction_ec: Cartesian3::new(0.0, 0.0, -1.0),
            distance: 0.0,
        }
    }

    /// Updates the shadow map for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        // DEVIATION: Requires frustum culling, cascade splitting, and framebuffer management
        if self.dirty {
            self.needs_update = true;
            self.dirty = false;
        }
    }

    /// Returns the shadow map matrix.
    pub fn shadow_map_matrix(&self) -> &Matrix4 {
        &self.shadow_map_matrix
    }

    /// Returns whether the shadow map is out of view.
    pub fn is_out_of_view(&self) -> bool {
        self.out_of_view
    }
}

impl Default for ShadowMap {
    fn default() -> Self { Self::new() }
}
