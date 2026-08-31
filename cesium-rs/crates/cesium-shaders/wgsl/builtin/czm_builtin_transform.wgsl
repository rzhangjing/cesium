// WGSL port of the coordinate-transform cluster of
// packages/engine/Source/Shaders/Builtin/Functions/*.glsl (SH-01 task):
//   eastNorthUpToEyeCoordinates.glsl, ellipsoidContainsPoint.glsl,
//   ellipsoidTextureCoordinates.glsl, eyeOffset.glsl,
//   eyeToWindowCoordinates.glsl, modelToWindowCoordinates.glsl,
//   windowToEyeCoordinates.glsl (incl. czm_screenToEyeCoordinates),
//   translateRelativeToEye.glsl, metersPerPixel.glsl, alphaWeight.glsl,
//   getDynamicAtmosphereLightDirection.glsl
//
// DEVIATION: self-contained module — constants and automatic uniforms are
// inlined / declared as explicit group(2) bindings (WGSL has no include and
// no implicit uniforms).
// DEVIATION: czm_alphaWeight reads gl_FragCoord.z in GLSL; WGSL receives it
// as a parameter (callers pass @builtin(position).z).
// DEVIATION: czm_screenToEyeCoordinates drops the GLSL
// `czm_inverseProjection == mat4(0.0)` fallback branch — WGSL forbids matrix
// equality comparison, and the branch only exists for legacy IE/Edge drivers.
// DEVIATION: czm_screenToEyeCoordinates(vec2, float) mirrors the
// non-LOG_DEPTH variant.
// DEVIATION: GLSL overloads of czm_windowToEyeCoordinates /
// czm_screenToEyeCoordinates get the `_depth` suffix in WGSL.

const czm_oneOverPi: f32 = 0.3183098861837907;
const czm_oneOverTwoPi: f32 = 0.15915494309189535;
const czm_sceneMode2D: f32 = 2.0;
const czm_sceneMode3D: f32 = 3.0;

@group(2) @binding(0) var<uniform> czm_normal3D: mat3x3<f32>;
@group(2) @binding(1) var<uniform> czm_inverseModelView: mat4x4<f32>;
@group(2) @binding(2) var<uniform> czm_projection: mat4x4<f32>;
@group(2) @binding(3) var<uniform> czm_viewportTransformation: mat4x4<f32>;
@group(2) @binding(4) var<uniform> czm_modelView: mat4x4<f32>;
@group(2) @binding(5) var<uniform> czm_inverseProjection: mat4x4<f32>;
@group(2) @binding(6) var<uniform> czm_viewport: vec4<f32>;
@group(2) @binding(7) var<uniform> czm_frustumPlanes: vec4<f32>;
@group(2) @binding(8) var<uniform> czm_currentFrustum: vec2<f32>;
@group(2) @binding(9) var<uniform> czm_sceneMode: f32;
@group(2) @binding(10) var<uniform> czm_orthographicIn3D: f32;
@group(2) @binding(11) var<uniform> czm_pixelRatio: f32;
@group(2) @binding(12) var<uniform> czm_encodedCameraPositionMCHigh: vec3<f32>;
@group(2) @binding(13) var<uniform> czm_encodedCameraPositionMCLow: vec3<f32>;
@group(2) @binding(14) var<uniform> czm_lightDirectionWC: vec3<f32>;
@group(2) @binding(15) var<uniform> czm_sunDirectionWC: vec3<f32>;

// ---- eastNorthUpToEyeCoordinates.glsl ---------------------------------------
fn czm_eastNorthUpToEyeCoordinates(positionMC: vec3<f32>, normalEC: vec3<f32>) -> mat3x3<f32> {
    let tangentMC = normalize(vec3(-positionMC.y, positionMC.x, 0.0)); // normalized surface tangent in model coordinates
    let tangentEC = normalize(czm_normal3D * tangentMC);               // normalized surface tangent in eye coordinates
    let bitangentEC = normalize(cross(normalEC, tangentEC));           // normalized surface bitangent in eye coordinates

    return mat3x3(tangentEC, bitangentEC, normalEC);
}

// ---- ellipsoidContainsPoint.glsl ---------------------------------------------
fn czm_ellipsoidContainsPoint(ellipsoid_inverseRadii: vec3<f32>, point: vec3<f32>) -> bool {
    let scaled = ellipsoid_inverseRadii * (czm_inverseModelView * vec4(point, 1.0)).xyz;
    return dot(scaled, scaled) <= 1.0;
}

// ---- ellipsoidTextureCoordinates.glsl -------------------------------------------
fn czm_ellipsoidTextureCoordinates(normal: vec3<f32>) -> vec2<f32> {
    return vec2(atan2(normal.y, normal.x) * czm_oneOverTwoPi + 0.5,
                asin(normal.z) * czm_oneOverPi + 0.5);
}

// ---- eyeOffset.glsl ---------------------------------------------------------------
fn czm_eyeOffset(positionEC: vec4<f32>, eye_offset: vec3<f32>) -> vec4<f32> {
    var p = positionEC;
    let zEyeOffset = normalize(p) * vec4(eye_offset.z);
    p.x = p.x + eye_offset.x + zEyeOffset.x;
    p.y = p.y + eye_offset.y + zEyeOffset.y;
    p.z = p.z + zEyeOffset.z;
    return p;
}

// ---- eyeToWindowCoordinates.glsl ------------------------------------------------------
fn czm_eyeToWindowCoordinates(positionEC: vec4<f32>) -> vec4<f32> {
    var q = czm_projection * positionEC;                    // clip coordinates
    q = vec4(q.xyz / vec3(q.w), q.w);                       // normalized device coordinates
    q = vec4((czm_viewportTransformation * vec4(q.xyz, 1.0)).xyz, q.w); // window coordinates
    return q;
}

