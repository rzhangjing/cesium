//! Ported from `packages/engine/Source/Core/WallGeometry.js`.
//!
//! A wall, similar to a KML line string, defined by a series of points
//! that extrude down to the ground.
//!
//! NOTE: `create_geometry` requires `WallGeometryLibrary` / `PolylinePipeline`.

use crate::cartesian3::Cartesian3;
use crate::ellipsoid::Ellipsoid;
use crate::math::CesiumMath;
use crate::vertex_format::VertexFormat;

/// A description of a wall.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WallGeometry {
    positions: Vec<Cartesian3>,
    maximum_heights: Option<Vec<f64>>,
    minimum_heights: Option<Vec<f64>>,
    vertex_format: VertexFormat,
    granularity: f64,
    ellipsoid: Ellipsoid,
}

impl WallGeometry {
    /// Creates a new `WallGeometry`.
    pub fn new(
        positions: Vec<Cartesian3>,
        maximum_heights: Option<Vec<f64>>,
        minimum_heights: Option<Vec<f64>>,
        vertex_format: Option<VertexFormat>,
        granularity: Option<f64>,
        ellipsoid: Option<Ellipsoid>,
    ) -> Self {
        debug_assert!(positions.len() >= 2, "At least 2 positions required.");
        if let Some(ref mh) = maximum_heights {
            debug_assert_eq!(mh.len(), positions.len());
        }
        if let Some(ref mh) = minimum_heights {
            debug_assert_eq!(mh.len(), positions.len());
        }
        Self {
            positions,
            maximum_heights,
            minimum_heights,
            vertex_format: vertex_format.unwrap_or_default(),
            granularity: granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE),
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84.clone()),
        }
    }

    /// Creates a wall from constant min/max heights.
    pub fn from_constant_heights(
        positions: Vec<Cartesian3>,
        minimum_height: Option<f64>,
        maximum_height: Option<f64>,
        vertex_format: Option<VertexFormat>,
        ellipsoid: Option<Ellipsoid>,
    ) -> Self {
        let len = positions.len();
        let min_heights = minimum_height.map(|h| vec![h; len]);
        let max_heights = maximum_height.map(|h| vec![h; len]);
        Self::new(positions, max_heights, min_heights, vertex_format, None, ellipsoid)
    }

    /// The positions.
    pub fn positions(&self) -> &[Cartesian3] {
        &self.positions
    }

    // TODO: create_geometry — requires WallGeometryLibrary / PolylinePipeline
}
