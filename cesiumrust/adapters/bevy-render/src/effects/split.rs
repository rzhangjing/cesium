//! M6.1：分屏（`Splitter` / `SplitDirection`）基础设施。
//!
//! FIX-SPLIT（Phase 3）：分屏脚手架原本寄生于
//! [`super::oit`]（迁移前树中的 `oit.rs:43-75`），因为 M6.1 是
//! “最后合并”的能力，借用了 OIT 的插件槽位。本模块将其解耦：
//! `SplitConfig` 资源、`SplitDragEvent`，以及
//! `split_direction_system`（CPU 侧分隔线拖拽交互）现在住在这里，
//! 并配有
//! 一个绘制可见分隔线的真正屏幕空间渲染节点。
//!
//! # 节点绘制（与不绘制）什么
//! 上游 CesiumJS 通过*逐图元丢弃*来分屏：标记为
//! `SplitDirection.LEFT` 的影像 / 图元只在 `Scene.splitPosition` 左侧渲染，
//! `RIGHT` 只在右侧，所以两半展示两套不同的图层状态。
//! 那条忠实路径是一次*材质 shader* 注入
//!（`SplitterConfig::wgsl_shader_modification()`），它触及 globe / tileset
//! shader（超出本模块文件范围），并暂缓到真实 GPU 任务
//!（`docs/deviations.md#dev-034`）。
//!
//! 属于屏幕空间的是可拖拽的分隔线把手本身 —— 一条位于 `splitPosition`
//! 的细竖线。[`SplitNode`]  精确地渲染它：对已解析场景色的
//! pass-through，上层绘制一条 `split.wgsl` 叠加线。门控 OFF 时节点从不被注册且没有 `Core3d`
//! 边，所以 v0 baseline 保持位精确（PSNR = ∞）。
//!
//! # 门控（单一真相源）
//! 门控名由应用层注册表
//! `application/cesium-app/src/feature_flags.rs`（`ENV_ENABLE_SPLIT` /
//! `split_enabled()`）拥有；下方的 [`ENV_ENABLE_SPLIT`] 是一个字节一致的镜像，由
//! crate 依赖方向（`cesium-app` → `cesium-bevy-render`）强制。默认
//! OFF ⇒ `effects::graph::M6WaveARenderGraphPlugin` 提前 return 且
//! [`register_split_node`] 从不被调用。
//!
//! # 已遵守的红线
//! - 领域保持 **f64**（`split_position` 是一个 `[0, 1]` 分数）；分数→
//!   viewport-像素的转换仅在 [`SplitUniform::from_domain`]
//!   GPU 边界发生。
//! - `split.wgsl` 中无 FMA 收缩，无 swizzle 赋值。

use bevy::core_pipeline::{
    core_3d::graph::Core3d, fullscreen_vertex_shader::fullscreen_shader_vertex_state,
};
use bevy::ecs::query::QueryItem;
use bevy::prelude::*;
use bevy::render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::{
        NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
    },
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId,
        ColorTargetState, ColorWrites, FilterMode, FragmentState, MultisampleState, Operations,
        PipelineCache, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipelineDescriptor, Sampler as GpuSampler, SamplerBindingType, SamplerDescriptor,
        ShaderStages, TextureFormat, TextureSampleType, UniformBuffer,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    view::{ExtractedView, ViewTarget},
    Render, RenderApp, RenderSet,
};
use cesium_effects::split::SplitterConfig;

use super::graph::gate_from_env_value;

// ─── Shader handle ───────────────────────────────────────────────────────────

/// 内嵌 `split.wgsl` shader 的唯一 handle。选取时避开了与其他所有
/// cesium shader handle 的碰撞（由 `split_shader_handle_unique` 断言）。
pub const SPLIT_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(0xCE51_5F11_0006_00AA);

// ─── 门控 ────────────────────────────────────────────────────────────────────

/// 门控 M6.1 split 节点的环境变量。与
/// `feature_flags::ENV_ENABLE_SPLIT` 字节一致（为何本地化参见模块 doc）。
pub const ENV_ENABLE_SPLIT: &str = "CESIUM_ENABLE_SPLIT";

/// 当 split 门控启用时返回 `true`。复用单一权威的
/// truthy 解析器（`gate_from_env_value`，全 crate 的 `{1, true, yes, on}` 集）。
#[inline]
pub fn split_gate_enabled() -> bool {
    gate_from_env_value(std::env::var(ENV_ENABLE_SPLIT).ok())
}

