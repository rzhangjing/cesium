//! Geographic positions and their projection into the two render spaces the
//! viewer uses.
//!
//! [`GeoPoint`] (lon/lat in **degrees**, height in **metres** above the WGS84
//! ellipsoid) is the single source of truth for every plotted coordinate. The
//! overlay is view-agnostic: it stores `GeoPoint`s and lets the render bridge
//! project them per active view:
//!  * [`geo_to_globe`] — 3D ECEF on the WGS84 ellipsoid, scaled into the viewer's
//!    render units (unit oblate ellipsoid, Z-up, `ECEF_m / METERS_PER_RENDER_UNIT`)
//!    so it lands exactly on the tile meshes the 3D globe draws.
//!  * [`geo_to_flat`] — 2D Geographic / equirectangular world units
//!    (`x = lon_rad`, `y = lat_rad`, R = 1), matching `map2d`'s projection.
//!
//! Reuses `cesium-geospatial`'s `Cartographic`/`Ellipsoid` for the WGS84 maths so
//! there is one geodetic implementation across the workspace.

use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};

/// Metres per render unit. Duplicated here (rather than imported from the
/// `cesium-bevy-render` adapter) to keep this core crate free of the rendering
/// stack; MUST stay in lockstep with `cesium_bevy_render::METERS_PER_RENDER_UNIT`
/// and the app's `6378137.0` convention.
pub const METERS_PER_RENDER_UNIT: f64 = 6_378_137.0;

/// A geographic position: longitude/latitude in degrees, height in metres above
/// the WGS84 ellipsoid. `height` is ignored by the flat (2D) projection.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeoPoint {
    /// Longitude in degrees, east positive ([-180, 180] by convention, not enforced).
    pub lon_deg: f64,
    /// Latitude in degrees, north positive ([-90, 90]).
    pub lat_deg: f64,
    /// Height in metres above the ellipsoid (0 == on the surface).
    pub height_m: f64,
}

impl GeoPoint {
    /// Construct from degrees + metre height.
    #[inline]
    pub fn new(lon_deg: f64, lat_deg: f64, height_m: f64) -> Self {
        Self {
            lon_deg,
            lat_deg,
            height_m,
        }
    }

    /// A surface point (height = 0).
    #[inline]
    pub fn surface(lon_deg: f64, lat_deg: f64) -> Self {
        Self::new(lon_deg, lat_deg, 0.0)
    }

    /// Great-circle (spherical) distance to another point in metres, on a sphere
    /// of radius [`METERS_PER_RENDER_UNIT`]. The reserved geodesic-measure hook
    /// (plan §6) and the sampling self-checks build on this.
    #[inline]
    pub fn surface_distance(&self, other: GeoPoint) -> f64 {
        let lat1 = self.lat_deg.to_radians();
        let lat2 = other.lat_deg.to_radians();
        let dlat = lat2 - lat1;
        let dlon = (other.lon_deg - self.lon_deg).to_radians();
        let h = (dlat * 0.5).sin();
        let v = (dlon * 0.5).sin();
        let a = h * h + lat1.cos() * lat2.cos() * v * v;
        2.0 * a.sqrt().asin() * METERS_PER_RENDER_UNIT
    }
}

/// Convert degrees to radians.
#[inline]
fn to_rad(deg: f64) -> f64 {
    deg.to_radians()
}

/// Convert radians to degrees.
#[inline]
fn to_deg(rad: f64) -> f64 {
    rad.to_degrees()
}

/// Geographic → WGS84 ECEF in metres.
#[inline]
pub fn geo_to_ecef_meters(p: GeoPoint) -> DVec3 {
    let c = Cartographic::from_degrees(p.lon_deg, p.lat_deg, p.height_m);
    Ellipsoid::WGS84.cartographic_to_cartesian(&c)
}

/// Geographic → viewer render units on the globe (Z-up unit oblate ellipsoid).
/// This is the position a 3D plot vertex must occupy to coincide with the
/// rendered terrain surface at the same lon/lat.
#[inline]
pub fn geo_to_globe(p: GeoPoint) -> DVec3 {
    geo_to_ecef_meters(p) / METERS_PER_RENDER_UNIT
}

