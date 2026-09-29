//! M5-E0：cesiumrust RenderGraph 基础设施。
//!
//! 为 cesiumrust 建立首个 `ViewNode` + `Extract<Component>` + `ExtractSchedule` +
//! `RenderApp` 装配模式。为 M5-E1（FXAA）和 M5-E2（SSAO）
//! 后处理节点提供可复用的 helper。
//!
//! # 架构
//! - 门控：`CESIUM_ENABLE_POSTPROCESS` 环境变量（同 `feature_flags::postprocess_enabled()`）
//! - 门控 OFF → **不注册任何渲染图节点** → v0 基线零差异（PSNR=∞）
//! - 门控 ON → pass-through 节点插入 `Core3d` 中 `EndMainPass` 与
//!   `Tonemapping` 之间，证明像素中性的基础设施。Daniel H2 链顺序
//!   （上游 CesiumJS 一致性）：`EndMainPass → PassThrough → AmbientOcclusion →
//!   Tonemapping → Fxaa → EndMainPassPostProcessing`——AO 在 tonemapping 之前（HDR
//!   场景），FXAA 最后（最终 LDR 图像）。
//!
//! # 偏差
//! cesiumrust 使用 Bevy 的 RenderGraph（子图 `Core3d`、`ViewNode` trait、
//! `RenderApp` 子 app 提取），而蓝图 cesium-rs 使用裸的
//! wgpu render-pass 链，没有图抽象。参见 `docs/deviations.md#dev-016`。
//!
//! # 参考（Bevy 0.15.3）
//! - `bevy_pbr/src/ssao/mod.rs` L25–131: ViewNode + ExtractSchedule + RenderApp pattern
//! - `bevy_core_pipeline/src/fxaa/node.rs`: simplest post-process ViewNode (86 lines)
//! - `bevy_core_pipeline/src/core_3d/mod.rs` L6–41: `Core3d`/`Node3d` labels
//! - `bevy_core_pipeline/src/fullscreen_vertex_shader/`: fullscreen triangle vertex state

use std::sync::Mutex;

use bevy::core_pipeline::{
    core_3d::graph::{Core3d, Node3d},
    fullscreen_vertex_shader::fullscreen_shader_vertex_state,
};
use bevy::ecs::query::QueryItem;
use bevy::prelude::*;
use bevy::render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::{
        InternedRenderLabel, NodeRunError, RenderGraph, RenderGraphApp, RenderGraphContext,
        RenderLabel, ViewNode, ViewNodeRunner,
    },
    render_resource::{
        binding_types::{sampler, texture_2d},
        BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, Extent3d, FilterMode,
        FragmentState, MultisampleState, Operations, PipelineCache, PrimitiveState,
        RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor,
        Sampler as GpuSampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
        SpecializedRenderPipeline, SpecializedRenderPipelines, Texture, TextureDescriptor,
        TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
        TextureViewDescriptor, TextureViewId,
    },
    renderer::{RenderContext, RenderDevice},
    view::{ExtractedView, ViewTarget},
    Render, RenderApp, RenderSet,
};

// ─── Shader handle ───────────────────────────────────────────────────────────

/// 内嵌 `pass_through.wgsl` shader 的唯一 handle。
/// 该值经挑选以避免与 Bevy 内部 handle 冲突。
pub const PASS_THROUGH_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0xCE51_E0E0_F00D_0001);

// ─── 门控 ────────────────────────────────────────────────────────────────────

/// cesium-app 中 `feature_flags::postprocess_enabled()` 读取的环境变量。
/// 在此重复是因为适配层不能导入应用层（DDD）。
const ENV_ENABLE_POSTPROCESS: &str = "CESIUM_ENABLE_POSTPROCESS";

/// `feature_flags::postprocess_builtin_enabled()` 读取的环境变量（M4.2 门控，用于
/// tonemapping / bloom / HDR 雾）。因同样的 DDD 原因在此重复。
const ENV_ENABLE_POSTPROCESS_BUILTIN: &str = "CESIUM_ENABLE_POSTPROCESS_BUILTIN";

/// 当后处理门控启用时返回 `true`。
/// 读取与 `feature_flags::postprocess_enabled()` 相同的环境变量。
/// 门控 M5-E 渲染图链（pass-through + FXAA + AO）。
#[inline]
pub fn postprocess_gate_enabled() -> bool {
    gate_from_env_value(std::env::var(ENV_ENABLE_POSTPROCESS).ok())
}

/// 当内置后处理门控启用时返回 `true`。
/// 读取与 `feature_flags::postprocess_builtin_enabled()` 相同的环境变量。
/// 门控 M4.2 雾清除色系统（tonemapping / bloom 住在
/// orbit_camera.rs 的相机束上）。与 M5-E FXAA/AO 门控保持分离，
/// 以便两个特性可独立切换（leader 裁决 Q5）。
#[inline]
pub fn builtin_gate_enabled() -> bool {
    gate_from_env_value(std::env::var(ENV_ENABLE_POSTPROCESS_BUILTIN).ok())
}

// Daniel H1 / L2：权威的门控解析器住在 `pipeline::fetch`——全
// crate 的 4-token truthy 集 `{1, true, yes, on}`，trim + 转小写
//（与 `feature_flags::truthy` 字节一致）。此前此处的本地 2-token 拷贝
//（`"1" | "true"`，未 trim）与它产生了分歧，而 `effects::gate_from_env_value`
// 为同名遮蔽了另一层含义。重新导出单一真相源，
// 以便 `postprocess_gate_enabled` / `builtin_gate_enabled` 和每个
// `effects::*` 消费者保持一致。
pub use crate::pipeline::fetch::gate_from_env_value;

// ─── 渲染图 label ─────────────────────────────────────────────────────

/// cesium 后处理节点在 `Core3d` 子图中的自定义节点 label。
///
/// 为 M5-E1（FXAA）和 M5-E2（AO）预先声明 label，使它们无需
/// 单独的 label 定义——直接复用这些变体即可。
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub enum CesiumPostProcessLabel {
    /// M5-E0：基础设施概念验证 pass-through 节点。
    PassThrough,
    /// M5-E1：FXAA 抗锯齿节点。
    Fxaa,
    /// M5-E2：屏幕空间环境光遮蔽节点。
    AmbientOcclusion,
}

// ─── 组件 ───────────────────────────────────────────────────────────────

/// 位于应运行 cesium 后处理链的相机上的标记组件。
/// 经 `ExtractSchedule` 提取到 render world。
#[derive(Component, Clone, Debug, Default, ExtractComponent)]
pub struct CesiumPassThrough {
    /// 为 `false` 时，节点的 `run()` 提前 return（零开销）。
    pub enabled: bool,
}

/// 提取 + 准备后存储在 render world 中的逐视图 pipeline ID。
#[derive(Component)]
pub struct CameraPassThroughPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

// ─── Pipeline ────────────────────────────────────────────────────────────────

/// 为 pass-through（及未来后处理）pipeline 持有 bind group layout 和采样器的
/// Render-world 资源。
#[derive(Resource)]
pub struct PassThroughPipeline {
    pub texture_bind_group_layout: BindGroupLayout,
    pub sampler: GpuSampler,
}

impl FromWorld for PassThroughPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();

        let texture_bind_group_layout = render_device.create_bind_group_layout(
            "cesium_pass_through_texture_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_pass_through_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture_bind_group_layout,
            sampler,
        }
    }
}

/// 特化 key：仅输出纹理格式不同（HDR vs LDR）。
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct PassThroughPipelineKey {
    pub texture_format: TextureFormat,
}

impl SpecializedRenderPipeline for PassThroughPipeline {
    type Key = PassThroughPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("cesium_pass_through_pipeline".into()),
            layout: vec![self.texture_bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: PASS_THROUGH_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: key.texture_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: Vec::new(),
            zero_initialize_workgroup_memory: false,
        }
    }
}

// ─── ViewNode ────────────────────────────────────────────────────────────────

/// cesiumrust 的 pass-through `ViewNode`——本代码库中首个此类节点。
///
/// 语义（对应 CesiumJS `PassThrough.glsl`）：采样输入纹理
/// 并原样写入输出。构造上即像素中性。
#[derive(Default)]
pub struct PassThroughNode {
    cached_texture_bind_group: Mutex<Option<(TextureViewId, BindGroup)>>,
}

