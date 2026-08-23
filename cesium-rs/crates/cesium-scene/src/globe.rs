//! Ported from `packages/engine/Source/Scene/Globe.js`.
//!
//! The globe rendered in the scene, including its terrain and imagery layers.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::ellipsoid_terrain_provider::EllipsoidTerrainProvider;
use cesium_core::event::Event;
use cesium_core::near_far_scalar::NearFarScalar;
use cesium_core::ray::Ray;
use cesium_core::rectangle::Rectangle;

use crate::frame_state::FrameState;
use crate::globe_surface_shader_set::GlobeSurfaceShaderSet;
use crate::globe_surface_tile_provider::GlobeSurfaceTileProvider;
use crate::globe_translucency::GlobeTranslucency;
use crate::imagery_layer_collection::ImageryLayerCollection;
use crate::quadtree_primitive::QuadtreePrimitive;
use crate::shadow_mode::ShadowMode;

/// The globe rendered in the scene, including its terrain and imagery layers.
/// Access the globe using `Scene::globe`.
pub struct Globe {
    // ---- Core references ----
    ellipsoid: Ellipsoid,
    imagery_layer_collection: ImageryLayerCollection,
    surface_shader_set: GlobeSurfaceShaderSet,
    surface: QuadtreePrimitive,
    surface_tile_provider: GlobeSurfaceTileProvider,
    terrain_provider_changed: Event,
    translucency: GlobeTranslucency,

    // ---- Visual properties ----
    underground_color: Color,
    underground_color_alpha_by_distance: NearFarScalar,

    // ---- Public properties (mirroring CesiumJS public fields) ----
    /// Determines if the globe will be shown.
    pub show: bool,
    /// The maximum screen-space error used to drive level-of-detail refinement.
    pub maximum_screen_space_error: f64,
    /// The size of the terrain tile cache.
    pub tile_cache_size: i32,
    /// Number of loading descendant tiles considered "too many".
    pub loading_descendant_limit: i32,
    /// Whether ancestors of rendered tiles should be preloaded.
    pub preload_ancestors: bool,
    /// Whether siblings of rendered tiles should be preloaded.
    pub preload_siblings: bool,
    /// Enable lighting the globe with the scene's light source.
    pub enable_lighting: bool,
    /// A multiplier to adjust terrain lambert lighting.
    pub lambert_diffuse_multiplier: f64,
    /// Enable dynamic lighting effects on atmosphere and fog.
    pub dynamic_atmosphere_lighting: bool,
    /// Whether dynamic atmosphere lighting uses the sun direction.
    pub dynamic_atmosphere_lighting_from_sun: bool,
    /// Enable the ground atmosphere.
    pub show_ground_atmosphere: bool,
    /// The intensity of the light for computing ground atmosphere color.
    pub atmosphere_light_intensity: f64,
    /// Rayleigh scattering coefficient for ground atmosphere.
    pub atmosphere_rayleigh_coefficient: Cartesian3,
    /// Mie scattering coefficient for ground atmosphere.
    pub atmosphere_mie_coefficient: Cartesian3,
    /// Rayleigh scale height in meters.
    pub atmosphere_rayleigh_scale_height: f64,
    /// Mie scale height in meters.
    pub atmosphere_mie_scale_height: f64,
    /// Anisotropy of the medium for Mie scattering.
    pub atmosphere_mie_anisotropy: f64,
    /// Hue shift for atmosphere.
    pub atmosphere_hue_shift: f64,
    /// Saturation shift for atmosphere.
    pub atmosphere_saturation_shift: f64,
    /// Brightness shift for atmosphere.
    pub atmosphere_brightness_shift: f64,
    /// The color to highlight terrain fill tiles.
    pub fill_highlight_color: Option<Color>,
    /// Distance at which lighting fades out.
    pub lighting_fade_out_distance: f64,
    /// Distance at which lighting fades in.
    pub lighting_fade_in_distance: f64,
    /// Distance at which night lighting fades out.
    pub night_fade_out_distance: f64,
    /// Distance at which night lighting fades in.
    pub night_fade_in_distance: f64,
    /// Whether the water effect is shown.
    pub show_water_effect: bool,
    /// Whether to show terrain skirts.
    pub show_skirts: bool,
    /// Whether back face culling is enabled.
    pub back_face_culling: bool,
    /// Vertex shadow darkness.
    pub vertex_shadow_darkness: f64,
    /// Shadow mode.
    pub shadows: ShadowMode,
    /// Whether the globe has been destroyed.
    is_destroyed: bool,
}

