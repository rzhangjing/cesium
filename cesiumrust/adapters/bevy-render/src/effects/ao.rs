//! M5-E2：屏幕空间环焦遮蔽（SSAO）`ViewNode`。
//!
//! 自行实现的半球 16 采样 SSAO 核 + 4×4 盒模糊，插入
//! M5-E0 渲染图基础设施（`graph.rs`）中 pass-through 节点与 FXAA 之间。
//! 对应 [`super::fxaa`] 的 `register_fxaa_node` 模式：本模块
//! 注册节点 / 资源 / 系统，**但从不创建图边** ——
//! `graph.rs::register_render_graph` 拥有单一线性链，所以 `Core3d` 中不会形成菱形。
//!
//! # 链上位置（由 graph.rs 拥有）
//! `EndMainPass → PassThrough → AmbientOcclusion → Tonemapping → Fxaa → EndMainPassPostProcessing`
//! AO 在 tonemapping **之前**作用于 HDR 场景色，FXAA 在 tonemapping **之后**
//! 作用于最终 LDR 图像 —— 与上游 CesiumJS 的
//! `PostProcessStageCollection` 顺序一致（AO → … → Tonemapping → … → FXAA 最后）。
//!
//! # 输入：DepthPrepass + NormalPrepass
//! SSAO 需要视图空间深度 + 法线。一旦相机携带 [`DepthPrepass`] +
//! [`NormalPrepass`]（两者先前全仓 **零使用**），Bevy 就通过
//! [`ViewPrepassTextures`] 暴露它们。它们在此由 [`setup_ao_prepass`] 在适配层启用
//! —— **不在**应用层相机 bundle（orbit_camera.rs 因 M5-E2 红线而超出范围）。
//!
//! # 蓝图
//! - `packages/engine/Source/Shaders/PostProcessStages/AmbientOcclusionGenerate.glsl` L1-144
//! - `packages/engine/Source/Shaders/PostProcessStages/AmbientOcclusionModulate.glsl` L1-11
//! - `packages/engine/Source/Scene/PostProcessStageLibrary.js` L496 / L599
//! - `bevy_pbr-0.15.3/src/ssao/mod.rs` L224-323 (SsaoNode) / L682-772 (prepass binding)
//!
//! # 偏差
//! WGSL 重写；半球核 SSAO（非 CesiumJS 的 HBAO 射线行进）。参见
//! `docs/deviations.md#dev-018`。

use std::collections::HashMap;
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
        NodeRunError, RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner,
    },
    render_resource::{
        binding_types::{sampler, texture_2d, texture_depth_2d, uniform_buffer},
        BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId,
        ColorTargetState, ColorWrites, FilterMode, FragmentState, MultisampleState, Operations,
        PipelineCache, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipelineDescriptor, Sampler as GpuSampler, SamplerBindingType, SamplerDescriptor,
        ShaderStages, Texture, TextureFormat, TextureSampleType, TextureView,
    },
    renderer::{RenderContext, RenderDevice},
    view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
    Render, RenderApp, RenderSet,
};

use super::graph::{create_post_process_texture, CesiumPostProcessLabel};
use super::post_process::PostProcessConfig;

// ─── Shader handle ───────────────────────────────────────────────────────────

/// 内嵌 `ao.wgsl` shader 的唯一 handle（与 pass-through /
/// FXAA handle 区分——参见下方的碰撞测试）。
pub const AO_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(0xCE51_E2E2_A0A0_0016);

/// AO 中间缓冲的格式（单个标量 AO 复制到 RGBA）。
const AO_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

// ─── 组件 ───────────────────────────────────────────────────────────────

/// 在相机实体上启用 cesiumrust SSAO 的标记组件。
///
/// 经 `ExtractComponentPlugin` 提取到 render world。当 `enabled == false` 时
/// [`AoNode`] 提前 return（零 GPU 开销）。`enabled` 由 `ao_system`
///（post_process.rs）从 `PostProcessConfig::ambient_occlusion_enabled` 同步，
/// 对应 `fxaa_system` → `CesiumFxaa`。
#[derive(Component, Clone, Debug, ExtractComponent)]
pub struct CesiumAmbientOcclusion {
    /// 该相机上 SSAO pass 的主开关。
    pub enabled: bool,
}

