//! Shadow mapping with cascaded shadow maps (CSM).
//!
//! Maps to CesiumJS `Scene/ShadowMap.js`:
//! - Shadow map configuration
//! - Cascaded shadow mapping for directional lights
//! - Shadow bias and filtering
//! - Per-type bias (terrain/primitive/point)
//! - Point light cube map shadows
//! - Shadow fading near horizon
//! - PCF soft shadow filtering

use glam::{DMat4, DVec3};

/// Shadow map type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowMapType {
    /// Single shadow map (for point/spot lights).
    Single,
    /// Cascaded shadow maps (for directional lights like the sun).
    Cascaded,
}

/// Light source type for shadow mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShadowLightType {
    /// Directional light (sun) — uses cascaded shadow maps.
    #[default]
    Directional,
    /// Point light — uses cube map (6 faces).
    Point,
    /// Spot light — uses single perspective shadow map.
    Spot,
}

/// Per-type shadow bias configuration.
///
/// Maps to CesiumJS ShadowMap `_terrainBias`, `_primitiveBias`, `_pointBias`.
#[derive(Debug, Clone, PartialEq)]
pub struct ShadowBias {
    /// Whether polygon offset is enabled.
    pub polygon_offset: bool,
    /// Polygon offset factor.
    pub polygon_offset_factor: f64,
    /// Polygon offset units.
    pub polygon_offset_units: f64,
    /// Whether normal offset is enabled.
    pub normal_offset: bool,
    /// Normal offset scale.
    pub normal_offset_scale: f64,
    /// Whether normal shading is enabled.
    pub normal_shading: bool,
    /// Normal shading smoothness.
    pub normal_shading_smooth: f64,
    /// Depth bias.
    pub depth_bias: f64,
}

impl ShadowBias {
    /// Default bias for terrain rendering.
    pub fn terrain(normal_offset: bool) -> Self {
        Self {
            polygon_offset: true,
            polygon_offset_factor: 1.1,
            polygon_offset_units: 4.0,
            normal_offset,
            normal_offset_scale: 0.5,
            normal_shading: true,
            normal_shading_smooth: 0.3,
            depth_bias: 0.0001,
        }
    }

    /// Default bias for primitive (3D model) rendering.
    pub fn primitive(normal_offset: bool) -> Self {
        Self {
            polygon_offset: true,
            polygon_offset_factor: 1.1,
            polygon_offset_units: 4.0,
            normal_offset,
            normal_offset_scale: 0.1,
            normal_shading: true,
            normal_shading_smooth: 0.05,
            depth_bias: 0.00002,
        }
    }

    /// Default bias for point light rendering.
    pub fn point(normal_offset: bool) -> Self {
        Self {
            polygon_offset: false,
            polygon_offset_factor: 1.1,
            polygon_offset_units: 4.0,
            normal_offset,
            normal_offset_scale: 0.0,
            normal_shading: true,
            normal_shading_smooth: 0.1,
            depth_bias: 0.0005,
        }
    }

    /// Computes the effective bias for a given surface normal and light direction.
    pub fn compute_effective_bias(&self, normal: DVec3, light_dir: DVec3) -> f64 {
        let mut bias = self.depth_bias;
        if self.normal_offset {
            let n_dot_l = normal.dot(-light_dir).abs();
            let slope_factor = (1.0 - n_dot_l * n_dot_l).sqrt().max(0.0);
            bias += self.normal_offset_scale * slope_factor;
        }
        bias
    }
}

/// PCF (Percentage-Closer Filtering) configuration for soft shadows.
#[derive(Debug, Clone, PartialEq)]
pub struct PcfConfig {
    /// Whether PCF is enabled.
    pub enabled: bool,
    /// Kernel size (1, 3, 5, 7).
    pub kernel_size: u32,
    /// Whether to use Poisson disk sampling instead of grid.
    pub use_poisson_disk: bool,
}

impl Default for PcfConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            kernel_size: 3,
            use_poisson_disk: false,
        }
    }
}

