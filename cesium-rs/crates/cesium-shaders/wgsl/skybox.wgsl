// Skybox shader: renders a starfield on a large sphere centered at origin.
//
// The sphere is rendered with front-face culling (we see the inside).
// Depth writes are OFF so the globe (rendered later) is not occluded.
//
// Binding contract:
//   group(0): CesiumAutomaticUniforms buffer (required by Context, unused here)
//   group(1): per-draw material resources
//     binding(0): star texture (texture_2d<f32>)
//     binding(1): sampler

struct CesiumAutomaticUniforms {
    czm_modelViewProjection: mat4x4<f32>,
    czm_modelView: mat4x4<f32>,
    czm_projection: mat4x4<f32>,
    czm_view: mat4x4<f32>,
    czm_model: mat4x4<f32>,
    czm_viewport: vec4<f32>,
};

@group(0) @binding(0) var<uniform> czm: CesiumAutomaticUniforms;
@group(1) @binding(0) var u_starTexture: texture_2d<f32>;
@group(1) @binding(1) var u_starSampler: sampler;

struct VSOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) v_uv: vec2<f32>,
};

@vertex
fn main(
    @location(0) a_position: vec4<f32>,
    @location(1) a_uv: vec4<f32>,
) -> VSOutput {
    var out: VSOutput;
    // Use modelViewProjection to place the sphere; the sphere is centered
    // at origin with a huge radius so the camera is always inside it.
    out.position = czm.czm_modelViewProjection * a_position;
    out.v_uv = a_uv.xy;
    return out;
}

@fragment
fn main(@location(0) v_uv: vec2<f32>) -> @location(0) vec4<f32> {
    let color = textureSample(u_starTexture, u_starSampler, v_uv);
    // Stars are additive: black background is transparent.
    return vec4<f32>(color.rgb, color.r);
}
