//! Ported from `packages/engine/Source/Core/loadKTX2.js`.
//!
//! Loads and parses KTX2 texture files.

/// Loads and parses KTX2 texture files.
/// Skeleton: requires KTX2 transcoder.
pub struct LoadKTX2;

impl LoadKTX2 {
    /// Loads KTX2 data from bytes.
    pub fn load(_data: &[u8]) -> Result<(), String> {
        // Skeleton: requires transcoder
        Err("Not implemented".to_string())
    }
}
