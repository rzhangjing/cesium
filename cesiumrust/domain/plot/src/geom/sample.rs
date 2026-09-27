//! Geometry sampling / subdivision (plan §6).
//!
//! Turns the parametric primitives into vertex [`Ring`]s of [`GeoPoint`]s, and
//! densifies straight geodesics into great-circle arcs so a line hugs the sphere
//! in the globe view. Everything is pure and works in geographic coordinates, so
//! the bridge can project the resulting rings with the same reprojection it uses
//! for every other vertex — and the whole thing is unit-testable headless.
//!
//! The geodesic maths uses a spherical Earth of radius [`METERS_PER_RENDER_UNIT`]
//! (the viewer's render-unit scale), which is exact-enough for the overlay's
//! ground circles / ellipses at typical plotting extents.

use std::f64::consts::TAU;

use crate::geo::{GeoPoint, METERS_PER_RENDER_UNIT};
use crate::model::geometry::{Arc3, Circle, Ellipse, Rectangle, Ring};

/// Default number of segments a full circle / ellipse is sampled into.
pub const DEFAULT_SEGMENTS: usize = 64;
/// Angular step (radians) a great-circle span is subdivided into (~1°), so a
/// long globe line follows the sphere instead of cutting a chord through it.
pub const GREAT_CIRCLE_STEP_RAD: f64 = 1.0f64.to_radians();

/// Great-circle destination point from `start`, travelling `dist_m` metres along
/// `bearing_deg` (clockwise from true north). Spherical model.
pub fn offset_point(start: GeoPoint, bearing_deg: f64, dist_m: f64) -> GeoPoint {
    let r = METERS_PER_RENDER_UNIT;
    let d = dist_m / r; // angular distance (radians)
    let th = bearing_deg.to_radians();
    let lat1 = start.lat_deg.to_radians();
    let lon1 = start.lon_deg.to_radians();
    let sin_lat2 = lat1.sin() * d.cos() + lat1.cos() * d.sin() * th.cos();
    let lat2 = sin_lat2.clamp(-1.0, 1.0).asin();
    let lon2 = lon1
        + (th.sin() * d.sin() * lat1.cos()).atan2(d.cos() - lat1.sin() * sin_lat2);
    GeoPoint::new(lon2.to_degrees(), lat2.to_degrees(), start.height_m)
}

/// A ground circle → `segments` vertices at constant `radius_m` from the centre,
/// starting due north and sweeping clockwise.
pub fn circle_ring(c: &Circle, segments: usize) -> Ring {
    let n = segments.max(3);
    (0..n)
        .map(|i| {
            let bearing = i as f64 * 360.0 / n as f64;
            offset_point(c.center, bearing, c.radius_m)
        })
        .collect()
}

/// A ground ellipse → `segments` vertices. The major axis points at
/// `rotation_deg` clockwise from north; a point is placed by rotating its
/// (along-major, along-minor) offsets into north/east metres and stepping from
/// the centre (a flat-earth ENU approximation, exact enough for plotting).
pub fn ellipse_ring(e: &Ellipse, segments: usize) -> Ring {
    let n = segments.max(3);
    let rot = e.rotation_deg.to_radians();
    (0..n)
        .map(|i| {
            let phi = i as f64 * TAU / n as f64;
            let along_major = e.semi_major_m * phi.cos();
            let along_minor = e.semi_minor_m * phi.sin();
            // Rotate into north / east components (major axis at `rot` from N).
            let north = along_major * rot.cos() - along_minor * rot.sin();
            let east = along_major * rot.sin() + along_minor * rot.cos();
            let p = offset_point(e.center, 0.0, north);
            offset_point(p, 90.0, east)
        })
        .collect()
}

/// A lat/lon rectangle → its four corners (west-south-east-north), CCW.
pub fn rectangle_ring(r: &Rectangle) -> Ring {
    vec![
        GeoPoint::surface(r.west, r.south),
        GeoPoint::surface(r.east, r.south),
        GeoPoint::surface(r.east, r.north),
        GeoPoint::surface(r.west, r.north),
    ]
}

/// A three-point arc (`start` → via `center` → `end`) sampled as a quadratic
/// Bézier in lon/lat whose control point is chosen so the curve passes exactly
/// through `center` at the midpoint. Endpoints and midpoint are exact.
pub fn arc_ring(a: &Arc3, segments: usize) -> Ring {
    let n = segments.max(2);
    // Control point P1 with B(0.5) = center  ⇒  P1 = 2·center − (P0 + P2)/2.
    let ctrl = |p0: f64, c: f64, p2: f64| 2.0 * c - 0.5 * (p0 + p2);
    let lon1 = ctrl(a.start.lon_deg, a.center.lon_deg, a.end.lon_deg);
    let lat1 = ctrl(a.start.lat_deg, a.center.lat_deg, a.end.lat_deg);
    let h1 = ctrl(a.start.height_m, a.center.height_m, a.end.height_m);
    (0..=n)
        .map(|i| {
            let t = i as f64 / n as f64;
            let mt = 1.0 - t;
            let lerp = |p0: f64, pc: f64, p2: f64| {
                mt * mt * p0 + 2.0 * mt * t * pc + t * t * p2
            };
            GeoPoint::new(
                lerp(a.start.lon_deg, lon1, a.end.lon_deg),
                lerp(a.start.lat_deg, lat1, a.end.lat_deg),
                lerp(a.start.height_m, h1, a.end.height_m),
            )
        })
        .collect()
}

