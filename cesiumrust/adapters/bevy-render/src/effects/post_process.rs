use bevy::prelude::*;
use cesium_effects::post_process::{
    AmbientOcclusionConfig, BloomConfig, ColorCorrectionConfig, FogConfig, PostProcessPipeline,
    PostProcessStageType, ToneMappingConfig,
};
#[allow(unused_imports)]
use glam::DVec3;

use super::ao::CesiumAmbientOcclusion;
use super::fxaa::CesiumFxaa;

/// Daniel M2: independent FXAA / AO sub-gate env vars, consumed **only** inside
/// effects.
///
/// **Single source of truth (task #81)**: both names are now registered in the
/// app-layer `feature_flags` registry (`ENV_ENABLE_FXAA` / `ENV_ENABLE_AO` with
/// the `fxaa_enabled()` / `ao_enabled()` accessors — the Terry M5-Verify Medium
/// finding). These two consts are therefore *mirrors*, not owners: they exist
/// only because `cesium-app` depends on `cesium-bevy-render` (never the reverse),
/// so this crate cannot import the registry. They are `pub` so
/// `feature_flags::adapter_gate_mirrors_are_byte_identical_to_the_registry`
/// can assert byte-equality across the crate boundary and turn a one-sided
/// rename red.
pub const ENV_ENABLE_FXAA: &str = "CESIUM_ENABLE_FXAA";
pub const ENV_ENABLE_AO: &str = "CESIUM_ENABLE_AO";

/// A sub-gate is ON unless its env var is explicitly set to a falsy token
/// (`0 / false / no / off / ""`). Unset ⇒ ON, so the master
/// `CESIUM_ENABLE_POSTPROCESS` gate alone still enables both effects (the pre-M2
/// behaviour). Delegates to the authoritative 4-token truthy parser
/// (`pipeline::fetch::gate_from_env_value` via `graph`) — no local copy (Daniel H1).
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
    /// Daniel M1: when `true` **and** FXAA is enabled, cameras carrying
    /// [`CesiumFxaa`] are forced to `Msaa::Off` so FXAA and the camera's hardware
    /// MSAA do not double-smooth the same edges. Recorded on the config so the
    /// coupling is explicit, discoverable and unit-testable.
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

/// M5-E2: drive the SSAO render-graph node on/off from [`PostProcessConfig`].
///
/// Syncs `config.ambient_occlusion_enabled` into every camera's
/// [`CesiumAmbientOcclusion`] marker component. The marker is extracted to the
/// render world by `ExtractComponentPlugin<CesiumAmbientOcclusion>` and read by
/// `AoNode::run` (which early-returns when `enabled == false`, giving zero GPU
/// cost when off). Mirrors [`fxaa_system`].
///
/// The SSAO kernel parameters (intensity=3.0, sample_radius=0.5, sample_count=16,
/// bias=0.001, length_cap=0.26) are **compile-time f32 constants** baked into
/// `ao.wgsl` (f64 in domain, projected to f32 at the GPU boundary); there is no
/// per-frame uniform to upload beyond the view matrix, so the enable toggle is
/// the runtime control surface. Runs in `Update`, **before**
/// [`super::ao::setup_ao_prepass`] (which attaches/detaches the depth+normal
/// prepass according to this flag).
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

/// M5-E1: drive the FXAA render-graph node on/off from [`PostProcessConfig`].
///
/// Syncs `config.fxaa_enabled` into every camera's [`CesiumFxaa`] marker
/// component. The marker is extracted to the render world by
/// `ExtractComponentPlugin<CesiumFxaa>` and read by `FxaaNode::run` (which
/// early-returns when `enabled == false`, giving zero GPU cost when off).
///
/// FXAA quality preset 12 parameters (PS=5, P0..P4, subpix=0.5,
/// edgeThreshold=0.125, edgeThresholdMin=0.0833) are **compile-time constants**
/// baked into `fxaa.wgsl` — mirroring CesiumJS, which hardcodes the preset in
/// the GLSL rather than exposing a uniform. There is therefore no per-frame
/// uniform buffer to upload; the enable toggle is the only runtime control
/// surface. Runs in `Update`.
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

/// FIX-MSAA-RESTORE: records a camera's user-authored [`Msaa`] that
/// [`fxaa_msaa_linkage_system`] suppressed to [`Msaa::Off`] while the FXAA↔MSAA
/// linkage was active, so the original can be restored the moment the linkage
/// ends instead of leaving the camera stuck at `Off`.
#[derive(Component, Clone, Copy, Debug)]
pub struct FxaaSuppressedMsaa(pub Msaa);

