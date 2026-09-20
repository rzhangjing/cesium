pub mod ao;
pub mod capability; // FIX-CAPPROBE (M11.6, scoped): pure GPU capability-probe +
                   // quality-tier ladder. Device-independent core is headless-tested;
                   // the render-node run() wiring + "tier=off → baseline ±3%" proof
                   // stays GPU-gated (docs/deferred.md#68).
pub mod clouds; // FIX-CLOUD-FULL (Phase 2, scoped): M6.6 screen-space clouds adapter.
                // See docs/deviations.md#dev-032.
pub mod clipping_planes;
pub mod fxaa;
pub mod graph;
pub mod ibl;
pub mod oit; // FIX-OIT-FULL (Phase 2): 9 Bevy 0.15 API migration errors resolved; module re-enabled.
              // See docs/deviations.md#dev-031 and docs/deferred.md #67.
pub mod panorama;
pub mod post_process;
pub mod particles;
pub mod split; // FIX-SPLIT (Phase 3): M6.1 split-screen; scaffolding migrated out of oit.rs.
               // See docs/deviations.md#dev-034.

#[allow(deprecated)] // FIX-REG-FACADE (DEV-029): re-export the deprecated facade for API continuity.
pub use ao::{
    CesiumAmbientOcclusion, CameraAoPipeline, AoNode, AoPipeline, register_ao_node,
    register_ao_node_main_world, register_ao_node_render_world, setup_ao_prepass,
};
#[allow(deprecated)] // FIX-REG-FACADE (DEV-029): re-export the deprecated facade for API continuity.
pub use clouds::{
    CesiumClouds, CesiumCloudsLabel, CloudsNode, CloudsPipeline, CloudsShadingMode,
    CloudsUniform, CameraCloudsPipeline, ViewCloudsUniform, clouds_gate_enabled,
    register_clouds_node, register_clouds_node_main_world, register_clouds_node_render_world,
    setup_clouds_prepass, CLOUDS_SHADER_HANDLE, CLOUD_BILLBOARD_SHADER_HANDLE,
    CLOUD_NOISE_SHADER_HANDLE, ENV_ENABLE_CLOUDS, MAX_CLOUDS,
};
#[allow(deprecated)] // FIX-REG-FACADE (DEV-029): re-export the deprecated facade for API continuity.
pub use clipping_planes::{
    CesiumClippingLabel, CesiumClippingPlanes, CameraClippingPipeline, ClippingPlanesNode,
    ClippingPlanesPipeline, ClippingPlanesUniform, ViewClippingUniform, clipping_gate_enabled,
    register_clipping_planes_node, register_clipping_planes_node_main_world,
    register_clipping_planes_node_render_world, setup_clipping_prepass, CLIPPING_SHADER_HANDLE,
    ENV_ENABLE_CLIPPING, MAX_CLIPPING_PLANES,
};
#[allow(deprecated)] // FIX-REG-FACADE (DEV-029): re-export the deprecated facade for API continuity.
pub use fxaa::{
    CesiumFxaa, CameraFxaaPipeline, FxaaNode, FxaaPipeline, register_fxaa_node,
    register_fxaa_node_main_world, register_fxaa_node_render_world,
};
#[allow(deprecated)] // FIX-REG-FACADE (DEV-029): re-export the deprecated facades for API continuity.
pub use graph::{
    CesiumPassThrough, CesiumPostProcessLabel, M6WaveARenderGraphPlugin, PassThroughNode,
    PassThroughPipeline, create_post_process_texture, finish_render_graph, gate_from_env_value,
    insert_node_in_core3d, postprocess_gate_enabled, register_m6_render_graph,
    register_m6_render_graph_main_world, register_m6_render_graph_render_world,
    register_render_graph, register_render_graph_main_world, register_render_graph_render_world,
    wire_m6_edges,
};
#[allow(deprecated)] // FIX-REG-FACADE (DEV-029): re-export the deprecated facade for API continuity.
pub use ibl::{
    CesiumIbl, CesiumIblLabel, CameraIblPipeline, IblNode, IblPipeline, IblUniform,
    ViewIblUniform, ibl_gate_enabled, register_ibl_node, register_ibl_node_main_world,
    register_ibl_node_render_world, setup_ibl_prepass, IBL_SHADER_HANDLE, ENV_ENABLE_IBL,
};
// FIX-OIT-FULL (Phase 2): public surface for the M6.4 OIT node. The register
// entry points are three-段式 (main/render split, DEV-029) so the Phase-3
// integrator can wire them from `Plugin::finish`.
#[allow(deprecated)] // FIX-REG-FACADE (DEV-029): re-export the deprecated facade for API continuity.
pub use oit::{
    CesiumOit, CesiumOitCompositeLabel, CesiumOitLabel, CameraOitPipeline,
    CameraOitCompositePipeline, OIT_ACCUMULATE_SHADER_HANDLE, OIT_COMPOSITE_SHADER_HANDLE,
    OITPlugin, OitCapabilitiesResource, OitCompositeNode, OitConfig, OitNode, OitPipeline,
    ENV_ENABLE_OIT, oit_gate_enabled, register_oit_node,
    register_oit_node_main_world, register_oit_node_render_world, setup_oit_prepass,
};
// FIX-SPLIT (Phase 3): the M6.1 split-screen public surface. `SplitConfig` /
// `SplitDragEvent` / `split_direction_system` moved here from `oit.rs`; the
// screen-space divider node (`SplitNode` / `SplitPipeline`) is new.
#[allow(deprecated)] // FIX-REG-FACADE (DEV-029): re-export the deprecated facade for API continuity.
pub use split::{
    CesiumSplit, CesiumSplitLabel, CameraSplitPipeline, ENV_ENABLE_SPLIT, SPLIT_SHADER_HANDLE,
    SplitConfig, SplitDragEvent, SplitNode, SplitPipeline, SplitUniform, ViewSplitUniform,
    prepare_split, register_split_node, register_split_node_main_world,
    register_split_node_render_world, split_direction_system, split_gate_enabled,
};
// M6 Wave A (task #81): the panorama module was left with only `pub mod` by
// M6.3 so this integration task could decide the public surface. Re-exported at
// the same depth as the `clipping_planes` / `ibl` siblings so `main.rs` can
// reach the component, the label and the gate through one path.
#[allow(deprecated)] // FIX-REG-FACADE (DEV-029): re-export the deprecated facade for API continuity.
pub use panorama::{
    CameraPanoramaBindGroup, CameraPanoramaPipeline, CesiumPanorama, CesiumPanoramaLabel,
    PanoramaNode, PanoramaPipeline, PanoramaPipelineKey, PanoramaUniforms, ENV_ENABLE_PANORAMA,
    PANORAMA_SHADER_HANDLE, insertion_hint, panorama_gate_enabled, register_panorama_node,
    register_panorama_node_main_world, register_panorama_node_render_world,
};
pub use particles::CesiumParticlePlugin;
// FIX-CAPPROBE: the device-independent quality-tier probe core + its pure degrade
// helpers, re-exported so the finish-time render wiring (and headless tests) reach
// them through the same module surface as the effect nodes.
pub use capability::{
    ao_sample_count, fxaa_steps, ibl_mip_levels, probe_quality_tier, DeviceCapabilitySnapshot,
    QualityTier,
};
pub use post_process::{
    CesiumEffectsPlugin, PostProcessConfig, ENV_ENABLE_AO, ENV_ENABLE_FXAA,
};

