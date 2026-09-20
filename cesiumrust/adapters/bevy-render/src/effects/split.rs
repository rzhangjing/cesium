//! M6.1: Split-screen (`Splitter` / `SplitDirection`) infrastructure.
//!
//! FIX-SPLIT (Phase 3): the split scaffolding was originally parasitic on
//! [`super::oit`] (`oit.rs:43-75` in the pre-migration tree) because M6.1 was the
//! "last merged" capability and borrowed OIT's plugin slot. This module decouples
//! it: the `SplitConfig` resource, the `SplitDragEvent`, and the
//! `split_direction_system` (the CPU-side divider-drag interaction) now live here,
//! alongside a proper screen-space render node that draws the visible divider.
//!
//! # What the node does (and does not) draw
//! Upstream CesiumJS splits by *per-primitive discard*: imagery / primitives tagged
//! `SplitDirection.LEFT` render only on the left of `Scene.splitPosition` and
//! `RIGHT` only on the right, so the two halves show two different layer states.
//! That faithful path is a *material-shader* injection
//! (`SplitterConfig::wgsl_shader_modification()`), which touches the globe / tileset
//! shaders (out of this module's file scope) and is deferred to the real-GPU task
//! (`docs/deviations.md#dev-034`).
//!
//! What *is* owned by the screen space is the draggable divider handle itself — a
//! thin vertical line at `splitPosition`. [`SplitNode`] renders exactly that: a
//! pass-through of the resolved scene colour with a `split.wgsl` overlay line drawn
//! on top. With the gate OFF the node is never registered and no `Core3d` edge
//! exists, so the v0 baselines stay bit-exact (PSNR = ∞).
//!
//! # Gate (single source of truth)
//! The gate name is owned by the app-layer registry
//! `application/cesium-app/src/feature_flags.rs` (`ENV_ENABLE_SPLIT` /
//! `split_enabled()`); [`ENV_ENABLE_SPLIT`] below is a byte-identical mirror forced
//! by the crate dependency direction (`cesium-app` → `cesium-bevy-render`). Default
//! OFF ⇒ `effects::graph::M6WaveARenderGraphPlugin` returns early and
//! [`register_split_node`] is never called.
//!
//! # Red lines honoured
//! - domain stays **f64** (`split_position` is a `[0, 1]` fraction); the fraction →
//!   viewport-pixel conversion happens ONLY at the [`SplitUniform::from_domain`]
//!   GPU boundary.
//! - No FMA contraction, no swizzle assignment in `split.wgsl`.

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

/// Unique handle for the embedded `split.wgsl` shader. Chosen to avoid collision
/// with every other cesium shader handle (asserted by `split_shader_handle_unique`).
pub const SPLIT_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(0xCE51_5F11_0006_00AA);

// ─── Gate ────────────────────────────────────────────────────────────────────

/// Env var gating the M6.1 split node. Byte-identical mirror of
/// `feature_flags::ENV_ENABLE_SPLIT` (see the module docs for why it is local).
pub const ENV_ENABLE_SPLIT: &str = "CESIUM_ENABLE_SPLIT";

/// Returns `true` when the split gate is enabled. Reuses the single authoritative
/// truthy parser (`gate_from_env_value`, the crate-wide `{1, true, yes, on}` set).
#[inline]
pub fn split_gate_enabled() -> bool {
    gate_from_env_value(std::env::var(ENV_ENABLE_SPLIT).ok())
}

// ─── Render graph label ──────────────────────────────────────────────────────

/// Node label for the cesium split node in `Core3d`.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct CesiumSplitLabel;

// ─── SplitConfig / SplitDragEvent / split_direction_system ───────────────────
// Migrated verbatim from `oit.rs` (FIX-SPLIT). These are the CPU-side divider
// interaction: a resource holding the drag state, an event carrying a new
// position, and a system that turns `CursorMoved` while dragging into events.

/// Divider-drag state resource (main world).
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
    /// Build from the domain [`SplitterConfig`] value object.
    pub fn from_splitter(config: &SplitterConfig) -> Self {
        Self {
            enabled: config.enabled,
            split_position: config.split_position,
            ..Default::default()
        }
    }
}

/// Emitted while the divider is dragged, carrying the new `[0, 1]` position.
#[derive(Event)]
pub struct SplitDragEvent {
    pub position: f64,
}

/// Turns `CursorMoved` events into [`SplitDragEvent`]s while [`SplitConfig.dragging`].
///
/// Inert when the split is disabled, so it never perturbs the golden path.
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

// ─── Component ───────────────────────────────────────────────────────────────

/// Component carrying the split state for a view (per-camera).
///
/// Placed on the camera (like [`super::clouds::CesiumClouds`]) to drive the
/// screen-space node; extracted to the render world via `ExtractComponentPlugin`.
/// The node early-returns when `enabled == false` (zero GPU cost, pixel-neutral).
/// `Default` is derived: `enabled = false` (conservative).
#[derive(Component, Clone, Debug, Default, ExtractComponent)]
pub struct CesiumSplit {
    /// Master enable for the split node on this view.
    pub enabled: bool,
    /// Divider centre as a `[0, 1]` fraction of the viewport width (domain f64).
    pub split_position: f64,
    /// Divider thickness in PIXELS.
    pub line_width_px: f64,
    /// Divider colour RGBA `[0, 1]`.
    pub color: [f64; 4],
}

