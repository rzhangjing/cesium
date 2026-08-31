// WGSL port of the depth cluster of
// packages/engine/Source/Shaders/Builtin/Functions/*.glsl (SH-01 task):
//   reverseLogDepth.glsl, readDepth.glsl, depthClamp.glsl,
//   writeDepthClamp.glsl, writeLogDepth.glsl,
//   vertexLogDepth.glsl (incl. czm_updatePositionDepth)
//
// DEVIATION: self-contained module — the log-depth automatic uniforms are
// declared here as explicit group(2) bindings (WGSL has no include and no
// implicit uniforms).
// DEVIATION: czm_reverseLogDepth mirrors the LOG_DEPTH-active variant.
// DEVIATION: czm_depthClamp mirrors the non-LOG_DEPTH + GL_EXT_frag_depth
// variant; the GLSL `out float v_WindowZ` becomes the returned
// czm_depthClampResult.windowZ (WGSL forbids global outputs).
// DEVIATION: czm_writeDepthClamp reads v_WindowZ / gl_FragCoord.w in GLSL;
// WGSL receives both as parameters and returns the gl_FragDepth value
// instead of writing the builtin (callers assign it to @builtin(frag_depth)).
// DEVIATION: czm_writeLogDepth mirrors the LOG_DEPTH-active,
// non-POLYGON_OFFSET variant; GLSL `discard` becomes the returned
// shouldDiscard flag (WGSL forbids discard outside fragment shaders and the
// mirrored helpers must be callable from any stage). The no-argument
// czm_writeLogDepth() overload (reading varying v_depthFromNearPlusOne) is
// not mirrored — WGSL has no implicit varyings.
// DEVIATION: czm_vertexLogDepth mirrors the LOG_DEPTH-active variant; the
// gl_Position read/write becomes parameter + returned struct fields.

@group(2) @binding(0) var<uniform> czm_currentFrustum: vec2<f32>;
@group(2) @binding(1) var<uniform> czm_log2FarDepthFromNearPlusOne: f32;
@group(2) @binding(2) var<uniform> czm_farDepthFromNearPlusOne: f32;
@group(2) @binding(3) var<uniform> czm_oneOverLog2FarDepthFromNearPlusOne: f32;

// ---- reverseLogDepth.glsl ----------------------------------------------------
fn czm_reverseLogDepth(logZ: f32) -> f32 {
    // LOG_DEPTH-active variant (see DEVIATION in file header).
    let nearPlane = czm_currentFrustum.x;
    let farPlane = czm_currentFrustum.y;
    let log2Depth = logZ * czm_log2FarDepthFromNearPlusOne;
    let depthFromNear = exp2(log2Depth) - 1.0;
    return farPlane * (1.0 - nearPlane / (depthFromNear + nearPlane)) / (farPlane - nearPlane);
}

// ---- readDepth.glsl ------------------------------------------------------------
fn czm_readDepth(depthTexture: texture_2d<f32>, samp: sampler, texCoords: vec2<f32>) -> f32 {
    // GLSL texture() implicit LOD mirrored as explicit level 0.0.
    return czm_reverseLogDepth(textureSampleLevel(depthTexture, samp, texCoords, 0.0).r);
}

// ---- depthClamp.glsl --------------------------------------------------------------
struct czm_depthClampResult {
    coords: vec4<f32>,
    windowZ: f32,
}
fn czm_depthClamp(coords: vec4<f32>) -> czm_depthClampResult {
    // Non-LOG_DEPTH + GL_EXT_frag_depth variant: set clip z to 0.0 and carry
    // the unaltered screen-space z via windowZ (emulated noperspective).
    var result: czm_depthClampResult;
    result.windowZ = (0.5 * (coords.z / coords.w) + 0.5) * coords.w;
    result.coords = vec4(coords.xy, 0.0, coords.w);
    return result;
}

// ---- writeDepthClamp.glsl --------------------------------------------------------------
fn czm_writeDepthClamp(windowZ: f32, fragCoordW: f32) -> f32 {
    // DEVIATION: GLSL reads v_WindowZ / gl_FragCoord.w implicitly and writes
    // gl_FragDepth; WGSL takes both as parameters and returns the depth value.
    return clamp(windowZ * fragCoordW, 0.0, 1.0);
}

// ---- writeLogDepth.glsl ---------------------------------------------------------------------
struct czm_writeLogDepthResult {
    depth: f32,
    shouldDiscard: bool,
}
fn czm_writeLogDepth(depth: f32) -> czm_writeLogDepthResult {
    var result: czm_writeLogDepthResult;

    // Discard the vertex if it's not between the near and far planes.
    // We allow a bit of epsilon on the near plane comparison because a 1.0
    // from the vertex shader (indicating the vertex should be _on_ the near
    // plane) will not necessarily come here as exactly 1.0.
    if (depth <= 0.9999999 || depth > czm_farDepthFromNearPlusOne) {
        result.depth = 0.0;
        result.shouldDiscard = true;
        return result;
    }

    result.depth = log2(depth) * czm_oneOverLog2FarDepthFromNearPlusOne;
    result.shouldDiscard = false;
    return result;
}

// ---- vertexLogDepth.glsl (incl. czm_updatePositionDepth) --------------------------------------
fn czm_updatePositionDepth(coords: vec4<f32>) -> vec4<f32> {
    // With the very high far/near ratios used with the logarithmic depth
    // buffer, floating point rounding errors can cause linear depth values
    // to end up on the wrong side of the far plane, even for vertices that
    // are really nowhere near it. Since we always write a correct logarithmic
    // depth value in the fragment shader anyway, we just need to make sure
    // such errors don't cause the primitive to be clipped entirely before
    // we even get to the fragment shader.
    return vec4(coords.xy, clamp(coords.z / coords.w, -1.0, 1.0) * coords.w, coords.w);
}

struct czm_vertexLogDepthResult {
    depthFromNearPlusOne: f32,
    position: vec4<f32>,
}
fn czm_vertexLogDepth(position: vec4<f32>) -> czm_vertexLogDepthResult {
    // DEVIATION: GLSL reads/writes gl_Position and writes the varying
    // v_depthFromNearPlusOne; WGSL takes the clip coordinates as a parameter
    // and returns both results.
    var result: czm_vertexLogDepthResult;
    result.depthFromNearPlusOne = (position.w - czm_currentFrustum.x) + 1.0;
    result.position = czm_updatePositionDepth(position);
    return result;
}
fn czm_vertexLogDepth_clip(clipCoords: vec4<f32>) -> czm_vertexLogDepthResult {
    return czm_vertexLogDepth(clipCoords);
}
