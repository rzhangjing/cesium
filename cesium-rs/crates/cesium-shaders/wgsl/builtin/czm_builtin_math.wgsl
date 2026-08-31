// WGSL port of the math/interpolation cluster of
// packages/engine/Source/Shaders/Builtin/Functions/*.glsl (SH-01 task):
//   branchFreeTernary.glsl, maximumComponent.glsl, transpose.glsl,
//   valueTransform.glsl, readNonPerspective.glsl, writeNonPerspective.glsl,
//   lineDistance.glsl, planeDistance.glsl, nearFarScalar.glsl,
//   latitudeToWebMercatorFraction.glsl, approximateTanh.glsl,
//   fastApproximateAtan.glsl, approximateSphericalCoordinates.glsl,
//   columbusViewMorph.glsl, cosineAndSine.glsl, computeTextureTransform.glsl,
//   tangentToEyeSpaceMatrix.glsl, transformPlane.glsl,
//   sphericalHarmonics.glsl, geodeticSurfaceNormal.glsl, equalsEpsilon.glsl
//
// DEVIATION: WGSL has no function overloading; GLSL overloads are mirrored
// with type suffixes (_vec2/_vec3/_vec4, 2-arg atan → czm_fastApproximateAtan2).
// DEVIATION: GLSL `float(bool)` inside czm_branchFreeTernary is mirrored with
// WGSL `select`; the branch-free property is preserved.
// DEVIATION: GLSL matrixCompMult (component-wise matrix product) has no WGSL
// builtin; mirrored column-wise in czm_valueTransform_mat2/3/4.
// DEVIATION: czm_transpose mirrors GLSL via the WGSL `transpose` builtin
// (identical semantics).
// DEVIATION: czm_cosineAndSine's GLSL body is fully unrolled (23 CORDIC
// iterations); mirrored as a data-driven loop over the identical constant
// table with the identical operation sequence.

const czm_pi: f32 = 3.141592653589793;
const czm_piOverTwo: f32 = 1.5707963267948966;

// ---- branchFreeTernary.glsl -------------------------------------------------
fn czm_branchFreeTernary(comparison: bool, a: f32, b: f32) -> f32 {
    let useA = select(0.0, 1.0, comparison);
    return a * useA + b * (1.0 - useA);
}
fn czm_branchFreeTernary_vec2(comparison: bool, a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    let useA = select(0.0, 1.0, comparison);
    return a * vec2(useA) + b * vec2(1.0 - useA);
}
fn czm_branchFreeTernary_vec3(comparison: bool, a: vec3<f32>, b: vec3<f32>) -> vec3<f32> {
    let useA = select(0.0, 1.0, comparison);
    return a * vec3(useA) + b * vec3(1.0 - useA);
}
fn czm_branchFreeTernary_vec4(comparison: bool, a: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    let useA = select(0.0, 1.0, comparison);
    return a * vec4(useA) + b * vec4(1.0 - useA);
}

// ---- maximumComponent.glsl ----------------------------------------------------
fn czm_maximumComponent_vec2(v: vec2<f32>) -> f32 {
    return max(v.x, v.y);
}
fn czm_maximumComponent_vec3(v: vec3<f32>) -> f32 {
    return max(max(v.x, v.y), v.z);
}
fn czm_maximumComponent_vec4(v: vec4<f32>) -> f32 {
    return max(max(max(v.x, v.y), v.z), v.w);
}

// ---- transpose.glsl -------------------------------------------------------------
fn czm_transpose_mat2(matrix: mat2x2<f32>) -> mat2x2<f32> {
    return transpose(matrix);
}
fn czm_transpose_mat3(matrix: mat3x3<f32>) -> mat3x3<f32> {
    return transpose(matrix);
}
fn czm_transpose_mat4(matrix: mat4x4<f32>) -> mat4x4<f32> {
    return transpose(matrix);
}

