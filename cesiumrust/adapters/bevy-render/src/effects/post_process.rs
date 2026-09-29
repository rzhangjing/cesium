use bevy::prelude::*;
use cesium_effects::post_process::{
    AmbientOcclusionConfig, BloomConfig, ColorCorrectionConfig, FogConfig, PostProcessPipeline,
    PostProcessStageType, ToneMappingConfig,
};
#[allow(unused_imports)]
use glam::DVec3;

use super::ao::CesiumAmbientOcclusion;
use super::fxaa::CesiumFxaa;

/// Daniel M2：独立的 FXAA / AO 子门控环境变量，**仅**在 effects
/// 内部消费。
///
/// **单一真相源（任务 #81）**：两个名字现在都注册在应用层的
/// `feature_flags` 注册表（`ENV_ENABLE_FXAA` / `ENV_ENABLE_AO`，搭配
/// `fxaa_enabled()` / `ao_enabled()` 访问器 —— Terry M5-Verify 的中等
/// 发现）。因此这两个 const 是*镜像*，而非所有者：它们存在
/// 仅因为 `cesium-app` 依赖 `cesium-bevy-render`（从不反向），
/// 所以本 crate 无法导入注册表。它们是 `pub` 的，以便
/// `feature_flags::adapter_gate_mirrors_are_byte_identical_to_the_registry`
/// 能跨 crate 边界断言字节相等，并使单方面的
/// 重命名变红。
pub const ENV_ENABLE_FXAA: &str = "CESIUM_ENABLE_FXAA";
pub const ENV_ENABLE_AO: &str = "CESIUM_ENABLE_AO";

/// 子门控默认 ON，除非其环境变量被显式设为 falsy token
///（`0 / false / no / off / ""`）。未设置 ⇒ ON，所以仅主
/// `CESIUM_ENABLE_POSTPROCESS` 门控仍会启用两个效果（M2 之前的
/// 行为）。委派给权威的 4-token truthy 解析器
///（`pipeline::fetch::gate_from_env_value`，经 `graph`）—— 无本地副本（Daniel H1）。
fn sub_gate_enabled(env: &str) -> bool {
    match std::env::var(env) {
        Ok(raw) => super::graph::gate_from_env_value(Some(raw)),
        Err(_) => true,
    }
}

