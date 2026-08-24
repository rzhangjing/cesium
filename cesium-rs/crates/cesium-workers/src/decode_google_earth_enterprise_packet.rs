//! Ported from `packages/engine/Source/Workers/decodeGoogleEarthEnterprisePacket.js`.
//!
//! Worker entry point for decoding Google Earth Enterprise terrain/image packets.

/// Decodes Google Earth Enterprise packets.
///
/// In CesiumJS, this receives a Google Earth Enterprise packet (containing
/// compressed terrain or imagery data) and decodes it into usable format.
pub fn decode_google_earth_enterprise_packet(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("decodeGoogleEarthEnterprisePacket"))
}

/// Decodes a Google Earth Enterprise packet (for in-process use).
///
/// # Arguments
/// * `packet_data` - The raw packet bytes.
///
/// Returns decoded packet data.
pub fn decode_google_earth_enterprise_packet_unpacked(
    _packet_data: &[u8],
) -> Vec<u8> {
    // DEVIATION: Google Earth Enterprise format decoding not yet implemented
    Vec::new()
}