/// Daniel M1: FXAA and the camera's hardware MSAA are both anti-aliasing. Running
/// them together double-smooths edges (and wastes the 4× MSAA resolve that FXAA
/// then re-blurs). When FXAA is enabled and the linkage is on, force every
/// `CesiumFxaa` camera to `Msaa::Off`. This lives in effects (a camera-traversal
/// system), **not** in `orbit_camera.rs` (out of scope for this task). Runs in
/// `Update`; the coupling is recorded via [`PostProcessConfig::fxaa_forces_msaa_off`].
///
/// FIX-MSAA-RESTORE: the suppression is now reversible — the first time a camera
/// is forced off its original value is remembered in [`FxaaSuppressedMsaa`], and
/// when the linkage turns off (FXAA disabled or `fxaa_forces_msaa_off` cleared)
/// the original is restored and the marker dropped. The previous one-way write
/// left cameras permanently at `Off` even after FXAA was disabled.
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
                // Not yet suppressed: remember the authored value (only if there
                // is one to remember) and force it off.
                None => {
                    if *msaa != Msaa::Off {
                        commands.entity(cam).insert(FxaaSuppressedMsaa(*msaa));
                        *msaa = Msaa::Off;
                    }
                }
                // Already suppressed: stay forced off while the linkage holds.
                Some(_) => *msaa = Msaa::Off,
            }
        } else if let Some(saved) = suppressed {
            // Linkage ended: restore the original and clear the marker.
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
    // M4.2: folded former `fog_system_inner` (exact duplicate of `fog_system`)
    // into this unified entry point. The fog logic below is byte-identical to
    // `fog_system`; the standalone `fog_system` remains for direct scheduling.
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

        // M4.2: fog clear-color system — gated by CESIUM_ENABLE_POSTPROCESS_BUILTIN
        // (tonemapping / bloom / HDR live on the camera bundle in orbit_camera.rs).
        // Kept on the BUILTIN gate so it never leaks into the M5-E FXAA comparison.
        if super::graph::builtin_gate_enabled() {
            app.add_systems(Update, post_process_system);
        }

        // M5-E1: FXAA render-graph node — gated by CESIUM_ENABLE_POSTPROCESS.
        // Gate OFF → no nodes registered → v0 baselines pixel-neutral (PSNR=∞).
        if super::graph::postprocess_gate_enabled() {
            // Daniel M2: FXAA and AO now have independent sub-gates
            // (`CESIUM_ENABLE_FXAA` / `CESIUM_ENABLE_AO`). Each defaults to ON when
            // unset, so the master gate alone still enables both (pre-M2 behaviour);
            // setting one to a falsy token disables just that effect.
            let fxaa_enabled = sub_gate_enabled(ENV_ENABLE_FXAA);
            let ao_enabled = sub_gate_enabled(ENV_ENABLE_AO);
            {
                let mut cfg = app.world_mut().resource_mut::<PostProcessConfig>();
                cfg.fxaa_enabled = fxaa_enabled;
                cfg.ambient_occlusion_enabled = ao_enabled;
            }
            // fxaa_system / ao_system sync these toggles into each camera's
            // CesiumFxaa / CesiumAmbientOcclusion markers.
            app.add_systems(Update, fxaa_system);
            // Daniel M1: FXAA on ⇒ force the FXAA cameras' MSAA off (no double AA).
            app.add_systems(Update, fxaa_msaa_linkage_system);
            // ao_system must run before setup_ao_prepass so the prepass
            // attach/detach decision sees the reconciled `enabled` flag.
            app.add_systems(
                Update,
                ao_system.before(super::ao::setup_ao_prepass),
            );
            // `build` half only. The render-world half (pipeline `init_resource`,
            // whose `FromWorld` reads `RenderDevice`) must wait for
            // `Plugin::finish` below: Bevy inserts `RenderDevice` into the render
            // world in `RenderPlugin::finish`, never in `build`
            // (docs/deviations.md#dev-029).
            super::graph::register_render_graph_main_world(app);
        }
    }

    /// `Plugin::finish`: `RenderPlugin::finish` has now created and inserted
    /// `RenderDevice` / `RenderQueue` / `RenderAdapter` into the render world, so
    /// the `PassThroughPipeline` / `FxaaPipeline` / `AoPipeline` resources, their
    /// `Core3d` nodes and Robin #72's H2 chain can be built safely. Gate OFF (the
    /// default) ⇒ no-op ⇒ v0 baselines pixel-neutral.
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

    /// M5-E1: `fxaa_system` must sync `PostProcessConfig.fxaa_enabled` into every
    /// camera's `CesiumFxaa` marker (the node on/off driver). Headless-testable
    /// because it is a pure ECS system — no GPU / render graph required.
    #[test]
    fn test_fxaa_system_syncs_config_to_component() {
        let mut app = App::new();
        app.init_resource::<PostProcessConfig>();
        app.add_systems(Update, fxaa_system);

        // Spawn a camera marker enabled=true; config default is fxaa_enabled=false.
        let e = app.world_mut().spawn(CesiumFxaa { enabled: true }).id();
        app.update();
        assert!(
            !app.world().get::<CesiumFxaa>(e).unwrap().enabled,
            "component must follow config (false)"
        );

        // Flip config on → component follows.
        app.world_mut().resource_mut::<PostProcessConfig>().fxaa_enabled = true;
        app.update();
        assert!(
            app.world().get::<CesiumFxaa>(e).unwrap().enabled,
            "component must follow config (true)"
        );
    }

    /// M5-E2: `ao_system` must sync `PostProcessConfig.ambient_occlusion_enabled`
    /// into every camera's `CesiumAmbientOcclusion` marker (the SSAO node on/off
    /// driver). Headless-testable because it is a pure ECS system — no GPU /
    /// render graph required. Mirrors `test_fxaa_system_syncs_config_to_component`.
    #[test]
    fn test_ao_system_syncs_config_to_component() {
        let mut app = App::new();
        app.init_resource::<PostProcessConfig>();
        app.add_systems(Update, ao_system);

        // Spawn a camera marker enabled=true; config default is ambient_occlusion_enabled=false.
        let e = app
            .world_mut()
            .spawn(CesiumAmbientOcclusion { enabled: true })
            .id();
        app.update();
        assert!(
            !app.world().get::<CesiumAmbientOcclusion>(e).unwrap().enabled,
            "component must follow config (false)"
        );

        // Flip config on → component follows.
        app.world_mut()
            .resource_mut::<PostProcessConfig>()
            .ambient_occlusion_enabled = true;
        app.update();
        assert!(
            app.world().get::<CesiumAmbientOcclusion>(e).unwrap().enabled,
            "component must follow config (true)"
        );
    }

    /// Daniel M1: when FXAA is enabled and the linkage is on, every `CesiumFxaa`
    /// camera is forced to `Msaa::Off` (no double anti-aliasing); disabling the
    /// linkage leaves the camera's MSAA untouched. Headless — pure ECS.
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

        // Linkage disabled ⇒ the camera's MSAA is left as authored.
        app.world_mut()
            .resource_mut::<PostProcessConfig>()
            .fxaa_forces_msaa_off = false;
        *app.world_mut().get_mut::<Msaa>(cam).unwrap() = Msaa::Sample4;
        app.update();
        assert_eq!(*app.world().get::<Msaa>(cam).unwrap(), Msaa::Sample4);
    }

    /// FIX-MSAA-RESTORE: disabling FXAA must restore the camera's *original*
    /// MSAA (remembered when it was forced off), not leave it stuck at `Off`.
    /// Unlike the test above, the camera's MSAA is never hand-edited, so only a
    /// real save/restore can make it pass. Headless — pure ECS.
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

        // Linkage on ⇒ forced off, and the authored value is remembered.
        app.update();
        assert_eq!(*app.world().get::<Msaa>(cam).unwrap(), Msaa::Off);
        assert_eq!(
            app.world().get::<FxaaSuppressedMsaa>(cam).map(|s| s.0),
            Some(Msaa::Sample4),
            "original MSAA must be remembered on suppression"
        );

        // Disable FXAA (no manual MSAA edit) ⇒ original restored, marker dropped.
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

    /// Daniel M2: the FXAA / AO sub-gates read distinct env-var names so they can
    /// be toggled independently under the master `CESIUM_ENABLE_POSTPROCESS` gate.
    #[test]
    fn test_sub_gate_env_names_are_distinct() {
        assert_eq!(ENV_ENABLE_FXAA, "CESIUM_ENABLE_FXAA");
        assert_eq!(ENV_ENABLE_AO, "CESIUM_ENABLE_AO");
        assert_ne!(ENV_ENABLE_FXAA, ENV_ENABLE_AO);
    }
}