#[derive(Resource, Debug, Clone)]
pub struct PostProcessConfig {
    pub fog_enabled: bool,
    pub tone_mapping_enabled: bool,
    pub bloom_enabled: bool,
    pub ambient_occlusion_enabled: bool,
    pub fxaa_enabled: bool,
    /// Daniel M1：当 `true` **且** FXAA 启用时，携带
    /// [`CesiumFxaa`] 的相机被强制为 `Msaa::Off`，以免 FXAA 与相机的硬件
    /// MSAA 对同一条边双重平滑。在配置上记录该耦合，以便它
    /// 显式、可发现、可单元测试。
    pub fxaa_forces_msaa_off: bool,
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
            fxaa_forces_msaa_off: true,
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

/// M5-E2：从 [`PostProcessConfig`] 驱动 SSAO render-graph 节点的开/关。
///
/// 将 `config.ambient_occlusion_enabled` 同步进每个相机的
/// [`CesiumAmbientOcclusion`] 标记组件。该标记由
/// `ExtractComponentPlugin<CesiumAmbientOcclusion>` 提取到 render world，并由
/// `AoNode::run` 读取（当 `enabled == false` 时提前 return，关闭时零 GPU
/// 开销）。对应 [`fxaa_system`]。
///
/// SSAO 核参数（intensity=3.0、sample_radius=0.5、sample_count=16、
/// bias=0.001、length_cap=0.26）是烧入 `ao.wgsl` 的**编译期 f32 常量**
///（领域中为 f64，在 GPU 边界投影为 f32）；除视图矩阵外没有
/// 逐帧 uniform 需上传，所以开关就是运行时控制面。运行在 `Update`，
/// **在** [`super::ao::setup_ao_prepass`]（它根据此标志
/// 挂载/摘除 depth+normal 前置 pass）**之前**。
pub fn ao_system(
    config: Res<PostProcessConfig>,
    mut query: Query<&mut CesiumAmbientOcclusion>,
) {
    for mut ao in &mut query {
        if ao.enabled != config.ambient_occlusion_enabled {
            ao.enabled = config.ambient_occlusion_enabled;
        }
    }
}

/// M5-E1：从 [`PostProcessConfig`] 驱动 FXAA render-graph 节点的开/关。
///
/// 将 `config.fxaa_enabled` 同步进每个相机的 [`CesiumFxaa`] 标记
/// 组件。该标记由 `ExtractComponentPlugin<CesiumFxaa>` 提取到
/// render world，并由 `FxaaNode::run` 读取（当 `enabled == false` 时
/// 提前 return，关闭时零 GPU 开销）。
///
/// FXAA 质量预设 12 参数（PS=5、P0..P4、subpix=0.5、
/// edgeThreshold=0.125、edgeThresholdMin=0.0833）是烧入 `fxaa.wgsl` 的
/// **编译期常量** —— 与 CesiumJS 一致，它在 GLSL 中硬编码预设
/// 而非暴露一个 uniform。因此没有逐帧
/// uniform 缓冲需上传；开关是唯一的运行时控制
/// 面。运行在 `Update`。
pub fn fxaa_system(
    config: Res<PostProcessConfig>,
    mut query: Query<&mut CesiumFxaa>,
) {
    for mut fxaa in &mut query {
        if fxaa.enabled != config.fxaa_enabled {
            fxaa.enabled = config.fxaa_enabled;
        }
    }
}

/// FIX-MSAA-RESTORE：记录相机用户自行设定的 [`Msaa`]，即
/// [`fxaa_msaa_linkage_system`] 在 FXAA↔MSAA 关联处于活动期时被压制为 [`Msaa::Off`]
/// 的值，以便一旦关联结束就能立即恢复原值，而不是
/// 把相机卡在 `Off`。
#[derive(Component, Clone, Copy, Debug)]
pub struct FxaaSuppressedMsaa(pub Msaa);

/// Daniel M1：FXAA 与相机的硬件 MSAA 都是抗锯齿。同时运行
/// 会对边双重平滑（并浪费 FXAA 随后又重新模糊的 4× MSAA 解析）。当 FXAA 启用
/// 且关联开启时，强制每个
/// `CesiumFxaa` 相机为 `Msaa::Off`。这位于 effects（一个相机遍历系统），
/// **不在** `orbit_camera.rs`（不属本任务范围）。运行在
/// `Update`；该耦合通过 [`PostProcessConfig::fxaa_forces_msaa_off`] 记录。
///
/// FIX-MSAA-RESTORE：压制现在可逆 —— 相机第一次被强制关闭时，其原值
/// 被记入 [`FxaaSuppressedMsaa`]，而当关联关闭（FXAA 禁用或
/// `fxaa_forces_msaa_off` 清除）时，原值被恢复且标记被删除。之前的单向写
/// 会让相机即便在 FXAA 禁用后也永久停在 `Off`。
pub fn fxaa_msaa_linkage_system(
    config: Res<PostProcessConfig>,
    mut commands: Commands,
    mut cameras: Query<
        (Entity, &mut Msaa, Option<&mut FxaaSuppressedMsaa>),
        With<CesiumFxaa>,
    >,
) {
    let linkage_on = config.fxaa_enabled && config.fxaa_forces_msaa_off;
    for (cam, mut msaa, suppressed) in &mut cameras {
        if linkage_on {
            match suppressed {
                // 尚未压制：记住已设定的值（仅当有值可记时）并强制将其
                // 关闭。
                None => {
                    if *msaa != Msaa::Off {
                        commands.entity(cam).insert(FxaaSuppressedMsaa(*msaa));
                        *msaa = Msaa::Off;
                    }
                }
                // 已压制：只要关联保持就继续强制关闭。
                Some(_) => *msaa = Msaa::Off,
            }
        } else if let Some(saved) = suppressed {
            // 关联结束：恢复原值并清除标记。
            *msaa = saved.0;
            commands.entity(cam).remove::<FxaaSuppressedMsaa>();
        }
    }
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
    mut clear_color: ResMut<ClearColor>,
    camera_query: Query<&Transform, With<Camera3d>>,
) {
    // M4.2：将旧的 `fog_system_inner`（`fog_system` 的完全重复）折入
    // 这个统一入口点。下方的雾逻辑与 `fog_system` 字节一致；
    // 独立的 `fog_system` 保留以便直接调度。
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
        app.init_resource::<PostProcessConfig>();

        // M4.2：雾清屏色系统 —— 由 CESIUM_ENABLE_POSTPROCESS_BUILTIN 门控
        //（tonemapping / bloom / HDR 位于 orbit_camera.rs 的相机 bundle 上）。
        // 保持在 BUILTIN 门控上，以免泄入 M5-E 的 FXAA 对比。
        if super::graph::builtin_gate_enabled() {
            app.add_systems(Update, post_process_system);
        }

        // M5-E1：FXAA render-graph 节点 —— 由 CESIUM_ENABLE_POSTPROCESS 门控。
        // 门控 OFF → 不注册节点 → v0 baseline 像素中性（PSNR=∞）。
        if super::graph::postprocess_gate_enabled() {
            // Daniel M2：FXAA 与 AO 现在拥有独立的子门控
            //（`CESIUM_ENABLE_FXAA` / `CESIUM_ENABLE_AO`）。各自未设置时默认为 ON，
            // 所以仅主门控仍会启用两者（M2 之前的行为）；
            // 将一个设为 falsy token 只会禁用该效果。
            let fxaa_enabled = sub_gate_enabled(ENV_ENABLE_FXAA);
            let ao_enabled = sub_gate_enabled(ENV_ENABLE_AO);
            {
                let mut cfg = app.world_mut().resource_mut::<PostProcessConfig>();
                cfg.fxaa_enabled = fxaa_enabled;
                cfg.ambient_occlusion_enabled = ao_enabled;
            }
            // fxaa_system / ao_system 将这些开关同步进每个相机的
            // CesiumFxaa / CesiumAmbientOcclusion 标记。
            app.add_systems(Update, fxaa_system);
            // Daniel M1：FXAA 开启 ⇒ 强制 FXAA 相机的 MSAA 关闭（无双重 AA）。
            app.add_systems(Update, fxaa_msaa_linkage_system);
            // ao_system 必须在 setup_ao_prepass 之前运行，以便前置 pass 的
            // 挂载/摘除决策能看到已调和的 `enabled` 标志。
            app.add_systems(
                Update,
                ao_system.before(super::ao::setup_ao_prepass),
            );
            // 仅 `build` 半边。render-world 半边（pipeline `init_resource`，
            // 其 `FromWorld` 读取 `RenderDevice`）必须等待下方的
            // `Plugin::finish`：Bevy 在 `RenderPlugin::finish` 中才将 `RenderDevice`
            // 插入 render world，从不在 `build` 中
            //（docs/deviations.md#dev-029）。
            super::graph::register_render_graph_main_world(app);
        }
    }

