//! Google Earth Enterprise metadata utilities.
//! Domain layer - pure Rust, no framework dependency.
//!
//! CesiumJS mapping: `packages/engine/Source/Core/GoogleEarthEnterpriseMetadata.js`
//! and `packages/engine/Source/Core/decodeGoogleEarthEnterpriseData.js`

/// Result of quadkey-to-tile conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuadKeyTile {
    pub x: u32,
    pub y: u32,
    pub level: u32,
}

/// Converts tile coordinates to a Google Earth Enterprise quadkey string.
///
/// Maps to CesiumJS `GoogleEarthEnterpriseMetadata.tileXYToQuadKey`.
///
/// Tile layout per level:
/// ```text
///  ___ ___
/// |   |   |
/// | 3 | 2 |
/// |-------|
/// | 0 | 1 |
/// |___|___|
/// ```
pub fn tile_xy_to_quad_key(x: u32, y: u32, level: u32) -> String {
    let mut quadkey = String::with_capacity((level + 1) as usize);
    for i in (0..=level).rev() {
        let bitmask = 1u32 << i;
        let mut digit: u32 = 0;

        if y & bitmask == 0 {
            // Top Row
            digit |= 2;
            if x & bitmask == 0 {
                // Right to left
                digit |= 1;
            }
        } else if x & bitmask != 0 {
            // Left to right
            digit |= 1;
        }

        quadkey.push(char::from_digit(digit, 10).unwrap());
    }
    quadkey
}

/// Converts a Google Earth Enterprise quadkey string to tile coordinates.
///
/// Maps to CesiumJS `GoogleEarthEnterpriseMetadata.quadKeyToTileXY`.
pub fn quad_key_to_tile_xy(quadkey: &str) -> QuadKeyTile {
    let mut x: u32 = 0;
    let mut y: u32 = 0;
    let level = quadkey.len() as u32 - 1;

    for i in (0..=level).rev() {
        let bitmask = 1u32 << i;
        let digit = quadkey.as_bytes()[(level - i) as usize] - b'0';

        if digit & 2 != 0 {
            // Top Row
            if digit & 1 == 0 {
                // Right to left
                x |= bitmask;
            }
        } else {
            y |= bitmask;
            if digit & 1 != 0 {
                // Left to right
                x |= bitmask;
            }
        }
    }

    QuadKeyTile { x, y, level }
}

const COMPRESSED_MAGIC: u32 = 0x7468dead;
const COMPRESSED_MAGIC_SWAP: u32 = 0xadde6874;

