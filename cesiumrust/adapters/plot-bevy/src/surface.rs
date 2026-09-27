//! The `GeoSurface` abstraction (plan §3): turning a screen cursor back into a
//! geographic coordinate, mode-independently.
//!
//! M2 uses this only for a couple of pure helpers the picking / drawing
//! milestones lean on, but the geometry is fully exercised here so the surface
//! contract is proven before any pointer event is wired. Both directions share
//! [`crate::reproject`]'s `ViewMetrics`, so "what the user sees" and "where a
//! vertex is drawn" can never disagree.
//!
//!  * [`ray_to_globe`] — a world-space pick ray against the SAME WGS84 ellipsoid
//!    the plot vertices are projected onto (semi-axes `1` equatorial,
//!    `0.99664719` polar, in render units), returning the near surface hit.
//!  * [`ray_to_flat`] — the ray's crossing of the `z = plane_z` equirectangular
//!    plane (for the orthographic top-down camera this is a straight drop).

use bevy::math::{Vec2, Vec3};
use cesium_plot::geo::{flat_to_geo, globe_to_geo, GeoPoint};

/// WGS84 polar/equatorial radius ratio in render units — MUST match
/// [`cesium_plot::geo::geo_to_globe`] so picks land exactly on drawn vertices.
const POLAR_RATIO: f64 = 6356752.314245 / 6378137.0;

/// Intersect a world ray (origin + direction, render units, globe centred at the
/// origin) with the plot ellipsoid and return the near surface hit as a
/// [`GeoPoint`]. `None` on a miss or when the only intersection is behind the
/// ray. General quadratic `|o + t·d|² = 1` in anisotropically-scaled space.
pub fn ray_to_globe(origin: Vec3, dir: Vec3) -> Option<GeoPoint> {
    let v = ray_to_globe_d(origin.as_dvec3(), dir.as_dvec3())?;
    globe_to_geo(v)
}

/// f64 core of [`ray_to_globe`], split out so the maths is exact and unit-closable.
fn ray_to_globe_d(origin: Vec3D, dir: Vec3D) -> Option<Vec3D> {
    let s = Vec3D::new(1.0, 1.0, 1.0 / POLAR_RATIO);
    let os = origin * s;
    let ds = dir * s;
    let a = ds.dot(ds);
    if a <= 1e-12 {
        return None;
    }
    let half_b = os.dot(ds);
    let c = os.dot(os) - 1.0;
    let disc = half_b * half_b - a * c;
    if disc <= 0.0 {
        return None;
    }
    let sq = disc.sqrt();
    let t0 = (-half_b - sq) / a;
    let t = if t0 > 0.0 { t0 } else { (-half_b + sq) / a };
    if t <= 0.0 {
        return None;
    }
    Some((os + ds * t) / s)
}

/// The geographic point where a ray crosses the flat-map plane `z == plane_z`.
/// For the orthographic top-down camera the ray is vertical so this is exact;
/// longitude is wrapped to `[-180, 180]`.
pub fn ray_to_flat(origin: Vec3, dir: Vec3, plane_z: f32) -> Option<GeoPoint> {
    if dir.z.abs() < 1e-6 {
        return None;
    }
    let t = (plane_z - origin.z) / dir.z;
    if t < 0.0 {
        return None;
    }
    let x = origin.x + dir.x * t;
    let y = origin.y + dir.y * t;
    let mut g = flat_to_geo(Vec2::new(x, y).as_dvec2());
    g.lon_deg = wrap_180(g.lon_deg);
    Some(g)
}

/// Normalise a longitude in degrees into `[-180, 180]`.
fn wrap_180(mut lon: f64) -> f64 {
    while lon > 180.0 {
        lon -= 360.0;
    }
    while lon < -180.0 {
        lon += 360.0;
    }
    lon
}

/// `Vec3` in double precision (glam's `DVec3`) — a local alias keeps the f64
/// core readable.
type Vec3D = glam::DVec3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_on_ray_hits_prime_meridian_equator() {
        // Camera on +X looking straight at the origin: the near hit is (1,0,0).
        let g = ray_to_globe(Vec3::new(3.0, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0)).unwrap();
        assert!((g.lon_deg - 0.0).abs() < 1e-6, "{g:?}");
        assert!((g.lat_deg - 0.0).abs() < 1e-6, "{g:?}");
    }

    #[test]
    fn north_pole_camera_hits_the_north_pole() {
        // +Z camera looking down: hit the north pole (lat 90).
        let g = ray_to_globe(Vec3::new(0.0, 0.0, 3.0), Vec3::new(0.0, 0.0, -1.0)).unwrap();
        assert!((g.lat_deg - 90.0).abs() < 1e-4, "{g:?}");
    }

    #[test]
    fn off_to_the_side_wraps_longitude() {
        // +Y camera looking at origin → (0,1,0) = 90°E.
        let g = ray_to_globe(Vec3::new(0.0, 3.0, 0.0), Vec3::new(0.0, -1.0, 0.0)).unwrap();
        assert!((g.lon_deg - 90.0).abs() < 1e-6, "{g:?}");
    }

    #[test]
    fn ray_missing_the_globe_is_none() {
        // Aim a kilometre off the limb.
        assert!(ray_to_globe(Vec3::new(3.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)).is_none());
    }

    #[test]
    fn flat_vertical_drop_is_exact_and_wraps() {
        // x = π ⇒ 180°E wraps to -180 (still the antimeridian).
        let g = ray_to_flat(Vec3::new(std::f32::consts::PI, 0.0, 100.0), Vec3::new(0.0, 0.0, -1.0), 0.0)
            .unwrap();
        assert!(g.lon_deg.abs() > 179.0, "antimeridian: {g:?}");
        // y = π/4 ⇒ 45°N.
        let g2 = ray_to_flat(Vec3::new(0.0, std::f32::consts::FRAC_PI_4, 100.0), Vec3::new(0.0, 0.0, -1.0), 0.0)
            .unwrap();
        assert!((g2.lat_deg - 45.0).abs() < 1e-3, "{g2:?}");
    }
}
