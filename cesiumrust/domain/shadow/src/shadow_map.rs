//! Shadow mapping with cascaded shadow maps (CSM).
//!
//! Maps to CesiumJS `Scene/ShadowMap.js`:
//! - Shadow map configuration
//! - Cascaded shadow mapping for directional lights
//! - Shadow bias and filtering

use glam::{DMat4, DVec3};

/// Shadow map type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowMapType {
    /// Single shadow map (for point/spot lights).
    Single,
    /// Cascaded shadow maps (for directional lights like the sun).
    Cascaded,
}

/// Shadow map configuration.
/// Maps to CesiumJS `ShadowMap` options
#[derive(Debug, Clone)]
pub struct ShadowMapConfig {
    /// Whether shadows are enabled.
    pub enabled: bool,
    /// Shadow map type.
    pub shadow_map_type: ShadowMapType,
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
    /// PCF kernel size (for soft shadows).
    pub pcf_kernel_size: u32,
    /// Darkness of shadows (0.0 = fully black, 1.0 = no shadow).
    pub darkness: f64,
    /// Whether the shadow map is fixed (doesn't update with camera).
    pub is_fixed: bool,
    /// Maximum distance for shadows.
    pub maximum_distance: f64,
}

impl Default for ShadowMapConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            shadow_map_type: ShadowMapType::Cascaded,
            resolution: 2048,
            cascade_count: 4,
            bias: 0.0005,
            normal_bias: 0.02,
            soft_shadows: true,
            pcf_kernel_size: 3,
            darkness: 0.3,
            is_fixed: false,
            maximum_distance: 10000.0,
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
}

impl ShadowMap {
    /// Creates a new shadow map.
    pub fn new(config: ShadowMapConfig, light_direction: DVec3) -> Self {
        Self {
            config,
            light_direction: light_direction.normalize(),
            light_position: DVec3::ZERO,
            cascades: Vec::new(),
            needs_update: true,
        }
    }

    /// Creates a shadow map for the sun.
    pub fn for_sun(sun_direction: DVec3) -> Self {
        Self::new(ShadowMapConfig::default(), -sun_direction)
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
        let factor = self.config.darkness + (1.0 - self.config.darkness) * shadow_factor;
        color * factor
    }
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