// ─── 渲染图 label ──────────────────────────────────────────────────────

/// cesium split 节点在 `Core3d` 中的节点 label。
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct CesiumSplitLabel;

// ─── SplitConfig / SplitDragEvent / split_direction_system ───────────────────
// 从 `oit.rs` 逐字迁移（FIX-SPLIT）。这些是 CPU 侧分隔线
// 交互：一个持有拖拽状态的资源、一个携带新位置的
// 事件，以及一个在拖拽时将 `CursorMoved` 转为事件的系统。

/// 分隔线拖拽状态资源（主 world）。
#[derive(Resource, Debug, Clone)]
pub struct SplitConfig {
    pub enabled: bool,
    pub split_position: f64,
    pub dragging: bool,
    pub drag_start_x: f64,
}

impl Default for SplitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            split_position: 0.5,
            dragging: false,
            drag_start_x: 0.0,
        }
    }
}

impl SplitConfig {
    /// 从领域 [`SplitterConfig`] 值对象构建。
    pub fn from_splitter(config: &SplitterConfig) -> Self {
        Self {
            enabled: config.enabled,
            split_position: config.split_position,
            ..Default::default()
        }
    }
}

/// 在拖拽分隔线时发出，携带新的 `[0, 1]` 位置。
#[derive(Event)]
pub struct SplitDragEvent {
    pub position: f64,
}

/// 在 [`SplitConfig.dragging`] 时将 `CursorMoved` 事件转为 [`SplitDragEvent`]。
///
/// 分屏禁用时惰性，所以它从不扰动黄金路径。
pub fn split_direction_system(
    config: Res<SplitConfig>,
    _mouse_input: Res<ButtonInput<MouseButton>>,
    mut cursor_moved: EventReader<CursorMoved>,
    mut split_events: EventWriter<SplitDragEvent>,
    windows: Query<&Window>,
) {
    if !config.enabled {
        return;
    }

    for cursor in cursor_moved.read() {
        if config.dragging {
            if let Ok(window) = windows.get_single() {
                let pos = cursor.position.x as f64 / window.width() as f64;
                split_events.send(SplitDragEvent {
                    position: pos.clamp(0.0, 1.0),
                });
            }
        }
    }
}

// ─── 组件 ───────────────────────────────────────────────────────────────

/// 为一个视图携带分屏状态的组件（逐相机）。
///
/// 放在相机上（像 [`super::clouds::CesiumClouds`]）以驱动
/// 屏幕空间节点；经 `ExtractComponentPlugin` 提取到 render world。
/// 当 `enabled == false` 时节点提前 return（零 GPU 开销，像素中性）。
/// `Default` 为派生：`enabled = false`（保守）。
#[derive(Component, Clone, Debug, Default, ExtractComponent)]
pub struct CesiumSplit {
    /// 该视图上 split 节点的主开关。
    pub enabled: bool,
    /// 分隔线中心，作为视口宽度的 `[0, 1]` 分数（领域 f64）。
    pub split_position: f64,
    /// 分隔线粗细，以 PIXELS 计。
    pub line_width_px: f64,
    /// 分隔线颜色 RGBA `[0, 1]`。
    pub color: [f64; 4],
}

