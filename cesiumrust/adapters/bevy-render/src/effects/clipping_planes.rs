//! M6.2：ClippingPlanes `ViewNode` + uniform 注入基础设施。
//!
//! 将裁削平面（clipping planes）能力接入 M5-E 渲染图基础设施
//!（`graph.rs`）：以 Hessian 法式定义平面，逐片元据其正负半空间裁剪模型。
//! 对应
//! [`super::fxaa`] / [`super::ao`] 模式：本模块注册节点 /
//! 资源 / 系统，**但从不创建图边** ——
//! `graph.rs::register_render_graph` 中的单一线性 `Core3d` 链
//! 拥有这些边，所以不会形成菱形。将裁削节点接入该链是集成任务 #81。
//!
//! # 两条裁削路径（参见 `shaders/clipping.wgsl`）
//! 1. **正向 pass（忠实）** —— `apply_clipping_planes(world_pos)` 被
//!    `#import` 到 globe / tileset 片元 shader 中，使被裁削的片元在几何栅格化期间
//!    `discard`，恰好在上游
//!    `modelClippingPlanesStage(inout vec4 color)` 运行的位置。该接线触及
//!    材质 shader（超出 M6.2 文件范围），暂缓到 #81。
//! 2. **屏幕空间节点（本文件）** —— [`ClippingPlanesNode`] 从深度前置 pass
//!    重建世界坐标（上游 `czm_windowToEyeCoordinates(gl_FragCoord)` 的 WGSL 类比），
//!    并应用相同的判定，用背景色 + 边缘高亮覆盖被裁削的片元。
//!    DEVIATION：屏幕空间节点无法 `discard` 已栅格化的几何，
//!    且平面在 CPU 上预先变换，而非逐片元的
//!    `czm_transformPlane`——参见 `docs/deviations.md#dev-023`。
//!
//! # 门控（单一真相源）
//! 门控名由应用层注册表
//! `application/cesium-app/src/feature_flags.rs`（`ENV_ENABLE_CLIPPING` /
//! `clipping_enabled()`）拥有；下方的 [`ENV_ENABLE_CLIPPING`] 是一个字节一致的镜像，
//! 由 crate 依赖方向（`cesium-app` → `cesium-bevy-render`）强制。默认
//! OFF ⇒ `effects::graph::M6WaveARenderGraphPlugin`（task #81）从两半都提前
//! return ⇒ [`register_clipping_planes_node`] 从不被调用且没有 `Core3d`
//! 边 ⇒ 节点从不运行 ⇒ dynamic_globe v0 基线保持像素中性（PSNR = ∞）。
//!
//! # 已遵守的红线
//! - 领域保持度量 **f64**；度量→渲染单位的转换
//!   （`distance / METERS_PER_RENDER_UNIT`，`METERS_PER_RENDER_UNIT = 6378137`）
//!   仅在 [`ClippingPlanesUniform::from_domain`] GPU 边界发生。
//! - `clipping.wgsl` 将 `dot(n,p)` 和 `+ w` 保持为两次舍入（无 FMA 融合）。
//! - glam fast-math 全仓禁用（此处不依赖非 IEEE 浮点）。
//!
//! # 设计要点
//! - 每个平面用 Hessian 法式（单位法向 + 到原点距离）描述一个半空间。
//! - 集合可按并集/交集语义组合多个平面的裁剪结果。
//! - 平面数据在 GPU 边界打包为 uniform；逐片元着色器据符号丢弃被裁像素。
//! - 节点 / 资源 / 系统注册沿用 fxaa / ao 模式；深度 prepass 重建复用 Bevy PBR 做法。

use bevy::core_pipeline::{
    core_3d::graph::Core3d,
    fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prepass::{DepthPrepass, ViewPrepassTextures},
};
use bevy::ecs::query::QueryItem;
use bevy::prelude::*;
use bevy::render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::{
        NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
    },
    render_resource::{
        binding_types::{sampler, texture_2d, texture_depth_2d, uniform_buffer},
        BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, FilterMode, FragmentState,
        MultisampleState, Operations, PipelineCache, PrimitiveState, RenderPassColorAttachment,
        RenderPassDescriptor, RenderPipelineDescriptor, Sampler as GpuSampler, SamplerBindingType,
        SamplerDescriptor, ShaderStages, TextureFormat, TextureSampleType,
        UniformBuffer,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
    Render, RenderApp, RenderSet,
};
use cesium_effects::clipping::ClippingPlaneCollection;

use crate::resources::METERS_PER_RENDER_UNIT;
use super::graph::gate_from_env_value;

// ─── Shader handle ───────────────────────────────────────────────────────────

/// 内嵌 `clipping.wgsl` shader 的唯一 handle（与 pass-through / FXAA / AO
/// handle 区分——参见下方的碰撞测试）。
pub const CLIPPING_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0xCE51_C1C1_0006_0002);