impl Default for CesiumAmbientOcclusion {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// AO 节点的逐视图缓存 pipeline ID（生成 + 模糊/调制）。
#[derive(Component)]
pub struct CameraAoPipeline {
    pub generate_id: CachedRenderPipelineId,
    pub blur_id: CachedRenderPipelineId,
}

// ─── Pipeline ────────────────────────────────────────────────────────────────

/// Render-world 资源：两个 AO pass 的 bind group 布局 + 采样器。
///
/// 两个 pass 共享 WGSL group(0)。`view` uniform（binding 3）被**两个**
/// 入口点静态使用，所以它必须出现在**两个**布局中（Ryan C2 ——
/// blur 布局之前省略了它，导致 `create_render_pipeline` 布局校验
/// 失败、blur pipeline 解析为 `None`，而 [`AoNode::run`] 中的 `AND`
/// 提前 return 静默地将整个 AO 节点变为 no-op —— 像素于是
/// 等于 gate OFF，而 `pixel_diff` 报出一个假绿）：
/// - generate：bindings 0..=3（深度、法线、point 采样器、view uniform）
/// - blur/modulate：bindings 3..=6（view uniform、ao 纹理、颜色纹理、线性采样器）
#[derive(Resource)]
pub struct AoPipeline {
    pub generate_bind_group_layout: BindGroupLayout,
    pub blur_bind_group_layout: BindGroupLayout,
    /// depth + normal 前置 pass 纹理的非滤波采样器。
    pub point_sampler: GpuSampler,
    /// AO 盒模糊 + 颜色调制的滤波采样器。
    pub linear_sampler: GpuSampler,
}

impl FromWorld for AoPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();

        // 生成 pass：depth (0)、normal (1)、point sampler (2)、view uniform (3)。
        let generate_bind_group_layout = render_device.create_bind_group_layout(
            "cesium_ao_generate_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_depth_2d(),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::NonFiltering),
                    uniform_buffer::<ViewUniform>(true),
                ),
            ),
        );

        // 模糊/调制 pass：view uniform（3，与 generate 共享 —— Ryan C2）、
        // ao 纹理（4）、颜色纹理（5）、线性采样器（6）。
        let blur_bind_group_layout = render_device.create_bind_group_layout(
            "cesium_ao_blur_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::FRAGMENT,
                (
                    (3, uniform_buffer::<ViewUniform>(true)),
                    (4, texture_2d(TextureSampleType::Float { filterable: true })),
                    (5, texture_2d(TextureSampleType::Float { filterable: true })),
                    (6, sampler(SamplerBindingType::Filtering)),
                ),
            ),
        );

        let point_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_ao_point_sampler"),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_ao_linear_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            generate_bind_group_layout,
            blur_bind_group_layout,
            point_sampler,
            linear_sampler,
        }
    }
}

// ─── ViewNode ────────────────────────────────────────────────────────────────

/// SSAO `ViewNode`：半球 16 采样生成 pass → AO 中间纹理，
/// 然后一个 4×4 盒模糊 + 调制 pass 作用于颜色目标。
///
/// **逐视图实体**缓存全屏 AO 中间纹理（Ryan L3 —— 之前的
/// 单槽缓存在多于一个相机活动时会颠簸并带有一个隐式顺序依赖）。
/// Bind group 每帧重建（它们依赖逐帧的 prepass views + post-process source）；
/// 逐帧 bind-group 缓存是一个已知的性能优化，暂缓到 M11.2 xvfb e2e
///（参见 `docs/deferred.md`）。
#[derive(Default)]
pub struct AoNode {
    /// 视图实体 →（宽、高、纹理、视图）；当某个视图的视口尺寸变化时条目会重建。
    cached_ao_textures: Mutex<HashMap<Entity, (u32, u32, Texture, TextureView)>>,
}