impl CesiumSplit {
    /// 一个便捷构造函数，在 `split_position` 处创建一个启用的分隔线。
    pub fn new(split_position: f64) -> Self {
        Self {
            enabled: true,
            split_position: split_position.clamp(0.0, 1.0),
            line_width_px: 2.0,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// 分隔线是否应当真正渲染。
    #[inline]
    pub fn is_active(&self) -> bool {
        self.enabled
    }
}

/// split 节点的逐视图缓存 pipeline ID。
#[derive(Component)]
pub struct CameraSplitPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

/// 持有打包后分隔线参数的逐视图 GPU uniform 缓冲。
#[derive(Component)]
pub struct ViewSplitUniform {
    pub buffer: UniformBuffer<SplitUniform>,
}

// ─── GPU uniform（f32 边界） ──────────────────────────────────────────────

/// GPU 面向的 split uniform。**仅 f32** —— 组件的 `[0, 1]` f64 分数
/// 和像素宽度在此收窄（[`SplitUniform::from_domain`]，红线）。
///
/// 布局必须与 `shaders/split.wgsl` 中的 `struct SplitData` 匹配（encase std140）。
/// 住在私有模块中并带 `#![allow(dead_code)]`（`clipping_planes.rs`
/// 约定 —— encase 的 `ShaderType` derive 会生成一个 helper，死代码分析会
/// 标记它，尽管每个字段都通过 `write_buffer` 上传）。
pub use split_uniform::SplitUniform;

mod split_uniform {
    #![allow(dead_code)]
    use bevy::prelude::Vec4;
    use bevy::render::render_resource::ShaderType;

    /// 匹配 `shaders/split.wgsl` 中的 `struct SplitData`（encase std140）。
    #[derive(ShaderType, Clone, Copy, Debug)]
    pub struct SplitUniform {
        /// 分隔线中心，以视口像素计（std140：偏移 0）。
        pub split_position_px: f32,
        /// 分隔线粗细，以像素计（std140：偏移 4）。
        pub line_width_px: f32,
        /// 分隔线颜色 RGBA（std140：16 字节对齐，偏移 16）。
        pub color: Vec4,
    }
}

impl Default for SplitUniform {
    fn default() -> Self {
        Self {
            split_position_px: 0.0,
            line_width_px: 0.0,
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
        }
    }
}

impl SplitUniform {
    /// 将一个 [`CesiumSplit`] 打包进 GPU uniform。`split_position` 是
    /// `[0, 1]` 领域分数；它**在此**乘以 `viewport_width_px`
    ///（唯一的 f64 → f32、分数 → 像素边界）。
    pub fn from_domain(component: &CesiumSplit, viewport_width_px: f32) -> Self {
        Self {
            split_position_px: (component.split_position * f64::from(viewport_width_px)) as f32,
            line_width_px: component.line_width_px.max(0.0) as f32,
            color: Vec4::new(
                component.color[0] as f32,
                component.color[1] as f32,
                component.color[2] as f32,
                component.color[3] as f32,
            ),
        }
    }
}

// ─── Pipeline ────────────────────────────────────────────────────────────────

/// Render-world 资源：split 节点的两个 bind group 布局 + 采样器。
///
/// - group 0：`screen_texture`（`texture_2d<f32>`，binding 0）+ 线性采样器（1）
/// - group 1：`SplitUniform`（binding 0）
#[derive(Resource)]
pub struct SplitPipeline {
    pub source_bind_group_layout: BindGroupLayout,
    pub split_bind_group_layout: BindGroupLayout,
    pub linear_sampler: GpuSampler,
}

impl FromWorld for SplitPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();

        let source_bind_group_layout = render_device.create_bind_group_layout(
            "cesium_split_source_bgl",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );

        let split_bind_group_layout = render_device.create_bind_group_layout(
            "cesium_split_uniform_bgl",
            &BindGroupLayoutEntries::single(
                ShaderStages::FRAGMENT,
                uniform_buffer::<SplitUniform>(false),
            ),
        );

        let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("cesium_split_linear_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            source_bind_group_layout,
            split_bind_group_layout,
            linear_sampler,
        }
    }
}

// ─── ViewNode ────────────────────────────────────────────────────────────────

/// Screen-space split `ViewNode` — pass-through + divider overlay line.
#[derive(Default)]
pub struct SplitNode;

impl ViewNode for SplitNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static CameraSplitPipeline,
        &'static CesiumSplit,
        &'static ViewSplitUniform,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (target, pipeline_handle, split, split_uniform): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if !split.is_active() {
            return Ok(());
        }

        let pipeline_cache = world.resource::<PipelineCache>();
        let split_pipeline = world.resource::<SplitPipeline>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_handle.pipeline_id) else {
            return Ok(());
        };
        let Some(split_binding) = split_uniform.buffer.binding() else {
            return Ok(());
        };

        let render_device = render_context.render_device().clone();
        let post_process = target.post_process_write();
        let source = post_process.source;
        let destination = post_process.destination;

        let source_bind_group = render_device.create_bind_group(
            Some("cesium_split_source_bg"),
            &split_pipeline.source_bind_group_layout,
            &BindGroupEntries::sequential((source, &split_pipeline.linear_sampler)),
        );
        let split_bind_group = render_device.create_bind_group(
            Some("cesium_split_uniform_bg"),
            &split_pipeline.split_bind_group_layout,
            &BindGroupEntries::single(split_binding),
        );

        let pass_descriptor = RenderPassDescriptor {
            label: Some("cesium_split_pass"),
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
        render_pass.set_bind_group(0, &source_bind_group, &[]);
        render_pass.set_bind_group(1, &split_bind_group, &[]);
        render_pass.draw(0..3, 0..1); // 全屏三角形

        Ok(())
    }
}

