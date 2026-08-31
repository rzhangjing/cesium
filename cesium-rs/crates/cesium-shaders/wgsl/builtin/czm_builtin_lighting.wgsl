// WGSL port of the lighting/shadow cluster of
// packages/engine/Source/Shaders/Builtin/Functions/*.glsl (SH-01 task):
//   getLambertDiffuse.glsl, getSpecular.glsl, getDefaultMaterial.glsl,
//   phong.glsl, translucentPhong.glsl, pbrLighting.glsl,
//   cascadeColor.glsl, cascadeDistance.glsl, cascadeMatrix.glsl,
//   cascadeWeights.glsl, antialias.glsl, backFacing.glsl,
//   shadowDepthCompare.glsl, shadowVisibility.glsl
//
// DEVIATION: self-contained module — the czm_material / czm_materialInput /
// czm_modelMaterial / czm_shadowParameters structs (Builtin/Structs) and the
// needed constants are inlined here (WGSL has no include).
// DEVIATION: GLSL implicit automatic uniforms (czm_sceneMode, czm_lightColor,
// shadowMap_cascade*) become explicit group(2) uniform bindings.
// DEVIATION: czm_pbrLighting is mirrored without the USE_SPECULAR /
// USE_ANISOTROPY preprocessor branches (isotropic, no specular-weight path).
// DEVIATION: czm_shadowVisibility / czm_private_shadowVisibility are mirrored
// without the USE_SOFT_SHADOWS / USE_NORMAL_SHADING / USE_CUBE_MAP_SHADOW
// branches (2D map, hard compare, no normal shading); cube maps keep only
// the depth-compare helpers.
// DEVIATION: czm_sampleShadowMap 2D mirrors the USE_SHADOW_DEPTH_TEXTURE path
// (direct .r read).
// DEVIATION: czm_backFacing reads gl_FrontFacing in GLSL; WGSL passes it as
// the @builtin(front_facing) value via a function parameter.
// DEVIATION: GLSL texture() (implicit LOD) mirrored with textureSampleLevel
// at level 0.0; GLSL step() mirrored with select(0.0, 1.0, ...).

const czm_pi: f32 = 3.141592653589793;
const czm_epsilon2: f32 = 0.01;
const czm_sceneMode3D: f32 = 3.0;

@group(2) @binding(0) var<uniform> czm_sceneMode: f32;
@group(2) @binding(1) var<uniform> czm_lightColor: vec3<f32>;
@group(2) @binding(2) var<uniform> shadowMap_cascadeDistances: vec4<f32>;
@group(2) @binding(3) var<uniform> shadowMap_cascadeMatrices: array<mat4x4<f32>, 4>;
@group(2) @binding(4) var<uniform> shadowMap_cascadeSplits: array<vec4<f32>, 2>;

// ---- inlined structs (Builtin/Structs) -------------------------------------
struct czm_material {
    diffuse: vec3<f32>,
    specular: f32,
    shininess: f32,
    normal: vec3<f32>,
    emission: vec3<f32>,
    alpha: f32,
}
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
struct czm_shadowParameters {
    texCoords: vec2<f32>,
    depthBias: f32,
    depth: f32,
    nDotL: f32,
    texelStepSize: vec2<f32>,
    normalShadingSmooth: f32,
    darkness: f32,
}

// ---- getLambertDiffuse.glsl / getSpecular.glsl ------------------------------
fn czm_getLambertDiffuse(lightDirectionEC: vec3<f32>, normalEC: vec3<f32>) -> f32 {
    return max(dot(lightDirectionEC, normalEC), 0.0);
}
fn czm_getSpecular(lightDirectionEC: vec3<f32>, toEyeEC: vec3<f32>, normalEC: vec3<f32>, shininess: f32) -> f32 {
    let toReflectedLight = reflect(-lightDirectionEC, normalEC);
    let specular = max(dot(toReflectedLight, toEyeEC), 0.0);
    return pow(specular, max(shininess, czm_epsilon2));
}

// ---- getDefaultMaterial.glsl -------------------------------------------------
fn czm_getDefaultMaterial(materialInput: czm_materialInput) -> czm_material {
    var material: czm_material;
    material.diffuse = vec3(0.0);
    material.specular = 0.0;
    material.shininess = 1.0;
    material.normal = materialInput.normalEC;
    material.emission = vec3(0.0);
    material.alpha = 1.0;
    return material;
}

