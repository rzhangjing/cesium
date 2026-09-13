//! Datum transformations between geodetic reference systems.
//!
//! Implements coordinate transformations between different geodetic datums:
//! - WGS84 (World Geodetic System 1984)
//! - CGCS2000 (China Geodetic Coordinate System 2000)
//! - ITRF (International Terrestrial Reference Frame)
//! - NAD83 (North American Datum 1983)
//! - GRS80 (Geodetic Reference System 1980)

use glam::{DMat3, DVec3};
use std::f64::consts::PI;

/// A geodetic datum definition.
#[derive(Debug, Clone, Copy)]
pub struct Datum {
    /// Datum name.
    pub name: &'static str,
    /// Semi-major axis (meters).
    pub semi_major_axis: f64,
    /// Inverse flattening (1/f).
    pub inverse_flattening: f64,
}

impl Datum {
    /// WGS84 datum.
    pub const WGS84: Self = Self {
        name: "WGS84",
        semi_major_axis: 6378137.0,
        inverse_flattening: 298.257223563,
    };

    /// CGCS2000 datum (China).
    pub const CGCS2000: Self = Self {
        name: "CGCS2000",
        semi_major_axis: 6378137.0,
        inverse_flattening: 298.257222101,
    };

    /// GRS80 datum.
    pub const GRS80: Self = Self {
        name: "GRS80",
        semi_major_axis: 6378137.0,
        inverse_flattening: 298.257222101,
    };

    /// ITRF2014 datum.
    pub const ITRF2014: Self = Self {
        name: "ITRF2014",
        semi_major_axis: 6378137.0,
        inverse_flattening: 298.257222101,
    };

    /// NAD83 datum (North America).
    pub const NAD83: Self = Self {
        name: "NAD83",
        semi_major_axis: 6378137.0,
        inverse_flattening: 298.257222101,
    };

    /// International 1924 (Hayford) datum.
    pub const INTERNATIONAL_1924: Self = Self {
        name: "International 1924",
        semi_major_axis: 6378388.0,
        inverse_flattening: 297.0,
    };

    /// Airy 1830 datum (UK).
    pub const AIRY_1830: Self = Self {
        name: "Airy 1830",
        semi_major_axis: 6377563.396,
        inverse_flattening: 299.3249646,
    };

    /// Returns the flattening factor (f = 1 / inverse_flattening).
    pub fn flattening(&self) -> f64 {
        1.0 / self.inverse_flattening
    }

    /// Returns the semi-minor axis (b = a * (1 - f)).
    pub fn semi_minor_axis(&self) -> f64 {
        self.semi_major_axis * (1.0 - self.flattening())
    }

    /// Returns the first eccentricity squared (e² = 2f - f²).
    pub fn eccentricity_squared(&self) -> f64 {
        let f = self.flattening();
        2.0 * f - f * f
    }

    /// Returns the radii as a DVec3 (a, a, b).
    pub fn radii(&self) -> DVec3 {
        DVec3::new(self.semi_major_axis, self.semi_major_axis, self.semi_minor_axis())
    }
}

/// 7-parameter Helmert transformation (Bursa-Wolf model).
///
/// Transforms coordinates from one datum to another using:
/// - 3 translations (dx, dy, dz)
/// - 3 rotations (rx, ry, rz) in arcseconds
/// - 1 scale factor (ds) in ppm
#[derive(Debug, Clone, Copy)]
pub struct HelmertTransform {
    /// Translation in X (meters).
    pub dx: f64,
    /// Translation in Y (meters).
    pub dy: f64,
    /// Translation in Z (meters).
    pub dz: f64,
    /// Rotation around X (arcseconds).
    pub rx: f64,
    /// Rotation around Y (arcseconds).
    pub ry: f64,
    /// Rotation around Z (arcseconds).
    pub rz: f64,
    /// Scale difference (ppm, parts per million).
    pub ds: f64,
}

impl HelmertTransform {
    /// Identity transformation (no change).
    pub const IDENTITY: Self = Self {
        dx: 0.0, dy: 0.0, dz: 0.0,
        rx: 0.0, ry: 0.0, rz: 0.0,
        ds: 0.0,
    };

    /// WGS84 to CGCS2000 transformation (essentially identity for most purposes).
    pub const WGS84_TO_CGCS2000: Self = Self {
        dx: 0.0, dy: 0.0, dz: 0.0,
        rx: 0.0, ry: 0.0, rz: 0.0,
        ds: 0.0,
    };

