// Hand-written WGSL port of the CesiumJS model textured fragment shader
// (packages/engine/Source/Shaders/ModelFS.glsl trimmed to the base color
// texture sample × factor path used by the cesium-rs Model runtime
// primitives).
//
// DEVIATION: the CesiumJS original evaluates the full PBR metallic
// roughness lighting model (metallic/roughness texture, IBL, normal /
// occlusion / emissive textures, alpha modes MASK cutoff discard excepted);
// the wgpu port samples the base color texture and multiplies the base
// color factor — lighting/PBR is deferred.
//
// Binding contract:
//   group(1) binding(0): u_baseColorTexture – the material base color map
//   group(1) binding(1): u_baseColorSampler – bound by the renderer with
//                        its shared sampler (glTF wrap/filter recorded at
//                        texture creation)
//   group(1) binding(2): u_baseColorFactor – the material base color
//                        (bound from DrawCommand uniform overrides through
//                        the renderer material scratch buffer)

@group(1) @binding(0) var u_baseColorTexture: texture_2d<f32>;
@group(1) @binding(1) var u_baseColorSampler: sampler;
@group(1) @binding(2) var<uniform> u_baseColorFactor: vec4<f32>;

@fragment
fn main(@location(0) v_textureCoordinates: vec2<f32>) -> @location(0) vec4<f32> {
    let texel = textureSample(u_baseColorTexture, u_baseColorSampler, v_textureCoordinates);
    return vec4<f32>(u_baseColorFactor.rgb * texel.rgb, u_baseColorFactor.a * texel.a);
}