impl ViewNode for AoNode {
    type ViewQuery = (
        Entity,
        &'static ViewTarget,
        &'static ExtractedView,
        &'static CameraAoPipeline,
        &'static CesiumAmbientOcclusion,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_entity, target, _view, pipeline_ids, ao, prepass, view_uniform_offset): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if !ao.enabled {
            return Ok(());
        }

        // SSAO 需要 depth + normal 两个 prepass 输入。
        let (Some(depth_view), Some(normal_view)) = (prepass.depth_view(), prepass.normal_view())
        else {
            return Ok(());
        };

        let pipeline_cache = world.resource::<PipelineCache>();
        let ao_pipeline = world.resource::<AoPipeline>();
        let view_uniforms = world.resource::<ViewUniforms>();

        let (Some(generate_pipeline), Some(blur_pipeline)) = (
            pipeline_cache.get_render_pipeline(pipeline_ids.generate_id),
            pipeline_cache.get_render_pipeline(pipeline_ids.blur_id),
        ) else {
            return Ok(());
        };

        let Some(view_uniform_binding) = view_uniforms.uniforms.binding() else {
            return Ok(());
        };

        // 克隆 RenderDevice（Arc 支撑，廉价），以免在后续可变的
        // `render_context.command_encoder()` 调用间持有对 `render_context` 的
        // 不可变借用。
        let render_device = render_context.render_device().clone();

        // ── AO 中间纹理（视口尺寸，逐视图实体缓存）──
        // Ryan L3：以视图实体为键，所以多个活动相机从不颠簸一个共享槽位
        //（之前的单槽 `Mutex<Option<..>>` 也带有一个隐式的跨视图顺序
        // 依赖）。仅当某个视图的视口尺寸变化时条目才重建。
        let width = prepass.size.width.max(1);
        let height = prepass.size.height.max(1);
        let mut cache = self.cached_ao_textures.lock().unwrap();
        let needs_recreate = cache
            .get(&view_entity)
            .map(|(w, h, _, _)| (*w, *h) != (width, height))
            .unwrap_or(true);
        if needs_recreate {
            let (texture, view) = create_post_process_texture(
                &render_device,
                "cesium_ao_texture",
                width,
                height,
                AO_TEXTURE_FORMAT,
            );
            cache.insert(view_entity, (width, height, texture, view));
        }
        let ao_texture_view = &cache.get(&view_entity).unwrap().3;

        // ── Pass 1：将 SSAO 生成进 AO 中间纹理 ──
        let generate_bind_group = render_device.create_bind_group(
            Some("cesium_ao_generate_bind_group"),
            &ao_pipeline.generate_bind_group_layout,
            &BindGroupEntries::sequential((
                depth_view,
                normal_view,
                &ao_pipeline.point_sampler,
                view_uniform_binding.clone(),
            )),
        );

