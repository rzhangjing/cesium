//! Star sphere and sky atmosphere enhancements.
//!
//! Maps to CesiumJS:
//! - `Scene/StarSphere.js` — star catalog rendering
//! - `Scene/SkyAtmosphere.js` — HSB shifts, dynamic lighting, per-fragment
//! - `Scene/SkyBox.js` — TEME frame sky box
//!
//! Domain layer — pure Rust, f64 precision.

use glam::DVec3;

// ─── Star Catalog ───────────────────────────────────────────────────────────

/// A single star entry in the star catalog.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Star {
    /// Right ascension (radians, 0..2π).
    pub right_ascension: f64,
    /// Declination (radians, -π/2..π/2).
    pub declination: f64,
    /// Apparent magnitude (lower = brighter).
    pub magnitude: f64,
    /// Star color temperature (Kelvin), used for spectral color.
    pub color_temperature: f64,
}

impl Star {
    /// Creates a star from RA/Dec in degrees and magnitude.
    pub fn from_degrees(ra_deg: f64, dec_deg: f64, magnitude: f64) -> Self {
        Self {
            right_ascension: ra_deg.to_radians(),
            declination: dec_deg.to_radians(),
            magnitude,
            color_temperature: 6500.0, // Default white
        }
    }

    /// Computes the unit direction vector (ECI/TEME frame) for this star.
    pub fn direction(&self) -> DVec3 {
        let cos_dec = self.declination.cos();
        DVec3::new(
            cos_dec * self.right_ascension.cos(),
            cos_dec * self.right_ascension.sin(),
            self.declination.sin(),
        )
    }

    /// Computes the visual brightness (0..1) from magnitude.
    ///
    /// Uses the Pogson scale: brightness ∝ 10^(-0.4 * magnitude).
    /// Normalized so magnitude 0 → 1.0, magnitude 6 → ~0.004.
    pub fn brightness(&self) -> f64 {
        10.0_f64.powf(-0.4 * self.magnitude)
    }

    /// Computes an approximate RGB color from the star's color temperature.
    ///
    /// Based on blackbody radiation approximation (Tanner Helland algorithm).
    pub fn spectral_color(&self) -> [f64; 3] {
        color_from_temperature(self.color_temperature)
    }
}

/// Star sphere configuration and rendering parameters.
///
/// Maps to CesiumJS `StarSphere` which renders stars on a celestial sphere.
#[derive(Debug, Clone)]
pub struct StarSphere {
    /// Whether the star sphere is shown.
    pub show: bool,
    /// The star catalog.
    pub stars: Vec<Star>,
    /// Minimum magnitude to render (stars dimmer than this are culled).
    pub minimum_magnitude: f64,
    /// Maximum magnitude to render (stars brighter than this are culled).
    pub maximum_magnitude: f64,
    /// Star point size in pixels (base size for magnitude 0).
    pub base_point_size: f64,
    /// Whether to use HDR rendering for stars.
    pub use_hdr: bool,
    /// Overall brightness multiplier.
    pub brightness_multiplier: f64,
}

impl Default for StarSphere {
    fn default() -> Self {
        Self {
            show: true,
            stars: Vec::new(),
            minimum_magnitude: -2.0,
            maximum_magnitude: 6.0,
            base_point_size: 3.0,
            use_hdr: true,
            brightness_multiplier: 1.0,
        }
    }
}

impl StarSphere {
    /// Creates a star sphere with a built-in bright star catalog.
    pub fn with_builtin_catalog() -> Self {
        Self {
            stars: builtin_bright_stars(),
            ..Default::default()
        }
    }

    /// Returns stars visible within the magnitude range.
    pub fn visible_stars(&self) -> impl Iterator<Item = &Star> {
        self.stars
            .iter()
            .filter(|s| s.magnitude >= self.minimum_magnitude && s.magnitude <= self.maximum_magnitude)
    }

    /// Computes the rendered point size for a star based on its magnitude.
    ///
    /// Brighter stars (lower magnitude) get larger point sizes.
    pub fn star_point_size(&self, star: &Star) -> f64 {
        let magnitude_range = self.maximum_magnitude - self.minimum_magnitude;
        if magnitude_range <= 0.0 {
            return self.base_point_size;
        }
        let t = (star.magnitude - self.minimum_magnitude) / magnitude_range;
        // Brighter (lower mag) → larger size
        self.base_point_size * (1.0 - t * 0.7)
    }

