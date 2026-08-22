//! Ported from `packages/engine/Source/Core/PolylineVolumeGeometry.js`.
//! NOTE: Requires `PolylinePipeline` for `create_geometry`.

use crate::cartesian3::Cartesian3;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PolylineVolumeGeometry {
    polyline_positions: Vec<Cartesian3>,
    shape_positions: Vec<Cartesian2Stub>,
    corner_type: u32, // 0=Rounded, 1=Mitered, 2=Beveled
    granularity: f64,
}

/// Minimal 2D point for shape cross-sections.
#[derive(Debug, Clone, Copy)]
pub struct Cartesian2Stub {
    pub x: f64,
    pub y: f64,
}

impl PolylineVolumeGeometry {
    pub fn new(
        polyline_positions: Vec<Cartesian3>,
        shape_positions: Vec<Cartesian2Stub>,
        corner_type: Option<u32>,
        granularity: Option<f64>,
    ) -> Self {
        Self {
            polyline_positions,
            shape_positions,
            corner_type: corner_type.unwrap_or(0),
            granularity: granularity.unwrap_or(0.017453292519943295),
        }
    }
    // TODO: create_geometry
}
