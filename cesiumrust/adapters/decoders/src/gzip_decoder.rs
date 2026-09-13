//! Gzip decompression using flate2.

use cesium_ports_driven::{PortError, PortResult};
use flate2::read::GzDecoder;
use std::io::Read;

pub fn decode_gzip(data: &[u8]) -> PortResult<Vec<u8>> {
    let mut decoder = GzDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| PortError::Decode(format!("failed to decompress gzip: {e}")))?;
    Ok(decompressed)
}