// ---- valueTransform.glsl ----------------------------------------------------------
fn czm_valueTransform(offset: f32, scale: f32, value: f32) -> f32 {
    return scale * value + offset;
}
fn czm_valueTransform_vec2(offset: vec2<f32>, scale: vec2<f32>, value: vec2<f32>) -> vec2<f32> {
    return scale * value + offset;
}
fn czm_valueTransform_vec3(offset: vec3<f32>, scale: vec3<f32>, value: vec3<f32>) -> vec3<f32> {
    return scale * value + offset;
}
fn czm_valueTransform_vec4(offset: vec4<f32>, scale: vec4<f32>, value: vec4<f32>) -> vec4<f32> {
    return scale * value + offset;
}
fn czm_valueTransform_mat2(offset: mat2x2<f32>, scale: mat2x2<f32>, value: mat2x2<f32>) -> mat2x2<f32> {
    // GLSL matrixCompMult: component-wise multiplication.
    return mat2x2(scale[0] * value[0], scale[1] * value[1]) + offset;
}
fn czm_valueTransform_mat3(offset: mat3x3<f32>, scale: mat3x3<f32>, value: mat3x3<f32>) -> mat3x3<f32> {
    return mat3x3(scale[0] * value[0], scale[1] * value[1], scale[2] * value[2]) + offset;
}
fn czm_valueTransform_mat4(offset: mat4x4<f32>, scale: mat4x4<f32>, value: mat4x4<f32>) -> mat4x4<f32> {
    return mat4x4(scale[0] * value[0], scale[1] * value[1],
        scale[2] * value[2], scale[3] * value[3]) + offset;
}

// ---- readNonPerspective.glsl / writeNonPerspective.glsl ---------------------------
fn czm_readNonPerspective(value: f32, oneOverW: f32) -> f32 {
    return value * oneOverW;
}
fn czm_readNonPerspective_vec2(value: vec2<f32>, oneOverW: f32) -> vec2<f32> {
    return value * vec2(oneOverW);
}
fn czm_readNonPerspective_vec3(value: vec3<f32>, oneOverW: f32) -> vec3<f32> {
    return value * vec3(oneOverW);
}
fn czm_readNonPerspective_vec4(value: vec4<f32>, oneOverW: f32) -> vec4<f32> {
    return value * vec4(oneOverW);
}
fn czm_writeNonPerspective(value: f32, w: f32) -> f32 {
    return value * w;
}
fn czm_writeNonPerspective_vec2(value: vec2<f32>, w: f32) -> vec2<f32> {
    return value * vec2(w);
}
fn czm_writeNonPerspective_vec3(value: vec3<f32>, w: f32) -> vec3<f32> {
    return value * vec3(w);
}
fn czm_writeNonPerspective_vec4(value: vec4<f32>, w: f32) -> vec4<f32> {
    return value * vec4(w);
}

// ---- lineDistance.glsl ----------------------------------------------------------------
fn czm_lineDistance(point1: vec2<f32>, point2: vec2<f32>, point: vec2<f32>) -> f32 {
    return abs((point2.y - point1.y) * point.x - (point2.x - point1.x) * point.y
        + point2.x * point1.y - point2.y * point1.x) / distance(point2, point1);
}

// ---- planeDistance.glsl -----------------------------------------------------------------
fn czm_planeDistance(plane: vec4<f32>, point: vec3<f32>) -> f32 {
    return dot(plane.xyz, point) + plane.w;
}
fn czm_planeDistance_nd(planeNormal: vec3<f32>, planeDistanceIn: f32, point: vec3<f32>) -> f32 {
    return dot(planeNormal, point) + planeDistanceIn;
}

// ---- nearFarScalar.glsl -------------------------------------------------------------------
fn czm_nearFarScalar(near_far_scalar: vec4<f32>, cameraDistSq: f32) -> f32 {
    let valueAtMin = near_far_scalar.y;
    let valueAtMax = near_far_scalar.w;
    let nearDistanceSq = near_far_scalar.x * near_far_scalar.x;
    let farDistanceSq = near_far_scalar.z * near_far_scalar.z;

    var t = (cameraDistSq - nearDistanceSq) / (farDistanceSq - nearDistanceSq);
    t = pow(clamp(t, 0.0, 1.0), 0.2);
    return mix(valueAtMin, valueAtMax, t);
}

