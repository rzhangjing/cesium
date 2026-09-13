//! Atmospheric scattering model.
//!
//! Maps to CesiumJS `Scene/SkyAtmosphere.js` and
//! `Core/Atmosphere.js`
//!
//! Implements Rayleigh scattering approximation for sky color computation.

use glam::DVec3;

/// Physical constants for Earth's atmosphere.
pub mod constants {
    /// Radius of the Earth at the equator (meters).
    pub const EARTH_RADIUS: f64 = 6378137.0;

    /// Height of the atmosphere (meters).
    pub const ATMOSPHERE_HEIGHT: f64 = 100000.0;

    /// Outer radius (Earth + atmosphere).
    pub const OUTER_RADIUS: f64 = EARTH_RADIUS + ATMOSPHERE_HEIGHT;

    /// Rayleigh scattering coefficient at sea level (per meter).
    /// Values for RGB wavelengths (680nm, 550nm, 440nm).
    pub const RAYLEIGH_COEFFICIENTS: [f64; 3] = [5.8e-6, 13.5e-6, 33.1e-6];

    /// Rayleigh scale height (meters).
    pub const RAYLEIGH_SCALE_HEIGHT: f64 = 8000.0;

    /// Mie scattering coefficient.
    pub const MIE_COEFFICIENT: f64 = 21e-6;

    /// Mie scale height (meters).
    pub const MIE_SCALE_HEIGHT: f64 = 1200.0;

    /// Mie preferred scattering direction (anisotropy).
    pub const MIE_ANISOTROPY: f64 = 0.758;

    /// Solar intensity.
    pub const SOLAR_INTENSITY: f64 = 20.0;
}

/// Atmosphere parameters for scattering computation.
#[derive(Debug, Clone)]
pub struct AtmosphereParameters {
    /// Inner radius (Earth surface).
    pub inner_radius: f64,
    /// Outer radius (atmosphere boundary).
    pub outer_radius: f64,
    /// Rayleigh coefficients [R, G, B].
    pub rayleigh_coefficients: [f64; 3],
    /// Rayleigh scale height.
    pub rayleigh_scale_height: f64,
    /// Mie coefficient.
    pub mie_coefficient: f64,
    /// Mie scale height.
    pub mie_scale_height: f64,
    /// Mie anisotropy (g parameter).
    pub mie_anisotropy: f64,
    /// Solar intensity multiplier.
    pub solar_intensity: f64,
}

impl Default for AtmosphereParameters {
    fn default() -> Self {
        Self {
            inner_radius: constants::EARTH_RADIUS,
            outer_radius: constants::OUTER_RADIUS,
            rayleigh_coefficients: constants::RAYLEIGH_COEFFICIENTS,
            rayleigh_scale_height: constants::RAYLEIGH_SCALE_HEIGHT,
            mie_coefficient: constants::MIE_COEFFICIENT,
            mie_scale_height: constants::MIE_SCALE_HEIGHT,
            mie_anisotropy: constants::MIE_ANISOTROPY,
            solar_intensity: constants::SOLAR_INTENSITY,
        }
    }
}

/// Computes the Rayleigh phase function.
///
/// # Arguments
/// * `cos_theta` - Cosine of the scattering angle
pub fn rayleigh_phase(cos_theta: f64) -> f64 {
    3.0 / (16.0 * std::f64::consts::PI) * (1.0 + cos_theta * cos_theta)
}

/// Computes the Henyey-Greenstein (Mie) phase function.
///
/// # Arguments
/// * `cos_theta` - Cosine of the scattering angle
/// * `g` - Anisotropy parameter (-1 to 1)
pub fn mie_phase(cos_theta: f64, g: f64) -> f64 {
    let g2 = g * g;
    let num = (1.0 - g2) * (1.0 + cos_theta * cos_theta);
    let denom = (2.0 + g2) * (1.0 + g2 - 2.0 * g * cos_theta).powf(1.5);
    num / (4.0 * std::f64::consts::PI * denom)
}