/// Decodes data received from a Google Earth Enterprise server.
///
/// Maps to CesiumJS `decodeGoogleEarthEnterpriseData`.
/// The algorithm is XOR-based: applying it twice returns the original data.
///
/// # Panics
/// Panics if `key` is empty or its length is not a multiple of 4.
pub fn decode_google_earth_enterprise_data(key: &[u8], data: &mut [u8]) {
    let key_length = key.len();
    assert!(
        key_length > 0 && key_length % 4 == 0,
        "The length of key must be greater than 0 and a multiple of 4."
    );

    // Check for compressed magic (already decoded / not encoded)
    if data.len() >= 4 {
        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic == COMPRESSED_MAGIC || magic == COMPRESSED_MAGIC_SWAP {
            return;
        }
    }

    // The algorithm requires key to be at least 24 bytes for the inner loop
    // to make progress (kp starts at max 16, accesses kp+7, increments by 24).
    // For shorter keys, fall back to simple repeating XOR.
    if key_length < 24 {
        for (i, byte) in data.iter_mut().enumerate() {
            *byte ^= key[i % key_length];
        }
        return;
    }

    let dpend = data.len();
    let dpend64 = dpend - (dpend % 8);
    let kpend = key_length;
    let mut dp = 0usize;
    let mut off = 8usize;
    let mut kp: usize = 0;

    // Process 8 bytes at a time
    while dp < dpend64 {
        off = (off + 8) % 24;
        kp = off;

        while dp < dpend64 && kp + 8 <= kpend {
            // XOR 4 bytes at dp
            for j in 0..4 {
                data[dp + j] ^= key[kp + j];
            }
            // XOR 4 bytes at dp+4
            for j in 0..4 {
                data[dp + 4 + j] ^= key[kp + 4 + j];
            }
            dp += 8;
            kp += 24;
        }
    }

    // Remaining 1-7 bytes
    if dp < dpend {
        if kp >= kpend {
            off = (off + 8) % 24;
            kp = off;
        }

        while dp < dpend {
            data[dp] ^= key[kp % kpend];
            dp += 1;
            kp += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_xy_to_quad_key() {
        assert_eq!(tile_xy_to_quad_key(1, 0, 0), "2");
        assert_eq!(tile_xy_to_quad_key(1, 2, 1), "02");
        assert_eq!(tile_xy_to_quad_key(3, 5, 2), "021");
        assert_eq!(tile_xy_to_quad_key(4, 7, 2), "100");
    }

    #[test]
    fn test_quad_key_to_tile_xy() {
        assert_eq!(quad_key_to_tile_xy("2"), QuadKeyTile { x: 1, y: 0, level: 0 });
        assert_eq!(quad_key_to_tile_xy("02"), QuadKeyTile { x: 1, y: 2, level: 1 });
        assert_eq!(quad_key_to_tile_xy("021"), QuadKeyTile { x: 3, y: 5, level: 2 });
        assert_eq!(quad_key_to_tile_xy("100"), QuadKeyTile { x: 4, y: 7, level: 2 });
    }

    #[test]
    fn test_roundtrip() {
        for level in 0..5 {
            let max = 1u32 << (level + 1);
            for x in 0..max.min(8) {
                for y in 0..max.min(8) {
                    let qk = tile_xy_to_quad_key(x, y, level);
                    let tile = quad_key_to_tile_xy(&qk);
                    assert_eq!(tile.x, x, "x mismatch for level={} x={} y={}", level, x, y);
                    assert_eq!(tile.y, y, "y mismatch for level={} x={} y={}", level, x, y);
                    assert_eq!(tile.level, level);
                }
            }
        }
    }

    #[test]
    fn test_decode_symmetric() {
        // XOR decode is symmetric: applying twice returns original
        let key: Vec<u8> = (0..16).collect(); // 16 bytes, multiple of 4
        let original: Vec<u8> = (100..132).collect(); // 32 bytes
        let mut data = original.clone();

        decode_google_earth_enterprise_data(&key, &mut data);
        assert_ne!(data, original); // Should be different after first decode

        decode_google_earth_enterprise_data(&key, &mut data);
        assert_eq!(data, original); // Should be back to original after second decode
    }

    #[test]
    fn test_decode_skips_compressed_magic() {
        let key: Vec<u8> = vec![1, 2, 3, 4];
        // Data starting with compressed magic (little-endian 0x7468dead)
        let mut data: Vec<u8> = vec![0xad, 0xde, 0x68, 0x74, 5, 6, 7, 8];
        let original = data.clone();

        decode_google_earth_enterprise_data(&key, &mut data);
        assert_eq!(data, original); // Should be unchanged

        // Also test compressedMagicSwap (0xadde6874)
        let mut data2: Vec<u8> = vec![0x74, 0x68, 0xde, 0xad, 5, 6, 7, 8];
        let original2 = data2.clone();
        decode_google_earth_enterprise_data(&key, &mut data2);
        assert_eq!(data2, original2);
    }

    #[test]
    #[should_panic(expected = "multiple of 4")]
    fn test_decode_invalid_key_length() {
        let key: Vec<u8> = vec![1, 2, 3]; // Not multiple of 4
        let mut data: Vec<u8> = vec![0; 8];
        decode_google_earth_enterprise_data(&key, &mut data);
    }
}
