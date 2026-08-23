//! Ported from `packages/engine/Source/Scene/Polyline.js`.
//!
//! A polyline primitive.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;

/// A polyline (line string) defined by a sequence of positions.
///
/// Mirrors CesiumJS `Polyline` (1197 lines).
pub struct Polyline {
    /// The positions defining the polyline.
    pub positions: Vec<Cartesian3>,
    /// The width of the polyline in pixels.
    pub width: f64,
    /// The color of the polyline.
    pub color: Color,
    /// Whether the polyline is shown.
    pub show: bool,
    /// Whether the polyline loops back to the first position.
    pub loop_: bool,
    /// The material ID (for textured polylines).
    pub material_id: Option<String>,
    /// Whether this polyline follows the surface.
    pub follow_surface: bool,
    /// The granularity (in radians) for surface-following polylines.
    pub granularity: f64,
    /// Whether this polyline has been destroyed.
    is_destroyed: bool,
}

impl Polyline {
    /// Creates a new Polyline.
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            width: 1.0,
            color: Color::new(1.0, 1.0, 1.0, 1.0),
            show: true,
            loop_: false,
            material_id: None,
            follow_surface: true,
            granularity: 0.0,
            is_destroyed: false,
        }
    }

    /// Returns whether this polyline has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }
}

impl Default for Polyline {
    fn default() -> Self { Self::new() }
}