impl ViewNode for PassThroughNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static CameraPassThroughPipeline,
        &'static CesiumPassThrough,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (target, pipeline_handle, pass_through): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        // FIX-GRAPH-WIRING / FIX-CAPPROBE：这仍然只是一个*组件启用*
        // 提前 return——它读取节点的 `enabled` 标志。纯粹的
        // 能力探测 / 质量档**核心**现已存在
        //（`crate::effects::capability::{probe_quality_tier, QualityTier, ...}`，
        // 已做无头测试），但尚未接入*以档位门控渲染*：
        // 那需要真实的 `RenderDevice`→快照探测 + 帧时间
        // 插桩来证明计划的 "tier=off → baseline ±3%"，两者都
        // 受 GPU 门控并被推迟（M11.6 / `docs/deferred.md#68`）。在无头下
        // 没有可查询的 `RenderAdapter`/`Features`，因此降级到
        // enabled 检查（而非猜测的档位）是正确、诚实的行为。
        if !pass_through.enabled {
            return Ok(());
        }

        let pipeline_cache = world.resource::<PipelineCache>();
        let pass_through_pipeline = world.resource::<PassThroughPipeline>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_handle.pipeline_id) else {
            return Ok(());
        };

        let post_process = target.post_process_write();
        let source = post_process.source;
        let destination = post_process.destination;

        // 跨帧缓存 bind group（源纹理变化时失效）。
        let mut cached = self.cached_texture_bind_group.lock().unwrap();
        let bind_group = match &mut *cached {
            Some((id, bg)) if source.id() == *id => bg,
            slot => {
                let bg = render_context.render_device().create_bind_group(
                    Some("cesium_pass_through_bind_group"),
                    &pass_through_pipeline.texture_bind_group_layout,
                    &BindGroupEntries::sequential((source, &pass_through_pipeline.sampler)),
                );
                let (_, bg) = slot.insert((source.id(), bg));
                bg
            }
        };

        let pass_descriptor = RenderPassDescriptor {
            label: Some("cesium_pass_through_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        let mut render_pass = render_context
            .command_encoder()
            .begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1); // 全屏三角形

        Ok(())
    }
}

// ─── 渲染系统 ──────────────────────────────────────────────────────────

/// 为每个带有 [`CesiumPassThrough`] 的相机视图准备特化 pipeline。
/// 运行于 `Render` schedule 的 `RenderSet::Prepare`。
pub fn prepare_pass_through_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<PassThroughPipeline>>,
    pass_through_pipeline: Res<PassThroughPipeline>,
    views: Query<(Entity, &ExtractedView, &CesiumPassThrough)>,
) {
    for (entity, view, pass_through) in &views {
        if !pass_through.enabled {
            continue;
        }

        let texture_format = if view.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::Rgba8UnormSrgb
        };

        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pass_through_pipeline,
            PassThroughPipelineKey { texture_format },
        );

        commands
            .entity(entity)
            .insert(CameraPassThroughPipeline { pipeline_id });
    }
}

// `ExtractSchedule` 经 `register_render_graph` 中注册的
// `ExtractComponentPlugin<CesiumPassThrough>` 得到演示。该插件内部
// 在 `ExtractSchedule` 中使用 `Extract<Query<(Entity, &CesiumPassThrough)>>`
// 每帧将组件从主 world 拷贝到 render world。
// 这与 Bevy 的 `FxaaPlugin`（bevy_core_pipeline 0.15.3）所用模式相同。

// ─── 可复用 helper ────────────────────────────────────────────────────────

/// 在 `Core3d` 中把一条渲染图节点插入到两个已有 label **之间**。
///
/// 创建边：`predecessor → new_label → successor`，并且——自 M6 Wave A
///（task #81）起——**先删除**已存在的 `predecessor → successor`
/// 边，使插入真正是*串行*的。
///
/// # 为何删除是必须的
/// `RenderGraph::add_node_edges` 只会*添加*。保留原来的
/// `predecessor → successor` 边会形成一个**菱形**：图随即
/// 同时包含直连边和 `predecessor → new_label → successor`
/// 路径，因此拓扑排序可自由地把插入的节点排到两者之间的任意位置
///（包括排在 successor 的消费者*之后*，或与 successor 并行）。
/// 这正是 Ultra Review 标记为 Daniel H2 的缺陷类，Lee 也针对
/// M6.3 panorama 槽位专门提出了它。Bevy 0.15.3 的 `RenderGraphApp`
/// 扩展 trait 并**不**暴露删除 helper，所以
/// 必须直接访问 `RenderGraph` 资源——参见
/// [`remove_core3d_edge`]。
///
/// # 用法（M6.3 panorama）
/// ```text
/// insert_node_in_core3d(
///     render_app,
///     CesiumPanoramaLabel,
///     Node3d::MainOpaquePass,          // predecessor
///     Node3d::MainTransmissivePass,    // successor
/// );
/// // then: render_app.add_render_graph_node::<ViewNodeRunner<PanoramaNode>>(Core3d, CesiumPanoramaLabel);
/// ```
///
/// # 注意
/// 节点本身必须通过 `add_render_graph_node` 单独添加，可在调用
/// 本 helper 之前或之后（本 helper 只管理边）。
pub fn insert_node_in_core3d(
    render_app: &mut bevy::app::SubApp,
    new_label: impl RenderLabel,
    predecessor: impl RenderLabel + Clone,
    successor: impl RenderLabel + Clone,
) {
    remove_core3d_edge(render_app, predecessor.clone(), successor.clone());
    render_app.add_render_graph_edges(Core3d, (predecessor, new_label, successor));
}

/// 删除一条 `Core3d` 节点边，容忍"边不存在"。
///
/// `RenderGraph::remove_node_edge` 在边缺失时返回
/// `Err(RenderGraphError::EdgeDoesNotExist)`——其文档
/// 句 "if either node does not exist then nothing happens" 描述的是
/// *效果*，而非返回值，且该调用从不 panic。边缺失是
/// 本 helper 调用方的常态（它们在重新接线串行链之前防御性地删除，
/// 且删除必须保持幂等，以便该函数
/// 可从任意 plugin-build 顺序调用），因此该错误以
/// `debug!` 记录并被吞掉。
///
/// 当不存在 `RenderGraph` 资源或不存在 `Core3d`
/// 子图（无头 `MinimalPlugins`）时同样静默降级，
/// 与 `add_render_graph_*` 家族的 `warn!`-并-继续 姿态一致。
fn remove_core3d_edge(
    render_app: &mut bevy::app::SubApp,
    output_node: impl RenderLabel,
    input_node: impl RenderLabel,
) {
    let Some(mut render_graph) = render_app.world_mut().get_resource_mut::<RenderGraph>() else {
        return;
    };
    let Some(graph) = render_graph.get_sub_graph_mut(Core3d) else {
        return;
    };
    if let Err(err) = graph.remove_node_edge(output_node, input_node) {
        bevy::log::debug!("remove_core3d_edge: {err} (tolerated: edge was already absent)");
    }
}

/// 添加一条 `Core3d` 节点边，容忍已存在的边。
///
/// [`remove_core3d_edge`] 的对应函数，必需是因为
/// `RenderGraph::add_node_edge` 会对 `try_add_node_edge` 做 `unwrap()`，所以
/// 两次重建同一串行链（plugin build 顺序无法从
/// `main.rs` 完全控制；参见 `wiring_is_idempotent`）会在第一遍创建的
/// 节点内部边上以 `EdgeAlreadyExists` panic。[`wire_m6_edges`] 逐边重建其
/// 链，因此已存在的边是*正常*的第二次调用情形：
/// 只吞掉恰好该错误，其余失败仍保持响亮（`InvalidNode`
/// 仍意味着某节点从未注册——这正是固定元组
/// `add_render_graph_edges` 免费送上的性质）。无
/// `RenderGraph` / `Core3d`（无头 `MinimalPlugins`）时同样降级为 no-op。
fn add_core3d_edge(
    render_app: &mut bevy::app::SubApp,
    output_node: impl RenderLabel,
    input_node: impl RenderLabel,
) {
    let Some(mut render_graph) = render_app.world_mut().get_resource_mut::<RenderGraph>() else {
        return;
    };
    let Some(graph) = render_graph.get_sub_graph_mut(Core3d) else {
        return;
    };
    if let Err(err) = graph.try_add_node_edge(output_node, input_node) {
        match err {
            bevy::render::render_graph::RenderGraphError::EdgeAlreadyExists { .. } => {
                bevy::log::debug!("add_core3d_edge: {err} (tolerated: idempotent re-wire)");
            }
            other => panic!("add_core3d_edge failed: {other}"),
        }
    }
}

