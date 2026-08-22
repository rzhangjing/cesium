//! Ported from `packages/engine/Source/Core/RectangleOutlineGeometry.js`.
//! NOTE: Requires `RectangleGeometryLibrary` for `create_geometry`.

use crate::rectangle::Rectangle;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RectangleOutlineGeometry {
    rectangle: Rectangle,
    height: f64,
    extruded_height: f64,
    granularity: f64,
}

impl RectangleOutlineGeometry {
    pub fn new(rectangle: Rectangle, height: Option<f64>, extruded_height: Option<f64>, granularity: Option<f64>) -> Self {
        Self { rectangle, height: height.unwrap_or(0.0), extruded_height: extruded_height.unwrap_or(0.0), granularity: granularity.unwrap_or(0.017453292519943295) }
    }
    // TODO: create_geometry
}
