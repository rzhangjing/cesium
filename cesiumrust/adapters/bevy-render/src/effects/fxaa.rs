//! M5-E1：FXAA 抗锯齿 ViewNode（仅质量预设 12）。
//!
//! 使用来自 M5-E0（`graph.rs`）的 RenderGraph
//! 基础设施实现 cesiumrust 的 FXAA 后处理节点。该节点插入在
//! `Core3d` 的 `Node3d::Tonemapping` 与 `Node3d::EndMainPassPostProcessing` 之间。
//!
//! # 蓝图
//! - `cesium-rs/crates/cesium-shaders/shaders/FXAA3_11.glsl`（651 行，preset 12 = L102-108）
//! - `packages/engine/Source/Shaders/PostProcessStages/FXAA.glsl`（21 行，接口）
//! - `packages/engine/Source/Scene/PostProcessStageLibrary.js` L611 `createFXAAStage`
//!
//! # 偏差
//! GLSL FXAA 3.11 的 WGSL 重写；仅实现质量预设 12（计划 L155）。
//! 参见 `docs/deviations.md#dev-017`。

use std::sync::Mutex;

use bevy::core_pipeline::{
    core_3d::graph::Core3d,
    fullscreen_vertex_shader::fullscreen_shader_vertex_state,
};
use bevy::ecs::query::QueryItem;
use bevy::prelude::*;
use bevy::render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::{
        NodeRunError, RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner,
    },
    render_resource::{
        binding_types::{sampler, texture_2d},
        BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, FilterMode, FragmentState,
        MultisampleState, Operations, PipelineCache, PrimitiveState, RenderPassColorAttachment,
        RenderPassDescriptor, RenderPipelineDescriptor, Sampler as GpuSampler, SamplerBindingType,
        SamplerDescriptor, ShaderStages, SpecializedRenderPipeline, SpecializedRenderPipelines,
        TextureFormat, TextureSampleType, TextureViewId,
    },
    renderer::RenderContext,
    view::{ExtractedView, ViewTarget},
    Render, RenderApp, RenderSet,
};

use super::graph::CesiumPostProcessLabel;

// ─── Shader handle ───────────────────────────────────────────────────────────

/// 内嵌 `fxaa.wgsl` shader 的唯一 handle。
pub const FXAA_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0xCE51_E1E1_F4AA_0012);

// ─── 组件 ───────────────────────────────────────────────────────────────

/// 在相机实体上启用 cesiumrust FXAA 的标记组件。
///
/// 通过 `ExtractComponentPlugin` 提取到 render world。当 `enabled == false` 时
/// `FxaaNode` 会提前 return（零 GPU 开销）。
#[derive(Component, Clone, Debug, ExtractComponent)]
pub struct CesiumFxaa {
    /// 该相机上 FXAA pass 的主开关。
    pub enabled: bool,
}

impl Default for CesiumFxaa {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// FXAA 节点的逐视图缓存 pipeline ID。
#[derive(Component)]
pub struct CameraFxaaPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

// ─── Pipeline ────────────────────────────────────────────────────────────────

/// Render-world 资源：FXAA 的 bind group 布局 + 采样器。
#[derive(Resource)]
pub struct FxaaPipeline {
    pub texture_bind_group_layout: BindGroupLayout,
    pub sampler: GpuSampler,
}

impl FromWorld for FxaaPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<bevy::render::renderer::RenderDevice>();

        let texture_bind_group_layout = render_device.create_bind_group_layout(
            "cesium_fxaa_texture_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_fxaa_sampler"),
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

/// 特化键：仅纹理格式（preset 12 硬编码在 WGSL 中）。
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct FxaaPipelineKey {
    pub texture_format: TextureFormat,
}

impl SpecializedRenderPipeline for FxaaPipeline {
    type Key = FxaaPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("cesium_fxaa_pipeline".into()),
            layout: vec![self.texture_bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: FXAA_SHADER_HANDLE,
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

/// FXAA `ViewNode` —— 在 tonemapping 之后、EndMainPassPostProcessing 之前运行。
///
/// 模式对应 `bevy_core_pipeline::fxaa::node::FxaaNode`（86 行），但使用
/// cesiumrust 自己的 WGSL shader（质量预设 12）。
#[derive(Default)]
pub struct FxaaNode {
    cached_texture_bind_group: Mutex<Option<(TextureViewId, BindGroup)>>,
}

impl ViewNode for FxaaNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static CameraFxaaPipeline,
        &'static CesiumFxaa,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (target, pipeline_handle, fxaa): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if !fxaa.enabled {
            return Ok(());
        }

