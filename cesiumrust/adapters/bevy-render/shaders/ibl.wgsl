// M6.5: cesiumrust Image-Based Lighting (IBL) — WGSL environment PBR.
// ============================================================================
// Faithful port of the upstream CesiumJS image-based-lighting shaders:
//   • packages/engine/Source/Shaders/Builtin/Functions/sphericalHarmonics.glsl
//       — czm_sphericalHarmonics: PRE-SCALED coefficient convention, evaluation is
//         the bare polynomial in (x,y,z) followed by `max(L, vec3(0.0))`.
//   • packages/engine/Source/Shaders/Builtin/Functions/pbrLighting.glsl
//       — GGX / smithVisibilityGGX / fresnelSchlick2 (versine^5 expanded).
//   • packages/engine/Source/Shaders/BrdfLutGeneratorFS.glsl
//       — integrateBrdf(roughness, NdotV): split-sum (scale, bias), 1024 samples.
//   • packages/engine/Source/Shaders/ConvolveSpecularMapFS.glsl
//       — prefilter: GGX importance sampling + NdotL-weighted average, 128 samples.
//   • packages/engine/Source/Shaders/Model/ImageBasedLightingStageFS.glsl
//       — textureIBL: Fdez-Aguera single- + multi-scattering environment BRDF.
//   Blueprint (踩坑参考): cesium-rs/crates/cesium-shaders.
//
// The WGSL below mirrors `domain/effects/src/ibl.rs` entry-for-entry so the CPU
// f64 reference and the GPU f32 shader can be cross-validated (same polynomial
// basis, same importance sampling, same UNFUSED Fresnel pow5).
//
// Four fragment entry points share this module:
//   1. fragment              — SCREEN-SPACE IBL apply node (IblNode, effects/ibl.rs):
//                              reconstructs world position + normal from the depth /
//                              normal prepass, evaluates `texture_ibl` and adds the
//                              environment lighting onto the (HDR) scene colour.
//   2. brdf_lut_fragment     — OFFLINE generator: split-sum BRDF integration LUT
//                              indexed by (NdotV, roughness), ≡ BrdfLutGeneratorFS.
//   3. irradiance_fragment   — OFFLINE generator: SH irradiance for the equirect
//                              direction from uv, ≡ czm_sphericalHarmonics.
//   4. prefilter_fragment    — OFFLINE generator: GGX-prefiltered specular radiance
//                              for the equirect direction from uv, ≡ ConvolveSpecularMapFS.
//   The three generators produce the textures/LUTs a production material path binds;
//   the apply node recomputes the BRDF split-sum inline (see DEVIATION) so this file
//   is self-contained. Integration task #81 owns the faithful forward wiring
//   (`texture_ibl` #import-ed into the globe / tileset / fabric fragment shaders,
//   exactly where upstream `textureIBL` runs) and the Core3d graph edges.
//
// # RED LINES (hard constraints)
//   • domain f64 → WGSL f32 boundary: every value here is f32; the domain SH
//     coefficients / material params stay metric f64 and are projected to f32 only
//     in `IblUniform::from_domain` (effects/ibl.rs).
//   • NO FMA CONTRACTION: the IBL numerics are integration-sensitive, so `a*b+c`
//     is kept as two IEEE roundings. The reflection vector is built as an explicit
//     `n * (2.0*dot(n,v))` followed by a SEPARATE `- v`; the Fresnel pow5 is the
//     UNFUSED expansion `vs2*vs2*versine`; no `fma()` builtin is used anywhere.
//   • `mod` is a WGSL RESERVED WORD (the M5-D C1 lesson) — the van der Corput
//     radical inverse uses integer bit ops (`i & 1u`, `i >> 1u`), never float mod.
//   • glam fast-math is disabled repo-wide; nothing here relies on non-IEEE floats.
//   • Environment COLOUR maps are sRGB (sampled as Rgba8UnormSrgb on the CPU side);
//     the LUT / prefilter outputs written here are LINEAR data (Rgba8Unorm /
//     Rgba16Float) — never sRGB — matching the normalMap/specularMap red line.
//
// # DEVIATION (docs/deviations.md#dev-024)
//   Upstream `textureIBL` runs inside the model material fragment stage, reading
//   PER-FRAGMENT albedo / roughness / specular F0 from the glTF material and a
//   PRECOMPUTED 256×256 `czm_brdfLut` texture. This screen-space apply node instead
//   (a) reconstructs only geometry (world position + normal) from the prepass and
//   reads material params from a UNIFORM (single-material scene), and (b) recomputes
//   the split-sum BRDF inline (`integrate_brdf`, BRDF_APPLY_SAMPLES taps) rather
//   than binding a LUT texture, so the file is self-contained without an offline
//   LUT-generation subsystem. Both are gated OFF by default (pixel-neutral); the
//   faithful per-fragment path is the `texture_ibl` #import wiring into the model
//   material shaders, which task #81 deliberately did NOT do (it owns only the
//   shared render graph) — tracked as deferred #53.
//
// Bevy reversed-Z: Bevy 0.15.3 uses an infinite-far reversed-Z depth buffer, so
// SKY / far plane == depth 0.0 and the near plane == 1.0. The apply node therefore
// early-outs on `depth <= 1e-6` (nothing to light in the sky), matching ao.wgsl.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View