    /// `Plugin::finish`：`RenderPlugin::finish` 现已创建并将 `RenderDevice` /
    /// `RenderQueue` / `RenderAdapter` 插入 render world，因此
    /// `PassThroughPipeline` / `FxaaPipeline` / `AoPipeline` 资源、它们的
    /// `Core3d` 节点以及 Robin #72 的 H2 链可以安全构建。门控 OFF（默认值）
    /// ⇒ no-op ⇒ v0 baseline 像素中性。
    fn finish(&self, app: &mut App) {
        if super::graph::postprocess_gate_enabled() {
            super::graph::finish_render_graph(app);
        }
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

    /// M5-E1：`fxaa_system` 必须将 `PostProcessConfig.fxaa_enabled` 同步进每个
    /// 相机的 `CesiumFxaa` 标记（节点开/关驱动）。可无头测试
    /// 因为它是一个纯 ECS 系统 —— 无需 GPU / render graph。
    #[test]
    fn test_fxaa_system_syncs_config_to_component() {
        let mut app = App::new();
        app.init_resource::<PostProcessConfig>();
        app.add_systems(Update, fxaa_system);

        // 生成一个 enabled=true 的相机标记；配置默认 fxaa_enabled=false。
        let e = app.world_mut().spawn(CesiumFxaa { enabled: true }).id();
        app.update();
        assert!(
            !app.world().get::<CesiumFxaa>(e).unwrap().enabled,
            "component must follow config (false)"
        );

        // 将配置翻为开 → 标记随之。
        app.world_mut().resource_mut::<PostProcessConfig>().fxaa_enabled = true;
        app.update();
        assert!(
            app.world().get::<CesiumFxaa>(e).unwrap().enabled,
            "component must follow config (true)"
        );
    }

    /// M5-E2：`ao_system` 必须将 `PostProcessConfig.ambient_occlusion_enabled`
    /// 同步进每个相机的 `CesiumAmbientOcclusion` 标记（SSAO 节点开/关
    /// 驱动）。可无头测试因为它是一个纯 ECS 系统 —— 无需 GPU /
    /// render graph。对应 `test_fxaa_system_syncs_config_to_component`。
    #[test]
    fn test_ao_system_syncs_config_to_component() {
        let mut app = App::new();
        app.init_resource::<PostProcessConfig>();
        app.add_systems(Update, ao_system);

        // 生成一个 enabled=true 的相机标记；配置默认 ambient_occlusion_enabled=false。
        let e = app
            .world_mut()
            .spawn(CesiumAmbientOcclusion { enabled: true })
            .id();
        app.update();
        assert!(
            !app.world().get::<CesiumAmbientOcclusion>(e).unwrap().enabled,
            "component must follow config (false)"
        );

        // 将配置翻为开 → 标记随之。
        app.world_mut()
            .resource_mut::<PostProcessConfig>()
            .ambient_occlusion_enabled = true;
        app.update();
        assert!(
            app.world().get::<CesiumAmbientOcclusion>(e).unwrap().enabled,
            "component must follow config (true)"
        );
    }

    /// Daniel M1：当 FXAA 启用且关联开启时，每个 `CesiumFxaa`
    /// 相机被强制为 `Msaa::Off`（无双重抗锯齿）；禁用关联则保持相机的 MSAA
    /// 不变。无头 —— 纯 ECS。
    #[test]
    fn test_fxaa_msaa_linkage_forces_msaa_off() {
        let mut app = App::new();
        app.insert_resource(PostProcessConfig {
            fxaa_enabled: true,
            ..Default::default()
        });
        app.add_systems(Update, fxaa_msaa_linkage_system);

        let cam = app
            .world_mut()
            .spawn((Msaa::Sample4, CesiumFxaa { enabled: true }))
            .id();
        app.update();
        assert_eq!(
            *app.world().get::<Msaa>(cam).unwrap(),
            Msaa::Off,
            "FXAA on ⇒ camera MSAA forced off"
        );

        // 关联禁用 ⇒ 相机的 MSAA 保持其设定的值。
        app.world_mut()
            .resource_mut::<PostProcessConfig>()
            .fxaa_forces_msaa_off = false;
        *app.world_mut().get_mut::<Msaa>(cam).unwrap() = Msaa::Sample4;
        app.update();
        assert_eq!(*app.world().get::<Msaa>(cam).unwrap(), Msaa::Sample4);
    }

    /// FIX-MSAA-RESTORE：禁用 FXAA 必须恢复相机*原始的*
    /// MSAA（在其被强制关闭时记住的值），而不是把它卡在 `Off`。
    /// 与上面的测试不同，相机的 MSAA 从不会被手工修改，所以只有
    /// 真正的 save/restore 才能使其通过。无头 —— 纯 ECS。
    #[test]
    fn test_fxaa_msaa_linkage_restores_original_msaa() {
        let mut app = App::new();
        app.insert_resource(PostProcessConfig {
            fxaa_enabled: true,
            ..Default::default()
        });
        app.add_systems(Update, fxaa_msaa_linkage_system);
        let cam = app
            .world_mut()
            .spawn((Msaa::Sample4, CesiumFxaa { enabled: true }))
            .id();

        // 关联开启 ⇒ 强制关闭，且设定的值被记住。
        app.update();
        assert_eq!(*app.world().get::<Msaa>(cam).unwrap(), Msaa::Off);
        assert_eq!(
            app.world().get::<FxaaSuppressedMsaa>(cam).map(|s| s.0),
            Some(Msaa::Sample4),
            "original MSAA must be remembered on suppression"
        );

        // 禁用 FXAA（不手工修改 MSAA）⇒ 恢复原值，标记被删除。
        app.world_mut()
            .resource_mut::<PostProcessConfig>()
            .fxaa_enabled = false;
        app.update();
        assert_eq!(
            *app.world().get::<Msaa>(cam).unwrap(),
            Msaa::Sample4,
            "disabling FXAA must restore the camera's authored MSAA"
        );
        assert!(
            app.world().get::<FxaaSuppressedMsaa>(cam).is_none(),
            "the suppression marker must be cleared after restoring"
        );
    }

    /// Daniel M2：FXAA / AO 子门控读取各自不同的环境变量名，以便
    /// 在主 `CESIUM_ENABLE_POSTPROCESS` 门控下可独立切换它们。
    #[test]
    fn test_sub_gate_env_names_are_distinct() {
        assert_eq!(ENV_ENABLE_FXAA, "CESIUM_ENABLE_FXAA");
        assert_eq!(ENV_ENABLE_AO, "CESIUM_ENABLE_AO");
        assert_ne!(ENV_ENABLE_FXAA, ENV_ENABLE_AO);
    }
}