// ---- phong.glsl ----------------------------------------------------------------
fn czm_private_getLambertDiffuseOfMaterial(lightDirectionEC: vec3<f32>, material: czm_material) -> f32 {
    return czm_getLambertDiffuse(lightDirectionEC, material.normal);
}
fn czm_private_getSpecularOfMaterial(lightDirectionEC: vec3<f32>, toEyeEC: vec3<f32>, material: czm_material) -> f32 {
    return czm_getSpecular(lightDirectionEC, toEyeEC, material.normal, material.shininess);
}
fn czm_phong(toEye: vec3<f32>, material: czm_material, lightDirectionEC: vec3<f32>) -> vec4<f32> {
    // Diffuse from directional light sources at eye (for top-down)
    var diffuse = czm_private_getLambertDiffuseOfMaterial(vec3(0.0, 0.0, 1.0), material);
    if (czm_sceneMode == czm_sceneMode3D) {
        // (and horizon views in 3D)
        diffuse = diffuse + czm_private_getLambertDiffuseOfMaterial(vec3(0.0, 1.0, 0.0), material);
    }

    let specular = czm_private_getSpecularOfMaterial(lightDirectionEC, toEye, material);

    // Temporary workaround for adding ambient.
    let materialDiffuse = material.diffuse * vec3(0.5);

    let ambient = materialDiffuse;
    var color = ambient + material.emission;
    color = color + materialDiffuse * vec3(diffuse) * czm_lightColor;
    color = color + vec3(material.specular) * vec3(specular) * czm_lightColor;

    return vec4(color, material.alpha);
}
fn czm_private_phong(toEye: vec3<f32>, material: czm_material, lightDirectionEC: vec3<f32>) -> vec4<f32> {
    let diffuse = czm_private_getLambertDiffuseOfMaterial(lightDirectionEC, material);
    let specular = czm_private_getSpecularOfMaterial(lightDirectionEC, toEye, material);

    let ambient = vec3(0.0);
    var color = ambient + material.emission;
    color = color + material.diffuse * vec3(diffuse) * czm_lightColor;
    color = color + vec3(material.specular) * vec3(specular) * czm_lightColor;

    return vec4(color, material.alpha);
}

// ---- translucentPhong.glsl -----------------------------------------------------
fn czm_translucentPhong(toEye: vec3<f32>, material: czm_material, lightDirectionEC: vec3<f32>) -> vec4<f32> {
    // Diffuse from directional light sources at eye (for top-down and horizon views)
    var diffuse = czm_getLambertDiffuse(vec3(0.0, 0.0, 1.0), material.normal);

    if (czm_sceneMode == czm_sceneMode3D) {
        // (and horizon views in 3D)
        diffuse = diffuse + czm_getLambertDiffuse(vec3(0.0, 1.0, 0.0), material.normal);
    }

    diffuse = clamp(diffuse, 0.0, 1.0);

    let specular = czm_getSpecular(lightDirectionEC, toEye, material.normal, material.shininess);

    // Temporary workaround for adding ambient.
    let materialDiffuse = material.diffuse * vec3(0.5);

    let ambient = materialDiffuse;
    var color = ambient + material.emission;
    color = color + materialDiffuse * vec3(diffuse) * czm_lightColor;
    color = color + vec3(material.specular) * vec3(specular) * czm_lightColor;

    return vec4(color, material.alpha);
}

