//! Polyline pipeline - arc subdivision with height interpolation.
//!
//! Faithful port of CesiumJS `PolylinePipeline.js`. The core routine
//! [`generate_arc`] subdivides a polyline into a geodesic arc on the ellipsoid,
//! raising every generated point to a (per-vertex interpolated) height. This is
//! the foundation of wall, corridor and polyline geometry.

use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::geodesic::EllipsoidGeodesic;
use crate::math_utils::chord_length;
use glam::DVec3;

/// Default granularity: one degree in radians (CesiumJS `RADIANS_PER_DEGREE`).
pub const DEFAULT_GRANULARITY: f64 = std::f64::consts::PI / 180.0;

/// Number of subdivisions for a segment so that no chord exceeds `min_distance`.
///
/// Maps to `PolylinePipeline.numberOfPoints`.
pub fn number_of_points(p0: DVec3, p1: DVec3, min_distance: f64) -> usize {
    let distance = p0.distance(p1);
    (distance / min_distance).ceil() as usize
}

/// Linearly subdivides heights between `h0` and `h1` into `num_points` samples.
///
/// Maps to the private `subdivideHeights`.
fn subdivide_heights(num_points: usize, h0: f64, h1: f64) -> Vec<f64> {
    let mut heights = vec![0.0; num_points];
    if (h0 - h1).abs() < f64::EPSILON {
        heights.fill(h0);
        return heights;
    }
    let d_height = h1 - h0;
    let height_per_vertex = d_height / num_points as f64;
    for (i, h) in heights.iter_mut().enumerate() {
        *h = h0 + i as f64 * height_per_vertex;
    }
    heights
}

/// Generates a single cartesian arc from `p0` to `p1` (includes `p0`, excludes
/// `p1`), appending results to `out`. Returns the number of points appended.
///
/// Maps to the private `generateCartesianArc`.
fn generate_cartesian_arc(
    p0: DVec3,
    p1: DVec3,
    min_distance: f64,
    ellipsoid: &Ellipsoid,
    h0: f64,
    h1: f64,
    out: &mut Vec<DVec3>,
) -> usize {
    let first = ellipsoid.scale_to_geodetic_surface(p0).unwrap_or(p0);
    let last = ellipsoid.scale_to_geodetic_surface(p1).unwrap_or(p1);
    let num_points = number_of_points(p0, p1, min_distance);

    let start = ellipsoid.cartesian_to_cartographic(first).unwrap_or_default();
    let end = ellipsoid.cartesian_to_cartographic(last).unwrap_or_default();
    let heights = subdivide_heights(num_points, h0, h1);

    let geodesic = EllipsoidGeodesic::new(start, end, ellipsoid);
    let surface_distance_between_points = geodesic.surface_distance() / num_points as f64;

    // First point at h0.
    let mut start_carto = start;
    start_carto.height = h0;
    out.push(ellipsoid.cartographic_to_cartesian(&start_carto));

    for (i, height) in heights.iter().enumerate().skip(1) {
        let mut carto =
            geodesic.interpolate_using_surface_distance(i as f64 * surface_distance_between_points);
        carto.height = *height;
        out.push(ellipsoid.cartographic_to_cartesian(&carto));
    }

    num_points
}

/// Options for [`generate_arc`].
pub struct ArcOptions<'a> {
    /// The polyline positions.
    pub positions: &'a [DVec3],
    /// Per-vertex heights. `None` means height 0 for every vertex.
    pub heights: Option<&'a [f64]>,
    /// Angular granularity in radians.
    pub granularity: f64,
    /// The reference ellipsoid.
    pub ellipsoid: &'a Ellipsoid,
}