    /// Computes the final rendered color for a star (with brightness applied).
    pub fn star_render_color(&self, star: &Star) -> [f64; 3] {
        let base_color = star.spectral_color();
        let brightness = star.brightness() * self.brightness_multiplier;
        [
            base_color[0] * brightness,
            base_color[1] * brightness,
            base_color[2] * brightness,
        ]
    }

    /// Adds a star to the catalog.
    pub fn add_star(&mut self, star: Star) {
        self.stars.push(star);
    }

    /// Returns the number of stars in the catalog.
    pub fn star_count(&self) -> usize {
        self.stars.len()
    }
}

/// Built-in catalog of the brightest stars (subset of Hipparcos/Yale BSC).
fn builtin_bright_stars() -> Vec<Star> {
    vec![
        // Sirius (α CMa) — brightest star
        Star { right_ascension: 101.287_f64.to_radians(), declination: (-16.716_f64).to_radians(), magnitude: -1.46, color_temperature: 9940.0 },
        // Canopus (α Car)
        Star { right_ascension: 95.988_f64.to_radians(), declination: (-52.696_f64).to_radians(), magnitude: -0.74, color_temperature: 7350.0 },
        // Arcturus (α Boo)
        Star { right_ascension: 213.915_f64.to_radians(), declination: 19.182_f64.to_radians(), magnitude: -0.05, color_temperature: 4286.0 },
        // Vega (α Lyr)
        Star { right_ascension: 279.234_f64.to_radians(), declination: 38.784_f64.to_radians(), magnitude: 0.03, color_temperature: 9602.0 },
        // Capella (α Aur)
        Star { right_ascension: 79.172_f64.to_radians(), declination: 45.998_f64.to_radians(), magnitude: 0.08, color_temperature: 4970.0 },
        // Rigel (β Ori)
        Star { right_ascension: 78.634_f64.to_radians(), declination: (-8.202_f64).to_radians(), magnitude: 0.13, color_temperature: 12100.0 },
        // Procyon (α CMi)
        Star { right_ascension: 114.825_f64.to_radians(), declination: 5.225_f64.to_radians(), magnitude: 0.34, color_temperature: 6530.0 },
        // Betelgeuse (α Ori)
        Star { right_ascension: 88.793_f64.to_radians(), declination: 7.407_f64.to_radians(), magnitude: 0.42, color_temperature: 3500.0 },
        // Altair (α Aql)
        Star { right_ascension: 297.696_f64.to_radians(), declination: 8.868_f64.to_radians(), magnitude: 0.77, color_temperature: 7550.0 },
        // Aldebaran (α Tau)
        Star { right_ascension: 68.980_f64.to_radians(), declination: 16.509_f64.to_radians(), magnitude: 0.85, color_temperature: 3910.0 },
        // Antares (α Sco)
        Star { right_ascension: 247.352_f64.to_radians(), declination: (-26.432_f64).to_radians(), magnitude: 1.09, color_temperature: 3660.0 },
        // Spica (α Vir)
        Star { right_ascension: 201.298_f64.to_radians(), declination: (-11.161_f64).to_radians(), magnitude: 1.04, color_temperature: 22400.0 },
        // Pollux (β Gem)
        Star { right_ascension: 116.329_f64.to_radians(), declination: 28.026_f64.to_radians(), magnitude: 1.14, color_temperature: 4666.0 },
        // Fomalhaut (α PsA)
        Star { right_ascension: 344.413_f64.to_radians(), declination: (-29.622_f64).to_radians(), magnitude: 1.16, color_temperature: 8590.0 },
        // Deneb (α Cyg)
        Star { right_ascension: 310.358_f64.to_radians(), declination: 45.280_f64.to_radians(), magnitude: 1.25, color_temperature: 8525.0 },
        // Regulus (α Leo)
        Star { right_ascension: 152.093_f64.to_radians(), declination: 11.967_f64.to_radians(), magnitude: 1.35, color_temperature: 12460.0 },
        // Castor (α Gem)
        Star { right_ascension: 113.650_f64.to_radians(), declination: 31.888_f64.to_radians(), magnitude: 1.58, color_temperature: 10286.0 },
        // Bellatrix (γ Ori)
        Star { right_ascension: 81.283_f64.to_radians(), declination: 6.350_f64.to_radians(), magnitude: 1.64, color_temperature: 22000.0 },
        // Alnilam (ε Ori)
        Star { right_ascension: 84.053_f64.to_radians(), declination: (-1.202_f64).to_radians(), magnitude: 1.69, color_temperature: 27500.0 },
        // Polaris (α UMi) — North Star
        Star { right_ascension: 37.954_f64.to_radians(), declination: 89.264_f64.to_radians(), magnitude: 1.98, color_temperature: 6015.0 },
    ]
}

