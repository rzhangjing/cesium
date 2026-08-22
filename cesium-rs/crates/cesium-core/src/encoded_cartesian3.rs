//! Ported from `packages/engine/Source/Core/EncodedCartesian3.js`.

use crate::cartesian3::Cartesian3;

/// A fixed-point encoding of a Cartesian3 with 64-bit floating-point components,
/// as two Cartesian3 values that, when converted to 32-bit and added, approximate
/// the original input.
#[derive(Debug, Clone, PartialEq)]
pub struct EncodedCartesian3 {
    /// The high bits for each component.
    pub high: Cartesian3,
    /// The low bits for each component.
    pub low: Cartesian3,
}

impl Default for EncodedCartesian3 {
    fn default() -> Self {
        Self {
            high: Cartesian3::ZERO,
            low: Cartesian3::ZERO,
        }
    }
}

/// Encoded high/low pair for a single f64 value.
#[derive(Debug, Clone, Copy, Default)]
pub struct EncodedValue {
    pub high: f64,
    pub low: f64,
}

impl EncodedCartesian3 {
    /// Encodes a 64-bit floating-point value as two floating-point values.
    pub fn encode(value: f64) -> EncodedValue {
        if value >= 0.0 {
            let double_high = (value / 65536.0).floor() * 65536.0;
            EncodedValue {
                high: double_high,
                low: value - double_high,
            }
        } else {
            let double_high = (-value / 65536.0).floor() * 65536.0;
            EncodedValue {
                high: -double_high,
                low: value + double_high,
            }
        }
    }

    /// Encodes a Cartesian3 as two Cartesian3 values (high + low).
    pub fn from_cartesian(cartesian: &Cartesian3) -> Self {
        let ex = Self::encode(cartesian.x);
        let ey = Self::encode(cartesian.y);
        let ez = Self::encode(cartesian.z);
        Self {
            high: Cartesian3::new(ex.high, ey.high, ez.high),
            low: Cartesian3::new(ex.low, ey.low, ez.low),
        }
    }

    /// Encodes and writes to an array: [high.x, high.y, high.z, low.x, low.y, low.z].
    pub fn write_elements(cartesian: &Cartesian3, array: &mut [f64], index: usize) {
        let encoded = Self::from_cartesian(cartesian);
        array[index] = encoded.high.x;
        array[index + 1] = encoded.high.y;
        array[index + 2] = encoded.high.z;
        array[index + 3] = encoded.low.x;
        array[index + 4] = encoded.low.y;
        array[index + 5] = encoded.low.z;
    }
}
