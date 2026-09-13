//! Map projections for coordinate transformation.
//!
//! Implements various map projections:
//! - Web Mercator (EPSG:3857)
//! - UTM (Universal Transverse Mercator)
//! - Polar Stereographic
//! - Equirectangular (Plate Carrée)

use glam::DVec2;
use std::f64::consts::FRAC_PI_2;

/// A 2D projected coordinate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectedCoordinate {
    /// X coordinate (easting in meters).
    pub x: f64,
    /// Y coordinate (northing in meters).
    pub y: f64,
}

impl ProjectedCoordinate {
    /// Creates a new projected coordinate.
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Converts to DVec2.
    pub fn to_vec2(self) -> DVec2 {
        DVec2::new(self.x, self.y)
    }

    /// Creates from DVec2.
    pub fn from_vec2(v: DVec2) -> Self {
        Self { x: v.x, y: v.y }
    }
}

/// A geographic coordinate (longitude, latitude in radians).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeographicCoordinate {
    /// Longitude in radians (-π to π).
    pub longitude: f64,
    /// Latitude in radians (-π/2 to π/2).
    pub latitude: f64,
}

impl GeographicCoordinate {
    /// Creates a new geographic coordinate from radians.
    pub fn from_radians(longitude: f64, latitude: f64) -> Self {
        Self {
            longitude,
            latitude,
        }
    }

    /// Creates from degrees.
    pub fn from_degrees(longitude: f64, latitude: f64) -> Self {
        Self {
            longitude: longitude.to_radians(),
            latitude: latitude.to_radians(),
        }
    }

    /// Returns longitude in degrees.
    pub fn longitude_degrees(&self) -> f64 {
        self.longitude.to_degrees()
    }

    /// Returns latitude in degrees.
    pub fn latitude_degrees(&self) -> f64 {
        self.latitude.to_degrees()
    }
}

// ============================================================================
// Web Mercator (EPSG:3857)
// ============================================================================

/// Web Mercator projection (EPSG:3857).
///
/// The standard projection used by most web mapping services.
/// Maps longitude/latitude to meters on a square tile grid.
#[derive(Debug, Clone, Copy)]
pub struct WebMercator {
    /// Earth radius used for projection (meters).
    pub radius: f64,
}

impl Default for WebMercator {
    fn default() -> Self {
        Self {
            radius: 6378137.0, // WGS84 semi-major axis
        }
    }
}

impl WebMercator {
    /// Creates a Web Mercator projection with the given radius.
    pub fn new(radius: f64) -> Self {
        Self { radius }
    }

    /// Maximum latitude that can be projected (in radians).
    pub const MAX_LATITUDE: f64 = 1.4844222297453324; // ~85.0511 degrees

    /// Projects a geographic coordinate to Web Mercator.
    ///
    /// # Arguments
    /// * `geo` - Geographic coordinate (longitude, latitude in radians)
    ///
    /// # Returns
    /// Projected coordinate (x, y in meters)
    pub fn project(&self, geo: &GeographicCoordinate) -> Option<ProjectedCoordinate> {
        if geo.latitude.abs() > Self::MAX_LATITUDE {
            return None;
        }

        let x = self.radius * geo.longitude;
        // Standard Web Mercator formula: y = R * ln(tan(π/4 + lat/2))
        let y = self.radius * (std::f64::consts::FRAC_PI_4 + geo.latitude / 2.0).tan().ln();

        Some(ProjectedCoordinate::new(x, y))
    }

    /// Unprojects a Web Mercator coordinate to geographic.
    ///
    /// # Arguments
    /// * `proj` - Projected coordinate (x, y in meters)
    ///
    /// # Returns
    /// Geographic coordinate (longitude, latitude in radians)
    pub fn unproject(&self, proj: &ProjectedCoordinate) -> GeographicCoordinate {
        let longitude = proj.x / self.radius;
        // Inverse: lat = 2 * atan(exp(y / R)) - π/2
        let latitude = 2.0 * (proj.y / self.radius).exp().atan() - FRAC_PI_2;

        GeographicCoordinate::from_radians(longitude, latitude)
    }

