/// An availability bitstream for use in an ImplicitSubtree.
/// Handles both Uint8Array bitstreams and constant values.
///
/// Faithful port of CesiumJS `ImplicitAvailabilityBitstream`.
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
            // if defined, constant must be true which means all tiles are available
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

    /// The length of the bitstream in bits.
    pub fn length_bits(&self) -> usize {
        self.length_bits
    }

    /// The number of bits in the bitstream with value 1.
    pub fn available_count(&self) -> Option<usize> {
        self.available_count
    }

    /// Get a bit from the availability bitstream as a Boolean.
    /// If the bitstream is a constant, the constant value is returned instead.
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

/// Count the number of bits with value 1 in the bitstream.
fn count_1_bits(bitstream: &[u8], length_bits: usize) -> usize {
    let mut count = 0;
    for i in 0..length_bits {
        let byte_index = i >> 3;
        let bit_index = i % 8;
        count += ((bitstream[byte_index] >> bit_index) & 1) as usize;
    }
    count
}
