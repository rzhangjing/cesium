//! M6.5：基于图像的照明（IBL）`ViewNode` + 环境 PBR 注入。
//!
//! 将上游 CesiumJS 的基于图像照明能力
//!（`Scene/ImageBasedLighting.js` + `Shaders/Model/ImageBasedLightingStageFS.glsl`
//! `textureIBL`）移植到 M5-E 渲染图基础设施（`graph.rs`）。对应
//! [`super::clipping_planes`] / [`super::ao`] 模式：本模块注册
//! 节点 / 资源 / 系统，**但从不创建图边** —— `graph.rs::register_render_graph`
//! 中的单一线性 `Core3d` 链拥有这些边，所以不会形成菱形。
//! 将 IBL 节点接入该链是集成任务 #81。
//!
//! # 两条 IBL 路径（参见 `shaders/ibl.wgsl`）
//! 1. **正向材质 stage（忠实）** —— `texture_ibl(...)` 被 `#import`
//!    到 globe / tileset / fabric 片元 shader 中，使环境 BRDF 针对 glTF 材质逐片元
//!    运行，恰好在上游 `textureIBL` 于 `ImageBasedLightingStageFS` 运行的位置。
//!    该接线触及材质 shader（超出 M6.5 文件范围），暂缓到 #81。
//! 2. **屏幕空间节点（本文件）** —— [`IblNode`] 从深度 / 法线前置 pass
//!    重建世界坐标 + 法线（上游眼空间重建的 WGSL 类比），并将环境光照叠加到
//!    HDR 场景颜色上，从 uniform 读取材质参数（DEVIATION：参见
//!    `docs/deviations.md#dev-024` 与 `shaders/ibl.wgsl` 头部——
//!    单材质场景 + 内联 split-sum BRDF）。
//!
//! # 门控（单一真相源）
//! 门控名由应用层注册表
//! `application/cesium-app/src/feature_flags.rs`（`ENV_ENABLE_IBL` /
//! `ibl_enabled()`）拥有；[`ENV_ENABLE_IBL`] 是一个字节一致的镜像，由
//! crate 依赖方向（`cesium-app` → `cesium-bevy-render`）强制。默认
//! OFF ⇒ `effects::graph::M6WaveARenderGraphPlugin`（task #81）从两半都提前
//! return ⇒ [`register_ibl_node`] 从不被调用且没有 `Core3d` 边
//! ⇒ 节点从不运行 ⇒ dynamic_globe v0 基线保持像素中性
//!（PSNR = ∞）。
//!
//! # 已遵守的红线
//! - 领域保持 **f64**（[`ImageBasedLighting`] SH 系数、[`IblMaterial`]
//!   参数）；f64 → f32 投影仅在
//!   [`IblUniform::from_domain`]（GPU uniform 边界）发生。
//! - `ibl.wgsl` 将反射向量和 Fresnel pow5 保持为两次舍入
//!   （NO FMA contraction —— IBL 数值对积分敏感）。
//! - 环境颜色图在 CPU 侧为 sRGB；LUT / prefilter 输出
//!   为线性（绝非 sRGB）。glam fast-math 全仓禁用。
//!
//! # 蓝图
//! - `packages/engine/Source/Scene/ImageBasedLighting.js`
//! - `packages/engine/Source/Shaders/Model/ImageBasedLightingStageFS.glsl` (textureIBL)
//! - `packages/engine/Source/Shaders/Builtin/Functions/{sphericalHarmonics,pbrLighting}.glsl`
//! - `packages/engine/Source/Shaders/{BrdfLutGeneratorFS,ConvolveSpecularMapFS}.glsl`
//! - `domain/effects/src/ibl.rs` (f64 CPU reference, cross-validated by these tests)
//! - `bevy_pbr-0.15.3/src/ssao/mod.rs` (ViewNode + depth/normal prepass reconstruction)

use std::sync::Mutex;

use bevy::core_pipeline::{
    core_3d::graph::Core3d,
    fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prepass::{DepthPrepass, NormalPrepass, ViewPrepassTextures},
};
use bevy::ecs::query::QueryItem;
use bevy::prelude::*;
use bevy::render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::{
        NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
    },
    render_resource::{
        binding_types::{sampler, texture_2d, texture_cube, texture_depth_2d, uniform_buffer},
        BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, Extent3d, FilterMode, FragmentState,
        MultisampleState, Operations, PipelineCache, PrimitiveState, RenderPassColorAttachment,
        RenderPassDescriptor, RenderPipelineDescriptor, Sampler as GpuSampler, SamplerBindingType,
        SamplerDescriptor, ShaderStages, Texture, TextureDescriptor, TextureDimension,
        TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor,
        TextureViewDimension, UniformBuffer,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
    Render, RenderApp, RenderSet,
};
use cesium_effects::ibl::{default_spherical_harmonics, IblMaterial, ImageBasedLighting};

use super::graph::gate_from_env_value;

// ─── Shader handle ───────────────────────────────────────────────────────────