/// 上传到 GPU uniform 的裁削平面最大数量。
///
/// 上游将任意数量打包进一个纹理；本移植上限为 8（一个裁削盒需要 6）。
/// 必须等于 `clipping.wgsl` 中的 `array<vec4<f32>, 8>` 大小
///（由 `wgsl_max_planes_matches_rust_const` 断言）。
pub const MAX_CLIPPING_PLANES: usize = 8;

// ─── 门控（应用层注册表的镜像——参见下方）

/// 门控 M6.2 裁削节点的环境变量。
///
/// **单一真相源（task #81）**：该名的拥有者是应用层
/// 注册表 `application/cesium-app/src/feature_flags.rs`（`ENV_ENABLE_CLIPPING`
/// 和 `clipping_enabled()` 访问器，列在 `RESERVED_FLAGS`，默认 OFF）。
/// 本 const 是一个*镜像*，仅因 `cesium-app` 依赖
/// `cesium-bevy-render`（绝不反向）而存在，所以本 crate 无法导入该
/// 注册表。它为 `pub`，以便
/// `feature_flags::adapter_gate_mirrors_are_byte_identical_to_the_registry` 能
/// 跨 crate 边界断言字节相等；注册本身由
/// `effects::graph::M6WaveARenderGraphPlugin` 驱动，它在每个插件阶段读取一次
/// [`clipping_gate_enabled`]。
pub const ENV_ENABLE_CLIPPING: &str = "CESIUM_ENABLE_CLIPPING";

/// 当裁削门控启用时返回 `true`。复用单一权威的
/// truthy 解析器（`gate_from_env_value`，全 crate 的 `{1, true, yes, on}` 集），
/// 所以它与其他所有 cesium 门控一致。
#[inline]
pub fn clipping_gate_enabled() -> bool {
    gate_from_env_value(std::env::var(ENV_ENABLE_CLIPPING).ok())
}

// ─── 渲染图 label（本地—#81 将其接入 Core3d 链）

/// cesium 裁削节点在 `Core3d` 中的节点 label。
///
/// 本地定义，以便本模块不必编辑 `graph.rs` 中共享的
/// `CesiumPostProcessLabel` 枚举。集成任务 #81 创建边：推荐位置是紧接
/// `Node3d::EndMainPass` 之后（该节点在几何之后、AO / tonemapping 之前
/// 直接作用于 HDR 场景）：
/// `EndMainPass → CesiumClippingLabel → PassThrough → AmbientOcclusion → …`。
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct CesiumClippingLabel;

// ─── 组件 ───────────────────────────────────────────────────────────────

/// 为一个视图携带活跃 [`ClippingPlaneCollection`] 的组件。
///
/// 放在相机上（像 [`super::fxaa::CesiumFxaa`]）以驱动屏幕空间节点；#81 也可
/// 将集合附加到 globe / tileset 实体上以实现忠实的正向注入路径。经
/// `ExtractComponentPlugin` 提取到 render world。当 `enabled == false` 或集合
/// 为空 / 禁用时节点提前 return（零 GPU 开销，像素中性）。
///
/// `Default` 为派生：`enabled = false`（保守，对应 `CesiumPassThrough`）
/// 与一个空的 `ClippingPlaneCollection`。
#[derive(Component, Clone, Debug, Default, ExtractComponent)]
pub struct CesiumClippingPlanes {
    /// 该视图上裁削节点的主开关。
    pub enabled: bool,
    /// 领域裁削平面集合（度量 f64）。
    pub collection: ClippingPlaneCollection,
}

impl CesiumClippingPlanes {
    /// 一个便捷构造函数，创建一个启用的视图裁削集合。
    pub fn new(collection: ClippingPlaneCollection) -> Self {
        Self {
            enabled: true,
            collection,
        }
    }

    /// 裁削是否应当真正运行（三个门控都一致）。
    #[inline]
    pub fn is_active(&self) -> bool {
        self.enabled && self.collection.enabled && !self.collection.is_empty()
    }
}

/// 裁削节点的逐视图缓存 pipeline ID。
#[derive(Component)]
pub struct CameraClippingPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

/// 持有打包后裁削平面的逐视图 GPU uniform 缓冲。
#[derive(Component)]
pub struct ViewClippingUniform {
    pub buffer: UniformBuffer<ClippingPlanesUniform>,
}

// ─── GPU uniform（f32 边界）

/// GPU 面向的裁削 uniform。**仅 f32** —— 领域集合保持度量
/// f64；[`ClippingPlanesUniform::from_domain`] 在此边界执行唯一的度量→
/// 渲染单位转换（红线）。
///
/// 布局必须与 `shaders/clipping.wgsl` 中的 `struct ClippingPlanes` 匹配。
///
/// 该 struct 住在带 `#![allow(dead_code)]` 的私有 `clipping_uniform`
/// 模块中（`sky_dome.rs` 约定）：encase 的 `ShaderType` derive 会生成一个
/// 模块级 helper，死代码分析会标记它，尽管每个字段都通过 `write_buffer`
/// 上传。字段值在 `from_domain_*` 单元测试中被断言。
pub use clipping_uniform::ClippingPlanesUniform;

