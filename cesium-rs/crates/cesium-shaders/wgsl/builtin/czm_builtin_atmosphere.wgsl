// WGSL port of the atmosphere/ray/fog cluster of
// packages/engine/Source/Shaders/Builtin/Functions/*.glsl (SH-01 task):
//   fog.glsl, pointAlongRay.glsl, isEmpty.glsl, isFull.glsl,
//   raySphereIntersectionInterval.glsl, rayEllipsoidIntersectionInterval.glsl,
//   computeScattering.glsl, computeAtmosphereColor.glsl,
//   computeGroundAtmosphereScattering.glsl
//
// DEVIATION: self-contained module — czm_ray / czm_raySegment structs,
// czm_emptyRaySegment/czm_fullRaySegment/czm_infinity constants,
// czm_approximateTanh, and the atmosphere/fog automatic uniforms are
// inlined / declared here.
// DEVIATION: GLSL `out` parameters of czm_computeScattering /
// czm_computeGroundAtmosphereScattering become the returned struct
// czm_scatteringResult.
// DEVIATION: GLSL `length(float)` equals abs; mirrored with abs().
// DEVIATION: GLSL struct equality against czm_emptyRaySegment is mirrored via
// czm_isEmpty (identical semantics: stop < 0.0).

const czm_infinity: f32 = 5906376272000.0;

@group(2) @binding(0) var<uniform> czm_viewerPositionWC: vec3<f32>;
@group(2) @binding(1) var<uniform> czm_atmosphereMieAnisotropy: f32;
@group(2) @binding(2) var<uniform> czm_atmosphereLightIntensity: vec3<f32>;
@group(2) @binding(3) var<uniform> czm_atmosphereRayleighScaleHeight: f32;
@group(2) @binding(4) var<uniform> czm_atmosphereMieScaleHeight: f32;
@group(2) @binding(5) var<uniform> czm_atmosphereRayleighCoefficient: vec3<f32>;
@group(2) @binding(6) var<uniform> czm_atmosphereMieCoefficient: f32;
@group(2) @binding(7) var<uniform> czm_fogDensity: f32;
@group(2) @binding(8) var<uniform> czm_inverseModelView: mat4x4<f32>;

// ---- inlined structs/constants (Builtin/Structs) -----------------------------
struct czm_ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}
struct czm_raySegment {
    start: f32,
    stop: f32,
}
var<private> czm_emptyRaySegment: czm_raySegment = czm_raySegment(-czm_infinity, -czm_infinity);
var<private> czm_fullRaySegment: czm_raySegment = czm_raySegment(0.0, czm_infinity);

// ---- fog.glsl --------------------------------------------------------------------
fn czm_fog(distanceToCamera: f32, color: vec3<f32>, fogColor: vec3<f32>) -> vec3<f32> {
    let scalar = distanceToCamera * czm_fogDensity;
    let fog = 1.0 - exp(-(scalar * scalar));
    return mix(color, fogColor, vec3(fog));
}
fn czm_fog_modified(distanceToCamera: f32, color: vec3<f32>, fogColor: vec3<f32>, fogModifierConstant: f32) -> vec3<f32> {
    let scalar = distanceToCamera * czm_fogDensity;
    let fog = 1.0 - exp(-((fogModifierConstant * scalar + fogModifierConstant)
        * (scalar * (1.0 + fogModifierConstant))));
    return mix(color, fogColor, vec3(fog));
}

// ---- pointAlongRay.glsl / isEmpty.glsl / isFull.glsl --------------------------------
fn czm_pointAlongRay(ray: czm_ray, time: f32) -> vec3<f32> {
    return ray.origin + vec3(time) * ray.direction;
}
fn czm_isEmpty(interval: czm_raySegment) -> bool {
    return interval.stop < 0.0;
}
fn czm_isFull(interval: czm_raySegment) -> bool {
    return interval.start == 0.0 && interval.stop == czm_infinity;
}

