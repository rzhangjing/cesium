use bevy::prelude::*;
use cesium_effects::post_process::{
    AmbientOcclusionConfig, BloomConfig, ColorCorrectionConfig, FogConfig, PostProcessPipeline,
    PostProcessStageType, ToneMappingConfig,
};
#[allow(unused_imports)]
use glam::DVec3;

#[derive(Resource, Debug, Clone)]
pub struct PostProcessConfig {
    pub fog_enabled: bool,
    pub tone_mapping_enabled: bool,
    pub bloom_enabled: bool,
    pub ambient_occlusion_enabled: bool,
    pub fxaa_enabled: bool,
    pub color_correction_enabled: bool,
    pub height_fog_enabled: bool,
    pub fog: FogConfig,
    pub tone_mapping: ToneMappingConfig,
    pub bloom: BloomConfig,
    pub ambient_occlusion: AmbientOcclusionConfig,
    pub color_correction: ColorCorrectionConfig,
    pub height_fog_base: f64,
    pub height_fog_falloff: f64,
}

impl Default for PostProcessConfig {
    fn default() -> Self {
        Self {
            fog_enabled: true,
            tone_mapping_enabled: true,
            bloom_enabled: false,
            ambient_occlusion_enabled: false,
            fxaa_enabled: false,
            color_correction_enabled: false,
            height_fog_enabled: false,
            fog: FogConfig::default(),
            tone_mapping: ToneMappingConfig::default(),
            bloom: BloomConfig::default(),
            ambient_occlusion: AmbientOcclusionConfig::default(),
            color_correction: ColorCorrectionConfig::default(),
            height_fog_base: 0.0,
            height_fog_falloff: 0.001,
        }
    }
}

impl PostProcessConfig {
    pub fn to_pipeline(&self) -> PostProcessPipeline {
        PostProcessPipeline {
            bloom: self.bloom.clone(),
            ambient_occlusion: self.ambient_occlusion.clone(),
            fog: self.fog.clone(),
            tone_mapping: self.tone_mapping.clone(),
            color_correction: self.color_correction.clone(),
        }
    }

    pub fn enabled_stages(&self) -> Vec<PostProcessStageType> {
        let mut stages = Vec::new();

        if self.ambient_occlusion_enabled {
            stages.push(PostProcessStageType::AmbientOcclusion);
        }
        if self.bloom_enabled {
            stages.push(PostProcessStageType::Bloom);
        }
        if self.fog_enabled || self.height_fog_enabled {
            stages.push(PostProcessStageType::Fog);
        }
        if self.color_correction_enabled {
            stages.push(PostProcessStageType::ColorCorrection);
        }
        if self.tone_mapping_enabled {
            stages.push(PostProcessStageType::ToneMapping);
        }

        stages
    }

    pub fn compute_height_fog(&self, height: f64) -> f64 {
        if !self.height_fog_enabled {
            return 0.0;
        }
        let relative_height = height - self.height_fog_base;
        let fog = 1.0 - (-self.height_fog_falloff * relative_height.max(0.0)).exp();
        fog.clamp(0.0, 1.0)
    }
}

pub fn fog_system(
    config: Res<PostProcessConfig>,
    mut clear_color: ResMut<ClearColor>,
    camera_query: Query<&Transform, With<Camera3d>>,
) {
    if !config.fog_enabled {
        return;
    }

    if let Ok(cam_transform) = camera_query.get_single() {
        let cam_pos = cam_transform.translation;
        let distance = cam_pos.length() as f64;

        let fog_factor = config.fog.compute_fog_factor(distance);
        let height_fog = config.compute_height_fog(cam_pos.y as f64);
        let combined_fog = (fog_factor + height_fog * (1.0 - fog_factor)).min(1.0);

        let fog_color = Vec3::new(
            config.fog.color.x as f32,
            config.fog.color.y as f32,
            config.fog.color.z as f32,
        );

        let current = clear_color.0.to_linear();
        let current_vec = Vec3::new(current.red, current.green, current.blue);
        let blended = current_vec.lerp(fog_color, combined_fog as f32);

        clear_color.0 = Color::linear_rgb(
            blended.x.clamp(0.0, 1.0),
            blended.y.clamp(0.0, 1.0),
            blended.z.clamp(0.0, 1.0),
        );
    }
}