/// 创建一张全屏分辨率纹理，适用于后处理中间
/// 目标（例如 AO 模糊缓冲、FXAA 历史）。
///
/// 返回一个 `(Texture, TextureView)` 对。调用方负责
/// 生命周期管理（通常存于 render-world 资源/组件中）。
pub fn create_post_process_texture(
    render_device: &RenderDevice,
    label: &str,
    width: u32,
    height: u32,
    format: TextureFormat,
) -> (Texture, TextureView) {
    let size = Extent3d {
        width: width.max(1),
        height: height.max(1),
        depth_or_array_layers: 1,
    };

    let texture = render_device.create_texture(&TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::TEXTURE_BINDING
            | TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let view = texture.create_view(&TextureViewDescriptor::default());
    (texture, view)
}

// ─── 插件注册入口 ─────────────────────────────────────────

/// 在 `RenderApp` 中注册 cesium 渲染图基础设施。
///
/// 仅当 `postprocess_gate_enabled()` 返回 `true` 时才由
/// [`CesiumEffectsPlugin`](super::CesiumEffectsPlugin) 调用。门控 OFF = no-op = 零节点 =
/// 像素中性（v0 基线不变）。
///
/// 本函数演示完整模式：
/// 1. `get_sub_app_mut(RenderApp)`——无头安好（无 render plugin 时返回 `None`）
/// 2. 经 `shader_registry::try_load_internal_shader` 注册 shader
/// 3. 初始化 render-world 资源
/// 4. 向 `ExtractSchedule` 和 `Render` 添加系统
/// 5. 向 `Core3d` 子图添加节点 + 边
///
/// M5-E1：还注册 FXAA 节点（[`super::fxaa::register_fxaa_node`]）。
/// M5-E2：还注册 SSAO 节点（[`super::ao::register_ao_node`]）并
/// 接线**单一线性链**（Daniel H2，上游 CesiumJS 一致性）
/// `EndMainPass → PassThrough → AmbientOcclusion → Tonemapping → Fxaa →
/// EndMainPassPostProcessing`——AO 在 tonemapping 之前（HDR
/// 场景），FXAA 最后（最终 LDR 图像）。
/// 所有节点都在此处注册（从不孤立注册），所以它们的边不会
/// 形成菱形。pass-through 节点像素中性且默认禁用
///（`CesiumPassThrough::default().enabled == false` → 提前 return，零 GPU
/// 开销）；当相机携带 `CesiumFxaa { enabled: true }` 时 FXAA 运行。
#[deprecated = "DEV-029 / FIX-REG-FACADE: call `register_render_graph_main_world` from `Plugin::build` and `register_render_graph_render_world` (or `finish_render_graph`) from `Plugin::finish`; this facade runs the finish half against a possibly device-less render world."]
pub fn register_render_graph(app: &mut App) {
    register_render_graph_main_world(app);
    // RenderApp 装配——无头安好（无 RenderPlugin 时返回 None）。
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        register_render_graph_render_world(render_app);
    }
}

/// cesium 后处理渲染图的 `Plugin::finish` 入口。
///
/// **为何存在**（task #81，`docs/deviations.md#dev-029`）：本模块中每个
/// `*Pipeline::from_world` 都读取 `RenderDevice`，而 Bevy 只在
/// `RenderPlugin::finish`（`bevy_render/src/lib.rs` L399-430）中才把 `RenderDevice` /
/// `RenderQueue` / `RenderAdapter` 插入 render world——*不是*在其
/// `build` 中。所以从插件的 `build` 调用
/// `render_app.init_resource::<PassThroughPipeline>()`（及 FXAA/AO 对应物）会在
/// **任何**带 `CESIUM_ENABLE_POSTPROCESS=1` 的真实 GPU 运行上以
/// "Requested resource RenderDevice does not exist in the World" panic。Bevy 自身
/// 正是这样拆分的（`bevy_pbr/src/ssao/mod.rs` L54 `build` = 主 world，
/// L80 `finish` = `init_resource::<SsaoPipelines>()`），所以
/// [`CesiumEffectsPlugin`](super::CesiumEffectsPlugin) 现在从 `build` 调用
/// [`register_render_graph_main_world`]，从 `finish` 调用本函数。
///
/// 无头安好：当不存在 `RenderApp` 子 app 时为 no-op。
pub fn finish_render_graph(app: &mut App) {
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    register_render_graph_render_world(render_app);
}

/// [`register_render_graph`] 的 `Plugin::build` 时半：shader +
/// `ExtractComponentPlugin` + pass-through、FXAA 和 AO 节点的主-world 前置 pass 系统。
/// 只触碰**主** world，因此在 `RenderDevice` 存在之前是安全的。
pub fn register_render_graph_main_world(app: &mut App) {
    // 注册 pass-through WGSL shader（经 shader_registry 无头安好）。
    crate::shader_registry::try_load_internal_shader(
        app,
        PASS_THROUGH_SHADER_HANDLE,
        include_str!("../../shaders/pass_through.wgsl"),
        "shaders/pass_through.wgsl",
    );

    // 注册 ExtractComponentPlugin——内部使用 ExtractSchedule +
    // Extract<Query<(Entity, &CesiumPassThrough)>> 每帧将组件从主 world
    // 拷贝到 render world（与 Bevy FxaaPlugin 同模式）。
    app.add_plugins(ExtractComponentPlugin::<CesiumPassThrough>::default());

    // M5-E1：注册 FXAA 节点（shader + extract + render-world 节点/系统）。
    // M5-E2：注册 SSAO 节点（shader + extract + 节点 + 主-world 中的深度/法线
    //        前置 pass 启用）。边**不**在此创建——下方
    //        统一的链拥有这些边，所以 `Core3d` 中不会形成菱形。
    super::fxaa::register_fxaa_node_main_world(app);
    super::ao::register_ao_node_main_world(app);
}

/// [`register_render_graph`] 的 `Plugin::finish` 时半：render-world pipeline
/// 资源、`Render` schedule 系统、三个 `Core3d` 节点以及
/// 统一线性链（Daniel H2）。需要 `RenderDevice`，故为 `finish`。
pub fn register_render_graph_render_world(render_app: &mut bevy::app::SubApp) {
    // FIX-REG-FACADE（DEV-029）：当 `RenderDevice` 缺失时降级为 no-op
    //（finish 半从 `build` 达到，或一个裸 render world）——`PassThroughPipeline`
    // 和 FXAA/AO 子半都会解引用它。参见
    // `crate::effects::render_world_missing_device`。
    if crate::effects::render_world_missing_device(render_app) {
        return;
    }

    super::fxaa::register_fxaa_node_render_world(render_app);
    super::ao::register_ao_node_render_world(render_app);

    render_app
        .init_resource::<PassThroughPipeline>()
        .init_resource::<SpecializedRenderPipelines<PassThroughPipeline>>()
        // Render schedule：逐视图 pipeline 特化
        .add_systems(
            Render,
            prepare_pass_through_pipelines.in_set(RenderSet::Prepare),
        )
        // 渲染图节点注册
        .add_render_graph_node::<ViewNodeRunner<PassThroughNode>>(
            Core3d,
            CesiumPostProcessLabel::PassThrough,
        );

    // 统一线性链（Daniel H2——上游 CesiumJS 一致性）。CesiumJS 先运行
    // AO（在 HDR 场景上），然后 Bloom / AutoExposure / Tonemapping，然后
    // FXAA 最后（在最终 LDR 图像上）：PostProcessStageCollection.js L799-834。
    // 此前的链把 AO 放在 Tonemapping *之后* 却声称"与 CesiumJS 一致"
    // ——一个错误说法。已重排使 AO 先于 Tonemapping，且 FXAA 是最后一个
    // cesium 节点：
    //   EndMainPass → PassThrough → AmbientOcclusion → Tonemapping → Fxaa → EndMainPassPostProcessing
    // 单一元组链避免了 cesium 节点间的重复边；默认的
    // `EndMainPass → Tonemapping` 和 `Tonemapping → EndMainPassPostProcessing`
    // 边仍在但是冗余的（拓扑顺序仍把 PassThrough → AO 排在 Tonemapping 之前，
    // 把 Fxaa 排在 EndMainPassPostProcessing 之前）。AO 读取
    // 深度/法线前置 pass（在帧起始填充）和 HDR 颜色目标——两者在
    // `EndMainPass` 都可用。
    render_app.add_render_graph_edges(
        Core3d,
        (
            Node3d::EndMainPass,
            CesiumPostProcessLabel::PassThrough,
            CesiumPostProcessLabel::AmbientOcclusion,
            Node3d::Tonemapping,
            CesiumPostProcessLabel::Fxaa,
            Node3d::EndMainPassPostProcessing,
        ),
    );
}

// ─── M6 Wave A 集成（task #81） ───────────────────────────────────────

/// 注册 M6 Wave A 节点 + `Core3d` 边（task #81 集成）。
///
/// 在 [`CesiumEffectsPlugin`](super::CesiumEffectsPlugin) 已添加**之后**由
/// `application/cesium-app/src/main.rs` 调用，以便当 M5-E
/// 主门控为 ON 时 `PassThrough` / `AmbientOcclusion` / `Fxaa` 节点
/// 和 Robin 的 #72 H2 链已存在并可被拼接进去。
///
/// # 门控契约
/// M6.2 clipping / M6.3 panorama / M6.5 IBL 各自**仅**在其
/// 自身门控为 ON 时注册并接线。三者全 OFF（默认）时本函数
/// 在触碰任何东西之前就返回：无节点、无边，且——关键地——
/// 无删除，所以 `Core3d` 逐字节保持为 M6 之前的图，v0
/// 基线保持像素中性（PSNR=∞）。
///
/// # 无头
/// 当不存在 `RenderApp` 子 app（无头 `MinimalPlugins`）时优雅降级：
/// 主-world 插件仍被添加，但不尝试任何图操作。
#[deprecated = "DEV-029 / FIX-REG-FACADE: call `register_m6_render_graph_main_world` from `Plugin::build` and `register_m6_render_graph_render_world` from `Plugin::finish`; this facade runs the finish half against a possibly device-less render world."]
pub fn register_m6_render_graph(app: &mut App) {
    register_m6_render_graph_main_world(app);
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        register_m6_render_graph_render_world(render_app);
    }
}

