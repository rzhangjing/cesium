// Hand-written WGSL port of packages/engine/Source/Shaders/GlobeVS.glsl
// (TEXONLY trimmed variant).
//
// DEVIATION: The CesiumJS original is a heavily #ifdef-driven GLSL shader
// (quantization, exaggeration, fog, atmosphere, clipping, 2D morphing, ...).
// Per docs/shader-strategy.md (hybrid route, Batch D) only the TEXONLY
// configuration is hand-translated to WGSL for the smoke path.
//
// Trimming scope vs. GlobeVS.glsl (TEXONLY only keeps):
//   - position3DAndHeight input (.xyz position, .w height)
//   - textureCoordAndEncodedNormals input (.xy texture coordinates only)
//   - position transform via czm_modelViewProjection
//     (original uses u_modifiedModelViewProjection / getPosition() codegen;
//      replaced by the czm automatic uniform for the smoke path)
// NOT ported: QUANTIZATION_BITS12, GEODETIC_SURFACE_NORMALS, EXAGGERATION,
//   2D/Mercator modes, fog, ground atmosphere, clipping polygons, normals,
//   u_center3D/u_tileRectangle, material slope/aspect/height varyings.
//
// Binding contract (shared by all hand-written WGSL in this directory):
//   group(0): CesiumAutomaticUniforms buffer
//     binding(0): struct with czm_modelViewProjection / czm_modelView /
//                 czm_projection / czm_view / czm_model / czm_viewport
//   group(1): per-draw material resources (see globe_fs.wgsl)

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
    @location(0) position3DAndHeight: vec4<f32>,
    @location(1) textureCoordAndEncodedNormals: vec4<f32>,
) -> VSOutput {
    var out: VSOutput;
    var position = vec4<f32>(position3DAndHeight.xyz, 1.0);
    out.position = czm.czm_modelViewProjection * position;
    out.v_textureCoordinates = textureCoordAndEncodedNormals.xy;
    return out;
}
