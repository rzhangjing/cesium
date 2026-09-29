//! Google Earth Enterprise 元数据工具。
//! 领域层 —— 纯 Rust，无框架依赖。
//!
//! CesiumJS 映射：`packages/engine/Source/Core/GoogleEarthEnterpriseMetadata.js`
//! 及 `packages/engine/Source/Core/decodeGoogleEarthEnterpriseData.js`

// 遗留 CesiumJS 移植风格债（deferred.md #18）；将在 M13 lint 清理，或本文件在其所属里程碑被重写时重新审视
#![allow(clippy::manual_is_multiple_of)]
/// quadkey 到瓦片转换的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuadKeyTile {
    pub x: u32,
    pub y: u32,
    pub level: u32,
}

/// 将瓦片坐标转换为 Google Earth Enterprise 的 quadkey 字符串。
///
/// 映射到 CesiumJS `GoogleEarthEnterpriseMetadata.tileXYToQuadKey`。
///
/// 每层级的瓦片布局：
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
            // 顶行
            digit |= 2;
            if x & bitmask == 0 {
                // 从右到左
                digit |= 1;
            }
        } else if x & bitmask != 0 {
            // 从左到右
            digit |= 1;
        }

        quadkey.push(char::from_digit(digit, 10).unwrap());
    }
    quadkey
}

/// 将 Google Earth Enterprise 的 quadkey 字符串转换为瓦片坐标。
///
/// 映射到 CesiumJS `GoogleEarthEnterpriseMetadata.quadKeyToTileXY`。
pub fn quad_key_to_tile_xy(quadkey: &str) -> QuadKeyTile {
    let mut x: u32 = 0;
    let mut y: u32 = 0;
    let level = quadkey.len() as u32 - 1;

    for i in (0..=level).rev() {
        let bitmask = 1u32 << i;
        let digit = quadkey.as_bytes()[(level - i) as usize] - b'0';

        if digit & 2 != 0 {
            // 顶行
            if digit & 1 == 0 {
                // 从右到左
                x |= bitmask;
            }
        } else {
            y |= bitmask;
            if digit & 1 != 0 {
                // 从左到右
                x |= bitmask;
            }
        }
    }

    QuadKeyTile { x, y, level }
}

const COMPRESSED_MAGIC: u32 = 0x7468dead;
const COMPRESSED_MAGIC_SWAP: u32 = 0xadde6874;

/// 解码从 Google Earth Enterprise 服务器接收的数据。
///
/// 映射到 CesiumJS `decodeGoogleEarthEnterpriseData`。
/// 该算法基于 XOR：应用两次会返回原始数据。
///
/// #  Panic
/// 若 `key` 为空或其长度不是 4 的倍数则 Panic。
pub fn decode_google_earth_enterprise_data(key: &[u8], data: &mut [u8]) {
    let key_length = key.len();
    assert!(
        key_length > 0 && key_length % 4 == 0,
        "The length of key must be greater than 0 and a multiple of 4."
    );

    // 检查压缩魔数（已解码 / 未编码）
    if data.len() >= 4 {
        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic == COMPRESSED_MAGIC || magic == COMPRESSED_MAGIC_SWAP {
            return;
        }
    }

    // 该算法要求 key 至少为 24 字节，内层循环
    // 才能推进（kp 从最大 16 开始，访问 kp+7，以 24 递增）。
    // 对于更短的 key，回退到简单的重复 XOR。
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

    // 每次处理 8 个字节
    while dp < dpend64 {
        off = (off + 8) % 24;
        kp = off;

        while dp < dpend64 && kp + 8 <= kpend {
            // 对 dp 处的 4 个字节做 XOR
            for j in 0..4 {
                data[dp + j] ^= key[kp + j];
            }
            // 对 dp+4 处的 4 个字节做 XOR
            for j in 0..4 {
                data[dp + 4 + j] ^= key[kp + 4 + j];
            }
            dp += 8;
            kp += 24;
        }
    }

    // 剩余的 1-7 个字节
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
        // XOR 解码是对称的：应用两次返回原始数据
        let key: Vec<u8> = (0..16).collect(); // 16 字节，4 的倍数
        let original: Vec<u8> = (100..132).collect(); // 32 字节
        let mut data = original.clone();

        decode_google_earth_enterprise_data(&key, &mut data);
        assert_ne!(data, original); // 首次解码后应不同

        decode_google_earth_enterprise_data(&key, &mut data);
        assert_eq!(data, original); // 第二次解码后应恢复为原始数据
    }

    #[test]
    fn test_decode_skips_compressed_magic() {
        let key: Vec<u8> = vec![1, 2, 3, 4];
        // 以压缩魔数开头的数据（小端 0x7468dead）
        let mut data: Vec<u8> = vec![0xad, 0xde, 0x68, 0x74, 5, 6, 7, 8];
        let original = data.clone();

        decode_google_earth_enterprise_data(&key, &mut data);
        assert_eq!(data, original); // 应保持不变

        // 也测试 compressedMagicSwap (0xadde6874)
        let mut data2: Vec<u8> = vec![0x74, 0x68, 0xde, 0xad, 5, 6, 7, 8];
        let original2 = data2.clone();
        decode_google_earth_enterprise_data(&key, &mut data2);
        assert_eq!(data2, original2);
    }

    #[test]
    #[should_panic(expected = "multiple of 4")]
    fn test_decode_invalid_key_length() {
        let key: Vec<u8> = vec![1, 2, 3]; // 不是 4 的倍数
        let mut data: Vec<u8> = vec![0; 8];
        decode_google_earth_enterprise_data(&key, &mut data);
    }
}