    /// Projects from degrees.
    pub fn project_degrees(&self, lon: f64, lat: f64) -> Option<ProjectedCoordinate> {
        self.project(&GeographicCoordinate::from_degrees(lon, lat))
    }

    /// Unprojects to degrees.
    pub fn unproject_to_degrees(&self, proj: &ProjectedCoordinate) -> (f64, f64) {
        let geo = self.unproject(proj);
        (geo.longitude_degrees(), geo.latitude_degrees())
    }
}

// ============================================================================
// UTM (Universal Transverse Mercator)
// ============================================================================

/// UTM zone information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtmZone {
    /// Zone number (1-60).
    pub zone: u32,
    /// Whether in the northern hemisphere.
    pub north: bool,
}

impl UtmZone {
    /// Computes the UTM zone for a given longitude/latitude (in degrees).
    pub fn from_lon_lat(lon_deg: f64, lat_deg: f64) -> Self {
        let zone = ((lon_deg + 180.0) / 6.0).floor() as u32 + 1;
        let zone = zone.clamp(1, 60);
        let north = lat_deg >= 0.0;

        Self { zone, north }
    }

    /// Returns the central meridian of the zone (in radians).
    pub fn central_meridian(&self) -> f64 {
        ((self.zone as f64 - 1.0) * 6.0 - 180.0 + 3.0).to_radians()
    }

    /// Returns the EPSG code for this zone.
    pub fn epsg_code(&self) -> u32 {
        if self.north {
            32600 + self.zone
        } else {
            32700 + self.zone
        }
    }
}

/// Universal Transverse Mercator projection.
///
/// Divides the Earth into 60 zones, each 6 degrees of longitude wide.
#[derive(Debug, Clone)]
pub struct Utm {
    /// Earth semi-major axis (meters).
    pub semi_major_axis: f64,
    /// Earth flattening.
    pub flattening: f64,
    /// Scale factor at central meridian.
    pub scale_factor: f64,
    /// False easting (meters).
    pub false_easting: f64,
    /// False northing for southern hemisphere (meters).
    pub false_northing_south: f64,
}

impl Default for Utm {
    fn default() -> Self {
        Self {
            semi_major_axis: 6378137.0,           // WGS84
            flattening: 1.0 / 298.257223563,      // WGS84
            scale_factor: 0.9996,
            false_easting: 500000.0,
            false_northing_south: 10000000.0,
        }
    }
}

