//! M6.4：cesiumrust OIT（与顺序无关的透明度）—— MRT 渲染图适配器。
//!
//! 为加权混合 OIT 实现 Bevy ViewNode 基础设施，使用
//! 两个 MRT 渲染目标（Rgba16Float 累积 + R8Unorm revealage）以及一个
//! 全屏合成 pass。
//!
//! # 蓝图
//! - `packages/engine/Source/Scene/OIT.js` L1-947 (28KB main implementation)
//!   - L30-34: capability detection (drawBuffers && colorBufferFloat && depthTexture && floatBlend)
//!   - L137-156: updateTextures (accumulation RGBA FLOAT + revealage RGBA FLOAT)
//!   - L164-226: updateFramebuffers (MRT 2-attachment FBO)
//!   - L408-417: translucentMRTBlend (RGB additive, Alpha multiplicative)
//!   - L487-492: mrtShaderSource (Ci*wzi → FragData_0, ai*wzi → FragData_1)
//!   - L786-828: executeTranslucentCommandsSortedMRT
//!   - L872-874: composite execution
//! - `packages/engine/Source/Shaders/CompositeOITFS.glsl` L1-32 (901B composite)
//! - `packages/engine/Source/Shaders/Builtin/Functions/alphaWeight.glsl` L4-11
//! - `domain/effects/src/oit.rs` L1-366 (CPU reference: compute_weight, accumulate, composite)
//!
//! # 架构
//! - `OitNode`：运行 MRT 累积 pass 的 ViewNode（读取场景颜色 + 深度，
//!   以相应的混合状态输出到 2 个 attachment）。
//! - `OitCompositeNode`：运行全屏合成的 ViewNode（读取累积 +
//!   revealage + 不透明，将最终颜色写入目标）。
//! - `OitTextureCache`：持有逐视图累积 + revealage
//!   纹理的 render-world 资源（在视口尺寸变化时重建）。
//! - 能力探测：`Plugin::finish` 读取 `RenderDevice::limits()` 以验证
//!   `max_color_attachments >= 2`（MRT）。探测失败 → 门控 OFF，不 panic。
//!
//! # 偏差
//! 参见 `docs/deviations.md#dev-031`（草稿，由集成者登记）。

use std::collections::HashMap;
use std::sync::Mutex;

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
        BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, BlendComponent,
        BlendFactor, BlendOperation, BlendState, CachedRenderPipelineId, ColorTargetState,
        ColorWrites, Extent3d, FilterMode, FragmentState, MultisampleState,
        Operations, PipelineCache, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipelineDescriptor, Sampler as GpuSampler, SamplerBindingType, SamplerDescriptor,
        ShaderStages, Texture, TextureDescriptor, TextureDimension, TextureFormat,
        TextureSampleType, TextureUsages, TextureView,
    },
    renderer::{RenderContext, RenderDevice},
    view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
    Render, RenderApp, RenderSet,
};
// `wgpu::Color` 不通过 Bevy 的 `render_resource` 按名重新导出
//（只有 `ColorTargetState`/`ColorWrites` 会）；MRT 累积 pass 需要它
// 来做逐帧 attachment 清除。关于为何该直接依赖是版本安全的
//（统一为 Bevy 所用的同一个 23.0.1 实例）参见 `Cargo.toml`。
use wgpu::Color as GpuColor;
use cesium_effects::{OitCapabilities, OitConfig as DomainOitConfig, OitMode};

use super::graph::gate_from_env_value;

// ─── Shader handle ─────────────────────────────────────────────────────────

/// 内嵌 `oit_accumulate.wgsl` shader 的唯一 handle。
pub const OIT_ACCUMULATE_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0xCE51_0170_0006_0004);

/// 内嵌 `oit_composite.wgsl` shader 的唯一 handle。
pub const OIT_COMPOSITE_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0xCE51_0170_0006_0005);

// ─── 门控 ───────────────────────────────────────────────────────────────────