pub fn bloom_system(
    _config: Res<PostProcessConfig>,
) {
}

pub fn ambient_occlusion_system(
    _config: Res<PostProcessConfig>,
) {
}

pub fn fxaa_system(
    _config: Res<PostProcessConfig>,
) {
}

pub fn color_correction_system(
    _config: Res<PostProcessConfig>,
) {
}

pub fn tone_mapping_system(
    _config: Res<PostProcessConfig>,
) {
}

pub fn post_process_system(
    config: Res<PostProcessConfig>,
    clear_color: ResMut<ClearColor>,
    camera_query: Query<&Transform, With<Camera3d>>,
) {
    fog_system_inner(&config, clear_color, camera_query);
}

fn fog_system_inner(
    config: &PostProcessConfig,
    mut clear_color: ResMut<ClearColor>,
    camera_query: Query<&Transform, With<Camera3d>>,
) {
    if !config.fog_enabled {
        return;
    }

    if let Ok(cam_transform) = camera_query.get_single() {
        let cam_pos = cam_transform.translation;
        let distance = cam_pos.length() as f64;

        let fog_factor = config.fog.compute_fog_factor(distance);
        let height_fog = config.compute_height_fog(cam_pos.y as f64);
        let combined_fog = (fog_factor + height_fog * (1.0 - fog_factor)).min(1.0);

        let fog_color = Vec3::new(
            config.fog.color.x as f32,
            config.fog.color.y as f32,
            config.fog.color.z as f32,
        );

        let current = clear_color.0.to_linear();
        let current_vec = Vec3::new(current.red, current.green, current.blue);
        let blended = current_vec.lerp(fog_color, combined_fog as f32);

        clear_color.0 = Color::linear_rgb(
            blended.x.clamp(0.0, 1.0),
            blended.y.clamp(0.0, 1.0),
            blended.z.clamp(0.0, 1.0),
        );
    }
}

pub struct CesiumEffectsPlugin;

