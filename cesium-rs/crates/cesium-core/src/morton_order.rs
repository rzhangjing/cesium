//! Ported from `packages/engine/Source/Core/MortonOrder.js`.
//!
//! Morton Order (aka Z-Order Curve) helper functions.
//! See <https://en.wikipedia.org/wiki/Z-order_curve>

/// Inserts one 0 bit of spacing between a number's bits.
fn insert_one_spacing(v: u32) -> u32 {
    let mut v = (v ^ (v << 8)) & 0x00FF_00FF;
    v = (v ^ (v << 4)) & 0x0F0F_0F0F;
    v = (v ^ (v << 2)) & 0x3333_3333;
    v = (v ^ (v << 1)) & 0x5555_5555;
    v
}

/// Inserts two 0 bits of spacing between a number's bits.
fn insert_two_spacing(v: u32) -> u32 {
    let mut v = (v ^ (v << 16)) & 0x0300_00FF;
    v = (v ^ (v << 8)) & 0x0300_F00F;
    v = (v ^ (v << 4)) & 0x030C_30C3;
    v = (v ^ (v << 2)) & 0x0924_9249;
    v
}

/// Removes one bit of spacing between bits.
fn remove_one_spacing(v: u32) -> u32 {
    let mut v = v & 0x5555_5555;
    v = (v ^ (v >> 1)) & 0x3333_3333;
    v = (v ^ (v >> 2)) & 0x0F0F_0F0F;
    v = (v ^ (v >> 4)) & 0x00FF_00FF;
    v = (v ^ (v >> 8)) & 0x0000_FFFF;
    v
}

/// Removes two bits of spacing between bits.
fn remove_two_spacing(v: u32) -> u32 {
    let mut v = v & 0x0924_9249;
    v = (v ^ (v >> 2)) & 0x030C_30C3;
    v = (v ^ (v >> 4)) & 0x0300_F00F;
    v = (v ^ (v >> 8)) & 0xFF00_00FF;
    v = (v ^ (v >> 16)) & 0x0000_03FF;
    v
}

/// Morton order encoding/decoding utilities.
pub struct MortonOrder;

impl MortonOrder {
    /// Computes the Morton index from 2D coordinates (bit interleaving).
    /// Inputs must be 16-bit unsigned integers (result is 32-bit).
    pub fn encode_2d(x: u16, y: u16) -> u32 {
        insert_one_spacing(x as u32) | (insert_one_spacing(y as u32) << 1)
    }

    /// Computes 2D coordinates from a Morton index (bit deinterleaving).
    /// Returns `(x, y)` where each is a 16-bit value.
    pub fn decode_2d(morton_index: u32) -> (u16, u16) {
        let x = remove_one_spacing(morton_index) as u16;
        let y = remove_one_spacing(morton_index >> 1) as u16;
        (x, y)
    }

    /// Computes the Morton index from 3D coordinates (bit interleaving).
    /// Inputs must be 10-bit unsigned integers (result is 30-bit).
    pub fn encode_3d(x: u16, y: u16, z: u16) -> u32 {
        insert_two_spacing(x as u32)
            | (insert_two_spacing(y as u32) << 1)
            | (insert_two_spacing(z as u32) << 2)
    }

    /// Computes 3D coordinates from a Morton index (bit deinterleaving).
    /// Returns `(x, y, z)` where each is a 10-bit value.
    pub fn decode_3d(morton_index: u32) -> (u16, u16, u16) {
        let x = remove_two_spacing(morton_index) as u16;
        let y = remove_two_spacing(morton_index >> 1) as u16;
        let z = remove_two_spacing(morton_index >> 2) as u16;
        (x, y, z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_2d_roundtrip() {
        for x in [0u16, 1, 6, 255, 1000, 65535] {
            for y in [0u16, 1, 6, 255, 1000, 65535] {
                let morton = MortonOrder::encode_2d(x, y);
                let (dx, dy) = MortonOrder::decode_2d(morton);
                assert_eq!((dx, dy), (x, y), "roundtrip failed for ({x}, {y})");
            }
        }
    }

    #[test]
    fn encode_decode_3d_roundtrip() {
        for x in [0u16, 1, 6, 511, 1023] {
            for y in [0u16, 1, 6, 511, 1023] {
                for z in [0u16, 1, 6, 511, 1023] {
                    let morton = MortonOrder::encode_3d(x, y, z);
                    let (dx, dy, dz) = MortonOrder::decode_3d(morton);
                    assert_eq!(
                        (dx, dy, dz),
                        (x, y, z),
                        "roundtrip failed for ({x}, {y}, {z})"
                    );
                }
            }
        }
    }

    #[test]
    fn known_2d_values() {
        // x=6 (110), y=0 → insert spacing in 6 = 10100 = 20, shift 0 = 0 → 20
        assert_eq!(MortonOrder::encode_2d(6, 0), 20);
        // x=0, y=6 → insert spacing in 6 = 20, shift left 1 = 40
        assert_eq!(MortonOrder::encode_2d(0, 6), 40);
    }

    #[test]
    fn known_3d_values() {
        // x=6, y=0, z=0 → insertTwoSpacing(6) = 72
        assert_eq!(MortonOrder::encode_3d(6, 0, 0), 72);
    }
}