/// Approximates an RGB color from a blackbody temperature (Kelvin).
///
/// Based on Tanner Helland's algorithm for color temperature to RGB.
fn color_from_temperature(kelvin: f64) -> [f64; 3] {
    let temp = kelvin.clamp(1000.0, 40000.0) / 100.0;

    // Red
    let r = if temp <= 66.0 {
        1.0
    } else {
        let x = temp - 60.0;
        (329.698727446 * x.powf(-0.1332047592) / 255.0).clamp(0.0, 1.0)
    };

    // Green
    let g = if temp <= 66.0 {
        (99.4708025861 * temp.ln() - 161.1195681661) / 255.0
    } else {
        let x = temp - 60.0;
        (288.1221695283 * x.powf(-0.0755148492) / 255.0).clamp(0.0, 1.0)
    };
    let g = g.clamp(0.0, 1.0);

    // Blue
    let b = if temp >= 66.0 {
        1.0
    } else if temp <= 19.0 {
        0.0
    } else {
        let x = temp - 10.0;
        (138.5177312231 * x.ln() - 305.0447927307) / 255.0
    };
    let b = b.clamp(0.0, 1.0);

    [r, g, b]
}

// ─── Sky Atmosphere Enhancements ──────────────────────────────────────────

/// Dynamic atmosphere lighting type.
///
/// Maps to CesiumJS `DynamicAtmosphereLightingType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DynamicAtmosphereLighting {
    /// Use the sun position for lighting.
    #[default]
    Sun,
    /// Use the moon position for lighting.
    Moon,
    /// Treat the light as always directly overhead (no dynamic lighting).
    None,
}

impl DynamicAtmosphereLighting {
    /// Returns the enum value used in shader uniforms.
    pub fn to_shader_value(&self) -> f64 {
        match self {
            Self::Sun => 1.0,
            Self::Moon => 2.0,
            Self::None => 0.0,
        }
    }
}

/// Hue-Saturation-Brightness shift for atmosphere rendering.
///
/// Maps to CesiumJS SkyAtmosphere `hueShift`, `saturationShift`, `brightnessShift`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HsbShift {
    /// Hue shift (0.0 = no shift, 1.0 = full rotation).
    pub hue: f64,
    /// Saturation shift (-1.0 = monochrome, 0.0 = no shift).
    pub saturation: f64,
    /// Brightness shift (-1.0 = complete darkness, 0.0 = no shift).
    pub brightness: f64,
}

impl HsbShift {
    /// Applies the HSB shift to an RGB color.
    ///
    /// Converts to HSB, applies shifts, converts back.
    pub fn apply(&self, color: [f64; 3]) -> [f64; 3] {
        if self.hue == 0.0 && self.saturation == 0.0 && self.brightness == 0.0 {
            return color;
        }

        let (mut h, mut s, mut b) = rgb_to_hsb(color[0], color[1], color[2]);

        // Apply hue shift (wrapping)
        h = (h + self.hue) % 1.0;
        if h < 0.0 {
            h += 1.0;
        }

        // Apply saturation shift
        s = (s + self.saturation).clamp(0.0, 1.0);

        // Apply brightness shift
        b = (b + self.brightness).clamp(0.0, 1.0);

        hsb_to_rgb(h, s, b)
    }
}

/// Enhanced sky atmosphere parameters.
///
/// Extends `AtmosphereParameters` with CesiumJS SkyAtmosphere features.
#[derive(Debug, Clone)]
pub struct SkyAtmosphereConfig {
    /// Whether the atmosphere is shown.
    pub show: bool,
    /// Compute atmosphere per-fragment instead of per-vertex.
    pub per_fragment_atmosphere: bool,
    /// The intensity of the light used for computing sky atmosphere color.
    pub light_intensity: f64,
    /// Rayleigh scattering coefficient [R, G, B].
    pub rayleigh_coefficient: DVec3,
    /// Mie scattering coefficient [R, G, B].
    pub mie_coefficient: DVec3,
    /// Rayleigh scale height (meters).
    pub rayleigh_scale_height: f64,
    /// Mie scale height (meters).
    pub mie_scale_height: f64,
    /// Mie anisotropy (g parameter, -1..1).
    pub mie_anisotropy: f64,
    /// HSB shift for atmosphere color.
    pub hsb_shift: HsbShift,
    /// Dynamic lighting type.
    pub dynamic_lighting: DynamicAtmosphereLighting,
    /// Outer ellipsoid scale factor (atmosphere extends beyond surface).
    pub outer_ellipsoid_scale: f64,
    /// Inner radius (Earth surface, meters).
    pub inner_radius: f64,
}