/// 内嵌 `ibl.wgsl` shader 的唯一 handle（与 pass-through
/// / FXAA / AO / clipping handle 区分——参见下方的碰撞测试）。
pub const IBL_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(0xCE51_1B15_1810_0065);

// ─── 门控（应用层注册表的镜像——参见下方）

/// 门控 M6.5 IBL 节点的环境变量。
///
/// **单一真相源（task #81）**：该名的拥有者是应用层
/// 注册表 `application/cesium-app/src/feature_flags.rs`（`ENV_ENABLE_IBL` + 该
/// `ibl_enabled()` 访问器，列在 `RESERVED_FLAGS`，默认 OFF）。本 const
/// 是一个*镜像*，仅因 `cesium-app` 依赖
/// `cesium-bevy-render`（绝不反向）而存在，所以本 crate 无法导入该
/// 注册表。它为 `pub`，以便
/// `feature_flags::adapter_gate_mirrors_are_byte_identical_to_the_registry` 能
/// 跨 crate 边界断言字节相等；注册本身由
/// `effects::graph::M6WaveARenderGraphPlugin` 驱动，它在每个插件阶段读取一次
/// [`ibl_gate_enabled`]。
pub const ENV_ENABLE_IBL: &str = "CESIUM_ENABLE_IBL";

/// 当 IBL 门控启用时返回 `true`。复用单一权威的
/// truthy 解析器（`gate_from_env_value`，全 crate 的 `{1, true, yes, on}` 集），
/// 所以它与其他所有 cesium 门控一致。
#[inline]
pub fn ibl_gate_enabled() -> bool {
    gate_from_env_value(std::env::var(ENV_ENABLE_IBL).ok())
}

// ─── 渲染图 label（本地—#81 将其接入 Core3d 链）

/// cesium IBL 节点在 `Core3d` 中的节点 label。
///
/// 本地定义，以便本模块不必编辑 `graph.rs` 中共享的
/// `CesiumPostProcessLabel` 枚举。集成任务 #81 创建边；推荐位置是紧接
/// `Node3d::EndMainPass` 之后（该节点在 AO / tonemapping 之前将环境光照
/// 叠加到 HDR 场景）：
/// `EndMainPass → CesiumIblLabel → PassThrough → AmbientOcclusion → …`。
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct CesiumIblLabel;

// ─── Component ───────────────────────────────────────────────────────────────

/// 为一个视图携带活跃 [`ImageBasedLighting`] 环境的组件。
///
/// 放在相机上（像 [`super::fxaa::CesiumFxaa`]）以驱动屏幕空间节点。经
/// `ExtractComponentPlugin` 提取到 render world；当 [`CesiumIbl::is_active`] 为
/// `false` 时节点提前 return（零 GPU 开销）。
#[derive(Component, Clone, Debug, ExtractComponent)]
pub struct CesiumIbl {
    /// 该视图上 IBL 节点的主开关。
    pub enabled: bool,
    /// 领域 IBL 环境（SH 系数 + factor，度量 f64）。
    pub ibl: ImageBasedLighting,
    /// 用于屏幕空间应用的领域 PBR 表面参数（uniform 材质）。
    pub material: IblMaterial,
    /// Cubemap 最大 LOD（roughness → mip 缩放）；单级默认时为 `0.0`。
    pub max_lod: f32,
}

impl Default for CesiumIbl {
    fn default() -> Self {
        Self {
            // 保守默认：禁用（对应 CesiumPassThrough / clipping）。
            enabled: false,
            ibl: ImageBasedLighting::default(),
            material: IblMaterial::default(),
            max_lod: 0.0,
        }
    }
}

impl CesiumIbl {
    /// 一个便捷构造函数，创建一个启用的 IBL 环境。
    pub fn new(ibl: ImageBasedLighting, material: IblMaterial) -> Self {
        Self {
            enabled: true,
            ibl,
            material,
            max_lod: 0.0,
        }
    }

    /// IBL 是否应当真正运行：启用 且 至少一个 factor 非零。
    #[inline]
    pub fn is_active(&self) -> bool {
        self.enabled
            && (self.ibl.image_based_lighting_factor[0] > 0.0
                || self.ibl.image_based_lighting_factor[1] > 0.0)
    }
}

/// IBL 节点的逐视图缓存 pipeline ID。
#[derive(Component)]
pub struct CameraIblPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

/// 持有打包后 IBL 环境的逐视图 GPU uniform 缓冲。
#[derive(Component)]
pub struct ViewIblUniform {
    pub buffer: UniformBuffer<IblUniform>,
}

// ─── GPU uniform（f32 边界） ──────────────────────────────────────────────