// ─── Apply-node bindings (group 0, entry `fragment`) ─────────────────────────
// Superset layout shared by all four entries: a generator entry statically uses a
// subset, and wgpu permits a pipeline layout to declare MORE bindings than an entry
// references, so one 8-binding layout covers every entry (see IblPipeline::from_world).
@group(0) @binding(0) var depth_prepass: texture_depth_2d;        // world-position reconstruction
@group(0) @binding(1) var normal_prepass: texture_2d<f32>;        // world normal (encoded n*0.5+0.5)
@group(0) @binding(2) var color_texture: texture_2d<f32>;         // HDR scene colour (post-process source)
@group(0) @binding(3) var point_sampler: sampler;                 // NonFiltering (depth + normal)
@group(0) @binding(4) var linear_sampler: sampler;                // Filtering (colour + cubemap)
@group(0) @binding(5) var<uniform> view: View;                    // dynamic offset per view
@group(0) @binding(6) var<uniform> ibl: IblData;                  // SH + material + factor
@group(0) @binding(7) var radiance_cubemap: texture_cube<f32>;    // (pre)filtered specular environment

// ─── IBL uniform (group 0, binding 6) ────────────────────────────────────────
// Packed on the CPU by IblUniform::from_domain (effects/ibl.rs). Layout MUST match
// that Rust `#[derive(ShaderType)]` struct field-for-field (all vec4 for alignment).
struct IblData {
    // 9 CesiumJS-convention (PRE-SCALED) SH irradiance coefficients; rgb in .xyz.
    sh: array<vec4<f32>, 9>,
    // x = perceptual roughness, yzw = specular F0 reflectance (linear rgb).
    material: vec4<f32>,
    // xyz = lambertian base colour (linear rgb), w = specular weight.
    diffuse: vec4<f32>,
    // x = diffuse IBL factor, y = specular IBL factor, z = cubemap max LOD, w = pad.
    params: vec4<f32>,
};

// ─── Constants ───────────────────────────────────────────────────────────────
const IBL_PI: f32 = 3.14159265358979;
const IBL_TWO_PI: f32 = 6.28318530717959;
// Faithful generator sample counts (≡ upstream BrdfLutGeneratorFS / ConvolveSpecularMapFS).
const BRDF_LUT_SAMPLES: i32 = 1024;
const PREFILTER_SAMPLES: i32 = 128;
// Runtime apply cost: fewer taps for the inline per-fragment split-sum (see DEVIATION).
const BRDF_APPLY_SAMPLES: i32 = 32;

// ─── Spherical harmonics (≡ czm_sphericalHarmonics) ──────────────────────────
// The 9 polynomial basis terms P_i(x,y,z) in upstream coefficient order
// [L00, L1_1, L10, L11, L2_2, L2_1, L20, L21, L22]. No normalisation constants:
// the coefficients are PRE-SCALED (cmgen bakes the basis constants + cosine-lobe
// irradiance transfer in), so evaluation is the bare polynomial.
fn sh_polynomial_basis(dir: vec3<f32>) -> array<f32, 9> {
    let x = dir.x;
    let y = dir.y;
    let z = dir.z;
    return array<f32, 9>(
        1.0,               // L00
        y,                 // L1_1
        z,                 // L10
        x,                 // L11
        y * x,             // L2_2
        y * z,             // L2_1
        3.0 * z * z - 1.0, // L20
        z * x,             // L21
        x * x - y * y,     // L22
    );
}

