//! Geodesic measurement utilities (plan §16 M9 "量测(距离/面积)" integration
//! point).
//!
//! Pure functions over model geometry so distance / area read-outs unit-test
//! headless; the bridge binds them to a measure tool / status line. Distances
//! reuse [`GeoPoint::surface_distance`] (great-circle metres); areas use the
//! spherical trapezoidal (Green's-theorem) ring integral, accurate to well
//! within a percent for authoring-sized polygons.
//!
//! These are deliberately *not* wired into rendering or history — they are the
//! stable entry points a later dedicated measurement milestone grows on.

use crate::geo::{GeoPoint, METERS_PER_RENDER_UNIT};
use crate::model::geometry::{Circle, Geometry, Polygon, Rectangle};

/// Great-circle length of a vertex chain (metres); `0` for fewer than two points.
pub fn path_length_m(points: &[GeoPoint]) -> f64 {
    points
        .windows(2)
        .map(|w| w[0].surface_distance(w[1]))
        .sum()
}

/// Perimeter of a closed ring (last vertex joined back to the first), metres.
pub fn ring_length_m(ring: &[GeoPoint]) -> f64 {
    if ring.len() < 2 {
        return 0.0;
    }
    let mut total = path_length_m(ring);
    total += ring[ring.len() - 1].surface_distance(ring[0]);
    total
}

/// Spherical area enclosed by a linear ring (m², unsigned). Uses the trapezoidal
/// line integral  `A = R² · |Σ (λ_{i+1} − λ_i)(sin φ_i + sin φ_{i+1})| / 2`.
/// Degenerate rings (< 3 points) have zero area.
pub fn ring_area_m2(ring: &[GeoPoint]) -> f64 {
    let n = ring.len();
    if n < 3 {
        return 0.0;
    }
    let mut sum = 0.0;
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        let dlon = (b.lon_deg - a.lon_deg).to_radians();
        let s = a.lat_deg.to_radians().sin() + b.lat_deg.to_radians().sin();
        sum += dlon * s;
    }
    (METERS_PER_RENDER_UNIT * METERS_PER_RENDER_UNIT * sum / 2.0).abs()
}

/// Area of a polygon = outer ring minus the sum of its holes (m²).
pub fn polygon_area_m2(pg: &Polygon) -> f64 {
    let mut area = ring_area_m2(&pg.outer);
    for h in &pg.holes {
        area -= ring_area_m2(h);
    }
    area.max(0.0)
}

/// Area of a ground circle treated as flat `π r²` (m²) — the radius is a ground
/// distance, so the planar form is the intended semantic here.
pub fn circle_area_m2(c: &Circle) -> f64 {
    std::f64::consts::PI * c.radius_m * c.radius_m
}

/// Rectangle area (m²): width (a great-circle span at the mean latitude) times
/// height (a meridional span).
pub fn rectangle_area_m2(r: &Rectangle) -> f64 {
    let mean_lat = (r.south + r.north) / 2.0;
    let west_edge = GeoPoint::surface(r.west, mean_lat);
    let east_edge = GeoPoint::surface(r.east, mean_lat);
    let width = west_edge.surface_distance(east_edge);
    let height = GeoPoint::surface(r.west, r.south).surface_distance(GeoPoint::surface(r.west, r.north));
    width * height
}

/// Total measured length of a geometry: polyline length, polygon / rectangle
/// perimeter, circle circumference; `0` for point-like kinds.
pub fn measure_length_m(g: &Geometry) -> f64 {
    match g {
        Geometry::Polyline(pl) => path_length_m(&pl.positions),
        Geometry::Polygon(pg) => {
            let mut t = ring_length_m(&pg.outer);
            for h in &pg.holes {
                t += ring_length_m(h);
            }
            t
        }
        Geometry::Rectangle(r) => {
            let corners = [
                GeoPoint::surface(r.west, r.south),
                GeoPoint::surface(r.east, r.south),
                GeoPoint::surface(r.east, r.north),
                GeoPoint::surface(r.west, r.north),
            ];
            ring_length_m(&corners)
        }
        Geometry::Circle(c) => 2.0 * std::f64::consts::PI * c.radius_m,
        Geometry::Composite(comp) => comp.parts.iter().map(measure_length_m).sum(),
        _ => 0.0,
    }
}