mod clipping_uniform {
    #![allow(dead_code)]
    use super::MAX_CLIPPING_PLANES;
    use bevy::prelude::Vec4;
    use bevy::render::render_resource::ShaderType;

    /// GPU 裁削 uniform；布局匹配 `shaders/clipping.wgsl` 中的
    /// `struct ClippingPlanes`（encase std140）。
    #[derive(ShaderType, Clone, Debug)]
    pub struct ClippingPlanesUniform {
        /// `xyz` = 单位法线（世界空间），`w` = 以 RENDER UNITS 计的有符号距离。
        pub planes: [Vec4; MAX_CLIPPING_PLANES],
        /// `rgb` = 边缘高亮颜色，`a` = 以 PIXELS 计的边缘宽度。
        pub edge_color: Vec4,
        /// 在节点路径上为被裁削的片元写入。
        pub background_color: Vec4,
        /// `x` = 平面数，`y` = union 标志，`z` = 启用，`w` = 填充。
        pub params: Vec4,
    }
}

impl Default for ClippingPlanesUniform {
    /// 默认均匀体：所有平面置零、边缘颜色全透，等效于不裁剪地透传源颜色。
    fn default() -> Self {
        Self {
            planes: [Vec4::ZERO; MAX_CLIPPING_PLANES],
            edge_color: Vec4::new(1.0, 1.0, 1.0, 0.0),
            // params.z = 0 ⇒ shader 原样透传源颜色
            //（像素中性）。FIX-CLIP-BGCOLOR：当节点确实激活时，它用
            // `blend: None` 将 background_color 覆在被裁削的像素上，而 FXAA
            // 将 alpha 直接透传到呈现，所以默认 alpha 为 0
            // 会在下游读成一个完全透明的空洞。默认改为
            // 不透明的黑色“空虚”揭露；用户通过集合覆盖。
            background_color: Vec4::new(0.0, 0.0, 0.0, 1.0),
            params: Vec4::ZERO,
        }
    }
}

impl ClippingPlanesUniform {
    /// 将一个领域 [`ClippingPlaneCollection`] 打包进 GPU uniform。
    ///
    /// - 平面在 CPU 上经 [`ClippingPlaneCollection::world_planes`] 烘焙进世界
    ///   空间（上游逐片元 `czm_transformPlane` 的 CPU 等价物），然后每个平面的
    ///   **度量距离除以 [`METERS_PER_RENDER_UNIT`]** 以进入渲染单位世界空间——
    ///   即从深度重建的 `world_pos` 所住的空间。法线是
    ///   单位方向（无尺度），所以直接转为 f32。
    /// - 数量被限幅到 [`MAX_CLIPPING_PLANES`]。
    /// - 当集合禁用或为空时 `params.z`（enabled）为 `0.0`，
    ///   这使得节点为纯透传（像素中性）。
    pub fn from_domain(collection: &ClippingPlaneCollection) -> Self {
        let mut planes = [Vec4::ZERO; MAX_CLIPPING_PLANES];

        let world = collection.world_planes();
        let count = world.len().min(MAX_CLIPPING_PLANES);
        for (slot, plane) in planes.iter_mut().zip(world.iter().take(count)) {
            *slot = Vec4::new(
                plane.normal.x as f32,
                plane.normal.y as f32,
                plane.normal.z as f32,
                // 度量→渲染单位（红线：除以 6378137）。
                (plane.distance / METERS_PER_RENDER_UNIT) as f32,
            );
        }

        let edge_color = Vec4::new(
            collection.edge_color[0] as f32,
            collection.edge_color[1] as f32,
            collection.edge_color[2] as f32,
            collection.edge_width as f32, // 保持在 PIXELS（shader 用 fwidth）
        );

        let active = collection.enabled && !collection.is_empty();
        let params = Vec4::new(
            count as f32,
            if collection.union_clipping_regions { 1.0 } else { 0.0 },
            if active { 1.0 } else { 0.0 },
            0.0,
        );

        Self {
            planes,
            edge_color,
            // FIX-CLIP-BGCOLOR：不透明 alpha，使节点路径背景不会在 FXAA
            // 将 alpha 透传到呈现后变成一个透明空洞。领域集合尚未
            // 对外暴露背景色；当它暴露时，将其贯穿到此（deferred #51 跟踪
            // 忠实的逐几何丢弃）。
            background_color: Vec4::new(0.0, 0.0, 0.0, 1.0),
            params,
        }
    }
}

// ─── Pipeline ────────────────────────────────────────────────────────────────

