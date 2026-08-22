//! Ported from `packages/engine/Source/Core/PolylinePipeline.js`.
//!
//! Polyline pipeline utilities. Skeleton implementation.

use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;

/// Computes the number of points between two Cartesian3 positions.
pub fn number_of_points(p0: &Cartesian3, p1: &Cartesian3, min_distance: f64) -> usize {
    let distance = Cartesian3::distance(p0, p1);
    (distance / min_distance).ceil() as usize
}

/// Computes the number of points for a rhumb line.
pub fn number_of_points_rhumb_line(
    p0: &Cartographic,
    p1: &Cartographic,
    granularity: f64,
) -> usize {
    let d_lon = p0.longitude - p1.longitude;
    let d_lat = p0.latitude - p1.latitude;
    let radians_distance_squared = d_lon * d_lon + d_lat * d_lat;
    usize::max(
        1,
        (radians_distance_squared / (granularity * granularity)).sqrt().ceil() as usize,
    )
}

/// Extracts heights from Cartesian3 positions using an ellipsoid.
pub fn extract_heights(positions: &[Cartesian3], ellipsoid: &crate::ellipsoid::Ellipsoid) -> Vec<f64> {
    let params = ellipsoid.ellipsoid_params();
    positions
        .iter()
        .map(|p| {
            let mut carto = Cartographic::default();
            Cartographic::from_cartesian(p, Some(&params), &mut carto);
            carto.height
        })
        .collect()
}
