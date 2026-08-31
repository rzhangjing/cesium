// WGSL port of the pack/unpack cluster of
// packages/engine/Source/Shaders/Builtin/Functions/*.glsl (SH-01 task):
//   round.glsl, signNotZero.glsl, packDepth.glsl, unpackDepth.glsl,
//   unpackFloat.glsl, unpackTexture.glsl, unpackUint.glsl, decodeRGB8.glsl,
//   decompressTextureCoordinates.glsl, octDecode.glsl
//
// DEVIATION: WGSL has no function overloading; GLSL overloads are mirrored
// with type suffixes (_vec2/_vec3/_vec4/_f32/_range/_triple).
// DEVIATION: GLSL `out` parameters of czm_octDecode(vec2, out, out, out)
// become a returned struct czm_octDecodeTriple.
// DEVIATION: GLSL `mod(x, y)` (floored remainder) has no WGSL builtin;
// mirrored by the private czm_mod_f32 helper below.

// ---- round.glsl ----------------------------------------------------------
fn czm_round(value: f32) -> f32 {
    return floor(value + 0.5);
}
fn czm_round_vec2(value: vec2<f32>) -> vec2<f32> {
    return floor(value + vec2(0.5));
}
fn czm_round_vec3(value: vec3<f32>) -> vec3<f32> {
    return floor(value + vec3(0.5));
}
fn czm_round_vec4(value: vec4<f32>) -> vec4<f32> {
    return floor(value + vec4(0.5));
}

// ---- signNotZero.glsl ------------------------------------------------------
fn czm_signNotZero(value: f32) -> f32 {
    return select(-1.0, 1.0, value >= 0.0);
}
fn czm_signNotZero_vec2(value: vec2<f32>) -> vec2<f32> {
    return vec2(czm_signNotZero(value.x), czm_signNotZero(value.y));
}
fn czm_signNotZero_vec3(value: vec3<f32>) -> vec3<f32> {
    return vec3(czm_signNotZero(value.x), czm_signNotZero(value.y), czm_signNotZero(value.z));
}
fn czm_signNotZero_vec4(value: vec4<f32>) -> vec4<f32> {
    return vec4(czm_signNotZero(value.x), czm_signNotZero(value.y),
        czm_signNotZero(value.z), czm_signNotZero(value.w));
}

// ---- packDepth.glsl ----------------------------------------------------------
fn czm_packDepth(depth: f32) -> vec4<f32> {
    // See Aras Pranckevičius' post Encoding Floats to RGBA
    var enc = vec4(1.0, 255.0, 65025.0, 16581375.0) * depth;
    enc = fract(enc);
    enc = enc - enc.yzww * vec4(1.0 / 255.0, 1.0 / 255.0, 1.0 / 255.0, 0.0);
    return enc;
}

// ---- unpackDepth.glsl --------------------------------------------------------
fn czm_unpackDepth(packedDepth: vec4<f32>) -> f32 {
    return dot(packedDepth, vec4(1.0, 1.0 / 255.0, 1.0 / 65025.0, 1.0 / 16581375.0));
}

// ---- GLSL mod mirror (floored remainder) ------------------------------------
fn czm_mod_f32(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}

// ---- unpackFloat.glsl --------------------------------------------------------
fn czm_unpackFloat(packedFloat: vec4<f32>) -> f32 {
    // Convert to [0.0, 255.0] and round to integer
    var packed = floor(packedFloat * 255.0 + 0.5);
    let sign_v = 1.0 - select(0.0, 1.0, packed.w >= 128.0) * 2.0;
    let exponent = 2.0 * czm_mod_f32(packed.w, 128.0)
        + select(0.0, 1.0, packed.z >= 128.0) - 127.0;
    if (exponent == -127.0) {
        return 0.0;
    }
    // GLSL `float(0x800000)` == 8388608.0
    let mantissa = czm_mod_f32(packed.z, 128.0) * 65536.0 + packed.y * 256.0
        + packed.x + 8388608.0;
    return sign_v * exp2(exponent - 23.0) * mantissa;
}

// ---- unpackTexture.glsl ------------------------------------------------------
fn czm_unpackTexture(packedValue: f32) -> u32 {
    return u32(czm_round(packedValue * 255.0));
}
fn czm_unpackTexture_vec2(packedValue: vec2<f32>) -> u32 {
    let rounded = czm_round_vec2(packedValue * vec2(255.0));
    let byte0 = u32(rounded.x);
    let byte1 = u32(rounded.y);
    return byte0 | (byte1 << 8u);
}
fn czm_unpackTexture_vec3(packedValue: vec3<f32>) -> u32 {
    let rounded = czm_round_vec3(packedValue * vec3(255.0));
    let byte0 = u32(rounded.x);
    let byte1 = u32(rounded.y);
    let byte2 = u32(rounded.z);
    return byte0 | (byte1 << 8u) | (byte2 << 16u);
}
fn czm_unpackTexture_vec4(packedValue: vec4<f32>) -> u32 {
    let rounded = czm_round_vec4(packedValue * vec4(255.0));
    let byte0 = u32(rounded.x);
    let byte1 = u32(rounded.y);
    let byte2 = u32(rounded.z);
    let byte3 = u32(rounded.w);
    return byte0 | (byte1 << 8u) | (byte2 << 16u) | (byte3 << 24u);
}

