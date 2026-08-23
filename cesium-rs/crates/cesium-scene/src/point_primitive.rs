//! Ported from `packages/engine/Source/Scene/PointPrimitive.js`.
//!
//! A point primitive in a collection.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;

use crate::frame_state::FrameState;

/// A point primitive in a [`PointPrimitiveCollection`](crate::point_primitive_collection::PointPrimitiveCollection).
///
/// Mirrors CesiumJS `PointPrimitive` (505 lines).
pub struct PointPrimitive {
    /// The position of the point in world coordinates.
    pub position: Cartesian3,
    /// The pixel offset from the position.
    pub pixel_offset: Cartesian3,
    /// The color of the point.
    pub color: Color,
    /// The outline color.
    pub outline_color: Color,
    /// The outline width in pixels.
    pub outline_width: f64,
    /// The size of the point in pixels.
    pub scale: f64,
    /// Whether the point is shown.
    pub show: bool,
    /// The distance display condition (near, far).
    pub distance_display_condition: Option<(f64, f64)>,
    /// Whether this point has been destroyed.
    is_destroyed: bool,
}

impl PointPrimitive {
    /// Creates a new PointPrimitive.
    pub fn new() -> Self {
        Self {
            position: Cartesian3::ZERO,
            pixel_offset: Cartesian3::ZERO,
            color: Color::new(1.0, 1.0, 1.0, 1.0),
            outline_color: Color::new(0.0, 0.0, 0.0, 1.0),
            outline_width: 0.0,
            scale: 1.0,
            show: true,
            distance_display_condition: None,
            is_destroyed: false,
        }
    }

    /// Updates the point for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        // DEVIATION: Requires GPU buffer update
    }

    /// Returns whether this point has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }
}

impl Default for PointPrimitive {
    fn default() -> Self { Self::new() }
}