/// Render-world 资源：裁削节点的 bind group 布局 + 采样器。
///
/// Group 0 bindings（必须匹配 `clipping.wgsl` + binding 覆盖测试）：
/// - 0：深度前置 pass（`texture_depth_2d`）— 世界坐标重建
/// - 1：颜色源（`texture_2d<f32>`）— 后处理输入
/// - 2：point 采样器（NonFiltering，深度）
/// - 3：linear 采样器（Filtering，颜色）
/// - 4：`ViewUniform`（动态偏移）— `view_from_clip` / `world_from_view`
/// - 5：`ClippingPlanesUniform` — 打包后的平面
#[derive(Resource)]
pub struct ClippingPlanesPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub point_sampler: GpuSampler,
    pub linear_sampler: GpuSampler,
}

impl FromWorld for ClippingPlanesPipeline {
    /// 从 render-world 构建裁削 pass 的设备资源：为深度纹理、裁削平面
    /// 均匀体、view uniform 与采样器创建 bind group 布局。
    ///
    /// # 参数
    /// - `render_world`：提供 `RenderDevice` 的渲染世界。
    ///
    /// # 返回
    /// 装配好的 [`ClippingPlanesPipeline`] 资源。
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();

        let bind_group_layout = render_device.create_bind_group_layout(
            "cesium_clipping_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_depth_2d(),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::NonFiltering),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<ClippingPlanesUniform>(false),
                ),
            ),
        );

        let point_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_clipping_point_sampler"),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_clipping_linear_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            bind_group_layout,
            point_sampler,
            linear_sampler,
        }
    }
}

// ─── ViewNode ────────────────────────────────────────────────────────────────

/// 屏幕空间裁削 `ViewNode`。
///
/// 从深度前置 pass 重建世界坐标，并应用集合的 union / intersection
/// 判定，用配置的背景色覆盖被裁削的片元并对边缘带做着色。当组件
/// 非活跃或深度前置 pass / pipeline 不可用时提前 return（像素中性）。
#[derive(Default)]
pub struct ClippingPlanesNode;

impl ViewNode for ClippingPlanesNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static CameraClippingPipeline,
        &'static CesiumClippingPlanes,
        &'static ViewClippingUniform,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
    );

    /// 运行裁削平面 pass：若未激活则直接返回；否则从深度 prepass 重建
    /// 世界坐标，据各裁削平面的半空间判定丢弃被裁像素并可选绘制边缘。
    ///
    /// # 参数
    /// - `_graph`：渲染图上下文（本节点无子 pass）。
    /// - `render_context`：当前 pass 的 GPU 命令记录器。
    /// - `target`：视图的渲染目标（后处理读写）。
    /// - `pipeline_handle`：该视图缓存的裁削 pipeline ID。
    /// - `clipping`：相机级裁削开关；`clipping_uniform`：本帧平面参数。
    /// - `prepass`：深度 prepass 纹理；`view_uniform_offset`：本视图偏移。
    /// - `world`：提供 pipeline/texture 资源的 render-world。
    ///
    /// # 返回
    /// 成功提交命令则为 `Ok(())`；前置资源未就绪时返回错误。
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (target, pipeline_handle, clipping, clipping_uniform, prepass, view_uniform_offset): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if !clipping.is_active() {
            return Ok(());
        }

        // 裁削需要深度前置 pass 来重建世界坐标。
        let Some(depth_view) = prepass.depth_view() else {
            return Ok(());
        };

        let pipeline_cache = world.resource::<PipelineCache>();
        let clipping_pipeline = world.resource::<ClippingPlanesPipeline>();
        let view_uniforms = world.resource::<ViewUniforms>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_handle.pipeline_id) else {
            return Ok(());
        };
        let Some(view_uniform_binding) = view_uniforms.uniforms.binding() else {
            return Ok(());
        };
        let Some(clipping_binding) = clipping_uniform.buffer.binding() else {
            return Ok(());
        };

        let render_device = render_context.render_device().clone();

        let post_process = target.post_process_write();
        let source = post_process.source;
        let destination = post_process.destination;

        let bind_group = render_device.create_bind_group(
            Some("cesium_clipping_bind_group"),
            &clipping_pipeline.bind_group_layout,
            &BindGroupEntries::sequential((
                depth_view,
                source,
                &clipping_pipeline.point_sampler,
                &clipping_pipeline.linear_sampler,
                view_uniform_binding.clone(),
                clipping_binding,
            )),
        );

        let pass_descriptor = RenderPassDescriptor {
            label: Some("cesium_clipping_pass"),
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
        // Binding 4（ViewUniform）为动态偏移；提供它的偏移（Ryan C2
        // 教训：一个 `&[]` 偏移列表会使 dynamic-buffer 校验失败）。
        render_pass.set_bind_group(0, &bind_group, &[view_uniform_offset.offset]);
        render_pass.draw(0..3, 0..1); // 全屏三角形

        Ok(())
    }
}

// ─── 渲染系统 ──────────────────────────────────────────────────────────

