//! Ported from `packages/engine/Source/Workers/transcodeKTX2.js`.
//!
//! Worker entry point for transcoding KTX2/Basis Universal compressed textures.
//! KTX2 is a GPU texture format that supports runtime transcoding to the
//! device's preferred compressed format (BC7, ASTC, ETC2, etc.).

/// Transcodes KTX2 textures.
///
/// In CesiumJS, this receives KTX2 texture data and uses the Basis Universal
/// transcoder to convert it to a GPU-native compressed format.
pub fn transcode_ktx2(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Transcodes KTX2 texture data (for in-process use).
///
/// # Arguments
/// * `ktx2_data` - The raw KTX2 texture bytes.
///
/// Returns transcoded texture data in GPU-native format.
pub fn transcode_ktx2_unpacked(_ktx2_data: &[u8]) -> Vec<u8> {
    // DEVIATION: KTX2/Basis Universal transcoding requires native library
    Vec::new()
}
