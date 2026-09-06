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
// Trimming scope vs. GlobeFS.glsl:
//   - one day texture sampled at v_textureCoordinates
//   - a simplified Lambert day/night terminator from a fixed sun direction
//     (the port does not yet drive czm_sunDirection, so lighting is not
//      scene-accurate — it only restores the sense of a lit sphere)
//   - a Fresnel-style atmospheric limb glow evaluated in view space
// NOT ported: full czm_computeMaterial, water mask, physical atmosphere/fog,
//   texture arrays and per-texture translation/scale, alpha/brightness/
//   contrast/hue/saturation/gamma adjustments, split direction, pick color,
//   initial color blending, surface shader set codegen.
//
// Binding contract (shared by all hand-written WGSL in this directory):
//   group(0): CesiumAutomaticUniforms buffer (declared in globe_vs.wgsl)
//   group(1): per-draw material resources
//     binding(0): sampled texture (texture_2d<f32>)
//     binding(1): sampler

// group(0) binding(0) is the shared CesiumAutomaticUniforms buffer (declared
// identically in globe_vs.wgsl; visibility is VERTEX_FRAGMENT so the fragment
// stage can read czm_modelView for the view-space atmosphere term).
struct CesiumAutomaticUniforms {
    czm_modelViewProjection: mat4x4<f32>,
    czm_modelView: mat4x4<f32>,
    czm_projection: mat4x4<f32>,
    czm_view: mat4x4<f32>,
    czm_model: mat4x4<f32>,
    czm_viewport: vec4<f32>,
};

@group(0) @binding(0) var<uniform> czm: CesiumAutomaticUniforms;

@group(1) @binding(0) var u_dayTexture: texture_2d<f32>;
@group(1) @binding(1) var u_daySampler: sampler;

@fragment
fn main(
    @location(0) v_textureCoordinates: vec2<f32>,
    @location(1) v_worldPosition: vec3<f32>,
) -> @location(0) vec4<f32> {
    let texColor = textureSample(u_dayTexture, u_daySampler, v_textureCoordinates);

    // Surface normal ≈ normalized geocentric position (a sphere approximation
    // of the WGS84 ellipsoid — adequate for a Lambert terminator).
    let N = normalize(v_worldPosition);

    // A fixed world-space sun direction gives the globe a day/night terminator.
    let sunDir = normalize(vec3<f32>(0.55, 0.35, 0.5));
    let ndl = max(dot(N, sunDir), 0.0);
    let diffuse = clamp(0.28 + ndl, 0.0, 1.15);

    // Atmospheric limb glow: a Fresnel-style rim evaluated in view space so it
    // tracks the silhouette as the camera orbits.
    let viewNormal = normalize((czm.czm_modelView * vec4<f32>(N, 0.0)).xyz);
    let viewPos = (czm.czm_modelView * vec4<f32>(v_worldPosition, 1.0)).xyz;
    let V = normalize(-viewPos);
    let rim = pow(1.0 - max(dot(viewNormal, V), 0.0), 3.0);
    let atmosphere = rim * vec3<f32>(0.35, 0.6, 1.0);

    let color = texColor.rgb * diffuse + atmosphere;
    return vec4<f32>(color, texColor.a);
}