// ---- latitudeToWebMercatorFraction.glsl ------------------------------------------------------
fn czm_latitudeToWebMercatorFraction(latitude: f32, southMercatorY: f32, oneOverMercatorHeight: f32) -> f32 {
    let sinLatitude = sin(latitude);
    let mercatorY = 0.5 * log((1.0 + sinLatitude) / (1.0 - sinLatitude));
    return (mercatorY - southMercatorY) * oneOverMercatorHeight;
}

// ---- approximateTanh.glsl -----------------------------------------------------------------------
fn czm_approximateTanh(x: f32) -> f32 {
    let x2 = x * x;
    return max(-1.0, min(1.0, x * (27.0 + x2) / (27.0 + 9.0 * x2)));
}

// ---- fastApproximateAtan.glsl ---------------------------------------------------------------------
fn czm_fastApproximateAtan(x: f32) -> f32 {
    return x * (-0.1784 * x - 0.0663 * x * x + 1.0301);
}
fn czm_fastApproximateAtan2(x: f32, y: f32) -> f32 {
    var t = abs(x); // t used as swap and atan result.
    var opposite = abs(y);
    let adjacent = max(t, opposite);
    opposite = min(t, opposite);

    t = czm_fastApproximateAtan(opposite / adjacent);

    t = czm_branchFreeTernary(abs(y) > abs(x), czm_piOverTwo - t, t);
    t = czm_branchFreeTernary(x < 0.0, czm_pi - t, t);
    t = czm_branchFreeTernary(y < 0.0, -t, t);
    return t;
}

// ---- approximateSphericalCoordinates.glsl ------------------------------------------------------------
fn czm_approximateSphericalCoordinates(normal: vec3<f32>) -> vec2<f32> {
    let latitudeApproximation = czm_fastApproximateAtan2(
        sqrt(normal.x * normal.x + normal.y * normal.y), normal.z);
    let longitudeApproximation = czm_fastApproximateAtan2(normal.x, normal.y);
    return vec2(latitudeApproximation, longitudeApproximation);
}

// ---- columbusViewMorph.glsl ---------------------------------------------------------------------------
fn czm_columbusViewMorph(position2D: vec4<f32>, position3D: vec4<f32>, time: f32) -> vec4<f32> {
    let p = position2D.xyz * vec3(1.0 - time) + position3D.xyz * vec3(time);
    return vec4(p, 1.0);
}

// ---- cosineAndSine.glsl (CORDIC) ------------------------------------------------------------------------
fn czm_private_cordic(angle_in: f32) -> vec2<f32> {
    var angle = angle_in;
    var vector = vec2(6.0725293500888267e-1, 0.0);
    var sense = select(1.0, -1.0, angle < 0.0);
    var rotation = mat2x2(vec2(1.0, sense), vec2(-sense, 1.0));
    vector = rotation * vector;
    angle = angle - sense * 7.8539816339744828e-1; // atan(2^-0)

    // atan(2^-1) .. atan(2^-23); mirrors the GLSL unrolled iterations.
    let atanTable = array<f32, 23>(
        4.6364760900080609e-1, 2.4497866312686414e-1, 1.2435499454676144e-1,
        6.2418809995957350e-2, 3.1239833430268277e-2, 1.5623728620476831e-2,
        7.8123410601011111e-3, 3.9062301319669718e-3, 1.9531225164788188e-3,
        9.7656218955931946e-4, 4.8828121119489829e-4, 2.4414062014936177e-4,
        1.2207031189367021e-4, 6.1035156174208773e-5, 3.0517578115526096e-5,
        1.5258789061315762e-5, 7.6293945311019700e-6, 3.8146972656064961e-6,
        1.9073486328101870e-6, 9.5367431640596084e-7, 4.7683715820308884e-7,
        2.3841857910155797e-7, 1.1920928955078125e-7);
    for (var i = 0; i < 23; i = i + 1) {
        sense = select(1.0, -1.0, angle < 0.0);
        let factor = sense * exp2(-f32(i + 1)); // 2^-(i+1)
        rotation[0][1] = factor;
        rotation[1][0] = -factor;
        vector = rotation * vector;
        angle = angle - sense * atanTable[i];
    }
    return vector;
}