impl Utm {
    /// Projects a geographic coordinate to UTM.
    ///
    /// # Arguments
    /// * `geo` - Geographic coordinate (longitude, latitude in radians)
    /// * `zone` - Target UTM zone (if None, auto-computed)
    ///
    /// # Returns
    /// Projected coordinate and the zone used
    pub fn project(
        &self,
        geo: &GeographicCoordinate,
        zone: Option<u32>,
    ) -> (ProjectedCoordinate, UtmZone) {
        let lon_deg = geo.longitude_degrees();
        let lat_deg = geo.latitude_degrees();

        let utm_zone = match zone {
            Some(z) => UtmZone {
                zone: z,
                north: lat_deg >= 0.0,
            },
            None => UtmZone::from_lon_lat(lon_deg, lat_deg),
        };

        let lambda_0 = utm_zone.central_meridian();
        let phi = geo.latitude;
        let lambda = geo.longitude;

        // Eccentricity squared
        let e2 = 2.0 * self.flattening - self.flattening * self.flattening;
        let e4 = e2 * e2;
        let e6 = e4 * e2;

        // Meridional arc length
        let n = self.semi_major_axis / (1.0 - e2 * phi.sin() * phi.sin()).sqrt();
        let t = phi.tan();
        let c = e2 / (1.0 - e2) * phi.cos() * phi.cos();
        let a = phi.cos() * (lambda - lambda_0);

        let a2 = a * a;
        let a3 = a2 * a;
        let a4 = a3 * a;
        let a5 = a4 * a;
        let a6 = a5 * a;

        // Transverse Mercator formulas
        let x = self.scale_factor * n * (a + (1.0 - t * t + c) * a3 / 6.0
            + (5.0 - 18.0 * t * t + t * t * t * t + 72.0 * c - 58.0 * e2 / (1.0 - e2)) * a5 / 120.0);

        // Meridional arc
        let m = self.semi_major_axis * (
            (1.0 - e2 / 4.0 - 3.0 * e4 / 64.0 - 5.0 * e6 / 256.0) * phi
            - (3.0 * e2 / 8.0 + 3.0 * e4 / 32.0 + 45.0 * e6 / 1024.0) * (2.0 * phi).sin()
            + (15.0 * e4 / 256.0 + 45.0 * e6 / 1024.0) * (4.0 * phi).sin()
            - (35.0 * e6 / 3072.0) * (6.0 * phi).sin()
        );

        let y = self.scale_factor * (m + n * t * (a2 / 2.0
            + (5.0 - t * t + 9.0 * c + 4.0 * c * c) * a4 / 24.0
            + (61.0 - 58.0 * t * t + t * t * t * t + 600.0 * c - 330.0 * e2 / (1.0 - e2)) * a6 / 720.0));

        let easting = x + self.false_easting;
        let northing = if utm_zone.north {
            y
        } else {
            y + self.false_northing_south
        };

        (ProjectedCoordinate::new(easting, northing), utm_zone)
    }

    /// Unprojects a UTM coordinate to geographic.
    ///
    /// # Arguments
    /// * `proj` - Projected coordinate (easting, northing in meters)
    /// * `zone` - UTM zone information
    ///
    /// # Returns
    /// Geographic coordinate (longitude, latitude in radians)
    pub fn unproject(&self, proj: &ProjectedCoordinate, zone: &UtmZone) -> GeographicCoordinate {
        let x = proj.x - self.false_easting;
        let y = if zone.north {
            proj.y
        } else {
            proj.y - self.false_northing_south
        };

        let e2 = 2.0 * self.flattening - self.flattening * self.flattening;
        let e4 = e2 * e2;
        let e6 = e4 * e2;
        let ep2 = e2 / (1.0 - e2);

        // Meridional arc at equator
        let m0 = 0.0; // Equator
        let m = m0 + y / self.scale_factor;

        let mu = m / (self.semi_major_axis * (1.0 - e2 / 4.0 - 3.0 * e4 / 64.0 - 5.0 * e6 / 256.0));

        let e1 = (1.0 - (1.0 - e2).sqrt()) / (1.0 + (1.0 - e2).sqrt());

        let phi1 = mu + (3.0 * e1 / 2.0 - 27.0 * e1.powi(3) / 32.0) * (2.0 * mu).sin()
            + (21.0 * e1 * e1 / 16.0 - 55.0 * e1.powi(4) / 32.0) * (4.0 * mu).sin()
            + (151.0 * e1.powi(3) / 96.0) * (6.0 * mu).sin()
            + (1097.0 * e1.powi(4) / 512.0) * (8.0 * mu).sin();

        let n1 = self.semi_major_axis / (1.0 - e2 * phi1.sin() * phi1.sin()).sqrt();
        let t1 = phi1.tan();
        let c1 = ep2 * phi1.cos() * phi1.cos();
        let r1 = self.semi_major_axis * (1.0 - e2) / (1.0 - e2 * phi1.sin() * phi1.sin()).powf(1.5);
        let d = x / (n1 * self.scale_factor);

        let d2 = d * d;
        let d3 = d2 * d;
        let d4 = d3 * d;
        let d5 = d4 * d;
        let d6 = d5 * d;

        let latitude = phi1 - (n1 * t1 / r1) * (d2 / 2.0
            - (5.0 + 3.0 * t1 * t1 + 10.0 * c1 - 4.0 * c1 * c1 - 9.0 * ep2) * d4 / 24.0
            + (61.0 + 90.0 * t1 * t1 + 298.0 * c1 + 45.0 * t1 * t1 * t1 * t1 - 252.0 * ep2 - 3.0 * c1 * c1) * d6 / 720.0);

        let longitude = zone.central_meridian() + (d - (1.0 + 2.0 * t1 * t1 + c1) * d3 / 6.0
            + (5.0 - 2.0 * c1 + 28.0 * t1 * t1 - 3.0 * c1 * c1 + 8.0 * ep2 + 24.0 * t1 * t1 * t1 * t1) * d5 / 120.0) / phi1.cos();

        GeographicCoordinate::from_radians(longitude, latitude)
    }
}