        let generate_pass = RenderPassDescriptor {
            label: Some("cesium_ao_generate_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: ao_texture_view,
                resolve_target: None,
                // 全屏三角形覆盖每个 texel，所以清除值总是被覆盖；
                // `Operations::default()`（clear + store）已足够，并与 FXAA /
                // pass-through 节点一致。
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        {
            let mut pass = render_context
                .command_encoder()
                .begin_render_pass(&generate_pass);
            pass.set_pipeline(generate_pipeline);
            pass.set_bind_group(0, &generate_bind_group, &[view_uniform_offset.offset]);
            pass.draw(0..3, 0..1); // 全屏三角形
        }

        // ── Pass 2：4×4 盒模糊 + 调制到颜色目标上 ──
        let post_process = target.post_process_write();
        let source = post_process.source;
        let destination = post_process.destination;

        let blur_bind_group = render_device.create_bind_group(
            Some("cesium_ao_blur_bind_group"),
            &ao_pipeline.blur_bind_group_layout,
            &BindGroupEntries::with_indices((
                (3, view_uniform_binding.clone()),
                (4, ao_texture_view),
                (5, source),
                (6, &ao_pipeline.linear_sampler),
            )),
        );

        let blur_pass = RenderPassDescriptor {
            label: Some("cesium_ao_blur_modulate_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        {
            let mut pass = render_context
                .command_encoder()
                .begin_render_pass(&blur_pass);
            pass.set_pipeline(blur_pipeline);
            // Ryan C2：blur 布局现在绑定带动态偏移的 view uniform
            //（binding 3，与 generate 共享），所以必须在此提供它的偏移
            // —— 一个 `&[]` 偏移列表会使动态缓冲校验失败。
            pass.set_bind_group(0, &blur_bind_group, &[view_uniform_offset.offset]);
            pass.draw(0..3, 0..1); // 全屏三角形
        }

        Ok(())
    }
}

// ─── 渲染系统 ──────────────────────────────────────────────────────────

/// 为每个启用的相机视图准备两个特化的 AO render pipeline。
/// 运行在 `Render`、`RenderSet::Prepare`。
pub fn prepare_ao_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    ao_pipeline: Res<AoPipeline>,
    views: Query<(Entity, &ExtractedView, &CesiumAmbientOcclusion)>,
) {
    for (entity, view, ao) in &views {
        if !ao.enabled {
            continue;
        }

        // 目标格式与 post-process 目标匹配（HDR 还是 LDR）。
        let destination_format = if view.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::Rgba8UnormSrgb
        };

        let generate_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("cesium_ao_generate_pipeline".into()),
            layout: vec![ao_pipeline.generate_bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: AO_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "fragment_generate".into(),
                targets: vec![Some(ColorTargetState {
                    format: AO_TEXTURE_FORMAT,
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

        let blur_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("cesium_ao_blur_pipeline".into()),
            layout: vec![ao_pipeline.blur_bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: AO_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "fragment_blur_modulate".into(),
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

        commands.entity(entity).insert(CameraAoPipeline {
            generate_id,
            blur_id,
        });
    }
}

// ─── 主 world 系统 ──────────────────────────────────────────────────────

/// 确保每个 `Camera3d` 携带 SSAO 标记 + 它所需的 depth/normal 前置 pass
/// 组件。**适配层**启用 `DepthPrepass` + `NormalPrepass`
///（两者先前全仓零使用），以便应用层相机
/// bundle（`orbit_camera.rs`）无需改动（M5-E2 红线）。
///
/// 运行在 `Update`。当 AO 禁用时，前置 pass 标记被移除，以免产生
/// 额外的几何 pass —— **除了**同位的 clipping / IBL 效果仍需要它们的
/// 情形（FIX-AO-PREPASS，见下）。仅在后处理门控为 ON 时才加入调度，
/// 所以 gate OFF ⇒ 无前置 pass ⇒ v0
/// baseline 像素中性。
///
/// FIX-AO-PREPASS：`DepthPrepass` / `NormalPrepass` 与 clipping、IBL 效果
/// **共享**，它们各自的 `setup_*_prepass` 系统是只插入（从不移除），
/// 并在同一个 `Update` 中无跨系统排序地运行。因此这里旧的无条件
/// `remove` 会抓走 clipping / IBL 相机刚被赋予的标记，结果取决于调度器
/// 顺序（而 AO+clipping 相机的 `NormalPrepass` 根本没有其他插入者）。
/// 现在移除由每个相机的*持久驱动组件*守考——
/// `CesiumClippingPlanes`（需要 `DepthPrepass`）与 `CesiumIbl`（需要两者）——
/// 所以决策从不读取瞬态标记本身，与顺序无关。
#[allow(clippy::type_complexity)] // Bevy 系统参数：(Entity, Option<&C>, Option<&I>) 过滤器
pub fn setup_ao_prepass(
    mut commands: Commands,
    config: Res<PostProcessConfig>,
    cameras: Query<
        (
            Entity,
            Option<&super::clipping_planes::CesiumClippingPlanes>,
            Option<&super::ibl::CesiumIbl>,
        ),
        With<Camera3d>,
    >,
    ao_markers: Query<&CesiumAmbientOcclusion>,
) {
    for (entity, clipping, ibl) in &cameras {
        let mut ecmd = commands.entity(entity);
        let enabled = match ao_markers.get(entity) {
            Ok(marker) => marker.enabled,
            Err(_) => {
                // 首次目击：从配置种子化标记，以便 AO 子门控
                //（Daniel M2 — `PostProcessConfig::ambient_occlusion_enabled`）决定
                // AO 是否开启；`ao_system` 在此后每帧调和它。
                ecmd.insert(CesiumAmbientOcclusion {
                    enabled: config.ambient_occlusion_enabled,
                });
                config.ambient_occlusion_enabled
            }
        };

        if enabled {
            ecmd.insert(DepthPrepass);
            ecmd.insert(NormalPrepass);
        } else {
            // FIX-AO-PREPASS：仅当*本*相机没有其他活动消费者时才 drop 一个共享标记。
            // `NormalPrepass` 由 AO + IBL 需求；
            // `DepthPrepass` 由 AO + clipping + IBL。驱动组件的存在
            // 就是需求信号（一个禁用但存在的效果被视为
            // 消费者，这是保守的：它只能避免过度热心的
            // 移除，从不破坏另一个效果）。
            if ibl.is_none() {
                ecmd.remove::<NormalPrepass>();
            }
            if clipping.is_none() && ibl.is_none() {
                ecmd.remove::<DepthPrepass>();
            }
        }
    }
}

#[cfg(test)]
mod ao_prepass_tests {
    use super::*;
    use crate::effects::clipping_planes::CesiumClippingPlanes;
    use crate::effects::ibl::CesiumIbl;
    use cesium_effects::ibl::{IblMaterial, ImageBasedLighting};

    fn app_with(ao_enabled: bool) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(PostProcessConfig {
            ambient_occlusion_enabled: ao_enabled,
            ..Default::default()
        });
        app.add_systems(Update, setup_ao_prepass);
        app
    }

    fn has<T: Component>(app: &App, e: Entity) -> bool {
        app.world().get::<T>(e).is_some()
    }

    /// FIX-AO-PREPASS：AO 关且无其他消费者 ⇒ AO 拥有这些标记并
    /// 将两者都移除。
    #[test]
    fn ao_off_yanks_prepass_when_no_other_consumer() {
        let mut app = app_with(false);
        let cam = app
            .world_mut()
            .spawn((Camera3d::default(), DepthPrepass, NormalPrepass))
            .id();
        app.update();
        assert!(!has::<DepthPrepass>(&app, cam), "no consumer ⇒ depth removed");
        assert!(!has::<NormalPrepass>(&app, cam), "no consumer ⇒ normal removed");
    }

    /// FIX-AO-PREPASS：在一个同时驱动 clipping / IBL 的相机上关闭 AO 时，绝不可
    /// 拽掉那些 effect 插入（并依赖）的共享标记。
    #[test]
    fn ao_off_preserves_shared_prepass_for_clipping_and_ibl() {
        let mut app = app_with(false);
        let clip_cam = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                CesiumClippingPlanes::default(),
                DepthPrepass,
                NormalPrepass,
            ))
            .id();
        let ibl_cam = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                CesiumIbl::new(ImageBasedLighting::default(), IblMaterial::default()),
                DepthPrepass,
                NormalPrepass,
            ))
            .id();
        app.update();
        // Clipping 保留 DepthPrepass；NormalPrepass 此处无其他消费者 → 被拽掉。
        assert!(has::<DepthPrepass>(&app, clip_cam), "clipping must retain depth prepass");
        assert!(!has::<NormalPrepass>(&app, clip_cam), "no ibl ⇒ normal prepass may be yanked");
        // IBL 需要两者 → 都不会被拽掉。
        assert!(has::<DepthPrepass>(&app, ibl_cam), "ibl must retain depth prepass");
        assert!(has::<NormalPrepass>(&app, ibl_cam), "ibl must retain normal prepass");
    }