/// Unit direction of a geographic point on the sphere (for great-circle slerp).
fn unit_dir(p: GeoPoint) -> glam::DVec3 {
    let lat = p.lat_deg.to_radians();
    let lon = p.lon_deg.to_radians();
    glam::DVec3::new(lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin())
}

fn dir_to_geo(d: glam::DVec3, height_m: f64) -> GeoPoint {
    let lat = d.z.clamp(-1.0, 1.0).asin().to_degrees();
    let lon = d.y.atan2(d.x).to_degrees();
    GeoPoint::new(lon, lat, height_m)
}

/// Spherical linear interpolation between two unit directions.
fn slerp(a: glam::DVec3, b: glam::DVec3, t: f64) -> glam::DVec3 {
    let dot = a.dot(b).clamp(-1.0, 1.0);
    let omega = dot.acos();
    if omega < 1e-12 {
        return a;
    }
    let s = omega.sin();
    (a * ((1.0 - t) * omega).sin() + b * (t * omega).sin()) / s
}

/// Densify a polyline so each great-circle span is broken into sub-steps of at
/// most `step_rad` (default [`GREAT_CIRCLE_STEP_RAD`]). Endpoints are preserved
/// and each pair is interpolated along the sphere, so a globe line follows the
/// surface rather than cutting a chord. Returns the subdivided coordinate list.
pub fn subdivide_great_circle(positions: &[GeoPoint], step_rad: f64) -> Vec<GeoPoint> {
    let step = step_rad.max(1e-6);
    let mut out = Vec::new();
    for (i, w) in positions.iter().enumerate() {
        if i + 1 < positions.len() {
            let a = unit_dir(*w);
            let b = unit_dir(positions[i + 1]);
            let omega = a.dot(b).clamp(-1.0, 1.0).acos();
            let n = (omega / step).ceil() as usize;
            out.push(*w);
            for k in 1..n {
                let t = k as f64 / n as f64;
                let h = w.height_m * (1.0 - t) + positions[i + 1].height_m * t;
                out.push(dir_to_geo(slerp(a, b, t), h));
            }
        } else {
            out.push(*w);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn offset_due_north_increases_latitude() {
        let start = GeoPoint::surface(0.0, 0.0);
        let n100 = offset_point(start, 0.0, 1_000_000.0);
        assert!(n100.lat_deg > 8.0 && n100.lat_deg < 10.0, "{n100:?}");
        assert!(approx(n100.lon_deg, 0.0, 1e-9));
        // Due east keeps the equator latitude, advances longitude.
        let e100 = offset_point(start, 90.0, 1_000_000.0);
        assert!(approx(e100.lat_deg, 0.0, 1e-6), "{e100:?}");
        assert!(e100.lon_deg > 8.0);
    }

    #[test]
    fn circle_is_closed_and_equidistant() {
        let c = Circle {
            center: GeoPoint::surface(10.0, 50.0),
            radius_m: 100_000.0,
        };
        let ring = circle_ring(&c, 36);
        assert_eq!(ring.len(), 36);
        for p in &ring {
            // Every vertex sits ~radius metres from the centre.
            let d = c.center.surface_distance(*p);
            assert!((d - 100_000.0).abs() < 500.0, "{d}");
        }
    }

    #[test]
    fn rectangle_corners() {
        let r = Rectangle {
            west: -10.0,
            south: 20.0,
            east: 30.0,
            north: 40.0,
        };
        let ring = rectangle_ring(&r);
        assert_eq!(ring.len(), 4);
        assert_eq!((ring[0].lon_deg, ring[0].lat_deg), (-10.0, 20.0));
        assert_eq!((ring[2].lon_deg, ring[2].lat_deg), (30.0, 40.0));
    }

    #[test]
    fn arc_hits_endpoints_and_via() {
        let a = Arc3 {
            start: GeoPoint::surface(0.0, 0.0),
            center: GeoPoint::surface(5.0, 5.0),
            end: GeoPoint::surface(10.0, 0.0),
        };
        let ring = arc_ring(&a, 8);
        assert_eq!(ring.len(), 9); // segments + 1 (inclusive endpoints)
        assert!(approx(ring[0].lon_deg, 0.0, 1e-9));
        assert!(approx(ring[8].lon_deg, 10.0, 1e-9));
        assert!(approx(ring[4].lon_deg, 5.0, 1e-6), "midpoint via");
        assert!(approx(ring[4].lat_deg, 5.0, 1e-6));
    }

    #[test]
    fn great_circle_densifies_and_preserves_endpoints() {
        let pts = vec![GeoPoint::surface(0.0, 0.0), GeoPoint::surface(90.0, 0.0)];
        let sub = subdivide_great_circle(&pts, 10f64.to_radians());
        assert!(sub.len() > 3, "should add intermediate points");
        assert!(approx(sub.first().unwrap().lon_deg, 0.0, 1e-9));
        assert!(approx(sub.last().unwrap().lon_deg, 90.0, 1e-9));
        // A shorter span than the step stays a single segment (endpoints only).
        let short = subdivide_great_circle(&pts, 200f64.to_radians());
        assert_eq!(short.len(), 2);
    }
}