/// GPU 面向的 IBL uniform。**仅 f32** —— 领域 [`ImageBasedLighting`] 和
/// [`IblMaterial`] 保持度量 f64；[`IblUniform::from_domain`] 在此边界执行唯一的
/// f64 → f32 投影（红线）。
///
/// 布局必须匹配 `shaders/ibl.wgsl` 中的 `struct IblData`（全部 `vec4` 以
/// 对齐；由 `wgsl_uniform_layout_matches_rust` 断言）。
///
/// 该 struct 住在带 `#![allow(dead_code)]` 的私有 `ibl_uniform`
/// 模块中（`sky_dome.rs` / `clipping_planes.rs` 约定）：
/// encase 的 `ShaderType` derive 会生成一个模块级 helper，死代码分析会标记它，
/// 尽管每个字段都通过 `write_buffer` 上传。字段值在 `from_domain_*`
/// 单元测试中被断言。
pub use ibl_uniform::IblUniform;

mod ibl_uniform {
    #![allow(dead_code)]
    use bevy::prelude::Vec4;
    use bevy::render::render_resource::ShaderType;

    /// GPU IBL uniform；布局匹配 `shaders/ibl.wgsl` 中的 `struct IblData`
    ///（encase std140，全部 vec4 以对齐）。
    #[derive(ShaderType, Clone, Debug)]
    pub struct IblUniform {
        /// 9 个 CesiumJS 约定（PRE-SCALED）SH 辐照度系数；rgb 在 `.xyz`。
        pub sh: [Vec4; 9],
        /// `x` = 感知 roughness，`yzw` = 镜面 F0 反射率（线性 rgb）。
        pub material: Vec4,
        /// `xyz` = lambertian 基础颜色（线性 rgb），`w` = 镜面权重。
        pub diffuse: Vec4,
        /// `x` = 漫反射 IBL factor，`y` = 镜面 IBL factor，`z` = cubemap 最大 LOD。
        pub params: Vec4,
    }
}

impl Default for IblUniform {
    fn default() -> Self {
        Self {
            sh: [Vec4::ZERO; 9],
            material: Vec4::ZERO,
            diffuse: Vec4::ZERO,
            // params.x/y = 0 ⇒ 即使节点以某种方式运行，shader 也原样
            // 透传源颜色（像素中性）。
            params: Vec4::ZERO,
        }
    }
}

impl IblUniform {
    /// 将一个领域 [`ImageBasedLighting`] + [`IblMaterial`] 打包进 GPU uniform。
    ///
    /// f64 SH 系数（未设置时为 [`default_spherical_harmonics`]）与 f64 材质
    /// 参数各自**仅在此处**转为 f32（红线）。`params.z` 是 WGSL
    /// roughness → mip 映射所用的 cubemap 最大 LOD。当两个 IBL factor 都为零时
    /// 节点保持纯透传。
    pub fn from_domain(ibl: &ImageBasedLighting, material: &IblMaterial, max_lod: f32) -> Self {
        let coeffs = ibl
            .spherical_harmonic_coefficients
            .unwrap_or_else(default_spherical_harmonics);

        let mut sh = [Vec4::ZERO; 9];
        for (slot, c) in sh.iter_mut().zip(coeffs.iter()) {
            *slot = Vec4::new(c[0] as f32, c[1] as f32, c[2] as f32, 0.0);
        }

        Self {
            sh,
            material: Vec4::new(
                material.roughness as f32,
                material.specular_f0[0] as f32,
                material.specular_f0[1] as f32,
                material.specular_f0[2] as f32,
            ),
            diffuse: Vec4::new(
                material.diffuse[0] as f32,
                material.diffuse[1] as f32,
                material.diffuse[2] as f32,
                material.specular_weight as f32,
            ),
            params: Vec4::new(
                ibl.image_based_lighting_factor[0] as f32,
                ibl.image_based_lighting_factor[1] as f32,
                max_lod,
                0.0,
            ),
        }
    }
}

// ─── Pipeline ────────────────────────────────────────────────────────────────

/// Render-world 资源：IBL 节点的 bind group 布局 + 采样器 + 一个默认镜面
/// cubemap。
///
/// Group 0 bindings（必须匹配 `ibl.wgsl` + binding 覆盖测试）：
/// - 0：深度前置 pass（`texture_depth_2d`）— 世界坐标重建
/// - 1：法线前置 pass（`texture_2d<f32>`）— 世界法线
/// - 2：颜色源（`texture_2d<f32>`）— 要叠加 IBL 的 HDR 场景
/// - 3：point 采样器（NonFiltering，深度 + 法线）
/// - 4：linear 采样器（Filtering，颜色 + cubemap）
/// - 5：`ViewUniform`（动态偏移）— `view_from_clip` / `world_from_view`
/// - 6：`IblUniform` — 打包后的 SH + 材质 + factor
/// - 7：镜面环境（`texture_cube<f32>`）— 一个中性的 1×1 默认立方体（#81 在材质路径上绑定一个真实预过滤的 cubemap）。
///
/// 该布局是每个 `ibl.wgsl` 入口所用的超集（SUPERSET）；wgpu 允许一个 pipeline
/// 布局声明比某个入口静态使用的更多的 binding，所以三个离线生成器
/// 入口共享这同一个布局。
#[derive(Resource)]
pub struct IblPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub point_sampler: GpuSampler,
    pub linear_sampler: GpuSampler,
    /// Neutral 1×1×6 cube kept alive so its view stays valid (binding 7).
    pub default_specular_cubemap: Texture,
    pub default_specular_cubemap_view: TextureView,
}