    /// AO 开启 ⇒ AO 插入两个标记（不变的正常路径）。
    #[test]
    fn ao_on_inserts_depth_and_normal_prepass() {
        let mut app = app_with(true);
        let cam = app.world_mut().spawn(Camera3d::default()).id();
        app.update();
        assert!(has::<DepthPrepass>(&app, cam), "AO on ⇒ depth prepass inserted");
        assert!(has::<NormalPrepass>(&app, cam), "AO on ⇒ normal prepass inserted");
    }
}

// ─── 注册 ────────────────────────────────────────────────────────────

/// 将 SSAO 节点注册进 `RenderApp`（shader + extract + 节点 + 系统），
/// 并在主 world 的相机上启用 depth/normal 前置 pass。
///
/// 当后处理门控为 ON 时由 `register_render_graph`（graph.rs）调用。
/// 注册节点**但从不创建图边** —— graph.rs 中的统一
/// 线性链拥有这些边（`Core3d` 中无菱形）。对应
/// [`super::fxaa::register_fxaa_node`]。
#[deprecated = "DEV-029 / FIX-REG-FACADE: call `register_ao_node_main_world` from `Plugin::build` and `register_ao_node_render_world` from `Plugin::finish`; this facade runs the finish half against a possibly device-less render world."]
pub fn register_ao_node(app: &mut App) {
    register_ao_node_main_world(app);
    // 无头的 `MinimalPlugins` 没有 `RenderApp` —— 优雅降级。
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        register_ao_node_render_world(render_app);
    }
}

