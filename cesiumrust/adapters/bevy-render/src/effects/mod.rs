pub mod ao;
pub mod capability; // FIX-CAPPROBE（M11.6，scoped）：纯 GPU 能力探针 +
                   // 质量档阶梯。设备无关的核心经无头测试；
                   // render-node run() 接线 + "tier=off → baseline ±3%" 的证明
                   // 保持 GPU 门控（docs/deferred.md#68）。
pub mod clouds; // FIX-CLOUD-FULL（Phase 2，scoped）：M6.6 屏幕空间云适配器。
                // 参见 docs/deviations.md#dev-032。
pub mod clipping_planes;
pub mod fxaa;
pub mod graph;
pub mod ibl;
pub mod oit; // FIX-OIT-FULL（Phase 2）：9 个 Bevy 0.15 API 迁移错误已解决；模块重新启用。
              // 参见 docs/deviations.md#dev-031 与 docs/deferred.md #67。
pub mod panorama;
pub mod post_process;
pub mod particles;
pub mod split; // FIX-SPLIT（Phase 3）：M6.1 分屏；脚手架已从 oit.rs 迁出。
               // 参见 docs/deviations.md#dev-034。

#[allow(deprecated)] // FIX-REG-FACADE (DEV-029)：为 API 连续性重新导出已弃用的 facade。
pub use ao::{
    CesiumAmbientOcclusion, CameraAoPipeline, AoNode, AoPipeline, register_ao_node,
    register_ao_node_main_world, register_ao_node_render_world, setup_ao_prepass,
};
#[allow(deprecated)] // FIX-REG-FACADE (DEV-029)：为 API 连续性重新导出已弃用的 facade。
pub use clouds::{
    CesiumClouds, CesiumCloudsLabel, CloudsNode, CloudsPipeline, CloudsShadingMode,
    CloudsUniform, CameraCloudsPipeline, ViewCloudsUniform, clouds_gate_enabled,
    register_clouds_node, register_clouds_node_main_world, register_clouds_node_render_world,
    setup_clouds_prepass, CLOUDS_SHADER_HANDLE, CLOUD_BILLBOARD_SHADER_HANDLE,
    CLOUD_NOISE_SHADER_HANDLE, ENV_ENABLE_CLOUDS, MAX_CLOUDS,
};
#[allow(deprecated)] // FIX-REG-FACADE (DEV-029)：为 API 连续性重新导出已弃用的 facade。
pub use clipping_planes::{
    CesiumClippingLabel, CesiumClippingPlanes, CameraClippingPipeline, ClippingPlanesNode,
    ClippingPlanesPipeline, ClippingPlanesUniform, ViewClippingUniform, clipping_gate_enabled,
    register_clipping_planes_node, register_clipping_planes_node_main_world,
    register_clipping_planes_node_render_world, setup_clipping_prepass, CLIPPING_SHADER_HANDLE,
    ENV_ENABLE_CLIPPING, MAX_CLIPPING_PLANES,
};
#[allow(deprecated)] // FIX-REG-FACADE (DEV-029)：为 API 连续性重新导出已弃用的 facade。
pub use fxaa::{
    CesiumFxaa, CameraFxaaPipeline, FxaaNode, FxaaPipeline, register_fxaa_node,
    register_fxaa_node_main_world, register_fxaa_node_render_world,
};
#[allow(deprecated)] // FIX-REG-FACADE (DEV-029)：为 API 连续性重新导出已弃用的 facade。
pub use graph::{
    CesiumPassThrough, CesiumPostProcessLabel, M6WaveARenderGraphPlugin, PassThroughNode,
    PassThroughPipeline, create_post_process_texture, finish_render_graph, gate_from_env_value,
    insert_node_in_core3d, postprocess_gate_enabled, register_m6_render_graph,
    register_m6_render_graph_main_world, register_m6_render_graph_render_world,
    register_render_graph, register_render_graph_main_world, register_render_graph_render_world,
    wire_m6_edges,
};
#[allow(deprecated)] // FIX-REG-FACADE (DEV-029)：为 API 连续性重新导出已弃用的 facade。
pub use ibl::{
    CesiumIbl, CesiumIblLabel, CameraIblPipeline, IblNode, IblPipeline, IblUniform,
    ViewIblUniform, ibl_gate_enabled, register_ibl_node, register_ibl_node_main_world,
    register_ibl_node_render_world, setup_ibl_prepass, IBL_SHADER_HANDLE, ENV_ENABLE_IBL,
};
// FIX-OIT-FULL（Phase 2）：M6.4 OIT 节点的公共接口面。register
// 入口点是三段式（main/render 分离，DEV-029），以便 Phase-3
// 集成者能从 `Plugin::finish` 接线它们。
#[allow(deprecated)] // FIX-REG-FACADE (DEV-029)：为 API 连续性重新导出已弃用的 facade。
pub use oit::{
    CesiumOit, CesiumOitCompositeLabel, CesiumOitLabel, CameraOitPipeline,
    CameraOitCompositePipeline, OIT_ACCUMULATE_SHADER_HANDLE, OIT_COMPOSITE_SHADER_HANDLE,
    OITPlugin, OitCapabilitiesResource, OitCompositeNode, OitConfig, OitNode, OitPipeline,
    ENV_ENABLE_OIT, oit_gate_enabled, register_oit_node,
    register_oit_node_main_world, register_oit_node_render_world, setup_oit_prepass,
};
// FIX-SPLIT（Phase 3）：M6.1 分屏的公共接口面。`SplitConfig` /
// `SplitDragEvent` / `split_direction_system` 从 `oit.rs` 迁到这里；
// 屏幕空间分隔线节点（`SplitNode` / `SplitPipeline`）是新增的。
#[allow(deprecated)] // FIX-REG-FACADE (DEV-029)：为 API 连续性重新导出已弃用的 facade。
pub use split::{
    CesiumSplit, CesiumSplitLabel, CameraSplitPipeline, ENV_ENABLE_SPLIT, SPLIT_SHADER_HANDLE,
    SplitConfig, SplitDragEvent, SplitNode, SplitPipeline, SplitUniform, ViewSplitUniform,
    prepare_split, register_split_node, register_split_node_main_world,
    register_split_node_render_world, split_direction_system, split_gate_enabled,
};
// M6 Wave A（任务 #81）：panorama 模块被 M6.3 只留下了 `pub mod`，
// 好让这个集成任务来决定公共接口面。重导出在与
// `clipping_planes` / `ibl` 同级模块相同的深度，以便 `main.rs` 能
// 通过一条路径访问到组件、label 和门控。
#[allow(deprecated)] // FIX-REG-FACADE (DEV-029)：为 API 连续性重新导出已弃用的 facade。
pub use panorama::{
    CameraPanoramaBindGroup, CameraPanoramaPipeline, CesiumPanorama, CesiumPanoramaLabel,
    PanoramaNode, PanoramaPipeline, PanoramaPipelineKey, PanoramaUniforms, ENV_ENABLE_PANORAMA,
    PANORAMA_SHADER_HANDLE, insertion_hint, panorama_gate_enabled, register_panorama_node,
    register_panorama_node_main_world, register_panorama_node_render_world,
};
pub use particles::CesiumParticlePlugin;
// FIX-CAPPROBE：设备无关的质量档探针核心 + 其纯降级
// 辅助函数，重导出以便 finish 时的 render 接线（以及无头测试）
// 通过与 effect 节点相同的模块接口面访问它们。
pub use capability::{
    ao_sample_count, fxaa_steps, ibl_mip_levels, probe_quality_tier, DeviceCapabilitySnapshot,
    QualityTier,
};
pub use post_process::{
    CesiumEffectsPlugin, PostProcessConfig, ENV_ENABLE_AO, ENV_ENABLE_FXAA,
};