// max(Σ c_i · P_i(dir), 0) per RGB channel — mirrors sphericalHarmonics.glsl's
// `max(L, vec3(0.0))` clamp (negative irradiance is unphysical).
fn spherical_harmonics_eval(coeffs: array<vec4<f32>, 9>, dir: vec3<f32>) -> vec3<f32> {
    let basis = sh_polynomial_basis(dir);
    var out = vec3<f32>(0.0, 0.0, 0.0);
    for (var i = 0; i < 9; i = i + 1) {
        out = out + coeffs[i].xyz * basis[i];
    }
    return max(out, vec3<f32>(0.0, 0.0, 0.0));
}

// ─── Low-discrepancy sampling (≡ hammersley2D / vdcRadicalInverse) ───────────
// Integer bit ops ONLY — no float `mod` (WGSL reserved word, M5-D C1 lesson).
fn radical_inverse_vdc(bits_in: u32) -> f32 {
    var i = bits_in;
    var value = 0.0;
    var inv_bi = 0.5;
    for (var k = 0; k < 32; k = k + 1) {
        if i == 0u {
            break;
        }
        value = value + f32(i & 1u) * inv_bi;
        inv_bi = inv_bi * 0.5;
        i = i >> 1u;
    }
    return value;
}

fn hammersley2d(i: i32, n: i32) -> vec2<f32> {
    return vec2<f32>(f32(i) / f32(max(n, 1)), radical_inverse_vdc(u32(i)));
}

// ─── PBR microfacet terms (≡ pbrLighting.glsl) ───────────────────────────────
// Smith joint GGX visibility = G / (4·NdotL·NdotV).
fn smith_visibility_ggx(alpha_roughness: f32, ndotl: f32, ndotv: f32) -> f32 {
    let a2 = alpha_roughness * alpha_roughness;
    let ggxv = ndotl * sqrt(max(ndotv * ndotv * (1.0 - a2) + a2, 0.0));
    let ggxl = ndotv * sqrt(max(ndotl * ndotl * (1.0 - a2) + a2, 0.0));
    let ggx = ggxv + ggxl;
    if ggx > 0.0 {
        return 0.5 / ggx;
    }
    return 0.0;
}

// Roughness-dependent Schlick Fresnel. The versine^5 is expanded as vs2*vs2*versine
// and kept UNFUSED (NO FMA contraction — IBL numerics red line).
fn fresnel_schlick2(f0: vec3<f32>, f90: vec3<f32>, vdoth: f32) -> vec3<f32> {
    let versine = 1.0 - vdoth;
    let vs2 = versine * versine;
    let pow5 = vs2 * vs2 * versine;
    return f0 + (f90 - f0) * pow5;
}

// GGX importance sample: the world-space half-vector H for a 2D quasi-random xi.
// Squares `alpha_roughness` internally (≡ importanceSampleGGX); call sites feed
// different arguments (prefilter passes raw roughness, integrate_brdf passes
// roughness²) exactly as the two upstream GLSL sites do.
fn importance_sample_ggx(xi: vec2<f32>, alpha_roughness: f32, n: vec3<f32>) -> vec3<f32> {
    let a2 = alpha_roughness * alpha_roughness;
    let phi = IBL_TWO_PI * xi.x;
    let denom = 1.0 + (a2 - 1.0) * xi.y;
    var cos_theta = 1.0;
    if denom > 0.0 {
        cos_theta = sqrt(max((1.0 - xi.y) / denom, 0.0));
    }
    let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
    let h = vec3<f32>(sin_theta * cos(phi), sin_theta * sin(phi), cos_theta);
    var up = vec3<f32>(0.0, 0.0, 1.0);
    if abs(n.z) >= 0.999 {
        up = vec3<f32>(1.0, 0.0, 0.0);
    }
    let tangent_x = normalize(cross(up, n));
    let tangent_y = cross(n, tangent_x);
    return tangent_x * h.x + tangent_y * h.y + n * h.z;
}

