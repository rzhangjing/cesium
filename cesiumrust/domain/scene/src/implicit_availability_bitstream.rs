// 位流实现沿用按字节打包的历史约定，待后续里程碑统一整理
#![allow(clippy::manual_div_ceil)]
/// 用于 ImplicitSubtree 的可用性位流。
/// 同时处理按字节打包的位流和常量值两种来源。
#[derive(Clone, Debug)]
pub struct ImplicitAvailabilityBitstream {
    /// 位流以位（bit）计的总长度。
    length_bits: usize,
    /// 值为 1 的位的数量；None 表示尚未统计。
    available_count: Option<usize>,
    /// 常量位值；Some 时整个位流退化为该单一布尔值。
    constant: Option<bool>,
    /// 按字节打包的位流缓冲；与 constant 互斥。
    bitstream: Option<Vec<u8>>,
}

/// 构造可用性位流所需的选项。
pub struct ImplicitAvailabilityBitstreamOptions {
    /// 位流以位计的总长度。
    pub length_bits: usize,
    /// 常量位值；Some 时忽略 bitstream。
    pub constant: Option<bool>,
    /// 按字节打包的位流缓冲。
    pub bitstream: Option<Vec<u8>>,
    /// 预先给定的可用位数量；None 时按需统计。
    pub available_count: Option<usize>,
    /// 是否在缺少 available_count 时自动统计 1 位的数量。
    pub compute_available_count_enabled: bool,
}

impl ImplicitAvailabilityBitstream {
    /// 依选项创建可用性位流，必要时校验字节长度并统计可用位数。
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