impl PcfConfig {
    /// Computes the PCF filter result given depth comparisons.
    ///
    /// Returns shadow factor in [0.0, 1.0] (0 = fully shadowed, 1 = fully lit).
    pub fn filter(&self, depth_comparisons: &[f64]) -> f64 {
        if !self.enabled || depth_comparisons.is_empty() {
            return if depth_comparisons.first().copied().unwrap_or(1.0) >= 0.0 {
                1.0
            } else {
                0.0
            };
        }
        let lit_count = depth_comparisons.iter().filter(|&&d| d >= 0.0).count();
        lit_count as f64 / depth_comparisons.len() as f64
    }

    /// Generates PCF sample offsets for the configured kernel.
    pub fn sample_offsets(&self) -> Vec<[f64; 2]> {
        if self.use_poisson_disk {
            return poisson_disk_samples(self.kernel_size);
        }
        let half = (self.kernel_size / 2) as f64;
        let mut offsets = Vec::new();
        for y in 0..self.kernel_size {
            for x in 0..self.kernel_size {
                let ox = (x as f64 - half) / half.max(1.0);
                let oy = (y as f64 - half) / half.max(1.0);
                offsets.push([ox, oy]);
            }
        }
        offsets
    }
}

/// Generates Poisson disk sample offsets.
fn poisson_disk_samples(count: u32) -> Vec<[f64; 2]> {
    const POISSON_16: [[f64; 2]; 16] = [
        [-0.9420162, -0.3990622], [0.9455861, -0.7689072],
        [-0.0941841, -0.9293887], [0.3449594, 0.2938776],
        [-0.9158858, 0.4577143], [-0.8154423, -0.8791246],
        [-0.3827754, 0.2767685], [0.9748440, 0.7564838],
        [0.4432333, -0.9751155], [0.5374298, -0.4737342],
        [-0.2649691, -0.4189302], [0.7919751, 0.1909019],
        [-0.2418884, 0.9970651], [-0.8140996, 0.9143759],
        [0.1998413, 0.7864137], [0.1438316, -0.1410079],
    ];
    let n = (count as usize).clamp(1, 16);
    POISSON_16[..n].to_vec()
}

/// Shadow map configuration.
/// Maps to CesiumJS `ShadowMap` options
#[derive(Debug, Clone)]
pub struct ShadowMapConfig {
    /// Whether shadows are enabled.
    pub enabled: bool,
    /// Shadow map type.
    pub shadow_map_type: ShadowMapType,
    /// Light source type.
    pub light_type: ShadowLightType,
    /// Shadow map resolution (width = height).
    pub resolution: u32,
    /// Number of cascades for CSM.
    pub cascade_count: u32,
    /// Bias to reduce shadow acne.
    pub bias: f64,
    /// Normal offset bias.
    pub normal_bias: f64,
    /// Whether to use soft shadows (PCF).
    pub soft_shadows: bool,
    /// PCF configuration.
    pub pcf: PcfConfig,
    /// Darkness of shadows (0.0 = fully black, 1.0 = no shadow).
    pub darkness: f64,
    /// Whether the shadow map is fixed (doesn't update with camera).
    pub is_fixed: bool,
    /// Maximum distance for shadows.
    pub maximum_distance: f64,
    /// Whether normal offset is applied.
    pub normal_offset: bool,
    /// Whether shadows fade out near the horizon.
    pub fading_enabled: bool,
    /// Point light radius (for point lights).
    pub point_light_radius: f64,
    /// Maximum cascade distances [4 values].
    pub maximum_cascade_distances: [f64; 4],
}

impl Default for ShadowMapConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            shadow_map_type: ShadowMapType::Cascaded,
            light_type: ShadowLightType::Directional,
            resolution: 2048,
            cascade_count: 4,
            bias: 0.0005,
            normal_bias: 0.02,
            soft_shadows: true,
            pcf: PcfConfig::default(),
            darkness: 0.3,
            is_fixed: false,
            maximum_distance: 5000.0,
            normal_offset: true,
            fading_enabled: true,
            point_light_radius: 100.0,
            maximum_cascade_distances: [25.0, 150.0, 700.0, f64::MAX],
        }
    }
}