/// 门控 M6.4 OIT 节点的环境变量。
///
/// **单一真相源（task #81）**：该名的拥有者是应用层
/// 注册表 `application/cesium-app/src/feature_flags.rs`（`ENV_ENABLE_OIT`
/// 列在 `RESERVED_FLAGS`，默认 OFF）。本 const 是一个*镜像*，
/// 仅因 `cesium-app` 依赖 `cesium-bevy-render`（绝不反向）而存在。
/// 集成任务 #93 将其提升为 ACTIVE。
pub const ENV_ENABLE_OIT: &str = "CESIUM_ENABLE_OIT";

/// 当 OIT 门控启用时返回 `true`。复用单一权威的
/// truthy 解析器（`gate_from_env_value`，全 crate 的 `{1, true, yes, on}` 集）。
#[inline]
pub fn oit_gate_enabled() -> bool {
    gate_from_env_value(std::env::var(ENV_ENABLE_OIT).ok())
}

// ─── 渲染图 label ────────────────────────────────────────────────────

/// OIT 累积（MRT）pass 在 `Core3d` 中的节点 label。
///
/// 推荐插入位置：`MainTransmissivePass → CesiumOitLabel → CesiumOitCompositeLabel → EndMainPass`。
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct CesiumOitLabel;

/// OIT 合成 pass 在 `Core3d` 中的节点 label。
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct CesiumOitCompositeLabel;

// ─── 组件 ──────────────────────────────────────────────────────────────

/// 在相机实体上启用 cesiumrust OIT 的标记组件。
///
/// 经 `ExtractComponentPlugin` 提取到 render world。当 `enabled == false` 时
/// `OitNode` 提前 return（零 GPU 开销，门控 OFF = v0 零差异）。
#[derive(Component, Clone, Debug, ExtractComponent)]
pub struct CesiumOit {
    /// 该相机上 OIT pass 的主开关。
    pub enabled: bool,
}

impl Default for CesiumOit {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// OIT 累积节点的逐视图缓存 pipeline ID。
#[derive(Component)]
pub struct CameraOitPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

/// OIT 合成节点的逐视图缓存 pipeline ID。
#[derive(Component)]
pub struct CameraOitCompositePipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

// ─── 适配器 OitConfig 资源（从脚手架保留） ───────────────────

#[derive(Resource, Debug, Clone)]
pub struct OitConfig {
    pub enabled: bool,
    pub mode: OitMode,
    pub depth_test: bool,
    pub depth_write: bool,
    pub accumulation_clear: [f64; 4],
    pub revealage_clear: f64,
}

impl Default for OitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: OitMode::None,
            depth_test: true,
            depth_write: true,
            accumulation_clear: [0.0, 0.0, 0.0, 0.0],
            revealage_clear: 1.0,
        }
    }
}

impl OitConfig {
    pub fn from_domain(config: &DomainOitConfig) -> Self {
        Self {
            enabled: config.is_active(),
            mode: config.mode,
            depth_test: true,
            depth_write: true,
            accumulation_clear: [0.0, 0.0, 0.0, 0.0],
            revealage_clear: 1.0,
        }
    }
}

// ─── OITPlugin ──────────────────────────────────────────────────────────────
// M6.1 Split 脚手架（`SplitConfig` / `SplitDragEvent` /
// `split_direction_system`）在 FIX-SPLIT（Phase 3）中迁移到了 `effects::split`；
// 本插件现在只初始化 OIT 配置。
pub struct OITPlugin;

impl Plugin for OITPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OitConfig>();
    }
}

// ─── OIT Pipeline 资源 ──────────────────────────────────────────────────

/// Render-world 资源：OIT pass 的 bind group layout + 采样器。
#[derive(Resource)]
pub struct OitPipeline {
    /// 累积 pass 的 layout（depth + scene_colour + 采样器 + view uniform）。
    pub accumulate_bind_group_layout: BindGroupLayout,
    /// 合成 pass 的 layout（不透明 + 累积 + revealage + 采样器）。
    pub composite_bind_group_layout: BindGroupLayout,
    /// 点采样器（非过滤，用于深度）。
    pub point_sampler: GpuSampler,
    /// 线性采样器（过滤，用于颜色纹理）。
    pub linear_sampler: GpuSampler,
}