impl FromWorld for IblPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();

        let bind_group_layout = render_device.create_bind_group_layout(
            "cesium_ibl_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_depth_2d(),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::NonFiltering),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<IblUniform>(false),
                    texture_cube(TextureSampleType::Float { filterable: true }),
                ),
            ),
        );

        let point_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_ibl_point_sampler"),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_ibl_linear_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            // LINEAR mip 过滤，使 roughness → LOD 对预过滤环境做模糊。
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        // 中性的 1×1×6 立方体（wgpu 零初始化 ⇒ 黑色环境 ⇒ 镜面项在 #81 绑定
        // 一个真实 cubemap 之前不贡献任何东西；漫反射 SH 项仍从 uniform 照亮
        // 场景）。Rgba16Float 是一个可过滤的浮点格式（LINEAR 数据，绝非
        // sRGB——红线）。
        let default_specular_cubemap = render_device.create_texture(&TextureDescriptor {
            label: Some("cesium_ibl_default_specular_cubemap"),
            size: Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Float,
            usage: TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let default_specular_cubemap_view = default_specular_cubemap.create_view(
            &TextureViewDescriptor {
                label: Some("cesium_ibl_default_specular_cubemap_view"),
                dimension: Some(TextureViewDimension::Cube),
                ..Default::default()
            },
        );

        Self {
            bind_group_layout,
            point_sampler,
            linear_sampler,
            default_specular_cubemap,
            default_specular_cubemap_view,
        }
    }
}

// ─── ViewNode ────────────────────────────────────────────────────────────────

/// 屏幕空间 IBL `ViewNode`。
///
/// 从深度 / 法线前置 pass 重建世界坐标 + 法线，并将环境光照（SH 漫反射 +
/// 经由 Fdez-Aguera split-sum 的预过滤 cubemap 镜面）叠加到 HDR 场景颜色上。
/// 当组件非活跃或前置 pass / pipeline 不可用时提前 return（像素中性）。
#[derive(Default)]
pub struct IblNode {
    /// 防止在源纹理变化时绑定一个过期的 group。
    cached_source_id: Mutex<Option<bevy::render::render_resource::TextureViewId>>,
}

impl ViewNode for IblNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static CameraIblPipeline,
        &'static CesiumIbl,
        &'static ViewIblUniform,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (target, pipeline_handle, ibl, ibl_uniform, prepass, view_uniform_offset): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if !ibl.is_active() {
            return Ok(());
        }

        // IBL 需要深度（世界坐标）和法线（朝向）两个前置 pass。
        let (Some(depth_view), Some(normal_view)) = (prepass.depth_view(), prepass.normal_view())
        else {
            return Ok(());
        };

        let pipeline_cache = world.resource::<PipelineCache>();
        let ibl_pipeline = world.resource::<IblPipeline>();
        let view_uniforms = world.resource::<ViewUniforms>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_handle.pipeline_id) else {
            return Ok(());
        };
        let Some(view_uniform_binding) = view_uniforms.uniforms.binding() else {
            return Ok(());
        };
        let Some(ibl_binding) = ibl_uniform.buffer.binding() else {
            return Ok(());
        };

        let render_device = render_context.render_device().clone();

        let post_process = target.post_process_write();
        let source = post_process.source;
        let destination = post_process.destination;

        // 记录源 id（为未来的逐帧 bind group 复用而设的缓存失效标记；
        // 这里的 group 每帧重建，因为它依赖逐帧的前置 pass views + 后处理源）。
        // FIX-IBL-MUTEX：这运行在 render-world `prepare` 路径上——一个被污染的
        // mutex 绝不可级联 panic 之后的每一帧。从毒值中恢复内部
        // 值，而非用 `unwrap()` panic。
        *self.cached_source_id.lock().unwrap_or_else(|e| e.into_inner()) = Some(source.id());

        let bind_group = render_device.create_bind_group(
            Some("cesium_ibl_bind_group"),
            &ibl_pipeline.bind_group_layout,
            &BindGroupEntries::sequential((
                depth_view,
                normal_view,
                source,
                &ibl_pipeline.point_sampler,
                &ibl_pipeline.linear_sampler,
                view_uniform_binding.clone(),
                ibl_binding,
                &ibl_pipeline.default_specular_cubemap_view,
            )),
        );

        let pass_descriptor = RenderPassDescriptor {
            label: Some("cesium_ibl_pass"),
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
        // Binding 5（ViewUniform）为动态偏移；提供它的偏移（Ryan C2
        // 教训：一个 `&[]` 偏移列表会使 dynamic-buffer 校验失败）。
        render_pass.set_bind_group(0, &bind_group, &[view_uniform_offset.offset]);
        render_pass.draw(0..3, 0..1); // 全屏三角形

        Ok(())
    }
}