/// Camera parameters for shadow map computation.
#[derive(Debug, Clone, Copy)]
pub struct ShadowCameraParams {
    /// Camera position in world space.
    pub position: DVec3,
    /// Camera view direction.
    pub direction: DVec3,
    /// Camera up vector.
    pub up: DVec3,
    /// Vertical field of view (radians).
    pub fov_y: f64,
    /// Aspect ratio (width / height).
    pub aspect_ratio: f64,
}

/// A single cascade in the cascaded shadow map.
#[derive(Debug, Clone)]
pub struct ShadowCascade {
    /// The light view-projection matrix for this cascade.
    pub light_view_projection: DMat4,
    /// The split distance (near plane of this cascade in view space).
    pub split_near: f64,
    /// The far plane of this cascade in view space.
    pub split_far: f64,
    /// The texel size (world units per texel).
    pub texel_size: f64,
}

/// The shadow map state.
#[derive(Debug, Clone)]
pub struct ShadowMap {
    /// Configuration.
    pub config: ShadowMapConfig,
    /// Light direction (normalized, pointing from light to scene).
    pub light_direction: DVec3,
    /// Light position (for point/spot lights).
    pub light_position: DVec3,
    /// The cascades (for CSM).
    pub cascades: Vec<ShadowCascade>,
    /// Whether the shadow map needs to be updated.
    pub needs_update: bool,
    /// Per-type bias configurations.
    pub terrain_bias: ShadowBias,
    /// Primitive bias.
    pub primitive_bias: ShadowBias,
    /// Point light bias.
    pub point_bias: ShadowBias,
    /// Current fade factor (1.0 = no fade, 0.0 = fully faded).
    pub fade_factor: f64,
    /// Whether the light is out of view.
    pub out_of_view: bool,
}

/// Global maximum shadow distance.
pub const SHADOW_MAP_MAXIMUM_DISTANCE: f64 = 20000.0;

impl ShadowMap {
    /// Creates a new shadow map.
    pub fn new(config: ShadowMapConfig, light_direction: DVec3) -> Self {
        let normal_offset = config.normal_offset;
        Self {
            config,
            light_direction: light_direction.normalize(),
            light_position: DVec3::ZERO,
            cascades: Vec::new(),
            needs_update: true,
            terrain_bias: ShadowBias::terrain(normal_offset),
            primitive_bias: ShadowBias::primitive(normal_offset),
            point_bias: ShadowBias::point(normal_offset),
            fade_factor: 1.0,
            out_of_view: false,
        }
    }

    /// Creates a shadow map for the sun.
    pub fn for_sun(sun_direction: DVec3) -> Self {
        Self::new(ShadowMapConfig::default(), -sun_direction)
    }

    /// Creates a shadow map for a point light.
    pub fn for_point_light(position: DVec3, radius: f64) -> Self {
        let config = ShadowMapConfig {
            shadow_map_type: ShadowMapType::Single,
            light_type: ShadowLightType::Point,
            cascade_count: 0,
            point_light_radius: radius,
            ..Default::default()
        };
        let mut map = Self::new(config, DVec3::ZERO);
        map.light_position = position;
        map
    }

    /// Creates a shadow map for a spot light.
    pub fn for_spot_light(position: DVec3, direction: DVec3) -> Self {
        let config = ShadowMapConfig {
            shadow_map_type: ShadowMapType::Single,
            light_type: ShadowLightType::Spot,
            cascade_count: 0,
            ..Default::default()
        };
        let mut map = Self::new(config, direction.normalize());
        map.light_position = position;
        map
    }

