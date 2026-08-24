// Hand-written WGSL port of packages/engine/Source/Shaders/ViewportQuadFS.glsl
// (texture-sampling variant).
//
// DEVIATION: The CesiumJS original samples through the Material system
// (e.g. Material.ImageType → czm_sampleMaterial + czm_texture). Per
// docs/shader-strategy.md (hybrid route) the smoke path is hand-translated to
// WGSL with a plain texture_2d + sampler pair baked in.
//
// Trimming scope vs. the GLSL original:
//   - No Fabric/material graph evaluation; direct texture sampling only.
//   - No czm_gammaCorrect / HDR / premultiplied-alpha handling.
//
// Binding contract (shared by all hand-written WGSL in this directory):
//   group(0): CesiumAutomaticUniforms buffer
//   group(1): per-draw material resources
//     binding(0): sampled texture (texture_2d<f32>)
//     binding(1): sampler

@group(1) @binding(0) var u_texture: texture_2d<f32>;
@group(1) @binding(1) var u_sampler: sampler;

@fragment
fn main(@location(0) v_textureCoordinates: vec2<f32>) -> @location(0) vec4<f32> {
    // out_FragColor = texture2D(u_texture, v_textureCoordinates)
    return textureSample(u_texture, u_sampler, v_textureCoordinates);
}