/// M6 的门控值，**每阶段**从 env 读取一次。
///
/// `build` 和 `finish` 是独立的调用，所以门控在各自中重新读取；
/// env 是进程全局且在 app 生命周期内稳定，所以两阶段
/// 总是一致。保持私有：[`wire_m6_edges`] 是可测试的面。
#[derive(Clone, Copy, Debug)]
struct M6Gates {
    panorama: bool,
    clipping: bool,
    ibl: bool,
    oit: bool,
    clouds: bool,
    split: bool,
}

impl M6Gates {
    fn from_env() -> Self {
        Self {
            panorama: crate::effects::panorama::panorama_gate_enabled(),
            clipping: crate::effects::clipping_planes::clipping_gate_enabled(),
            ibl: crate::effects::ibl::ibl_gate_enabled(),
            oit: crate::effects::oit::oit_gate_enabled(),
            clouds: crate::effects::clouds::clouds_gate_enabled(),
            split: crate::effects::split::split_gate_enabled(),
        }
    }

    fn any(self) -> bool {
        self.panorama
            || self.clipping
            || self.ibl
            || self.oit
            || self.clouds
            || self.split
    }
}

/// [`register_m6_render_graph`] 的 `Plugin::build` 时半：WGSL shader +
/// `ExtractComponentPlugin` + 处于 ON 的各个 M6 门控的主-world 前置 pass 系统。
/// 仅主 world，所以不需要 `RenderDevice`。
pub fn register_m6_render_graph_main_world(app: &mut App) {
    let gates = M6Gates::from_env();
    if !gates.any() {
        return;
    }

    // 节点 + shader + ExtractComponentPlugin。每个 `register_*_node_main_world`
    // 刻意**不建任何边**，所以 [`register_m6_render_graph_render_world`] 中的
    // 单一接线点是唯一可能形成菱形的地方。
    if gates.panorama {
        crate::effects::panorama::register_panorama_node_main_world(app);
    }
    if gates.clipping {
        crate::effects::clipping_planes::register_clipping_planes_node_main_world(app);
    }
    if gates.ibl {
        crate::effects::ibl::register_ibl_node_main_world(app);
    }
    if gates.oit {
        crate::effects::oit::register_oit_node_main_world(app);
    }
    if gates.clouds {
        crate::effects::clouds::register_clouds_node_main_world(app);
    }
    if gates.split {
        crate::effects::split::register_split_node_main_world(app);
    }
}

/// [`register_m6_render_graph`] 的 `Plugin::finish` 时半：render-world
/// pipeline 资源 + `Core3d` 节点 + 边。必须从 `finish` 运行，
/// 因为每个 pipeline 的 `FromWorld` 读取 `RenderDevice`
///（`docs/deviations.md#dev-029`）。
pub fn register_m6_render_graph_render_world(render_app: &mut bevy::app::SubApp) {
    // FIX-REG-FACADE（DEV-029）：当 `RenderDevice` 缺失时降级为 no-op
    //（finish 半从 `build` 达到，或一个裸 render world）——下方每个逐节点
    // render 半和 `wire_m6_edges` 都需要设备。参见
    // `crate::effects::render_world_missing_device`。
    if crate::effects::render_world_missing_device(render_app) {
        return;
    }

    let gates = M6Gates::from_env();
    if !gates.any() {
        return;
    }

    if gates.panorama {
        crate::effects::panorama::register_panorama_node_render_world(render_app);
    }
    if gates.clipping {
        crate::effects::clipping_planes::register_clipping_planes_node_render_world(render_app);
    }
    if gates.ibl {
        crate::effects::ibl::register_ibl_node_render_world(render_app);
    }
    if gates.oit {
        crate::effects::oit::register_oit_node_render_world(render_app);
    }
    if gates.clouds {
        crate::effects::clouds::register_clouds_node_render_world(render_app);
    }
    if gates.split {
        crate::effects::split::register_split_node_render_world(render_app);
    }

    wire_m6_edges(
        render_app,
        gates.panorama,
        gates.clipping,
        gates.ibl,
        gates.oit,
        gates.clouds,
        gates.split,
        postprocess_gate_enabled(),
    );
}

/// `application/cesium-app/src/main.rs` 为 M6 Wave A（task #81）添加的插件。
///
/// 把注册拆分到 `build`/`finish` 是**强制的**，而非
/// 风格问题：`PanoramaPipeline` / `ClippingPlanesPipeline` / `IblPipeline` 都
/// 通过读取 `RenderDevice` 实现 `FromWorld`，而 Bevy 只在
/// `RenderPlugin::finish` 中才把它插入 render world。在 `build` 中做 render-world 半
/// 会在真实 GPU 上 panic（本地在三个 M6 门控全 ON 时已复现，
/// `panorama.rs:471`）。镜像 `bevy_pbr` 的 `ScreenSpaceAmbientOcclusionPlugin`。
///
/// 必须添加在 [`CesiumEffectsPlugin`](super::CesiumEffectsPlugin) **之后**
///（即在插件注册表中靠后），以便当 M5-E 主门控 ON 时，
/// [`wire_m6_edges`] 拼接进去时 Robin 的 #72 H2 链已存在。
///
/// 门控契约：三个 M6 门控全 OFF（默认）时两半都立即返回
///——无 shader、无插件、无节点、无边、无删除——所以 `Core3d`
/// 逐字节保持为 M6 之前的图，v0 基线保持
/// 像素中性（PSNR=∞）。
pub struct M6WaveARenderGraphPlugin;

impl bevy::app::Plugin for M6WaveARenderGraphPlugin {
    fn build(&self, app: &mut App) {
        register_m6_render_graph_main_world(app);
    }

    fn finish(&self, app: &mut App) {
        // 无头 `MinimalPlugins`（以及任何无 `RenderPlugin` 的 app）没有
        // `RenderApp` 子 app——优雅降级而非 panic。
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        register_m6_render_graph_render_world(render_app);
    }
}

