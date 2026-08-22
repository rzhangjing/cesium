//! Ported from `packages/engine/Source/Core/decodeGoogleEarthEnterpriseData.js`.
//!
//! Decodes data received from the Google Earth Enterprise server.

const COMPRESSED_MAGIC: u32 = 0x7468dead;
const COMPRESSED_MAGIC_SWAP: u32 = 0xadde6874;

/// Decodes Google Earth Enterprise data using XOR with the given key.
///
/// The `key` and `data` slices are modified in-place (data is XOR-decoded).
pub fn decode_google_earth_enterprise_data(key: &[u8], data: &mut [u8]) {
    let key_length = key.len();
    if key_length == 0 || key_length % 4 != 0 {
        return;
    }

    // Check magic
    if data.len() >= 4 {
        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic == COMPRESSED_MAGIC || magic == COMPRESSED_MAGIC_SWAP {
            return;
        }
    }

    let data_length = data.len();
    let dpend64 = data_length - (data_length % 8);

    let mut dp = 0usize;
    let mut off = 8usize;

    while dp < dpend64 {
        off = (off + 8) % 24;
        let mut kp = off;

        while dp < dpend64 && kp < key_length {
            // XOR 8 bytes at a time
            for b in 0..8 {
                if dp + b < data_length && kp + b < key_length {
                    data[dp + b] ^= key[kp + b];
                }
            }
            dp += 8;
            kp += 24;
        }
    }

    // Remaining 1-7 bytes
    if dp < data_length {
        if kp_from_off(off, key_length) >= key_length {
            off = (off + 8) % 24;
        }
        let mut kp = off;
        while dp < data_length {
            if kp >= key_length {
                kp = 0;
            }
            data[dp] ^= key[kp];
            dp += 1;
            kp += 1;
        }
    }
}

fn kp_from_off(off: usize, _key_length: usize) -> usize {
    off
}