/// Computes the atmospheric density at a given height using exponential decay.
///
/// # Arguments
/// * `height` - Height above surface (meters)
/// * `scale_height` - Scale height (meters)
pub fn atmospheric_density(height: f64, scale_height: f64) -> f64 {
    (-height / scale_height).exp()
}

/// Computes approximate sky color for a given view and sun direction.
///
/// This is a simplified single-scattering approximation.
///
/// # Arguments
/// * `view_direction` - Normalized view direction from camera
/// * `sun_direction` - Normalized direction to the sun
/// * `camera_height` - Camera height above surface (meters)
/// * `params` - Atmosphere parameters
///
/// # Returns
/// Approximate sky color [R, G, B] (linear, may exceed 1.0)
pub fn compute_sky_color(
    view_direction: DVec3,
    sun_direction: DVec3,
    camera_height: f64,
    params: &AtmosphereParameters,
) -> [f64; 3] {
    let cos_theta = view_direction.dot(sun_direction);

    // Phase functions
    let rayleigh_p = rayleigh_phase(cos_theta);
    let mie_p = mie_phase(cos_theta, params.mie_anisotropy);

    // Density at camera height
    let height_above_surface = camera_height.max(0.0);
    let rayleigh_density = atmospheric_density(height_above_surface, params.rayleigh_scale_height);
    let mie_density = atmospheric_density(height_above_surface, params.mie_scale_height);

    // Optical depth approximation (simplified)
    let path_length = params.outer_radius - params.inner_radius;

    let mut color = [0.0f64; 3];
    for (c, beta) in color.iter_mut().zip(params.rayleigh_coefficients.iter()) {
        let rayleigh = beta * rayleigh_density * rayleigh_p * path_length;
        let mie = params.mie_coefficient * mie_density * mie_p * path_length;
        *c = (rayleigh + mie) * params.solar_intensity;
    }

    color
}

/// Computes the horizon glow color based on sun elevation.
///
/// # Arguments
/// * `sun_elevation` - Sun elevation angle in radians (negative = below horizon)
///
/// # Returns
/// Horizon glow color [R, G, B]
pub fn compute_horizon_glow(sun_elevation: f64) -> [f64; 3] {
    // Sun below horizon: reddish glow
    // Sun above horizon: whitish-blue
    let t = (sun_elevation / (std::f64::consts::FRAC_PI_2)).clamp(-1.0, 1.0);

    if t < 0.0 {
        // Sunset/sunrise colors
        let factor = 1.0 + t; // 0 at -90°, 1 at horizon
        [
            0.8 * factor,
            0.3 * factor * factor,
            0.1 * factor * factor * factor,
        ]
    } else {
        // Daytime sky
        [0.4 + 0.3 * t, 0.6 + 0.2 * t, 0.9 + 0.1 * t]
    }
}

/// Sky box configuration.
#[derive(Debug, Clone)]
pub struct SkyBoxConfig {
    /// Whether the sky box is shown.
    pub show: bool,
    /// Source URIs for the 6 faces [+X, -X, +Y, -Y, +Z, -Z].
    pub sources: [Option<String>; 6],
    /// Rotation angle (radians).
    pub rotation: f64,
}

impl Default for SkyBoxConfig {
    fn default() -> Self {
        Self {
            show: true,
            sources: [None, None, None, None, None, None],
            rotation: 0.0,
        }
    }
}

/// Lighting configuration for the scene.
#[derive(Debug, Clone)]
pub struct LightingConfig {
    /// Sun direction in ECEF (normalized).
    pub sun_direction: DVec3,
    /// Sun color [R, G, B].
    pub sun_color: [f64; 3],
    /// Sun intensity.
    pub sun_intensity: f64,
    /// Ambient light color [R, G, B].
    pub ambient_color: [f64; 3],
    /// Ambient intensity.
    pub ambient_intensity: f64,
    /// Whether shadows are enabled.
    pub shadows_enabled: bool,
}

