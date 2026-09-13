//! Ground atmosphere and sky rendering.
//!
//! Maps to CesiumJS atmosphere effects:
//! - `Scene/SkyAtmosphere.js`
//! - `Scene/SkyBox.js`
//! - Ground atmosphere (view from surface)

use glam::DVec3;

/// Sky atmosphere configuration.
///
/// Maps to CesiumJS `Scene/SkyAtmosphere.js`
#[derive(Debug, Clone)]
pub struct SkyAtmosphereConfig {
    /// Whether the sky atmosphere is shown.
    pub show: bool,
    /// Hue shift (-1.0 to 1.0).
    pub hue_shift: f64,
    /// Saturation shift (-1.0 to 1.0).
    pub saturation_shift: f64,
    /// Brightness shift (-1.0 to 1.0).
    pub brightness_shift: f64,
    /// Per-position radius for atmosphere (meters).
    pub atmosphere_radius: f64,
}

impl Default for SkyAtmosphereConfig {
    fn default() -> Self {
        Self {
            show: true,
            hue_shift: 0.0,
            saturation_shift: 0.0,
            brightness_shift: 0.0,
            atmosphere_radius: 6378137.0 + 60000.0, // Earth radius + 60km atmosphere
        }
    }
}

/// Sky box configuration for star rendering.
///
/// Maps to CesiumJS `Scene/SkyBox.js`
#[derive(Debug, Clone)]
pub struct SkyBoxConfig {
    /// Whether the sky box is shown.
    pub show: bool,
    /// Source URLs for cube map faces [px, nx, py, ny, pz, nz].
    pub sources: Option<[String; 6]>,
    /// Star sphere radius.
    pub radius: f64,
}

impl Default for SkyBoxConfig {
    fn default() -> Self {
        Self {
            show: true,
            sources: None,
            radius: 1e15, // Very large radius for stars
        }
    }
}

/// Ground atmosphere parameters for view-from-surface rendering.
#[derive(Debug, Clone)]
pub struct GroundAtmosphere {
    /// Rayleigh scattering coefficients [r, g, b].
    pub rayleigh_coefficients: [f64; 3],
    /// Mie scattering coefficient.
    pub mie_coefficient: f64,
    /// Mie directional factor (g).
    pub mie_g: f64,
    /// Atmosphere scale height (meters).
    pub scale_height: f64,
    /// Sun intensity factor.
    pub sun_intensity: f64,
}

impl Default for GroundAtmosphere {
    fn default() -> Self {
        Self {
            // Standard Rayleigh scattering (blue sky)
            rayleigh_coefficients: [5.5e-6, 13.0e-6, 22.4e-6],
            mie_coefficient: 21e-6,
            mie_g: 0.758,
            scale_height: 8000.0,
            sun_intensity: 20.0,
        }
    }
}

impl GroundAtmosphere {
    /// Computes the sky color for a given view and sun direction.
    ///
    /// # Arguments
    /// * `view_direction` - Normalized view direction
    /// * `sun_direction` - Normalized direction to the sun
    /// * `camera_height` - Camera height above surface (meters)
    ///
    /// # Returns
    /// RGB color [0.0-1.0]
    pub fn compute_sky_color(
        &self,
        view_direction: DVec3,
        sun_direction: DVec3,
        camera_height: f64,
    ) -> [f64; 3] {
        let cos_theta = view_direction.dot(sun_direction);

        // Rayleigh phase function
        let rayleigh_phase = 0.75 * (1.0 + cos_theta * cos_theta);

        // Mie phase function (Henyey-Greenstein)
        let g2 = self.mie_g * self.mie_g;
        let mie_phase = (1.0 - g2)
            / (4.0 * std::f64::consts::PI * (1.0 + g2 - 2.0 * self.mie_g * cos_theta).powf(1.5));

        // Optical depth (simplified)
        let height_factor = (-camera_height / self.scale_height).exp();

        let mut color = [0.0f64; 3];
        for (c, beta) in color.iter_mut().zip(self.rayleigh_coefficients.iter()) {
            let rayleigh = beta * rayleigh_phase;
            let mie = self.mie_coefficient * mie_phase;
            let optical_depth = (rayleigh + mie) * height_factor;

            // Transmittance
            let transmittance = (-optical_depth * 1000.0).exp();

            // In-scattering
            let in_scatter = (1.0 - transmittance) * self.sun_intensity;

            *c = (rayleigh / (rayleigh + mie + 1e-10)) * in_scatter;
        }

        // Tone mapping (simple Reinhard)
        for c in color.iter_mut() {
            *c = *c / (1.0 + *c);
            *c = c.clamp(0.0, 1.0);
        }

        color
    }

