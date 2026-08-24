// Hand-written WGSL port of the CesiumJS model color fragment shader
// (packages/engine/Source/Shaders/ModelFS.glsl trimmed to the flat base
// color path used by the cesium-rs Model runtime primitives).
//
// DEVIATION: the CesiumJS original evaluates the full PBR metallic
// roughness lighting model (IBL, specular environment maps, normal /
// occlusion / emissive textures, custom shaders); the wgpu port shades
// with the material base color factor only — lighting/PBR is deferred.
//
// Binding contract:
//   group(1) binding(0): u_baseColorFactor – the material base color
//                        (bound from DrawCommand uniform overrides through
//                        the renderer material scratch buffer)

@group(1) @binding(0) var<uniform> u_baseColorFactor: vec4<f32>;

@fragment
fn main() -> @location(0) vec4<f32> {
    return u_baseColorFactor;
}