// ── 为 M6 Wave A 组装点重导出的领域值对象 ──────
//
// `application/cesium-app/src/main.rs` 从领域值对象构建三个 M6 相机组件
//（*来自领域值对象*：`CesiumClippingPlanes::new(collection)`、
// `CesiumIbl::new(ibl, material)`、
// `CesiumPanorama::from_domain_cubemap(&pano, image, brightness)` —— 即 cubemap
// 配对，也就是上游的 `SkyBox`/`CubeMapPanorama`；等距柱状投影的
// `Bubble` 配对也被重导出了，但其默认的 0.0157 渲染单位
// 半径对于轨道相机而言位于地球内部，所以它并非应用所
// 组装的 —— 参见 `docs/deviations.md#dev-028`）。
// FIX-MODRS-COMMENT：`cesium-app` *确实*声明了一个直接的 `cesium-effects`
// 依赖（`application/cesium-app/Cargo.toml`，为应用层
// 领域值对象组装而刻意保留 —— 这是 DDD 正确的
// 方向），但其源码通过本 adapter 的重导出而非 `cesium_effects::` 路径
// 访问这些类型（`cesium-app` 里任何地方都没有对该 crate 的直接 `use`），
// 所以应用保持单一导入接口 ——
// 与 `cesium_bevy_render::LightingMode` 所用的模式相同，
// `feature_flags.rs` 消费它。该 adapter 仍是唯一把
// 领域 f64 收窄为 GPU f32 的地方。
pub use cesium_effects::clipping::{ClippingPlane, ClippingPlaneCollection};
pub use cesium_effects::cloud::{CloudCollection, CumulusCloud};
pub use cesium_effects::ibl::{IblMaterial, ImageBasedLighting};
pub use cesium_effects::panorama::{CubeMapPanorama, EquirectangularPanorama};

// ─── FIX-REG-FACADE（DEV-029 收口）— render-world `RenderDevice` 守卫 ───────
//
/// 当 render world 尚**无** `RenderDevice` 时返回 `true`，即
/// `Plugin::finish` 时的 render 半边被过早到达（从插件的
/// `build` 出发）或针对一个裸 / 无头的 render world。
///
/// 本模块中每一个 `*Pipeline::from_world` —— 以及 OIT 能力探针 ——
/// 都会解引用 `RenderDevice`，而 Bevy 只在 `RenderPlugin::finish`
///（`bevy_render/src/lib.rs`）中才将其插入 render world，绝不会在其 `build` 中。旧代码
/// 因此会在 finish 半边从 `build` 被误用时 panic，报错 "RenderDevice does not exist in the World"，
/// 然而一个无头的
/// `MinimalPlugins` 应用根本没有 `RenderApp`，所以 `get_sub_app_mut(RenderApp)`
/// 返回 `None`，误用对测试套件一直隐形（永远
/// 绿）。以设备缺失为条件进行守卫，可使*正确的* `finish` 时行为
/// 保持字节一致（那时设备总在 → 守卫为 false），
/// 同时把误用变成一个可断言的 no-op —— 参见回归测试
/// `graph::tests::render_world_without_device_degrades_to_noop`。
pub(crate) fn render_world_missing_device(render_app: &bevy::app::SubApp) -> bool {
    render_app
        .world()
        .get_resource::<bevy::render::renderer::RenderDevice>()
        .is_none()
}