// ─── 渲染系统 ──────────────────────────────────────────────────────────

/// 为每个活动视图准备 split pipeline + 逐视图 uniform。
pub fn prepare_split(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    split_pipeline: Res<SplitPipeline>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    views: Query<(Entity, &ExtractedView, &CesiumSplit)>,
) {
    for (entity, view, split) in &views {
        if !split.is_active() {
            continue;
        }

        let destination_format = if view.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::Rgba8UnormSrgb
        };

        let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("cesium_split_pipeline".into()),
            layout: vec![
                split_pipeline.source_bind_group_layout.clone(),
                split_pipeline.split_bind_group_layout.clone(),
            ],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: SPLIT_SHADER_HANDLE,
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

        let mut buffer = UniformBuffer::from(SplitUniform::from_domain(
            split,
            view.viewport.z as f32,
        ));
        buffer.write_buffer(&render_device, &render_queue);

        commands.entity(entity).insert((
            CameraSplitPipeline { pipeline_id },
            ViewSplitUniform { buffer },
        ));
    }
}

// ─── 注册 ────────────────────────────────────────────────────────────

/// 注册 split 节点（三段式，DEV-029）。是对 main / render
/// 两半的便捷包装；无头安好（没有 `RenderApp` → render 半边被跳过）。
#[deprecated = "DEV-029 / FIX-REG-FACADE: call `register_split_node_main_world` from `Plugin::build` and `register_split_node_render_world` from `Plugin::finish`; this facade runs the finish half against a possibly device-less render world."]
pub fn register_split_node(app: &mut App) {
    register_split_node_main_world(app);
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        register_split_node_render_world(render_app);
    }
}

/// `Plugin::build` 时半边：WGSL shader + `ExtractComponentPlugin` +
/// 分隔线拖拽脚手架（资源 + 事件 + 系统）。仅主 world。
pub fn register_split_node_main_world(app: &mut App) {
    crate::shader_registry::try_load_internal_shader(
        app,
        SPLIT_SHADER_HANDLE,
        include_str!("../../shaders/split.wgsl"),
        "shaders/split.wgsl",
    );

    app.add_plugins(ExtractComponentPlugin::<CesiumSplit>::default());

    // 迁移过来的分隔线拖拽交互（原先在 `OITPlugin`）。
    app.init_resource::<SplitConfig>()
        .add_event::<SplitDragEvent>()
        .add_systems(Update, split_direction_system);
}

/// `Plugin::finish` 时半边：render-world pipeline 资源 + `Core3d` 节点。
/// 读取 `RenderDevice`（经 `SplitPipeline` 的 `FromWorld`），故为 `finish`。
pub fn register_split_node_render_world(render_app: &mut bevy::app::SubApp) {
    // FIX-REG-FACADE（DEV-029）：当 `RenderDevice` 缺失时降级为 no-op
    //（finish 半边从 `build` 到达，或一个裸 render world）。参见
    // `crate::effects::render_world_missing_device`。
    if crate::effects::render_world_missing_device(render_app) {
        return;
    }

    render_app
        .init_resource::<SplitPipeline>()
        .add_systems(Render, prepare_split.in_set(RenderSet::Prepare))
        .add_render_graph_node::<ViewNodeRunner<SplitNode>>(Core3d, CesiumSplitLabel);
    // 注意：边由 `effects::graph::wire_m6_edges`（共享 `Core3d` 链的唯一所有者）
    // 创建 —— 从不在此，所以不会形成菱形。
}

