//! Sun and Moon position computation.
//!
//! Maps to CesiumJS `Core/Simon1994PlanetaryPositions.js`
//! Computes approximate positions of the Sun and Moon in ECEF coordinates
//! based on Julian Date.

use glam::DVec3;

/// Astronomical unit in meters.
pub const AU_IN_METERS: f64 = 1.495978707e11;

/// Julian date of J2000 epoch (2000-01-01 12:00 TT).
pub const J2000_EPOCH: f64 = 2451545.0;

/// Number of Julian centuries since J2000.
fn julian_centuries(julian_date: f64) -> f64 {
    (julian_date - J2000_EPOCH) / 36525.0
}

/// Computes the Sun's position in Earth-Centered Inertial (ECI) coordinates.
///
/// Uses a simplified VSOP87 theory (low precision, ~0.01 degree accuracy).
///
/// # Arguments
/// * `julian_date` - The Julian Date (TT)
///
/// # Returns
/// Sun position in ECI (meters), ICRF frame
pub fn compute_sun_position_eci(julian_date: f64) -> DVec3 {
    let t = julian_centuries(julian_date);

    // Mean longitude (degrees)
    let l0 = normalize_degrees(280.46646 + 36000.76983 * t + 0.0003032 * t * t);

    // Mean anomaly (degrees)
    let m = normalize_degrees(357.52911 + 35999.05029 * t - 0.0001537 * t * t);
    let m_rad = m.to_radians();

    // Equation of center
    let c = (1.914602 - 0.004817 * t - 0.000014 * t * t) * m_rad.sin()
        + (0.019993 - 0.000101 * t) * (2.0 * m_rad).sin()
        + 0.000289 * (3.0 * m_rad).sin();

    // Sun's true longitude
    let sun_lon = (l0 + c).to_radians();

    // Sun's distance (AU)
    let e = 0.016708634 - 0.000042037 * t - 0.0000001267 * t * t;
    let v = m_rad + c.to_radians();
    let r = 1.000001018 * (1.0 - e * e) / (1.0 + e * v.cos());

    // Convert to meters
    let r_meters = r * AU_IN_METERS;

    // Obliquity of ecliptic
    let epsilon = (23.439291 - 0.0130042 * t).to_radians();

    // ECI coordinates (ICRF approximation)
    let x = r_meters * sun_lon.cos();
    let y = r_meters * sun_lon.sin() * epsilon.cos();
    let z = r_meters * sun_lon.sin() * epsilon.sin();

    DVec3::new(x, y, z)
}

/// Computes the Sun's direction (normalized) from Earth in ECI.
pub fn compute_sun_direction_eci(julian_date: f64) -> DVec3 {
    compute_sun_position_eci(julian_date).normalize()
}

/// Computes the Moon's position in Earth-Centered Inertial (ECI) coordinates.
///
/// Uses a simplified lunar theory (low precision, ~0.1 degree accuracy).
///
/// # Arguments
/// * `julian_date` - The Julian Date (TT)
///
/// # Returns
/// Moon position in ECI (meters), ICRF frame
pub fn compute_moon_position_eci(julian_date: f64) -> DVec3 {
    let t = julian_centuries(julian_date);

    // Moon's mean longitude
    let l = normalize_degrees(218.3165 + 481267.8813 * t);
    let l_rad = l.to_radians();

    // Moon's mean anomaly
    let m = normalize_degrees(134.9634 + 477198.8676 * t);
    let m_rad = m.to_radians();

    // Moon's mean elongation
    let d = normalize_degrees(297.8502 + 445267.1115 * t);
    let d_rad = d.to_radians();

    // Moon's argument of latitude
    let f = normalize_degrees(93.2720 + 483202.0175 * t);
    let f_rad = f.to_radians();

    // Sun's mean anomaly
    let ms = normalize_degrees(357.5291 + 35999.0503 * t);
    let ms_rad = ms.to_radians();

    // Ecliptic longitude (simplified)
    let lambda = l_rad
        + 0.1098_f64.to_radians() * m_rad.sin()
        + 0.0223_f64.to_radians() * (2.0 * d_rad - m_rad).sin()
        + 0.0115_f64.to_radians() * (2.0 * d_rad).sin()
        + 0.0037_f64.to_radians() * ms_rad.sin();

    // Ecliptic latitude (simplified)
    let beta = 0.0895_f64.to_radians() * f_rad.sin()
        + 0.0049_f64.to_radians() * (m_rad + f_rad).sin()
        + 0.0048_f64.to_radians() * (m_rad - f_rad).sin();

    // Distance (km → meters)
    let dist_km = 385001.0 - 20905.0 * m_rad.cos() - 3699.0 * (2.0 * d_rad - m_rad).cos()
        - 2956.0 * (2.0 * d_rad).cos();
    let dist_meters = dist_km * 1000.0;

    // Obliquity of ecliptic
    let epsilon = (23.439291 - 0.0130042 * t).to_radians();

    // Ecliptic to equatorial
    let x_ecl = dist_meters * beta.cos() * lambda.cos();
    let y_ecl = dist_meters * beta.cos() * lambda.sin();
    let z_ecl = dist_meters * beta.sin();

    // Rotate by obliquity
    let x = x_ecl;
    let y = y_ecl * epsilon.cos() - z_ecl * epsilon.sin();
    let z = y_ecl * epsilon.sin() + z_ecl * epsilon.cos();

    DVec3::new(x, y, z)
}