// Split-sum environment-BRDF integration → (scale, bias), indexed by (NdotV,
// roughness) ≡ BrdfLutGeneratorFS.glsl::integrateBrdf.
fn integrate_brdf(roughness: f32, ndotv_in: f32, samples: i32) -> vec2<f32> {
    let ndotv = clamp(ndotv_in, 0.0, 1.0);
    let v = vec3<f32>(sqrt(max(1.0 - ndotv * ndotv, 0.0)), 0.0, ndotv);
    let alpha_roughness = roughness * roughness;
    let n = max(samples, 1);
    var a = 0.0;
    var b = 0.0;
    for (var i = 0; i < n; i = i + 1) {
        let xi = hammersley2d(i, n);
        let h = importance_sample_ggx(xi, alpha_roughness, vec3<f32>(0.0, 0.0, 1.0));
        // NO FMA: `h * (2.0*dot(v,h))` then a SEPARATE `- v` (two roundings).
        let l = h * (2.0 * dot(v, h)) - v;
        let ndotl = clamp(l.z, 0.0, 1.0);
        let ndoth = clamp(h.z, 0.0, 1.0);
        let vdoth = clamp(dot(v, h), 0.0, 1.0);
        if ndotl > 0.0 && ndoth > 0.0 {
            let g = smith_visibility_ggx(alpha_roughness, ndotl, ndotv);
            let g_vis = 4.0 * g * vdoth * ndotl / ndoth;
            // Fc = (1 - VdotH)^5, UNFUSED pow5 expansion.
            let versine = 1.0 - vdoth;
            let vs2 = versine * versine;
            let fc = vs2 * vs2 * versine;
            a = a + (1.0 - fc) * g_vis;
            b = b + fc * g_vis;
        }
    }
    return vec2<f32>(a / f32(n), b / f32(n));
}

// GGX-prefiltered specular radiance ≡ ConvolveSpecularMapFS.glsl: importance-sample
// the environment, weight each tap by NdotL, normalise by the accumulated weight.
fn prefilter_specular(roughness: f32, dir: vec3<f32>, samples: i32) -> vec3<f32> {
    let v = normalize(dir);
    let n = max(samples, 1);
    var color = vec3<f32>(0.0, 0.0, 0.0);
    var weight = 0.0;
    for (var i = 0; i < n; i = i + 1) {
        let xi = hammersley2d(i, n);
        // ConvolveSpecularMapFS passes RAW roughness (squared inside importance_sample_ggx).
        let h = importance_sample_ggx(xi, roughness, v);
        let l = h * (2.0 * dot(v, h)) - v;
        let ndotl = max(dot(v, l), 0.0);
        if ndotl > 0.0 {
            // FIX-IBL-LOD: LOD is hardcoded to 0 because the prefilter output
            // texture is currently a 1×1×6 mip=1 neutral placeholder (see
            // `effects/ibl.rs`), so there is no mip chain to select from and LOD 0
            // is equivalent to every other level. Roughness→mip selection
            // (`lod = roughness * (mip_count - 1)`) lands with the real GPU
            // prefilter driver — deferred #54.
            let s = textureSampleLevel(radiance_cubemap, linear_sampler, normalize(l), 0.0).rgb;
            color = color + s * ndotl;
            weight = weight + ndotl;
        }
    }
    if weight > 0.0 {
        return color / weight;
    }
    return vec3<f32>(0.0, 0.0, 0.0);
}

// Equirectangular direction from a [0,1]² uv (offline generators render to a 2D
// target). Cesium world space is **z-up** — latitude rides the `z` component via
// `asin`, longitude is `atan2(y, x)` — matching `direction_to_equirect_uv` in
// `panorama.wgsl` so both equirect maps agree on one convention. The old form put
// `sin(theta)` in `y` (y-up), disagreeing with the rest of the engine.
// FIX-IBL-ZUP; f64 twin in `domain/effects/src/ibl.rs::direction_from_uv`.
fn direction_from_uv(uv: vec2<f32>) -> vec3<f32> {
    let lon = IBL_TWO_PI * (uv.x - 0.5); // [-π, π]
    let lat = IBL_PI * (uv.y - 0.5);     // [-π/2, π/2]
    let cos_lat = cos(lat);
    return normalize(vec3<f32>(cos_lat * cos(lon), cos_lat * sin(lon), sin(lat)));
}