// ─── 测试 ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_component_default_disabled() {
        let c = CesiumSplit::default();
        assert!(!c.enabled);
        assert!(!c.is_active());
    }

    #[test]
    fn split_config_default_and_from_splitter() {
        let config = SplitConfig::default();
        assert!(!config.enabled);
        assert!(!config.dragging);
        assert_eq!(config.split_position, 0.5);

        let domain_cfg = SplitterConfig::new(true, 0.3);
        let config = SplitConfig::from_splitter(&domain_cfg);
        assert!(config.enabled);
        assert_eq!(config.split_position, 0.3);
    }

    #[test]
    fn split_gate_const_is_stable_and_independent() {
        assert_eq!(ENV_ENABLE_SPLIT, "CESIUM_ENABLE_SPLIT");
        // 该 const 必须是一个字节一致的大写 env 名。
        assert_eq!(ENV_ENABLE_SPLIT, ENV_ENABLE_SPLIT.to_uppercase().as_str());
        assert_ne!(ENV_ENABLE_SPLIT, "CESIUM_ENABLE_OIT");
        assert_ne!(ENV_ENABLE_SPLIT, "CESIUM_ENABLE_POSTPROCESS");
    }

    #[test]
    fn split_gate_default_off_without_env() {
        std::env::remove_var(ENV_ENABLE_SPLIT);
        assert!(!split_gate_enabled());
    }

    #[test]
    fn split_shader_handle_unique() {
        assert_ne!(
            SPLIT_SHADER_HANDLE,
            super::super::graph::PASS_THROUGH_SHADER_HANDLE
        );
        assert_ne!(SPLIT_SHADER_HANDLE, crate::effects::CLIPPING_SHADER_HANDLE);
        assert_ne!(SPLIT_SHADER_HANDLE, crate::effects::OIT_ACCUMULATE_SHADER_HANDLE);
        assert_ne!(SPLIT_SHADER_HANDLE, crate::effects::CLOUDS_SHADER_HANDLE);
    }

    /// GPU 边界处的分数→像素收窄（红线，f64 → f32）。
    #[test]
    fn split_uniform_narrows_fraction_to_pixels() {
        let component = CesiumSplit {
            enabled: true,
            split_position: 0.25,
            line_width_px: 3.0,
            color: [0.1, 0.2, 0.3, 1.0],
        };
        let u = SplitUniform::from_domain(&component, 1920.0);
        assert_eq!(u.split_position_px, 480.0); // 0.25 * 1920
        assert_eq!(u.line_width_px, 3.0);
        assert_eq!(u.color, Vec4::new(0.1, 0.2, 0.3, 1.0));
    }

    // ─── Naga 防线：解析 + 校验 + binding 覆盖 ─────────────

    /// 为 `#import` 指令提供的 stub，naga 在没有 Bevy prelude 时无法解析它
    ///（与 `clouds.rs` 同技术）。
    const SPLIT_WGSL_IMPORT_STUBS: &str = "\
struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
";

    fn split_stubbed_wgsl(path: &str) -> String {
        let mut source = String::from(SPLIT_WGSL_IMPORT_STUBS);
        for line in path.lines() {
            if line.starts_with("#import") {
                continue;
            }
            source.push_str(line);
            source.push('\n');
        }
        source
    }

    fn used_bindings(
        module: &naga::Module,
        entry_name: &str,
    ) -> std::collections::BTreeSet<(u32, u32)> {
        let entry = module
            .entry_points
            .iter()
            .find(|e| e.name == entry_name)
            .unwrap_or_else(|| panic!("missing entry point {entry_name}"));
        let mut used = std::collections::BTreeSet::new();
        for (_, expr) in entry.function.expressions.iter() {
            if let naga::Expression::GlobalVariable(handle) = *expr {
                if let Some(binding) = &module.global_variables[handle].binding {
                    used.insert((binding.group, binding.binding));
                }
            }
        }
        used
    }

    fn validate(source: &str, label: &str) -> naga::Module {
        let module = naga::front::wgsl::parse_str(source)
            .unwrap_or_else(|error| panic!("{label} does not parse:\n{}", error.emit_to_string(source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap_or_else(|error| panic!("{label} does not validate: {error}"));
        module
    }

    #[test]
    fn split_wgsl_parses_and_type_checks_under_naga() {
        let source = split_stubbed_wgsl(include_str!("../../shaders/split.wgsl"));
        let module = validate(&source, "split.wgsl");
        let entry_points: Vec<_> = module
            .entry_points
            .iter()
            .map(|e| (e.name.as_str(), e.stage))
            .collect();
        assert_eq!(
            entry_points,
            vec![("fragment", naga::ShaderStage::Fragment)],
            "split.wgsl must expose exactly one fragment entry point"
        );
    }

    #[test]
    fn split_wgsl_bindings_covered_by_layout() {
        let source = split_stubbed_wgsl(include_str!("../../shaders/split.wgsl"));
        let module = validate(&source, "split.wgsl");
        // SplitPipeline：group 0 bindings 0..=1（纹理、采样器）+ group 1
        // binding 0（uniform）。
        let layout: std::collections::BTreeSet<(u32, u32)> =
            [(0, 0), (0, 1), (1, 0)].into_iter().collect();
        let used = used_bindings(&module, "fragment");
        let missing: Vec<(u32, u32)> = used.difference(&layout).copied().collect();
        assert!(
            missing.is_empty(),
            "split.wgsl uses bindings {missing:?} absent from SplitPipeline layout"
        );
    }
}