    /// WGS84 to ITRF2014 transformation.
    pub const WGS84_TO_ITRF2014: Self = Self {
        dx: 0.0, dy: 0.0, dz: 0.0,
        rx: 0.0, ry: 0.0, rz: 0.0,
        ds: 0.0,
    };

    /// WGS84 to NAD83 transformation.
    pub const WGS84_TO_NAD83: Self = Self {
        dx: 1.004, dy: -1.910, dz: -0.515,
        rx: 0.0267, ry: 0.00034, rz: 0.011,
        ds: -0.0015,
    };

    /// ED50 to WGS84 transformation (Europe).
    pub const ED50_TO_WGS84: Self = Self {
        dx: -87.0, dy: -98.0, dz: -121.0,
        rx: 0.0, ry: 0.0, rz: 0.0,
        ds: 0.0,
    };

    /// Tokyo Datum to WGS84 transformation (Japan).
    pub const TOKYO_TO_WGS84: Self = Self {
        dx: -146.414, dy: 507.337, dz: 680.507,
        rx: 0.0, ry: 0.0, rz: 0.0,
        ds: 0.0,
    };

    /// Applies the transformation to ECEF coordinates.
    ///
    /// Uses the Bursa-Wolf formula:
    /// X_target = dx + (1 + ds*1e-6) * (X + rz*Y - ry*Z)
    /// Y_target = dy + (1 + ds*1e-6) * (-rz*X + Y + rx*Z)
    /// Z_target = dz + (1 + ds*1e-6) * (ry*X - rx*Y + Z)
    ///
    /// # Arguments
    /// * `ecef` - ECEF coordinates in the source datum
    ///
    /// # Returns
    /// ECEF coordinates in the target datum
    pub fn apply(&self, ecef: DVec3) -> DVec3 {
        // Convert arcseconds to radians
        let arcsec_to_rad = PI / (180.0 * 3600.0);
        let rx_rad = self.rx * arcsec_to_rad;
        let ry_rad = self.ry * arcsec_to_rad;
        let rz_rad = self.rz * arcsec_to_rad;

        // Scale factor (ppm to dimensionless)
        let scale = 1.0 + self.ds * 1e-6;

        // Build rotation matrix
        let rotation = DMat3::from_cols_array(&[
            1.0, rx_rad, -ry_rad,
            -rx_rad, 1.0, rz_rad,
            ry_rad, -rz_rad, 1.0,
        ]);

        // Apply transformation
        let translated = DVec3::new(self.dx, self.dy, self.dz);
        translated + scale * (rotation * ecef)
    }

    /// Returns the inverse transformation.
    pub fn inverse(&self) -> Self {
        Self {
            dx: -self.dx,
            dy: -self.dy,
            dz: -self.dz,
            rx: -self.rx,
            ry: -self.ry,
            rz: -self.rz,
            ds: -self.ds,
        }
    }
}

/// Molodensky transformation (3-parameter datum shift).
///
/// A simplified transformation that only uses translations (dx, dy, dz).
/// Suitable for low-accuracy applications (< 10m).
#[derive(Debug, Clone, Copy)]
pub struct MolodenskyTransform {
    /// Translation in X (meters).
    pub dx: f64,
    /// Translation in Y (meters).
    pub dy: f64,
    /// Translation in Z (meters).
    pub dz: f64,
}

impl MolodenskyTransform {
    /// Applies the Molodensky transformation to ECEF coordinates.
    ///
    /// # Arguments
    /// * `ecef` - ECEF coordinates in the source datum
    ///
    /// # Returns
    /// ECEF coordinates in the target datum
    pub fn apply(&self, ecef: DVec3) -> DVec3 {
        ecef + DVec3::new(self.dx, self.dy, self.dz)
    }
}

/// Converts between geographic (lat/lon/h) and ECEF coordinates for a given datum.
#[derive(Debug, Clone, Copy)]
pub struct DatumConverter {
    /// The datum to use.
    pub datum: Datum,
}

impl DatumConverter {
    /// Creates a new datum converter.
    pub fn new(datum: Datum) -> Self {
        Self { datum }
    }

    /// Converts geographic coordinates (lon, lat in radians, height in meters) to ECEF.
    pub fn geographic_to_ecef(&self, lon: f64, lat: f64, height: f64) -> DVec3 {
        let a = self.datum.semi_major_axis;
        let e2 = self.datum.eccentricity_squared();

        let sin_lat = lat.sin();
        let cos_lat = lat.cos();
        let sin_lon = lon.sin();
        let cos_lon = lon.cos();

        let n = a / (1.0 - e2 * sin_lat * sin_lat).sqrt();

        let x = (n + height) * cos_lat * cos_lon;
        let y = (n + height) * cos_lat * sin_lon;
        let z = (n * (1.0 - e2) + height) * sin_lat;

        DVec3::new(x, y, z)
    }