impl CesiumSplit {
    /// Convenience constructor for an enabled divider at `split_position`.
    pub fn new(split_position: f64) -> Self {
        Self {
            enabled: true,
            split_position: split_position.clamp(0.0, 1.0),
            line_width_px: 2.0,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// Whether the divider should actually render.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.enabled
    }
}

/// Per-view cached pipeline ID for the split node.
#[derive(Component)]
pub struct CameraSplitPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

/// Per-view GPU uniform buffer holding the packed divider parameters.
#[derive(Component)]
pub struct ViewSplitUniform {
    pub buffer: UniformBuffer<SplitUniform>,
}

// ─── GPU uniform (f32 boundary) ──────────────────────────────────────────────

/// GPU-facing split uniform. **f32 only** — the component's `[0, 1]` f64 fraction
/// and pixel width narrow here ([`SplitUniform::from_domain`], red line).
///
/// Layout must match `struct SplitData` in `shaders/split.wgsl` (encase std140).
/// Lives in a private module with `#![allow(dead_code)]` (the `clipping_planes.rs`
/// convention — the encase `ShaderType` derive emits a helper the dead-code pass
/// flags even though every field is uploaded via `write_buffer`).
pub use split_uniform::SplitUniform;

mod split_uniform {
    #![allow(dead_code)]
    use bevy::prelude::Vec4;
    use bevy::render::render_resource::ShaderType;

    /// Matches `struct SplitData` in `shaders/split.wgsl` (encase std140).
    #[derive(ShaderType, Clone, Copy, Debug)]
    pub struct SplitUniform {
        /// Divider centre in viewport pixels (std140: offset 0).
        pub split_position_px: f32,
        /// Divider thickness in pixels (std140: offset 4).
        pub line_width_px: f32,
        /// Divider colour RGBA (std140: 16-byte aligned, offset 16).
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
    /// Packs a [`CesiumSplit`] into the GPU uniform. `split_position` is the
    /// `[0, 1]` domain fraction; it is multiplied by `viewport_width_px` **here**
    /// (the single f64 → f32, fraction → pixel boundary).
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

/// Render-world resource: the two bind group layouts + sampler for the split node.
///
/// - group 0: `screen_texture` (`texture_2d<f32>`, binding 0) + linear sampler (1)
/// - group 1: `SplitUniform` (binding 0)
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
        render_pass.draw(0..3, 0..1); // fullscreen triangle

        Ok(())
    }
}

// ─── Render systems ──────────────────────────────────────────────────────────

/// Prepares the split pipeline + per-view uniform for each active view.
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

// ─── Registration ────────────────────────────────────────────────────────────

/// Register the split node (three-stage, DEV-029). Convenience wrapper around the
/// main / render halves; headless-safe (no `RenderApp` → render half skipped).
#[deprecated = "DEV-029 / FIX-REG-FACADE: call `register_split_node_main_world` from `Plugin::build` and `register_split_node_render_world` from `Plugin::finish`; this facade runs the finish half against a possibly device-less render world."]
pub fn register_split_node(app: &mut App) {
    register_split_node_main_world(app);
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        register_split_node_render_world(render_app);
    }
}

/// `Plugin::build`-time half: WGSL shader + `ExtractComponentPlugin` + the
/// divider-drag scaffolding (resource + event + system). Main world only.
pub fn register_split_node_main_world(app: &mut App) {
    crate::shader_registry::try_load_internal_shader(
        app,
        SPLIT_SHADER_HANDLE,
        include_str!("../../shaders/split.wgsl"),
        "shaders/split.wgsl",
    );

    app.add_plugins(ExtractComponentPlugin::<CesiumSplit>::default());

    // Migrated divider-drag interaction (formerly in `OITPlugin`).
    app.init_resource::<SplitConfig>()
        .add_event::<SplitDragEvent>()
        .add_systems(Update, split_direction_system);
}

/// `Plugin::finish`-time half: render-world pipeline resource + `Core3d` node.
/// Reads `RenderDevice` (via `SplitPipeline`'s `FromWorld`), hence `finish`.
pub fn register_split_node_render_world(render_app: &mut bevy::app::SubApp) {
    // FIX-REG-FACADE (DEV-029): degrade to a no-op when `RenderDevice` is absent
    // (finish half reached from `build`, or a bare render world). See
    // `crate::effects::render_world_missing_device`.
    if crate::effects::render_world_missing_device(render_app) {
        return;
    }

    render_app
        .init_resource::<SplitPipeline>()
        .add_systems(Render, prepare_split.in_set(RenderSet::Prepare))
        .add_render_graph_node::<ViewNodeRunner<SplitNode>>(Core3d, CesiumSplitLabel);
    // NOTE: edges are created by `effects::graph::wire_m6_edges`, the single owner
    // of the shared `Core3d` chain — never here, so no diamond can form.
}

// ─── Tests ───────────────────────────────────────────────────────────────────

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
        // The const must be a byte-identical uppercase env name.
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

    /// The fraction → pixel narrowing at the GPU boundary (red line, f64 → f32).
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

    // ─── Naga defence line: parse + validate + binding coverage ─────────────

    /// Stubs for the `#import` directive naga cannot resolve without the Bevy
    /// prelude (same technique as `clouds.rs`).
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
        // SplitPipeline: group 0 bindings 0..=1 (texture, sampler) + group 1
        // binding 0 (uniform).
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
