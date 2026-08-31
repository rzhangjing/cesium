// WGSL port of the color-space cluster of
// packages/engine/Source/Shaders/Builtin/Functions/*.glsl (SH-01 task):
//   HSBToRGB.glsl, RGBToHSB.glsl, HSLToRGB.glsl, RGBToHSL.glsl, hue.glsl,
//   saturation.glsl, luminance.glsl, multiplyWithColorBalance.glsl,
//   linearToSrgb.glsl, srgbToLinear.glsl, applyHSBShift.glsl,
//   acesTonemapping.glsl, pbrNeutralTonemapping.glsl, RGBToXYZ.glsl,
//   XYZToRGB.glsl, gammaCorrect.glsl, inverseGamma.glsl
//
// DEVIATION: WGSL has no function overloading; vec4 overloads get the _vec4
// suffix.
// DEVIATION: GLSL `step(edge, x)` is mirrored with `select(0.0, 1.0, ...)`.
// DEVIATION: GLSL two-argument `atan(y, x)` maps to WGSL `atan2(y, x)`.
// DEVIATION: czm_gammaCorrect is mirrored in its `#ifdef HDR` active form
// (the inactive no-op form is not expressible without a preprocessor).
// DEVIATION: `czm_gamma` is a CesiumJS automatic uniform; mirrored here as an
// explicit uniform binding (group(2), see builtin_wgsl.rs module docs).

const czm_epsilon7: f32 = 0.0000001;

@group(2) @binding(0) var<uniform> czm_gamma: f32;

// ---- HSBToRGB.glsl -----------------------------------------------------------
const K_HSB2RGB: vec4<f32> = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
fn czm_HSBToRGB(hsb: vec3<f32>) -> vec3<f32> {
    let p = abs(fract(hsb.xxx + K_HSB2RGB.xyz) * vec3(6.0) - K_HSB2RGB.www);
    return hsb.z * mix(K_HSB2RGB.xxx, clamp(p - K_HSB2RGB.xxx, vec3(0.0), vec3(1.0)), vec3(hsb.y));
}

// ---- RGBToHSB.glsl -------------------------------------------------------------
const K_RGB2HSB: vec4<f32> = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
fn czm_RGBToHSB(rgb: vec3<f32>) -> vec3<f32> {
    // GLSL step(rgb.b, rgb.g) == 1.0 when rgb.g >= rgb.b
    let p = mix(vec4(rgb.bg, K_RGB2HSB.wz), vec4(rgb.gb, K_RGB2HSB.xy),
        select(0.0, 1.0, rgb.g >= rgb.b));
    let q = mix(vec4(p.xyw, rgb.r), vec4(rgb.r, p.yzx), select(0.0, 1.0, p.x >= rgb.r));
    let d = q.x - min(q.w, q.y);
    return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + czm_epsilon7)),
        d / (q.x + czm_epsilon7), q.x);
}

// ---- HSLToRGB.glsl ---------------------------------------------------------------
fn czm_private_hueToRGB(hue: f32) -> vec3<f32> {
    let r = abs(hue * 6.0 - 3.0) - 1.0;
    let g = 2.0 - abs(hue * 6.0 - 2.0);
    let b = 2.0 - abs(hue * 6.0 - 4.0);
    return clamp(vec3(r, g, b), vec3(0.0), vec3(1.0));
}
fn czm_HSLToRGB(hsl: vec3<f32>) -> vec3<f32> {
    let rgb = czm_private_hueToRGB(hsl.x);
    let c = (1.0 - abs(2.0 * hsl.z - 1.0)) * hsl.y;
    return (rgb - vec3(0.5)) * vec3(c) + vec3(hsl.z);
}

