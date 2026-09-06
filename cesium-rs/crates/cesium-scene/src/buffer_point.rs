//! Ported from `packages/engine/Source/Scene/BufferPoint.js`.

/// A point in a buffer point collection.
///
/// Represents a single point with position, color, and display properties.
pub struct BufferPoint {
    /// X position in buffer coordinates.
    pub x: f64,
    /// Y position in buffer coordinates.
    pub y: f64,
    /// Red component (0.0–1.0).
    pub red: f32,
    /// Green component (0.0–1.0).
    pub green: f32,
    /// Blue component (0.0–1.0).
    pub blue: f32,
    /// Alpha component (0.0–1.0).
    pub alpha: f32,
    /// Point size in pixels.
    pub pixel_size: f32,
    /// Whether this point is visible.
    pub show: bool,
}

impl BufferPoint {
    /// Creates a new BufferPoint.
    pub fn new() -> Self {
        Self {
            x: 0.0, y: 0.0,
            red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0,
            pixel_size: 10.0, show: true,
        }
    }
}

impl Default for BufferPoint {
    fn default() -> Self { Self::new() }
}