// ---- modelToWindowCoordinates.glsl --------------------------------------------------------
fn czm_modelToWindowCoordinates(position: vec4<f32>) -> vec4<f32> {
    let positionEC = czm_modelView * position;
    var q = czm_projection * positionEC;
    q = vec4(q.xyz / vec3(q.w), q.w);                       // normalized device coordinates
    q = vec4((czm_viewportTransformation * vec4(q.xyz, 1.0)).xyz, q.w); // window coordinates
    return q;
}

// ---- windowToEyeCoordinates.glsl (incl. screenToEyeCoordinates) -----------------------------
fn czm_screenToEyeCoordinates(screenCoordinate: vec4<f32>) -> vec4<f32> {
    // Reconstruct NDC coordinates
    let x = 2.0 * screenCoordinate.x - 1.0;
    let y = 2.0 * screenCoordinate.y - 1.0;
    let z = (screenCoordinate.z - czm_viewportTransformation[3][2]) / czm_viewportTransformation[2][2];
    var q = vec4(x, y, z, 1.0);

    // Reverse the perspective division to obtain clip coordinates.
    q = q / vec4(screenCoordinate.w);

    // Reverse the projection transformation to obtain eye coordinates.
    q = czm_inverseProjection * q;
    return q;
}
fn czm_windowToEyeCoordinates(fragmentCoordinate: vec4<f32>) -> vec4<f32> {
    let screenCoordXY = (fragmentCoordinate.xy - czm_viewport.xy) / czm_viewport.zw;
    return czm_screenToEyeCoordinates(vec4(screenCoordXY, fragmentCoordinate.zw));
}
fn czm_screenToEyeCoordinates_depth(screenCoordinateXY: vec2<f32>, depthOrLogDepth: f32) -> vec4<f32> {
    // Non-LOG_DEPTH variant (see DEVIATION in file header).
    let screenCoord = vec4(screenCoordinateXY, depthOrLogDepth, 1.0);
    return czm_screenToEyeCoordinates(screenCoord);
}
fn czm_windowToEyeCoordinates_depth(fragmentCoordinateXY: vec2<f32>, depthOrLogDepth: f32) -> vec4<f32> {
    let screenCoordXY = (fragmentCoordinateXY.xy - czm_viewport.xy) / czm_viewport.zw;
    return czm_screenToEyeCoordinates_depth(screenCoordXY, depthOrLogDepth);
}

// ---- translateRelativeToEye.glsl --------------------------------------------------------------
fn czm_translateRelativeToEye(high: vec3<f32>, low: vec3<f32>) -> vec4<f32> {
    var highDifference = high - czm_encodedCameraPositionMCHigh;
    // This check handles the case when NaN values have gotten into `highDifference`.
    // Such a thing could happen on devices running iOS.
    if (length(highDifference) == 0.0) {
        highDifference = vec3(0.0);
    }
    let lowDifference = low - czm_encodedCameraPositionMCLow;

    return vec4(highDifference + lowDifference, 1.0);
}

// ---- metersPerPixel.glsl -------------------------------------------------------------------------
fn czm_metersPerPixel(positionEC: vec4<f32>, pixelRatio: f32) -> f32 {
    let width = czm_viewport.z;
    let height = czm_viewport.w;
    var pixelWidth: f32;
    var pixelHeight: f32;

    let top = czm_frustumPlanes.x;
    let bottom = czm_frustumPlanes.y;
    let left = czm_frustumPlanes.z;
    let right = czm_frustumPlanes.w;

    if (czm_sceneMode == czm_sceneMode2D || czm_orthographicIn3D == 1.0) {
        let frustumWidth = right - left;
        let frustumHeight = top - bottom;
        pixelWidth = frustumWidth / width;
        pixelHeight = frustumHeight / height;
    } else {
        let distanceToPixel = -positionEC.z;
        let inverseNear = 1.0 / czm_currentFrustum.x;
        var tanTheta = top * inverseNear;
        pixelHeight = 2.0 * distanceToPixel * tanTheta / height;
        tanTheta = right * inverseNear;
        pixelWidth = 2.0 * distanceToPixel * tanTheta / width;
    }

    return max(pixelWidth, pixelHeight) * pixelRatio;
}
fn czm_metersPerPixel_default(positionEC: vec4<f32>) -> f32 {
    return czm_metersPerPixel(positionEC, czm_pixelRatio);
}

// ---- alphaWeight.glsl ---------------------------------------------------------------------------------
fn czm_alphaWeight(a: f32, fragCoordZ: f32) -> f32 {
    // DEVIATION: GLSL reads gl_FragCoord.z implicitly (see file header).
    let z = (fragCoordZ - czm_viewportTransformation[3][2]) / czm_viewportTransformation[2][2];
    return pow(a + 0.01, 4.0) + max(1e-2, min(3.0 * 1e3, 0.003 / (1e-5 + pow(abs(z) / 200.0, 4.0))));
}

// ---- getDynamicAtmosphereLightDirection.glsl --------------------------------------------------------------
fn czm_getDynamicAtmosphereLightDirection(positionWC: vec3<f32>, lightEnum: f32) -> vec3<f32> {
    let NONE = 0.0;
    let SCENE_LIGHT = 1.0;
    let SUNLIGHT = 2.0;

    // GLSL float(lightEnum == X) mirrored with select.
    let lightDirection =
        positionWC * vec3(select(0.0, 1.0, lightEnum == NONE))
        + czm_lightDirectionWC * vec3(select(0.0, 1.0, lightEnum == SCENE_LIGHT))
        + czm_sunDirectionWC * vec3(select(0.0, 1.0, lightEnum == SUNLIGHT));
    return normalize(lightDirection);
}
