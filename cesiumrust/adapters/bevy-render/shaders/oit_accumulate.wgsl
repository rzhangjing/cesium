// M6.4: cesiumrust OIT accumulate pass — MRT dual-output weighted blending.
// ============================================================================
// Screen-space pass that reads the scene colour (containing translucent
// fragments) and the depth prepass, computes the McGuire-Bavoil 2013 per-
// fragment weight, and outputs to two render targets:
//   @location(0) accumulate (Rgba16Float): rgb = Ci*wzi, a = ai*wzi
//   @location(1) revealage  (R8Unorm):     r  = ai (blended multiplicatively)
//
// The blend state on attachment 1 is (Zero, OneMinusSrc) so the GPU computes
// Π(1−ai) across all translucent fragments — the classic WBOIT revealage.
//
// # Blueprints (paths + line numbers)
//   CesiumJS upstream (authoritative reference):
//     • packages/engine/Source/Scene/OIT.js L487-492 (mrtShaderSource):
//         vec3 Ci = czm_out_FragColor.rgb * czm_out_FragColor.a;
//         float ai = czm_out_FragColor.a;
//         float wzi = czm_alphaWeight(ai);
//         out_FragData_0 = vec4(Ci * wzi, ai);
//         out_FragData_1 = vec4(ai * wzi);
//     • packages/engine/Source/Scene/OIT.js L408-417 (translucentMRTBlend):
//         RGB: ONE + ONE (additive), Alpha: ZERO + ONE_MINUS_SOURCE_ALPHA
//     • packages/engine/Source/Shaders/Builtin/Functions/alphaWeight.glsl L4-11:
//         czm_alphaWeight — upstream weighting function (NOT used here; see DEVIATION)
//     • packages/engine/Source/Scene/OIT.js L137-156 (updateTextures):
//         accumulation = RGBA FLOAT, revealage = RGBA FLOAT (both float in upstream)
//
//   Domain (CPU reference — WGSL matches this formula):
//     • domain/effects/src/oit.rs L160-167 (OitConfig::compute_weight):
//         w = alpha * clamp(0.03 / (1e-5 + (depth/200)^4), 0.01, 3000)
//
// # DEVIATIONS from upstream
//   1. Weight formula: domain compute_weight (α·clamp(0.03/(1e-5+z⁴/200⁴),0.01,3000))
//      differs from upstream czm_alphaWeight (pow(a+0.01,4)+max(1e-2,min(3e3,…))).
//      We match domain — single source of truth for CPU/GPU parity.
//   2. Revealage format: R8Unorm (stores Π(1−ai) via multiplicative blend) instead
//      of upstream RGBA-float. Weight sum Σ(ai·wzi) goes to accumulate.a (Rgba16Float).
//   3. Screen-space approximation: reads already-composited scene colour rather than
//      intercepting per-fragment translucent draw commands. Full geometry-level MRT
//      redirection is deferred to integrator (deep Bevy render-graph surgery).
//   4. Shader outputs ai to @location(1) (not ai*wzi as upstream) because the
//      multiplicative blend (Zero, OneMinusSrc) computes Π(1−ai) directly from ai.
//
// # RED LINES
//   • METERS_PER_RENDER_UNIT = 6378137 — depth is in render units (0..1 NDC),
//     the /200.0 constant operates on NDC-reconstructed linear depth.
//   • NO FMA contraction — each arithmetic step written separately.
//   • domain f64 → f32 only at this WGSL boundary.
//   • WGSL reserved word avoidance: no `mod` identifier used.
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View

// ─── Bindings ───────────────────────────────────────────────────────────────
@group(0) @binding(0) var depth_prepass: texture_depth_2d;
@group(0) @binding(1) var scene_color: texture_2d<f32>;
@group(0) @binding(2) var point_sampler: sampler;
@group(0) @binding(3) var<uniform> view: View;

// ─── McGuire-Bavoil 2013 weight (matches domain/effects/src/oit.rs L164-167) ─
// w = alpha * clamp(0.03 / (1e-5 + (depth/200)^4), 0.01, 3000)
// depth: linear view-space depth in render units (positive, near→far).
fn alpha_weight(alpha: f32, depth: f32) -> f32 {
    // Step 1: depth_term = (depth / 200.0)^4
    let d_over_200 = depth / 200.0;
    let d2 = d_over_200 * d_over_200;
    let depth_term = d2 * d2;

    // Step 2: raw_weight = 0.03 / (1e-5 + depth_term)
    let denominator = 1e-5 + depth_term;
    let raw_weight = 0.03 / denominator;

    // Step 3: clamped = clamp(raw_weight, 0.01, 3000.0)
    let clamped_weight = clamp(raw_weight, 0.01, 3000.0);

    // Step 4: final = alpha * clamped
    return alpha * clamped_weight;
}

// ─── Reconstruct linear depth from the depth prepass ──────────────────────────
// Bevy 0.15 reversed-Z: near=1, far=0. view_from_clip inverse-projects NDC→view.
fn reconstruct_linear_depth(uv: vec2<f32>) -> f32 {
    let raw_depth = textureSampleLevel(depth_prepass, point_sampler, uv, 0.0);
    // Reversed-Z: raw_depth ∈ [0,1], 1=near, 0=far. Sky pixels = 0.
    if raw_depth < 1e-6 {
        // Sky / far plane — no translucent contribution.
        return 1e9;
    }
    // Convert NDC depth to clip-space z (reversed-Z: clip_z = 1 - ndc_z is wrong;
    // Bevy uses wgpu convention where NDC z ∈ [0,1] directly maps to depth buffer).
    // Reconstruct view-space position via inverse projection.
    let ndc = vec4<f32>(uv * 2.0 - 1.0, raw_depth, 1.0);
    let view_pos = view.view_from_clip * ndc;
    // Perspective divide → view-space position (negative z = in front of camera).
    let linear_z = abs(view_pos.z / view_pos.w);
    return linear_z;
}

// ─── Fragment entry: MRT dual-output accumulate ──────────────────────────────
// Two colour targets, matching the Rust ColorTargetState list in
// `prepare_oit_pipelines`: @location(0)→Rgba16Float accumulation,
// @location(1)→R8Unorm revealage. `@builtin(position)` is deliberately NOT an
// output (it is not a valid fragment-output built-in; naga rejects it with
// InvalidBuiltInStage) — the pass writes no depth/position, only colour.
struct OitAccumulateOutput {
    @location(0) accumulate: vec4<f32>,
    @location(1) revealage: f32,
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> OitAccumulateOutput {
    var out: OitAccumulateOutput;

    let uv = in.uv;
    let color = textureSampleLevel(scene_color, point_sampler, uv, 0.0);

    // If fully opaque (alpha ≈ 1) or sky, output zero contribution.
    let ai = color.a;
    if ai > 0.999 || ai < 0.001 {
        out.accumulate = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        out.revealage = 0.0;
        return out;
    }

    let depth = reconstruct_linear_depth(uv);

    // Premultiply: Ci = rgb * alpha (OIT.js L488)
    let ci = color.rgb * ai;

    // Weight: McGuire-Bavoil 2013 via domain formula
    let wzi = alpha_weight(ai, depth);

    // Accumulate output: vec4(Ci * wzi, ai * wzi) — OIT.js L491
    out.accumulate = vec4<f32>(ci * wzi, ai * wzi);

    // Revealage output: ai (GPU blend computes Π(1−ai) via OneMinusSrc)
    out.revealage = ai;

    return out;
}