impl Default for SkyAtmosphereConfig {
    fn default() -> Self {
        Self {
            show: true,
            per_fragment_atmosphere: false,
            light_intensity: 50.0,
            rayleigh_coefficient: DVec3::new(5.5e-6, 13.0e-6, 28.4e-6),
            mie_coefficient: DVec3::new(21e-6, 21e-6, 21e-6),
            rayleigh_scale_height: 10000.0,
            mie_scale_height: 3200.0,
            mie_anisotropy: 0.9,
            hsb_shift: HsbShift::default(),
            dynamic_lighting: DynamicAtmosphereLighting::Sun,
            outer_ellipsoid_scale: 1.025,
            inner_radius: 6378137.0, // WGS84 equatorial radius
        }
    }
}

impl SkyAtmosphereConfig {
    /// Computes the outer radius (atmosphere boundary).
    pub fn outer_radius(&self) -> f64 {
        self.inner_radius * self.outer_ellipsoid_scale
    }

    /// Computes the atmosphere color for a given view/sun configuration.
    ///
    /// # Arguments
    /// * `view_direction` - Normalized view direction from camera
    /// * `sun_direction` - Normalized direction to the sun
    /// * `camera_height` - Camera height above surface (meters)
    pub fn compute_color(
        &self,
        view_direction: DVec3,
        sun_direction: DVec3,
        camera_height: f64,
    ) -> [f64; 3] {
        if !self.show {
            return [0.0; 3];
        }

        let cos_theta = view_direction.dot(sun_direction);

        // Phase functions
        let rayleigh_p = rayleigh_phase_fn(cos_theta);
        let mie_p = mie_phase_fn(cos_theta, self.mie_anisotropy);

        // Density at camera height
        let height = camera_height.max(0.0);
        let rayleigh_density = (-height / self.rayleigh_scale_height).exp();
        let mie_density = (-height / self.mie_scale_height).exp();

        // Optical depth
        let path_length = self.outer_radius() - self.inner_radius;

        let mut color = [0.0f64; 3];
        let rayleigh = [self.rayleigh_coefficient.x, self.rayleigh_coefficient.y, self.rayleigh_coefficient.z];
        let mie = [self.mie_coefficient.x, self.mie_coefficient.y, self.mie_coefficient.z];
        for (c, (beta_r, beta_m)) in color.iter_mut().zip(rayleigh.iter().zip(mie.iter())) {
            let rayleigh_val = beta_r * rayleigh_density * rayleigh_p * path_length;
            let mie_val = beta_m * mie_density * mie_p * path_length;
            *c = (rayleigh_val + mie_val) * self.light_intensity;
        }

        // Apply HSB shift
        self.hsb_shift.apply(color)
    }

    /// Returns the radii and dynamic atmosphere color uniform vector.
    ///
    /// Maps to CesiumJS `u_radiiAndDynamicAtmosphereColor`.
    pub fn radii_and_dynamic_color(&self) -> DVec3 {
        DVec3::new(
            self.outer_radius(),
            self.inner_radius,
            self.dynamic_lighting.to_shader_value(),
        )
    }
}

/// Rayleigh phase function.
fn rayleigh_phase_fn(cos_theta: f64) -> f64 {
    3.0 / (16.0 * std::f64::consts::PI) * (1.0 + cos_theta * cos_theta)
}

/// Henyey-Greenstein (Mie) phase function.
fn mie_phase_fn(cos_theta: f64, g: f64) -> f64 {
    let g2 = g * g;
    let num = (1.0 - g2) * (1.0 + cos_theta * cos_theta);
    let denom = (2.0 + g2) * (1.0 + g2 - 2.0 * g * cos_theta).powf(1.5);
    num / (4.0 * std::f64::consts::PI * denom)
}

// ─── Sky Box TEME Frame ─────────────────────────────────────────────────────