/// Inverse of [`geo_to_globe`]: render-unit globe position back to geographic.
/// Returns `None` only for the degenerate ellipsoid-centre case.
#[inline]
pub fn globe_to_geo(v: DVec3) -> Option<GeoPoint> {
    let meters = v * METERS_PER_RENDER_UNIT;
    Cartographic::from_cartesian(meters, &Ellipsoid::WGS84).map(|c| GeoPoint {
        lon_deg: to_deg(c.longitude),
        lat_deg: to_deg(c.latitude),
        height_m: c.height,
    })
}

/// Geographic → 2D flat world units (`x = lon_rad`, `y = lat_rad`, R = 1),
/// matching `map2d`'s equirectangular layout. Longitude is taken verbatim (no
/// wrap); the caller may wrap for placement near the antimeridian.
#[inline]
pub fn geo_to_flat(p: GeoPoint) -> DVec2 {
    DVec2::new(to_rad(p.lon_deg), to_rad(p.lat_deg))
}

/// Inverse of [`geo_to_flat`]: flat world units back to geographic (height 0).
#[inline]
pub fn flat_to_geo(v: DVec2) -> GeoPoint {
    GeoPoint::surface(to_deg(v.x), to_deg(v.y))
}

/// Axis-aligned geographic bounding box in degrees. `west` may exceed the
/// [-180, 180] range for boxes straddling the antimeridian; the model keeps the
/// raw span and only normalises when it needs to. Height is not tracked (2D
/// culling proxy only).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeoBounds {
    pub west_deg: f64,
    pub south_deg: f64,
    pub east_deg: f64,
    pub north_deg: f64,
}

impl GeoBounds {
    /// A degenerate box at a single point.
    #[inline]
    pub fn from_point(p: GeoPoint) -> Self {
        Self {
            west_deg: p.lon_deg,
            south_deg: p.lat_deg,
            east_deg: p.lon_deg,
            north_deg: p.lat_deg,
        }
    }

    /// Empty/inverted sentinel used as the fold seed for [`GeoBounds::union`].
    #[inline]
    pub fn empty() -> Self {
        Self {
            west_deg: f64::MAX,
            south_deg: f64::MAX,
            east_deg: f64::MIN,
            north_deg: f64::MIN,
        }
    }