/// [`register_ao_node`] 的 `Plugin::build` 时半边：所有住在**主** world 的东西
///（WGSL shader 资源 + `ExtractComponentPlugin` + 附加 `DepthPrepass`/`NormalPrepass`
/// 的 `setup_ao_prepass` 系统）。
///
/// 由 task #81 拆分——参见 `docs/deviations.md#dev-029`。`AoPipeline` 的
/// `FromWorld` 读取 `RenderDevice`，而 Bevy 仅在 `RenderPlugin::finish` 中把它插入
/// render world，所以下方的 render-world 半边必须从插件的 `finish` 运行，
/// 绝不从其 `build` 运行。
pub fn register_ao_node_main_world(app: &mut App) {
    // 注册 AO WGSL shader（经 shader_registry 无头安好）。
    crate::shader_registry::try_load_internal_shader(
        app,
        AO_SHADER_HANDLE,
        include_str!("../../shaders/ao.wgsl"),
        "shaders/ao.wgsl",
    );

    // ExtractComponentPlugin：每帧 主 → render world（ExtractSchedule）。
    app.add_plugins(ExtractComponentPlugin::<CesiumAmbientOcclusion>::default());

    // 主 world：为相机附加 DepthPrepass + NormalPrepass（SSAO 输入）。
    // 在此添加（仅门控 ON），使应用层相机 bundle 保持不变。
    app.add_systems(Update, setup_ao_prepass);
}

/// [`register_ao_node`] 的 `Plugin::finish` 时半边：render-world pipeline
/// 资源 + `Core3d` 节点。
pub fn register_ao_node_render_world(render_app: &mut bevy::app::SubApp) {
    // FIX-REG-FACADE (DEV-029)：当 `RenderDevice` 缺失时降级为 no-op
    //（finish 半边从 `build` 到达，或一个裸 render world）。参见
    // `crate::effects::render_world_missing_device`。
    if crate::effects::render_world_missing_device(render_app) {
        return;
    }

    render_app
        .init_resource::<AoPipeline>()
        .add_systems(Render, prepare_ao_pipelines.in_set(RenderSet::Prepare))
        .add_render_graph_node::<ViewNodeRunner<AoNode>>(
            Core3d,
            CesiumPostProcessLabel::AmbientOcclusion,
        );
    // 注意：边由 `register_render_graph`（统一线性链）创建。
}

