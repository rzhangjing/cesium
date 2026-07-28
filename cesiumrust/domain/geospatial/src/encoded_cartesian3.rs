//! EncodedCartesian3 - fixed-point encoding of Cartesian3 for GPU rendering.
//! Maps to CesiumJS `Core/EncodedCartesian3.js`

use glam::DVec3;

/// A fixed-point encoding of a Cartesian3 as two Cartesian3 values (high and low)
/// that, when converted to 32-bit floating-point and added, approximate the original input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EncodedCartesian3 {
    /// The high bits for each component.
    pub high: DVec3,
    /// The low bits for each component.
    pub low: DVec3,
}

impl Default for EncodedCartesian3 {
    fn default() -> Self {
        Self {
            high: DVec3::ZERO,
            low: DVec3::ZERO,
        }
    }
}

/// Encodes a 64-bit floating-point value as two f64 values (high, low) that,
/// when converted to 32-bit floating-point and added, approximate the original input.
///
/// Maps to `EncodedCartesian3.encode`
pub fn encode(value: f64) -> (f64, f64) {
    if value >= 0.0 {
        let double_high = (value / 65536.0).floor() * 65536.0;
        (double_high, value - double_high)
    } else {
        let double_high = (-value / 65536.0).floor() * 65536.0;
        (-double_high, value + double_high)
    }
}

/// Encodes a Cartesian3 as an EncodedCartesian3.
///
/// Maps to `EncodedCartesian3.fromCartesian`
pub fn from_cartesian(cartesian: DVec3) -> EncodedCartesian3 {
    let (hx, lx) = encode(cartesian.x);
    let (hy, ly) = encode(cartesian.y);
    let (hz, lz) = encode(cartesian.z);
    EncodedCartesian3 {
        high: DVec3::new(hx, hy, hz),
        low: DVec3::new(lx, ly, lz),
    }
}

/// Encodes a Cartesian3 and writes it to an array as
/// [high.x, high.y, high.z, low.x, low.y, low.z] starting at `index`.
///
/// Maps to `EncodedCartesian3.writeElements`
pub fn write_elements(cartesian: DVec3, array: &mut [f64], index: usize) {
    let encoded = from_cartesian(cartesian);
    array[index] = encoded.high.x;
    array[index + 1] = encoded.high.y;
    array[index + 2] = encoded.high.z;
    array[index + 3] = encoded.low.x;
    array[index + 4] = encoded.low.y;
    array[index + 5] = encoded.low.z;
}