// ── Domain value objects re-exported for the M6 Wave A composition site ──────
//
// `application/cesium-app/src/main.rs` builds the three M6 camera components
// *from domain value objects* (`CesiumClippingPlanes::new(collection)`,
// `CesiumIbl::new(ibl, material)`,
// `CesiumPanorama::from_domain_cubemap(&pano, image, brightness)` — the cubemap
// pairing, i.e. upstream `SkyBox`/`CubeMapPanorama`; the equirectangular
// `Bubble` pairing is re-exported too but its default radius of 0.0157 render
// units sits inside the globe for the orbit camera, so it is not what the app
// composes — see `docs/deviations.md#dev-028`).
// FIX-MODRS-COMMENT: `cesium-app` *does* declare a direct `cesium-effects`
// dependency (`application/cesium-app/Cargo.toml`, kept deliberately for the
// application-layer composition of domain value objects — the DDD-correct
// direction), but its source reaches these types through this adapter's
// re-exports rather than a `cesium_effects::` path (there is no direct `use` of
// the crate anywhere in `cesium-app`), so the app keeps a single import surface —
// the same pattern used for `cesium_bevy_render::LightingMode`, which
// `feature_flags.rs` consumes. The adapter remains the only place that narrows
// domain f64 to GPU f32.
pub use cesium_effects::clipping::{ClippingPlane, ClippingPlaneCollection};
pub use cesium_effects::cloud::{CloudCollection, CumulusCloud};
pub use cesium_effects::ibl::{IblMaterial, ImageBasedLighting};
pub use cesium_effects::panorama::{CubeMapPanorama, EquirectangularPanorama};

// ─── FIX-REG-FACADE (DEV-029 收口) — render-world `RenderDevice` guard ───────
//
/// Returns `true` when the render world has **no** `RenderDevice` yet, i.e. a
/// `Plugin::finish`-time render half was reached too early (from a plugin's
/// `build`) or against a bare / headless render world.
///
/// Every `*Pipeline::from_world` in this module — and the OIT capability probe —
/// dereferences `RenderDevice`, which Bevy only inserts into the render world in
/// `RenderPlugin::finish` (`bevy_render/src/lib.rs`), never in its `build`. The
/// old code therefore panicked with "RenderDevice does not exist in the World"
/// whenever the finish halves were misused from `build`, yet a headless
/// `MinimalPlugins` app has no `RenderApp` at all, so `get_sub_app_mut(RenderApp)`
/// returned `None` and the misuse stayed invisible to the test suite (always
/// green). Guarding on the device's absence keeps the *correct* `finish`-time
/// behaviour byte-identical (the device is always present then → guard is false)
/// while turning the misuse into an assertable no-op — see the regression test
/// `graph::tests::render_world_without_device_degrades_to_noop`.
pub(crate) fn render_world_missing_device(render_app: &bevy::app::SubApp) -> bool {
    render_app
        .world()
        .get_resource::<bevy::render::renderer::RenderDevice>()
        .is_none()
}