    /// Converts ECEF coordinates to geographic (lon, lat in radians, height in meters).
    ///
    /// Uses iterative method for latitude calculation.
    pub fn ecef_to_geographic(&self, ecef: DVec3) -> (f64, f64, f64) {
        let a = self.datum.semi_major_axis;
        let e2 = self.datum.eccentricity_squared();

        let x = ecef.x;
        let y = ecef.y;
        let z = ecef.z;

        let lon = y.atan2(x);

        let p = (x * x + y * y).sqrt();

        // Iterative calculation for latitude
        let mut lat = z.atan2(p * (1.0 - e2));
        let mut n = a;

        for _ in 0..10 {
            let sin_lat = lat.sin();
            n = a / (1.0 - e2 * sin_lat * sin_lat).sqrt();
            let new_lat = (z + e2 * n * sin_lat).atan2(p);
            if (new_lat - lat).abs() < 1e-12 {
                break;
            }
            lat = new_lat;
        }

        let sin_lat = lat.sin();
        let cos_lat = lat.cos();

        let height = if cos_lat.abs() > 1e-10 {
            p / cos_lat - n
        } else {
            z.abs() / sin_lat.abs() - n * (1.0 - e2)
        };

        (lon, lat, height)
    }
}

/// Gets the Helmert transformation between two datums.
///
/// Returns None if no predefined transformation exists.
pub fn get_helmert_transform(from: &Datum, to: &Datum) -> Option<HelmertTransform> {
    // Check for predefined transformations
    if from.name == "WGS84" && to.name == "CGCS2000" {
        return Some(HelmertTransform::WGS84_TO_CGCS2000);
    }
    if from.name == "CGCS2000" && to.name == "WGS84" {
        return Some(HelmertTransform::WGS84_TO_CGCS2000.inverse());
    }
    if from.name == "WGS84" && to.name == "ITRF2014" {
        return Some(HelmertTransform::WGS84_TO_ITRF2014);
    }
    if from.name == "ITRF2014" && to.name == "WGS84" {
        return Some(HelmertTransform::WGS84_TO_ITRF2014.inverse());
    }
    if from.name == "WGS84" && to.name == "NAD83" {
        return Some(HelmertTransform::WGS84_TO_NAD83);
    }
    if from.name == "NAD83" && to.name == "WGS84" {
        return Some(HelmertTransform::WGS84_TO_NAD83.inverse());
    }
    if from.name == "ED50" && to.name == "WGS84" {
        return Some(HelmertTransform::ED50_TO_WGS84);
    }
    if from.name == "WGS84" && to.name == "ED50" {
        return Some(HelmertTransform::ED50_TO_WGS84.inverse());
    }

    None
}

