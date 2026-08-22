//! Ported from `packages/engine/Source/Core/PinBuilder.js`.
//!
//! Generates pin images for map markers.

/// Generates pin marker images for use on maps.
/// Skeleton: requires canvas rendering.
pub struct PinBuilder;

impl PinBuilder {
    /// Creates a new pin builder.
    pub fn new() -> Self {
        Self
    }

    /// Creates a pin from text.
    pub fn from_text(_text: &str, _color: u32, _size: i32) -> Result<Vec<u8>, String> {
        // Skeleton: requires canvas rendering
        Err("Not implemented".to_string())
    }

    /// Creates a pin from a URL.
    pub fn from_url(_url: &str, _color: u32, _size: i32) -> Result<Vec<u8>, String> {
        Err("Not implemented".to_string())
    }
}
