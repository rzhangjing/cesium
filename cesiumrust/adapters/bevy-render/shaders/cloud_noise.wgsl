// M6.6: cesiumrust Clouds — Perlin-Worley 3D noise generation (compute pass).
// ============================================================================
// Faithful port of the upstream CesiumJS cloud-noise generator:
//   • packages/engine/Source/Shaders/CloudNoiseFS.glsl L1-92 (3122 B)
//       — random3 / getWorleyCellPoint / worleyNoise / worleyFBMNoise / main.
//         Upstream renders this as a FULLSCREEN FRAGMENT pass into a 2D atlas
//         (4096 × 512 RGBA8 = a 128³ volume packed row-wise, `_textureSliceWidth
//         = 128`, `_noiseTextureRows = 4`). This port generates the SAME three
//         Worley-FBM channels but writes them straight into a `texture_3d`
//         (rgba8unorm) via a compute shader — the SPIKE-confirmed 3D-texture
//         route (see the M6.6 report). The atlas `voxelToUV` gymnastics in
//         CloudCollectionFS.glsl collapse into a single hardware trilinear
//         `textureSampleLevel` on the 3D texture (clouds.wgsl).
//   • packages/engine/Source/Scene/CloudCollection.js L117-119, L232-238, L697-731
//       — the noise-texture dimensions + the fullscreen quad that drives the FS.
//   • domain/effects/src/cloud.rs `worley_noise` / `worley_fbm` / `NoiseVolume`
//       — the f64 CPU reference this shader mirrors (cross-validated by tests).
//   Blueprint (踩坑参考): cesium-rs/crates/cesium-shaders/shaders/CloudNoiseFS.glsl
//
// # SPIKE (M6.6 step 1)
//   This file is the proof that Bevy 0.15 / wgpu / naga 23.1 support 3D textures:
//   `texture_storage_3d<rgba8unorm, write>` + `textureStore(coords: vec3<i32>, …)`
//   parse AND validate under naga (see `spike_3d_texture_support_under_naga` +
//   `cloud_noise_wgsl_parses_and_type_checks_under_naga` in effects/clouds.rs).
//   3D textures are a CORE wgpu feature (no `Features::` flag, no downlevel
//   limit); the adapter still probes gracefully and falls back to a CPU upload
//   of the domain `NoiseVolume` if a device cannot create a D3 texture.
//
// # RED LINES (hard constraints)
//   • NO `mod(` CALL — `mod` is a WGSL RESERVED WORD. The positive-modulo `wrap`
//     uses the `%` remainder OPERATOR (valid on f32), never a `mod()` builtin.
//   • NO FMA CONTRACTION: every `a * b + c` is kept as two roundings; the noise
//     field must not depend on the compiler fusing multiply-add.
//   • glam fast-math is disabled repo-wide; nothing here relies on non-IEEE floats.
//   • domain f64 → WGSL f32 boundary: the CPU reference is f64; this shader is the
//     f32 GPU mirror. `noise_detail` / `offset` arrive already in f32 via uniform.
//   • MEMORY BUDGET: 128³ × RGBA8 = 8 MB GPU (rgba8unorm). The CPU f64 reference
//     volume is ~50 MB (3 channels) and is only materialised for tests/upload.
//
// # DEVIATION (docs/deviations.md#dev-032, draft)
//   Upstream stores the noise in a 2D atlas and hand-rolls trilinear sampling
//   (voxelToUV + lerpSamplesX). This port uses a real 3D texture + hardware
//   trilinear, and generates it with a compute shader instead of a fullscreen
//   fragment pass. Numerically equivalent (same Worley-FBM), structurally simpler.

struct NoiseParams {
    /// x = u_noiseDetail (default 16), y = volume edge (128), z = slice width, w = pad.
    detail_and_dim: vec4<f32>,
    /// xyz = u_noiseOffset, w = pad.
    offset: vec4<f32>,
}

// The generated 128³ noise volume. rgba8unorm is a WRITABLE storage format in
// wgpu/naga; RGB carry worley0/1/2, A is 1.0 (mirrors CloudNoiseFS.glsl L91).
@group(0) @binding(0) var noise_out: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(1) var<uniform> params: NoiseParams;

const MAX_FBM_ITERATIONS: u32 = 10u;
const WORLEY_FBM_PERSISTENCE: f32 = 0.625;

fn fract_v3(v: vec3<f32>) -> vec3<f32> {
    return v - floor(v);
}

fn floor_v3(v: vec3<f32>) -> vec3<f32> {
    return floor(v);
}

