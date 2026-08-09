use bevy::prelude::*;
use cesium_shadow::{
    PcfConfig, ShadowCameraParams,
    ShadowLightType, ShadowMap, ShadowMapConfig, ShadowMapType,
};
use glam::DVec3;

#[derive(Resource, Debug, Clone)]
pub struct ShadowConfig {
    pub enabled: bool,
    pub cascade_count: u32,
    pub max_distance: f64,
    pub bias: f64,
    pub normal_bias: f64,
    pub soft_shadows: bool,
    pub pcf_kernel_size: u32,
    pub darkness: f64,
    pub resolution: u32,
    pub fading_enabled: bool,
}

impl Default for ShadowConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cascade_count: 4,
            max_distance: 5000.0,
            bias: 0.0005,
            normal_bias: 0.02,
            soft_shadows: true,
            pcf_kernel_size: 3,
            darkness: 0.3,
            resolution: 2048,
            fading_enabled: true,
        }
    }
}

impl ShadowConfig {
    pub fn to_domain_config(&self) -> ShadowMapConfig {
        ShadowMapConfig {
            enabled: self.enabled,
            shadow_map_type: ShadowMapType::Cascaded,
            light_type: ShadowLightType::Directional,
            resolution: self.resolution,
            cascade_count: self.cascade_count,
            bias: self.bias,
            normal_bias: self.normal_bias,
            soft_shadows: self.soft_shadows,
            pcf: PcfConfig {
                enabled: self.soft_shadows,
                kernel_size: self.pcf_kernel_size,
                use_poisson_disk: false,
            },
            darkness: self.darkness,
            is_fixed: false,
            maximum_distance: self.max_distance,
            normal_offset: true,
            fading_enabled: self.fading_enabled,
            point_light_radius: 100.0,
            maximum_cascade_distances: [25.0, 150.0, 700.0, f64::MAX],
        }
    }
}

#[derive(Resource, Clone)]
pub struct ShadowState {
    pub needs_update: bool,
    pub cascades: Vec<([f32; 16], f32, f32)>,
    pub fade_factor: f64,
    pub light_direction: DVec3,
}

impl Default for ShadowState {
    fn default() -> Self {
        Self {
            needs_update: true,
            cascades: Vec::new(),
            fade_factor: 1.0,
            light_direction: DVec3::new(0.5, -1.0, 0.5).normalize(),
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct ShadowCaster;

pub struct CesiumShadowPlugin;

impl Plugin for CesiumShadowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShadowConfig>()
            .init_resource::<ShadowState>()
            .add_systems(Update, shadow_update_system);
    }
}

pub fn shadow_update_system(
    config: Res<ShadowConfig>,
    mut state: ResMut<ShadowState>,
    directional_light_query: Query<&Transform, With<DirectionalLight>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) {
    if !config.enabled {
        return;
    }

    if let Ok(light_transform) = directional_light_query.get_single() {
        let light_dir = DVec3::new(
            light_transform.forward().x as f64,
            light_transform.forward().y as f64,
            light_transform.forward().z as f64,
        )
        .normalize();
        state.light_direction = -light_dir;
    }

    if let Ok((_camera, cam_transform)) = camera_query.get_single() {
        let cam_pos = DVec3::new(
            cam_transform.translation().x as f64,
            cam_transform.translation().y as f64,
            cam_transform.translation().z as f64,
        );
        let cam_forward = DVec3::new(
            cam_transform.forward().x as f64,
            cam_transform.forward().y as f64,
            cam_transform.forward().z as f64,
        );
        let cam_up = DVec3::new(
            cam_transform.up().x as f64,
            cam_transform.up().y as f64,
            cam_transform.up().z as f64,
        );

        let domain_config = config.to_domain_config();
        let mut shadow_map = ShadowMap::new(domain_config, state.light_direction);

        let fov_y = 60.0_f64.to_radians();
        let aspect = 16.0 / 9.0;

        let shadow_cam = ShadowCameraParams {
            position: cam_pos,
            direction: cam_forward,
            up: cam_up,
            fov_y,
            aspect_ratio: aspect,
        };

        let near = 0.1;
        let far = config.max_distance;

        shadow_map.update_cascades(&shadow_cam, near, far);

        state.cascades.clear();
        for cascade in &shadow_map.cascades {
            let mat: [[f64; 4]; 4] = cascade.light_view_projection.to_cols_array_2d();
            let f32_mat: [f32; 16] = [
                mat[0][0] as f32,
                mat[0][1] as f32,
                mat[0][2] as f32,
                mat[0][3] as f32,
                mat[1][0] as f32,
                mat[1][1] as f32,
                mat[1][2] as f32,
                mat[1][3] as f32,
                mat[2][0] as f32,
                mat[2][1] as f32,
                mat[2][2] as f32,
                mat[2][3] as f32,
                mat[3][0] as f32,
                mat[3][1] as f32,
                mat[3][2] as f32,
                mat[3][3] as f32,
            ];
            state.cascades.push((
                f32_mat,
                cascade.split_near as f32,
                cascade.split_far as f32,
            ));
        }

        let light_elevation = state.light_direction.y.max(0.0).asin();
        shadow_map.update_fade(light_elevation);
        state.fade_factor = shadow_map.fade_factor;

        state.needs_update = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_config_default() {
        let config = ShadowConfig::default();
        assert!(config.enabled);
        assert_eq!(config.cascade_count, 4);
        assert_eq!(config.max_distance, 5000.0);
        assert!(config.soft_shadows);
        assert_eq!(config.pcf_kernel_size, 3);
    }

    #[test]
    fn test_shadow_config_to_domain() {
        let config = ShadowConfig::default();
        let domain = config.to_domain_config();
        assert!(domain.enabled);
        assert_eq!(domain.cascade_count, 4);
        assert_eq!(domain.resolution, 2048);
        assert!(domain.soft_shadows);
        assert_eq!(domain.pcf.kernel_size, 3);
    }

    #[test]
    fn test_shadow_state_default() {
        let state = ShadowState::default();
        assert!(state.needs_update);
        assert!(state.cascades.is_empty());
        assert_eq!(state.fade_factor, 1.0);
        assert!(state.light_direction.length() > 0.0);
    }

    #[test]
    fn test_pcf_filter() {
        let pcf = PcfConfig {
            enabled: true,
            kernel_size: 3,
            use_poisson_disk: false,
        };
        let comparisons = vec![1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let factor = pcf.filter(&comparisons);
        assert!(factor > 0.8);

        let all_shadowed = vec![-1.0; 9];
        assert_eq!(pcf.filter(&all_shadowed), 0.0);
    }
}