impl Plugin for CesiumEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PostProcessConfig>()
            .add_systems(Update, post_process_system);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_effects::post_process::ToneMappingOperator;

    #[test]
    fn test_post_process_config_default() {
        let cfg = PostProcessConfig::default();
        assert!(cfg.fog_enabled);
        assert!(cfg.tone_mapping_enabled);
        assert!(!cfg.bloom_enabled);
        assert!(!cfg.ambient_occlusion_enabled);
        assert!(!cfg.fxaa_enabled);
        assert!(!cfg.color_correction_enabled);
    }

    #[test]
    fn test_post_process_config_disable() {
        let cfg = PostProcessConfig {
            fog_enabled: false,
            tone_mapping_enabled: false,
            ..Default::default()
        };
        assert!(!cfg.fog_enabled);
        assert!(!cfg.tone_mapping_enabled);
    }

    #[test]
    fn test_fog_factor_at_ground() {
        let fog = FogConfig::default();
        let factor = fog.compute_fog_factor(0.0);
        assert!((factor - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_fog_factor_far() {
        let fog = FogConfig {
            enabled: true,
            density: 2.0e-4,
            ..Default::default()
        };
        let factor = fog.compute_fog_factor(50000.0);
        assert!(factor > 0.9, "Expect heavy fog at 50km");
    }

    #[test]
    fn test_fog_disabled() {
        let fog = FogConfig {
            enabled: false,
            density: 2.0e-4,
            ..Default::default()
        };
        assert!((fog.compute_fog_factor(50000.0)).abs() < 1e-10);
    }

    #[test]
    fn test_tone_mapping_reinhard() {
        let config = ToneMappingConfig {
            operator: ToneMappingOperator::Reinhard,
            exposure: 1.0,
            white_point: 100.0,
        };
        let hdr = glam::DVec3::new(2.0, 2.0, 2.0);
        let ldr = config.apply(hdr);
        assert!((ldr.x - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_tone_mapping_aces() {
        let config = ToneMappingConfig {
            operator: ToneMappingOperator::AcesFilmic,
            exposure: 1.0,
            white_point: 1.0,
        };
        let hdr = glam::DVec3::new(1.0, 1.0, 1.0);
        let ldr = config.apply(hdr);
        assert!(ldr.x < 1.0);
        assert!(ldr.x > 0.0);
    }

    #[test]
    fn test_tone_mapping_none() {
        let config = ToneMappingConfig {
            operator: ToneMappingOperator::None,
            exposure: 1.0,
            white_point: 1.0,
        };
        let hdr = glam::DVec3::new(0.5, 0.7, 0.9);
        let ldr = config.apply(hdr);
        assert!((hdr - ldr).length() < 1e-10);
    }

    #[test]
    fn test_bloom_disabled_by_default() {
        let cfg = PostProcessConfig::default();
        assert!(!cfg.bloom_enabled);
        assert!(!cfg.bloom.enabled);
        assert_eq!(cfg.bloom.compute_bloom(10.0), 0.0);
    }

    #[test]
    fn test_bloom_enabled() {
        let cfg = PostProcessConfig {
            bloom_enabled: true,
            bloom: BloomConfig {
                enabled: true,
                threshold: 0.8,
                intensity: 1.0,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(cfg.bloom.compute_bloom(0.5), 0.0);
        assert!((cfg.bloom.compute_bloom(1.0) - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_ambient_occlusion_disabled_by_default() {
        let cfg = PostProcessConfig::default();
        assert!(!cfg.ambient_occlusion_enabled);
    }

    #[test]
    fn test_ambient_occlusion_partial() {
        let cfg = PostProcessConfig {
            ambient_occlusion_enabled: true,
            ambient_occlusion: AmbientOcclusionConfig {
                enabled: true,
                intensity: 1.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let result = cfg.ambient_occlusion.compute_ao(0.5);
        assert!((result - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_color_correction() {
        let cfg = PostProcessConfig {
            color_correction_enabled: true,
            color_correction: ColorCorrectionConfig {
                enabled: true,
                brightness: 0.1,
                contrast: 1.0,
                saturation: 1.0,
                hue: 0.0,
            },
            ..Default::default()
        };
        let color = DVec3::new(0.5, 0.5, 0.5);
        let result = cfg.color_correction.apply(color);
        assert!((result.x - 0.6).abs() < 1e-10);
    }

    #[test]
    fn test_post_process_chain_order() {
        let cfg = PostProcessConfig {
            ambient_occlusion_enabled: true,
            bloom_enabled: true,
            fog_enabled: true,
            color_correction_enabled: true,
            tone_mapping_enabled: true,
            fxaa_enabled: true,
            ..Default::default()
        };
        let stages = cfg.enabled_stages();
        assert_eq!(stages[0], PostProcessStageType::AmbientOcclusion);
        assert_eq!(stages[1], PostProcessStageType::Bloom);
        assert_eq!(stages[2], PostProcessStageType::Fog);
        assert_eq!(stages[3], PostProcessStageType::ColorCorrection);
        assert_eq!(stages[4], PostProcessStageType::ToneMapping);
    }

    #[test]
    fn test_height_fog_disabled() {
        let cfg = PostProcessConfig::default();
        assert!(!cfg.height_fog_enabled);
        assert_eq!(cfg.compute_height_fog(1000.0), 0.0);
    }

    #[test]
    fn test_height_fog_at_sea_level() {
        let cfg = PostProcessConfig {
            height_fog_enabled: true,
            height_fog_base: 0.0,
            height_fog_falloff: 0.001,
            ..Default::default()
        };
        let fog = cfg.compute_height_fog(0.0);
        assert!((fog - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_height_fog_at_altitude() {
        let cfg = PostProcessConfig {
            height_fog_enabled: true,
            height_fog_base: 0.0,
            height_fog_falloff: 1.0e-4,
            ..Default::default()
        };
        let fog = cfg.compute_height_fog(10000.0);
        assert!(fog > 0.5);
    }

    #[test]
    fn test_to_pipeline() {
        let cfg = PostProcessConfig {
            bloom_enabled: true,
            bloom: BloomConfig {
                enabled: true,
                threshold: 0.9,
                ..Default::default()
            },
            ..Default::default()
        };
        let pipeline = cfg.to_pipeline();
        assert!(pipeline.bloom.enabled);
        assert!((pipeline.bloom.threshold - 0.9).abs() < 1e-10);
    }
}