// Mirror of CloudNoiseFS.glsl `wrap` (L6-13): positive modulo in [0, range).
// Uses the `%` remainder OPERATOR — never the reserved word `mod`.
fn wrapf(value: f32, range_length: f32) -> f32 {
    if (value < 0.0) {
        let abs_value = abs(value);
        let mod_value = abs_value % range_length;
        return (range_length - mod_value) % range_length;
    }
    return value % range_length;
}

fn wrap_vec(value: vec3<f32>, range_length: f32) -> vec3<f32> {
    return vec3<f32>(
        wrapf(value.x, range_length),
        wrapf(value.y, range_length),
        wrapf(value.z, range_length),
    );
}

// Mirror of CloudNoiseFS.glsl `random3` (L21-25).
fn worley_random3(p: vec3<f32>) -> vec3<f32> {
    let dot1 = dot(p, vec3<f32>(127.1, 311.7, 932.8));
    let dot2 = dot(p, vec3<f32>(269.5, 183.3, 421.4));
    return fract_v3(vec3<f32>(sin(dot1 - dot2), cos(dot1 * dot2), dot1 * dot2));
}

// Mirror of CloudNoiseFS.glsl `getWorleyCellPoint` (L29-36).
fn worley_cell_point(
    center_cell: vec3<f32>,
    offset: vec3<f32>,
    detail: f32,
    noise_offset: vec3<f32>,
    slice_width: f32,
) -> vec3<f32> {
    var cell = wrap_vec(center_cell + offset, slice_width / detail);
    cell = cell + floor_v3(noise_offset / detail);
    return offset + worley_random3(cell);
}

// Mirror of CloudNoiseFS.glsl `worleyNoise` (L38-58): shortest distance to the
// nearest jittered cell centre over the 3×3×3 neighbourhood.
fn worley_noise(
    p: vec3<f32>,
    freq: f32,
    detail: f32,
    noise_offset: vec3<f32>,
    slice_width: f32,
) -> f32 {
    let center_cell = floor_v3(p * freq);
    let point_in_cell = fract_v3(p * freq);
    var shortest_distance: f32 = 1000.0;
    for (var z: i32 = -1; z <= 1; z = z + 1) {
        for (var y: i32 = -1; y <= 1; y = y + 1) {
            for (var x: i32 = -1; x <= 1; x = x + 1) {
                let offset = vec3<f32>(f32(x), f32(y), f32(z));
                let point = worley_cell_point(
                    center_cell, offset, detail, noise_offset, slice_width,
                );
                let distance = length(point_in_cell - point);
                if (distance < shortest_distance) {
                    shortest_distance = distance;
                }
            }
        }
    }
    return shortest_distance;
}

// Mirror of CloudNoiseFS.glsl `worleyFBMNoise` (L62-76).
fn worley_fbm(
    p: vec3<f32>,
    octaves: u32,
    scale: f32,
    detail: f32,
    noise_offset: vec3<f32>,
    slice_width: f32,
) -> f32 {
    var noise: f32 = 0.0;
    var freq: f32 = 1.0;
    var persistence: f32 = WORLEY_FBM_PERSISTENCE;
    for (var i: u32 = 0u; i < MAX_FBM_ITERATIONS; i = i + 1u) {
        if (i >= octaves) {
            break;
        }
        // NO FMA: the multiply and the accumulate stay separate roundings.
        noise = noise + worley_noise(p * scale, freq * scale, detail, noise_offset, slice_width)
            * persistence;
        persistence = persistence * 0.5;
        freq = freq * 2.0;
    }
    return noise;
}

// One invocation produces one voxel (x, y, z) of the 128³ volume.
// Mirrors CloudNoiseFS.glsl `main` (L78-92): position = voxel / detail, then three
// clamped worleyFBM channels at scale 1/2/3.
@compute @workgroup_size(4, 4, 4)
fn generate_noise(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = u32(params.detail_and_dim.y);
    if (gid.x >= dim || gid.y >= dim || gid.z >= dim) {
        return;
    }
    let detail = params.detail_and_dim.x;
    let slice_width = params.detail_and_dim.z;
    let noise_offset = params.offset.xyz;
    let position = vec3<f32>(f32(gid.x), f32(gid.y), f32(gid.z)) / detail;

    let worley0 = clamp(worley_fbm(position, 3u, 1.0, detail, noise_offset, slice_width), 0.0, 1.0);
    let worley1 = clamp(worley_fbm(position, 3u, 2.0, detail, noise_offset, slice_width), 0.0, 1.0);
    let worley2 = clamp(worley_fbm(position, 3u, 3.0, detail, noise_offset, slice_width), 0.0, 1.0);

    textureStore(noise_out, vec3<i32>(gid), vec4<f32>(worley0, worley1, worley2, 1.0));
}