        let pipeline_cache = world.resource::<PipelineCache>();
        let fxaa_pipeline = world.resource::<FxaaPipeline>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_handle.pipeline_id) else {
            return Ok(());
        };

        let post_process = target.post_process_write();
        let source = post_process.source;
        let destination = post_process.destination;

        let mut cached = self.cached_texture_bind_group.lock().unwrap();
        let bind_group = match &mut *cached {
            Some((id, bg)) if source.id() == *id => bg,
            slot => {
                let bg = render_context.render_device().create_bind_group(
                    Some("cesium_fxaa_bind_group"),
                    &fxaa_pipeline.texture_bind_group_layout,
                    &BindGroupEntries::sequential((source, &fxaa_pipeline.sampler)),
                );
                let (_, bg) = slot.insert((source.id(), bg));
                bg
            }
        };

        let pass_descriptor = RenderPassDescriptor {
            label: Some("cesium_fxaa_pass"),
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

/// 逐相机视图准备特化的 FXAA pipeline。
/// 运行在 `Render` 调度、`RenderSet::Prepare`。
pub fn prepare_fxaa_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<FxaaPipeline>>,
    fxaa_pipeline: Res<FxaaPipeline>,
    views: Query<(Entity, &ExtractedView, &CesiumFxaa)>,
) {
    for (entity, view, fxaa) in &views {
        if !fxaa.enabled {
            continue;
        }

        let texture_format = if view.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::Rgba8UnormSrgb
        };

        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &fxaa_pipeline,
            FxaaPipelineKey { texture_format },
        );

        commands.entity(entity).insert(CameraFxaaPipeline { pipeline_id });
    }
}

// ─── 注册 ────────────────────────────────────────────────────────────

/// 将 FXAA 节点注册进 `RenderApp`（shader + extract + node + 系统）。
///
/// 当后处理门控为 ON 时，从 `register_render_graph`（graph.rs）调用。本
/// 函数注册节点**但不创建 graph 边** ——
/// `register_render_graph` 拥有单一线性链（Daniel H2，上游
/// CesiumJS 对齐）`EndMainPass → PassThrough → AmbientOcclusion → Tonemapping →
/// Fxaa → EndMainPassPostProcessing`，所以 cesium 节点从不在
/// `Core3d` 中形成菱形。
///
/// 位置理由：FXAA 在 tonemapping **之后**（HDR 线性 → LDR 完成）、上采样
/// **之前**运行，与 CesiumJS 中 FXAA 作用于最终
/// LDR 图像的做法一致。参见 `docs/deviations.md#dev-017`。
#[deprecated = "DEV-029 / FIX-REG-FACADE: call `register_fxaa_node_main_world` from `Plugin::build` and `register_fxaa_node_render_world` from `Plugin::finish`; this facade runs the finish half against a possibly device-less render world."]
pub fn register_fxaa_node(app: &mut App) {
    register_fxaa_node_main_world(app);
    // 无头 `MinimalPlugins` 没有 `RenderApp` —— 优雅降级。
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        register_fxaa_node_render_world(render_app);
    }
}

/// [`register_fxaa_node`] 的 `Plugin::build` 时半边：生住在
/// **主** world 中的一切（WGSL shader 资产 + `ExtractComponentPlugin`）。
///
/// 由任务 #81 拆出 —— 参见 `docs/deviations.md#dev-029`。`FxaaPipeline` 的
/// `FromWorld` 读取 `RenderDevice`，而 Bevy 只在 `RenderPlugin::finish`
///（`bevy_render/src/lib.rs` L399-430）中才将 `RenderDevice` 插入 render
/// world，所以从任何插件的 `build` 调用 [`register_fxaa_node_render_world`] 会
/// panic，报错 "RenderDevice does not exist in the World"。因此调用者必须
/// 分别从 `build` 和 `finish` 运行两个半边（Bevy 自身使用的
/// 模式：`bevy_pbr/src/ssao/mod.rs` L54 `build` / L80 `finish`）。
pub fn register_fxaa_node_main_world(app: &mut App) {
    // 注册 FXAA WGSL shader（无头安好）。
    crate::shader_registry::try_load_internal_shader(
        app,
        FXAA_SHADER_HANDLE,
        include_str!("../../shaders/fxaa.wgsl"),
        "shaders/fxaa.wgsl",
    );

    // CesiumFxaa 的 ExtractComponentPlugin（ExtractSchedule：主→render world）。
    app.add_plugins(ExtractComponentPlugin::<CesiumFxaa>::default());
}