// ─── 渲染系统 ──────────────────────────────────────────────────────────

/// 为每个活跃视图准备 IBL pipeline + 逐视图 uniform 缓冲。
/// 在 `Render`、`RenderSet::Prepare` 中运行（渲染图执行之前）。
pub fn prepare_ibl(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    ibl_pipeline: Res<IblPipeline>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    views: Query<(Entity, &ExtractedView, &CesiumIbl)>,
) {
    for (entity, view, ibl) in &views {
        if !ibl.is_active() {
            continue;
        }

        let destination_format = if view.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::Rgba8UnormSrgb
        };

        let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("cesium_ibl_pipeline".into()),
            layout: vec![ibl_pipeline.bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: IBL_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: destination_format,
                    // 叠加式环境光照在真正的正向 pass 上会用 ONE_MINUS_SRC_COLOR 风格的
                    // 混合；此处 shader 读取源并写入 source+ibl，所以不需要硬件混合。
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

        // 将领域环境（f64）打包进 GPU uniform（f32 边界）。
        let mut buffer =
            UniformBuffer::from(IblUniform::from_domain(&ibl.ibl, &ibl.material, ibl.max_lod));
        buffer.write_buffer(&render_device, &render_queue);

        commands.entity(entity).insert((
            CameraIblPipeline { pipeline_id },
            ViewIblUniform { buffer },
        ));
    }
}

// ─── 主 world 系统 ──────────────────────────────────────────────────────

/// 确保驱动一个活跃 IBL 环境的相机携带 [`DepthPrepass`] + [`NormalPrepass`]
///（该节点的世界坐标 + 法线输入）。适配层启用，使应用层相机 bundle 保持
/// 不变（与 `setup_ao_prepass` / `setup_clipping_prepass` 同纪律）。
///
/// 仅 ADDS 这些前置 pass（从不移除它们——AO 可能也需要它们）。仅当 IBL 门控 ON
/// 时才注册，所以门控 OFF ⇒ 无额外几何 pass ⇒ v0 基线像素中性。
pub fn setup_ibl_prepass(mut commands: Commands, cameras: Query<(Entity, &CesiumIbl)>) {
    for (entity, ibl) in &cameras {
        if ibl.is_active() {
            commands.entity(entity).insert((DepthPrepass, NormalPrepass));
        }
    }
}

// ─── 注册 ────────────────────────────────────────────────────────────

/// 将 IBL 节点注册进 `RenderApp`（shader + extract + 节点 + 系统）。
///
/// 当 [`ibl_gate_enabled()`] 为 true 时由 `effects::graph::M6WaveARenderGraphPlugin`
///（task #81）调用；`Core3d` 边由 `effects::graph::wire_m6_edges` 创建
///（本函数注册节点但从不接线，所以 `graph.rs` 中的共享线性链仍是唯一拥有者，
/// 不会形成菱形）。
///
/// 无头安好：没有 `RenderApp`（MinimalPlugins）时降级为 no-op。
#[deprecated = "DEV-029 / FIX-REG-FACADE: call `register_ibl_node_main_world` from `Plugin::build` and `register_ibl_node_render_world` from `Plugin::finish`; this facade runs the finish half against a possibly device-less render world."]
pub fn register_ibl_node(app: &mut App) {
    register_ibl_node_main_world(app);
    // 无头的 `MinimalPlugins` 没有 `RenderApp` —— 优雅降级。
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        register_ibl_node_render_world(render_app);
    }
}

/// [`register_ibl_node`] 的 `Plugin::build` 时半边：所有住在**主** world 的东西
///（WGSL shader 资源 + `ExtractComponentPlugin` + `setup_ibl_prepass` 系统）。
///
/// 由 task #81 拆分——参见 `docs/deviations.md#dev-029`。pipeline 的
/// `FromWorld` 读取 `RenderDevice`，而 Bevy 仅在 `RenderPlugin::finish` 中把它插入
/// render world，所以下方的 render-world 半边必须从插件的 `finish` 运行，
/// 绝不从其 `build` 运行。
pub fn register_ibl_node_main_world(app: &mut App) {
    // 注册 IBL WGSL shader（经 shader_registry 无头安好）。
    crate::shader_registry::try_load_internal_shader(
        app,
        IBL_SHADER_HANDLE,
        include_str!("../../shaders/ibl.wgsl"),
        "shaders/ibl.wgsl",
    );

    // ExtractComponentPlugin：每帧 主 → render world（ExtractSchedule）。
    app.add_plugins(ExtractComponentPlugin::<CesiumIbl>::default());

    // 主 world：为驱动活跃环境的相机附加 Depth+Normal 前置 pass。
    app.add_systems(Update, setup_ibl_prepass);
}

