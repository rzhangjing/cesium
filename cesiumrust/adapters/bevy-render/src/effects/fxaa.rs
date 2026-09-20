//! M5-E1: FXAA anti-aliasing ViewNode (quality preset 12 only).
//!
//! Implements the cesiumrust FXAA post-process node using the RenderGraph
//! infrastructure from M5-E0 (`graph.rs`). The node is inserted between
//! `Node3d::Tonemapping` and `Node3d::EndMainPassPostProcessing` in `Core3d`.
//!
//! # Blueprint
//! - `cesium-rs/crates/cesium-shaders/shaders/FXAA3_11.glsl` (651 lines, preset 12 = L102-108)
//! - `packages/engine/Source/Shaders/PostProcessStages/FXAA.glsl` (21 lines, interface)
//! - `packages/engine/Source/Scene/PostProcessStageLibrary.js` L611 `createFXAAStage`
//!
//! # DEVIATION
//! WGSL rewrite of GLSL FXAA 3.11; only quality preset 12 implemented (plan L155).
//! See `docs/deviations.md#dev-017`.

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

/// Unique handle for the embedded `fxaa.wgsl` shader.
pub const FXAA_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0xCE51_E1E1_F4AA_0012);

// ─── Component ───────────────────────────────────────────────────────────────

/// Marker component enabling cesiumrust FXAA on a camera entity.
///
/// Extracted to render world via `ExtractComponentPlugin`. The `FxaaNode`
/// early-returns when `enabled == false` (zero GPU cost).
#[derive(Component, Clone, Debug, ExtractComponent)]
pub struct CesiumFxaa {
    /// Master enable for the FXAA pass on this camera.
    pub enabled: bool,
}

impl Default for CesiumFxaa {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Per-view cached pipeline ID for the FXAA node.
#[derive(Component)]
pub struct CameraFxaaPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

// ─── Pipeline ────────────────────────────────────────────────────────────────

/// Render-world resource: bind group layout + sampler for FXAA.
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

/// Specialization key: texture format only (preset 12 is hardcoded in WGSL).
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

/// FXAA `ViewNode` — runs after tonemapping, before EndMainPassPostProcessing.
///
/// Pattern matches `bevy_core_pipeline::fxaa::node::FxaaNode` (86 lines) with
/// cesiumrust's own WGSL shader (quality preset 12).
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
        render_pass.draw(0..3, 0..1); // fullscreen triangle

        Ok(())
    }
}

// ─── Render systems ──────────────────────────────────────────────────────────

/// Prepares specialized FXAA pipelines per camera view.
/// Runs in `Render` schedule, `RenderSet::Prepare`.
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

// ─── Registration ────────────────────────────────────────────────────────────

/// Register the FXAA node into `RenderApp` (shader + extract + node + systems).
///
/// Called from `register_render_graph` (graph.rs) when the post-process gate is
/// ON. This function registers the node **but does not create graph edges** —
/// `register_render_graph` owns the single linear chain (Daniel H2, upstream
/// CesiumJS parity) `EndMainPass → PassThrough → AmbientOcclusion → Tonemapping →
/// Fxaa → EndMainPassPostProcessing`, so the cesium nodes never form a diamond in
/// `Core3d`.
///
/// Position rationale: FXAA runs **after** tonemapping (HDR linear → LDR done)
/// and **before** upscaling, matching CesiumJS where FXAA operates on the final
/// LDR image. See `docs/deviations.md#dev-017`.
#[deprecated = "DEV-029 / FIX-REG-FACADE: call `register_fxaa_node_main_world` from `Plugin::build` and `register_fxaa_node_render_world` from `Plugin::finish`; this facade runs the finish half against a possibly device-less render world."]
pub fn register_fxaa_node(app: &mut App) {
    register_fxaa_node_main_world(app);
    // Headless `MinimalPlugins` has no `RenderApp` — degrade gracefully.
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        register_fxaa_node_render_world(render_app);
    }
}

/// `Plugin::build`-time half of [`register_fxaa_node`]: everything that lives in
/// the **main** world (WGSL shader asset + `ExtractComponentPlugin`).
///
/// Split out by task #81 — see `docs/deviations.md#dev-029`. `FxaaPipeline`'s
/// `FromWorld` reads `RenderDevice`, and Bevy only inserts `RenderDevice` into
/// the render world in `RenderPlugin::finish` (`bevy_render/src/lib.rs`
/// L399-430), so calling [`register_fxaa_node_render_world`] from any plugin's
/// `build` panics with "RenderDevice does not exist in the World". Callers must
/// therefore run the two halves from `build` and `finish` respectively (the
/// pattern Bevy itself uses: `bevy_pbr/src/ssao/mod.rs` L54 `build` / L80
/// `finish`).
pub fn register_fxaa_node_main_world(app: &mut App) {
    // Register FXAA WGSL shader (headless-safe).
    crate::shader_registry::try_load_internal_shader(
        app,
        FXAA_SHADER_HANDLE,
        include_str!("../../shaders/fxaa.wgsl"),
        "shaders/fxaa.wgsl",
    );

    // ExtractComponentPlugin for CesiumFxaa (ExtractSchedule: main→render world).
    app.add_plugins(ExtractComponentPlugin::<CesiumFxaa>::default());
}

/// `Plugin::finish`-time half of [`register_fxaa_node`]: the render-world
/// pipeline resources + the `Core3d` node.
pub fn register_fxaa_node_render_world(render_app: &mut bevy::app::SubApp) {
    // FIX-REG-FACADE (DEV-029): degrade to a no-op when `RenderDevice` is absent
    // (finish half reached from `build`, or a bare render world). See
    // `crate::effects::render_world_missing_device`.
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
    // NOTE: edges are created by `register_render_graph` (unified linear chain).
}

// ─── Tests ───────────────────────────────────────────────────────────────────

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
        // Should not panic (no RenderApp).
        #[allow(deprecated)]
        register_fxaa_node(&mut app);
    }

    #[test]
    fn fxaa_shader_handle_unique() {
        // Ensure no collision with pass-through handle.
        assert_ne!(
            FXAA_SHADER_HANDLE,
            super::super::graph::PASS_THROUGH_SHADER_HANDLE
        );
    }

    // ─── Ryan C1 defence line: headless naga parse + validate + layout parity ──

    /// naga has no preprocessor, so `fxaa.wgsl`'s single `#import`
    /// (`FullscreenVertexOutput`) is replaced by a stub declaring the field the
    /// fragment reads (`position`). Everything else is the real shader source.
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

    /// Headless proof the FXAA shader is real: parsed + type-checked by **naga**
    /// (the same WGSL front end `bevy_render` compiles it with on the GPU path).
    /// Device readback still needs xvfb (`.github/workflows/cesiumrust-e2e.yml`).
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

    /// Ryan C1 (catches the **C2** class of bug): every binding the `fragment`
    /// entry statically uses must be present in the Rust `FxaaPipeline` layout
    /// (group 0: binding 0 = screen texture, binding 1 = sampler). Guards against a
    /// silent pipeline-build failure that would no-op FXAA while `pixel_diff`
    /// reported a false green.
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