    /// Computes the horizon glow color near sunset/sunrise.
    ///
    /// # Arguments
    /// * `sun_elevation` - Sun elevation angle in radians
    ///
    /// # Returns
    /// RGB color for horizon glow
    pub fn compute_horizon_glow(&self, sun_elevation: f64) -> [f64; 3] {
        // Glow is strongest when sun is near horizon
        let t = (-sun_elevation.abs() / 0.2).exp();

        // Orange/red glow
        [
            (1.0 * t).clamp(0.0, 1.0),
            (0.4 * t).clamp(0.0, 1.0),
            (0.1 * t).clamp(0.0, 1.0),
        ]
    }

    /// Computes the zenith color (sky directly overhead).
    pub fn compute_zenith_color(&self, sun_elevation: f64) -> [f64; 3] {
        // Blue sky during day, dark at night
        let day_factor = (sun_elevation / 0.3).clamp(0.0, 1.0);

        [
            0.1 * day_factor,
            0.3 * day_factor,
            0.8 * day_factor,
        ]
    }
}

/// Lighting configuration for globe rendering.
///
/// Maps to CesiumJS globe lighting
#[derive(Debug, Clone)]
pub struct GlobeLighting {
    /// Whether lighting is enabled.
    pub enabled: bool,
    /// Sun direction (normalized, in ECEF).
    pub sun_direction: DVec3,
    /// Sun color [r, g, b].
    pub sun_color: [f64; 3],
    /// Ambient light color [r, g, b].
    pub ambient_color: [f64; 3],
    /// Whether to show the day/night terminator.
    pub show_terminator: bool,
    /// Specular intensity for water surfaces.
    pub specular_intensity: f64,
}

impl Default for GlobeLighting {
    fn default() -> Self {
        Self {
            enabled: false,
            sun_direction: DVec3::X,
            sun_color: [1.0, 1.0, 0.9],
            ambient_color: [0.1, 0.1, 0.15],
            show_terminator: true,
            specular_intensity: 0.5,
        }
    }
}

impl GlobeLighting {
    /// Computes the diffuse lighting factor at a surface point.
    ///
    /// # Arguments
    /// * `surface_normal` - Surface normal at the point
    ///
    /// # Returns
    /// Diffuse factor [0.0-1.0]
    pub fn compute_diffuse(&self, surface_normal: DVec3) -> f64 {
        if !self.enabled {
            return 1.0;
        }
        surface_normal.dot(self.sun_direction).max(0.0)
    }

    /// Computes the specular highlight for water surfaces.
    ///
    /// # Arguments
    /// * `surface_normal` - Surface normal
    /// * `view_direction` - Direction from surface to camera
    ///
    /// # Returns
    /// Specular intensity [0.0-1.0]
    pub fn compute_specular(&self, surface_normal: DVec3, view_direction: DVec3) -> f64 {
        if !self.enabled || self.specular_intensity <= 0.0 {
            return 0.0;
        }

        // Blinn-Phong specular
        let half_vector = (self.sun_direction + view_direction).normalize();
        let n_dot_h = surface_normal.dot(half_vector).max(0.0);

        // High shininess for water
        n_dot_h.powf(64.0) * self.specular_intensity
    }

    /// Computes the final lit color for a surface.
    pub fn compute_lit_color(
        &self,
        base_color: [f64; 3],
        surface_normal: DVec3,
        view_direction: DVec3,
        is_water: bool,
    ) -> [f64; 3] {
        if !self.enabled {
            return base_color;
        }

        let diffuse = self.compute_diffuse(surface_normal);

        let mut result = [0.0f64; 3];
        for i in 0..3 {
            let sun_contrib = base_color[i] * self.sun_color[i] * diffuse;
            let ambient_contrib = base_color[i] * self.ambient_color[i];
            result[i] = (sun_contrib + ambient_contrib).clamp(0.0, 1.0);
        }

        // Add specular for water
        if is_water {
            let specular = self.compute_specular(surface_normal, view_direction);
            for (r, sun) in result.iter_mut().zip(self.sun_color.iter()) {
                *r = (*r + specular * sun).clamp(0.0, 1.0);
            }
        }

        result
    }

