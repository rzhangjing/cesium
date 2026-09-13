//! Globe surface rendering and terrain interaction.
//!
//! Maps to CesiumJS `Scene/Globe.js`:
//! - Globe surface properties
//! - Depth test against terrain
//! - Elevation queries
//! - Globe translucency
//! - Underground rendering
//! - Lighting fade distances
//! - Terrain skirts and back-face culling

use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::rectangle::Rectangle;
use glam::DVec3;
use std::f64::consts::PI;

/// Shadow mode for globe rendering.
/// Maps to CesiumJS `Scene/ShadowMode`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShadowMode {
    /// Shadows are disabled.
    Disabled,
    /// Globe only receives shadows.
    #[default]
    ReceiveOnly,
    /// Globe only casts shadows.
    CastOnly,
    /// Globe both casts and receives shadows.
    Enabled,
}

/// Near/far scalar for distance-based interpolation.
/// Maps to CesiumJS `Core/NearFarScalar`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearFarScalar {
    /// Near distance.
    pub near: f64,
    /// Value at near distance.
    pub near_value: f64,
    /// Far distance.
    pub far: f64,
    /// Value at far distance.
    pub far_value: f64,
}

impl NearFarScalar {
    /// Creates a new near/far scalar.
    pub fn new(near: f64, near_value: f64, far: f64, far_value: f64) -> Self {
        Self { near, near_value, far, far_value }
    }

    /// Interpolates the value at a given distance.
    pub fn interpolate(&self, distance: f64) -> f64 {
        if distance <= self.near {
            return self.near_value;
        }
        if distance >= self.far {
            return self.far_value;
        }
        let t = (distance - self.near) / (self.far - self.near);
        self.near_value + t * (self.far_value - self.near_value)
    }
}

/// Globe rendering configuration.
///
/// Maps to CesiumJS `Scene/Globe.js`
#[derive(Debug, Clone)]
pub struct GlobeConfig {
    /// Whether the globe is shown.
    pub show: bool,
    /// Whether to enable depth test against terrain.
    pub depth_test_against_terrain: bool,
    /// Whether the globe is translucent (for underground rendering).
    pub translucency_enabled: bool,
    /// Globe translucency front face alpha.
    pub front_face_alpha: f64,
    /// Globe translucency back face alpha.
    pub back_face_alpha: f64,
    /// Whether to show the ground atmosphere effect.
    pub show_ground_atmosphere: bool,
    /// Whether to show the water effect.
    pub show_water_effect: bool,
    /// Base color when no imagery is available [r, g, b].
    pub base_color: [f64; 3],
    /// Maximum screen space error for terrain tiles.
    pub maximum_screen_space_error: f64,
    /// Tile cache size (number of tiles).
    pub tile_cache_size: usize,
    /// Whether to enable lighting (day/night).
    pub enable_lighting: bool,
    /// Loading descendant limit for tile scheduling.
    pub loading_descendant_limit: u32,
    /// Whether to preload ancestor tiles.
    pub preload_ancestors: bool,
    /// Whether to preload sibling tiles.
    pub preload_siblings: bool,
    /// Fill highlight color [r, g, b, a] (None = no highlight).
    pub fill_highlight_color: Option<[f64; 4]>,
    /// Lambert diffuse multiplier for terrain lighting.
    pub lambert_diffuse_multiplier: f64,
    /// Whether dynamic atmosphere lighting is enabled.
    pub dynamic_atmosphere_lighting: bool,
    /// Whether dynamic atmosphere lighting uses sun direction.
    pub dynamic_atmosphere_lighting_from_sun: bool,
    /// Atmosphere light intensity.
    pub atmosphere_light_intensity: f64,
    /// Rayleigh scattering coefficient [r, g, b].
    pub atmosphere_rayleigh_coefficient: [f64; 3],
    /// Mie scattering coefficient [r, g, b].
    pub atmosphere_mie_coefficient: [f64; 3],
    /// Rayleigh scale height (meters).
    pub atmosphere_rayleigh_scale_height: f64,
    /// Mie scale height (meters).
    pub atmosphere_mie_scale_height: f64,
    /// Mie anisotropy factor (-1.0 to 1.0).
    pub atmosphere_mie_anisotropy: f64,
    /// Distance where lighting fades out (meters).
    pub lighting_fade_out_distance: f64,
    /// Distance where lighting fades in (meters).
    pub lighting_fade_in_distance: f64,
    /// Distance where night fades out (meters).
    pub night_fade_out_distance: f64,
    /// Distance where night fades in (meters).
    pub night_fade_in_distance: f64,
    /// Whether to show terrain skirts.
    pub show_skirts: bool,
    /// Whether to cull back-facing terrain.
    pub back_face_culling: bool,
    /// Vertex shadow darkness (0.0 to 1.0).
    pub vertex_shadow_darkness: f64,
    /// Underground color [r, g, b] (None = disabled).
    pub underground_color: Option<[f64; 3]>,
    /// Underground color alpha by distance.
    pub underground_color_alpha_by_distance: Option<NearFarScalar>,
    /// Cartographic limit rectangle for rendering.
    pub cartographic_limit_rectangle: Rectangle,
    /// Shadow mode.
    pub shadows: ShadowMode,
    /// Atmosphere hue shift (-1.0 to 1.0).
    pub atmosphere_hue_shift: f64,
    /// Atmosphere saturation shift (-1.0 to 1.0).
    pub atmosphere_saturation_shift: f64,
    /// Atmosphere brightness shift (-1.0 to 1.0).
    pub atmosphere_brightness_shift: f64,
}