// ---- pbrLighting.glsl (isotropic variant, see DEVIATION) ------------------------
fn czm_private_lambertianDiffuse(diffuseColor: vec3<f32>) -> vec3<f32> {
    return diffuseColor / vec3(czm_pi);
}
fn czm_private_fresnelSchlick2(f0: vec3<f32>, f90: vec3<f32>, VdotH: f32) -> vec3<f32> {
    let versine = 1.0 - VdotH;
    // pow(versine, 5.0) is slow. See https://stackoverflow.com/a/68793086/10082269
    let versineSquared = versine * versine;
    return f0 + (f90 - f0) * vec3(versineSquared * versineSquared * versine);
}
fn czm_private_smithVisibilityGGX(alphaRoughness: f32, NdotL: f32, NdotV: f32) -> f32 {
    let alphaRoughnessSq = alphaRoughness * alphaRoughness;

    let GGXV = NdotL * sqrt(NdotV * NdotV * (1.0 - alphaRoughnessSq) + alphaRoughnessSq);
    let GGXL = NdotV * sqrt(NdotL * NdotL * (1.0 - alphaRoughnessSq) + alphaRoughnessSq);

    let GGX = GGXV + GGXL;
    if (GGX > 0.0) {
        return 0.5 / GGX;
    }
    return 0.0;
}
fn czm_private_GGX(alphaRoughness: f32, NdotH: f32) -> f32 {
    let alphaRoughnessSquared = alphaRoughness * alphaRoughness;
    let f = (NdotH * alphaRoughnessSquared - NdotH) * NdotH + 1.0;
    return alphaRoughnessSquared / (czm_pi * f * f);
}
fn czm_private_computeDirectSpecularStrength(normal: vec3<f32>, lightDirection: vec3<f32>, viewDirection: vec3<f32>, halfwayDirection: vec3<f32>, alphaRoughness: f32) -> f32 {
    let NdotL = clamp(dot(normal, lightDirection), 0.0, 1.0);
    let NdotV = clamp(dot(normal, viewDirection), 0.0, 1.0);
    let G = czm_private_smithVisibilityGGX(alphaRoughness, NdotL, NdotV);
    let NdotH = clamp(dot(normal, halfwayDirection), 0.0, 1.0);
    let D = czm_private_GGX(alphaRoughness, NdotH);
    return G * D;
}
fn czm_private_maximumComponent_vec3(v: vec3<f32>) -> f32 {
    // Private mirror of maximumComponent.glsl (self-contained module).
    return max(max(v.x, v.y), v.z);
}
fn czm_pbrLighting(viewDirectionEC: vec3<f32>, normalEC: vec3<f32>, lightDirectionEC: vec3<f32>, material: czm_modelMaterial) -> vec3<f32> {
    let halfwayDirectionEC = normalize(viewDirectionEC + lightDirectionEC);
    let VdotH = clamp(dot(viewDirectionEC, halfwayDirectionEC), 0.0, 1.0);
    let NdotL = clamp(dot(normalEC, lightDirectionEC), 0.001, 1.0);

    let f0 = material.specular;
    let reflectance = czm_private_maximumComponent_vec3(f0);
    // Typical dielectrics will have reflectance 0.04, so f90 will be 1.0.
    // In this case, at grazing angle, all incident energy is reflected.
    let f90 = vec3(clamp(reflectance * 25.0, 0.0, 1.0));
    let F = czm_private_fresnelSchlick2(f0, f90, VdotH);

    let alphaRoughness = material.roughness * material.roughness;
    let specularStrength = czm_private_computeDirectSpecularStrength(normalEC, lightDirectionEC, viewDirectionEC, halfwayDirectionEC, alphaRoughness);
    let specularContribution = F * vec3(specularStrength);

    let diffuseColor = material.diffuse;
    // F here represents the specular contribution
    let diffuseContribution = (vec3(1.0) - F) * czm_private_lambertianDiffuse(diffuseColor);

    // Lo = (diffuse + specular) * Li * NdotL
    return (diffuseContribution + specularContribution) * vec3(NdotL);
}

// ---- cascadeColor.glsl / cascadeDistance.glsl / cascadeMatrix.glsl / cascadeWeights.glsl ----
fn czm_cascadeColor(weights: vec4<f32>) -> vec4<f32> {
    return vec4(1.0, 0.0, 0.0, 1.0) * vec4(weights.x)
         + vec4(0.0, 1.0, 0.0, 1.0) * vec4(weights.y)
         + vec4(0.0, 0.0, 1.0, 1.0) * vec4(weights.z)
         + vec4(1.0, 0.0, 1.0, 1.0) * vec4(weights.w);
}
fn czm_cascadeDistance(weights: vec4<f32>) -> f32 {
    return dot(shadowMap_cascadeDistances, weights);
}
fn czm_cascadeMatrix(weights: vec4<f32>) -> mat4x4<f32> {
    return shadowMap_cascadeMatrices[0] * weights.x
         + shadowMap_cascadeMatrices[1] * weights.y
         + shadowMap_cascadeMatrices[2] * weights.z
         + shadowMap_cascadeMatrices[3] * weights.w;
}
fn czm_cascadeWeights(depthEye: f32) -> vec4<f32> {
    // GLSL: step(split0, depthEye) * step(depthEye, split1)
    let nearIn = select(vec4(0.0), vec4(1.0), vec4(depthEye) >= shadowMap_cascadeSplits[0]);
    let farIn = select(vec4(0.0), vec4(1.0), shadowMap_cascadeSplits[1] >= vec4(depthEye));
    return nearIn * farIn;
}

