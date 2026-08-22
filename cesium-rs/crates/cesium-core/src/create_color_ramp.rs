//! Ported from `packages/engine/Source/Core/createColorRamp.js`.
//!
//! Creates a color ramp that linearly interpolates between the given colors.
//! Skeleton implementation – the JS version uses a canvas element; in Rust
//! we perform pure linear interpolation.

use crate::color::Color;

/// Creates a color ramp by linearly interpolating between the given colors.
/// Returns a Vec of RGBA bytes (4 bytes per pixel, `ramp_length` pixels).
pub fn create_color_ramp(colors: &[Color], ramp_length: usize) -> Vec<u8> {
    if colors.is_empty() || ramp_length == 0 {
        return Vec::new();
    }
    if colors.len() == 1 {
        let c = &colors[0];
        let mut ramp = Vec::with_capacity(ramp_length * 4);
        for _ in 0..ramp_length {
            ramp.push(f64_to_u8(c.red));
            ramp.push(f64_to_u8(c.green));
            ramp.push(f64_to_u8(c.blue));
            ramp.push(f64_to_u8(c.alpha));
        }
        return ramp;
    }

    let step = 1.0 / (colors.len() - 1) as f64;
    let mut ramp = Vec::with_capacity(ramp_length * 4);

    for i in 0..ramp_length {
        let t = i as f64 / (ramp_length - 1).max(1) as f64;
        let segment = ((t / step) as usize).min(colors.len() - 2);
        let local_t = (t - segment as f64 * step) / step;

        let c0 = &colors[segment];
        let c1 = &colors[segment + 1];

        ramp.push(f64_to_u8(lerp_f64(c0.red, c1.red, local_t)));
        ramp.push(f64_to_u8(lerp_f64(c0.green, c1.green, local_t)));
        ramp.push(f64_to_u8(lerp_f64(c0.blue, c1.blue, local_t)));
        ramp.push(f64_to_u8(lerp_f64(c0.alpha, c1.alpha, local_t)));
    }

    ramp
}

fn lerp_f64(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn f64_to_u8(v: f64) -> u8 {
    (v * 255.0).round().clamp(0.0, 255.0) as u8
}