// ---- unpackUint.glsl ---------------------------------------------------------
fn czm_unpackUint(packedValue: f32) -> i32 {
    return i32(czm_round(packedValue * 255.0));
}
fn czm_unpackUint_vec2(packedValue: vec2<f32>) -> i32 {
    let rounded = czm_round_vec2(packedValue * vec2(255.0));
    return i32(dot(rounded, vec2(1.0, 256.0)));
}
fn czm_unpackUint_vec3(packedValue: vec3<f32>) -> i32 {
    let rounded = czm_round_vec3(packedValue * vec3(255.0));
    return i32(dot(rounded, vec3(1.0, 256.0, 65536.0)));
}
fn czm_unpackUint_vec4(packedValue: vec4<f32>) -> i32 {
    let rounded = czm_round_vec4(packedValue * vec4(255.0));
    return i32(dot(rounded, vec4(1.0, 256.0, 65536.0, 16777216.0)));
}

// ---- decodeRGB8.glsl -----------------------------------------------------------
fn czm_decodeRGB8(encoded: f32) -> vec4<f32> {
    let SHIFT_RIGHT16 = 1.0 / 65536.0;
    let SHIFT_RIGHT8 = 1.0 / 256.0;
    let SHIFT_LEFT16 = 65536.0;
    let SHIFT_LEFT8 = 256.0;

    var color = vec4(255.0);
    color.r = floor(encoded * SHIFT_RIGHT16);
    color.g = floor((encoded - color.r * SHIFT_LEFT16) * SHIFT_RIGHT8);
    color.b = floor(encoded - color.r * SHIFT_LEFT16 - color.g * SHIFT_LEFT8);
    return color / vec4(255.0);
}

// ---- decompressTextureCoordinates.glsl ------------------------------------------
fn czm_decompressTextureCoordinates(encoded: f32) -> vec2<f32> {
    let temp = encoded / 4096.0;
    let xZeroTo4095 = floor(temp);
    let stx = xZeroTo4095 / 4095.0;
    let sty = (encoded - xZeroTo4095 * 4096.0) / 4095.0;
    return vec2(stx, sty);
}

// ---- octDecode.glsl ---------------------------------------------------------------
fn czm_octDecode_range(encoded_in: vec2<f32>, range: f32) -> vec3<f32> {
    if (encoded_in.x == 0.0 && encoded_in.y == 0.0) {
        return vec3(0.0);
    }
    var encoded = encoded_in / vec2(range) * 2.0 - vec2(1.0);
    var v = vec3(encoded.x, encoded.y, 1.0 - abs(encoded.x) - abs(encoded.y));
    if (v.z < 0.0) {
        // DEVIATION: WGSL forbids swizzle assignment; mirrored component-wise.
        let folded = (vec2(1.0) - abs(v.yx)) * czm_signNotZero_vec2(v.xy);
        v.x = folded.x;
        v.y = folded.y;
    }
    return normalize(v);
}
fn czm_octDecode(encoded: vec2<f32>) -> vec3<f32> {
    return czm_octDecode_range(encoded, 255.0);
}
fn czm_octDecode_f32(encoded: f32) -> vec3<f32> {
    let temp = encoded / 256.0;
    let x = floor(temp);
    let y = (temp - x) * 256.0;
    return czm_octDecode(vec2(x, y));
}
struct czm_octDecodeTriple {
    vector1: vec3<f32>,
    vector2: vec3<f32>,
    vector3: vec3<f32>,
}
fn czm_octDecode_triple(encoded: vec2<f32>) -> czm_octDecodeTriple {
    var result: czm_octDecodeTriple;
    var temp = encoded.x / 65536.0;
    let x = floor(temp);
    let encodedFloat1 = (temp - x) * 65536.0;

    temp = encoded.y / 65536.0;
    let y = floor(temp);
    let encodedFloat2 = (temp - y) * 65536.0;

    result.vector1 = czm_octDecode_f32(encodedFloat1);
    result.vector2 = czm_octDecode_f32(encodedFloat2);
    result.vector3 = czm_octDecode(vec2(x, y));
    return result;
}