/// Sky box with TEME (True Equator Mean Equinox) frame support.
///
/// Maps to CesiumJS `SkyBox` which uses TEME axes for star rendering.
#[derive(Debug, Clone)]
pub struct SkyBoxState {
    /// Whether the sky box is shown.
    pub show: bool,
    /// Source URIs for the 6 cube map faces [+X, -X, +Y, -Y, +Z, -Z].
    pub sources: [Option<String>; 6],
    /// Rotation angle around Z axis (radians) for TEME alignment.
    pub teme_rotation: f64,
}

impl Default for SkyBoxState {
    fn default() -> Self {
        Self {
            show: true,
            sources: [None, None, None, None, None, None],
            teme_rotation: 0.0,
        }
    }
}

impl SkyBoxState {
    /// Computes the TEME-to-ECEF rotation matrix for a given GMST angle.
    ///
    /// The sky box is defined in TEME axes and must be rotated to align
    /// with the ECEF frame for rendering.
    pub fn teme_to_ecef_rotation(&self, gmst: f64) -> [[f64; 3]; 3] {
        let angle = gmst + self.teme_rotation;
        let cos_a = angle.cos();
        let sin_a = angle.sin();

        // Rotation around Z axis
        [
            [cos_a, sin_a, 0.0],
            [-sin_a, cos_a, 0.0],
            [0.0, 0.0, 1.0],
        ]
    }

    /// Transforms a TEME direction to ECEF.
    pub fn teme_to_ecef(&self, teme_dir: DVec3, gmst: f64) -> DVec3 {
        let rot = self.teme_to_ecef_rotation(gmst);
        DVec3::new(
            rot[0][0] * teme_dir.x + rot[0][1] * teme_dir.y + rot[0][2] * teme_dir.z,
            rot[1][0] * teme_dir.x + rot[1][1] * teme_dir.y + rot[1][2] * teme_dir.z,
            rot[2][0] * teme_dir.x + rot[2][1] * teme_dir.y + rot[2][2] * teme_dir.z,
        )
    }

    /// Returns whether all 6 face sources are defined.
    pub fn is_complete(&self) -> bool {
        self.sources.iter().all(|s| s.is_some())
    }
}

// ─── HSB Conversion Utilities ───────────────────────────────────────────────

/// Converts RGB (0..1) to HSB (H: 0..1, S: 0..1, B: 0..1).
fn rgb_to_hsb(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        ((g - b) / delta) % 6.0 / 6.0
    } else if max == g {
        ((b - r) / delta + 2.0) / 6.0
    } else {
        ((r - g) / delta + 4.0) / 6.0
    };
    let h = if h < 0.0 { h + 1.0 } else { h };

    let s = if max == 0.0 { 0.0 } else { delta / max };
    let brightness = max;

    (h, s, brightness)
}

