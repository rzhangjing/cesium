//! Rust mirrors of `packages/engine/Specs/Scene/Model/AlphaPipelineStageSpec.js`,
//! `LightingPipelineStageSpec.js`, `ModelColorPipelineStageSpec.js`, and the
//! stage-ordering contract of `ModelRuntimePrimitiveSpec.js` (`configurePipeline`).
//!
//! DEVIATION (B3.5, adapted pipeline): the CesiumJS specs assert on the GLSL a
//! stage appends to a `ShaderBuilder` (`ShaderBuilderTester.expectHasFragmentDefines`
//! / `expectHasFragmentUniforms`). The wgpu port renders through static WGSL, so
//! these specs instead assert on the *values the stages record* on the
//! [`PrimitiveRenderResources`] bag — the resolved pass / render state / lighting
//! model / color-blend factor — which is what the ported draw path consumes.
//! Shader-source assertions that have no ported equivalent (`ALPHA_MODE_MASK`
//! discard, `model_lightColorHdr`, `HAS_MODEL_COLOR` color-mask zeroing) are
//! covered by the fidelity notes in `docs/deviations.md`.
//!
//! The stage-isolation specs still build a real [`Context`] because
//! [`PipelineContext`] carries one for the geometry/material stages; they skip
//! gracefully when no GPU adapter exists, mirroring the model GPU-batch
//! convention. The `ColorBlendMode::get_color_blend` spec is pure logic.

use cesium_core::color::Color;
use cesium_core::math::CesiumMath;
use cesium_renderer::context::Context;
use cesium_renderer::pass::Pass;
use cesium_renderer::render_state::BlendingFactor;

use cesium_scene::frame_state::FrameState;
use cesium_scene::gltf_loader::{GltfJson, GltfPrimitive};
use cesium_scene::gltf_pipeline::parse_glb::parse_glb;
use cesium_scene::model::alpha_pipeline_stage::AlphaPipelineStage;
use cesium_scene::model::lighting_model::LightingModel;
use cesium_scene::model::lighting_pipeline_stage::LightingPipelineStage;
use cesium_scene::model::model::{ColorBlendMode, Model};
use cesium_scene::model::model_color_pipeline_stage::ModelColorPipelineStage;
use cesium_scene::model::model_pipeline_stage::{configure_pipeline, PipelineContext};
use cesium_scene::model::primitive_render_resources::PrimitiveRenderResources;

// ---------------------------------------------------------------------------
// GPU acquisition (mirrors the model GPU-batch convention)
// ---------------------------------------------------------------------------

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

fn try_gpu() -> Option<Gpu> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(
        instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
    )
    .ok()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("model_pipeline_stage_spec"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::default(),
        trace: wgpu::Trace::Off,
    }))
    .ok()?;
    Some(Gpu { device, queue })
}

/// Loads and parses the BoxTextured.glb fixture (an opaque, textured, PBR cube).
fn box_textured_gltf() -> GltfJson {
    let path = cesium_specs::data_path("Models/glTF-2.0/BoxTextured/glTF-Binary/BoxTextured.glb");
    let glb = std::fs::read(path).expect("BoxTextured.glb fixture missing");
    parse_glb(&glb).expect("BoxTextured.glb must parse")
}

/// Builds a [`PipelineContext`] with model-level defaults (opaque white color,
/// Highlight blend, lighting + back-face culling on, opaque pass). Individual
/// specs override the fields they exercise.
fn ctx_with<'a>(
    gltf: &'a GltfJson,
    primitive: &'a GltfPrimitive,
    context: &'a Context,
) -> PipelineContext<'a> {
    PipelineContext {
        gltf,
        primitive,
        node_index: 0,
        context,
        model_color: Color::new(1.0, 1.0, 1.0, 1.0),
        color_blend_mode: ColorBlendMode::Highlight,
        color_blend_amount: 0.0,
        enable_lighting: true,
        back_face_culling: true,
        opaque_pass: Pass::Opaque,
    }
}

// ---------------------------------------------------------------------------
// ColorBlendMode.getColorBlend (pure logic; mirrors ColorBlendModeSpec.js)
// ---------------------------------------------------------------------------

