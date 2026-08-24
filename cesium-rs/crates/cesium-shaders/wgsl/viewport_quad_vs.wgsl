// Hand-written WGSL port of packages/engine/Source/Shaders/ViewportQuadVS.glsl
//
// DEVIATION: CesiumJS ships this shader as GLSL compiled through ShaderSource +
// material injection. Per docs/shader-strategy.md (hybrid route, Batch D), the
// key smoke-path shaders are hand-translated to WGSL instead of naga GLSL
// translation (naga glsl-in cannot parse CesiumJS sampler/uniform conventions).
//
// Trimming scope vs. the GLSL original:
//   - position3DAndHeight: only .xy (clip-space position) + height 0.0 are used;
//     the height/scaling branch (czm_maximumTerrainHeight etc.) is not ported.
//   - textureCoordAndEncodedAttributes: only .xy (texture coordinates) are used;
//     the pick/encoded-attribute packing is not ported.
//
// Vertex inputs mirror the CesiumJS attribute names:
//   position3DAndHeight             (vec4, location 0)
//   textureCoordAndEncodedAttributes (vec4, location 1)

struct VSOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) v_textureCoordinates: vec2<f32>,
};

@vertex
fn main(
    @location(0) position3DAndHeight: vec4<f32>,
    @location(1) textureCoordAndEncodedAttributes: vec4<f32>,
) -> VSOutput {
    var out: VSOutput;
    // gl_Position = vec4(position3DAndHeight.xy, 0.0, 1.0);
    out.position = vec4<f32>(position3DAndHeight.xy, 0.0, 1.0);
    out.v_textureCoordinates = textureCoordAndEncodedAttributes.xy;
    return out;
}
