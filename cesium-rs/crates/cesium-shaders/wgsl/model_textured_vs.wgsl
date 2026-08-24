// Hand-written WGSL port of the CesiumJS model textured vertex shader
// (packages/engine/Source/Shaders/ModelVS.glsl trimmed to the position +
// first texture-coordinate set path used by the cesium-rs Model runtime
// primitives).
//
// DEVIATION: the CesiumJS original supports skinning, morph targets,
// quantization/dequantization and multiple texture-coordinate sets; the
// wgpu port passes POSITION and TEXCOORD_0 through directly (the per-draw
// model matrix folds node world transforms).
//
// Binding contract (shared by all hand-written WGSL in this directory):
//   group(0): CesiumAutomaticUniforms buffer
//     binding(0): struct with czm_modelViewProjection / czm_modelView /
//                 czm_projection / czm_view / czm_model / czm_viewport

struct CesiumAutomaticUniforms {
    czm_modelViewProjection: mat4x4<f32>,
    czm_modelView: mat4x4<f32>,
    czm_projection: mat4x4<f32>,
    czm_view: mat4x4<f32>,
    czm_model: mat4x4<f32>,
    czm_viewport: vec4<f32>,
};

@group(0) @binding(0) var<uniform> czm: CesiumAutomaticUniforms;

struct VSOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) v_textureCoordinates: vec2<f32>,
};

@vertex
fn main(
    @location(0) position: vec3<f32>,
    @location(1) texcoord: vec2<f32>,
) -> VSOutput {
    var out: VSOutput;
    out.position = czm.czm_modelViewProjection * vec4<f32>(position, 1.0);
    out.v_textureCoordinates = texcoord;
    return out;
}