// ---- antialias.glsl -----------------------------------------------------------------------
fn czm_antialias(color1: vec4<f32>, color2: vec4<f32>, currentColor: vec4<f32>, dist: f32, fuzzFactor: f32) -> vec4<f32> {
    let val1 = clamp(dist / fuzzFactor, 0.0, 1.0);
    let val2 = clamp((dist - 0.5) / fuzzFactor, 0.0, 1.0);
    var v = val1 * (1.0 - val2);
    v = v * v * (3.0 - (2.0 * v));
    v = pow(v, 0.5); // makes the transition nicer

    let midColor = (color1 + color2) * vec4(0.5);
    return mix(midColor, currentColor, vec4(v));
}
fn czm_antialias_default(color1: vec4<f32>, color2: vec4<f32>, currentColor: vec4<f32>, dist: f32) -> vec4<f32> {
    return czm_antialias(color1, color2, currentColor, dist, 0.1);
}

// ---- backFacing.glsl ------------------------------------------------------------------------
fn czm_backFacing(front_facing: bool) -> bool {
    // DEVIATION: GLSL reads gl_FrontFacing implicitly; WGSL receives the
    // @builtin(front_facing) value as a parameter.
    return front_facing == false;
}

// ---- shadowDepthCompare.glsl -------------------------------------------------------------------
fn czm_private_unpackDepth(packedDepth: vec4<f32>) -> f32 {
    // Private mirror of unpackDepth.glsl (self-contained module).
    return dot(packedDepth, vec4(1.0, 1.0 / 255.0, 1.0 / 65025.0, 1.0 / 16581375.0));
}
fn czm_sampleShadowMap_cube(shadowMap: texture_cube<f32>, samp: sampler, d: vec3<f32>) -> f32 {
    // GLSL texture() implicit LOD mirrored as explicit level 0.0.
    return czm_private_unpackDepth(textureSampleLevel(shadowMap, samp, d, 0.0));
}
fn czm_sampleShadowMap_2d(shadowMap: texture_2d<f32>, samp: sampler, uv: vec2<f32>) -> f32 {
    // USE_SHADOW_DEPTH_TEXTURE variant (see DEVIATION in file header).
    return textureSampleLevel(shadowMap, samp, uv, 0.0).r;
}
fn czm_shadowDepthCompare_cube(shadowMap: texture_cube<f32>, samp: sampler, uv: vec3<f32>, depth: f32) -> f32 {
    // GLSL step(depth, sample)
    return select(0.0, 1.0, czm_sampleShadowMap_cube(shadowMap, samp, uv) >= depth);
}
fn czm_shadowDepthCompare_2d(shadowMap: texture_2d<f32>, samp: sampler, uv: vec2<f32>, depth: f32) -> f32 {
    return select(0.0, 1.0, czm_sampleShadowMap_2d(shadowMap, samp, uv) >= depth);
}

// ---- shadowVisibility.glsl (2D hard-compare variant, see DEVIATION) ------------------------------
fn czm_private_shadowVisibility(visibility: f32, nDotL: f32, normalShadingSmooth: f32, darkness: f32) -> f32 {
    // Non-USE_NORMAL_SHADING variant: strength modulation omitted.
    var v = visibility;
    v = max(v, darkness);
    return v;
}
fn czm_shadowVisibility(shadowMap: texture_2d<f32>, samp: sampler, shadowParameters: czm_shadowParameters) -> f32 {
    let depthBias = shadowParameters.depthBias;
    var depth = shadowParameters.depth;
    let nDotL = shadowParameters.nDotL;
    let normalShadingSmooth = shadowParameters.normalShadingSmooth;
    let darkness = shadowParameters.darkness;
    let uv = shadowParameters.texCoords;

    depth = depth - depthBias;
    let visibility = czm_shadowDepthCompare_2d(shadowMap, samp, uv, depth);

    return czm_private_shadowVisibility(visibility, nDotL, normalShadingSmooth, darkness);
}
