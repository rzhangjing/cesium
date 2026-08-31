// WGSL port of packages/engine/Source/Shaders/Builtin/Structs/*.glsl
// (all 8 czm_* GLSL structs mirrored 1:1, SH-01 task).
//
// DEVIATION: WGSL has no preprocessor; the GLSL `#ifdef`-gated optional
// fields are resolved to a fixed variant per struct and documented below.
//
// DEVIATION: czm_modelMaterial is mirrored WITHOUT the USE_SPECULAR /
// USE_ANISOTROPY / USE_CLEARCOAT variant fields (specularWeight,
// anisotropicT/B/anisotropyStrength, clearcoat*) — base-field variant only,
// matching the default (no-extension) GLSL compilation.
//
// DEVIATION: czm_shadowParameters is mirrored with `texCoords: vec2<f32>`
// (the non-USE_CUBE_MAP_SHADOW variant).

// depthRangeStruct.glsl
struct czm_depthRangeStruct {
    near: f32,
    far: f32,
}

// material.glsl
struct czm_material {
    diffuse: vec3<f32>,
    specular: f32,
    shininess: f32,
    normal: vec3<f32>,
    emission: vec3<f32>,
    alpha: f32,
}

// materialInput.glsl
struct czm_materialInput {
    s: f32,
    st: vec2<f32>,
    str: vec3<f32>,
    normalEC: vec3<f32>,
    tangentToEyeMatrix: mat3x3<f32>,
    positionToEyeEC: vec3<f32>,
    height: f32,
    slope: f32,
    aspect: f32,
    waterMask: f32,
}

// modelMaterial.glsl (base variant, see DEVIATION above)
struct czm_modelMaterial {
    baseColor: vec4<f32>,
    diffuse: vec3<f32>,
    alpha: f32,
    specular: vec3<f32>,
    roughness: f32,
    normalEC: vec3<f32>,
    occlusion: f32,
    emissive: vec3<f32>,
}

// modelVertexOutput.glsl
struct czm_modelVertexOutput {
    positionMC: vec3<f32>,
    pointSize: f32,
}

// ray.glsl
struct czm_ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}

// raySegment.glsl (+ czm_emptyRaySegment / czm_fullRaySegment constants;
// czm_infinity is mirrored locally, value identical to Constants/infinity.glsl)
struct czm_raySegment {
    start: f32,
    stop: f32,
}

const czm_infinity: f32 = 5906376272000.0;

var<private> czm_emptyRaySegment: czm_raySegment = czm_raySegment(-czm_infinity, -czm_infinity);
var<private> czm_fullRaySegment: czm_raySegment = czm_raySegment(0.0, czm_infinity);

// shadowParameters.glsl (non-cube-map variant, see DEVIATION above)
struct czm_shadowParameters {
    texCoords: vec2<f32>,
    depthBias: f32,
    depth: f32,
    nDotL: f32,
    texelStepSize: vec2<f32>,
    normalShadingSmooth: f32,
    darkness: f32,
}

// Referencing function so every declared struct/symbol is exercised by
// validation (WGSL has no "declaration-only" validation like GLSL).
fn czm_structs_probe() -> f32 {
    let dr = czm_depthRangeStruct(0.0, 1.0);
    let m = czm_material(vec3(0.0), 0.0, 1.0, vec3(0.0), vec3(0.0), 1.0);
    let mi = czm_materialInput(0.0, vec2(0.0), vec3(0.0), vec3(0.0),
        mat3x3(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0), vec3(0.0, 0.0, 1.0)),
        vec3(0.0), 0.0, 0.0, 0.0, 0.0);
    let mm = czm_modelMaterial(vec4(0.0), vec3(0.0), 1.0, vec3(0.0), 0.0,
        vec3(0.0), 1.0, vec3(0.0));
    let mvo = czm_modelVertexOutput(vec3(0.0), 1.0);
    let r = czm_ray(vec3(0.0), vec3(0.0, 0.0, 1.0));
    var seg = czm_raySegment(0.0, 1.0);
    seg.start = czm_emptyRaySegment.start + czm_fullRaySegment.start;
    seg.stop = czm_emptyRaySegment.stop + czm_fullRaySegment.stop;
    let sp = czm_shadowParameters(vec2(0.0), 0.0, 0.0, 0.0, vec2(0.0), 0.0, 0.0);
    return dr.near + dr.far + m.specular + m.shininess + m.alpha
        + mi.s + mi.height + mi.slope + mi.aspect + mi.waterMask
        + mm.alpha + mm.roughness + mm.occlusion
        + mvo.pointSize + r.origin.x + seg.start + seg.stop
        + sp.depthBias + sp.depth + sp.nDotL + sp.normalShadingSmooth + sp.darkness;
}