// ─── textureIBL (≡ ImageBasedLightingStageFS.glsl, Fdez-Aguera) ──────────────
// Full image-based-lighting contribution: diffuse from the SH irradiance with
// multi-scattering energy compensation, specular from the (pre)filtered environment
// modulated by the split-sum BRDF. `reflect_env` is the specular radiance already
// sampled at the reflection direction / roughness-appropriate LOD by the caller.
fn texture_ibl(
    normal: vec3<f32>,
    view_dir: vec3<f32>,
    roughness: f32,
    f0: vec3<f32>,
    diffuse_color: vec3<f32>,
    specular_weight: f32,
    ibl_factor: vec2<f32>,
    reflect_env: vec3<f32>,
) -> vec3<f32> {
    let n = normalize(normal);
    let v = normalize(view_dir);
    let ndotv = clamp(dot(n, v), 0.0, 1.0);

    // Roughness-dependent Fresnel (Fdez-Aguera): f90 = max(1 - roughness, f0).
    let one_minus_r = 1.0 - roughness;
    let f90 = max(vec3<f32>(one_minus_r, one_minus_r, one_minus_r), f0);
    let single_fresnel = fresnel_schlick2(f0, f90, ndotv);

    let brdf = integrate_brdf(roughness, ndotv, BRDF_APPLY_SAMPLES);
    let lut_scale = brdf.x;
    let lut_bias = brdf.y;

    // FssEss = specularWeight · (F · scale + bias), per channel.
    let fss_ess = specular_weight * (single_fresnel * lut_scale + lut_bias);

    // Diffuse irradiance from the SH coefficients.
    let irradiance = spherical_harmonics_eval(ibl.sh, n);

    // Multi-scattering energy compensation (Fdez-Aguera / Kulla-Conty average).
    let average_fresnel = f0 + (vec3<f32>(1.0, 1.0, 1.0) - f0) / 21.0;
    let ems = specular_weight * (1.0 - lut_scale - lut_bias);
    let num = fss_ess * average_fresnel * ems;
    let denom = vec3<f32>(1.0, 1.0, 1.0) - average_fresnel * ems;
    // Per-channel guard matching the domain reference (denom ≥ 0 here; the domain
    // returns 0 when |denom| ≤ 1e-12). `select` evaluates both arms but the inf from
    // a near-zero denominator is discarded, never propagated into the result.
    let fms_ems = vec3<f32>(
        select(0.0, num.x / denom.x, abs(denom.x) > 1e-12),
        select(0.0, num.y / denom.y, abs(denom.y) > 1e-12),
        select(0.0, num.z / denom.z, abs(denom.z) > 1e-12),
    );
    let dielectric = (vec3<f32>(1.0, 1.0, 1.0) - fss_ess - fms_ems) * diffuse_color;
    var out = irradiance * (fms_ems + dielectric) * ibl_factor.x;

    // Specular: environment radiance at the reflection direction.
    out = out + reflect_env * fss_ess * ibl_factor.y;
    return out;
}

// ─── Prepass reconstruction (mirrors ao.wgsl) ────────────────────────────────
fn reconstruct_world_position(depth: f32, uv: vec2<f32>) -> vec3<f32> {
    let clip_xy = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - 2.0 * uv.y);
    let t = view.view_from_clip * vec4<f32>(clip_xy, depth, 1.0);
    let view_pos = t.xyz / t.w;
    let w = view.world_from_view * vec4<f32>(view_pos, 1.0);
    return w.xyz;
}

// `normalize` with a NaN guard (copied from `panorama.wgsl` `safe_normalize`):
// returns `fallback` when `value` is effectively zero. `normalize(vec3(0))` is
// `0/0 = NaN`, and a normal prepass that was never written (sky pixels / first
// frame) yields exactly that, poisoning the whole frame through `texture_ibl`.
// FIX-IBL-NORMALGUARD. (`mod` is a WGSL reserved word — helpers avoid it.)
fn safe_normalize(value: vec3<f32>, fallback: vec3<f32>) -> vec3<f32> {
    if (dot(value, value) <= 1.0e-24) {
        return fallback;
    }
    return normalize(value);
}