/// Total measured area of a geometry; `0` for open / point-like kinds.
pub fn measure_area_m2(g: &Geometry) -> f64 {
    match g {
        Geometry::Polygon(pg) => polygon_area_m2(pg),
        Geometry::Rectangle(r) => rectangle_area_m2(r),
        Geometry::Circle(c) => circle_area_m2(c),
        Geometry::Composite(comp) => comp.parts.iter().map(measure_area_m2).sum(),
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::geometry::Polyline;

    fn p(lon: f64, lat: f64) -> GeoPoint {
        GeoPoint::surface(lon, lat)
    }

    #[test]
    fn one_degree_latitude_is_about_111km() {
        let d = p(0.0, 0.0).surface_distance(p(0.0, 1.0));
        assert!((d - 111_319.0).abs() < 100.0, "got {d}");
    }

    #[test]
    fn path_length_sums_segments() {
        let line = vec![p(0.0, 0.0), p(0.0, 1.0), p(0.0, 2.0)];
        let total = path_length_m(&line);
        let one = p(0.0, 0.0).surface_distance(p(0.0, 1.0));
        assert!((total - 2.0 * one).abs() < 1.0);
        assert_eq!(path_length_m(&[p(0.0, 0.0)]), 0.0);
    }

    #[test]
    fn unit_square_area_near_equator() {
        // A 1°×1° ring near the equator ≈ 1.239e10 m².
        let ring = vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)];
        let area = ring_area_m2(&ring);
        let expect = 1.239e10;
        assert!(
            (area - expect).abs() / expect < 0.01,
            "area {area} not within 1% of {expect}"
        );
        assert_eq!(ring_area_m2(&[p(0.0, 0.0), p(1.0, 1.0)]), 0.0, "degenerate ring");
    }

    #[test]
    fn polygon_area_subtracts_holes() {
        let outer = vec![p(0.0, 0.0), p(2.0, 0.0), p(2.0, 2.0), p(0.0, 2.0)];
        let hole = vec![p(0.5, 0.5), p(1.5, 0.5), p(1.5, 1.5), p(0.5, 1.5)];
        let pg = Polygon {
            outer: outer.clone(),
            holes: vec![hole.clone()],
        };
        let with_hole = polygon_area_m2(&pg);
        let net = ring_area_m2(&outer) - ring_area_m2(&hole);
        assert!((with_hole - net).abs() < 1.0);
        assert!(with_hole > 0.0);
    }

    #[test]
    fn circle_metrics() {
        let c = Circle {
            center: p(0.0, 0.0),
            radius_m: 1000.0,
        };
        assert!((circle_area_m2(&c) - std::f64::consts::PI * 1e6).abs() < 1.0);
        assert!((measure_length_m(&Geometry::Circle(c)) - 2.0 * std::f64::consts::PI * 1000.0).abs() < 1.0);
    }

    #[test]
    fn geometry_dispatch_matches_parts() {
        let line = Geometry::Polyline(Polyline {
            positions: vec![p(0.0, 0.0), p(0.0, 1.0)],
        });
        assert!(measure_length_m(&line) > 100_000.0);
        assert_eq!(measure_area_m2(&line), 0.0, "open line has no area");
        let rect = Geometry::Rectangle(Rectangle {
            west: 0.0,
            south: 0.0,
            east: 1.0,
            north: 1.0,
        });
        let rect_area = measure_area_m2(&rect);
        let square_area = ring_area_m2(&[p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)]);
        assert!(
            (rect_area - square_area).abs() / square_area < 0.02,
            "rect {rect_area} vs ring {square_area}"
        );
    }
}