impl Default for GlobeConfig {
    fn default() -> Self {
        let min_radius = Ellipsoid::WGS84.minimum_radius();
        Self {
            show: true,
            depth_test_against_terrain: false,
            translucency_enabled: false,
            front_face_alpha: 1.0,
            back_face_alpha: 1.0,
            show_ground_atmosphere: true,
            show_water_effect: true,
            base_color: [0.0, 0.0, 0.5], // Ocean blue
            maximum_screen_space_error: 2.0,
            tile_cache_size: 100,
            enable_lighting: false,
            loading_descendant_limit: 20,
            preload_ancestors: true,
            preload_siblings: false,
            fill_highlight_color: None,
            lambert_diffuse_multiplier: 0.9,
            dynamic_atmosphere_lighting: true,
            dynamic_atmosphere_lighting_from_sun: false,
            atmosphere_light_intensity: 10.0,
            atmosphere_rayleigh_coefficient: [5.5e-6, 13.0e-6, 28.4e-6],
            atmosphere_mie_coefficient: [21e-6, 21e-6, 21e-6],
            atmosphere_rayleigh_scale_height: 10000.0,
            atmosphere_mie_scale_height: 3200.0,
            atmosphere_mie_anisotropy: 0.9,
            lighting_fade_out_distance: PI * 0.5 * min_radius,
            lighting_fade_in_distance: PI * min_radius,
            night_fade_out_distance: PI * 0.5 * min_radius,
            night_fade_in_distance: 5.0 * PI * 0.5 * min_radius,
            show_skirts: true,
            back_face_culling: true,
            vertex_shadow_darkness: 0.3,
            underground_color: Some([0.0, 0.0, 0.0]),
            underground_color_alpha_by_distance: Some(NearFarScalar::new(
                min_radius / 1000.0,
                0.0,
                min_radius / 5.0,
                1.0,
            )),
            cartographic_limit_rectangle: Rectangle::MAX_VALUE,
            shadows: ShadowMode::ReceiveOnly,
            atmosphere_hue_shift: 0.0,
            atmosphere_saturation_shift: 0.0,
            atmosphere_brightness_shift: 0.0,
        }
    }
}

/// Globe surface for terrain interaction.
#[derive(Debug, Clone)]
pub struct GlobeSurface {
    /// The ellipsoid shape.
    pub ellipsoid: Ellipsoid,
    /// Configuration.
    pub config: GlobeConfig,
    /// Minimum terrain height in the current view.
    pub minimum_terrain_height: f64,
    /// Maximum terrain height in the current view.
    pub maximum_terrain_height: f64,
}

impl GlobeSurface {
    /// Creates a new globe surface with WGS84 ellipsoid.
    pub fn new() -> Self {
        Self {
            ellipsoid: Ellipsoid::WGS84,
            config: GlobeConfig::default(),
            minimum_terrain_height: 0.0,
            maximum_terrain_height: 0.0,
        }
    }

    /// Creates a globe surface with a custom ellipsoid.
    pub fn with_ellipsoid(ellipsoid: Ellipsoid) -> Self {
        Self {
            ellipsoid,
            config: GlobeConfig::default(),
            minimum_terrain_height: 0.0,
            maximum_terrain_height: 0.0,
        }
    }

    /// Gets the surface normal at a position.
    pub fn get_surface_normal(&self, position: DVec3) -> DVec3 {
        self.ellipsoid
            .geodetic_surface_normal(position)
            .unwrap_or(DVec3::Z)
    }

    /// Gets the height at a cartographic position (simplified).
    ///
    /// In a full implementation, this would query terrain tiles.
    pub fn get_height(&self, _cartographic: &Cartographic) -> Option<f64> {
        // Simplified: return 0 (ellipsoid surface)
        Some(0.0)
    }

