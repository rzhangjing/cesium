//! Ported from `packages/engine/Source/Core/WallOutlineGeometry.js`.
//!
//! A description of a wall outline.
//!
//! NOTE: `create_geometry` requires `WallGeometryLibrary` / `PolylinePipeline`.

use crate::cartesian3::Cartesian3;
use crate::ellipsoid::Ellipsoid;
use crate::math::CesiumMath;

/// A description of a wall outline.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WallOutlineGeometry {
    positions: Vec<Cartesian3>,
    maximum_heights: Option<Vec<f64>>,
    minimum_heights: Option<Vec<f64>>,
    granularity: f64,
    ellipsoid: Ellipsoid,
}

impl WallOutlineGeometry {
    /// Creates a new `WallOutlineGeometry`.
    pub fn new(
        positions: Vec<Cartesian3>,
        maximum_heights: Option<Vec<f64>>,
        minimum_heights: Option<Vec<f64>>,
        granularity: Option<f64>,
        ellipsoid: Option<Ellipsoid>,
    ) -> Self {
        debug_assert!(positions.len() >= 2, "At least 2 positions required.");
        Self {
            positions,
            maximum_heights,
            minimum_heights,
            granularity: granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE),
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84.clone()),
        }
    }

    /// Creates a wall outline from constant min/max heights.
    pub fn from_constant_heights(
        positions: Vec<Cartesian3>,
        minimum_height: Option<f64>,
        maximum_height: Option<f64>,
        ellipsoid: Option<Ellipsoid>,
    ) -> Self {
        let len = positions.len();
        let min_heights = minimum_height.map(|h| vec![h; len]);
        let max_heights = maximum_height.map(|h| vec![h; len]);
        Self::new(positions, max_heights, min_heights, None, ellipsoid)
    }

    /// The positions.
    pub fn positions(&self) -> &[Cartesian3] {
        &self.positions
    }

    // TODO: create_geometry — requires WallGeometryLibrary / PolylinePipeline
}