    /// Computes the shadow fade factor based on light elevation.
    ///
    /// Shadows fade out as the light approaches the horizon.
    ///
    /// # Arguments
    /// * `light_elevation` - Light elevation angle in radians (0 = horizon, π/2 = overhead)
    pub fn compute_fade_factor(&self, light_elevation: f64) -> f64 {
        if !self.config.fading_enabled {
            return 1.0;
        }

        // Fade starts at ~10 degrees above horizon, fully faded at horizon
        let fade_start = 10.0_f64.to_radians();
        let fade_end = 0.0_f64.to_radians();

        if light_elevation >= fade_start {
            1.0
        } else if light_elevation <= fade_end {
            0.0
        } else {
            (light_elevation - fade_end) / (fade_start - fade_end)
        }
    }

    /// Updates the fade factor based on light elevation.
    pub fn update_fade(&mut self, light_elevation: f64) {
        self.fade_factor = self.compute_fade_factor(light_elevation);
    }

    /// Returns the number of shadow passes required.
    pub fn pass_count(&self) -> usize {
        match self.config.light_type {
            ShadowLightType::Point => 6, // Cube map: 6 faces
            ShadowLightType::Spot => 1,
            ShadowLightType::Directional => {
                if self.config.cascade_count > 0 {
                    self.config.cascade_count as usize
                } else {
                    1
                }
            }
        }
    }

    /// Returns the bias configuration for a given receiver type.
    pub fn bias_for_type(&self, receiver_type: ShadowBiasType) -> &ShadowBias {
        match receiver_type {
            ShadowBiasType::Terrain => &self.terrain_bias,
            ShadowBiasType::Primitive => &self.primitive_bias,
            ShadowBiasType::Point => &self.point_bias,
        }
    }

    /// Computes the cascade splits using practical split scheme.
    ///
    /// # Arguments
    /// * `near` - Camera near plane
    /// * `far` - Camera far plane (or maximum shadow distance)
    /// * `lambda` - Blend factor between logarithmic (0.0) and uniform (1.0) splits
    pub fn compute_cascade_splits(&self, near: f64, far: f64, lambda: f64) -> Vec<f64> {
        let count = self.config.cascade_count as usize;
        let mut splits = Vec::with_capacity(count + 1);

        splits.push(near);

        for i in 1..count {
            let t = i as f64 / count as f64;

            // Logarithmic split
            let log_split = near * (far / near).powf(t);

            // Uniform split
            let uniform_split = near + (far - near) * t;

            // Blend between logarithmic and uniform
            let split = lambda * log_split + (1.0 - lambda) * uniform_split;
            splits.push(split);
        }

        splits.push(far);
        splits
    }

    /// Computes the light view-projection matrix for a cascade.
    ///
    /// # Arguments
    /// * `camera` - Camera parameters
    /// * `cascade_near` - Near plane distance for this cascade
    /// * `cascade_far` - Far plane distance for this cascade
    pub fn compute_cascade_matrix(
        &self,
        camera: &ShadowCameraParams,
        cascade_near: f64,
        cascade_far: f64,
    ) -> DMat4 {
        // Compute the frustum corners in world space
        let corners = compute_frustum_corners(
            camera.position,
            camera.direction,
            camera.up,
            cascade_near,
            cascade_far,
            camera.fov_y,
            camera.aspect_ratio,
        );

        // Compute the centroid of the frustum
        let centroid = corners.iter().sum::<DVec3>() / 8.0;

        // Light view matrix (looking from light direction)
        let light_right = self.light_direction.cross(DVec3::Y).normalize();
        let light_up = light_right.cross(self.light_direction).normalize();

        let light_view = look_at_matrix(centroid - self.light_direction * 1000.0, centroid, light_up);

        // Transform corners to light space
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut min_z = f64::INFINITY;
        let mut max_z = f64::NEG_INFINITY;

        for corner in &corners {
            let transformed = light_view.transform_point3(*corner);
            min_x = min_x.min(transformed.x);
            max_x = max_x.max(transformed.x);
            min_y = min_y.min(transformed.y);
            max_y = max_y.max(transformed.y);
            min_z = min_z.min(transformed.z);
            max_z = max_z.max(transformed.z);
        }

        // Add some padding
        let padding = 10.0;
        min_x -= padding;
        max_x += padding;
        min_y -= padding;
        max_y += padding;
        min_z -= padding;
        max_z += padding;

        // Orthographic projection
        let light_projection = orthographic_matrix(min_x, max_x, min_y, max_y, min_z, max_z);

        light_projection * light_view
    }