/// Mirrors `ColorBlendMode.getColorBlend`: HIGHLIGHT → 0.0, REPLACE → 1.0,
/// MIX → clamp(amount, EPSILON4, 1.0).
#[test]
fn color_blend_mode_get_color_blend() {
    // HIGHLIGHT ignores the amount.
    assert_eq!(ColorBlendMode::Highlight.get_color_blend(0.25), 0.0);
    assert_eq!(ColorBlendMode::Highlight.get_color_blend(0.9), 0.0);
    // REPLACE ignores the amount.
    assert!((ColorBlendMode::Replace.get_color_blend(0.25) - 1.0).abs() < 1e-6);
    // MIX passes the amount through within range.
    assert!((ColorBlendMode::Mix.get_color_blend(0.25) - 0.25).abs() < 1e-6);
    // MIX clamps below to EPSILON4 (0.0 is reserved for HIGHLIGHT).
    let low = ColorBlendMode::Mix.get_color_blend(0.0);
    assert!((low - CesiumMath::EPSILON4 as f32).abs() < 1e-9, "got {low}");
    // MIX clamps above to 1.0.
    assert!((ColorBlendMode::Mix.get_color_blend(2.0) - 1.0).abs() < 1e-6);
}

// ---------------------------------------------------------------------------
// configurePipeline ordering (mirrors ModelRuntimePrimitive.configurePipeline)
// ---------------------------------------------------------------------------

/// The adapted pipeline assembles the applicable CesiumJS stages in order:
/// Geometry → Material → ModelColor → Lighting → Alpha.
#[test]
fn configure_pipeline_orders_stages() {
    let Some(gpu) = try_gpu() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let context = Context::new(gpu.device.clone(), gpu.queue.clone(), 64, 64, None);
    let gltf = GltfJson::default();
    let primitive = GltfPrimitive::default();
    let ctx = ctx_with(&gltf, &primitive, &context);

    let names: Vec<&'static str> = configure_pipeline(&ctx).into_iter().map(|(name, _)| name).collect();
    assert_eq!(
        names,
        vec![
            "GeometryPipelineStage",
            "MaterialPipelineStage",
            "ModelColorPipelineStage",
            "LightingPipelineStage",
            "AlphaPipelineStage",
        ]
    );
}

// ---------------------------------------------------------------------------
// AlphaPipelineStage (mirrors AlphaPipelineStageSpec.js)
// ---------------------------------------------------------------------------

/// Mirrors `it("defaults to the model's pass if not specified")`: with no
/// `alphaOptions.pass`, the resolved pass is the model's opaque pass.
#[test]
fn alpha_stage_defaults_to_model_pass() {
    let Some(gpu) = try_gpu() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let context = Context::new(gpu.device.clone(), gpu.queue.clone(), 64, 64, None);
    let gltf = GltfJson::default();
    let primitive = GltfPrimitive::default();
    let mut ctx = ctx_with(&gltf, &primitive, &context);
    // A non-default opaque pass keeps the assertion mutation-visible.
    ctx.opaque_pass = Pass::Cesium3dTile;

    let mut rres = PrimitiveRenderResources::new(0);
    assert!(rres.alpha_options.pass.is_none());
    AlphaPipelineStage::process(&mut rres, &ctx).unwrap();

    assert_eq!(rres.pass, Pass::Cesium3dTile);
    assert!(!rres.translucent);
    assert!(rres.render_state.depth_test.enabled);
    assert!(rres.render_state.depth_mask);
    assert!(!rres.render_state.blending.enabled);
}