    /// True when nothing has been folded in yet.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.west_deg > self.east_deg || self.south_deg > self.north_deg
    }

    /// Smallest box containing both. Union with an `empty()` box yields the
    /// other operand, so `iter().fold(empty(), union)` works.
    #[inline]
    pub fn union(self, other: Self) -> Self {
        if other.is_empty() {
            return self;
        }
        if self.is_empty() {
            return other;
        }
        Self {
            west_deg: self.west_deg.min(other.west_deg),
            south_deg: self.south_deg.min(other.south_deg),
            east_deg: self.east_deg.max(other.east_deg),
            north_deg: self.north_deg.max(other.north_deg),
        }
    }

    /// Bounds over a non-empty point cloud (first seed, then union).
    #[inline]
    pub fn from_points<'a>(pts: impl IntoIterator<Item = &'a GeoPoint>) -> Self {
        pts.into_iter()
            .fold(Self::empty(), |acc, p| acc.union(Self::from_point(*p)))
    }

    /// Longitude span in degrees (0..360).
    #[inline]
    pub fn width_deg(&self) -> f64 {
        (self.east_deg - self.west_deg).abs()
    }

    /// Latitude span in degrees (0..180).
    #[inline]
    pub fn height_deg(&self) -> f64 {
        (self.north_deg - self.south_deg).abs()
    }

    /// Whether `p` lies inside the box (inclusive on every edge). An
    /// empty / inverted box contains nothing. This is the broad-phase test the
    /// picker uses to reject a cursor that provably cannot touch an element, so
    /// it must be paired with a *conservative* box (see `Geometry::bounds`).
    #[inline]
    pub fn contains(&self, p: GeoPoint) -> bool {
        !self.is_empty()
            && p.lon_deg >= self.west_deg
            && p.lon_deg <= self.east_deg
            && p.lat_deg >= self.south_deg
            && p.lat_deg <= self.north_deg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn equator_prime_meridian_is_unit_x() {
        // (0°, 0°) → ECEF (a,0,0) → render (1,0,0); Z-up leaves north pole on +Z.
        let v = geo_to_globe(GeoPoint::surface(0.0, 0.0));
        assert!(approx(v.x, 1.0, 1e-9), "{v}");
        assert!(approx(v.y, 0.0, 1e-9), "{v}");
        assert!(approx(v.z, 0.0, 1e-9), "{v}");
    }

    #[test]
    fn north_pole_sits_on_positive_z() {
        let v = geo_to_globe(GeoPoint::surface(0.0, 90.0));
        // Polar radius b / a ≈ 0.99664719 (oblate), on +Z.
        assert!(approx(v.x, 0.0, 1e-9) && approx(v.y, 0.0, 1e-9), "{v}");
        assert!(approx(v.z, 0.99664719, 1e-6), "{v}");
        assert!(v.z > 0.0, "north pole must be +Z");
    }

    #[test]
    fn east_90_is_unit_y() {
        let v = geo_to_globe(GeoPoint::surface(90.0, 0.0));
        assert!(approx(v.x, 0.0, 1e-9), "{v}");
        assert!(approx(v.y, 1.0, 1e-9), "{v}");
    }

    #[test]
    fn globe_roundtrip_recovers_geo() {
        for (lon, lat) in [
            (0.0, 0.0),
            (116.4, 39.9),
            (-73.98, 40.7),
            (179.9, -89.0),
            (-179.9, 45.0),
        ] {
            let p = GeoPoint::new(lon, lat, 1234.5);
            let back = globe_to_geo(geo_to_globe(p)).expect("valid point");
            assert!(approx(back.lon_deg, lon, 1e-6), "lon {lon} → {}", back.lon_deg);
            assert!(approx(back.lat_deg, lat, 1e-6), "lat {lat} → {}", back.lat_deg);
            assert!(approx(back.height_m, 1234.5, 1e-2), "height {}", back.height_m);
        }
    }

    #[test]
    fn flat_uses_radians_and_inverts() {
        let p = GeoPoint::surface(180.0, 45.0);
        let f = geo_to_flat(p);
        assert!(approx(f.x, std::f64::consts::PI, 1e-9), "{f}");
        assert!(approx(f.y, std::f64::consts::FRAC_PI_4, 1e-9), "{f}");
        let back = flat_to_geo(f);
        assert!(approx(back.lon_deg, 180.0, 1e-9));
        assert!(approx(back.lat_deg, 45.0, 1e-9));
    }

    #[test]
    fn bounds_union_and_fold() {
        assert!(GeoBounds::empty().is_empty());
        let a = GeoBounds::from_point(GeoPoint::surface(10.0, 20.0));
        let b = GeoBounds::from_point(GeoPoint::surface(-30.0, 40.0));
        let u = a.union(b);
        assert_eq!((u.west_deg, u.east_deg, u.south_deg, u.north_deg), (-30.0, 10.0, 20.0, 40.0));
        // Folding a cloud matches pairwise union and ignores nothing.
        let cloud = vec![
            GeoPoint::surface(0.0, 0.0),
            GeoPoint::surface(5.0, -8.0),
            GeoPoint::surface(-2.0, 3.0),
        ];
        let cb = GeoBounds::from_points(&cloud);
        assert_eq!((cb.west_deg, cb.east_deg, cb.south_deg, cb.north_deg), (-2.0, 5.0, -8.0, 3.0));
        assert!(approx(cb.width_deg(), 7.0, 1e-12));
        assert!(approx(cb.height_deg(), 11.0, 1e-12));
        // union with empty is identity.
        assert_eq!(cb.union(GeoBounds::empty()), cb);
        // contains: inclusive edges, and an empty box contains nothing.
        assert!(cb.contains(GeoPoint::surface(-2.0, 3.0))); // corner (west,north)
        assert!(cb.contains(GeoPoint::surface(0.0, 0.0))); // interior
        assert!(!cb.contains(GeoPoint::surface(-2.1, 3.0))); // just outside west
        assert!(!cb.contains(GeoPoint::surface(5.0, 3.1))); // just outside north
        assert!(!GeoBounds::empty().contains(GeoPoint::surface(0.0, 0.0)));
    }
}
