//! Ported from `packages/engine/Source/Core/HilbertOrder.js`.
//!
//! Hilbert Order helper functions.

/// Computes the Hilbert index at the given level from 2D coordinates.
pub fn encode_2d(level: u32, x: u32, y: u32) -> u64 {
    let n = 1u32 << level;
    assert!(level >= 1, "Hilbert level cannot be less than 1.");
    assert!(
        x < n && y < n,
        "Invalid coordinates for given level."
    );

    let mut px = x;
    let mut py = y;
    let mut index: u64 = 0;

    let mut s = n / 2;
    while s > 0 {
        let rx = if (px & s) > 0 { 1u32 } else { 0 };
        let ry = if (py & s) > 0 { 1u32 } else { 0 };
        index += ((3 * rx) ^ ry) as u64 * (s as u64) * (s as u64);
        rotate(n, &mut px, &mut py, rx, ry);
        s /= 2;
    }

    index
}

/// Computes the 2D coordinates from the Hilbert index at the given level.
pub fn decode_2d(level: u32, index: u64) -> (u32, u32) {
    assert!(level >= 1, "Hilbert level cannot be less than 1.");
    let max_index = 1u64 << (2 * level);
    assert!(
        index < max_index,
        "Hilbert index exceeds valid maximum for given level."
    );

    let n = 1u32 << level;
    let mut px: u32 = 0;
    let mut py: u32 = 0;
    let mut t = index;

    let mut s = 1u32;
    while s < n {
        let rx = (1 & (t / 2)) as u32;
        let ry = (1 & (t ^ rx as u64)) as u32;
        rotate(s, &mut px, &mut py, rx, ry);
        px += s * rx;
        py += s * ry;
        t /= 4;
        s *= 2;
    }

    (px, py)
}

fn rotate(n: u32, px: &mut u32, py: &mut u32, rx: u32, ry: u32) {
    if ry != 0 {
        return;
    }

    if rx == 1 {
        *px = n - 1 - *px;
        *py = n - 1 - *py;
    }

    let t = *px;
    *px = *py;
    *py = t;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        for level in 1..6u32 {
            let n = 1u32 << level;
            for x in 0..n {
                for y in 0..n {
                    let index = encode_2d(level, x, y);
                    let (dx, dy) = decode_2d(level, index);
                    assert_eq!((dx, dy), (x, y), "Roundtrip failed for ({}, {}) at level {}", x, y, level);
                }
            }
        }
    }

    #[test]
    fn test_known_values() {
        // Level 1: 2x2 grid - verify roundtrip consistency
        for x in 0..2u32 {
            for y in 0..2u32 {
                let idx = encode_2d(1, x, y);
                let (dx, dy) = decode_2d(1, idx);
                assert_eq!((dx, dy), (x, y));
            }
        }
        // All 4 indices should be distinct
        let mut indices: Vec<u64> = (0..2u32)
            .flat_map(|x| (0..2u32).map(move |y| encode_2d(1, x, y)))
            .collect();
        indices.sort();
        assert_eq!(indices, vec![0, 1, 2, 3]);
    }
}