    /// Updates the cascades based on the camera.
    pub fn update_cascades(
        &mut self,
        camera: &ShadowCameraParams,
        near: f64,
        far: f64,
    ) {
        if !self.config.enabled {
            return;
        }

        let effective_far = far.min(self.config.maximum_distance);
        let splits = self.compute_cascade_splits(near, effective_far, 0.5);

        self.cascades.clear();

        for i in 0..self.config.cascade_count as usize {
            let cascade_near = splits[i];
            let cascade_far = splits[i + 1];

            let light_vp = self.compute_cascade_matrix(
                camera,
                cascade_near,
                cascade_far,
            );

            let texel_size = (cascade_far - cascade_near) / self.config.resolution as f64;

            self.cascades.push(ShadowCascade {
                light_view_projection: light_vp,
                split_near: cascade_near,
                split_far: cascade_far,
                texel_size,
            });
        }

        self.needs_update = false;
    }

    /// Computes the shadow factor for a world position.
    ///
    /// Returns a value in [darkness, 1.0] where darkness = fully shadowed.
    pub fn compute_shadow_factor(&self, _world_position: DVec3, view_depth: f64) -> f64 {
        if !self.config.enabled || self.cascades.is_empty() {
            return 1.0;
        }

        // Find the appropriate cascade
        let cascade = self.cascades.iter().find(|c| {
            view_depth >= c.split_near && view_depth < c.split_far
        });

        match cascade {
            Some(_) => {
                // In a real implementation, we would sample the shadow map here.
                // For now, return a placeholder based on darkness.
                // The actual shadow lookup would compare depth with the shadow map.
                1.0 // Placeholder: no shadow (would need actual depth comparison)
            }
            None => 1.0, // Outside shadow range
        }
    }

    /// Applies shadow to a color.
    pub fn apply_shadow(&self, color: DVec3, shadow_factor: f64) -> DVec3 {
        let effective_factor = shadow_factor * self.fade_factor;
        let factor = self.config.darkness + (1.0 - self.config.darkness) * effective_factor;
        color * factor
    }
}

/// Receiver type for bias selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowBiasType {
    /// Terrain receiver.
    Terrain,
    /// Primitive (3D model) receiver.
    Primitive,
    /// Point light receiver.
    Point,
}

/// Computes the 8 corners of a view frustum slice.
fn compute_frustum_corners(
    camera_position: DVec3,
    camera_direction: DVec3,
    camera_up: DVec3,
    near: f64,
    far: f64,
    fov_y: f64,
    aspect_ratio: f64,
) -> [DVec3; 8] {
    let camera_right = camera_direction.cross(camera_up).normalize();
    let camera_up = camera_right.cross(camera_direction).normalize();

    let near_height = 2.0 * (fov_y / 2.0).tan() * near;
    let near_width = near_height * aspect_ratio;

    let far_height = 2.0 * (fov_y / 2.0).tan() * far;
    let far_width = far_height * aspect_ratio;

    let near_center = camera_position + camera_direction * near;
    let far_center = camera_position + camera_direction * far;

    let near_up = camera_up * (near_height / 2.0);
    let near_right = camera_right * (near_width / 2.0);

    let far_up = camera_up * (far_height / 2.0);
    let far_right = camera_right * (far_width / 2.0);

    [
        // Near plane corners
        near_center - near_right + near_up,
        near_center + near_right + near_up,
        near_center + near_right - near_up,
        near_center - near_right - near_up,
        // Far plane corners
        far_center - far_right + far_up,
        far_center + far_right + far_up,
        far_center + far_right - far_up,
        far_center - far_right - far_up,
    ]
}