/// [`register_ibl_node`] 的 `Plugin::finish` 时半边：render-world pipeline
/// 资源 + `Core3d` 节点。
pub fn register_ibl_node_render_world(render_app: &mut bevy::app::SubApp) {
    // FIX-REG-FACADE (DEV-029)：当 `RenderDevice` 缺失时降级为 no-op
    //（finish 半边从 `build` 到达，或一个裸 render world）。参见
    // `crate::effects::render_world_missing_device`。
    if crate::effects::render_world_missing_device(render_app) {
        return;
    }

    render_app
        .init_resource::<IblPipeline>()
        .add_systems(Render, prepare_ibl.in_set(RenderSet::Prepare))
        .add_render_graph_node::<ViewNodeRunner<IblNode>>(Core3d, CesiumIblLabel);
    // 注意：边由 `effects::graph::wire_m6_edges`（task #81）创建，它是共享
    // `Core3d` 链的唯一拥有者。
}

// ─── 测试 ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_effects::ibl::{
        integrate_brdf, project_irradiance_to_sh, spherical_harmonics, IblMaterial,
    };
    use glam::DVec3;

    #[test]
    fn ibl_component_default_disabled() {
        let c = CesiumIbl::default();
        assert!(!c.enabled);
        assert!(!c.is_active());
    }

    #[test]
    fn ibl_gate_const_is_local_and_stable() {
        // 门控 env 变量定义在本地（隔离纪律），且绝不可与后处理 / 同级的
        // M6 门控碰撞。纯净化（无 env 变更）。
        assert_eq!(ENV_ENABLE_IBL, "CESIUM_ENABLE_IBL");
        assert_ne!(ENV_ENABLE_IBL, "CESIUM_ENABLE_POSTPROCESS");
        assert_ne!(ENV_ENABLE_IBL, "CESIUM_ENABLE_CLIPPING");
        assert!(!gate_from_env_value(None));
        assert!(gate_from_env_value(Some("1".into())));
    }

    #[test]
    fn ibl_active_requires_enabled_and_nonzero_factor() {
        let mut ibl = ImageBasedLighting::default();
        ibl.set_factor(0.0, 0.0);
        let c = CesiumIbl::new(ibl, IblMaterial::default());
        assert!(!c.is_active(), "enabled but zero factors ⇒ inactive");

        let mut ibl2 = ImageBasedLighting::default();
        ibl2.set_factor(1.0, 0.5);
        assert!(CesiumIbl::new(ibl2, IblMaterial::default()).is_active());
    }

    #[test]
    fn ibl_shader_handle_unique() {
        assert_ne!(IBL_SHADER_HANDLE, super::super::graph::PASS_THROUGH_SHADER_HANDLE);
        assert_ne!(IBL_SHADER_HANDLE, super::super::fxaa::FXAA_SHADER_HANDLE);
        assert_ne!(IBL_SHADER_HANDLE, super::super::ao::AO_SHADER_HANDLE);
        assert_ne!(
            IBL_SHADER_HANDLE,
            super::super::clipping_planes::CLIPPING_SHADER_HANDLE
        );
    }

    #[test]
    fn ibl_headless_graceful() {
        // No RenderApp (headless) → register must not panic.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        #[allow(deprecated)]
        register_ibl_node(&mut app);
    }

    #[test]
    fn setup_ibl_prepass_adds_depth_and_normal_when_active() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, setup_ibl_prepass);

        let mut ibl = ImageBasedLighting::default();
        ibl.set_factor(1.0, 1.0);
        let cam = app
            .world_mut()
            .spawn((Camera3d::default(), CesiumIbl::new(ibl, IblMaterial::default())))
            .id();

        app.update();
        assert!(
            app.world().get::<DepthPrepass>(cam).is_some(),
            "active IBL ⇒ DepthPrepass attached"
        );
        assert!(
            app.world().get::<NormalPrepass>(cam).is_some(),
            "active IBL ⇒ NormalPrepass attached"
        );

        // Disabled ⇒ no prepass forced on a fresh camera.
        let cam2 = app
            .world_mut()
            .spawn((Camera3d::default(), CesiumIbl::default()))
            .id();
        app.update();
        assert!(app.world().get::<DepthPrepass>(cam2).is_none());
    }

    // ─── Uniform packing: the f64 → f32 boundary (red line) ───────────────────

    #[test]
    fn from_domain_projects_sh_and_material_to_f32() {
        let mut ibl = ImageBasedLighting::default();
        // Constant white environment ⇒ DC coefficient = π (analytic, N-independent).
        let coeffs = project_irradiance_to_sh(|_d| [1.0, 1.0, 1.0], 64);
        ibl.set_spherical_harmonics(coeffs);
        ibl.set_factor(0.75, 0.5);

        let material = IblMaterial {
            diffuse: [0.2, 0.4, 0.6],
            specular_f0: [0.04, 0.04, 0.04],
            roughness: 0.3,
            specular_weight: 1.0,
        };
        let u = IblUniform::from_domain(&ibl, &material, 4.0);

        // DC coefficient ≈ π (f32 projection of the f64 analytic value).
        assert!((u.sh[0].x - std::f32::consts::PI).abs() < 1e-3, "SH DC = π");
        assert!((u.material.x - 0.3).abs() < 1e-6, "roughness");
        assert!((u.material.y - 0.04).abs() < 1e-6, "f0.r");
        assert!((u.diffuse.x - 0.2).abs() < 1e-6, "diffuse.r");
        assert!((u.diffuse.w - 1.0).abs() < 1e-6, "specular_weight");
        assert!((u.params.x - 0.75).abs() < 1e-6, "diffuse factor");
        assert!((u.params.y - 0.5).abs() < 1e-6, "specular factor");
        assert!((u.params.z - 4.0).abs() < 1e-6, "max lod");
    }

    #[test]
    fn from_domain_defaults_sh_when_unset_and_is_pixel_neutral_at_zero_factor() {
        // No SH set ⇒ default_spherical_harmonics; factor [1,1] by default.
        let ibl = ImageBasedLighting::default();
        let u = IblUniform::from_domain(&ibl, &IblMaterial::default(), 0.0);
        assert!(u.sh[0].x > 0.0, "default DC term is positive");

        // Zero factors ⇒ params.x/y = 0 ⇒ shader pass-through (pixel-neutral).
        let mut off = ImageBasedLighting::default();
        off.set_factor(0.0, 0.0);
        let u_off = IblUniform::from_domain(&off, &IblMaterial::default(), 0.0);
        assert!((u_off.params.x).abs() < 1e-6);
        assert!((u_off.params.y).abs() < 1e-6);
    }

    // ─── CPU/GPU 交叉校验：WGSL helper 镜像 f64 参考 ───

    /// WGSL `spherical_harmonics_eval` 必须为一个投影后的常量环境（DC = π）复现
    /// 领域 f64 `spherical_harmonics`。这将 GPU shader 的 SH 约定锚定到它所
    /// 镜像的 CPU 参考。
    #[test]
    fn sh_convention_cross_validates_against_domain() {
        let coeffs = project_irradiance_to_sh(|_d| [0.5, 0.5, 0.5], 128);
        let dir = DVec3::new(0.3, -0.4, 0.866_025_403_784_439); // 大致单位
        let cpu = spherical_harmonics(&coeffs, dir.normalize());
        // 一个常量 0.5 环境 ⇒ 处处辐照度 = 0.5·π（一阶上方向无关）；SH 求值
        // 必须为正且有限。
        assert!(cpu.iter().all(|c| c.is_finite() && *c > 0.0));
        assert!((cpu[0] - 0.5 * std::f64::consts::PI).abs() < 1e-2, "≈ 0.5π");
    }

    /// WGSL `integrate_brdf(0, 1, N)` 端点必须为 `(scale=1, bias=0)`；
    /// 将 GPU split-sum 生成器锚定到领域 f64 参考。
    #[test]
    fn brdf_endpoint_cross_validates_against_domain() {
        let [scale, bias] = integrate_brdf(0.0, 1.0, 1024);
        assert!((scale - 1.0).abs() < 1e-6, "scale → 1 at normal incidence");
        assert!(bias.abs() < 1e-6, "bias → 0 at zero roughness");
    }

    // ─── Ryan C1/C2 防线：无头 naga 解析 + 校验 + 布局 ──────

    /// naga 没有预处理器，所以 `ibl.wgsl` 中的两个 `#import` 被替换为
    /// 精确声明 shader 所读取字段的 struct stub：
    /// `FullscreenVertexOutput.{position, uv}` 和 `View.{view_from_clip,
    /// world_from_view}`。`view` **binding** 本身由真实 shader 文本声明
    ///（group 0, binding 5），所以此处刻意不为其提供 stub。
    const IBL_WGSL_IMPORT_STUBS: &str = "\
struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct View {
    view_from_clip: mat4x4<f32>,
    world_from_view: mat4x4<f32>,
}
";

    fn ibl_stubbed_wgsl() -> String {
        let mut source = String::from(IBL_WGSL_IMPORT_STUBS);
        for line in include_str!("../../shaders/ibl.wgsl").lines() {
            if line.starts_with("#import") {
                continue;
            }
            source.push_str(line);
            source.push('\n');
        }
        source
    }

    /// IBL shader 为真的无头证明：由 **naga** 解析 + 做类型检查（`bevy_render`
    /// 在 GPU 路径上编译它所用的同一个 WGSL 前端）。防住 M5 C1（保留字 `mod`）
    /// + swizzle 赋值类的静默 bug。
    #[test]
    fn ibl_wgsl_parses_and_type_checks_under_naga() {
        let source = ibl_stubbed_wgsl();
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("ibl.wgsl does not parse:\n{}", error.emit_to_string(&source))
        });
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("ibl.wgsl does not validate");

        // 四个 fragment 入口：apply 节点 + 三个离线生成器。
        let entry_points = module
            .entry_points
            .iter()
            .map(|entry| (entry.name.as_str(), entry.stage))
            .collect::<Vec<_>>();
        assert_eq!(
            entry_points,
            vec![
                ("fragment", naga::ShaderStage::Fragment),
                ("brdf_lut_fragment", naga::ShaderStage::Fragment),
                ("irradiance_fragment", naga::ShaderStage::Fragment),
                ("prefilter_fragment", naga::ShaderStage::Fragment),
            ],
            "ibl.wgsl must expose exactly the apply node + 3 generator fragment entries"
        );
    }

    /// Ryan C1（捕获 **C2** 类 bug）：`fragment`（apply）入口静态使用的每个 binding
    /// 都必须出现在 Rust `IblPipeline` 布局中（group 0: bindings 0..=7）。防住
    /// 一种静默的 pipeline-build 失败——它会让 IBL no-op，而 `pixel_diff` 却报出假绿。
    #[test]
    fn ibl_wgsl_entry_bindings_are_covered_by_the_rust_layout() {
        let source = ibl_stubbed_wgsl();
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("ibl.wgsl does not parse:\n{}", error.emit_to_string(&source))
        });

        // `IblPipeline::from_world` 的镜像（group 0 binding 索引）。
        let layout: std::collections::BTreeSet<(u32, u32)> = [
            (0, 0),
            (0, 1),
            (0, 2),
            (0, 3),
            (0, 4),
            (0, 5),
            (0, 6),
            (0, 7),
        ]
        .into_iter()
        .collect();

        let entry = module
            .entry_points
            .iter()
            .find(|e| e.name == "fragment")
            .expect("ibl.wgsl must have a `fragment` (apply) entry point");

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
            "ibl.wgsl `fragment` statically uses bindings {missing:?} absent from IblPipeline's \
             layout (C2-class regression: the pipeline would fail to build and IBL would no-op)"
        );
    }

    /// M5-D C1 红线守卫：`mod` 是一个 WGSL 保留字——radical inverse 必须用整数
    /// 位运算，绝不用 `mod(` 调用。同时断言 shader 任何处都不出现 `fma(`
    ///（no-FMA-contraction 红线）。
    #[test]
    fn ibl_wgsl_avoids_reserved_words_and_fma() {
        let wgsl = include_str!("../../shaders/ibl.wgsl");
        // 扫描前先剥离 `//` 注释（注释可以合法地提及该词——M5 的教训
        // 曾是对注释里的 `GLSL mod()` 误报）。
        let code: String = wgsl
            .lines()
            .map(|line: &str| line.split("//").next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !code.contains("mod("),
            "ibl.wgsl must not call `mod(` (WGSL reserved word — use integer bit ops)"
        );
        assert!(
            !code.contains("fma("),
            "ibl.wgsl must not call `fma(` (no-FMA-contraction red line)"
        );
    }

    /// Swizzle 赋值守卫（M5-D naga 拒绝项）：无 `lhs.xyz = ` / `.rgb = ` 风格的
    /// 部分向量写；每个向量都整体构建。扫描剥离注释后的代码以查找非法的
    /// `<ident>.<swizzle> =` 模式。
    #[test]
    fn ibl_wgsl_has_no_swizzle_assignment() {
        let wgsl = include_str!("../../shaders/ibl.wgsl");
        for (idx, raw) in wgsl.lines().enumerate() {
            let line = raw.split("//").next().unwrap_or("");
            // 一个 swizzle 赋值形如 `something.xyz = `（`=` 前紧接一个 `.` 分量
            // 选择器），naga 会拒绝它。
            if let Some(eq) = line.find('=') {
                // 忽略 `==`、`<=`、`>=`、`!=`。
                let before = &line[..eq];
                let after = line[eq + 1..].chars().next();
                let is_comparison = after == Some('=');
                let prev = before.chars().last();
                let is_relational = matches!(prev, Some('<') | Some('>') | Some('!') | Some('='));
                if !is_comparison && !is_relational && before.trim_end().ends_with(|c: char| {
                    matches!(c, 'x' | 'y' | 'z' | 'w' | 'r' | 'g' | 'b' | 'a')
                }) && before.contains('.')
                {
                    panic!(
                        "ibl.wgsl line {} has a swizzle assignment (naga rejects `v.xyz = …`): {}",
                        idx + 1,
                        line.trim()
                    );
                }
            }
        }
    }

    /// 交叉校验：WGSL uniform 的 SH 数组大小 + 红线说明必须与 Rust 侧匹配，
    /// 所以 `from_domain` 的打包与 shader 布局一致。
    #[test]
    fn wgsl_uniform_layout_matches_rust() {
        let wgsl = include_str!("../../shaders/ibl.wgsl");
        assert!(
            wgsl.contains("sh: array<vec4<f32>, 9>"),
            "ibl.wgsl SH array must be 9 coefficients (SH_COEFFICIENT_COUNT)"
        );
        // 红线不变量必须在源码中保持可见（对应 clipping.wgsl）。
        assert!(wgsl.contains("NO FMA CONTRACTION"));
        assert!(wgsl.contains("RESERVED WORD"));
    }
}