impl FromWorld for OitPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();

        // Accumulate pass: depth_prepass(0) + scene_color(1) + point_sampler(2) + view(3)
        let accumulate_bind_group_layout = render_device.create_bind_group_layout(
            "cesium_oit_accumulate_bgl",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_depth_2d(),
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    sampler(SamplerBindingType::NonFiltering),
                    uniform_buffer::<ViewUniform>(true),
                ),
            ),
        );

        // Composite pass: opaque(0) + accumulation(1) + revealage(2) + linear_sampler(3)
        let composite_bind_group_layout = render_device.create_bind_group_layout(
            "cesium_oit_composite_bgl",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );

        let point_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_oit_point_sampler"),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_oit_linear_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            accumulate_bind_group_layout,
            composite_bind_group_layout,
            point_sampler,
            linear_sampler,
        }
    }
}

// ─── OIT 纹理缓存 ──────────────────────────────────────────────────────

/// 逐视图 OIT 中间纹理（累积 + revealage）。
///
/// 在视口尺寸变化时重建。在同一帧内于 `OitNode`（写）与
/// `OitCompositeNode`（读）之间共享。
#[derive(Resource, Default)]
pub struct OitTextureCache {
    pub cache: Mutex<HashMap<Entity, OitViewTextures>>,
}

pub struct OitViewTextures {
    pub width: u32,
    pub height: u32,
    pub accumulation_texture: Texture,
    pub revealage_texture: Texture,
    /// 合成 pass 的可绑定视图（保留为 TextureView 对象）。
    pub accumulation_bind_view: TextureView,
    pub revealage_bind_view: TextureView,
}

