// WGSL port of the texture-sampling cluster of
// packages/engine/Source/Shaders/Builtin/Functions/*.glsl (SH-01 task):
//   textureCube.glsl, getWaterNoise.glsl, clipPolygons.glsl,
//   unpackClippingExtents.glsl
//
// DEVIATION: self-contained module — GLSL combined sampler2D/samplerCube
// parameters are split into WGSL texture + sampler parameters (WGSL has no
// combined image samplers).
// DEVIATION: GLSL texture() (implicit LOD) mirrored with textureSampleLevel
// at level 0.0; GLSL textureSize() mirrored with textureDimensions().
// DEVIATION: czm_textureCube mirrors the WebGL2 `texture()` path with
// explicit level 0.0 (WGSL implicit-LOD sampling is fragment-stage only and
// the mirrored helpers must be callable from any stage).
// DEVIATION: czm_clipPolygons mirrors the non-CLIPPING_INVERSE variant;
// GLSL `discard` becomes the returned bool (callers discard when true).
// DEVIATION: GLSL mod() mirrored with the czm_mod_f32 helper (floor-based).

// ---- private helper: GLSL mod ------------------------------------------------
fn czm_mod_f32(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}

// ---- textureCube.glsl ------------------------------------------------------------
fn czm_textureCube(cubeTexture: texture_cube<f32>, samp: sampler, p: vec3<f32>) -> vec4<f32> {
    return textureSampleLevel(cubeTexture, samp, p, 0.0);
}
fn czm_textureCube_lod(cubeTexture: texture_cube<f32>, samp: sampler, p: vec3<f32>, lod: f32) -> vec4<f32> {
    return textureSampleLevel(cubeTexture, samp, p, lod);
}

// ---- getWaterNoise.glsl ----------------------------------------------------------------
fn czm_getWaterNoise(normalMap: texture_2d<f32>, samp: sampler, uv: vec2<f32>, time: f32, angleInRadians: f32) -> vec4<f32> {
    let cosAngle = cos(angleInRadians);
    let sinAngle = sin(angleInRadians);

    // time dependent sampling directions
    var s0 = vec2(1.0 / 17.0, 0.0);
    var s1 = vec2(-1.0 / 29.0, 0.0);
    var s2 = vec2(1.0 / 101.0, 1.0 / 59.0);
    var s3 = vec2(-1.0 / 109.0, -1.0 / 57.0);

    // rotate sampling direction by specified angle
    s0 = vec2((cosAngle * s0.x) - (sinAngle * s0.y), (sinAngle * s0.x) + (cosAngle * s0.y));
    s1 = vec2((cosAngle * s1.x) - (sinAngle * s1.y), (sinAngle * s1.x) + (cosAngle * s1.y));
    s2 = vec2((cosAngle * s2.x) - (sinAngle * s2.y), (sinAngle * s2.x) + (cosAngle * s2.y));
    s3 = vec2((cosAngle * s3.x) - (sinAngle * s3.y), (sinAngle * s3.x) + (cosAngle * s3.y));

    var uv0 = (uv / vec2(103.0)) + (time * s0);
    var uv1 = uv / vec2(107.0) + (time * s1) + vec2(0.23);
    var uv2 = uv / vec2(897.0, 983.0) + (time * s2) + vec2(0.51);
    var uv3 = uv / vec2(991.0, 877.0) + (time * s3) + vec2(0.71);

    uv0 = fract(uv0);
    uv1 = fract(uv1);
    uv2 = fract(uv2);
    uv3 = fract(uv3);
    // GLSL texture() implicit LOD mirrored as explicit level 0.0.
    let noise = (textureSampleLevel(normalMap, samp, uv0, 0.0))
              + (textureSampleLevel(normalMap, samp, uv1, 0.0))
              + (textureSampleLevel(normalMap, samp, uv2, 0.0))
              + (textureSampleLevel(normalMap, samp, uv3, 0.0));

    // average and scale to between -1 and 1
    return ((noise / vec4(4.0)) - vec4(0.5)) * vec4(2.0);
}

// ---- clipPolygons.glsl ----------------------------------------------------------------------
fn czm_private_getSignedDistance(uv: vec2<f32>, clippingDistance: texture_2d<f32>, samp: sampler) -> f32 {
    let signedDistance = textureSampleLevel(clippingDistance, samp, uv, 0.0).r;
    return (signedDistance - 0.5) * 2.0;
}
fn czm_clipPolygons(clippingDistance: texture_2d<f32>, samp: sampler, extentsLength: i32, clippingPosition: vec2<f32>, regionIndex: i32) -> bool {
    // Returns true when the GLSL implementation would `discard`
    // (non-CLIPPING_INVERSE variant, see DEVIATION in file header).

    // Position is completely outside of polygons bounds
    let rectUv = clippingPosition;
    if (regionIndex < 0 || rectUv.x <= 0.0 || rectUv.y <= 0.0 || rectUv.x >= 1.0 || rectUv.y >= 1.0) {
        // GLSL returns without discarding in the non-CLIPPING_INVERSE case.
        return false;
    }

    let clippingDistanceTextureDimensions = vec2<f32>(textureDimensions(clippingDistance));
    let sampleOffset = max(vec2(1.0) / clippingDistanceTextureDimensions, vec2(0.005));
    _ = sampleOffset; // GLSL keeps this unused local; preserved for fidelity.
    var dimension = f32(extentsLength);
    if (extentsLength > 2) {
        dimension = ceil(log2(f32(extentsLength)));
    }

    let textureOffset = vec2(czm_mod_f32(f32(regionIndex), dimension), floor(f32(regionIndex) / dimension)) / vec2(dimension);
    let uv = textureOffset + rectUv / vec2(dimension);

    let signedDistance = czm_private_getSignedDistance(uv, clippingDistance, samp);

    // Non-CLIPPING_INVERSE variant.
    if (signedDistance < 0.0) {
        return true;
    }
    return false;
}

// ---- unpackClippingExtents.glsl ------------------------------------------------------------------
fn czm_private_getLookupUv(dimensions: vec2<f32>, i: i32) -> vec2<f32> {
    let pixY = i / i32(dimensions.x);
    let pixX = i - (pixY * i32(dimensions.x));
    let pixelWidth = 1.0 / dimensions.x;
    let pixelHeight = 1.0 / dimensions.y;
    let u = (f32(pixX) + 0.5) * pixelWidth; // sample from center of pixel
    let v = (f32(pixY) + 0.5) * pixelHeight;
    return vec2(u, v);
}
fn czm_unpackClippingExtents(extentsTexture: texture_2d<f32>, samp: sampler, index: i32) -> vec4<f32> {
    let textureDimensions = vec2<f32>(textureDimensions(extentsTexture));
    return textureSampleLevel(extentsTexture, samp, czm_private_getLookupUv(textureDimensions, index), 0.0);
}