/// Mirrors `it("sets render state options when given translucent pass")`:
/// a translucent pass disables the depth mask and enables ALPHA_BLEND.
#[test]
fn alpha_stage_sets_render_state_for_translucent_pass() {
    let Some(gpu) = try_gpu() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let context = Context::new(gpu.device.clone(), gpu.queue.clone(), 64, 64, None);
    let gltf = GltfJson::default();
    let primitive = GltfPrimitive::default();
    let ctx = ctx_with(&gltf, &primitive, &context);

    let mut rres = PrimitiveRenderResources::new(0);
    rres.alpha_options.pass = Some(Pass::Translucent);
    AlphaPipelineStage::process(&mut rres, &ctx).unwrap();

    assert_eq!(rres.pass, Pass::Translucent);
    assert!(rres.translucent);
    // depthMask false + ALPHA_BLEND (mirrors the JS renderStateOptions asserts).
    assert!(!rres.render_state.depth_mask);
    let blending = &rres.render_state.blending;
    assert!(blending.enabled);
    assert_eq!(blending.function_source_rgb, BlendingFactor::SrcAlpha);
    assert_eq!(blending.function_source_alpha, BlendingFactor::One);
    assert_eq!(blending.function_destination_rgb, BlendingFactor::OneMinusSrcAlpha);
    assert_eq!(blending.function_destination_alpha, BlendingFactor::OneMinusSrcAlpha);
    // Culling stays disabled at build time (finalized per-frame in Model::update).
    assert!(!rres.render_state.cull.enabled);
}

/// Mirrors `it("handles alphaCutoff")`: the cutoff is preserved on the alpha
/// options. DEVIATION: the JS `ALPHA_MODE_MASK` define + `u_alphaCutoff`
/// uniform have no static-WGSL equivalent (discard deferred), so the port only
/// records the cutoff.
#[test]
fn alpha_stage_preserves_alpha_cutoff() {
    let Some(gpu) = try_gpu() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let context = Context::new(gpu.device.clone(), gpu.queue.clone(), 64, 64, None);
    let gltf = GltfJson::default();
    let primitive = GltfPrimitive::default();
    let ctx = ctx_with(&gltf, &primitive, &context);

    let mut rres = PrimitiveRenderResources::new(0);
    rres.alpha_options.alpha_cutoff = Some(0.6);
    rres.alpha_options.pass = Some(Pass::Translucent);
    AlphaPipelineStage::process(&mut rres, &ctx).unwrap();

    assert_eq!(rres.alpha_options.alpha_cutoff, Some(0.6));
}

// ---------------------------------------------------------------------------
// LightingPipelineStage (mirrors LightingPipelineStageSpec.js)
// ---------------------------------------------------------------------------

/// Mirrors `it("supports unlit lighting")` / `it("supports PBR lighting")` /
/// the `enableLighting` gate: the resolved lighting model follows
/// `lightingOptions.lightingModel` when lighting is enabled, else UNLIT.
#[test]
fn lighting_stage_resolves_lighting_model() {
    let Some(gpu) = try_gpu() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let context = Context::new(gpu.device.clone(), gpu.queue.clone(), 64, 64, None);
    let gltf = GltfJson::default();
    let primitive = GltfPrimitive::default();

    // UNLIT material + lighting enabled → UNLIT.
    let mut ctx = ctx_with(&gltf, &primitive, &context);
    let mut rres = PrimitiveRenderResources::new(0);
    rres.lighting_options.lighting_model = LightingModel::Unlit;
    LightingPipelineStage::process(&mut rres, &ctx).unwrap();
    assert_eq!(rres.lighting_model, LightingModel::Unlit);

    // PBR material + lighting enabled → PBR.
    let mut rres = PrimitiveRenderResources::new(0);
    rres.lighting_options.lighting_model = LightingModel::Pbr;
    LightingPipelineStage::process(&mut rres, &ctx).unwrap();
    assert_eq!(rres.lighting_model, LightingModel::Pbr);

    // Lighting disabled forces UNLIT even for a PBR material.
    ctx.enable_lighting = false;
    let mut rres = PrimitiveRenderResources::new(0);
    rres.lighting_options.lighting_model = LightingModel::Pbr;
    LightingPipelineStage::process(&mut rres, &ctx).unwrap();
    assert_eq!(rres.lighting_model, LightingModel::Unlit);
}

// ---------------------------------------------------------------------------
// ModelColorPipelineStage (mirrors ModelColorPipelineStageSpec.js)
// ---------------------------------------------------------------------------