fn czm_cosineAndSine(angle: f32) -> vec2<f32> {
    if (angle < -czm_piOverTwo || angle > czm_piOverTwo) {
        if (angle < 0.0) {
            return -czm_private_cordic(angle + czm_pi);
        } else {
            return -czm_private_cordic(angle - czm_pi);
        }
    } else {
        return czm_private_cordic(angle);
    }
}

// ---- computeTextureTransform.glsl ----------------------------------------------------------------------
fn czm_computeTextureTransform(texCoord: vec2<f32>, textureTransform: mat3x3<f32>) -> vec2<f32> {
    return (textureTransform * vec3(texCoord, 1.0)).xy;
}

// ---- tangentToEyeSpaceMatrix.glsl ------------------------------------------------------------------------
fn czm_tangentToEyeSpaceMatrix(normalEC: vec3<f32>, tangentEC: vec3<f32>, bitangentEC: vec3<f32>) -> mat3x3<f32> {
    let normal = normalize(normalEC);
    let tangent = normalize(tangentEC);
    let bitangent = normalize(bitangentEC);
    // GLSL mat3(tangent.xyz, bitangent.xyz, normal.xyz) — column-major, same as WGSL.
    return mat3x3(tangent, bitangent, normal);
}

// ---- transformPlane.glsl ------------------------------------------------------------------------------------
fn czm_transformPlane(plane: vec4<f32>, transform: mat4x4<f32>) -> vec4<f32> {
    let transformedPlane = transform * plane;
    // Convert the transformed plane to Hessian Normal Form
    let normalMagnitude = length(transformedPlane.xyz);
    return transformedPlane / vec4(normalMagnitude);
}

// ---- sphericalHarmonics.glsl -----------------------------------------------------------------------------------
fn czm_sphericalHarmonics(normal: vec3<f32>, coefficients: array<vec3<f32>, 9>) -> vec3<f32> {
    let L00 = coefficients[0];
    let L1_1 = coefficients[1];
    let L10 = coefficients[2];
    let L11 = coefficients[3];
    let L2_2 = coefficients[4];
    let L2_1 = coefficients[5];
    let L20 = coefficients[6];
    let L21 = coefficients[7];
    let L22 = coefficients[8];

    let x = normal.x;
    let y = normal.y;
    let z = normal.z;

    let L =
          L00
        + L1_1 * vec3(y)
        + L10 * vec3(z)
        + L11 * vec3(x)
        + L2_2 * vec3(y * x)
        + L2_1 * vec3(y * z)
        + L20 * vec3(3.0 * z * z - 1.0)
        + L21 * vec3(z * x)
        + L22 * vec3(x * x - y * y);

    return max(L, vec3(0.0));
}

// ---- geodeticSurfaceNormal.glsl -----------------------------------------------------------------------------------
fn czm_geodeticSurfaceNormal(positionOnEllipsoid: vec3<f32>, ellipsoidCenter: vec3<f32>, oneOverEllipsoidRadiiSquared: vec3<f32>) -> vec3<f32> {
    return normalize((positionOnEllipsoid - ellipsoidCenter) * oneOverEllipsoidRadiiSquared);
}

// ---- equalsEpsilon.glsl -----------------------------------------------------------------------------------------------
fn czm_equalsEpsilon(left: f32, right: f32, epsilon: f32) -> bool {
    return abs(left - right) <= epsilon;
}
fn czm_equalsEpsilon_vec2(left: vec2<f32>, right: vec2<f32>, epsilon: f32) -> bool {
    return all(abs(left - right) <= vec2(epsilon));
}
fn czm_equalsEpsilon_vec3(left: vec3<f32>, right: vec3<f32>, epsilon: f32) -> bool {
    return all(abs(left - right) <= vec3(epsilon));
}
fn czm_equalsEpsilon_vec4(left: vec4<f32>, right: vec4<f32>, epsilon: f32) -> bool {
    return all(abs(left - right) <= vec4(epsilon));
}
