//! Ported from `packages/engine/Source/Core/srgbToLinear.js`.

/// Converts the value from sRGB color space to linear color space.
pub fn srgb_to_linear(value: f64) -> f64 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// Converts the value from linear color space to sRGB color space.
pub fn linear_to_srgb(value: f64) -> f64 {
    if value < 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}