// ============================================================================
// Polar Stereographic
// ============================================================================

/// Polar Stereographic projection.
///
/// Used for polar regions (above 84°N or below 80°S).
#[derive(Debug, Clone, Copy)]
pub struct PolarStereographic {
    /// Earth semi-major axis (meters).
    pub semi_major_axis: f64,
    /// Earth flattening.
    pub flattening: f64,
    /// True scale latitude (radians, usually 71° for Arctic, 71° for Antarctic).
    pub standard_parallel: f64,
    /// Whether projecting the north pole (true) or south pole (false).
    pub north_pole: bool,
}

impl Default for PolarStereographic {
    fn default() -> Self {
        Self {
            semi_major_axis: 6378137.0,
            flattening: 1.0 / 298.257223563,
            standard_parallel: 71.0_f64.to_radians(),
            north_pole: true,
        }
    }
}

impl PolarStereographic {
    /// Projects a geographic coordinate to Polar Stereographic.
    pub fn project(&self, geo: &GeographicCoordinate) -> ProjectedCoordinate {
        let e = (2.0 * self.flattening - self.flattening * self.flattening).sqrt();

        let phi = if self.north_pole {
            geo.latitude
        } else {
            -geo.latitude
        };

        let lambda = geo.longitude;

        // Conformal latitude
        let t = ((FRAC_PI_2 - phi) / 2.0).tan() /
            ((1.0 - e * phi.sin()) / (1.0 + e * phi.sin())).powf(e / 2.0);

        // Scale factor at standard parallel
        let phi_c = self.standard_parallel;
        let t_c = ((FRAC_PI_2 - phi_c) / 2.0).tan() /
            ((1.0 - e * phi_c.sin()) / (1.0 + e * phi_c.sin())).powf(e / 2.0);

        let m_c = phi_c.cos() / (1.0 - e * e * phi_c.sin() * phi_c.sin()).sqrt();
        let rho = self.semi_major_axis * m_c * t / t_c;

        let x = rho * lambda.sin();
        let y = if self.north_pole {
            -rho * lambda.cos()
        } else {
            rho * lambda.cos()
        };

        ProjectedCoordinate::new(x, y)
    }

    /// Unprojects a Polar Stereographic coordinate to geographic.
    pub fn unproject(&self, proj: &ProjectedCoordinate) -> GeographicCoordinate {
        let e = (2.0 * self.flattening - self.flattening * self.flattening).sqrt();

        let phi_c = self.standard_parallel;
        let t_c = ((FRAC_PI_2 - phi_c) / 2.0).tan() /
            ((1.0 - e * phi_c.sin()) / (1.0 + e * phi_c.sin())).powf(e / 2.0);
        let m_c = phi_c.cos() / (1.0 - e * e * phi_c.sin() * phi_c.sin()).sqrt();

        let x = proj.x;
        let y = if self.north_pole { -proj.y } else { proj.y };

        let rho = (x * x + y * y).sqrt();
        let t = rho * t_c / (self.semi_major_axis * m_c);

        // Iterative solution for latitude
        let mut phi = FRAC_PI_2 - 2.0 * t.atan();
        for _ in 0..10 {
            let e_sin_phi = e * phi.sin();
            let new_phi = FRAC_PI_2 - 2.0 * (t * ((1.0 - e_sin_phi) / (1.0 + e_sin_phi)).powf(e / 2.0)).atan();
            if (new_phi - phi).abs() < 1e-12 {
                break;
            }
            phi = new_phi;
        }

        let latitude = if self.north_pole { phi } else { -phi };
        let longitude = y.atan2(x);

        GeographicCoordinate::from_radians(longitude, latitude)
    }
}