/// Subdivides a polyline into a geodesic arc and raises every point to its
/// (interpolated) height.
///
/// Maps to `PolylinePipeline.generateArc`.
pub fn generate_arc(options: &ArcOptions) -> Vec<DVec3> {
    let positions = options.positions;
    let ellipsoid = options.ellipsoid;
    let length = positions.len();

    if length < 1 {
        return Vec::new();
    }

    let height_at = |i: usize| -> f64 {
        match options.heights {
            Some(h) => h[i],
            None => 0.0,
        }
    };

    if length == 1 {
        let mut p = ellipsoid.scale_to_geodetic_surface(positions[0]).unwrap_or(positions[0]);
        let height = height_at(0);
        if height != 0.0 {
            let n = ellipsoid.geodetic_surface_normal(p).unwrap_or(DVec3::Z);
            p += n * height;
        }
        return vec![p];
    }

    let min_distance = chord_length(options.granularity, ellipsoid.maximum_radius());

    let mut result = Vec::new();
    for i in 0..length - 1 {
        let p0 = positions[i];
        let p1 = positions[i + 1];
        let h0 = height_at(i);
        let h1 = height_at(i + 1);
        generate_cartesian_arc(p0, p1, min_distance, ellipsoid, h0, h1, &mut result);
    }

    // Append the final point exactly.
    let last_point = positions[length - 1];
    let mut carto = ellipsoid
        .cartesian_to_cartographic(last_point)
        .unwrap_or(Cartographic::ZERO);
    carto.height = height_at(length - 1);
    result.push(ellipsoid.cartographic_to_cartesian(&carto));

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math_utils::to_radians;

    #[test]
    fn test_number_of_points() {
        let p0 = DVec3::new(0.0, 0.0, 0.0);
        let p1 = DVec3::new(10.0, 0.0, 0.0);
        assert_eq!(number_of_points(p0, p1, 3.0), 4); // ceil(10/3)
        assert_eq!(number_of_points(p0, p1, 5.0), 2); // ceil(10/5)
    }

    #[test]
    fn test_generate_arc_single_point() {
        let ell = Ellipsoid::WGS84;
        let pos = ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0));
        let opts = ArcOptions {
            positions: &[pos],
            heights: None,
            granularity: DEFAULT_GRANULARITY,
            ellipsoid: &ell,
        };
        let arc = generate_arc(&opts);
        assert_eq!(arc.len(), 1);
    }

    #[test]
    fn test_generate_arc_endpoints_preserved() {
        let ell = Ellipsoid::WGS84;
        let p0 = ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0));
        let p1 = ell.cartographic_to_cartesian(&Cartographic::from_degrees(10.0, 0.0, 0.0));
        let opts = ArcOptions {
            positions: &[p0, p1],
            heights: None,
            granularity: DEFAULT_GRANULARITY,
            ellipsoid: &ell,
        };
        let arc = generate_arc(&opts);
        assert!(arc.len() >= 3, "arc len {}", arc.len());
        // First point near p0, last point near p1.
        assert!((arc[0] - p0).length() < 1.0);
        assert!((arc[arc.len() - 1] - p1).length() < 1.0);
        // All points on the surface (height ~ 0).
        for p in &arc {
            let c = ell.cartesian_to_cartographic(*p).unwrap();
            assert!(c.height.abs() < 1e-3, "height {}", c.height);
        }
    }

    #[test]
    fn test_generate_arc_with_heights() {
        let ell = Ellipsoid::WGS84;
        let p0 = ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0));
        let p1 = ell.cartographic_to_cartesian(&Cartographic::from_degrees(10.0, 0.0, 0.0));
        let heights = [1000.0, 1000.0];
        let opts = ArcOptions {
            positions: &[p0, p1],
            heights: Some(&heights),
            granularity: DEFAULT_GRANULARITY,
            ellipsoid: &ell,
        };
        let arc = generate_arc(&opts);
        // Every point should be at ~1000 m height.
        for p in &arc {
            let c = ell.cartesian_to_cartographic(*p).unwrap();
            assert!((c.height - 1000.0).abs() < 1.0, "height {}", c.height);
        }
    }

    #[test]
    fn test_generate_arc_geodesic_not_linear() {
        // A geodesic between two points at the same latitude (off equator) bows
        // toward the pole relative to a constant-latitude line.
        let ell = Ellipsoid::WGS84;
        let p0 = ell.cartographic_to_cartesian(&Cartographic::from_degrees(-10.0, 45.0, 0.0));
        let p1 = ell.cartographic_to_cartesian(&Cartographic::from_degrees(10.0, 45.0, 0.0));
        let opts = ArcOptions {
            positions: &[p0, p1],
            heights: None,
            granularity: to_radians(0.5),
            ellipsoid: &ell,
        };
        let arc = generate_arc(&opts);
        // The midpoint latitude of a great circle should be > 45 degrees.
        let mid = arc[arc.len() / 2];
        let mid_carto = ell.cartesian_to_cartographic(mid).unwrap();
        assert!(
            mid_carto.latitude > to_radians(45.0),
            "mid lat {}",
            mid_carto.latitude
        );
    }
}
