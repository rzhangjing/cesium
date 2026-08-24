// Hand-written WGSL port of the CesiumJS billboard batch fragment shader
// (packages/engine/Source/Shaders/BillboardCollectionFS.glsl trimmed to the
// atlas sample + color multiply path used by the cesium-rs billboard /
// point-primitive / label batches).
//
// DEVIATION: the CesiumJS original supports horizontal/vertical origin
// flipping and pick-color encoding; the wgpu port samples the texture atlas
// and multiplies the billboard color (pre-expansion of the flip happens on
// the CPU through the texture coordinates, and picking is not yet wired).
//
// Binding contract:
//   group(1) binding(0): u_atlas – the billboard texture atlas
//   group(1) binding(1): u_atlasSampler – bound by the renderer with its
//                        shared linear sampler

@group(1) @binding(0) var u_atlas: texture_2d<f32>;
@group(1) @binding(1) var u_atlasSampler: sampler;

@fragment
fn main(
    @location(0) v_textureCoordinates: vec2<f32>,
    @location(1) v_color: vec4<f32>,
) -> @location(0) vec4<f32> {
    let texel = textureSample(u_atlas, u_atlasSampler, v_textureCoordinates);
    return vec4<f32>(v_color.rgb * texel.rgb, v_color.a * texel.a);
}
