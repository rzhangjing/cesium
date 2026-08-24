// Hand-written WGSL port of the CesiumJS model color vertex shader
// (packages/engine/Source/Shaders/ModelVS.glsl trimmed to the minimal
// position-only path used by the cesium-rs Model runtime primitives).
//
// DEVIATION: the CesiumJS original supports skinning, morph targets,
// quantization/dequantization and point-cloud sizing through generated
// shader stages; the wgpu port transforms the POSITION attribute directly
// by czm_modelViewProjection (the per-draw model matrix folds node world
// transforms, mirroring the JS ModelDrawCommand model matrix composition).
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

@vertex
fn main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return czm.czm_modelViewProjection * vec4<f32>(position, 1.0);
}