/// 为一个视图实体创建或检索 OIT 纹理。若分配失败则返回 `None`。
fn ensure_oit_textures(
    cache: &OitTextureCache,
    render_device: &RenderDevice,
    entity: Entity,
    width: u32,
    height: u32,
) -> bool {
    let mut map = cache.cache.lock().unwrap();
    if let Some(existing) = map.get(&entity) {
        if existing.width == width && existing.height == height {
            return true; // 已按正确尺寸分配。
        }
    }

    // 分配累积纹理（Rgba16Float）。
    let accumulation_texture = render_device.create_texture(&TextureDescriptor {
        label: Some("cesium_oit_accumulation"),
        size: Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let accumulation_bind_view = accumulation_texture.create_view(&Default::default());

    // 分配 revealage 纹理（R8Unorm）。
    let revealage_texture = render_device.create_texture(&TextureDescriptor {
        label: Some("cesium_oit_revealage"),
        size: Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::R8Unorm,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let revealage_bind_view = revealage_texture.create_view(&Default::default());

    map.insert(
        entity,
        OitViewTextures {
            width,
            height,
            accumulation_texture,
            revealage_texture,
            accumulation_bind_view,
            revealage_bind_view,
        },
    );
    true
}

// ─── OitNode（MRT 累积 pass） ─────────────────────────────────────────

/// 运行 OIT MRT 累积 pass 的 ViewNode。
///
/// 读取场景颜色 + 深度前置 pass，输出到两个渲染目标：
/// - Attachment 0（Rgba16Float）：加法混合 → Σ(Ci·wzi), Σ(ai·wzi)
/// - Attachment 1（R8Unorm）：乘法混合 → Π(1−ai)
#[derive(Default)]
pub struct OitNode;

impl ViewNode for OitNode {
    type ViewQuery = (
        Entity,
        &'static ViewTarget,
        &'static ExtractedView,
        &'static CameraOitPipeline,
        &'static CesiumOit,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (entity, target, _view, pipeline_handle, oit, prepass, view_uniform_offset): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if !oit.enabled {
            return Ok(());
        }

        let pipeline_cache = world.resource::<PipelineCache>();
        let oit_pipeline = world.resource::<OitPipeline>();
        let texture_cache = world.resource::<OitTextureCache>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_handle.pipeline_id) else {
            return Ok(());
        };

        let Some(depth_view) = prepass.depth_view() else {
            return Ok(());
        };

        // 确保该视图的 OIT 纹理存在。
        let target_size = target.main_texture_view();
        let _ = target_size; // 通过纹理缓存间接使用。
        let viewport_size = world.resource::<OitCapabilitiesResource>();
        if !viewport_size.mrt_supported {
            return Ok(()); // 能力探测失败——优雅退出。
        }

        // 从视图目标获取纹理尺寸。
        let (width, height) = {
            let cache = texture_cache.cache.lock().unwrap();
            if let Some(tex) = cache.get(&entity) {
                (tex.width, tex.height)
            } else {
                return Ok(()); // 纹理尚未准备。
            }
        };
        let _ = (width, height);

        // 构建 bind group：depth + scene source + 采样器 + view uniform。
        let post_process = target.post_process_write();
        let source = post_process.source;

        let view_uniforms = world.resource::<ViewUniforms>();
        let Some(view_uniform_buffer) = view_uniforms.uniforms.binding() else {
            return Ok(());
        };

        let bind_group = render_context.render_device().create_bind_group(
            Some("cesium_oit_accumulate_bg"),
            &oit_pipeline.accumulate_bind_group_layout,
            &BindGroupEntries::sequential((
                depth_view,
                source,
                &oit_pipeline.point_sampler,
                view_uniform_buffer,
            )),
        );

        // 获取用于 MRT attachment 的 OIT 纹理视图。
        let cache = texture_cache.cache.lock().unwrap();
        let Some(textures) = cache.get(&entity) else {
            return Ok(());
        };

        let pass_descriptor = RenderPassDescriptor {
            label: Some("cesium_oit_accumulate_pass"),
            color_attachments: &[
                // Attachment 0：累积（Rgba16Float，加法混合）
                Some(RenderPassColorAttachment {
                    view: &textures.accumulation_bind_view,
                    resolve_target: None,
                    ops: Operations {
                        load: bevy::render::render_resource::LoadOp::Clear(GpuColor::TRANSPARENT),
                        store: bevy::render::render_resource::StoreOp::Store,
                    },
                }),
                // Attachment 1：revealage（R8Unorm，乘法混合）
                Some(RenderPassColorAttachment {
                    view: &textures.revealage_bind_view,
                    resolve_target: None,
                    ops: Operations {
                        load: bevy::render::render_resource::LoadOp::Clear(GpuColor::WHITE),
                        store: bevy::render::render_resource::StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        let mut render_pass = render_context
            .command_encoder()
            .begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[view_uniform_offset.offset]);
        render_pass.draw(0..3, 0..1); // 全屏三角形

        Ok(())
    }
}

// ─── OitCompositeNode（全屏合成） ────────────────────────────────

/// 将 OIT 缓冲与不透明场景合成的 ViewNode。
///
/// 读取累积 + revealage 纹理和不透明场景颜色，
/// 应用 CompositeOITFS 公式，写入最终混合结果。
#[derive(Default)]
pub struct OitCompositeNode;

impl ViewNode for OitCompositeNode {
    type ViewQuery = (
        Entity,
        &'static ViewTarget,
        &'static CameraOitCompositePipeline,
        &'static CesiumOit,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (entity, target, pipeline_handle, oit): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if !oit.enabled {
            return Ok(());
        }

        let pipeline_cache = world.resource::<PipelineCache>();
        let oit_pipeline = world.resource::<OitPipeline>();
        let texture_cache = world.resource::<OitTextureCache>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_handle.pipeline_id) else {
            return Ok(());
        };

        let post_process = target.post_process_write();
        let source = post_process.source; // 不透明场景
        let destination = post_process.destination;

        // 获取 OIT 纹理视图。
        let cache = texture_cache.cache.lock().unwrap();
        let Some(textures) = cache.get(&entity) else {
            return Ok(());
        };

        let bind_group = render_context.render_device().create_bind_group(
            Some("cesium_oit_composite_bg"),
            &oit_pipeline.composite_bind_group_layout,
            &BindGroupEntries::sequential((
                source,
                &textures.accumulation_bind_view,
                &textures.revealage_bind_view,
                &oit_pipeline.linear_sampler,
            )),
        );
        drop(cache);

        let pass_descriptor = RenderPassDescriptor {
            label: Some("cesium_oit_composite_pass"),
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
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1); // 全屏三角形

        Ok(())
    }
}

// ─── 能力探测资源 ──────────────────────────────────────────────

/// 存储 OIT 能力探测结果的 render-world 资源。
///
/// 在 `Plugin::finish` 期间通过读取 `RenderDevice::limits()` 填充。
/// 探测失败 → `mrt_supported = false` → 节点提前 return（绝不 panic）。
#[derive(Resource, Debug, Clone, Default)]
pub struct OitCapabilitiesResource {
    pub mrt_supported: bool,
    pub max_color_attachments: u32,
}

impl OitCapabilitiesResource {
    /// 探测渲染设备对 OIT MRT 的支持。
    ///
    /// wgpu/WebGPU 中唯一可靠的运行时判别依据是
    /// `max_color_attachments >= 2`（对应上游 `context.drawBuffers`）。
    /// 浮点混合和 color-buffer-float 在 WebGPU 中由核心保证
    ///（不存在运行时查询 API）；深度纹理同理。
    ///
    /// 对应 OIT.js L30-34：
    /// ```js
    /// extensionsSupported = colorBufferFloat && depthTexture && floatBlend;
    /// _translucentMRTSupport = drawBuffers && extensionsSupported;
    /// ```
    pub fn probe(render_device: &RenderDevice) -> Self {
        let limits = render_device.limits();
        let max_ca = limits.max_color_attachments;
        let mrt_supported = max_ca >= 2;

        Self {
            mrt_supported,
            max_color_attachments: max_ca,
        }
    }

    /// 转换为领域 capabilities 以便互操作。
    pub fn to_domain_caps(&self) -> OitCapabilities {
        OitCapabilities {
            mrt_supported: self.mrt_supported,
            // 在 wgpu/WebGPU 中，当 Rgba16Float 可渲染时这些由核心保证。
            float_blend_supported: self.mrt_supported,
            depth_texture_supported: true,
            color_buffer_float: true,
        }
    }
}

// ─── 渲染系统 ─────────────────────────────────────────────────────────

/// 为每个相机视图准备 OIT 纹理和专用 pipeline。
/// 运行在 `Render` 调度的 `RenderSet::Prepare`。
pub fn prepare_oit_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
    oit_pipeline: Res<OitPipeline>,
    caps: Res<OitCapabilitiesResource>,
    texture_cache: Res<OitTextureCache>,
    views: Query<(Entity, &ExtractedView, &CesiumOit, &ViewTarget)>,
) {
    if !caps.mrt_supported {
        return;
    }

    for (entity, view, oit, _target) in &views {
        if !oit.enabled {
            continue;
        }

        let width = view.viewport.z.max(1);
        let height = view.viewport.w.max(1);

        // 确保 OIT 中间纹理已分配。
        ensure_oit_textures(&texture_cache, &render_device, entity, width, height);

        // 累积 pipeline：2 个颜色目标（Rgba16Float 加法 + R8Unorm 乘法）。
        let accumulate_pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("cesium_oit_accumulate_pipeline".into()),
            layout: vec![oit_pipeline.accumulate_bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: OIT_ACCUMULATE_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "fragment".into(),
                targets: vec![
                    // Attachment 0：Rgba16Float，加法混合（ONE + ONE）
                    Some(ColorTargetState {
                        format: TextureFormat::Rgba16Float,
                        blend: Some(BlendState {
                            color: BlendComponent {
                                src_factor: BlendFactor::One,
                                dst_factor: BlendFactor::One,
                                operation: BlendOperation::Add,
                            },
                            alpha: BlendComponent {
                                src_factor: BlendFactor::One,
                                dst_factor: BlendFactor::One,
                                operation: BlendOperation::Add,
                            },
                        }),
                        write_mask: ColorWrites::ALL,
                    }),
                    // Attachment 1：R8Unorm，乘法混合（ZERO + ONE_MINUS_SRC）
                    Some(ColorTargetState {
                        format: TextureFormat::R8Unorm,
                        blend: Some(BlendState {
                            color: BlendComponent {
                                src_factor: BlendFactor::Zero,
                                dst_factor: BlendFactor::OneMinusSrc,
                                operation: BlendOperation::Add,
                            },
                            alpha: BlendComponent {
                                src_factor: BlendFactor::Zero,
                                dst_factor: BlendFactor::OneMinusSrc,
                                operation: BlendOperation::Add,
                            },
                        }),
                        write_mask: ColorWrites::RED,
                    }),
                ],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: Vec::new(),
            zero_initialize_workgroup_memory: false,
        });

        // 合成 pipeline：单个目标（view format），无混合。
        let output_format = if view.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::Rgba8UnormSrgb
        };

        let composite_pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("cesium_oit_composite_pipeline".into()),
            layout: vec![oit_pipeline.composite_bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: OIT_COMPOSITE_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: output_format,
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

        commands.entity(entity).insert((
            CameraOitPipeline { pipeline_id: accumulate_pipeline_id },
            CameraOitCompositePipeline { pipeline_id: composite_pipeline_id },
        ));
    }
}

/// 主 world 系统：为带 `CesiumOit` 的相机附加 `DepthPrepass`。
///
/// 累积 pass 需要深度来进行权重计算。
pub fn setup_oit_prepass(
    mut commands: Commands,
    cameras: Query<Entity, (With<CesiumOit>, Without<DepthPrepass>)>,
) {
    for entity in &cameras {
        commands.entity(entity).insert(DepthPrepass);
    }
}

// ─── 注册 ───────────────────────────────────────────────────────────

/// 将 OIT 节点注册进 `RenderApp`（shader + extract + 节点 + 系统）。
///
/// 当 OIT 门控 ON 时由 `M6WaveARenderGraphPlugin` 调用。
/// 不创建图边——集成者 #93 拥有链拓扑。
#[deprecated = "DEV-029 / FIX-REG-FACADE: call `register_oit_node_main_world` from `Plugin::build` and `register_oit_node_render_world` from `Plugin::finish`; this facade runs the finish half against a possibly device-less render world."]
pub fn register_oit_node(app: &mut App) {
    register_oit_node_main_world(app);
    // 无头 `MinimalPlugins` 没有 `RenderApp`——优雅降级。
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        register_oit_node_render_world(render_app);
    }
}

/// `Plugin::build` 时的前半：所有住在**主** world 中的东西。
///
/// 按 DEV-029 模式拆出。`OitPipeline` 的 `FromWorld` 读取 `RenderDevice`，
/// 而 Bevy 只在 `RenderPlugin::finish` 中把 `RenderDevice` 插入 render world，
/// 所以 render-world 半必须从 `finish` 调用。
pub fn register_oit_node_main_world(app: &mut App) {
    // 注册 OIT WGSL shader（无头安好）。
    crate::shader_registry::try_load_internal_shader(
        app,
        OIT_ACCUMULATE_SHADER_HANDLE,
        include_str!("../../shaders/oit_accumulate.wgsl"),
        "shaders/oit_accumulate.wgsl",
    );
    crate::shader_registry::try_load_internal_shader(
        app,
        OIT_COMPOSITE_SHADER_HANDLE,
        include_str!("../../shaders/oit_composite.wgsl"),
        "shaders/oit_composite.wgsl",
    );

    // CesiumOit 的 ExtractComponentPlugin。
    app.add_plugins(ExtractComponentPlugin::<CesiumOit>::default());

    // 主 world 前置 pass 设置。
    app.add_systems(bevy::app::Last, setup_oit_prepass);
}

/// `Plugin::finish` 时的后半：render-world pipeline 资源 + 节点。
///
/// **必须**从 `Plugin::finish` 调用（DEV-029：RenderDevice 只在
/// RenderPlugin::finish 插入它之后存在）。
pub fn register_oit_node_render_world(render_app: &mut bevy::app::SubApp) {
    // FIX-REG-FACADE（DEV-029）：当 `RenderDevice` 缺失时降级为 no-op
    //（从 `build` 到达 finish 半，或一个裸 render world）——下面的能力
    // 探测会解引用它。参见
    // `crate::effects::render_world_missing_device`。
    if crate::effects::render_world_missing_device(render_app) {
        return;
    }

    // 能力探测——读取 RenderDevice.limits()（仅在 finish 中可用）。
    let render_device = render_app
        .world()
        .resource::<RenderDevice>()
        .clone();
    let caps = OitCapabilitiesResource::probe(&render_device);

    render_app
        .insert_resource(caps)
        .init_resource::<OitPipeline>()
        .init_resource::<OitTextureCache>()
        .add_systems(Render, prepare_oit_pipelines.in_set(RenderSet::Prepare))
        .add_render_graph_node::<ViewNodeRunner<OitNode>>(Core3d, CesiumOitLabel)
        .add_render_graph_node::<ViewNodeRunner<OitCompositeNode>>(Core3d, CesiumOitCompositeLabel);
    // NOTE：边由集成者 #93 创建（wire_m6_edges 或等价物）。
}

// ─── 测试 ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_effects::OitCapabilities;

    #[test]
    fn test_oit_config_default() {
        let config = OitConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.mode, OitMode::None);
        assert!(config.depth_test);
    }

    #[test]
    fn test_oit_config_from_domain_active() {
        let domain = DomainOitConfig::default();
        let config = OitConfig::from_domain(&domain);
        assert!(!config.enabled);
    }

    #[test]
    fn test_oit_config_from_domain_mrt() {
        let caps = OitCapabilities {
            mrt_supported: true,
            float_blend_supported: true,
            depth_texture_supported: true,
            color_buffer_float: true,
        };
        let domain = DomainOitConfig::from_capabilities(&caps);
        let config = OitConfig::from_domain(&domain);
        assert!(config.enabled);
        assert_eq!(config.mode, OitMode::WeightedBlendedMrt);
    }

    // M6.1 Split 测试（`test_split_config_*` / `test_split_direction_properties`）
    // 随脚手架一起在 FIX-SPLIT（Phase 3）中迁移到了 `effects::split`。

    // ─── 新增 M6.4 测试 ─────────────────────────────────────────────────────

    #[test]
    fn test_oit_gate_default_off() {
        // 未设置环境变量时，门控应为 OFF。
        std::env::remove_var(ENV_ENABLE_OIT);
        assert!(!oit_gate_enabled());
    }

    #[test]
    fn test_oit_shader_handles_unique() {
        assert_ne!(OIT_ACCUMULATE_SHADER_HANDLE, OIT_COMPOSITE_SHADER_HANDLE);
        assert_ne!(
            OIT_ACCUMULATE_SHADER_HANDLE,
            super::super::graph::PASS_THROUGH_SHADER_HANDLE
        );
    }

    #[test]
    fn test_cesium_oit_component_default() {
        let oit = CesiumOit::default();
        assert!(oit.enabled);
    }

    #[test]
    fn test_oit_capabilities_resource_default() {
        let caps = OitCapabilitiesResource::default();
        assert!(!caps.mrt_supported);
        assert_eq!(caps.max_color_attachments, 0);
    }

    #[test]
    fn test_oit_capabilities_to_domain() {
        let caps = OitCapabilitiesResource {
            mrt_supported: true,
            max_color_attachments: 8,
        };
        let domain = caps.to_domain_caps();
        assert!(domain.mrt_supported);
        assert!(domain.float_blend_supported);
        assert!(domain.depth_texture_supported);
        assert!(domain.color_buffer_float);
        assert!(domain.translucent_mrt_supported());
    }

    #[test]
    fn test_oit_headless_graceful() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // 不应 panic（没有 RenderApp）。
        #[allow(deprecated)]
        register_oit_node(&mut app);
    }

    // ─── Naga 防线：解析 + 校验 + binding 覆盖 ─────────────

    /// 用于 naga 无法解析的 `#import` 指令的 stub。
    const OIT_WGSL_IMPORT_STUBS: &str = "\
struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
struct View {
    view_from_clip: mat4x4<f32>,
    clip_from_view: mat4x4<f32>,
    view_from_world: mat4x4<f32>,
    world_from_view: mat4x4<f32>,
    clip_from_world: mat4x4<f32>,
    world_from_clip: mat4x4<f32>,
    world_position: vec3<f32>,
    near: f32,
    far: f32,
    width: f32,
    height: f32,
    viewport: vec4<f32>,
    frustum: vec4<f32>,
}
";

    fn oit_stubbed_wgsl(path: &str) -> String {
        let mut source = String::from(OIT_WGSL_IMPORT_STUBS);
        for line in path.lines() {
            if line.starts_with("#import") {
                continue;
            }
            source.push_str(line);
            source.push('\n');
        }
        source
    }

    #[test]
    fn oit_accumulate_wgsl_parses_and_type_checks_under_naga() {
        let source = oit_stubbed_wgsl(include_str!("../../shaders/oit_accumulate.wgsl"));
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("oit_accumulate.wgsl does not parse:\n{}", error.emit_to_string(&source))
        });
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("oit_accumulate.wgsl does not validate");

        let entry_points: Vec<_> = module
            .entry_points
            .iter()
            .map(|e| (e.name.as_str(), e.stage))
            .collect();
        assert_eq!(
            entry_points,
            vec![("fragment", naga::ShaderStage::Fragment)],
            "oit_accumulate.wgsl must expose exactly one fragment entry point"
        );
    }

    #[test]
    fn oit_composite_wgsl_parses_and_type_checks_under_naga() {
        let source = oit_stubbed_wgsl(include_str!("../../shaders/oit_composite.wgsl"));
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("oit_composite.wgsl does not parse:\n{}", error.emit_to_string(&source))
        });
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("oit_composite.wgsl does not validate");

        let entry_points: Vec<_> = module
            .entry_points
            .iter()
            .map(|e| (e.name.as_str(), e.stage))
            .collect();
        assert_eq!(
            entry_points,
            vec![("fragment", naga::ShaderStage::Fragment)],
            "oit_composite.wgsl must expose exactly one fragment entry point"
        );
    }

    #[test]
    fn oit_accumulate_wgsl_bindings_covered_by_layout() {
        let source = oit_stubbed_wgsl(include_str!("../../shaders/oit_accumulate.wgsl"));
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("oit_accumulate.wgsl parse failed:\n{}", error.emit_to_string(&source))
        });

        // Rust layout: group(0) bindings 0..3 (depth, scene_color, point_sampler, view)
        let layout: std::collections::BTreeSet<(u32, u32)> =
            [(0, 0), (0, 1), (0, 2), (0, 3)].into_iter().collect();

        let entry = module
            .entry_points
            .iter()
            .find(|e| e.name == "fragment")
            .expect("must have fragment entry");

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
                    naga::Expression::CallResult(func_handle) if visited.insert(func_handle) => {
                        stack.push(&module.functions[func_handle]);
                    }
                    _ => {}
                }
            }
        }

        let missing: Vec<(u32, u32)> = used.difference(&layout).copied().collect();
        assert!(
            missing.is_empty(),
            "oit_accumulate.wgsl uses bindings {missing:?} absent from OitPipeline layout"
        );
    }

    #[test]
    fn oit_composite_wgsl_bindings_covered_by_layout() {
        let source = oit_stubbed_wgsl(include_str!("../../shaders/oit_composite.wgsl"));
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("oit_composite.wgsl parse failed:\n{}", error.emit_to_string(&source))
        });

        // Rust layout: group(0) bindings 0..3 (opaque, accumulation, revealage, linear_sampler)
        let layout: std::collections::BTreeSet<(u32, u32)> =
            [(0, 0), (0, 1), (0, 2), (0, 3)].into_iter().collect();

        let entry = module
            .entry_points
            .iter()
            .find(|e| e.name == "fragment")
            .expect("must have fragment entry");

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
                    naga::Expression::CallResult(func_handle) if visited.insert(func_handle) => {
                        stack.push(&module.functions[func_handle]);
                    }
                    _ => {}
                }
            }
        }

        let missing: Vec<(u32, u32)> = used.difference(&layout).copied().collect();
        assert!(
            missing.is_empty(),
            "oit_composite.wgsl uses bindings {missing:?} absent from OitPipeline layout"
        );
    }
}
