//! Ported from `packages/engine/Source/Scene/GlobeSurfaceTileProvider.js`.
//!
//! Renders a tile of the globe surface.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::near_far_scalar::NearFarScalar;

use crate::frame_state::FrameState;
use crate::globe_surface_shader_set::GlobeSurfaceShaderSet;
use crate::imagery_layer_collection::ImageryLayerCollection;
use crate::shadow_mode::ShadowMode;

/// Renders a tile of the globe surface.
///
/// This is the workhorse of the globe rendering system — for each visible quadtree tile,
/// it assembles terrain geometry, imagery textures, and shader uniforms, then issues
/// draw commands.
pub struct GlobeSurfaceTileProvider {
    // ---- Configuration ----
    surface_shader_set: GlobeSurfaceShaderSet,

    // ---- Terrain ----
    enable_lighting: bool,
    lambert_diffuse_multiplier: f64,
    show_skirts: bool,
    back_face_culling: bool,
    vertex_shadow_darkness: f64,

    // ---- Atmosphere ----
    dynamic_atmosphere_lighting: bool,
    dynamic_atmosphere_lighting_from_sun: bool,
    show_ground_atmosphere: bool,
    atmosphere_light_intensity: f64,
    atmosphere_rayleigh_coefficient: Cartesian3,
    atmosphere_mie_coefficient: Cartesian3,
    atmosphere_rayleigh_scale_height: f64,
    atmosphere_mie_scale_height: f64,
    atmosphere_mie_anisotropy: f64,
    hue_shift: f64,
    saturation_shift: f64,
    brightness_shift: f64,

    // ---- Lighting fade ----
    lighting_fade_out_distance: f64,
    lighting_fade_in_distance: f64,
    night_fade_out_distance: f64,
    night_fade_in_distance: f64,

    // ---- Water ----
    has_water_mask: bool,
    show_water_effect: bool,
    zoomed_out_ocean_specular_intensity: f64,

    // ---- Underground coloring ----
    underground_color: Color,
    underground_color_alpha_by_distance: NearFarScalar,

    // ---- Shadows ----
    shadows: ShadowMode,

    // ---- Fill highlight ----
    fill_highlight_color: Option<Color>,

    // ---- Base color (when no imagery) ----
    base_color: Color,
}

impl GlobeSurfaceTileProvider {
    /// Creates a new GlobeSurfaceTileProvider.
    pub fn new() -> Self {
        Self {
            surface_shader_set: GlobeSurfaceShaderSet::new(),
            enable_lighting: false,
            lambert_diffuse_multiplier: 0.9,
            show_skirts: true,
            back_face_culling: true,
            vertex_shadow_darkness: 0.6,
            dynamic_atmosphere_lighting: true,
            dynamic_atmosphere_lighting_from_sun: false,
            show_ground_atmosphere: true,
            atmosphere_light_intensity: 10.0,
            atmosphere_rayleigh_coefficient: Cartesian3::new(5.5e-6, 13.0e-6, 28.4e-6),
            atmosphere_mie_coefficient: Cartesian3::new(21e-6, 21e-6, 21e-6),
            atmosphere_rayleigh_scale_height: 10000.0,
            atmosphere_mie_scale_height: 3200.0,
            atmosphere_mie_anisotropy: 0.999,
            hue_shift: 0.0,
            saturation_shift: 0.0,
            brightness_shift: 0.0,
            lighting_fade_out_distance: 1.0e7,
            lighting_fade_in_distance: 1.0e7,
            night_fade_out_distance: 1.0e7,
            night_fade_in_distance: 1.0e7,
            has_water_mask: false,
            show_water_effect: true,
            zoomed_out_ocean_specular_intensity: 0.4,
            underground_color: Color::new(0.0, 0.0, 0.0, 1.0),
            underground_color_alpha_by_distance: NearFarScalar::default(),
            shadows: ShadowMode::Disabled,
            fill_highlight_color: None,
            base_color: Color::new(0.0, 0.1, 0.3, 1.0), // dark blue-ish ocean
        }
    }

    // ---- Setters (called by Globe::begin_frame to propagate properties) ----

    pub fn set_enable_lighting(&mut self, value: bool) { self.enable_lighting = value; }
    pub fn set_dynamic_atmosphere_lighting(&mut self, value: bool) { self.dynamic_atmosphere_lighting = value; }
    pub fn set_show_ground_atmosphere(&mut self, value: bool) { self.show_ground_atmosphere = value; }
    pub fn set_atmosphere_light_intensity(&mut self, value: f64) { self.atmosphere_light_intensity = value; }
    pub fn set_shadows(&mut self, value: ShadowMode) { self.shadows = value; }
    pub fn set_show_skirts(&mut self, value: bool) { self.show_skirts = value; }
    pub fn set_back_face_culling(&mut self, value: bool) { self.back_face_culling = value; }
    pub fn set_vertex_shadow_darkness(&mut self, value: f64) { self.vertex_shadow_darkness = value; }
    pub fn set_underground_color(&mut self, value: Color) { self.underground_color = value; }
    pub fn set_lambert_diffuse_multiplier(&mut self, value: f64) { self.lambert_diffuse_multiplier = value; }
    pub fn set_lighting_fade_out_distance(&mut self, value: f64) { self.lighting_fade_out_distance = value; }
    pub fn set_lighting_fade_in_distance(&mut self, value: f64) { self.lighting_fade_in_distance = value; }
    pub fn set_has_water_mask(&mut self, value: bool) { self.has_water_mask = value; }
    pub fn set_show_water_effect(&mut self, value: bool) { self.show_water_effect = value; }
    pub fn set_fill_highlight_color(&mut self, value: Option<Color>) { self.fill_highlight_color = value; }

    // ---- Getters ----

    /// Gets the base color used when no imagery is available.
    pub fn base_color(&self) -> &Color { &self.base_color }
    /// Sets the base color.
    pub fn set_base_color(&mut self, value: Color) { self.base_color = value; }
    /// Whether lighting is enabled.
    pub fn enable_lighting(&self) -> bool { self.enable_lighting }
    /// Whether ground atmosphere is shown.
    pub fn show_ground_atmosphere(&self) -> bool { self.show_ground_atmosphere }
    /// The shadow mode.
    pub fn shadows(&self) -> ShadowMode { self.shadows }

    // ---- Frame lifecycle ----

    /// Called at the beginning of each frame.
    pub fn begin_frame(&mut self, _frame_state: &FrameState) {
        // In full port: process tile load queues, start new loads
    }

    /// Renders a single tile.
    pub fn render_tile(&self, _tile: &crate::quadtree_tile::QuadtreeTile, _frame_state: &FrameState) {
        // In full port: assemble draw commands for terrain + imagery
    }

    /// Called at the end of each frame.
    pub fn end_frame(&mut self, _frame_state: &FrameState) {
        // In full port: free tiles not needed this frame
    }
}

impl Default for GlobeSurfaceTileProvider {
    fn default() -> Self { Self::new() }
}