// ============================================================================
// Equirectangular (Plate Carrée)
// ============================================================================

/// Equirectangular projection (Plate Carrée, EPSG:4326-like).
///
/// Simple projection where longitude maps to X and latitude maps to Y.
#[derive(Debug, Clone, Copy)]
pub struct Equirectangular {
    /// Earth radius (meters).
    pub radius: f64,
}

impl Default for Equirectangular {
    fn default() -> Self {
        Self { radius: 6378137.0 }
    }
}

impl Equirectangular {
    /// Projects a geographic coordinate to Equirectangular.
    pub fn project(&self, geo: &GeographicCoordinate) -> ProjectedCoordinate {
        let x = self.radius * geo.longitude;
        let y = self.radius * geo.latitude;
        ProjectedCoordinate::new(x, y)
    }

    /// Unprojects an Equirectangular coordinate to geographic.
    pub fn unproject(&self, proj: &ProjectedCoordinate) -> GeographicCoordinate {
        let longitude = proj.x / self.radius;
        let latitude = proj.y / self.radius;
        GeographicCoordinate::from_radians(longitude, latitude)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 1e-6;

    #[test]
    fn test_web_mercator_origin() {
        let mercator = WebMercator::default();
        let geo = GeographicCoordinate::from_radians(0.0, 0.0);

        let proj = mercator.project(&geo).unwrap();
        assert!((proj.x).abs() < TOLERANCE);
        assert!((proj.y).abs() < TOLERANCE);
    }

    #[test]
    fn test_web_mercator_roundtrip() {
        let mercator = WebMercator::default();
        let original = GeographicCoordinate::from_degrees(-73.9857, 40.7484); // NYC

        let proj = mercator.project(&original).unwrap();
        let recovered = mercator.unproject(&proj);

        assert!((recovered.longitude - original.longitude).abs() < TOLERANCE);
        assert!((recovered.latitude - original.latitude).abs() < TOLERANCE);
    }

    #[test]
    fn test_web_mercator_max_latitude() {
        let mercator = WebMercator::default();

        // At max latitude
        let geo = GeographicCoordinate::from_radians(0.0, WebMercator::MAX_LATITUDE);
        assert!(mercator.project(&geo).is_some());

        // Beyond max latitude
        let geo = GeographicCoordinate::from_radians(0.0, WebMercator::MAX_LATITUDE + 0.1);
        assert!(mercator.project(&geo).is_none());
    }

    #[test]
    fn test_web_mercator_known_values() {
        let mercator = WebMercator::default();

        // London (0°, 51.5°)
        let proj = mercator.project_degrees(0.0, 51.5).unwrap();
        assert!((proj.x).abs() < 1.0); // Near prime meridian
        // y at 51.5° should be positive and in the millions of meters
        assert!(proj.y > 6_000_000.0 && proj.y < 7_000_000.0);
    }

    #[test]
    fn test_utm_zone_calculation() {
        // New York: ~74°W, 40°N → Zone 18N
        let zone = UtmZone::from_lon_lat(-73.9857, 40.7484);
        assert_eq!(zone.zone, 18);
        assert!(zone.north);

        // Sydney: ~151°E, 33°S → Zone 56S
        let zone = UtmZone::from_lon_lat(151.2093, -33.8688);
        assert_eq!(zone.zone, 56);
        assert!(!zone.north);
    }

    #[test]
    fn test_utm_zone_epsg() {
        let zone_north = UtmZone { zone: 33, north: true };
        assert_eq!(zone_north.epsg_code(), 32633);

        let zone_south = UtmZone { zone: 33, north: false };
        assert_eq!(zone_south.epsg_code(), 32733);
    }

    #[test]
    fn test_utm_project_roundtrip() {
        let utm = Utm::default();
        let original = GeographicCoordinate::from_degrees(-73.9857, 40.7484); // NYC

        let (proj, zone) = utm.project(&original, None);
        let recovered = utm.unproject(&proj, &zone);

        assert!((recovered.longitude - original.longitude).abs() < TOLERANCE);
        assert!((recovered.latitude - original.latitude).abs() < TOLERANCE);
    }

    #[test]
    fn test_utm_false_easting() {
        let utm = Utm::default();
        // Point at zone 31 central meridian (3°E), equator
        let geo = GeographicCoordinate::from_degrees(3.0, 0.0);

        let (proj, _zone) = utm.project(&geo, Some(31));

        // At the central meridian, easting should be exactly 500000m (false easting)
        assert!((proj.x - 500000.0).abs() < 1.0);
        // At the equator, northing should be ~0
        assert!(proj.y.abs() < 1.0);
    }

    #[test]
    fn test_polar_stereographic_origin() {
        let proj = PolarStereographic::default();
        let geo = GeographicCoordinate::from_radians(0.0, FRAC_PI_2); // North pole

        let projected = proj.project(&geo);
        assert!(projected.x.abs() < 1.0);
        assert!(projected.y.abs() < 1.0);
    }

    #[test]
    fn test_polar_stereographic_roundtrip() {
        let proj = PolarStereographic::default();
        let original = GeographicCoordinate::from_degrees(45.0, 85.0); // Near north pole

        let projected = proj.project(&original);
        let recovered = proj.unproject(&projected);

        assert!((recovered.longitude - original.longitude).abs() < 0.01_f64.to_radians());
        assert!((recovered.latitude - original.latitude).abs() < 0.01_f64.to_radians());
    }

    #[test]
    fn test_equirectangular_origin() {
        let proj = Equirectangular::default();
        let geo = GeographicCoordinate::from_radians(0.0, 0.0);

        let projected = proj.project(&geo);
        assert!((projected.x).abs() < TOLERANCE);
        assert!((projected.y).abs() < TOLERANCE);
    }

    #[test]
    fn test_equirectangular_roundtrip() {
        let proj = Equirectangular::default();
        let original = GeographicCoordinate::from_degrees(120.0, 30.0);

        let projected = proj.project(&original);
        let recovered = proj.unproject(&projected);

        assert!((recovered.longitude - original.longitude).abs() < TOLERANCE);
        assert!((recovered.latitude - original.latitude).abs() < TOLERANCE);
    }

    #[test]
    fn test_geographic_coordinate_from_degrees() {
        let geo = GeographicCoordinate::from_degrees(180.0, 90.0);
        assert!((geo.longitude - std::f64::consts::PI).abs() < TOLERANCE);
        assert!((geo.latitude - FRAC_PI_2).abs() < TOLERANCE);
    }

    #[test]
    fn test_projected_coordinate_vec2() {
        let proj = ProjectedCoordinate::new(100.0, 200.0);
        let vec = proj.to_vec2();
        assert!((vec.x - 100.0).abs() < TOLERANCE);
        assert!((vec.y - 200.0).abs() < TOLERANCE);

        let back = ProjectedCoordinate::from_vec2(vec);
        assert_eq!(back, proj);
    }

    #[test]
    fn test_utm_zone_central_meridian() {
        let zone = UtmZone { zone: 1, north: true };
        // Zone 1: -180 to -174, central meridian = -177°
        let cm = zone.central_meridian().to_degrees();
        assert!((cm - (-177.0)).abs() < TOLERANCE);

        let zone = UtmZone { zone: 31, north: true };
        // Zone 31: 0 to 6, central meridian = 3°
        let cm = zone.central_meridian().to_degrees();
        assert!((cm - 3.0).abs() < TOLERANCE);
    }
}