/// Mirrors `it("configures the render resources for opaque color")`: an opaque
/// model color keeps the pass and records `getColorBlend(REPLACE, 0.25) == 1.0`.
#[test]
fn model_color_stage_opaque() {
    let Some(gpu) = try_gpu() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let context = Context::new(gpu.device.clone(), gpu.queue.clone(), 64, 64, None);
    let gltf = GltfJson::default();
    let primitive = GltfPrimitive::default();
    let mut ctx = ctx_with(&gltf, &primitive, &context);
    ctx.model_color = Color::new(1.0, 0.0, 0.0, 1.0); // Color.RED
    ctx.color_blend_mode = ColorBlendMode::Replace;
    ctx.color_blend_amount = 0.25;

    let mut rres = PrimitiveRenderResources::new(0);
    rres.alpha_options.pass = Some(Pass::Opaque);
    ModelColorPipelineStage::process(&mut rres, &ctx).unwrap();

    assert!((rres.color_blend - 1.0).abs() < 1e-6);
    // Opaque model color (alpha 1) leaves the pass unchanged.
    assert_eq!(rres.alpha_options.pass, Some(Pass::Opaque));
}

/// Mirrors `it("configures the render resources for translucent color")`: a
/// translucent model color forces the translucent pass and records
/// `getColorBlend(MIX, 0.25) == 0.25`.
#[test]
fn model_color_stage_translucent() {
    let Some(gpu) = try_gpu() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let context = Context::new(gpu.device.clone(), gpu.queue.clone(), 64, 64, None);
    let gltf = GltfJson::default();
    let primitive = GltfPrimitive::default();
    let mut ctx = ctx_with(&gltf, &primitive, &context);
    ctx.model_color = Color::new(1.0, 0.0, 0.0, 0.2); // Color.RED.withAlpha(0.2)
    ctx.color_blend_mode = ColorBlendMode::Mix;
    ctx.color_blend_amount = 0.25;

    let mut rres = PrimitiveRenderResources::new(0);
    rres.alpha_options.pass = Some(Pass::Opaque);
    ModelColorPipelineStage::process(&mut rres, &ctx).unwrap();

    assert!((rres.color_blend - 0.25).abs() < 1e-6);
    // Translucent model color (alpha 0.2) routes through the translucent pass.
    assert_eq!(rres.alpha_options.pass, Some(Pass::Translucent));
}

// ---------------------------------------------------------------------------
// End-to-end: the full chain over BoxTextured (mirrors ModelSpec.js build path)
// ---------------------------------------------------------------------------

/// GPU-required: running the adapted chain over BoxTextured yields an opaque,
/// textured, PBR-lit primitive whose stored render state / pass match the
/// pre-refactor inline path (behavior preservation), with the default
/// Highlight color blend recording 0.0.
#[test]
fn stage_chain_configures_box_textured() {
    let Some(gpu) = try_gpu() else {
        eprintln!("no GPU adapter available; skipping");
        return;
    };
    let mut context = Context::new(gpu.device.clone(), gpu.queue.clone(), 64, 64, None);
    let mut model = Model::from_gltf(box_textured_gltf());
    model.update(&FrameState::new(), &mut context);

    assert!(model.ready, "the first GPU update must finish the model");
    assert_eq!(model.runtime_primitives().len(), 1);
    let primitive = &model.runtime_primitives()[0];

    // Behavior preserved from the pre-refactor inline build path.
    assert!(primitive.is_textured(), "BoxTextured takes the textured path");
    assert_eq!(primitive.count, 36, "unit cube: 12 triangles indexed");
    assert!(primitive.vertex_array.is_some());

    // Stage-chain outputs: opaque PBR cube, default Highlight blend.
    assert_eq!(primitive.pass, Pass::Opaque);
    assert!(!primitive.translucent);
    assert!(!primitive.double_sided);
    assert_eq!(primitive.lighting_model, LightingModel::Pbr);
    assert!((primitive.color_blend - 0.0).abs() < 1e-6);
    assert!(primitive.render_state.depth_test.enabled);
    assert!(primitive.render_state.depth_mask);
    assert!(!primitive.render_state.blending.enabled);

    // The bounding sphere radius is the unit-cube half diagonal (√3/2).
    let radius = model.bounding_sphere.radius;
    assert!(
        (radius - (3.0_f64.sqrt() / 2.0)).abs() < 1e-6,
        "bounding sphere radius {radius} must be the unit-cube half diagonal"
    );
}