/// Computes the Moon's direction (normalized) from Earth in ECI.
pub fn compute_moon_direction_eci(julian_date: f64) -> DVec3 {
    compute_moon_position_eci(julian_date).normalize()
}

/// Approximate GMST (Greenwich Mean Sidereal Time) in radians.
pub fn compute_gmst(julian_date: f64) -> f64 {
    let t = julian_centuries(julian_date);
    // GMST in degrees
    let gmst_deg = 280.46061837 + 360.98564736629 * (julian_date - J2000_EPOCH)
        + 0.000387933 * t * t
        - t * t * t / 38710000.0;
    normalize_degrees(gmst_deg).to_radians()
}

/// Rotates an ECI vector to ECEF using GMST.
pub fn eci_to_ecef(eci: DVec3, julian_date: f64) -> DVec3 {
    let gmst = compute_gmst(julian_date);
    let cos_g = gmst.cos();
    let sin_g = gmst.sin();

    DVec3::new(
        cos_g * eci.x + sin_g * eci.y,
        -sin_g * eci.x + cos_g * eci.y,
        eci.z,
    )
}

/// Computes the Sun's position in ECEF coordinates.
pub fn compute_sun_position_ecef(julian_date: f64) -> DVec3 {
    let eci = compute_sun_position_eci(julian_date);
    eci_to_ecef(eci, julian_date)
}

/// Computes the Moon's position in ECEF coordinates.
pub fn compute_moon_position_ecef(julian_date: f64) -> DVec3 {
    let eci = compute_moon_position_eci(julian_date);
    eci_to_ecef(eci, julian_date)
}

/// Normalizes degrees to [0, 360).
fn normalize_degrees(degrees: f64) -> f64 {
    let result = degrees % 360.0;
    if result < 0.0 {
        result + 360.0
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_sun_position_j2000() {
        // At J2000 epoch, sun should be roughly in the direction of
        // ecliptic longitude ~280 degrees
        let pos = compute_sun_position_eci(J2000_EPOCH);

        // Distance should be approximately 1 AU
        let dist = pos.length();
        assert!((dist - AU_IN_METERS).abs() / AU_IN_METERS < 0.02); // Within 2%
    }

    #[test]
    fn test_sun_direction_normalized() {
        let dir = compute_sun_direction_eci(J2000_EPOCH);
        assert!((dir.length() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_moon_position_distance() {
        // Moon should be approximately 385,000 km from Earth
        let pos = compute_moon_position_eci(J2000_EPOCH);
        let dist_km = pos.length() / 1000.0;

        // Moon distance varies between ~356,000 and ~407,000 km
        assert!(dist_km > 350_000.0);
        assert!(dist_km < 410_000.0);
    }

    #[test]
    fn test_moon_direction_normalized() {
        let dir = compute_moon_direction_eci(J2000_EPOCH);
        assert!((dir.length() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_gmst_range() {
        let gmst = compute_gmst(J2000_EPOCH);
        assert!(gmst >= 0.0);
        assert!(gmst < 2.0 * PI);
    }

    #[test]
    fn test_eci_to_ecef_preserves_magnitude() {
        let eci = DVec3::new(1.0e11, 2.0e10, 3.0e10);
        let ecef = eci_to_ecef(eci, J2000_EPOCH);

        // Rotation should preserve magnitude
        assert!((eci.length() - ecef.length()).abs() / eci.length() < 1e-10);
    }

    #[test]
    fn test_sun_position_ecef() {
        let pos = compute_sun_position_ecef(J2000_EPOCH);
        let dist = pos.length();
        assert!((dist - AU_IN_METERS).abs() / AU_IN_METERS < 0.02);
    }

    #[test]
    fn test_normalize_degrees() {
        assert!((normalize_degrees(370.0) - 10.0).abs() < 1e-10);
        assert!((normalize_degrees(-10.0) - 350.0).abs() < 1e-10);
        assert!((normalize_degrees(720.0) - 0.0).abs() < 1e-10);
    }
}