// ─── 测试 ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ao_component_default_enabled() {
        assert!(CesiumAmbientOcclusion::default().enabled);
    }

    #[test]
    fn ao_shader_handle_unique() {
        // 与 pass-through / FXAA handle 无碰撞。
        assert_ne!(AO_SHADER_HANDLE, super::super::graph::PASS_THROUGH_SHADER_HANDLE);
        assert_ne!(AO_SHADER_HANDLE, super::super::fxaa::FXAA_SHADER_HANDLE);
    }

    #[test]
    fn ao_headless_graceful() {
        // 无 RenderApp（无头）→ register_ao_node 绝不可 panic。
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        #[allow(deprecated)]
        register_ao_node(&mut app);
    }

    /// 适配层前置 pass 启用：一个 `Camera3d` 获得 SSAO 标记 +
    /// `DepthPrepass` + `NormalPrepass`（先前零使用的输入），并在 AO 禁用时
    /// 失去这些前置 pass 标记。因它是一个纯主 world ECS 系统而无头可测——
    /// 无需 GPU / render graph。
    #[test]
    fn setup_ao_prepass_attaches_and_removes_prepass() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // AO 子门控 ON（Daniel M2）—— `setup_ao_prepass` 现在从 config 种下标记，
        // 所以对本测试而言该资源必须存在并启用 AO。
        app.insert_resource(PostProcessConfig {
            ambient_occlusion_enabled: true,
            ..Default::default()
        });
        app.add_systems(Update, setup_ao_prepass);

        let cam = app.world_mut().spawn(Camera3d::default()).id();

        // Frame 1: marker absent → seeded enabled from config + prepass attached.
        app.update();
        assert!(app.world().get::<CesiumAmbientOcclusion>(cam).is_some());
        assert!(app.world().get::<DepthPrepass>(cam).is_some());
        assert!(app.world().get::<NormalPrepass>(cam).is_some());

        // 禁用 AO（对应 ao_system 同步 config=false）→ 前置 pass 移除。
        app.world_mut()
            .get_mut::<CesiumAmbientOcclusion>(cam)
            .unwrap()
            .enabled = false;
        app.update();
        assert!(app.world().get::<DepthPrepass>(cam).is_none());
        assert!(app.world().get::<NormalPrepass>(cam).is_none());
    }

    /// Daniel M2：当 AO 子门控 OFF（config 默认）时，首次目击将标记
    /// 种为**禁用**且不附加任何前置 pass——所以门控 OFF 不产生
    /// 任何额外几何 pass，v0 基线保持像素中性。
    #[test]
    fn setup_ao_prepass_off_seeds_disabled_no_prepass() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<PostProcessConfig>(); // ambient_occlusion_enabled = false
        app.add_systems(Update, setup_ao_prepass);

        let cam = app.world_mut().spawn(Camera3d::default()).id();
        app.update();
        assert!(
            !app.world().get::<CesiumAmbientOcclusion>(cam).unwrap().enabled,
            "AO sub-gate off ⇒ marker seeded disabled"
        );
        assert!(app.world().get::<DepthPrepass>(cam).is_none());
        assert!(app.world().get::<NormalPrepass>(cam).is_none());
    }

    // ─── Ryan C1 防线：无头 naga 解析 + 校验 + 布局一致性 ──

    /// naga 没有预处理器，所以 `ao.wgsl` 中的两个 `#import` 被替换为
    /// 精确声明 shader 所读取字段的 struct stub：
    /// `FullscreenVertexOutput.{position, uv}` 和
    /// `View.{view_from_clip, view_from_world, clip_from_view, viewport}`。
    /// `view` **binding** 本身由真实 shader 文本声明（group 0,
    /// binding 3），所以此处刻意不为其提供 stub。其余全是
    /// 实际 shader 源码，因此这校验了真实的 SSAO 代码。
    const AO_WGSL_IMPORT_STUBS: &str = "\
struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct View {
    view_from_clip: mat4x4<f32>,
    view_from_world: mat4x4<f32>,
    clip_from_view: mat4x4<f32>,
    viewport: vec4<f32>,
}
";

    fn ao_stubbed_wgsl() -> String {
        let mut source = String::from(AO_WGSL_IMPORT_STUBS);
        for line in include_str!("../../shaders/ao.wgsl").lines() {
            if line.starts_with("#import") {
                continue;
            }
            source.push_str(line);
            source.push('\n');
        }
        source
    }

    /// SSAO shader 为真的最强无头证据：它由 **naga** 解析并做类型检查，
    /// 即 `bevy_render` 在 GPU 路径上编译它所用的同一个 WGSL 前端。
    /// 字面意义的设备回读仍需 xvfb
    ///（`.github/workflows/cesiumrust-e2e.yml`，M11.2）。
    #[test]
    fn ao_wgsl_parses_and_type_checks_under_naga() {
        let source = ao_stubbed_wgsl();
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("ao.wgsl does not parse:\n{}", error.emit_to_string(&source))
        });
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("ao.wgsl does not validate");

        let mut entry_points = module
            .entry_points
            .iter()
            .map(|entry| (entry.name.as_str(), entry.stage))
            .collect::<Vec<_>>();
        entry_points.sort_by_key(|(name, _)| *name);
        assert_eq!(
            entry_points,
            vec![
                ("fragment_blur_modulate", naga::ShaderStage::Fragment),
                ("fragment_generate", naga::ShaderStage::Fragment),
            ],
            "ao.wgsl must expose exactly the two fragment entry points"
        );
    }

    /// 遍历一个入口点的调用图，收集它静态引用的每个 `(group, binding)`
    /// —— `GlobalVariable` 表达式，并递归穿过
    /// 局部函数的 `CallResult` 表达式（例如 `view` 仅在
    /// `reconstruct_view_position` / `load_view_normal` 内部被触碰）。
    fn entry_used_bindings(
        module: &naga::Module,
        entry: &naga::EntryPoint,
    ) -> std::collections::BTreeSet<(u32, u32)> {
        let mut out = std::collections::BTreeSet::new();
        let mut visited = std::collections::HashSet::new();
        let mut stack: Vec<&naga::Function> = vec![&entry.function];
        while let Some(func) = stack.pop() {
            for (_, expr) in func.expressions.iter() {
                match *expr {
                    naga::Expression::GlobalVariable(handle) => {
                        if let Some(binding) = &module.global_variables[handle].binding {
                            out.insert((binding.group, binding.binding));
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
        out
    }

    /// Ryan C1 防线（在测试时捕获 **C2** 类 bug）：入口点静态使用的每个
    /// binding 都必须出现在对应的 Rust `BindGroupLayout` 中。blur 入口使用共享的
    /// `view` uniform（group 0, binding 3）；倘若 Rust blur 布局再次省略它，
    /// `create_render_pipeline` 会失败，blur pipeline 会解析为 `None`，整个 AO 节点
    /// 会静默 no-op —— 在 `pixel_diff` 中报出假绿（与 gate-OFF 相等的像素）。
    #[test]
    fn ao_wgsl_entry_bindings_are_covered_by_the_rust_layouts() {
        let source = ao_stubbed_wgsl();
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("ao.wgsl does not parse:\n{}", error.emit_to_string(&source))
        });

        // `AoPipeline::from_world` 的镜像（group 0 binding 索引）。
        let generate_layout: std::collections::BTreeSet<(u32, u32)> =
            [(0, 0), (0, 1), (0, 2), (0, 3)].into_iter().collect();
        let blur_layout: std::collections::BTreeSet<(u32, u32)> =
            [(0, 3), (0, 4), (0, 5), (0, 6)].into_iter().collect();

        for entry in &module.entry_points {
            let used = entry_used_bindings(&module, entry);
            let layout = match entry.name.as_str() {
                "fragment_generate" => &generate_layout,
                "fragment_blur_modulate" => &blur_layout,
                other => panic!("unexpected ao.wgsl entry point `{other}`"),
            };
            let missing: Vec<(u32, u32)> = used.difference(layout).copied().collect();
            assert!(
                missing.is_empty(),
                "ao.wgsl entry `{}` statically uses bindings {missing:?} that are absent from its \
                 Rust BindGroupLayout (C2 regression: create_render_pipeline would fail and AO \
                 would silently no-op)",
                entry.name
            );
        }
    }
}