// ---- raySphereIntersectionInterval.glsl ----------------------------------------------
fn czm_raySphereIntersectionInterval(ray: czm_ray, center: vec3<f32>, radius: f32) -> czm_raySegment {
    let o = ray.origin;
    let d = ray.direction;

    let oc = o - center;

    let a = dot(d, d);
    let b = 2.0 * dot(d, oc);
    let c = dot(oc, oc) - (radius * radius);

    let det = (b * b) - (4.0 * a * c);

    if (det < 0.0) {
        return czm_emptyRaySegment;
    }

    let sqrtDet = sqrt(det);

    let t0 = (-b - sqrtDet) / (2.0 * a);
    let t1 = (-b + sqrtDet) / (2.0 * a);

    return czm_raySegment(t0, t1);
}

// ---- rayEllipsoidIntersectionInterval.glsl -----------------------------------------------
fn czm_rayEllipsoidIntersectionInterval(ray: czm_ray, ellipsoid_center: vec3<f32>, ellipsoid_inverseRadii: vec3<f32>) -> czm_raySegment {
    // ray and ellipsoid center in eye coordinates. radii in model coordinates.
    let q0 = ellipsoid_inverseRadii * (czm_inverseModelView * vec4(ray.origin, 1.0)).xyz;
    let w = ellipsoid_inverseRadii * (czm_inverseModelView * vec4(ray.direction, 0.0)).xyz;

    let q = q0 - ellipsoid_inverseRadii * (czm_inverseModelView * vec4(ellipsoid_center, 1.0)).xyz;

    let q2 = dot(q, q);
    let qw = dot(q, w);

    if (q2 > 1.0) { // Outside ellipsoid.
        if (qw >= 0.0) { // Looking outward or tangent (0 intersections).
            return czm_emptyRaySegment;
        } else { // qw < 0.0.
            let qw2 = qw * qw;
            let difference = q2 - 1.0; // Positively valued.
            let w2 = dot(w, w);
            let product = w2 * difference;

            if (qw2 < product) { // Imaginary roots (0 intersections).
                return czm_emptyRaySegment;
            } else if (qw2 > product) { // Distinct roots (2 intersections).
                let discriminant = qw * qw - product;
                let temp = -qw + sqrt(discriminant); // Avoid cancellation.
                let root0 = temp / w2;
                let root1 = difference / temp;
                if (root0 < root1) {
                    return czm_raySegment(root0, root1);
                } else {
                    return czm_raySegment(root1, root0);
                }
            } else { // qw2 == product. Repeated roots (2 intersections).
                let root = sqrt(difference / w2);
                return czm_raySegment(root, root);
            }
        }
    } else if (q2 < 1.0) { // Inside ellipsoid (2 intersections).
        let difference = q2 - 1.0; // Negatively valued.
        let w2 = dot(w, w);
        let product = w2 * difference; // Negatively valued.
        let discriminant = qw * qw - product;
        let temp = -qw + sqrt(discriminant); // Positively valued.
        return czm_raySegment(0.0, temp / w2);
    } else { // q2 == 1.0. On ellipsoid.
        if (qw < 0.0) { // Looking inward.
            let w2 = dot(w, w);
            return czm_raySegment(0.0, -qw / w2);
        } else { // qw >= 0.0. Looking outward or tangent.
            return czm_emptyRaySegment;
        }
    }
}

// ---- approximateTanh.glsl (private mirror, self-contained) -------------------------------
fn czm_private_approximateTanh(x: f32) -> f32 {
    let x2 = x * x;
    return max(-1.0, min(1.0, x * (27.0 + x2) / (27.0 + 9.0 * x2)));
}