/// Transforms ECEF coordinates from one datum to another.
///
/// Uses Helmert transformation if available, otherwise returns identity.
pub fn transform_ecef(ecef: DVec3, from: &Datum, to: &Datum) -> DVec3 {
    if from.name == to.name {
        return ecef;
    }

    match get_helmert_transform(from, to) {
        Some(transform) => transform.apply(ecef),
        None => ecef, // Fallback: no transformation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 1e-6;

    #[test]
    fn test_datum_wgs84() {
        let datum = Datum::WGS84;
        assert_eq!(datum.semi_major_axis, 6378137.0);
        assert!((datum.flattening() - 1.0 / 298.257223563).abs() < 1e-15);
    }

    #[test]
    fn test_datum_cgcs2000() {
        let datum = Datum::CGCS2000;
        assert_eq!(datum.semi_major_axis, 6378137.0);
        // CGCS2000 has slightly different flattening
        assert!((datum.inverse_flattening - 298.257222101).abs() < 1e-10);
    }

    #[test]
    fn test_datum_semi_minor_axis() {
        let datum = Datum::WGS84;
        let b = datum.semi_minor_axis();
        assert!((b - 6356752.314245).abs() < 1.0);
    }

    #[test]
    fn test_helmert_identity() {
        let transform = HelmertTransform::IDENTITY;
        let ecef = DVec3::new(6378137.0, 0.0, 0.0);

        let result = transform.apply(ecef);
        assert!((result - ecef).length() < 1e-6);
    }

    #[test]
    fn test_helmert_inverse() {
        let transform = HelmertTransform::WGS84_TO_NAD83;
        let inverse = transform.inverse();

        let ecef = DVec3::new(6378137.0, 0.0, 0.0);
        let transformed = transform.apply(ecef);
        let recovered = inverse.apply(transformed);

        assert!((recovered - ecef).length() < 0.01);
    }

    #[test]
    fn test_helmert_wgs84_to_cgcs2000() {
        // WGS84 to CGCS2000 is essentially identity
        let transform = HelmertTransform::WGS84_TO_CGCS2000;
        let ecef = DVec3::new(6378137.0, 0.0, 0.0);

        let result = transform.apply(ecef);
        assert!((result - ecef).length() < 1e-6);
    }

    #[test]
    fn test_helmert_translation() {
        let transform = HelmertTransform {
            dx: 10.0, dy: 20.0, dz: 30.0,
            rx: 0.0, ry: 0.0, rz: 0.0,
            ds: 0.0,
        };

        let ecef = DVec3::new(0.0, 0.0, 0.0);
        let result = transform.apply(ecef);

        assert!((result.x - 10.0).abs() < TOLERANCE);
        assert!((result.y - 20.0).abs() < TOLERANCE);
        assert!((result.z - 30.0).abs() < TOLERANCE);
    }

    #[test]
    fn test_molodensky_transform() {
        let transform = MolodenskyTransform {
            dx: 10.0, dy: 20.0, dz: 30.0,
        };

        let ecef = DVec3::new(1000.0, 2000.0, 3000.0);
        let result = transform.apply(ecef);

        assert!((result.x - 1010.0).abs() < TOLERANCE);
        assert!((result.y - 2020.0).abs() < TOLERANCE);
        assert!((result.z - 3030.0).abs() < TOLERANCE);
    }

    #[test]
    fn test_datum_converter_ecef_roundtrip() {
        let converter = DatumConverter::new(Datum::WGS84);

        let lon = 2.0_f64.to_radians(); // 2°E
        let lat = 49.0_f64.to_radians(); // 49°N
        let height = 100.0;

        let ecef = converter.geographic_to_ecef(lon, lat, height);
        let (lon2, lat2, height2) = converter.ecef_to_geographic(ecef);

        assert!((lon2 - lon).abs() < 1e-10);
        assert!((lat2 - lat).abs() < 1e-10);
        assert!((height2 - height).abs() < 0.01);
    }

    #[test]
    fn test_datum_converter_equator() {
        let converter = DatumConverter::new(Datum::WGS84);

        let ecef = converter.geographic_to_ecef(0.0, 0.0, 0.0);
        assert!((ecef.x - 6378137.0).abs() < 1.0);
        assert!(ecef.y.abs() < 1.0);
        assert!(ecef.z.abs() < 1.0);
    }

    #[test]
    fn test_datum_converter_pole() {
        let converter = DatumConverter::new(Datum::WGS84);
        let pi_2 = std::f64::consts::FRAC_PI_2;

        let ecef = converter.geographic_to_ecef(0.0, pi_2, 0.0);
        assert!(ecef.x.abs() < 1.0);
        assert!(ecef.y.abs() < 1.0);
        assert!((ecef.z - Datum::WGS84.semi_minor_axis()).abs() < 1.0);
    }

    #[test]
    fn test_transform_ecef_same_datum() {
        let ecef = DVec3::new(1000.0, 2000.0, 3000.0);
        let result = transform_ecef(ecef, &Datum::WGS84, &Datum::WGS84);
        assert!((result - ecef).length() < 1e-10);
    }

    #[test]
    fn test_transform_ecef_wgs84_to_cgcs2000() {
        let ecef = DVec3::new(6378137.0, 0.0, 0.0);
        let result = transform_ecef(ecef, &Datum::WGS84, &Datum::CGCS2000);

        // Should be essentially the same (identity transform)
        assert!((result - ecef).length() < 0.001);
    }

    #[test]
    fn test_get_helmert_transform_known() {
        let transform = get_helmert_transform(&Datum::WGS84, &Datum::CGCS2000);
        assert!(transform.is_some());
    }

    #[test]
    fn test_get_helmert_transform_unknown() {
        let transform = get_helmert_transform(&Datum::AIRY_1830, &Datum::CGCS2000);
        assert!(transform.is_none());
    }

    #[test]
    fn test_datum_radii() {
        let radii = Datum::WGS84.radii();
        assert!((radii.x - 6378137.0).abs() < 1e-6);
        assert!((radii.y - 6378137.0).abs() < 1e-6);
        assert!((radii.z - 6356752.314245).abs() < 1.0);
    }

    #[test]
    fn test_ecentricity_squared() {
        let e2 = Datum::WGS84.eccentricity_squared();
        assert!((e2 - 0.00669437999014).abs() < 1e-10);
    }
}