impl Default for LightingConfig {
    fn default() -> Self {
        Self {
            sun_direction: DVec3::new(1.0, 0.0, 0.0),
            sun_color: [1.0, 1.0, 0.9],
            sun_intensity: 1.0,
            ambient_color: [0.1, 0.1, 0.15],
            ambient_intensity: 0.3,
            shadows_enabled: true,
        }
    }
}

impl LightingConfig {
    /// Updates the sun direction from a Julian Date.
    pub fn update_from_julian_date(&mut self, julian_date: f64) {
        self.sun_direction = crate::celestial::compute_sun_direction_eci(julian_date);
    }

    /// Computes the sun elevation angle at a given position.
    ///
    /// # Arguments
    /// * `surface_normal` - The surface normal at the position (normalized, ECEF)
    pub fn sun_elevation_at(&self, surface_normal: DVec3) -> f64 {
        let cos_angle = surface_normal.dot(self.sun_direction);
        std::f64::consts::FRAC_PI_2 - cos_angle.acos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_rayleigh_phase() {
        // Forward scattering (cos_theta = 1)
        let forward = rayleigh_phase(1.0);
        // Backward scattering (cos_theta = -1)
        let backward = rayleigh_phase(-1.0);
        // Rayleigh is symmetric
        assert!((forward - backward).abs() < 1e-10);

        // Side scattering (cos_theta = 0)
        let side = rayleigh_phase(0.0);
        assert!(side < forward);
    }

    #[test]
    fn test_mie_phase() {
        let g = 0.758;
        // Forward scattering should be strongest for positive g
        let forward = mie_phase(1.0, g);
        let backward = mie_phase(-1.0, g);
        assert!(forward > backward);
    }

    #[test]
    fn test_atmospheric_density() {
        // At sea level, density = 1
        assert!((atmospheric_density(0.0, 8000.0) - 1.0).abs() < 1e-10);
        // At one scale height, density = 1/e
        assert!((atmospheric_density(8000.0, 8000.0) - 1.0 / std::f64::consts::E).abs() < 1e-10);
    }

    #[test]
    fn test_compute_sky_color() {
        let params = AtmosphereParameters::default();
        let view = DVec3::new(0.0, 0.0, 1.0);
        let sun = DVec3::new(0.0, 0.0, 1.0);

        let color = compute_sky_color(view, sun, 0.0, &params);

        // Blue channel should be strongest (Rayleigh scattering)
        assert!(color[2] > color[0]); // Blue > Red
    }

    #[test]
    fn test_horizon_glow_sunset() {
        // Sun slightly below horizon
        let color = compute_horizon_glow(-0.1);
        assert!(color[0] > color[2]); // Red > Blue at sunset
    }

    #[test]
    fn test_horizon_glow_noon() {
        // Sun overhead
        let color = compute_horizon_glow(PI / 2.0);
        assert!(color[2] > color[0]); // Blue > Red at noon
    }

    #[test]
    fn test_lighting_config() {
        let config = LightingConfig::default();
        assert!((config.sun_direction.length() - 1.0).abs() < 1e-10);
        assert!(config.shadows_enabled);
    }

    #[test]
    fn test_sun_elevation() {
        let config = LightingConfig {
            sun_direction: DVec3::new(0.0, 0.0, 1.0), // Sun directly overhead (Z-up)
            ..Default::default()
        };

        // Surface normal pointing at sun
        let elevation = config.sun_elevation_at(DVec3::new(0.0, 0.0, 1.0));
        assert!((elevation - PI / 2.0).abs() < 1e-10); // 90 degrees

        // Surface normal perpendicular to sun
        let elevation = config.sun_elevation_at(DVec3::new(1.0, 0.0, 0.0));
        assert!(elevation.abs() < 1e-10); // 0 degrees (horizon)
    }
}
