// 遗留 CesiumJS 移植风格债（deferred.md #18）；将在 M13 lint 清理，或本文件在其所属里程碑被重写时重新审视
#![allow(clippy::manual_div_ceil)]
/// 用于 ImplicitSubtree 的可用性位流。
/// 同时处理 Uint8Array 位流和常量值。
///
/// 对 CesiumJS `ImplicitAvailabilityBitstream` 忠实移植。
#[derive(Clone, Debug)]
pub struct ImplicitAvailabilityBitstream {
    length_bits: usize,
    available_count: Option<usize>,
    constant: Option<bool>,
    bitstream: Option<Vec<u8>>,
}

pub struct ImplicitAvailabilityBitstreamOptions {
    pub length_bits: usize,
    pub constant: Option<bool>,
    pub bitstream: Option<Vec<u8>>,
    pub available_count: Option<usize>,
    pub compute_available_count_enabled: bool,
}

impl ImplicitAvailabilityBitstream {
    pub fn new(options: ImplicitAvailabilityBitstreamOptions) -> Self {
        let length_bits = options.length_bits;
        let mut available_count = options.available_count;
        let constant = options.constant;
        let bitstream = options.bitstream;

        if constant.is_some() {
            // 若已定义，constant 必为 true，意味着所有瓦片均可用
            available_count = Some(length_bits);
        } else if let Some(ref bs) = bitstream {
            let expected_length = (length_bits + 7) / 8;
            assert_eq!(
                bs.len(),
                expected_length,
                "Availability bitstream must be exactly {} bytes long to store {} bits. Actual bitstream was {} bytes long.",
                expected_length,
                length_bits,
                bs.len()
            );

            if available_count.is_none() && options.compute_available_count_enabled {
                available_count = Some(count_1_bits(bs, length_bits));
            }
        }

        Self {
            length_bits,
            available_count,
            constant,
            bitstream,
        }
    }

    /// 位流以位（bit）计的长度。
    pub fn length_bits(&self) -> usize {
        self.length_bits
    }

    /// 位流中值为 1 的位的数量。
    pub fn available_count(&self) -> Option<usize> {
        self.available_count
    }

    /// 以布尔值形式从可用性位流中获取一个位。
    /// 若位流为常量，则返回该常量值。
    pub fn get_bit(&self, index: usize) -> bool {
        assert!(
            index < self.length_bits,
            "Bit index out of bounds."
        );

        if let Some(c) = self.constant {
            return c;
        }

        let bs = self.bitstream.as_ref().unwrap();
        let byte_index = index >> 3;
        let bit_index = index % 8;
        ((bs[byte_index] >> bit_index) & 1) == 1
    }
}

/// 统计位流中值为 1 的位的数量。
fn count_1_bits(bitstream: &[u8], length_bits: usize) -> usize {
    let mut count = 0;
    for i in 0..length_bits {
        let byte_index = i >> 3;
        let bit_index = i % 8;
        count += ((bitstream[byte_index] >> bit_index) & 1) as usize;
    }
    count
}