/// [`register_fxaa_node`] 的 `Plugin::finish` 时半边：render-world
/// pipeline 资源 + `Core3d` 节点。
pub fn register_fxaa_node_render_world(render_app: &mut bevy::app::SubApp) {
    // FIX-REG-FACADE（DEV-029）：当 `RenderDevice` 缺失时降级为 no-op
    //（finish 半边从 `build` 到达，或一个裸 render world）。参见
    // `crate::effects::render_world_missing_device`。
    if crate::effects::render_world_missing_device(render_app) {
        return;
    }

    render_app
        .init_resource::<FxaaPipeline>()
        .init_resource::<SpecializedRenderPipelines<FxaaPipeline>>()
        .add_systems(Render, prepare_fxaa_pipelines.in_set(RenderSet::Prepare))
        .add_render_graph_node::<ViewNodeRunner<FxaaNode>>(
            Core3d,
            CesiumPostProcessLabel::Fxaa,
        );
    // 注意：边由 `register_render_graph` 创建（统一线性链）。
}

// ─── 测试 ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fxaa_component_default_enabled() {
        let f = CesiumFxaa::default();
        assert!(f.enabled);
    }

    #[test]
    fn fxaa_headless_graceful() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // 不应 panic（没有 RenderApp）。
        #[allow(deprecated)]
        register_fxaa_node(&mut app);
    }

    #[test]
    fn fxaa_shader_handle_unique() {
        // 确保不与 pass-through handle 冲突。
        assert_ne!(
            FXAA_SHADER_HANDLE,
            super::super::graph::PASS_THROUGH_SHADER_HANDLE
        );
    }

    // ─── Ryan C1 防线：无头 naga 解析 + 校验 + 布局对齐 ──

    /// naga 没有预处理器，所以 `fxaa.wgsl` 的那一个 `#import`
    ///（`FullscreenVertexOutput`）被一个 stub 取代，该 stub 声明片元
    /// 读取的字段（`position`）。其余全是真实的 shader 源码。
    const FXAA_WGSL_IMPORT_STUBS: &str = "\
struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
";

    fn fxaa_stubbed_wgsl() -> String {
        let mut source = String::from(FXAA_WGSL_IMPORT_STUBS);
        for line in include_str!("../../shaders/fxaa.wgsl").lines() {
            if line.starts_with("#import") {
                continue;
            }
            source.push_str(line);
            source.push('\n');
        }
        source
    }

    /// FXAA shader 真实性的无头证明：由 **naga** 解析 + 类型检查
    ///（与 `bevy_render` 在 GPU 路径上编译它所用的 WGSL 前端相同）。
    /// 设备回读仍需 xvfb（`.github/workflows/cesiumrust-e2e.yml`）。
    #[test]
    fn fxaa_wgsl_parses_and_type_checks_under_naga() {
        let source = fxaa_stubbed_wgsl();
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("fxaa.wgsl does not parse:\n{}", error.emit_to_string(&source))
        });
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("fxaa.wgsl does not validate");

        let entry_points = module
            .entry_points
            .iter()
            .map(|entry| (entry.name.as_str(), entry.stage))
            .collect::<Vec<_>>();
        assert_eq!(
            entry_points,
            vec![("fragment", naga::ShaderStage::Fragment)],
            "fxaa.wgsl must expose exactly one fragment entry point"
        );
    }

    /// Ryan C1（捕捉 **C2** 类 bug）：`fragment` 入口静态使用的每一个 binding
    /// 都必须存在于 Rust 的 `FxaaPipeline` 布局中
    ///（group 0：binding 0 = 屏幕纹理，binding 1 = 采样器）。以防一种
    /// 静默的 pipeline-build 失败使 FXAA 变为 no-op，而 `pixel_diff`
    /// 却报出一个假绿。
    #[test]
    fn fxaa_wgsl_entry_bindings_are_covered_by_the_rust_layout() {
        let source = fxaa_stubbed_wgsl();
        let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
            panic!("fxaa.wgsl does not parse:\n{}", error.emit_to_string(&source))
        });

        let layout: std::collections::BTreeSet<(u32, u32)> =
            [(0, 0), (0, 1)].into_iter().collect();

        let entry = module
            .entry_points
            .iter()
            .find(|e| e.name == "fragment")
            .expect("fxaa.wgsl must have a `fragment` entry point");

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
            "fxaa.wgsl `fragment` statically uses bindings {missing:?} absent from FxaaPipeline's \
             layout (C2-class regression: the pipeline would fail to build and FXAA would no-op)"
        );
    }
}
