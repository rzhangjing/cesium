//! Ported from `packages/engine/Source/Core/PolylineVolumeOutlineGeometry.js`.
//! NOTE: Requires `PolylinePipeline` for `create_geometry`.

use crate::cartesian3::Cartesian3;
use crate::polyline_volume_geometry::Cartesian2Stub;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PolylineVolumeOutlineGeometry {
    polyline_positions: Vec<Cartesian3>,
    shape_positions: Vec<Cartesian2Stub>,
    corner_type: u32,
    granularity: f64,
}

impl PolylineVolumeOutlineGeometry {
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