/// 为一个**显式**门控组合接线 M6 `Core3d` 边。
///
/// 从 [`register_m6_render_graph`] 拆分出来，以便每个组合都可测试，
/// 无需修改进程全局 env 变量（那会与套件其余部分产生竞争）。
/// 覆盖 Wave A 门控（panorama / clipping / ibl）加上 Phase-3 FIX-INTEG
/// 门控（oit / clouds）。
///
/// # 所产生的链形状
///
/// Panorama——主 pass 内部的串行插入：
/// `MainOpaquePass → CesiumPanoramaLabel → MainTransmissivePass`。
///
/// 为何在 `MainOpaquePass` *之后*：Bevy 的 `texture_attachment.rs` 在视图目标的
/// **首次**使用时发出 `LoadOp::Clear`，之后发 `LoadOp::Load`，所以在不透明 pass
/// 之前绘制的 panorama 会被它直接擦除。之后绘制意味着 panorama 只填充地球
/// 留在远深度的像素，且较后的 `MainTransparentPass`（starfield r=50 → sky dome
/// r=40）仍会在其上绘制。sky dome 的三种排序机制
///（depth_bias / `Premultiplied` / `cull_mode: Front`）**未**被触碰。
///
/// OIT（Phase-3 FIX-INTEG）——拼接到透明尾部的一对两节点串行：`MainTransparentPass →
/// CesiumOitLabel → CesiumOitCompositeLabel →
/// EndMainPass`。累积节点紧接 Bevy 自身的透明 pass 之后绘制，
/// 合成在 `EndMainPass` 之前完成，所以后处理区域看到
/// 混合结果。与下方 EndMainPass 区域相互独立，所以 OIT 门控
/// 既不倍增 EndMainPass 组合，也不受其影响。
///（把 Bevy 的透明几何忠实地重路由*进* MRT 目标
/// 需要一个透明阶段 render-mesh pipeline modifier——已推迟；参见
/// `docs/deviations.md#dev-031`。）
///
/// Clipping + IBL + Clouds——在后处理（HDR）区域的串行插入：
/// `EndMainPass → [CesiumClippingLabel] → [CesiumIblLabel] → [CesiumCloudsLabel] →
/// <successor>`，
/// 其中 `<successor>` 在 M5-E 主门控 ON 时为 `CesiumPostProcessLabel::PassThrough`
/// ——所以 Robin 的 #72 H2 链
/// `PassThrough → AmbientOcclusion → Tonemapping → Fxaa →
/// EndMainPassPostProcessing`（AO 在 tonemapping 之前的 HDR 上，FXAA 最后
/// 在 LDR 上；上游 CesiumJS `PostProcessStageCollection.js` 一致性）被逐字
/// 保留——门控 OFF 时为 `Node3d::Tonemapping`（Bevy 自身对 `EndMainPass` 的默认
/// 后继）。
///
/// 全部 ON 时的完整形状：
/// `MainTransparentPass → Oit → OitComposite → EndMainPass → Clipping → Ibl →
/// Clouds → PassThrough → AmbientOcclusion → Tonemapping → Fxaa →
/// EndMainPassPostProcessing`。
///
/// 三个 EndMainPass 区域节点都是屏幕空间的，它们的相对顺序是一条
/// 已记录的偏差（`docs/deviations.md`）：IBL 加法地把
/// 环境光注入 HDR 场景，clipping 重绘被裁削的区域，
/// 而云合成体积天空颜色，所以 clipping 先运行并胜出。
/// 忠实上游路径（材质 shader 中逐片元 `discard` 用于
/// clipping、逐材质 IBL 因子、逐云 billboard）已被推迟。
//
// 元数是有意为之的：每个 M6 门控都是独立的 `bool`，所以每个子集
// 组合都能在无头下被驱动，无需修改进程全局 env 变量
//（那会与套件其余部分竞争）。一个 `M6Gates`-by-value 参数
// 被否决，因为接线测试独立驱动这些标志。
#[allow(clippy::too_many_arguments)]
pub fn wire_m6_edges(
    render_app: &mut bevy::app::SubApp,
    panorama: bool,
    clipping: bool,
    ibl: bool,
    oit: bool,
    clouds: bool,
    split: bool,
    postprocess: bool,
) {
    if panorama {
        // `insert_node_in_core3d` 先删除 `MainOpaquePass → MainTransmissivePass`，
        // 所以 panorama 是它们之间*唯一*的路径而非两条之一
        //（Lee 的 M6.3 菱形警告；与 Daniel H2 同一缺陷类）。
        insert_node_in_core3d(
            render_app,
            crate::effects::panorama::CesiumPanoramaLabel,
            Node3d::MainOpaquePass,
            Node3d::MainTransmissivePass,
        );
    }

    // OIT——在透明尾部它自己的串行槽位，独立于下方
    // EndMainPass 后处理区域：`MainTransparentPass → Oit →
    // OitComposite → EndMainPass`。累积节点紧接 Bevy
    // 自身透明 pass（其 MRT 目标）之后运行，合成在
    // `EndMainPass` 之前完成，所以下游后处理节点看到混合结果。
    // 两次链式 `insert_node_in_core3d` 调用使其无菱形；门控
    // OFF 时此处什么都不触碰图（v0 中性）。
    if oit {
        insert_node_in_core3d(
            render_app,
            crate::effects::oit::CesiumOitLabel,
            Node3d::MainTransparentPass,
            Node3d::EndMainPass,
        );
        insert_node_in_core3d(
            render_app,
            crate::effects::oit::CesiumOitCompositeLabel,
            crate::effects::oit::CesiumOitLabel,
            Node3d::EndMainPass,
        );
    }

    // Clipping + IBL + Clouds + Split——在后处理（HDR）区域的串行插入，
    // 终止于后处理头（M5-E 主门控 ON 时为 `PassThrough`），
    // 否则为 Bevy 自身的 `Tonemapping`。四者全无 ON 时整个区域被跳过，
    // 所以 `EndMainPass` 的边恰好保持 Bevy 离开时的样子
    //（v0 像素中性）。
    if !(clipping || ibl || clouds || split) {
        return;
    }

    // 以类型擦除的 `InternedRenderLabel` 构建有序链，使三个可选节点的
    // 任何子集都能组合，而无需两节点情形所需的组合式 `match`。
    // `InternedRenderLabel` 是 `Copy` 且实现 `RenderLabel`，所以连续的对可直接
    // 喂给 `add_render_graph_edge`。
    let successor: InternedRenderLabel = if postprocess {
        CesiumPostProcessLabel::PassThrough.intern()
    } else {
        Node3d::Tonemapping.intern()
    };

    // 在通过新节点重建单一串行路径之前，必须丢弃 `EndMainPass → successor`
    // 边，否则图仍是一个菱形（拓扑排序可能绕过插入的节点）。防御性地
    // 丢弃*两个*候选：postprocess OFF 时只有 `Tonemapping` 存在；
    // ON 时，`register_render_graph` 也留下了冗余的直连边。
    remove_core3d_edge(render_app, Node3d::EndMainPass, Node3d::Tonemapping);
    if postprocess {
        remove_core3d_edge(
            render_app,
            Node3d::EndMainPass,
            CesiumPostProcessLabel::PassThrough,
        );
    }

    let end_main_pass = Node3d::EndMainPass.intern();
    let mut chain: Vec<InternedRenderLabel> = Vec::with_capacity(5);
    chain.push(end_main_pass);
    if clipping {
        chain.push(crate::effects::clipping_planes::CesiumClippingLabel.intern());
    }
    if ibl {
        chain.push(crate::effects::ibl::CesiumIblLabel.intern());
    }
    if clouds {
        chain.push(crate::effects::clouds::CesiumCloudsLabel.intern());
    }
    if split {
        chain.push(crate::effects::split::CesiumSplitLabel.intern());
    }
    chain.push(successor);

    // 连续的成对边。每个节点恰好有一个后继 ⇒ 严格
    // 串行链，永非菱形。`add_core3d_edge`（而非会 panic 的
    // `add_render_graph_edge`）容忍更早接线的 pass 已创建的链内边，
    // 所以重跑是幂等的（`wiring_is_idempotent`）。
    // `add_render_graph_edge` 是单一的 Bevy API（`IntoRenderNodeArray` 只
    // 覆盖固定元组，无法表达依赖门控的节点列表）。
    for pair in chain.windows(2) {
        add_core3d_edge(render_app, pair[0], pair[1]);
    }
}