/// Converts HSB (H: 0..1, S: 0..1, B: 0..1) to RGB (0..1).
fn hsb_to_rgb(h: f64, s: f64, b: f64) -> [f64; 3] {
    if s == 0.0 {
        return [b, b, b];
    }

    let h6 = h * 6.0;
    let i = h6.floor() as i32;
    let f = h6 - i as f64;
    let p = b * (1.0 - s);
    let q = b * (1.0 - s * f);
    let t = b * (1.0 - s * (1.0 - f));

    match i % 6 {
        0 => [b, t, p],
        1 => [q, b, p],
        2 => [p, b, t],
        3 => [p, q, b],
        4 => [t, p, b],
        _ => [b, p, q],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    // ─── Star tests ─────────────────────────────────────────────────────

    #[test]
    fn test_star_direction_normalized() {
        let star = Star::from_degrees(101.287, -16.716, -1.46);
        let dir = star.direction();
        assert!((dir.length() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_star_direction_poles() {
        // Star at north celestial pole
        let star = Star {
            right_ascension: 0.0,
            declination: PI / 2.0,
            magnitude: 2.0,
            color_temperature: 6500.0,
        };
        let dir = star.direction();
        assert!((dir.z - 1.0).abs() < 1e-10);
        assert!(dir.x.abs() < 1e-10);
        assert!(dir.y.abs() < 1e-10);
    }

    #[test]
    fn test_star_brightness_pogson() {
        let bright = Star { magnitude: 0.0, ..Star::from_degrees(0.0, 0.0, 0.0) };
        let dim = Star { magnitude: 5.0, ..Star::from_degrees(0.0, 0.0, 5.0) };

        // Magnitude 0 → brightness 1.0
        assert!((bright.brightness() - 1.0).abs() < 1e-10);
        // Magnitude 5 → ~0.01
        assert!(dim.brightness() < 0.02);
        assert!(dim.brightness() > 0.005);
    }

    #[test]
    fn test_star_spectral_color_hot() {
        // Hot blue star (20000K)
        let star = Star { color_temperature: 20000.0, ..Star::from_degrees(0.0, 0.0, 0.0) };
        let color = star.spectral_color();
        // Blue should dominate
        assert!(color[2] > color[0]);
    }

    #[test]
    fn test_star_spectral_color_cool() {
        // Cool red star (3000K)
        let star = Star { color_temperature: 3000.0, ..Star::from_degrees(0.0, 0.0, 0.0) };
        let color = star.spectral_color();
        // Red should dominate
        assert!(color[0] > color[2]);
    }

    #[test]
    fn test_star_sphere_builtin_catalog() {
        let sphere = StarSphere::with_builtin_catalog();
        assert_eq!(sphere.star_count(), 20);
        assert!(sphere.show);
    }

    #[test]
    fn test_star_sphere_visible_filter() {
        let mut sphere = StarSphere::default();
        sphere.minimum_magnitude = 0.0;
        sphere.maximum_magnitude = 2.0;
        sphere.add_star(Star::from_degrees(0.0, 0.0, -1.0)); // Too bright (below min)
        sphere.add_star(Star::from_degrees(10.0, 10.0, 1.0)); // Visible
        sphere.add_star(Star::from_degrees(20.0, 20.0, 5.0)); // Too dim (above max)

        let visible: Vec<_> = sphere.visible_stars().collect();
        assert_eq!(visible.len(), 1);
        assert!((visible[0].magnitude - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_star_point_size() {
        let sphere = StarSphere {
            minimum_magnitude: 0.0,
            maximum_magnitude: 6.0,
            base_point_size: 4.0,
            ..Default::default()
        };

        let bright = Star::from_degrees(0.0, 0.0, 0.0);
        let dim = Star::from_degrees(0.0, 0.0, 6.0);

        let bright_size = sphere.star_point_size(&bright);
        let dim_size = sphere.star_point_size(&dim);

        // Brighter stars should be larger
        assert!(bright_size > dim_size);
        assert!((bright_size - 4.0).abs() < 1e-10); // Full base size
    }

    #[test]
    fn test_star_render_color() {
        let sphere = StarSphere {
            brightness_multiplier: 2.0,
            ..Default::default()
        };
        let star = Star { magnitude: 0.0, color_temperature: 6500.0, ..Star::from_degrees(0.0, 0.0, 0.0) };
        let color = sphere.star_render_color(&star);

        // brightness = 10^(-0.4*0) * 2.0 = 2.0
        // All channels should be > 0
        assert!(color[0] > 0.0);
        assert!(color[1] > 0.0);
        assert!(color[2] > 0.0);
    }

    // ─── Sky Atmosphere tests ───────────────────────────────────────────

    #[test]
    fn test_dynamic_lighting_values() {
        assert_eq!(DynamicAtmosphereLighting::Sun.to_shader_value(), 1.0);
        assert_eq!(DynamicAtmosphereLighting::Moon.to_shader_value(), 2.0);
        assert_eq!(DynamicAtmosphereLighting::None.to_shader_value(), 0.0);
    }

    #[test]
    fn test_hsb_shift_noop() {
        let shift = HsbShift::default();
        let color = [0.5, 0.3, 0.8];
        let result = shift.apply(color);
        assert!((result[0] - color[0]).abs() < 1e-10);
        assert!((result[1] - color[1]).abs() < 1e-10);
        assert!((result[2] - color[2]).abs() < 1e-10);
    }

    #[test]
    fn test_hsb_shift_brightness_down() {
        let shift = HsbShift { brightness: -0.5, ..Default::default() };
        let color = [1.0, 0.0, 0.0]; // Pure red, B=1.0
        let result = shift.apply(color);
        // Brightness should decrease
        assert!(result[0] < 1.0);
    }

    #[test]
    fn test_hsb_shift_saturation_zero() {
        let shift = HsbShift { saturation: -1.0, ..Default::default() };
        let color = [1.0, 0.0, 0.0]; // Pure red, S=1.0
        let result = shift.apply(color);
        // Should become grayscale (all channels equal)
        assert!((result[0] - result[1]).abs() < 1e-6);
        assert!((result[1] - result[2]).abs() < 1e-6);
    }

    #[test]
    fn test_sky_atmosphere_config_defaults() {
        let config = SkyAtmosphereConfig::default();
        assert!(config.show);
        assert!(!config.per_fragment_atmosphere);
        assert!((config.light_intensity - 50.0).abs() < 1e-10);
        assert!((config.mie_anisotropy - 0.9).abs() < 1e-10);
        assert!((config.outer_ellipsoid_scale - 1.025).abs() < 1e-10);
    }

    #[test]
    fn test_sky_atmosphere_outer_radius() {
        let config = SkyAtmosphereConfig::default();
        let outer = config.outer_radius();
        assert!((outer - 6378137.0 * 1.025).abs() < 1.0);
    }

    #[test]
    fn test_sky_atmosphere_compute_color() {
        let config = SkyAtmosphereConfig::default();
        let view = DVec3::new(0.0, 0.0, 1.0);
        let sun = DVec3::new(0.0, 0.0, 1.0);

        let color = config.compute_color(view, sun, 0.0);

        // Should produce non-zero color
        assert!(color[0] > 0.0 || color[1] > 0.0 || color[2] > 0.0);
    }

    #[test]
    fn test_sky_atmosphere_hidden() {
        let config = SkyAtmosphereConfig { show: false, ..Default::default() };
        let color = config.compute_color(DVec3::Z, DVec3::Z, 0.0);
        assert_eq!(color, [0.0; 3]);
    }

    #[test]
    fn test_radii_and_dynamic_color() {
        let config = SkyAtmosphereConfig::default();
        let v = config.radii_and_dynamic_color();
        assert!((v.x - config.outer_radius()).abs() < 1e-6);
        assert!((v.y - config.inner_radius).abs() < 1e-6);
        assert!((v.z - 1.0).abs() < 1e-10); // Sun
    }

    // ─── Sky Box tests ──────────────────────────────────────────────────

    #[test]
    fn test_sky_box_teme_rotation() {
        let sky_box = SkyBoxState::default();
        let rot = sky_box.teme_to_ecef_rotation(0.0);

        // At GMST=0, rotation should be identity
        assert!((rot[0][0] - 1.0).abs() < 1e-10);
        assert!((rot[1][1] - 1.0).abs() < 1e-10);
        assert!((rot[2][2] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_sky_box_teme_to_ecef() {
        let sky_box = SkyBoxState::default();
        let dir = DVec3::new(1.0, 0.0, 0.0);

        // At GMST=0, should be unchanged
        let ecef = sky_box.teme_to_ecef(dir, 0.0);
        assert!((ecef - dir).length() < 1e-10);

        // At GMST=π/2, X should rotate to -Y
        let ecef_90 = sky_box.teme_to_ecef(dir, PI / 2.0);
        assert!(ecef_90.x.abs() < 1e-10);
        assert!((ecef_90.y - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_sky_box_is_complete() {
        let mut sky_box = SkyBoxState::default();
        assert!(!sky_box.is_complete());

        sky_box.sources = [
            Some("px.png".into()), Some("nx.png".into()),
            Some("py.png".into()), Some("ny.png".into()),
            Some("pz.png".into()), Some("nz.png".into()),
        ];
        assert!(sky_box.is_complete());
    }

    // ─── HSB conversion tests ───────────────────────────────────────────

    #[test]
    fn test_rgb_hsb_roundtrip() {
        let colors = [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.5, 0.3, 0.8],
            [1.0, 1.0, 1.0],
            [0.0, 0.0, 0.0],
        ];

        for c in &colors {
            let (h, s, b) = rgb_to_hsb(c[0], c[1], c[2]);
            let rgb = hsb_to_rgb(h, s, b);
            assert!((rgb[0] - c[0]).abs() < 1e-6, "R mismatch for {:?}", c);
            assert!((rgb[1] - c[1]).abs() < 1e-6, "G mismatch for {:?}", c);
            assert!((rgb[2] - c[2]).abs() < 1e-6, "B mismatch for {:?}", c);
        }
    }

    #[test]
    fn test_color_temperature_white() {
        // ~6500K should be roughly white
        let color = color_from_temperature(6500.0);
        assert!(color[0] > 0.9);
        assert!(color[1] > 0.9);
        assert!(color[2] > 0.9);
    }
}