fn load_world_normal(uv: vec2<f32>) -> vec3<f32> {
    // Bevy stores *world* normals encoded as (n*0.5+0.5); whole-var reassign (no
    // swizzle assignment — naga rejects `v.xyz = ...`).
    var world_normal = textureSampleLevel(normal_prepass, point_sampler, uv, 0.0).xyz;
    world_normal = (world_normal * 2.0) - 1.0;
    // Guard against an unwritten (zero) normal prepass → NaN. Fallback is the
    // world up axis; a degenerate pixel contributes a neutral, finite term.
    return safe_normalize(world_normal, vec3<f32>(0.0, 0.0, 1.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// Entry 1 — SCREEN-SPACE IBL APPLY NODE (IblNode, effects/ibl.rs)
// ═══════════════════════════════════════════════════════════════════════════
@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let color = textureSampleLevel(color_texture, linear_sampler, uv, 0.0);

    // Gate / disable: both IBL factors zero ⇒ pure pass-through (pixel-neutral).
    if ibl.params.x <= 0.0 && ibl.params.y <= 0.0 {
        return color;
    }

    let depth = textureSampleLevel(depth_prepass, point_sampler, uv, 0.0);
    // Reversed-Z: sky / infinite far plane == 0.0 ⇒ nothing to light.
    if depth <= 1.0e-6 {
        return color;
    }

    let world_pos = reconstruct_world_position(depth, uv);
    let world_normal = load_world_normal(uv);
    // Camera world position = translation column of world_from_view.
    let cam_pos = view.world_from_view[3].xyz;
    let view_dir = normalize(cam_pos - world_pos);

    // Reflection direction: NO FMA — explicit scale then a separate subtract.
    let ndotv_raw = dot(world_normal, view_dir);
    let scaled = world_normal * (2.0 * ndotv_raw);
    let reflect_dir = normalize(scaled - view_dir);

    // Sample the (pre)filtered environment at the roughness-appropriate LOD.
    let lod = ibl.material.x * ibl.params.z;
    let reflect_env = textureSampleLevel(radiance_cubemap, linear_sampler, reflect_dir, lod).rgb;

    let ibl_light = texture_ibl(
        world_normal,
        view_dir,
        ibl.material.x,
        ibl.material.yzw,
        ibl.diffuse.xyz,
        ibl.diffuse.w,
        ibl.params.xy,
        reflect_env,
    );

    // Additive environment lighting onto the (HDR) scene colour.
    return vec4<f32>(color.rgb + ibl_light, color.a);
}

// ═══════════════════════════════════════════════════════════════════════════
// Entry 2 — BRDF integration LUT generator (≡ BrdfLutGeneratorFS.glsl)
// ═══════════════════════════════════════════════════════════════════════════
// uv.x = NdotV, uv.y = roughness (matches texture(czm_brdfLut, vec2(NdotV, roughness))).
// NOTE (FIX-IBL-GENERATOR): entries 2–4 below (brdf_lut_fragment,
// irradiance_fragment, prefilter_fragment) are the offline / GPU generator pass.
// They are declared here and pinned by the naga entry-point assertion in
// `effects/ibl.rs`, but are NOT yet driven by any Rust render pipeline — the
// apply node (entry 1) is the only wired path. The full generator pipeline is
// deferred #54; do not assume these run at frame time.
@fragment
fn brdf_lut_fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let ndotv = in.uv.x;
    let roughness = in.uv.y;
    let sb = integrate_brdf(roughness, ndotv, BRDF_LUT_SAMPLES);
    return vec4<f32>(sb.x, sb.y, 0.0, 1.0);
}

// ═══════════════════════════════════════════════════════════════════════════
// Entry 3 — SH irradiance generator (≡ czm_sphericalHarmonics)
// ═══════════════════════════════════════════════════════════════════════════
@fragment
fn irradiance_fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let dir = direction_from_uv(in.uv);
    let irr = spherical_harmonics_eval(ibl.sh, dir);
    return vec4<f32>(irr, 1.0);
}

// ═══════════════════════════════════════════════════════════════════════════
// Entry 4 — Prefiltered specular generator (≡ ConvolveSpecularMapFS.glsl)
// ═══════════════════════════════════════════════════════════════════════════
@fragment
fn prefilter_fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let dir = direction_from_uv(in.uv);
    let radiance = prefilter_specular(ibl.material.x, dir, PREFILTER_SAMPLES);
    return vec4<f32>(radiance, 1.0);
}