// ─── 测试 ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// 门控 OFF：`register_render_graph` 从不被调用 → 图中无节点。
    /// 这是证明像素中性的结构性断言（PSNR=∞）。
    #[test]
    fn gate_off_no_render_graph_registration() {
        // 纯逻辑测试：无需操作 env 变量。
        assert!(!gate_from_env_value(None));
        assert!(!gate_from_env_value(Some("0".into())));
        assert!(!gate_from_env_value(Some("false".into())));

        // MinimalPlugins app (headless, no RenderApp).
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // 不要调用 register_render_graph（那是 CesiumEffectsPlugin 在门控 OFF 时所做的）。
        // 结构性断言：不存在 RenderApp 子 app → 图无法包含我们的节点。
        assert!(app.get_sub_app_mut(RenderApp).is_none());
    }

    /// 门控 ON 但无头（无 RenderApp）：`register_render_graph` 优雅降级。
    #[test]
    fn gate_on_headless_graceful_degradation() {
        // 验证解析接受 "1" 和 "true"（大小写不敏感）。
        assert!(gate_from_env_value(Some("1".into())));
        assert!(gate_from_env_value(Some("true".into())));
        assert!(gate_from_env_value(Some("TRUE".into())));
        assert!(gate_from_env_value(Some("True".into())));

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // 不应 panic——因为 get_sub_app_mut(RenderApp) 返回 None 而降级。
        //（模拟无头环境下的门控 ON。）
        #[allow(deprecated)]
        register_render_graph(&mut app);
    }

    /// FIX-REG-FACADE（DEV-029 收口）：一个*存在*但不携带
    /// `RenderDevice` 的 render world——正是插件 `build` 期间的状态，也是无头
    /// `MinimalPlugins` app 永远无法复现的（它根本没有 `RenderApp`，
    /// 所以该误用此前被隐藏，而*无头优雅*测试是空洞地通过的）。
    /// 现在每个 `register_*_render_world` 半必须降级为 no-op 而非 panic，
    /// 且不留下**任何**半初始化的 pipeline
    /// 资源。这是 §8.10 要求用来打破恒绿模式的断言：
    /// 去掉守卫，调用就会在此 panic。
    #[test]
    fn render_world_without_device_degrades_to_noop() {
        // 一个裸 render world：无 `RenderDevice`，从 `Plugin::build` 所见。
        let mut render_app = bevy::app::SubApp::new();
        assert!(
            render_app.world().get_resource::<RenderDevice>().is_none(),
            "fixture must have no RenderDevice for the guard to trigger",
        );

        // 后处理 render 半（PassThrough + FXAA + AO + 边）。
        register_render_graph_render_world(&mut render_app);
        assert!(
            render_app
                .world()
                .get_resource::<PassThroughPipeline>()
                .is_none(),
            "guard must skip PassThroughPipeline init when RenderDevice is absent",
        );

        // 一个具代表性的 M6 逐节点 render 半（split）。
        crate::effects::split::register_split_node_render_world(&mut render_app);
        assert!(
            render_app
                .world()
                .get_resource::<crate::effects::split::SplitPipeline>()
                .is_none(),
            "guard must skip SplitPipeline init when RenderDevice is absent",
        );

        // m6 总括 render 半同样受设备门控（提前返回）。
        register_m6_render_graph_render_world(&mut render_app);
    }

    /// 门控解析边界情况（Daniel H1：现为权威的 4-token truthy
    /// 集 `{1, true, yes, on}`，trim + 转小写——`pipeline::fetch`）。
    #[test]
    fn gate_env_parsing_edge_cases() {
        assert!(!gate_from_env_value(Some("".into())));
        assert!(!gate_from_env_value(Some("2".into())));
        assert!(!gate_from_env_value(Some("off".into())));
        assert!(!gate_from_env_value(Some("no".into())));
        // `yes` / `on` 被旧的 2-token 本地拷贝拒绝；权威
        // 解析器接受它们（大小写不敏感）。
        assert!(gate_from_env_value(Some("yes".into())));
        assert!(gate_from_env_value(Some("on".into())));
        assert!(gate_from_env_value(Some("YES".into())));
        assert!(gate_from_env_value(Some("On".into())));
        // trim + 转小写——旧拷贝两者都不做。
        assert!(gate_from_env_value(Some(" 1 ".into())));
        assert!(gate_from_env_value(Some("\ttrue\n".into())));
        assert!(gate_from_env_value(Some(" True ".into())));
    }

    /// CesiumPassThrough 组件默认值：禁用（保守）。
    #[test]
    fn component_default_disabled() {
        let c = CesiumPassThrough::default();
        assert!(!c.enabled);
    }

    /// Label 枚举：确保所有计划中的变体在编译期存在。
    #[test]
    fn labels_compile_time_existence() {
        let _pt = CesiumPostProcessLabel::PassThrough;
        let _fxaa = CesiumPostProcessLabel::Fxaa;
        let _ao = CesiumPostProcessLabel::AmbientOcclusion;
    }

    /// M5-E1：两个后处理门控读取不同的 env 变量，所以 FXAA/AO
    ///（`CESIUM_ENABLE_POSTPROCESS`）和 tonemapping/bloom/fog
    ///（`CESIUM_ENABLE_POSTPROCESS_BUILTIN`）可独立切换
    ///（leader 裁决 Q5）。通过 const 名断言以避免 env 变量竞争。
    #[test]
    fn postprocess_gates_are_independent() {
        assert_ne!(ENV_ENABLE_POSTPROCESS, ENV_ENABLE_POSTPROCESS_BUILTIN);
        assert_eq!(ENV_ENABLE_POSTPROCESS, "CESIUM_ENABLE_POSTPROCESS");
        assert_eq!(ENV_ENABLE_POSTPROCESS_BUILTIN, "CESIUM_ENABLE_POSTPROCESS_BUILTIN");
        // 两者使用相同的 truthy 谓词（纯函数，无 env 修改）。
        assert!(gate_from_env_value(Some("1".into())));
        assert!(!gate_from_env_value(None));
    }

    // ─── M6 Wave A 接线测试（task #81） ────────────────────────────────────

    // 在此导入而非文件顶部：`EmptyNode` 只被 fixture 需要，
    // 且 crate 级导入在非测试构建中会 `unused`
    //（clippy `-D warnings`）。
    use bevy::render::render_graph::EmptyNode;

    /// 构建一个 `App`，其 world 携带一个复现 Bevy 0.15.3 默认链
    ///（`bevy_core_pipeline/src/core_3d/mod.rs` L193-209）的 `Core3d` 子图：
    /// `StartMainPass → MainOpaquePass → MainTransmissivePass →
    /// MainTransparentPass → EndMainPass → Tonemapping →
    /// EndMainPassPostProcessing → Upscaling`。
    ///
    /// `with_postprocess` 额外添加 cesium M5-E 节点和 Robin 的
    /// #72 H2 链，与 `register_render_graph` 构建它的样子完全一致。
    fn core3d_fixture(with_postprocess: bool) -> App {
        let mut app = App::new();
        let mut sub = RenderGraph::default();

        for label in [
            Node3d::StartMainPass,
            Node3d::MainOpaquePass,
            Node3d::MainTransmissivePass,
            Node3d::MainTransparentPass,
            Node3d::EndMainPass,
            Node3d::Tonemapping,
            Node3d::EndMainPassPostProcessing,
            Node3d::Upscaling,
        ] {
            sub.add_node(label, EmptyNode);
        }
        sub.add_node_edges((
            Node3d::StartMainPass,
            Node3d::MainOpaquePass,
            Node3d::MainTransmissivePass,
            Node3d::MainTransparentPass,
            Node3d::EndMainPass,
            Node3d::Tonemapping,
            Node3d::EndMainPassPostProcessing,
            Node3d::Upscaling,
        ));

        // M6 节点本身在 fixture 中总是存在：在生产中 `register_*_node`
        // 在 `wire_m6_edges` 运行前添加它们，且 `add_render_graph_edges`
        // 会对无节点的 label panic。
        sub.add_node(crate::effects::panorama::CesiumPanoramaLabel, EmptyNode);
        sub.add_node(crate::effects::clipping_planes::CesiumClippingLabel, EmptyNode);
        sub.add_node(crate::effects::ibl::CesiumIblLabel, EmptyNode);
        // Phase-3 FIX-INTEG：OIT（accumulate + composite）和 Clouds 节点
        // 在 fixture 中也总是存在（生产 `register_*_node` 在
        // `wire_m6_edges` 前添加它们），所以门控组合可在无头下被驱动。
        sub.add_node(crate::effects::oit::CesiumOitLabel, EmptyNode);
        sub.add_node(crate::effects::oit::CesiumOitCompositeLabel, EmptyNode);
        sub.add_node(crate::effects::clouds::CesiumCloudsLabel, EmptyNode);
        sub.add_node(crate::effects::split::CesiumSplitLabel, EmptyNode);

        if with_postprocess {
            sub.add_node(CesiumPostProcessLabel::PassThrough, EmptyNode);
            sub.add_node(CesiumPostProcessLabel::AmbientOcclusion, EmptyNode);
            sub.add_node(CesiumPostProcessLabel::Fxaa, EmptyNode);
            sub.add_node_edges((
                Node3d::EndMainPass,
                CesiumPostProcessLabel::PassThrough,
                CesiumPostProcessLabel::AmbientOcclusion,
                Node3d::Tonemapping,
                CesiumPostProcessLabel::Fxaa,
                Node3d::EndMainPassPostProcessing,
            ));
        }

        let mut root = RenderGraph::default();
        root.add_sub_graph(Core3d, sub);
        app.insert_resource(root);
        app
    }

    /// 节点出站邻居的排序 debug label。
    fn out_labels(app: &App, label: impl RenderLabel) -> Vec<String> {
        let graph = app.world().resource::<RenderGraph>();
        let sub = graph.get_sub_graph(Core3d).expect("Core3d sub-graph");
        let mut v: Vec<String> = sub
            .iter_node_outputs(label)
            .expect("node exists")
            .map(|(_edge, node)| format!("{:?}", node.label))
            .collect();
        v.sort();
        v
    }

    /// 节点入站邻居的排序 debug label。
    fn in_labels(app: &App, label: impl RenderLabel) -> Vec<String> {
        let graph = app.world().resource::<RenderGraph>();
        let sub = graph.get_sub_graph(Core3d).expect("Core3d sub-graph");
        let mut v: Vec<String> = sub
            .iter_node_inputs(label)
            .expect("node exists")
            .map(|(_edge, node)| format!("{:?}", node.label))
            .collect();
        v.sort();
        v
    }

    fn name(label: impl RenderLabel) -> String {
        format!("{:?}", label)
    }

    /// **所有 M6 门控 OFF 是严格的 no-op**：不加一条边，也不删一条
    /// 边。这是 v0 像素中性的结构性证明（PSNR=∞）。
    #[test]
    fn all_gates_off_leaves_the_default_chain_untouched() {
        for pp in [false, true] {
            let app = core3d_fixture(pp);
            let before: Vec<(String, Vec<String>)> = [
                Node3d::MainOpaquePass,
                Node3d::MainTransmissivePass,
                Node3d::MainTransparentPass,
                Node3d::EndMainPass,
                Node3d::Tonemapping,
            ]
            // `Node3d` 是 `Clone` 但**不是** `Copy`（`Dyn` 变体拥有一个
            // `InternedRenderLabel`），所以按值 `array::map` + 一次 `clone` 是把
            // 同一 label 喂给两个按值参数的、move 干净的方式。
            .map(|l| (name(l.clone()), out_labels(&app, l)))
            .to_vec();

            let mut app = app;
            wire_m6_edges(app.main_mut(), false, false, false, false, false, false, pp);

            let after: Vec<(String, Vec<String>)> = [
                Node3d::MainOpaquePass,
                Node3d::MainTransmissivePass,
                Node3d::MainTransparentPass,
                Node3d::EndMainPass,
                Node3d::Tonemapping,
            ]
            .map(|l| (name(l.clone()), out_labels(&app, l)))
            .to_vec();

            assert_eq!(before, after, "gate-OFF must not touch Core3d (pp={pp})");
            // 且 M6 节点保持孤立：完全无边。
            assert!(out_labels(&app, crate::effects::panorama::CesiumPanoramaLabel).is_empty());
            assert!(out_labels(&app, crate::effects::clipping_planes::CesiumClippingLabel).is_empty());
            assert!(out_labels(&app, crate::effects::ibl::CesiumIblLabel).is_empty());
            assert!(out_labels(&app, crate::effects::oit::CesiumOitLabel).is_empty());
            assert!(out_labels(&app, crate::effects::oit::CesiumOitCompositeLabel).is_empty());
            assert!(out_labels(&app, crate::effects::clouds::CesiumCloudsLabel).is_empty());
            assert!(out_labels(&app, crate::effects::split::CesiumSplitLabel).is_empty());
            // Bevy 拥有的透明尾部保持原样：MainTransparentPass →
            // EndMainPass 直连（门控 OFF 时不拼入任何 OIT 节点）。
            assert_eq!(
                out_labels(&app, Node3d::MainTransparentPass),
                vec![name(Node3d::EndMainPass)]
            );
        }
    }

    /// Panorama 插入是**串行，非菱形**（Lee 的 M6.3 警告）：
    /// 已存在的 `MainOpaquePass → MainTransmissivePass` 边必须
    /// 消失，只留一条经由 `CesiumPanoramaLabel` 的路径。
    #[test]
    fn panorama_insertion_is_serial_not_a_diamond() {
        let mut app = core3d_fixture(false);
        wire_m6_edges(app.main_mut(), true, false, false, false, false, false, false);

        assert_eq!(
            out_labels(&app, Node3d::MainOpaquePass),
            vec![name(crate::effects::panorama::CesiumPanoramaLabel)],
            "MainOpaquePass must have exactly ONE successor (the diamond edge removed)"
        );
        assert_eq!(
            in_labels(&app, Node3d::MainTransmissivePass),
            vec![name(crate::effects::panorama::CesiumPanoramaLabel)],
            "MainTransmissivePass must have exactly ONE predecessor"
        );
        assert_eq!(
            out_labels(&app, crate::effects::panorama::CesiumPanoramaLabel),
            vec![name(Node3d::MainTransmissivePass)]
        );
        // 下游顺序未被触碰：透明 pass（starfield r=50
        // → sky dome r=40）仍跟随透射 pass。
        assert_eq!(
            out_labels(&app, Node3d::MainTransmissivePass),
            vec![name(Node3d::MainTransparentPass)]
        );
    }

    /// Clipping + IBL 拼接进 HDR 区域，**不扰动** Robin 的 #72 H2 链
    ///（`PassThrough → AmbientOcclusion → Tonemapping → Fxaa →
    /// EndMainPassPostProcessing`）。
    #[test]
    fn clipping_and_ibl_prepend_the_h2_chain() {
        let mut app = core3d_fixture(true);
        wire_m6_edges(app.main_mut(), false, true, true, false, false, false, true);

        assert_eq!(
            out_labels(&app, Node3d::EndMainPass),
            vec![name(crate::effects::clipping_planes::CesiumClippingLabel)],
            "the redundant direct EndMainPass→Tonemapping edge must be gone too"
        );
        assert_eq!(
            out_labels(&app, crate::effects::clipping_planes::CesiumClippingLabel),
            vec![name(crate::effects::ibl::CesiumIblLabel)]
        );
        assert_eq!(
            out_labels(&app, crate::effects::ibl::CesiumIblLabel),
            vec![name(CesiumPostProcessLabel::PassThrough)]
        );
        // H2 重排被逐字保留：AO 在 tonemapping 之前的 HDR 场景上，
        // FXAA 最后于 LDR 图像。
        assert_eq!(
            out_labels(&app, CesiumPostProcessLabel::PassThrough),
            vec![name(CesiumPostProcessLabel::AmbientOcclusion)]
        );
        assert_eq!(
            out_labels(&app, CesiumPostProcessLabel::AmbientOcclusion),
            vec![name(Node3d::Tonemapping)]
        );
        let tone = out_labels(&app, Node3d::Tonemapping);
        assert!(tone.contains(&name(CesiumPostProcessLabel::Fxaa)), "got {tone:?}");
        assert_eq!(
            out_labels(&app, CesiumPostProcessLabel::Fxaa),
            vec![name(Node3d::EndMainPassPostProcessing)]
        );
        // Tonemapping 不得新增一个直连的 EndMainPass 前驱。
        assert!(!in_labels(&app, Node3d::Tonemapping)
            .contains(&name(Node3d::EndMainPass)));
    }

    /// M5-E 主门控 OFF 时 cesium 后处理节点不存在，
    /// 所以 clipping/IBL 必须拼接到 Bevy 自身的 `Tonemapping` 后继，
    /// 且从不引用 `PassThrough`（那会以 `InvalidNode` panic）。
    #[test]
    fn clipping_only_without_postprocess_targets_tonemapping() {
        let mut app = core3d_fixture(false);
        wire_m6_edges(app.main_mut(), false, true, false, false, false, false, false);

        assert_eq!(
            out_labels(&app, Node3d::EndMainPass),
            vec![name(crate::effects::clipping_planes::CesiumClippingLabel)]
        );
        assert_eq!(
            out_labels(&app, crate::effects::clipping_planes::CesiumClippingLabel),
            vec![name(Node3d::Tonemapping)]
        );
        assert_eq!(
            out_labels(&app, Node3d::Tonemapping),
            vec![name(Node3d::EndMainPassPostProcessing)]
        );
        // IBL 保持孤立。
        assert!(out_labels(&app, crate::effects::ibl::CesiumIblLabel).is_empty());
    }

    /// 仅 IBL，主门控 ON。
    #[test]
    fn ibl_only_with_postprocess_targets_pass_through() {
        let mut app = core3d_fixture(true);
        wire_m6_edges(app.main_mut(), false, false, true, false, false, false, true);

        assert_eq!(
            out_labels(&app, Node3d::EndMainPass),
            vec![name(crate::effects::ibl::CesiumIblLabel)]
        );
        assert_eq!(
            out_labels(&app, crate::effects::ibl::CesiumIblLabel),
            vec![name(CesiumPostProcessLabel::PassThrough)]
        );
        assert!(out_labels(&app, crate::effects::clipping_planes::CesiumClippingLabel).is_empty());
    }

    /// 三个 M6 门控同时 ON：主 pass 中的 panorama *以及*
    /// HDR 区域中的 clipping+IBL，两者皆串行。
    #[test]
    fn all_three_gates_on_compose_without_diamonds() {
        let mut app = core3d_fixture(true);
        wire_m6_edges(app.main_mut(), true, true, true, false, false, false, true);

        assert_eq!(out_labels(&app, Node3d::MainOpaquePass).len(), 1);
        assert_eq!(in_labels(&app, Node3d::MainTransmissivePass).len(), 1);
        assert_eq!(out_labels(&app, Node3d::EndMainPass).len(), 1);
        assert_eq!(
            out_labels(&app, Node3d::EndMainPass),
            vec![name(crate::effects::clipping_planes::CesiumClippingLabel)]
        );
        // 每个 M6 节点至多一个后继：任何地方都无菱形。
        assert_eq!(out_labels(&app, crate::effects::panorama::CesiumPanoramaLabel).len(), 1);
        assert_eq!(out_labels(&app, crate::effects::clipping_planes::CesiumClippingLabel).len(), 1);
        assert_eq!(out_labels(&app, crate::effects::ibl::CesiumIblLabel).len(), 1);
    }

    /// FIX-GRAPH-WIRING：`panorama=T, clipping=T, ibl=F`——此前未测试的
    /// 8 组合矩阵中的一个单元格。主-pass panorama 和 HDR 区域
    /// clipping 各自**串行**组合（每个恰好一个后继）；IBL 因其门控
    /// 关闭而保持孤立。
    #[test]
    fn panorama_and_clipping_without_ibl_compose_serially() {
        let mut app = core3d_fixture(true);
        wire_m6_edges(app.main_mut(), true, true, false, false, false, false, true);

        // Panorama 是 MainOpaquePass → MainTransmissivePass 的唯一路径。
        assert_eq!(
            out_labels(&app, Node3d::MainOpaquePass),
            vec![name(crate::effects::panorama::CesiumPanoramaLabel)]
        );
        assert_eq!(
            in_labels(&app, Node3d::MainTransmissivePass),
            vec![name(crate::effects::panorama::CesiumPanoramaLabel)]
        );
        // Clipping 拼接到后处理头（主门控 ON）。
        assert_eq!(
            out_labels(&app, Node3d::EndMainPass),
            vec![name(crate::effects::clipping_planes::CesiumClippingLabel)]
        );
        assert_eq!(
            out_labels(&app, crate::effects::clipping_planes::CesiumClippingLabel),
            vec![name(CesiumPostProcessLabel::PassThrough)]
        );
        // IBL 门控 off ⇒ 节点孤立（无边）。
        assert!(
            out_labels(&app, crate::effects::ibl::CesiumIblLabel).is_empty(),
            "ibl gate off ⇒ node stays isolated"
        );
        // 无菱形：各一个后继。
        assert_eq!(
            out_labels(&app, crate::effects::panorama::CesiumPanoramaLabel).len(),
            1
        );
        assert_eq!(
            out_labels(&app, crate::effects::clipping_planes::CesiumClippingLabel).len(),
            1
        );
    }

    /// FIX-GRAPH-WIRING：`panorama=T, clipping=F, ibl=T`——8 组合矩阵
    /// 最后一个未覆盖的单元格。Panorama + IBL 各自串行组合；clipping
    /// 保持孤立。
    #[test]
    fn panorama_and_ibl_without_clipping_compose_serially() {
        let mut app = core3d_fixture(true);
        wire_m6_edges(app.main_mut(), true, false, true, false, false, false, true);

        assert_eq!(
            out_labels(&app, Node3d::MainOpaquePass),
            vec![name(crate::effects::panorama::CesiumPanoramaLabel)]
        );
        assert_eq!(
            in_labels(&app, Node3d::MainTransmissivePass),
            vec![name(crate::effects::panorama::CesiumPanoramaLabel)]
        );
        // IBL 拼接到后处理头；clipping 未触碰。
        assert_eq!(
            out_labels(&app, Node3d::EndMainPass),
            vec![name(crate::effects::ibl::CesiumIblLabel)]
        );
        assert_eq!(
            out_labels(&app, crate::effects::ibl::CesiumIblLabel),
            vec![name(CesiumPostProcessLabel::PassThrough)]
        );
        assert!(
            out_labels(&app, crate::effects::clipping_planes::CesiumClippingLabel).is_empty(),
            "clipping gate off ⇒ node stays isolated"
        );
        assert_eq!(
            out_labels(&app, crate::effects::panorama::CesiumPanoramaLabel).len(),
            1
        );
        assert_eq!(
            out_labels(&app, crate::effects::ibl::CesiumIblLabel).len(),
            1
        );
    }

    /// FIX-INTEG（Phase 3）：仅 OIT 在透明尾部组合成一条串行链
    ///——`MainTransparentPass → Oit → OitComposite → EndMainPass`——已删除
    /// 现存的 `MainTransparentPass → EndMainPass` 边（无菱形），
    /// 且不触碰 EndMainPass 后处理区域（clipping/ibl/
    /// clouds 门控关闭）。
    #[test]
    fn oit_composes_serially_in_transparent_tail() {
        let mut app = core3d_fixture(true);
        wire_m6_edges(app.main_mut(), false, false, false, true, false, false, true);

        use crate::effects::oit::{CesiumOitCompositeLabel, CesiumOitLabel};
        assert_eq!(
            out_labels(&app, Node3d::MainTransparentPass),
            vec![name(CesiumOitLabel)],
            "the direct MainTransparentPass→EndMainPass edge must be gone"
        );
        assert_eq!(out_labels(&app, CesiumOitLabel), vec![name(CesiumOitCompositeLabel)]);
        assert_eq!(
            out_labels(&app, CesiumOitCompositeLabel),
            vec![name(Node3d::EndMainPass)]
        );
        assert_eq!(in_labels(&app, Node3d::EndMainPass), vec![name(CesiumOitCompositeLabel)]);
        // 后处理区域未触碰（clipping/ibl/clouds 全 OFF）：M6 节点
        // 保持孤立，EndMainPass 保留 Bevy 自身的默认后继
        //（v0 中性——只有当其中之一 ON 时该区域才被拼接）。
        assert!(out_labels(&app, crate::effects::clipping_planes::CesiumClippingLabel).is_empty());
        assert!(out_labels(&app, crate::effects::ibl::CesiumIblLabel).is_empty());
        assert!(out_labels(&app, crate::effects::clouds::CesiumCloudsLabel).is_empty());
    }

    /// FIX-INTEG（Phase 3）：Clouds 拼接到 EndMainPass HDR 链，当那些门控也 ON 时
    /// 位于 clipping + IBL 之后，目标是后处理头。
    /// `EndMainPass → Clipping → Ibl → Clouds → PassThrough`。
    #[test]
    fn clouds_append_the_h2_chain() {
        let mut app = core3d_fixture(true);
        wire_m6_edges(app.main_mut(), false, true, true, false, true, false, true);

        use crate::effects::clipping_planes::CesiumClippingLabel;
        use crate::effects::clouds::CesiumCloudsLabel;
        use crate::effects::ibl::CesiumIblLabel;
        assert_eq!(out_labels(&app, Node3d::EndMainPass), vec![name(CesiumClippingLabel)]);
        assert_eq!(out_labels(&app, CesiumClippingLabel), vec![name(CesiumIblLabel)]);
        assert_eq!(out_labels(&app, CesiumIblLabel), vec![name(CesiumCloudsLabel)]);
        assert_eq!(
            out_labels(&app, CesiumCloudsLabel),
            vec![name(CesiumPostProcessLabel::PassThrough)]
        );
        // 仅 Clouds 门控 ON（无 clipping/ibl）仍落在 PassThrough 上。
        let mut app2 = core3d_fixture(true);
        wire_m6_edges(app2.main_mut(), false, false, false, false, true, false, true);
        assert_eq!(
            out_labels(&app2, Node3d::EndMainPass),
            vec![name(CesiumCloudsLabel)]
        );
        assert_eq!(
            out_labels(&app2, CesiumCloudsLabel),
            vec![name(CesiumPostProcessLabel::PassThrough)]
        );
    }

    /// FIX-INTEG / FIX-SPLIT（Phase 3）：六个 M6 门控全 ON 时组合而无任何
    /// 菱形——主 pass 中的 panorama，透明尾部中的 OIT 对，
    /// 以及 HDR 区域中的 clipping→ibl→clouds→split 链，每个节点
    /// 恰好一个后继。
    #[test]
    fn all_six_gates_on_compose_without_diamonds() {
        let mut app = core3d_fixture(true);
        wire_m6_edges(app.main_mut(), true, true, true, true, true, true, true);

        use crate::effects::clipping_planes::CesiumClippingLabel;
        use crate::effects::clouds::CesiumCloudsLabel;
        use crate::effects::ibl::CesiumIblLabel;
        use crate::effects::oit::{CesiumOitCompositeLabel, CesiumOitLabel};
        use crate::effects::panorama::CesiumPanoramaLabel;
        use crate::effects::split::CesiumSplitLabel;
        // 主-pass panorama：单一路径。
        assert_eq!(out_labels(&app, CesiumPanoramaLabel).len(), 1);
        // 透明尾部：各恰好一个后继。
        assert_eq!(out_labels(&app, Node3d::MainTransparentPass), vec![name(CesiumOitLabel)]);
        assert_eq!(out_labels(&app, CesiumOitLabel).len(), 1);
        assert_eq!(out_labels(&app, CesiumOitCompositeLabel).len(), 1);
        // HDR 区域：EndMainPass → Clipping → Ibl → Clouds → Split → PassThrough。
        assert_eq!(out_labels(&app, Node3d::EndMainPass), vec![name(CesiumClippingLabel)]);
        assert_eq!(out_labels(&app, CesiumClippingLabel), vec![name(CesiumIblLabel)]);
        assert_eq!(out_labels(&app, CesiumIblLabel), vec![name(CesiumCloudsLabel)]);
        assert_eq!(out_labels(&app, CesiumCloudsLabel), vec![name(CesiumSplitLabel)]);
        assert_eq!(
            out_labels(&app, CesiumSplitLabel),
            vec![name(CesiumPostProcessLabel::PassThrough)]
        );
    }

    /// 接线是幂等的：调用两次不得添加第二条边也不得
    /// panic（plugin build 顺序无法从 `main.rs` 完全控制）。
    #[test]
    fn wiring_is_idempotent() {
        let mut app = core3d_fixture(true);
        wire_m6_edges(app.main_mut(), true, true, true, false, false, false, true);
        let first: Vec<Vec<String>> = [
            Node3d::MainOpaquePass,
            Node3d::EndMainPass,
            Node3d::Tonemapping,
        ]
        .map(|l| out_labels(&app, l))
        .to_vec();

        wire_m6_edges(app.main_mut(), true, true, true, false, false, false, true);
        let second: Vec<Vec<String>> = [
            Node3d::MainOpaquePass,
            Node3d::EndMainPass,
            Node3d::Tonemapping,
        ]
        .map(|l| out_labels(&app, l))
        .to_vec();

        assert_eq!(first, second, "re-wiring must be a no-op, not an edge duplicate");
    }

    /// `register_m6_render_graph` 在无头 `MinimalPlugins` app（无 `RenderApp`、
    /// 无 `RenderGraph`）上必须不 panic，无论环境门控如何。
    #[test]
    fn register_m6_render_graph_is_headless_safe() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        #[allow(deprecated)]
        register_m6_render_graph(&mut app);
        assert!(app.get_sub_app_mut(RenderApp).is_none());
    }
}
