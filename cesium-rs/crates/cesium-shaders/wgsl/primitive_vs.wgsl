// Hand-written WGSL port of the CesiumJS PerInstanceAppearance vertex path
// (packages/engine/Source/Shaders/Appearances/PerInstanceColorAppearanceVS.glsl
// trimmed to the fixed position + normal layout used by cesium-rs Primitive).
//
// DEVIATION: the CesiumJS original generates GLSL from the Appearance's
// vertex shader source plus czm_normal/czm_modelView automatic uniforms.
// The wgpu port uses a fixed attribute layout (position vec3, normal vec3)
// and transforms the normal with czm_modelView assuming a rotation-only
// model (the cesium-rs Primitive bakes its modelMatrix into the vertex
// positions during geometry preparation, mirroring
// GeometryPipeline.transformToWorldCoordinates).
//
// Binding contract (shared by all hand-written WGSL in this directory):
//   group(0): CesiumAutomaticUniforms buffer
//     binding(0): struct with czm_modelViewProjection / czm_modelView /
//                 czm_projection / czm_view / czm_model / czm_viewport
//   group(1): per-draw material resources (see primitive_fs.wgsl)

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
    @location(0) v_normal: vec3<f32>,
};

@vertex
fn main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
) -> VSOutput {
    var out: VSOutput;
    out.position = czm.czm_modelViewProjection * vec4<f32>(position, 1.0);
    // DEVIATION: no czm_normal automatic uniform in the wgpu port; the
    // model matrix is identity at draw time (baked into positions), so the
    // modelView rotation applied to the normal is exact.
    out.v_normal = (czm.czm_modelView * vec4<f32>(normal, 0.0)).xyz;
    return out;
}