impl Globe {
    /// Creates a new Globe.
    pub fn new(ellipsoid: Option<Ellipsoid>) -> Self {
        let ellipsoid = ellipsoid.unwrap_or(Ellipsoid::WGS84);
        let terrain_provider = EllipsoidTerrainProvider::new(None, Some(ellipsoid.clone()));
        let imagery_layer_collection = ImageryLayerCollection::new();
        let surface_shader_set = GlobeSurfaceShaderSet::new();
        let surface_tile_provider = GlobeSurfaceTileProvider::new();
        let surface = QuadtreePrimitive::new();
        let translucency = GlobeTranslucency::new();

        let max_radius = ellipsoid.maximum_radius();

        Self {
            ellipsoid,
            imagery_layer_collection,
            surface_shader_set,
            surface,
            surface_tile_provider,
            terrain_provider_changed: Event::new(),
            translucency,
            underground_color: Color::new(0.0, 0.0, 0.0, 1.0),
            underground_color_alpha_by_distance: NearFarScalar::new(
                max_radius / 1000.0,
                0.0,
                max_radius / 5.0,
                1.0,
            ),
            show: true,
            maximum_screen_space_error: 2.0,
            tile_cache_size: 100,
            loading_descendant_limit: 20,
            preload_ancestors: true,
            preload_siblings: false,
            enable_lighting: false,
            lambert_diffuse_multiplier: 0.9,
            dynamic_atmosphere_lighting: true,
            dynamic_atmosphere_lighting_from_sun: false,
            show_ground_atmosphere: ellipsoid == Ellipsoid::WGS84,
            atmosphere_light_intensity: 10.0,
            atmosphere_rayleigh_coefficient: Cartesian3::new(5.5e-6, 13.0e-6, 28.4e-6),
            atmosphere_mie_coefficient: Cartesian3::new(21e-6, 21e-6, 21e-6),
            atmosphere_rayleigh_scale_height: 10000.0,
            atmosphere_mie_scale_height: 3200.0,
            atmosphere_mie_anisotropy: 0.999,
            atmosphere_hue_shift: 0.0,
            atmosphere_saturation_shift: 0.0,
            atmosphere_brightness_shift: 0.0,
            fill_highlight_color: None,
            lighting_fade_out_distance: 1.0e7,
            lighting_fade_in_distance: 1.0e7,
            night_fade_out_distance: 1.0e7,
            night_fade_in_distance: 1.0e7,
            show_water_effect: true,
            show_skirts: true,
            back_face_culling: true,
            vertex_shadow_darkness: 0.6,
            shadows: ShadowMode::Disabled,
            is_destroyed: false,
        }
    }

    // ---- Getters ----

    /// Gets the ellipsoid describing the shape of this globe.
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    /// Gets the collection of image layers rendered on this globe.
    pub fn imagery_layers(&self) -> &ImageryLayerCollection {
        &self.imagery_layer_collection
    }

    /// Gets the event raised when the terrain provider is changed.
    pub fn terrain_provider_changed(&self) -> &Event {
        &self.terrain_provider_changed
    }

    /// Gets the globe translucency properties.
    pub fn translucency(&self) -> &GlobeTranslucency {
        &self.translucency
    }