// ---- RGBToHSL.glsl -----------------------------------------------------------------
fn czm_private_RGBtoHCV(rgb: vec3<f32>) -> vec3<f32> {
    // Based on work by Sam Hocevar and Emil Persson.
    // DEVIATION: GLSL ternaries mirrored with select (identical semantics).
    let p = select(vec4(rgb.gb, 0.0, -1.0 / 3.0), vec4(rgb.bg, -1.0, 2.0 / 3.0), rgb.g < rgb.b);
    let q = select(vec4(rgb.r, p.yzx), vec4(p.xyw, rgb.r), rgb.r < p.x);
    let c = q.x - min(q.w, q.y);
    let h = abs((q.w - q.y) / (6.0 * c + czm_epsilon7) + q.z);
    return vec3(h, c, q.x);
}
fn czm_RGBToHSL(rgb: vec3<f32>) -> vec3<f32> {
    let hcv = czm_private_RGBtoHCV(rgb);
    let l = hcv.z - hcv.y * 0.5;
    let s = hcv.y / (1.0 - abs(l * 2.0 - 1.0) + czm_epsilon7);
    return vec3(hcv.x, s, l);
}

// ---- hue.glsl --------------------------------------------------------------------------
const czm_hue_toYIQ: mat3x3<f32> = mat3x3(
    vec3(0.299,     0.587,     0.114),
    vec3(0.595716, -0.274453, -0.321263),
    vec3(0.211456, -0.522591,  0.311135));
const czm_hue_toRGB: mat3x3<f32> = mat3x3(
    vec3(1.0,  0.9563,  0.6210),
    vec3(1.0, -0.2721, -0.6474),
    vec3(1.0, -1.107,   1.7046));
fn czm_hue(rgb: vec3<f32>, adjustment: f32) -> vec3<f32> {
    let yiq = czm_hue_toYIQ * rgb;
    let hueShifted = atan2(yiq.z, yiq.y) + adjustment;
    let chroma = sqrt(yiq.z * yiq.z + yiq.y * yiq.y);

    let color = vec3(yiq.x, chroma * cos(hueShifted), chroma * sin(hueShifted));
    return czm_hue_toRGB * color;
}

// ---- saturation.glsl ----------------------------------------------------------------------
fn czm_saturation(rgb: vec3<f32>, adjustment: f32) -> vec3<f32> {
    // Algorithm from Chapter 16 of OpenGL Shading Language
    let W = vec3(0.2125, 0.7154, 0.0721);
    let intensity = vec3(dot(rgb, W));
    return mix(intensity, rgb, vec3(adjustment));
}

// ---- luminance.glsl -------------------------------------------------------------------------
fn czm_luminance(rgb: vec3<f32>) -> f32 {
    let W = vec3(0.2125, 0.7154, 0.0721);
    return dot(rgb, W);
}

// ---- multiplyWithColorBalance.glsl -------------------------------------------------------------
fn czm_multiplyWithColorBalance(left: vec3<f32>, right: vec3<f32>) -> vec3<f32> {
    // Algorithm from Chapter 10 of Graphics Shaders.
    let W = vec3(0.2125, 0.7154, 0.0721);

    let target = left * right;
    let leftLuminance = dot(left, W);
    let rightLuminance = dot(right, W);
    let targetLuminance = dot(target, W);

    return ((leftLuminance + rightLuminance) / (2.0 * targetLuminance)) * target;
}

// ---- linearToSrgb.glsl / srgbToLinear.glsl --------------------------------------------------------
fn czm_linearToSrgb(linearIn: vec3<f32>) -> vec3<f32> {
    return pow(linearIn, vec3(1.0 / 2.2));
}
fn czm_linearToSrgb_vec4(linearIn: vec4<f32>) -> vec4<f32> {
    let srgbOut = pow(linearIn.rgb, vec3(1.0 / 2.2));
    return vec4(srgbOut, linearIn.a);
}
fn czm_srgbToLinear(srgbIn: vec3<f32>) -> vec3<f32> {
    return pow(srgbIn, vec3(2.2));
}
fn czm_srgbToLinear_vec4(srgbIn: vec4<f32>) -> vec4<f32> {
    let linearOut = pow(srgbIn.rgb, vec3(2.2));
    return vec4(linearOut, srgbIn.a);
}

// ---- applyHSBShift.glsl -----------------------------------------------------------------------------
fn czm_applyHSBShift(rgb: vec3<f32>, hsbShift: vec3<f32>, ignoreBlackPixels: bool) -> vec3<f32> {
    var hsb = czm_RGBToHSB(rgb);

    hsb.x = hsb.x + hsbShift.x; // hue
    hsb.y = clamp(hsb.y + hsbShift.y, 0.0, 1.0); // saturation

    if (ignoreBlackPixels) {
        hsb.z = select(0.0, hsb.z + hsbShift.z, hsb.z > czm_epsilon7);
    } else {
        hsb.z = hsb.z + hsbShift.z;
    }
    hsb.z = clamp(hsb.z, 0.0, 1.0);

    return czm_HSBToRGB(hsb);
}