    /// Picks the globe surface with a ray.
    ///
    /// Returns the intersection point in world coordinates.
    pub fn pick(&self, ray_origin: DVec3, ray_direction: DVec3) -> Option<DVec3> {
        // Ray-ellipsoid intersection using the standard quadratic formula
        // For ellipsoid x^2/a^2 + y^2/b^2 + z^2/c^2 = 1
        // Ray: P = O + t*D
        // Substituting: (O + t*D)^T * M * (O + t*D) = 1
        // where M = diag(1/a^2, 1/b^2, 1/c^2)

        let radii = self.ellipsoid.radii();
        let one_over_radii_sq = DVec3::new(
            1.0 / (radii.x * radii.x),
            1.0 / (radii.y * radii.y),
            1.0 / (radii.z * radii.z),
        );

        let o = ray_origin;
        let d = ray_direction;

        // Quadratic coefficients: A*t^2 + B*t + C = 0
        let o_scaled = o * one_over_radii_sq;
        let d_scaled = d * one_over_radii_sq;

        let a = d.dot(d_scaled);
        let b = 2.0 * o.dot(d_scaled);
        let c = o.dot(o_scaled) - 1.0;

        let discriminant = b * b - 4.0 * a * c;

        if discriminant < 0.0 {
            return None;
        }

        let sqrt_disc = discriminant.sqrt();
        let t1 = (-b - sqrt_disc) / (2.0 * a);
        let t2 = (-b + sqrt_disc) / (2.0 * a);

        // Find the nearest positive t
        let t = if t1 >= 0.0 {
            t1
        } else if t2 >= 0.0 {
            t2
        } else {
            return None;
        };

        Some(ray_origin + ray_direction * t)
    }

    /// Computes the horizon distance from a height.
    ///
    /// Returns the distance to the horizon in meters.
    pub fn horizon_distance(&self, height: f64) -> f64 {
        let r = self.ellipsoid.maximum_radius();
        // d = sqrt((r + h)^2 - r^2) = sqrt(2*r*h + h^2)
        (2.0 * r * height + height * height).sqrt()
    }

    /// Computes the dip angle of the horizon from a height.
    ///
    /// Returns the angle in radians below horizontal.
    pub fn horizon_dip_angle(&self, height: f64) -> f64 {
        let r = self.ellipsoid.maximum_radius();
        // cos(dip) = r / (r + h)
        (r / (r + height)).acos()
    }

    /// Checks if a position is on the visible hemisphere.
    pub fn is_on_visible_hemisphere(&self, position: DVec3, camera_position: DVec3) -> bool {
        let surface_normal = self.get_surface_normal(position);
        let to_camera = (camera_position - position).normalize();
        surface_normal.dot(to_camera) > 0.0
    }

    /// Computes the approximate level of detail for a tile.
    pub fn compute_tile_sse(
        &self,
        tile_geometric_error: f64,
        distance: f64,
        viewport_height: f64,
        sse_denominator: f64,
    ) -> f64 {
        // SSE = (geometricError * viewportHeight) / (distance * sseDenominator)
        if distance <= 0.0 {
            return f64::MAX;
        }
        (tile_geometric_error * viewport_height) / (distance * sse_denominator)
    }

    /// Determines if a tile should be refined based on SSE.
    pub fn should_refine_tile(&self, sse: f64) -> bool {
        sse > self.config.maximum_screen_space_error
    }
}

impl Default for GlobeSurface {
    fn default() -> Self {
        Self::new()
    }
}

/// Globe translucency settings.
///
/// Maps to CesiumJS `GlobeTranslucency.js`
#[derive(Debug, Clone)]
pub struct GlobeTranslucency {
    /// Whether translucency is enabled.
    pub enabled: bool,
    /// Front face alpha (0.0 = transparent, 1.0 = opaque).
    pub front_face_alpha: f64,
    /// Back face alpha.
    pub back_face_alpha: f64,
}

impl Default for GlobeTranslucency {
    fn default() -> Self {
        Self {
            enabled: false,
            front_face_alpha: 1.0,
            back_face_alpha: 1.0,
        }
    }
}

