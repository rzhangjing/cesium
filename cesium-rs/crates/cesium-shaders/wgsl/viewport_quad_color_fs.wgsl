// Hand-written WGSL port of packages/engine/Source/Shaders/ViewportQuadFS.glsl
// (solid-color material variant).
//
// DEVIATION: The CesiumJS original delegates to the Material system
// (czm_getMaterial with a Fabric "Color" material). Per docs/shader-strategy.md
// (hybrid route) the smoke path is hand-translated to WGSL with the Color
// material baked in: a single vec4 uniform at group(1) binding(0).
//
// Trimming scope vs. the GLSL original:
//   - No Fabric/material graph evaluation; only Material.ColorType semantics.
//   - No czm_gammaCorrect / HDR handling.
//
// Binding contract (shared by all hand-written WGSL in this directory):
//   group(0): CesiumAutomaticUniforms buffer (see cesium_shaders::wgsl docs)
//   group(1): per-draw material resources (here: material color buffer)

struct MaterialUniforms {
    color: vec4<f32>,
};

@group(1) @binding(0) var<uniform> material: MaterialUniforms;

@fragment
fn main(@location(0) v_textureCoordinates: vec2<f32>) -> @location(0) vec4<f32> {
    // out_FragColor = material.color (Material.ColorType)
    return material.color;
}
