// Fabric procedural material shader.
//
// This is the bevy-render adapter counterpart of the domain `cesium-material`
// crate. The domain layer assembles CesiumJS Fabric GLSL source text; this
// adapter renders the equivalent procedural patterns natively in WGSL so they
// run on Bevy's wgpu pipeline without a runtime GLSL->WGSL transpiler.
//
// Each pattern below is a faithful port of the corresponding CesiumJS material
// shader in `Source/Shaders/Materials/*.glsl`, including the shared built-ins
// `czm_antialias` and `czm_gammaCorrect` (non-HDR path, which is the identity).
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
    color_a: vec4<f32>, // light/even/color/fadeIn/base/waterColor
    color_b: vec4<f32>, // dark/odd/fadeOut/outlineColor/rimColor/landColor/gapColor
    color_c: vec4<f32>, // Image tint (color)
    repeat_offset: vec4<f32>, // x=repeat.x, y=repeat.y, z=offset(stripe), w=maximumDistance(fade)
    line_params: vec4<f32>,   // x=lineCount.x, y=lineCount.y, z=lineThickness.x, w=lineThickness.y
    line_off_cell: vec4<f32>, // x=lineOffset.x, y=lineOffset.y, z=cellAlpha, w=(spare)
    fade_dir_time: vec4<f32>, // x=fadeDirection.x, y=fadeDirection.y, z=time.x, w=time.y
    // --- Extension fields for materials 7–20 ---
    // x=glowPower, y=taperPower, z=outlineWidth/rimWidth, w=dashLength
    extra_a: vec4<f32>,
    // x=spacing(contour), y=contourWidth, z=strength(normal/bump), w=dashPattern
    extra_b: vec4<f32>,
    // x=minHeight(ramp), y=maxHeight(ramp), z=time(water), w=animationSpeed
    extra_c: vec4<f32>,
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
        // ===================================================================
        // New material types (7–20)
        // ===================================================================

        case 7u: { // PolylineArrow (PolylineArrowMaterial.glsl)
            let px_ratio = f32(params.pixel_ratio);
            let fw_st = abs(dpdx(st.x)) + abs(dpdy(st.x));
            let base = 1.0 - fw_st * 10.0 * px_ratio;
            // getPointOnLine for upper arrow edge: (base,1.0) -> center(1.0,0.5)
            let slope_upper = 0.5 / (base - 1.0);
            let pt_on_upper = slope_upper * (st.x - base) + 1.0;
            // getPointOnLine for lower arrow edge: (base,0.0) -> center(1.0,0.5)
            let slope_lower = 0.5 / (base - 1.0);
            let pt_on_lower = slope_lower * (st.x - base);

            let half_width = 0.15;
            var s = step(0.5 - half_width, st.y);
            s *= 1.0 - step(0.5 + half_width, st.y);
            s *= 1.0 - step(base, st.x);

            var t = step(base, st.x);
            t *= 1.0 - step(pt_on_upper, st.y);
            t *= step(pt_on_lower, st.y);

            var dist: f32;
            if st.x < base {
                let d1 = abs(st.y - (0.5 - half_width));
                let d2 = abs(st.y - (0.5 + half_width));
                dist = min(d1, d2);
            } else {
                let d1 = select(1e10, abs(st.x - base), st.y < 0.5 - half_width || st.y > 0.5 + half_width);
                let d2 = abs(st.y - pt_on_upper);
                let d3 = abs(st.y - pt_on_lower);
                dist = min(min(d1, d2), d3);
            }

            let current_color = mix(vec4<f32>(0.0), params.color_a, clamp(s + t, 0.0, 1.0));
            var color = czm_antialias(vec4<f32>(0.0), params.color_a, current_color, dist, 0.03);
            color = czm_gamma_correct(color);
            diffuse = color.rgb;
            alpha = color.a;
        }

        case 8u: { // PolylineDash (PolylineDashMaterial.glsl)
            let dash_length = params.extra_a.w;
            let dash_pattern = params.extra_b.w;
            let px_ratio = f32(params.pixel_ratio);

            // Compute line direction from screen-space derivatives of st
            let dsd = vec2<f32>(dpdx(st).x, dpdy(st).x);
            let angle = atan2(dsd.y, dsd.x);
            let c = cos(-angle);
            let ss = sin(-angle);
            let rot_x = c * st.x + ss * st.y;

            // Dash pattern: 16-bit mask
            let dash_pos = fract(rot_x / (dash_length * px_ratio));
            let mask_idx = floor(dash_pos * 16.0);
            let mask_test = floor(dash_pattern / pow(2.0, mask_idx));
            let on = mod(mask_test, 2.0) >= 1.0;

            let frag_color = select(params.color_b, params.color_a, on);
            if frag_color.a < 0.005 { discard; }
            let c2 = czm_gamma_correct(frag_color);
            emission = c2.rgb;
            alpha = c2.a;
        }

        case 9u: { // PolylineGlow (PolylineGlowMaterial.glsl)
            let glow_power = params.extra_a.x;
            let taper_power = params.extra_a.y;
            var glow = glow_power / abs(st.y - 0.5) - (glow_power / 0.5);

            if taper_power <= 0.99999 {
                glow *= min(1.0, taper_power / (0.5 - st.x * 0.5) - (taper_power / 0.5));
            }

            var frag_color: vec4<f32>;
            frag_color.rgb = max(vec3<f32>(glow - 1.0 + params.color_a.rgb), params.color_a.rgb);
            frag_color.a = clamp(glow, 0.0, 1.0) * params.color_a.a;
            frag_color = czm_gamma_correct(frag_color);
            emission = frag_color.rgb;
            alpha = frag_color.a;
        }

        case 10u: { // PolylineOutline (PolylineOutlineMaterial.glsl)
            let width = params.extra_a.z; // outlineWidth
            // v_width is approximated; in CesiumJS it comes from the vertex shader.
            // We use 1.0 as a default polyline width.
            let v_width = 1.0;
            let half_interior = 0.5 * (v_width - width) / v_width;
            let b = step(0.5 - half_interior, st.y);
            let b2 = b * (1.0 - step(0.5 + half_interior, st.y));

            let d1 = abs(st.y - (0.5 - half_interior));
            let d2 = abs(st.y - (0.5 + half_interior));
            let dist = min(d1, d2);

            let current_color = mix(params.color_b, params.color_a, b2);
            var out_color = czm_antialias(params.color_b, params.color_a, current_color, dist, 0.03);
            out_color = czm_gamma_correct(out_color);
            diffuse = out_color.rgb;
            alpha = out_color.a;
        }

        case 11u: { // ElevationContour (ElevationContourMaterial.glsl)
            // Use world_position.y as proxy for materialInput.height
            let height = in.world_position.y;
            let spacing = params.extra_b.x;
            let contour_w = params.extra_b.y;
            let px_ratio = f32(params.pixel_ratio);

            let distance_to_contour = glsl_mod(height, spacing);

            let dxc = abs(dpdx(height));
            let dyc = abs(dpdy(height));
            let d_f = max(dxc, dyc) * px_ratio * contour_w;
            let contour_a = select(0.0, 1.0, distance_to_contour < d_f);

            let out_color = czm_gamma_correct(vec4<f32>(params.color_a.rgb, contour_a * params.color_a.a));
            diffuse = out_color.rgb;
            alpha = out_color.a;
        }

        case 12u: { // ElevationRamp (ElevationRampMaterial.glsl)
            let height = in.world_position.y;
            let min_h = params.extra_c.x;
            let max_h = params.extra_c.y;
            let scaled = clamp((height - min_h) / (max_h - min_h), 0.0, 1.0);
            let ramp_color = textureSample(image_texture, image_sampler, vec2<f32>(scaled, 0.5));
            let c = czm_gamma_correct(ramp_color);
            diffuse = c.rgb;
            alpha = c.a;
        }

        case 13u: { // AspectRamp (AspectRampMaterial.glsl)
            let nrm = normalize(in.world_normal);
            let aspect = atan2(-nrm.z, nrm.x); // range [-PI, PI]
            let aspect_norm = aspect * 0.15915494309 + 0.5; // map to [0, 1]
            let ramp_color = textureSample(image_texture, image_sampler, vec2<f32>(aspect_norm, 0.5));
            let c = czm_gamma_correct(ramp_color);
            diffuse = c.rgb;
            alpha = c.a;
        }

        case 14u: { // SlopeRamp (SlopeRampMaterial.glsl)
            let nrm = normalize(in.world_normal);
            let up = vec3<f32>(0.0, 1.0, 0.0);
            let slope_angle = acos(clamp(abs(dot(nrm, up)), 0.0, 1.0));
            let slope_norm = slope_angle / 1.57079632679; // divide by PI/2
            let ramp_color = textureSample(image_texture, image_sampler, vec2<f32>(slope_norm, 0.5));
            let c = czm_gamma_correct(ramp_color);
            diffuse = c.rgb;
            alpha = c.a;
        }

        case 15u: { // NormalMap (NormalMapMaterial.glsl)
            let strength = params.extra_b.z;
            let repeat = params.repeat_offset.xy;
            let tex_val = textureSample(image_texture, image_sampler, fract(repeat * st));
            var nts = tex_val.rgb;
            nts.xy = nts.xy * 2.0 - 1.0;
            nts.z = clamp(1.0 - strength, 0.1, 1.0);
            nts = normalize(nts);
            // Approximate eye-space normal: use world_normal as TBN basis
            let wn = normalize(in.world_normal);
            diffuse = wn * 0.5 + 0.5; // visualize normal
            alpha = 1.0;
        }

        case 16u: { // BumpMap (BumpMapMaterial.glsl)
            let strength = params.extra_b.z;
            let repeat = params.repeat_offset.xy;
            let center_uv = fract(repeat * st);
            let center_bump = textureSample(image_texture, image_sampler, center_uv).r;

            // Simple finite difference for bump
            let dx_uv = fract(repeat * (st + vec2<f32>(0.001, 0.0)));
            let right_bump = textureSample(image_texture, image_sampler, dx_uv).r;
            let dy_uv = fract(repeat * (st + vec2<f32>(0.0, 0.001)));
            let top_bump = textureSample(image_texture, image_sampler, dy_uv).r;

            let nts = normalize(vec3<f32>(
                center_bump - right_bump,
                center_bump - top_bump,
                clamp(1.0 - strength, 0.1, 1.0)));
            let wn = normalize(in.world_normal);
            diffuse = nts * 0.5 + 0.5; // visualize perturbed normal
            alpha = 1.0;
        }

        case 17u: { // Water (Water.glsl — simplified stub)
            let time = params.extra_c.z;
            let speed = params.extra_c.w;
            let t = time * speed;

            // Simple animated wave effect using sinusoidal displacement
            let wave1 = sin(st.x * 10.0 + t) * cos(st.y * 8.0 + t * 0.7) * 0.15;
            let wave2 = sin(st.x * 15.0 - t * 0.6) * sin(st.y * 12.0 + t * 0.8) * 0.1;
            let wave = wave1 + wave2;

            let water_color = params.color_a;
            let blend_color = params.color_b;
            let specular = clamp(wave + 0.3, 0.0, 1.0);

            var frag = mix(blend_color, water_color, specular);
            frag.rgb += 0.05 * wave;
            frag = czm_gamma_correct(frag);
            diffuse = frag.rgb;
            alpha = frag.a;
        }

        case 18u: { // RimLighting (RimLightingMaterial.glsl)
            let rim_width = params.extra_a.z;
            let wn = normalize(in.world_normal);
            let view_dir = normalize(mesh_view_bindings::view.world_position.xyz - in.world_position.xyz);
            let d = 1.0 - abs(dot(wn, view_dir));
            let s = smoothstep(1.0 - rim_width, 1.0, d);

            let out_color = czm_gamma_correct(params.color_a);
            let out_rim = czm_gamma_correct(params.color_b);

            diffuse = out_color.rgb;
            emission = out_rim.rgb * s;
            alpha = mix(out_color.a, out_rim.a, s);
        }

        case 19u: { // ElevationBand (ElevationBandMaterial.glsl — simplified stub)
            let height = in.world_position.y;
            let min_h = params.extra_c.x;
            let max_h = params.extra_c.y;
            let scaled = clamp((height - min_h) / (max_h - min_h), 0.0, 1.0);

            // Discrete bands: quantize height into steps
            let num_bands = 5.0;
            let band = floor(scaled * num_bands) / num_bands;
            let ramp_color = textureSample(image_texture, image_sampler, vec2<f32>(band, 0.5));
            let c = czm_gamma_correct(ramp_color);
            diffuse = c.rgb;
            alpha = c.a;
        }

        case 20u: { // WaterMask (WaterMaskMaterial.glsl)
            // Use height threshold as proxy for waterMask
            let height = in.world_position.y;
            let water_level = params.extra_c.x;
            let water_mask = smoothstep(water_level - 0.1, water_level + 0.1, height);
            let out_color = mix(params.color_a, params.color_b, water_mask);
            let c = czm_gamma_correct(out_color);
            diffuse = c.rgb;
            alpha = c.a;
        }

        default: {
            let c = czm_gamma_correct(params.color_a);
            diffuse = c.rgb;
            alpha = c.a;
        }
    }

    return vec4<f32>(diffuse + emission, alpha);
}
