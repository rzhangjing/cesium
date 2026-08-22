//! Ported from `packages/engine/Source/Core/SimplePolylineGeometry.js`.
//!
//! A description of a polyline modeled as a line strip.
//!
//! NOTE: Full `create_geometry` requires `PolylinePipeline` and `Color`
//! which have not yet been ported. The data structure and pack/unpack
//! stubs are available.

use crate::cartesian3::Cartesian3;
use crate::ellipsoid::Ellipsoid;

/// Arc type for polyline interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ArcType {
    /// Follows the shortest path on the ellipsoid surface.
    Geodesic = 0,
    /// Straight line in Cartesian space.
    Straight = 1,
    /// Follows a path of constant bearing.
    Rhumb = 2,
}

/// A polyline described by a sequence of positions.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SimplePolylineGeometry {
    positions: Vec<Cartesian3>,
    colors: Option<Vec<[f64; 4]>>,
    colors_per_vertex: bool,
    arc_type: ArcType,
    granularity: f64,
    ellipsoid: Ellipsoid,
}

impl SimplePolylineGeometry {
    /// Creates a new `SimplePolylineGeometry`.
    pub fn new(
        positions: Vec<Cartesian3>,
        colors: Option<Vec<[f64; 4]>>,
        colors_per_vertex: Option<bool>,
        arc_type: Option<ArcType>,
        granularity: Option<f64>,
        ellipsoid: Option<Ellipsoid>,
    ) -> Self {
        debug_assert!(positions.len() >= 2, "At least two positions are required.");
        Self {
            positions,
            colors,
            colors_per_vertex: colors_per_vertex.unwrap_or(false),
            arc_type: arc_type.unwrap_or(ArcType::Geodesic),
            granularity: granularity.unwrap_or(0.017453292519943295), // RADIANS_PER_DEGREE
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84.clone()),
        }
    }

    /// The positions.
    pub fn positions(&self) -> &[Cartesian3] {
        &self.positions
    }

    // TODO: create_geometry — requires PolylinePipeline + Color port
}
