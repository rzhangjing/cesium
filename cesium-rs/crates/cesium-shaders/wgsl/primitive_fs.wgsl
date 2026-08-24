// Hand-written WGSL port of the CesiumJS PerInstanceAppearance fragment path
// (packages/engine/Source/Shaders/Appearances/PerInstanceColorAppearanceFS.glsl
// trimmed to a Lambert-shaded flat color).
//
// DEVIATION: the CesiumJS original supports materials and flat/translucent
// variants through GLSL codegen; the wgpu port shades the per-instance color
// with a fixed directional (Lambert) light, which is the visual contract of
// the default `PerInstanceColorAppearance` for the smoke path.
//
// Binding contract:
//   group(1) binding(0): u_color (vec4<f32>), the per-instance color
//     (mirrors the JS per-vertex `color` attribute flattened to one draw
//      uniform per geometry instance).

@group(1) @binding(0) var<uniform> u_color: vec4<f32>;

@fragment
fn main(@location(0) v_normal: vec3<f32>) -> @location(0) vec4<f32> {
    let normal = normalize(v_normal);
    // Fixed eye-space light direction (CesiumJS default lighting uses the
    // sun direction; the wgpu smoke path uses a constant approximation).
    let light_direction = normalize(vec3<f32>(0.35, 0.55, 0.75));
    let diffuse = max(dot(normal, light_direction), 0.0);
    let shade = 0.25 + 0.75 * diffuse;
    return vec4<f32>(u_color.rgb * shade, u_color.a);
}
