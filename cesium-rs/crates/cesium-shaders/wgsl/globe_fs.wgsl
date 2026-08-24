// Hand-written WGSL port of packages/engine/Source/Shaders/GlobeFS.glsl
// (TEXONLY trimmed variant).
//
// DEVIATION: The CesiumJS original supports N day textures (TEXTURE_UNITS),
// lighting (czm_computeMaterial with diffuse/specular), water mask,
// atmosphere fog, day/night alpha, split, HSB adjustments, underground color,
// translucency, etc. Per docs/shader-strategy.md (hybrid route, Batch D) only
// the TEXONLY configuration is hand-translated to WGSL for the smoke path:
// position + UV + single texture sample.
//
// Trimming scope vs. GlobeFS.glsl (TEXONLY keeps only):
//   - one day texture sampled at v_textureCoordinates
//   - out_FragColor = sampled color
// NOT ported: lighting / czm_computeMaterial, water mask, atmosphere/fog,
//   texture arrays and per-texture translation/scale, alpha/brightness/
//   contrast/hue/saturation/gamma adjustments, split direction, pick color,
//   initial color blending, surface shader set codegen.
//
// Binding contract (shared by all hand-written WGSL in this directory):
//   group(0): CesiumAutomaticUniforms buffer (declared in globe_vs.wgsl)
//   group(1): per-draw material resources
//     binding(0): sampled texture (texture_2d<f32>)
//     binding(1): sampler

@group(1) @binding(0) var u_dayTexture: texture_2d<f32>;
@group(1) @binding(1) var u_daySampler: sampler;

@fragment
fn main(@location(0) v_textureCoordinates: vec2<f32>) -> @location(0) vec4<f32> {
    // out_FragColor = texture(u_dayTextures[0], v_textureCoordinates.xy)
    return textureSample(u_dayTexture, u_daySampler, v_textureCoordinates);
}
