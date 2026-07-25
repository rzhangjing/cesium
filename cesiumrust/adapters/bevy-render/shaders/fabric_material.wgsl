// Fabric procedural material shader.
//
// This is the bevy-render adapter counterpart of the domain `cesium-material`
// crate. The domain layer assembles CesiumJS Fabric GLSL source text; this
// adapter renders the equivalent procedural patterns natively in WGSL so they
// run on Bevy's wgpu pipeline without a runtime GLSL->WGSL transpiler.
//
// Each pattern below is a faithful port of the corresponding CesiumJS material
// shader in `Source/Shaders/Materials/*.glsl`, including the shared built-ins
// `czm_antialias` and `czm_gammaCorrect` (non-HDR path, which is the identity)
// and the `czm_getDefaultMaterial` base values (diffuse=0, emission=0, alpha=1).
//
// The `kind` field of `FabricParams` selects the pattern. Uniform values are
// packed from the domain `Material::uniforms()` map by the Rust adapter.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings

struct FabricParams {
    kind: u32,          // FabricKind discriminant
    horizontal: u32,    // Stripe: 0/1 (GLSL bool horizontal)
    repeat_flag: u32,   // Fade: 0/1 (GLSL bool repeat)
    pixel_ratio: u32,   // Grid: czm_pixelRatio (as u32, typically 1)
    color_a: vec4<f32>, // light/even/color/fadeIn/base
    color_b: vec4<f32>, // dark/odd/fadeOut
    color_c: vec4<f32>, // Image tint (color)
    repeat_offset: vec4<f32>, // x=repeat.x, y=repeat.y, z=offset(stripe), w=maximumDistance(fade)
    line_params: vec4<f32>,   // x=lineCount.x, y=lineCount.y, z=lineThickness.x, w=lineThickness.y
    line_off_cell: vec4<f32>, // x=lineOffset.x, y=lineOffset.y, z=cellAlpha, w=(spare)
    fade_dir_time: vec4<f32>, // x=fadeDirection.x, y=fadeDirection.y, z=time.x, w=time.y
}

@group(2) @binding(0) var<uniform> params: FabricParams;
@group(2) @binding(1) var image_texture: texture_2d<f32>;
@group(2) @binding(2) var image_sampler: sampler;

// ---------------------------------------------------------------------------
// Shared built-ins (ported from Shaders/Builtin/Functions/*.glsl)
// ---------------------------------------------------------------------------

// czm_antialias (Shaders/Builtin/Functions/antialias.glsl)
fn czm_antialias(color1: vec4<f32>, color2: vec4<f32>, current_color: vec4<f32>, dist: f32, fuzz_factor: f32) -> vec4<f32> {
    var val1 = clamp(dist / fuzz_factor, 0.0, 1.0);
    let val2 = clamp((dist - 0.5) / fuzz_factor, 0.0, 1.0);
    val1 = val1 * (1.0 - val2);
    var v = val1 * val1 * (3.0 - (2.0 * val1));
    v = pow(v, 0.5); // makes the transition nicer
    let mid_color = (color1 + color2) * 0.5;
    return mix(mid_color, current_color, v);
}

// GLSL mod() (floor modulus); WGSL `%` is remainder, so implement mod faithfully.
fn glsl_mod(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}

// czm_gammaCorrect (Shaders/Builtin/Functions/gammaCorrect.glsl).
// The domain default is non-HDR, where this is the identity.
fn czm_gamma_correct(color: vec4<f32>) -> vec4<f32> {
    return color;
}

// ---------------------------------------------------------------------------
// Pattern helpers
// ---------------------------------------------------------------------------

// Fade material getTime() (Shaders/Materials/FadeMaterial.glsl)
fn fade_get_time(t: f32, coord: f32, maximum_distance: f32, repeat: bool) -> f32 {
    let scalar = 1.0 / maximum_distance;
    var q = distance(vec2<f32>(t, coord), vec2<f32>(0.0, 0.0)) * scalar;
    if repeat {
        let r = distance(vec2<f32>(t, coord + 1.0), vec2<f32>(0.0, 0.0)) * scalar;
        let s = distance(vec2<f32>(t, coord - 1.0), vec2<f32>(0.0, 0.0)) * scalar;
        q = min(min(r, s), q);
    }
    return clamp(q, 0.0, 1.0);
}

// ---------------------------------------------------------------------------
// Fragment entry
// ---------------------------------------------------------------------------

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // materialInput.st: use the mesh UVs when present, otherwise fall back to a
    // spherical parameterisation of the world position (unit-sphere showcase).
#ifdef VERTEX_UVS_A
    let st = in.uv;
#else
    let nrm = normalize(in.world_position.xyz);
    let st = vec2<f32>(
        atan2(nrm.z, nrm.x) * 0.15915494309 + 0.5,
        acos(clamp(nrm.y, -1.0, 1.0)) * 0.31830988618);