    /// Computes the night-side color (city lights approximation).
    pub fn compute_night_color(&self, base_color: [f64; 3]) -> [f64; 3] {
        // Darken significantly on night side
        [
            base_color[0] * 0.02,
            base_color[1] * 0.02,
            base_color[2] * 0.05,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    #[test]
    fn test_sky_atmosphere_default() {
        let config = SkyAtmosphereConfig::default();
        assert!(config.show);
        assert!((config.hue_shift).abs() < 1e-10);
        assert!(config.atmosphere_radius > 6378137.0);
    }

    #[test]
    fn test_sky_box_default() {
        let config = SkyBoxConfig::default();
        assert!(config.show);
        assert!(config.sources.is_none());
        assert!(config.radius > 1e10);
    }

    #[test]
    fn test_ground_atmosphere_day() {
        let atmosphere = GroundAtmosphere::default();

        // Looking up with sun overhead
        let view = DVec3::new(0.0, 0.0, 1.0);
        let sun = DVec3::new(0.0, 0.0, 1.0);

        let color = atmosphere.compute_sky_color(view, sun, 0.0);

        // Should be blue-ish
        assert!(color[2] > color[0]); // Blue > Red
    }

    #[test]
    fn test_ground_atmosphere_sunset() {
        let atmosphere = GroundAtmosphere::default();

        // Sun at horizon
        let view = DVec3::new(1.0, 0.0, 0.0);
        let sun = DVec3::new(1.0, 0.0, 0.0);

        let color = atmosphere.compute_sky_color(view, sun, 0.0);

        // All channels should be valid
        for c in &color {
            assert!(*c >= 0.0 && *c <= 1.0);
        }
    }

    #[test]
    fn test_horizon_glow_sunset() {
        let atmosphere = GroundAtmosphere::default();

        // Sun slightly below horizon
        let glow = atmosphere.compute_horizon_glow(-0.1);

        // Should have warm colors
        assert!(glow[0] > glow[2]); // Red > Blue
    }

    #[test]
    fn test_horizon_glow_noon() {
        let atmosphere = GroundAtmosphere::default();

        // Sun high in sky
        let glow = atmosphere.compute_horizon_glow(FRAC_PI_2);

        // Should be minimal glow
        assert!(glow[0] < 0.1);
    }

    #[test]
    fn test_zenith_color_day() {
        let atmosphere = GroundAtmosphere::default();
        let color = atmosphere.compute_zenith_color(0.5);

        // Blue sky
        assert!(color[2] > color[0]);
    }

    #[test]
    fn test_zenith_color_night() {
        let atmosphere = GroundAtmosphere::default();
        let color = atmosphere.compute_zenith_color(-0.5);

        // Dark sky
        assert!(color[2] < 0.1);
    }

    #[test]
    fn test_globe_lighting_disabled() {
        let lighting = GlobeLighting::default();
        assert!(!lighting.enabled);

        let normal = DVec3::Z;
        assert!((lighting.compute_diffuse(normal) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_globe_lighting_enabled() {
        let lighting = GlobeLighting {
            enabled: true,
            sun_direction: DVec3::new(0.0, 0.0, 1.0),
            ..Default::default()
        };

        // Surface facing the sun
        let normal = DVec3::new(0.0, 0.0, 1.0);
        let diffuse = lighting.compute_diffuse(normal);
        assert!((diffuse - 1.0).abs() < 1e-10);

        // Surface facing away
        let normal_away = DVec3::new(0.0, 0.0, -1.0);
        let diffuse_away = lighting.compute_diffuse(normal_away);
        assert!(diffuse_away.abs() < 1e-10);
    }

    #[test]
    fn test_specular_water() {
        let lighting = GlobeLighting {
            enabled: true,
            sun_direction: DVec3::new(0.0, 0.0, 1.0),
            ..Default::default()
        };

        let normal = DVec3::new(0.0, 0.0, 1.0);
        let view = DVec3::new(0.0, 0.0, 1.0);

        let specular = lighting.compute_specular(normal, view);
        assert!(specular > 0.0);
    }

    #[test]
    fn test_lit_color() {
        let lighting = GlobeLighting {
            enabled: true,
            sun_direction: DVec3::Z,
            ..Default::default()
        };

        let base = [0.5, 0.5, 0.5];
        let normal = DVec3::Z;
        let view = DVec3::Z;

        let lit = lighting.compute_lit_color(base, normal, view, false);

        // Should be brighter than ambient
        assert!(lit[0] > 0.1);
    }

    #[test]
    fn test_night_color() {
        let lighting = GlobeLighting::default();
        let base = [0.5, 0.5, 0.5];
        let night = lighting.compute_night_color(base);

        // Should be very dark
        assert!(night[0] < 0.05);
        assert!(night[1] < 0.05);
    }
}