/// 为每个活跃视图准备裁削 pipeline + 逐视图 uniform 缓冲。
/// 在 `Render`、`RenderSet::Prepare` 中运行（渲染图执行之前）。
pub fn prepare_clipping(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    clipping_pipeline: Res<ClippingPlanesPipeline>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    views: Query<(Entity, &ExtractedView, &CesiumClippingPlanes)>,
) {
    for (entity, view, clipping) in &views {
        if !clipping.is_active() {
            continue;
        }

        let destination_format = if view.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::Rgba8UnormSrgb
        };

        let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("cesium_clipping_pipeline".into()),
            layout: vec![clipping_pipeline.bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: CLIPPING_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: destination_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: Vec::new(),
            zero_initialize_workgroup_memory: false,
        });

        // 将领域集合（度量 f64）打包进 GPU uniform（f32，渲染单位距离）
        // 并上传。
        let mut buffer = UniformBuffer::from(ClippingPlanesUniform::from_domain(
            &clipping.collection,
        ));
        buffer.write_buffer(&render_device, &render_queue);

        commands.entity(entity).insert((
            CameraClippingPipeline { pipeline_id },
            ViewClippingUniform { buffer },
        ));
    }
}

// ─── 主 world 系统 ──────────────────────────────────────────────────────

/// 确保驱动一个活跃裁削集合的相机携带 [`DepthPrepass`]（该节点的世界坐标输入）。
/// 适配层启用，使应用层相机 bundle 保持不变（与 `setup_ao_prepass` 同纪律）。
///
/// 仅 ADDS `DepthPrepass`（从不移除它——AO 可能也需要它）。仅当裁削门控 ON 时
/// 才注册，所以门控 OFF ⇒ 无额外几何 pass ⇒ v0 基线像素中性。
pub fn setup_clipping_prepass(
    mut commands: Commands,
    cameras: Query<(Entity, &CesiumClippingPlanes)>,
) {
    for (entity, clipping) in &cameras {
        if clipping.is_active() {
            commands.entity(entity).insert(DepthPrepass);
        }
    }
}

// ─── 注册 ────────────────────────────────────────────────────────────

/// 将裁削节点注册进 `RenderApp`（shader + extract + 节点 + 系统）。
///
/// 当 [`clipping_gate_enabled()`] 为 true 时由 `effects::graph::M6WaveARenderGraphPlugin`
///（task #81）调用；`Core3d` 边由 `effects::graph::wire_m6_edges` 创建
///（本函数注册节点但从不接线，所以 `graph.rs` 中的共享线性链仍是唯一拥有者，
/// 不会形成菱形）。
///
/// 无头安好：没有 `RenderApp`（MinimalPlugins）时降级为 no-op。
#[deprecated = "DEV-029 / FIX-REG-FACADE: call `register_clipping_planes_node_main_world` from `Plugin::build` and `register_clipping_planes_node_render_world` from `Plugin::finish`; this facade runs the finish half against a possibly device-less render world."]
pub fn register_clipping_planes_node(app: &mut App) {
    register_clipping_planes_node_main_world(app);
    // 无头的 `MinimalPlugins` 没有 `RenderApp` —— 优雅降级。
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        register_clipping_planes_node_render_world(render_app);
    }
}

/// [`register_clipping_planes_node`] 的 `Plugin::build` 时半边：所有住在**主** world
/// 的东西（WGSL shader 资源 + `ExtractComponentPlugin` + `setup_clipping_prepass` 系统）。
///
/// 由 task #81 拆分——参见 `docs/deviations.md#dev-029`。pipeline 的
/// `FromWorld` 读取 `RenderDevice`，而 Bevy 仅在 `RenderPlugin::finish` 中把它插入
/// render world，所以下方的 render-world 半边必须从插件的 `finish` 运行，
/// 绝不从其 `build` 运行。
pub fn register_clipping_planes_node_main_world(app: &mut App) {
    // 注册裁削 WGSL shader（经 shader_registry 无头安好）。
    crate::shader_registry::try_load_internal_shader(
        app,
        CLIPPING_SHADER_HANDLE,
        include_str!("../../shaders/clipping.wgsl"),
        "shaders/clipping.wgsl",
    );

    // ExtractComponentPlugin：每帧 主 → render world（ExtractSchedule）。
    app.add_plugins(ExtractComponentPlugin::<CesiumClippingPlanes>::default());

    // 主 world：为驱动活跃集合的相机附加 DepthPrepass。
    app.add_systems(Update, setup_clipping_prepass);
}

/// [`register_clipping_planes_node`] 的 `Plugin::finish` 时半边：render-world pipeline
/// 资源 + `Core3d` 节点。
pub fn register_clipping_planes_node_render_world(render_app: &mut bevy::app::SubApp) {
    // FIX-REG-FACADE (DEV-029)：当 `RenderDevice` 缺失时降级为 no-op
    //（finish 半边从 `build` 到达，或一个裸 render world）。参见
    // `crate::effects::render_world_missing_device`。
    if crate::effects::render_world_missing_device(render_app) {
        return;
    }

    render_app
        .init_resource::<ClippingPlanesPipeline>()
        .add_systems(Render, prepare_clipping.in_set(RenderSet::Prepare))
        .add_render_graph_node::<ViewNodeRunner<ClippingPlanesNode>>(
            Core3d,
            CesiumClippingLabel,
        );
    // 注意：边由 `effects::graph::wire_m6_edges`（task #81）创建，它是共享
    // `Core3d` 链的唯一拥有者。
}