// ---- acesTonemapping.glsl -------------------------------------------------------------------------------
fn czm_acesTonemapping(color_in: vec3<f32>) -> vec3<f32> {
    let g = 0.985;
    let a = 0.065;
    let b = 0.0001;
    let c = 0.433;
    let d = 0.238;

    var color = (color_in * (color_in + vec3(a)) - vec3(b)) / (color_in * (vec3(g) * color_in + vec3(c)) + vec3(d));
    color = clamp(color, vec3(0.0), vec3(1.0));
    return color;
}

// ---- pbrNeutralTonemapping.glsl --------------------------------------------------------------------------
fn czm_private_branchFreeTernary_f32(comparison: bool, a: f32, b: f32) -> f32 {
    // Private mirror of branchFreeTernary.glsl (self-contained module).
    let useA = select(0.0, 1.0, comparison);
    return a * useA + b * (1.0 - useA);
}
fn czm_pbrNeutralTonemapping(color_in: vec3<f32>) -> vec3<f32> {
    // KhronosGroup https://github.com/KhronosGroup/ToneMapping/tree/main/PBR_Neutral
    let startCompression = 0.8 - 0.04;
    let desaturation = 0.15;

    var color = color_in;
    let x = min(color.r, min(color.g, color.b));
    let offset = czm_private_branchFreeTernary_f32(x < 0.08, x - 6.25 * x * x, 0.04);
    color = color - vec3(offset);

    let peak = max(color.r, max(color.g, color.b));
    if (peak < startCompression) {
        return color;
    }

    let d = 1.0 - startCompression;
    let newPeak = 1.0 - d * d / (peak + d - startCompression);
    color = color * vec3(newPeak / peak);

    let g = 1.0 - 1.0 / (desaturation * (peak - newPeak) + 1.0);
    return mix(color, vec3(newPeak), vec3(g));
}

// ---- RGBToXYZ.glsl ------------------------------------------------------------------------------------------
const czm_RGB2XYZ: mat3x3<f32> = mat3x3(
    vec3(0.4124, 0.2126, 0.0193),
    vec3(0.3576, 0.7152, 0.1192),
    vec3(0.1805, 0.0722, 0.9505));
fn czm_RGBToXYZ(rgb: vec3<f32>) -> vec3<f32> {
    let xyz = czm_RGB2XYZ * rgb;
    // DEVIATION: GLSL swizzle assignment `Yxy.gb = ...` mirrored explicitly.
    let temp = dot(vec3(1.0), xyz);
    return vec3(xyz.g, xyz.r / temp, xyz.g / temp);
}

// ---- XYZToRGB.glsl --------------------------------------------------------------------------------------------
const czm_XYZ2RGB: mat3x3<f32> = mat3x3(
    vec3( 3.2405, -0.9693,  0.0556),
    vec3(-1.5371,  1.8760, -0.2040),
    vec3(-0.4985,  0.0416,  1.0572));
fn czm_XYZToRGB(Yxy: vec3<f32>) -> vec3<f32> {
    let xyz = vec3(Yxy.r * Yxy.g / Yxy.b, Yxy.r, Yxy.r * (1.0 - Yxy.g - Yxy.b) / Yxy.b);
    return czm_XYZ2RGB * xyz;
}

// ---- gammaCorrect.glsl -----------------------------------------------------------------------------------------
fn czm_gammaCorrect(color: vec3<f32>) -> vec3<f32> {
    // HDR variant active (see DEVIATION in file header).
    return pow(color, vec3(czm_gamma));
}
fn czm_gammaCorrect_vec4(color: vec4<f32>) -> vec4<f32> {
    return vec4(pow(color.rgb, vec3(czm_gamma)), color.a);
}

// ---- inverseGamma.glsl --------------------------------------------------------------------------------------------
fn czm_inverseGamma(color: vec3<f32>) -> vec3<f32> {
    return pow(color, vec3(1.0 / czm_gamma));
}