    /// Gets or sets the underground color.
    pub fn underground_color(&self) -> &Color {
        &self.underground_color
    }

    /// Sets the underground color.
    pub fn set_underground_color(&mut self, color: Color) {
        self.underground_color = color;
    }

    /// Gets the underground color alpha by distance.
    pub fn underground_color_alpha_by_distance(&self) -> &NearFarScalar {
        &self.underground_color_alpha_by_distance
    }

    /// Sets the underground color alpha by distance.
    pub fn set_underground_color_alpha_by_distance(&mut self, value: NearFarScalar) {
        debug_assert!(value.far >= value.near, "far distance must be greater than near distance");
        self.underground_color_alpha_by_distance = value;
    }

    // ---- Frame lifecycle ----

    /// Updates the globe for the current frame.
    pub fn update(&mut self, frame_state: &FrameState) {
        if !self.show {
            return;
        }
        if frame_state.passes.main {
            self.surface.update(frame_state);
        }
    }

    /// Called at the beginning of each frame.
    pub fn begin_frame(&mut self, frame_state: &FrameState) {
        if !frame_state.passes.main {
            return;
        }

        // Propagate globe properties to the surface tile provider
        self.surface.set_maximum_screen_space_error(self.maximum_screen_space_error);
        self.surface.set_tile_cache_size(self.tile_cache_size);
        self.surface.set_loading_descendant_limit(self.loading_descendant_limit);
        self.surface.set_preload_ancestors(self.preload_ancestors);
        self.surface.set_preload_siblings(self.preload_siblings);

        self.surface_tile_provider.set_enable_lighting(self.enable_lighting);
        self.surface_tile_provider.set_dynamic_atmosphere_lighting(self.dynamic_atmosphere_lighting);
        self.surface_tile_provider.set_show_ground_atmosphere(self.show_ground_atmosphere);
        self.surface_tile_provider.set_atmosphere_light_intensity(self.atmosphere_light_intensity);
        self.surface_tile_provider.set_shadows(self.shadows);
        self.surface_tile_provider.set_show_skirts(self.show_skirts);
        self.surface_tile_provider.set_back_face_culling(self.back_face_culling);
        self.surface_tile_provider.set_vertex_shadow_darkness(self.vertex_shadow_darkness);
        self.surface_tile_provider.set_underground_color(self.underground_color.clone());
        self.surface_tile_provider.set_lambert_diffuse_multiplier(self.lambert_diffuse_multiplier);

        self.surface.begin_frame(frame_state);
    }

    /// Renders the globe.
    pub fn render(&mut self, frame_state: &FrameState) {
        if !self.show {
            return;
        }
        self.surface.render(frame_state);
    }

    /// Called at the end of each frame.
    pub fn end_frame(&mut self, frame_state: &FrameState) {
        if !self.show {
            return;
        }
        if frame_state.passes.main {
            self.surface.end_frame(frame_state);
        }
    }

    // ---- Picking ----

    /// Finds an intersection between a ray and the globe surface.
    pub fn pick_world_coordinates(
        &self,
        ray: &Ray,
        _cull_back_faces: Option<bool>,
    ) -> Option<Cartesian3> {
        // Simplified: in full port, traverses quadtree tiles to find closest intersection
        let _ = ray;
        None
    }

    /// Picks the globe at the given window position.
    pub fn pick(&self, ray: &Ray) -> Option<Cartesian3> {
        self.pick_world_coordinates(ray, None)
    }

    /// Gets the height of the terrain at the given cartographic position.
    pub fn get_height(&self, _cartographic: &cesium_core::cartographic::Cartographic) -> f64 {
        // Simplified: in full port, queries terrain provider for actual height
        0.0
    }

    // ---- Lifecycle ----

    /// Returns true if this object was destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys the WebGL resources held by this object.
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}

impl Default for Globe {
    fn default() -> Self {
        Self::new(None)
    }
}