// ─── 测试 ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_effects::clipping::ClippingPlane;
    use glam::DVec3;

    #[test]
    fn clipping_component_default_disabled() {
        let c = CesiumClippingPlanes::default();
        assert!(!c.enabled);
        assert!(!c.is_active());
    }

    #[test]
    fn clipping_gate_const_is_local_and_stable() {
        // 门控 env 变量定义在本地（隔离纪律），且绝不可与后处理门控
        // 碰撞。纯断言（无 env 变更）。
        assert_eq!(ENV_ENABLE_CLIPPING, "CESIUM_ENABLE_CLIPPING");
        assert_ne!(ENV_ENABLE_CLIPPING, "CESIUM_ENABLE_POSTPROCESS");
        // 复用权威的 truthy 解析器。
        assert!(!gate_from_env_value(None));
        assert!(gate_from_env_value(Some("1".into())));
    }

    #[test]
    fn clipping_shader_handle_unique() {
        assert_ne!(
            CLIPPING_SHADER_HANDLE,
            super::super::graph::PASS_THROUGH_SHADER_HANDLE
        );
        assert_ne!(CLIPPING_SHADER_HANDLE, super::super::fxaa::FXAA_SHADER_HANDLE);
        assert_ne!(CLIPPING_SHADER_HANDLE, super::super::ao::AO_SHADER_HANDLE);
    }

    #[test]
    fn clipping_headless_graceful() {
        // 无 RenderApp（无头）→ register 绝不可 panic。
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        #[allow(deprecated)]
        register_clipping_planes_node(&mut app);
    }

    // ─── Uniform 打包：度量→渲染单位边界（红线）────────

    #[test]
    fn from_domain_divides_distance_by_meters_per_render_unit() {
        // 一个沿 +Y 的平面，距 6_378_137 m（== 1 渲染单位）。它的法线是单位
        //（不变）；它的度量距离到达时必须除以 6378137。
        let collection = ClippingPlaneCollection::with_planes(vec![ClippingPlane::new(
            DVec3::Y,
            METERS_PER_RENDER_UNIT,
        )]);
        let u = ClippingPlanesUniform::from_domain(&collection);

        // 单位矩阵 model_matrix ⇒ 世界平面 == 局部平面。
        let plane = u.planes[0];
        assert!((plane.x).abs() < 1e-6, "normal.x");
        assert!((plane.y - 1.0).abs() < 1e-6, "normal.y preserved");
        assert!((plane.z).abs() < 1e-6, "normal.z");
        // 6378137 m / 6378137 = 1.0 render unit.
        assert!(
            (plane.w - 1.0).abs() < 1e-5,
            "distance must be metric / METERS_PER_RENDER_UNIT, got {}",
            plane.w
        );

        // params: count = 1, union = 0, enabled = 1.
        assert!((u.params.x - 1.0).abs() < 1e-6);
        assert!((u.params.y).abs() < 1e-6);
        assert!((u.params.z - 1.0).abs() < 1e-6);
    }

    #[test]
    fn from_domain_union_flag_and_edge_width() {
        let mut collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
            ClippingPlane::new(DVec3::X, 0.0),
        ]);
        collection.union_clipping_regions = true;
        collection.edge_width = 3.0;
        collection.edge_color = [1.0, 0.0, 0.0, 1.0];

        let u = ClippingPlanesUniform::from_domain(&collection);
        assert!((u.params.x - 2.0).abs() < 1e-6, "count");
        assert!((u.params.y - 1.0).abs() < 1e-6, "union flag set");
        assert!((u.params.z - 1.0).abs() < 1e-6, "enabled");
        assert!((u.edge_color.w - 3.0).abs() < 1e-6, "edge width in pixels");
        assert!((u.edge_color.x - 1.0).abs() < 1e-6, "edge colour r");
    }

    #[test]
    fn from_domain_disabled_or_empty_is_pixel_neutral() {
        // Empty collection ⇒ params.z = 0 ⇒ shader passes source through (neutral).
        let empty = ClippingPlaneCollection::default();
        assert!((ClippingPlanesUniform::from_domain(&empty).params.z).abs() < 1e-6);

        // Non-empty but disabled ⇒ also params.z = 0.
        let mut disabled = ClippingPlaneCollection::with_planes(vec![ClippingPlane::new(
            DVec3::Y,
            0.0,
        )]);
        disabled.enabled = false;
        assert!((ClippingPlanesUniform::from_domain(&disabled).params.z).abs() < 1e-6);
    }

    #[test]
    fn from_domain_caps_at_max_planes() {
        let planes: Vec<ClippingPlane> = (0..(MAX_CLIPPING_PLANES + 4))
            .map(|i| ClippingPlane::new(DVec3::Y, i as f64))
            .collect();
        let collection = ClippingPlaneCollection::with_planes(planes);
        let u = ClippingPlanesUniform::from_domain(&collection);
        assert!((u.params.x - MAX_CLIPPING_PLANES as f32).abs() < 1e-6);
    }

    #[test]
    fn setup_clipping_prepass_adds_depth_prepass_when_active() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, setup_clipping_prepass);

        let collection =
            ClippingPlaneCollection::with_planes(vec![ClippingPlane::new(DVec3::Y, 0.0)]);
        let cam = app
            .world_mut()
            .spawn((Camera3d::default(), CesiumClippingPlanes::new(collection)))
            .id();

        app.update();
        assert!(
            app.world().get::<DepthPrepass>(cam).is_some(),
            "active clipping ⇒ DepthPrepass attached"
        );

        // Disable ⇒ no new prepass forced on a fresh camera.
        let cam2 = app
            .world_mut()
            .spawn((Camera3d::default(), CesiumClippingPlanes::default()))
            .id();
        app.update();
        assert!(app.world().get::<DepthPrepass>(cam2).is_none());
    }

    // ─── Ryan C1/C2 防线：无头 naga 解析 + 校验 + 布局 ──────

    /// naga 没有预处理器，所以 `clipping.wgsl` 中的两个 `#import` 被替换为
    /// 精确声明 shader 所读取字段的 struct stub：
    /// `FullscreenVertexOutput.{position, uv}` 和 `View.{view_from_clip,
    /// world_from_view}`。`view` **binding** 本身由真实 shader 文本声明
    ///（group 0, binding 4），所以此处刻意不为其提供 stub。
    const CLIPPING_WGSL_IMPORT_STUBS: &str = "\
struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct View {
    view_from_clip: mat4x4<f32>,
    world_from_view: mat4x4<f32>,
}
";

    fn clipping_stubbed_wgsl() -> String {
        let mut source = String::from(CLIPPING_WGSL_IMPORT_STUBS);
        for line in include_str!("../../shaders/clipping.wgsl").lines() {
            if line.starts_with("#import") {
                continue;
            }
            source.push_str(line);
            source.push('\n');
        }
        source
    }

    /// 裁削 shader 为真的无头证明：由 **naga** 解析 + 做类型检查
    ///（`bevy_render` 在 GPU 路径上编译它所用的同一个 WGSL 前端）。
    /// 防住 M5 C1（保留字）类的静默失败 bug。
    #[test]
    fn clipping_wgsl_parses_and_type_checks_under_naga() {
        let source = clipping_stubbed_wgsl();
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("clipping.wgsl does not parse:\n{}", error.emit_to_string(&source))
        });
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("clipping.wgsl does not validate");

        let entry_points = module
            .entry_points
            .iter()
            .map(|entry| (entry.name.as_str(), entry.stage))
            .collect::<Vec<_>>();
        assert_eq!(
            entry_points,
            vec![("fragment", naga::ShaderStage::Fragment)],
            "clipping.wgsl must expose exactly one fragment entry point"
        );
    }

    /// Ryan C1（捕获 **C2** 类 bug）：`fragment` 入口静态使用的每个 binding
    /// 都必须出现在 Rust `ClippingPlanesPipeline` 布局中（group 0: bindings 0..=5）。
    /// 防住一种静默的 pipeline-build 失败——它会让裁削 no-op，而
    /// `pixel_diff` 却报出假绿。
    #[test]
    fn clipping_wgsl_entry_bindings_are_covered_by_the_rust_layout() {
        let source = clipping_stubbed_wgsl();
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("clipping.wgsl does not parse:\n{}", error.emit_to_string(&source))
        });

        // `ClippingPlanesPipeline::from_world` 的镜像（group 0 binding 索引）。
        let layout: std::collections::BTreeSet<(u32, u32)> =
            [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5)].into_iter().collect();

        let entry = module
            .entry_points
            .iter()
            .find(|e| e.name == "fragment")
            .expect("clipping.wgsl must have a `fragment` entry point");

        let mut used = std::collections::BTreeSet::new();
        let mut visited = std::collections::HashSet::new();
        let mut stack: Vec<&naga::Function> = vec![&entry.function];
        while let Some(func) = stack.pop() {
            for (_, expr) in func.expressions.iter() {
                match *expr {
                    naga::Expression::GlobalVariable(handle) => {
                        if let Some(binding) = &module.global_variables[handle].binding {
                            used.insert((binding.group, binding.binding));
                        }
                    }
                    naga::Expression::CallResult(func_handle)
                        if visited.insert(func_handle) =>
                    {
                        stack.push(&module.functions[func_handle]);
                    }
                    _ => {}
                }
            }
        }
        let missing: Vec<(u32, u32)> = used.difference(&layout).copied().collect();
        assert!(
            missing.is_empty(),
            "clipping.wgsl `fragment` statically uses bindings {missing:?} absent from \
             ClippingPlanesPipeline's layout (C2-class regression: the pipeline would fail \
             to build and clipping would silently no-op)"
        );
    }

    /// 交叉校验：WGSL uniform 数组大小必须等于 Rust 的 [`MAX_CLIPPING_PLANES`]
    /// const，所以 `from_domain` 的限幅与 shader 一致。
    #[test]
    fn wgsl_max_planes_matches_rust_const() {
        let wgsl = include_str!("../../shaders/clipping.wgsl");
        assert!(
            wgsl.contains(&format!("array<vec4<f32>, {MAX_CLIPPING_PLANES}>")),
            "clipping.wgsl plane array size must match MAX_CLIPPING_PLANES = {MAX_CLIPPING_PLANES}"
        );
        assert!(
            wgsl.contains(&format!("const MAX_CLIPPING_PLANES: i32 = {MAX_CLIPPING_PLANES};")),
            "clipping.wgsl MAX_CLIPPING_PLANES const must match Rust"
        );
    }

    /// 红线守卫：WGSL 必须在源码中保留度量→渲染单位的转换说明
    /// 与 NO-FMA-CONTRACTION 不变量，使其可见（对应 sky_atmosphere.wgsl
    /// 的头部断言）。
    #[test]
    fn wgsl_declares_red_lines() {
        let wgsl = include_str!("../../shaders/clipping.wgsl");
        assert!(wgsl.contains("METERS_PER_RENDER_UNIT = 6378137"));
        assert!(wgsl.contains("NO FMA CONTRACTION"));
    }

    /// GPU `apply_clipping_planes` 累加的 f32 镜像（`clipping.wgsl`），
    /// 留在此处以便 shader 的有符号 min/max + `<= 0.0` 语义能与 f64 领域
    /// 参考 [`ClippingPlaneCollection::clip_signed`] 交叉校验。FIX-CLIP-CPUREF。
    fn gpu_clip_mirror_f32(planes: &[(glam::Vec3, f32)], union: bool, world_pos: glam::Vec3) -> (bool, f32) {
        let mut any_outside = false;
        let mut all_outside = true;
        let mut clip_amount = 0f32;
        for (i, (n, d0)) in planes.iter().enumerate() {
            let d = n.dot(world_pos) + *d0;
            if d <= 0.0 {
                any_outside = true;
            } else {
                all_outside = false;
            }
            clip_amount = if union {
                if i == 0 { d } else { d.min(clip_amount) }
            } else {
                d.max(clip_amount)
            };
        }
        (if union { any_outside } else { all_outside }, clip_amount)
    }

    #[test]
    fn clipping_gpu_signed_accumulation_matches_the_cpu_reference() {
        // 确定性 LCG（无 rand 依赖）。值在 [0, 1)。
        struct Lcg(u64);
        impl Lcg {
            fn next01(&mut self) -> f64 {
                self.0 = self
                    .0
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((self.0 >> 40) as f64) / ((1u64 << 24) as f64)
            }
            fn signed(&mut self, scale: f64) -> f64 {
                (self.next01() * 2.0 - 1.0) * scale
            }
        }
        let mut rng = Lcg(0x9E37_79B9_7F4A_7C15);

        for union in [true, false] {
            for trial in 0..200u32 {
                let plane_count = 1 + (rng.next01() * 4.0).min(4.0) as usize;
                let mut planes = Vec::new();
                let mut domain = Vec::new();
                for _ in 0..plane_count {
                    let n = DVec3::new(rng.signed(1.0), rng.signed(1.0), rng.signed(1.0));
                    let n = if n.length_squared() < 1e-12 { DVec3::Y } else { n.normalize() };
                    let dist = rng.signed(6.0);
                    let p = ClippingPlane::new(n, dist);
                    // ClippingPlane::new 会重新归一化；读回权威的
                    // normal/distance，使 f32 镜像看到*同一个*平面。
                    domain.push((glam::Vec3::new(p.normal.x as f32, p.normal.y as f32, p.normal.z as f32), p.distance as f32));
                    planes.push(p);
                }
                let point = DVec3::new(rng.signed(8.0), rng.signed(8.0), rng.signed(8.0));

                let mut collection = ClippingPlaneCollection::with_planes(planes);
                collection.union_clipping_regions = union;
                let (cpu_clipped, cpu_amount) = collection.clip_signed(point);

                let gpu_point = glam::Vec3::new(point.x as f32, point.y as f32, point.z as f32);
                let (gpu_clipped, gpu_amount) = gpu_clip_mirror_f32(&domain, union, gpu_point);
                let gpu_amount: f64 = gpu_amount as f64;

                assert_eq!(
                    cpu_clipped, gpu_clipped,
                    "trial {trial} union={union}: clipped mismatch"
                );
                assert!(
                    (cpu_amount - gpu_amount).abs() < 1e-3,
                    "trial {trial} union={union}: clip_amount {cpu_amount} vs {gpu_amount}"
                );
            }
        }
    }
}