impl GlobeTranslucency {
    /// Creates new translucency settings.
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            ..Default::default()
        }
    }

    /// Computes the effective alpha for a surface facing the camera.
    pub fn front_alpha(&self) -> f64 {
        if self.enabled {
            self.front_face_alpha
        } else {
            1.0
        }
    }

    /// Computes the effective alpha for a surface facing away from the camera.
    pub fn back_alpha(&self) -> f64 {
        if self.enabled {
            self.back_face_alpha
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_globe_config_default() {
        let config = GlobeConfig::default();
        assert!(config.show);
        assert!(!config.depth_test_against_terrain);
        assert!(!config.translucency_enabled);
        assert!(config.show_ground_atmosphere);
        assert!((config.maximum_screen_space_error - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_globe_surface_normal() {
        let globe = GlobeSurface::new();
        // At the north pole, normal should point up (Z axis)
        let north_pole = DVec3::new(0.0, 0.0, 6356752.3142);
        let normal = globe.get_surface_normal(north_pole);
        assert!(normal.z > 0.99);
    }

    #[test]
    fn test_horizon_distance() {
        let globe = GlobeSurface::new();
        // At 1000m height, horizon should be ~113 km
        let dist = globe.horizon_distance(1000.0);
        assert!(dist > 100_000.0 && dist < 120_000.0);
    }

    #[test]
    fn test_horizon_distance_zero() {
        let globe = GlobeSurface::new();
        let dist = globe.horizon_distance(0.0);
        assert!(dist.abs() < 1e-6);
    }

    #[test]
    fn test_horizon_dip_angle() {
        let globe = GlobeSurface::new();
        // At 1000m, dip angle should be small (~0.03 rad)
        let dip = globe.horizon_dip_angle(1000.0);
        assert!(dip > 0.0 && dip < 0.1);
    }

    #[test]
    fn test_pick_globe_from_above() {
        let globe = GlobeSurface::new();
        // Ray from above looking down
        let origin = DVec3::new(0.0, 0.0, 10_000_000.0);
        let direction = DVec3::new(0.0, 0.0, -1.0);

        let hit = globe.pick(origin, direction);
        assert!(hit.is_some());

        let hit_point = hit.unwrap();
        // Should hit near the north pole surface
        assert!((hit_point.z - 6356752.3142).abs() < 1.0);
    }

    #[test]
    fn test_pick_globe_miss() {
        let globe = GlobeSurface::new();
        // Ray pointing away from Earth
        let origin = DVec3::new(0.0, 0.0, 10_000_000.0);
        let direction = DVec3::new(0.0, 0.0, 1.0);

        let hit = globe.pick(origin, direction);
        assert!(hit.is_none());
    }

    #[test]
    fn test_visible_hemisphere() {
        let globe = GlobeSurface::new();
        let camera = DVec3::new(0.0, 0.0, 10_000_000.0);

        // North pole should be visible
        let north_pole = DVec3::new(0.0, 0.0, 6356752.3142);
        assert!(globe.is_on_visible_hemisphere(north_pole, camera));

        // South pole should not be visible
        let south_pole = DVec3::new(0.0, 0.0, -6356752.3142);
        assert!(!globe.is_on_visible_hemisphere(south_pole, camera));
    }

    #[test]
    fn test_tile_sse_computation() {
        let globe = GlobeSurface::new();

        // High geometric error, close distance → high SSE
        let sse = globe.compute_tile_sse(1000.0, 1000.0, 1080.0, 1.0);
        assert!(sse > 100.0);

        // Low geometric error, far distance → low SSE
        let sse = globe.compute_tile_sse(1.0, 1_000_000.0, 1080.0, 1.0);
        assert!(sse < 1.0);
    }

    #[test]
    fn test_should_refine_tile() {
        let globe = GlobeSurface::new();
        assert!(globe.should_refine_tile(10.0)); // SSE > 2.0
        assert!(!globe.should_refine_tile(1.0)); // SSE < 2.0
    }

    #[test]
    fn test_globe_translucency() {
        let translucency = GlobeTranslucency::new(true);
        assert!(translucency.enabled);
        assert!((translucency.front_alpha() - 1.0).abs() < 1e-10);

        let mut t2 = GlobeTranslucency::new(true);
        t2.front_face_alpha = 0.5;
        assert!((t2.front_alpha() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_globe_translucency_disabled() {
        let translucency = GlobeTranslucency::default();
        assert!(!translucency.enabled);
        // When disabled, always returns 1.0
        assert!((translucency.front_alpha() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_get_height() {
        let globe = GlobeSurface::new();
        let carto = Cartographic::from_radians(0.0, 0.0, 0.0);
        let height = globe.get_height(&carto);
        assert_eq!(height, Some(0.0));
    }

    #[test]
    fn test_globe_surface_custom_ellipsoid() {
        let ellipsoid = Ellipsoid::new(1000.0, 1000.0, 1000.0);
        let globe = GlobeSurface::with_ellipsoid(ellipsoid);

        let dist = globe.horizon_distance(100.0);
        // Smaller ellipsoid → shorter horizon distance
        assert!(dist < 1000.0);
    }
}