/// Creates a look-at view matrix.
fn look_at_matrix(eye: DVec3, target: DVec3, up: DVec3) -> DMat4 {
    let z = (eye - target).normalize();
    let x = up.cross(z).normalize();
    let y = z.cross(x);

    DMat4::from_cols_array(&[
        x.x, y.x, z.x, 0.0,
        x.y, y.y, z.y, 0.0,
        x.z, y.z, z.z, 0.0,
        -x.dot(eye), -y.dot(eye), -z.dot(eye), 1.0,
    ])
}

/// Creates an orthographic projection matrix.
fn orthographic_matrix(left: f64, right: f64, bottom: f64, top: f64, near: f64, far: f64) -> DMat4 {
    let rcp_width = 1.0 / (right - left);
    let rcp_height = 1.0 / (top - bottom);
    let rcp_depth = 1.0 / (far - near);

    DMat4::from_cols_array(&[
        2.0 * rcp_width, 0.0, 0.0, 0.0,
        0.0, 2.0 * rcp_height, 0.0, 0.0,
        0.0, 0.0, -2.0 * rcp_depth, 0.0,
        -(right + left) * rcp_width, -(top + bottom) * rcp_height, -(far + near) * rcp_depth, 1.0,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_4;

    #[test]
    fn test_shadow_map_creation() {
        let shadow_map = ShadowMap::new(ShadowMapConfig::default(), DVec3::new(0.0, -1.0, 0.0));

        assert!(shadow_map.config.enabled);
        assert_eq!(shadow_map.config.cascade_count, 4);
        assert!(shadow_map.needs_update);
    }

    #[test]
    fn test_shadow_map_for_sun() {
        let sun_direction = DVec3::new(0.5, -0.7, 0.3).normalize();
        let shadow_map = ShadowMap::for_sun(sun_direction);

        // Light direction should be opposite to sun direction
        assert!((shadow_map.light_direction - (-sun_direction)).length() < 1e-10);
    }

    #[test]
    fn test_cascade_splits_count() {
        let shadow_map = ShadowMap::new(ShadowMapConfig::default(), DVec3::new(0.0, -1.0, 0.0));

        let splits = shadow_map.compute_cascade_splits(0.1, 1000.0, 0.5);

        // Should have cascade_count + 1 splits
        assert_eq!(splits.len(), 5); // 4 cascades = 5 splits
        assert!((splits[0] - 0.1).abs() < 1e-10);
        assert!((splits[4] - 1000.0).abs() < 1e-10);
    }

    #[test]
    fn test_cascade_splits_monotonic() {
        let shadow_map = ShadowMap::new(ShadowMapConfig::default(), DVec3::new(0.0, -1.0, 0.0));

        let splits = shadow_map.compute_cascade_splits(0.1, 1000.0, 0.5);

        for i in 1..splits.len() {
            assert!(splits[i] > splits[i - 1]);
        }
    }

    #[test]
    fn test_cascade_splits_lambda() {
        let shadow_map = ShadowMap::new(ShadowMapConfig::default(), DVec3::new(0.0, -1.0, 0.0));

        // Lambda = 0.0: uniform splits
        let uniform = shadow_map.compute_cascade_splits(0.1, 1000.0, 0.0);
        let expected_uniform = 0.1 + (1000.0 - 0.1) * 0.25;
        assert!((uniform[1] - expected_uniform).abs() < 1.0);

        // Lambda = 1.0: logarithmic splits
        let logarithmic = shadow_map.compute_cascade_splits(0.1, 1000.0, 1.0);
        let expected_log = 0.1_f64 * (1000.0_f64 / 0.1_f64).powf(0.25);
        assert!((logarithmic[1] - expected_log).abs() < 0.1);
    }

    #[test]
    fn test_update_cascades() {
        let mut shadow_map = ShadowMap::new(ShadowMapConfig::default(), DVec3::new(0.0, -1.0, 0.0));

        let camera = ShadowCameraParams {
            position: DVec3::ZERO,
            direction: DVec3::new(0.0, 0.0, -1.0),
            up: DVec3::Y,
            fov_y: FRAC_PI_4,
            aspect_ratio: 16.0 / 9.0,
        };

        shadow_map.update_cascades(&camera, 0.1, 1000.0);

        assert_eq!(shadow_map.cascades.len(), 4);
        assert!(!shadow_map.needs_update);
    }

    #[test]
    fn test_cascade_texel_size() {
        let mut shadow_map = ShadowMap::new(ShadowMapConfig::default(), DVec3::new(0.0, -1.0, 0.0));

        let camera = ShadowCameraParams {
            position: DVec3::ZERO,
            direction: DVec3::new(0.0, 0.0, -1.0),
            up: DVec3::Y,
            fov_y: FRAC_PI_4,
            aspect_ratio: 16.0 / 9.0,
        };

        shadow_map.update_cascades(&camera, 0.1, 1000.0);

        for cascade in &shadow_map.cascades {
            assert!(cascade.texel_size > 0.0);
        }
    }

    #[test]
    fn test_shadow_factor_no_cascades() {
        let shadow_map = ShadowMap::new(ShadowMapConfig::default(), DVec3::new(0.0, -1.0, 0.0));

        // No cascades computed yet
        let factor = shadow_map.compute_shadow_factor(DVec3::ZERO, 100.0);
        assert_eq!(factor, 1.0);
    }

    #[test]
    fn test_shadow_disabled() {
        let config = ShadowMapConfig {
            enabled: false,
            ..Default::default()
        };
        let shadow_map = ShadowMap::new(config, DVec3::new(0.0, -1.0, 0.0));

        let factor = shadow_map.compute_shadow_factor(DVec3::ZERO, 100.0);
        assert_eq!(factor, 1.0);
    }

    #[test]
    fn test_apply_shadow() {
        let shadow_map = ShadowMap::new(ShadowMapConfig::default(), DVec3::new(0.0, -1.0, 0.0));

        let color = DVec3::new(1.0, 1.0, 1.0);

        // Full shadow (factor = 0.0)
        let shadowed = shadow_map.apply_shadow(color, 0.0);
        assert!((shadowed.x - shadow_map.config.darkness).abs() < 1e-10);

        // No shadow (factor = 1.0)
        let lit = shadow_map.apply_shadow(color, 1.0);
        assert!((lit.x - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_frustum_corners() {
        let corners = compute_frustum_corners(
            DVec3::ZERO,
            DVec3::new(0.0, 0.0, -1.0),
            DVec3::Y,
            1.0,
            10.0,
            FRAC_PI_4,
            1.0,
        );

        // All corners should be in front of the camera (negative Z)
        for corner in &corners {
            assert!(corner.z < 0.0);
        }
    }

    #[test]
    fn test_look_at_matrix() {
        let matrix = look_at_matrix(
            DVec3::new(0.0, 0.0, 5.0),
            DVec3::ZERO,
            DVec3::Y,
        );

        // Origin should transform to (0, 0, -5)
        let transformed = matrix.transform_point3(DVec3::ZERO);
        assert!((transformed.x).abs() < 1e-10);
        assert!((transformed.y).abs() < 1e-10);
        assert!((transformed.z - (-5.0)).abs() < 1e-10);
    }

    #[test]
    fn test_orthographic_matrix() {
        let matrix = orthographic_matrix(-1.0, 1.0, -1.0, 1.0, 0.1, 100.0);

        // Center should map to (0, 0, z)
        let center = matrix.transform_point3(DVec3::ZERO);
        assert!((center.x).abs() < 1e-10);
        assert!((center.y).abs() < 1e-10);
    }
}
