//! Ported from `packages/engine/Source/Workers/createGroundPolylineGeometry.js`.
//!
//! Worker entry point for creating ground-clamped polyline geometry.
//! This generates geometry that is draped onto the terrain surface.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::ground_polyline_geometry::GroundPolylineGeometry;

/// Creates ground polyline geometry in a worker.
///
/// Deserializes polyline positions, width, and arc type from packed bytes.
/// Constructs `GroundPolylineGeometry` and returns the packed result.
pub fn create_ground_polyline_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createGroundPolylineGeometry"))
}

/// Creates a ground polyline geometry from unpacked parameters (for in-process use).
///
/// Mirror of the JS worker body: the JS entry waits for
/// `ApproximateTerrainHeights.initialize()` before creating the geometry;
/// the Rust mirror performs the synchronous `initialize()` first (reads the
/// static approximate terrain heights table from disk).
///
/// # Arguments
/// * `positions` - The polyline positions (will be clamped to ground).
/// * `width` - The polyline width in pixels.
///
/// # Panics
/// Panics if the approximate terrain heights asset cannot be loaded
/// (the JS equivalent rejects the initialization promise).
pub fn create_ground_polyline_geometry_unpacked(
    positions: &[Cartesian3],
    width: f64,
) -> Option<cesium_core::geometry::Geometry> {
    cesium_core::approximate_terrain_heights::initialize()
        .expect("ApproximateTerrainHeights.initialize failed");
    let ground_polyline_geometry =
        GroundPolylineGeometry::new(positions.to_vec(), Some(width), None, None, None);
    GroundPolylineGeometry::create_geometry(&ground_polyline_geometry)
}