// ---- computeScattering.glsl -------------------------------------------------------------------
struct czm_scatteringResult {
    rayleighColor: vec3<f32>,
    mieColor: vec3<f32>,
    opacity: f32,
}
fn czm_computeScattering(primaryRay: czm_ray, primaryRayLength: f32, lightDirection: vec3<f32>, atmosphereInnerRadius: f32) -> czm_scatteringResult {
    let ATMOSPHERE_THICKNESS = 111e3; // The thickness of the atmosphere in meters.
    let PRIMARY_STEPS_MAX = 16; // Maximum number of times the primary ray is sampled.
    let LIGHT_STEPS_MAX = 4; // Maximum number of times the light is sampled.

    var result: czm_scatteringResult;

    let atmosphereOuterRadius = atmosphereInnerRadius + ATMOSPHERE_THICKNESS;

    let origin = vec3(0.0);

    var primaryRayAtmosphereIntersect = czm_raySphereIntersectionInterval(primaryRay, origin, atmosphereOuterRadius);

    if (czm_isEmpty(primaryRayAtmosphereIntersect)) {
        return result;
    }

    // GLSL length(primaryRayLength) == abs for scalars.
    let x = 1e-7 * primaryRayAtmosphereIntersect.stop / abs(primaryRayLength);
    let w_stop_gt_lprl = 0.5 * (1.0 + czm_private_approximateTanh(x));

    let start_0 = primaryRayAtmosphereIntersect.start;
    primaryRayAtmosphereIntersect.start = max(primaryRayAtmosphereIntersect.start, 0.0);
    primaryRayAtmosphereIntersect.stop = min(primaryRayAtmosphereIntersect.stop, abs(primaryRayLength));

    // ATMOSPHERE_THICKNESS used as an ad-hoc constant, no precise meaning here,
    // only the order of magnitude matters.
    let x_o_a = start_0 - ATMOSPHERE_THICKNESS;
    let w_inside_atmosphere = 1.0 - 0.5 * (1.0 + czm_private_approximateTanh(x_o_a));
    let PRIMARY_STEPS = PRIMARY_STEPS_MAX - i32(w_inside_atmosphere * 12.0);
    let LIGHT_STEPS = LIGHT_STEPS_MAX - i32(w_inside_atmosphere * 2.0);

    var rayPositionLength = primaryRayAtmosphereIntersect.start;
    let totalRayLength = primaryRayAtmosphereIntersect.stop - rayPositionLength;
    let rayStepLengthIncrease = w_inside_atmosphere * ((1.0 - w_stop_gt_lprl) * totalRayLength
        / (f32(PRIMARY_STEPS * (PRIMARY_STEPS + 1)) / 2.0));
    var rayStepLength = max(1.0 - w_inside_atmosphere, w_stop_gt_lprl) * totalRayLength
        / max(7.0 * w_inside_atmosphere, f32(PRIMARY_STEPS));

    var rayleighAccumulation = vec3(0.0);
    var mieAccumulation = vec3(0.0);
    var opticalDepth = vec2(0.0);
    let heightScale = vec2(czm_atmosphereRayleighScaleHeight, czm_atmosphereMieScaleHeight);

    for (var i: i32 = 0; i < PRIMARY_STEPS_MAX; i = i + 1) {
        if (i >= PRIMARY_STEPS) {
            break;
        }

        let samplePosition = primaryRay.origin + primaryRay.direction * vec3(rayPositionLength + rayStepLength);

        let sampleHeight = length(samplePosition) - atmosphereInnerRadius;

        let sampleDensity = exp(vec2(-sampleHeight) / heightScale) * vec2(rayStepLength);
        opticalDepth = opticalDepth + sampleDensity;

        let lightRay = czm_ray(samplePosition, lightDirection);
        let lightRayAtmosphereIntersect = czm_raySphereIntersectionInterval(lightRay, origin, atmosphereOuterRadius);

        let lightStepLength = lightRayAtmosphereIntersect.stop / f32(LIGHT_STEPS);
        var lightPositionLength = 0.0;

        var lightOpticalDepth = vec2(0.0);

        for (var j: i32 = 0; j < LIGHT_STEPS_MAX; j = j + 1) {
            if (j >= LIGHT_STEPS) {
                break;
            }

            let lightPosition = samplePosition + lightDirection * vec3(lightPositionLength + lightStepLength * 0.5);

            let lightHeight = length(lightPosition) - atmosphereInnerRadius;

            lightOpticalDepth = lightOpticalDepth + exp(vec2(-lightHeight) / heightScale) * vec2(lightStepLength);

            lightPositionLength = lightPositionLength + lightStepLength;
        }

        let attenuation = exp(-((czm_atmosphereMieCoefficient * (opticalDepth.y + lightOpticalDepth.y))
            + (czm_atmosphereRayleighCoefficient * vec3(opticalDepth.x + lightOpticalDepth.x))));

        rayleighAccumulation = rayleighAccumulation + sampleDensity.x * attenuation;
        mieAccumulation = mieAccumulation + sampleDensity.y * attenuation;

        // GLSL: rayPositionLength += (rayStepLength += rayStepLengthIncrease);
        rayStepLength = rayStepLength + rayStepLengthIncrease;
        rayPositionLength = rayPositionLength + rayStepLength;
    }

    result.rayleighColor = czm_atmosphereRayleighCoefficient * rayleighAccumulation;
    result.mieColor = czm_atmosphereMieCoefficient * mieAccumulation;

    // GLSL: length(exp(-(mie * od.y + rayleigh * od.x))), vec3 argument.
    result.opacity = length(exp(-((czm_atmosphereMieCoefficient * vec3(opticalDepth.y))
        + (czm_atmosphereRayleighCoefficient * vec3(opticalDepth.x)))));
    return result;
}