#endif

    // czm_getDefaultMaterial base values.
    var diffuse = vec3<f32>(0.0, 0.0, 0.0);
    var emission = vec3<f32>(0.0, 0.0, 0.0);
    var alpha = 1.0;

    switch params.kind {
        case 0u: { // Color
            let c = czm_gamma_correct(params.color_a);
            diffuse = c.rgb;
            alpha = c.a;
        }
        case 1u: { // Image
            let uv = fract(params.repeat_offset.xy * st);
            let tex = textureSample(image_texture, image_sampler, uv);
            let c = czm_gamma_correct(vec4<f32>(tex.rgb * params.color_c.rgb, tex.a * params.color_c.a));
            diffuse = c.rgb;
            alpha = c.a;
        }
        case 2u: { // Checkerboard
            let rpt = params.repeat_offset.xy;
            let b = glsl_mod(floor(rpt.x * st.x) + floor(rpt.y * st.y), 2.0); // 0.0 or 1.0
            var scaled_width = fract(rpt.x * st.x);
            scaled_width = abs(scaled_width - floor(scaled_width + 0.5));
            var scaled_height = fract(rpt.y * st.y);
            scaled_height = abs(scaled_height - floor(scaled_height + 0.5));
            let value = min(scaled_width, scaled_height);
            let current_color = mix(params.color_a, params.color_b, b);
            var color = czm_antialias(params.color_a, params.color_b, current_color, value, 0.03);
            color = czm_gamma_correct(color);
            diffuse = color.rgb;
            alpha = color.a;
        }
        case 3u: { // Stripe
            let horizontal_f = select(0.0, 1.0, params.horizontal == 1u);
            let coord = mix(st.x, st.y, horizontal_f);
            let offset = params.repeat_offset.z;
            let rpt = params.repeat_offset.x;
            let value = fract((coord - offset) * (rpt * 0.5));
            let dist = min(value, min(abs(value - 0.5), 1.0 - value));
            let current_color = mix(params.color_a, params.color_b, step(0.5, value));
            var color = czm_antialias(params.color_a, params.color_b, current_color, dist, 0.1);
            color = czm_gamma_correct(color);
            diffuse = color.rgb;
            alpha = color.a;
        }
        case 4u: { // Grid
            let line_count = params.line_params.xy;
            let line_thickness = params.line_params.zw;
            let line_offset = params.line_off_cell.xy;
            let cell_alpha = params.line_off_cell.z;
            let pixel_ratio = f32(params.pixel_ratio);

            var scaled_width = fract(line_count.x * st.x - line_offset.x);
            scaled_width = abs(scaled_width - floor(scaled_width + 0.5));
            var scaled_height = fract(line_count.y * st.y - line_offset.y);
            scaled_height = abs(scaled_height - floor(scaled_height + 0.5));

            // Derivatives branch (GLSL #if __VERSION__==300 path),Listing 4.13
            // from "3D Engine Design for Virtual Globes".
            let fuzz = 1.2;
            let thickness = (line_thickness * pixel_ratio) - vec2<f32>(1.0, 1.0);
            let dx = abs(dpdx(st));
            let dy = abs(dpdy(st));
            let d_f = vec2<f32>(max(dx.x, dy.x), max(dx.y, dy.y)) * line_count;
            var value = min(
                smoothstep(d_f.x * thickness.x, d_f.x * (fuzz + thickness.x), scaled_width),
                smoothstep(d_f.y * thickness.y, d_f.y * (fuzz + thickness.y), scaled_height));

            // Rim suppression (edges taken from RimLightingMaterial.glsl).
            let n = normalize(in.world_normal);
            let view_dir = normalize(mesh_view_bindings::view.world_position.xyz - in.world_position.xyz);
            let d_rim = 1.0 - abs(dot(n, view_dir));
            let s_rim = smoothstep(0.8, 1.0, d_rim);
            value = value * (1.0 - s_rim);

            var half_color = vec4<f32>(params.color_a.rgb * 0.5, 0.0);
            half_color.a = params.color_a.a * (1.0 - ((1.0 - cell_alpha) * value));
            half_color = czm_gamma_correct(half_color);
            diffuse = half_color.rgb;
            emission = half_color.rgb;
            alpha = half_color.a;
        }
        case 5u: { // Dot
            let rpt = params.repeat_offset.xy;
            let b = smoothstep(0.3, 0.32, length(fract(rpt * st) - vec2<f32>(0.5, 0.5))); // 0.0 or 1.0
            var color = mix(params.color_a, params.color_b, b);
            color = czm_gamma_correct(color);
            diffuse = color.rgb;
            alpha = color.a;
        }
        case 6u: { // Fade
            let max_dist = params.repeat_offset.w;
            let rep = params.repeat_flag == 1u;
            let s = fade_get_time(params.fade_dir_time.z, st.x, max_dist, rep) * params.fade_dir_time.x;
            let t = fade_get_time(params.fade_dir_time.w, st.y, max_dist, rep) * params.fade_dir_time.y;
            let u = length(vec2<f32>(s, t));
            var color = mix(params.color_a, params.color_b, u);
            color = czm_gamma_correct(color);
            emission = color.rgb;
            alpha = color.a;
        }
        default: {
            let c = czm_gamma_correct(params.color_a);
            diffuse = c.rgb;
            alpha = c.a;
        }
    }

    return vec4<f32>(diffuse + emission, alpha);
}