// ---- computeAtmosphereColor.glsl -------------------------------------------------------------------
fn czm_computeAtmosphereColor(positionWC: vec3<f32>, lightDirection: vec3<f32>, rayleighColor: vec3<f32>, mieColor: vec3<f32>, opacity: f32) -> vec4<f32> {
    let cameraToPositionWC = positionWC - czm_viewerPositionWC;
    let cameraToPositionWCDirection = normalize(cameraToPositionWC);

    let cosAngle = dot(cameraToPositionWCDirection, lightDirection);
    let cosAngleSq = cosAngle * cosAngle;

    let G = czm_atmosphereMieAnisotropy;
    let GSq = G * G;

    let rayleighPhase = 3.0 / (50.2654824574) * (1.0 + cosAngleSq);
    let miePhase = 3.0 / (25.1327412287) * ((1.0 - GSq) * (cosAngleSq + 1.0))
        / (pow(1.0 + GSq - 2.0 * cosAngle * G, 1.5) * (2.0 + GSq));

    let rayleigh = rayleighPhase * rayleighColor;
    let mie = miePhase * mieColor;

    let color = (rayleigh + mie) * czm_atmosphereLightIntensity;

    return vec4(color, opacity);
}
fn czm_computeAtmosphereColor_ray(primaryRay: czm_ray, lightDirection: vec3<f32>, rayleighColor: vec3<f32>, mieColor: vec3<f32>, opacity: f32) -> vec4<f32> {
    let direction = normalize(primaryRay.direction);

    let cosAngle = dot(direction, lightDirection);
    let cosAngleSq = cosAngle * cosAngle;

    let G = czm_atmosphereMieAnisotropy;
    let GSq = G * G;

    let rayleighPhase = 3.0 / (50.2654824574) * (1.0 + cosAngleSq);
    let miePhase = 3.0 / (25.1327412287) * ((1.0 - GSq) * (cosAngleSq + 1.0))
        / (pow(1.0 + GSq - 2.0 * cosAngle * G, 1.5) * (2.0 + GSq));

    let rayleigh = rayleighPhase * rayleighColor;
    let mie = miePhase * mieColor;

    let color = (rayleigh + mie) * czm_atmosphereLightIntensity;

    return vec4(color, opacity);
}

// ---- computeGroundAtmosphereScattering.glsl -------------------------------------------------------------
fn czm_computeGroundAtmosphereScattering(positionWC: vec3<f32>, lightDirection: vec3<f32>) -> czm_scatteringResult {
    let cameraToPositionWC = positionWC - czm_viewerPositionWC;
    let cameraToPositionWCDirection = normalize(cameraToPositionWC);
    let primaryRay = czm_ray(czm_viewerPositionWC, cameraToPositionWCDirection);

    let atmosphereInnerRadius = length(positionWC);

    return czm_computeScattering(
        primaryRay,
        length(cameraToPositionWC),
        lightDirection,
        atmosphereInnerRadius);
}
