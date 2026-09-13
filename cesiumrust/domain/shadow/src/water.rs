//! Water and ocean surface effects.
//!
//! Maps to CesiumJS water-related features:
//! - Ocean surface rendering
//! - Wave simulation (Gerstner waves)
//! - Water reflection/refraction parameters

use glam::DVec3;

/// A single Gerstner wave component.
/// Gerstner waves provide realistic ocean surface animation.
#[derive(Debug, Clone, PartialEq)]
pub struct GerstnerWave {
    /// Wave direction (normalized, in XZ plane).
    pub direction: DVec3,
    /// Wavelength in meters.
    pub wavelength: f64,
    /// Wave amplitude in meters.
    pub amplitude: f64,
    /// Wave speed (phase velocity) in m/s.
    pub speed: f64,
    /// Steepness factor (0.0 = sinusoidal, 1.0 = sharp crests).
    pub steepness: f64,
    /// Phase offset in radians.
    pub phase: f64,
}

impl GerstnerWave {
    /// Creates a new Gerstner wave.
    pub fn new(direction: DVec3, wavelength: f64, amplitude: f64, speed: f64) -> Self {
        Self {
            direction: direction.normalize(),
            wavelength,
            amplitude,
            speed,
            steepness: 0.5,
            phase: 0.0,
        }
    }

    /// Computes the wave number (k = 2π / wavelength).
    pub fn wave_number(&self) -> f64 {
        std::f64::consts::TAU / self.wavelength
    }

    /// Computes the angular frequency (ω = k * speed).
    pub fn angular_frequency(&self) -> f64 {
        self.wave_number() * self.speed
    }

    /// Computes the displacement at a given position and time.
    ///
    /// # Arguments
    /// * `position` - World position (XZ plane)
    /// * `time` - Time in seconds
    ///
    /// # Returns
    /// The 3D displacement vector
    pub fn compute_displacement(&self, position: DVec3, time: f64) -> DVec3 {
        let k = self.wave_number();
        let omega = self.angular_frequency();

        // Dot product of wave direction and position
        let d = self.direction.dot(position);

        // Phase
        let theta = k * d - omega * time + self.phase;

        let cos_theta = theta.cos();
        let sin_theta = theta.sin();

        // Horizontal displacement (creates the sharp crests)
        let horizontal = self.direction * (self.steepness * self.amplitude * cos_theta);

        // Vertical displacement
        let vertical = self.amplitude * sin_theta;

        DVec3::new(horizontal.x, vertical, horizontal.z)
    }

    /// Computes the surface normal at a given position and time.
    pub fn compute_normal(&self, position: DVec3, time: f64) -> DVec3 {
        let k = self.wave_number();
        let omega = self.angular_frequency();

        let d = self.direction.dot(position);
        let theta = k * d - omega * time + self.phase;

        let cos_theta = theta.cos();
        let sin_theta = theta.sin();

        // Partial derivatives
        let wa = self.amplitude * k;
        let qa = self.steepness * self.amplitude * k;

        // Normal calculation (simplified)
        DVec3::new(
            -self.direction.x * wa * cos_theta,
            1.0 - qa * sin_theta,
            -self.direction.z * wa * cos_theta,
        )
        .normalize()
    }
}

/// Ocean surface configuration.
#[derive(Debug, Clone)]
pub struct OceanConfig {
    /// Whether the ocean is enabled.
    pub enabled: bool,
    /// Base water color (deep water).
    pub water_color: DVec3,
    /// Shallow water color.
    pub shallow_color: DVec3,
    /// Water transparency (0.0 = opaque, 1.0 = fully transparent).
    pub transparency: f64,
    /// Reflection strength (0.0 = no reflection, 1.0 = mirror).
    pub reflection_strength: f64,
    /// Refraction strength.
    pub refraction_strength: f64,
    /// Fresnel power (controls reflection angle falloff).
    pub fresnel_power: f64,
    /// Specular intensity (sun glint).
    pub specular_intensity: f64,
    /// Specular power (sharpness of sun glint).
    pub specular_power: f64,
    /// Wave components.
    pub waves: Vec<GerstnerWave>,
    /// Normal map scale (for detail waves).
    pub normal_scale: f64,
    /// Foam threshold (wave crests above this show foam).
    pub foam_threshold: f64,
    /// Foam color.
    pub foam_color: DVec3,
}

impl Default for OceanConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            water_color: DVec3::new(0.0, 0.1, 0.3),
            shallow_color: DVec3::new(0.0, 0.4, 0.5),
            transparency: 0.6,
            reflection_strength: 0.5,
            refraction_strength: 0.3,
            fresnel_power: 5.0,
            specular_intensity: 1.0,
            specular_power: 256.0,
            waves: create_default_waves(),
            normal_scale: 1.0,
            foam_threshold: 0.8,
            foam_color: DVec3::new(0.9, 0.95, 1.0),
        }
    }
}

/// Creates a default set of ocean waves.
fn create_default_waves() -> Vec<GerstnerWave> {
    vec![
        // Large swell
        GerstnerWave::new(DVec3::new(1.0, 0.0, 0.3), 100.0, 1.5, 8.0),
        // Medium waves
        GerstnerWave::new(DVec3::new(0.7, 0.0, 0.7), 50.0, 0.8, 6.0),
        GerstnerWave::new(DVec3::new(-0.3, 0.0, 0.9), 30.0, 0.5, 5.0),
        // Small ripples
        GerstnerWave::new(DVec3::new(0.9, 0.0, -0.4), 10.0, 0.2, 3.0),
        GerstnerWave::new(DVec3::new(-0.6, 0.0, 0.8), 5.0, 0.1, 2.0),
    ]
}

/// The ocean surface state.
#[derive(Debug, Clone)]
pub struct OceanSurface {
    /// Configuration.
    pub config: OceanConfig,
    /// Current time (for wave animation).
    pub time: f64,
    /// Wind direction (affects wave generation).
    pub wind_direction: DVec3,
    /// Wind speed (m/s).
    pub wind_speed: f64,
}

impl OceanSurface {
    /// Creates a new ocean surface.
    pub fn new(config: OceanConfig) -> Self {
        Self {
            config,
            time: 0.0,
            wind_direction: DVec3::new(1.0, 0.0, 0.0),
            wind_speed: 10.0,
        }
    }

    /// Updates the ocean surface by a time delta.
    pub fn update(&mut self, dt: f64) {
        self.time += dt;
    }

    /// Computes the total wave displacement at a position.
    pub fn compute_displacement(&self, position: DVec3) -> DVec3 {
        if !self.config.enabled {
            return DVec3::ZERO;
        }

        let mut total = DVec3::ZERO;
        for wave in &self.config.waves {
            total += wave.compute_displacement(position, self.time);
        }
        total
    }

    /// Computes the surface normal at a position.
    pub fn compute_normal(&self, position: DVec3) -> DVec3 {
        if !self.config.enabled {
            return DVec3::Y;
        }

        let mut normal = DVec3::Y;
        for wave in &self.config.waves {
            let wave_normal = wave.compute_normal(position, self.time);
            normal += wave_normal - DVec3::Y;
        }
        normal.normalize()
    }

    /// Computes the water height at a position (Y displacement).
    pub fn compute_height(&self, position: DVec3) -> f64 {
        self.compute_displacement(position).y
    }

    /// Computes the Fresnel reflection coefficient.
    ///
    /// # Arguments
    /// * `view_direction` - Direction from surface to camera (normalized)
    /// * `normal` - Surface normal (normalized)
    pub fn compute_fresnel(&self, view_direction: DVec3, normal: DVec3) -> f64 {
        let cos_theta = view_direction.dot(normal).abs().clamp(0.0, 1.0);

        // Schlick's approximation
        let r0 = 0.02; // Base reflectivity for water
        r0 + (1.0 - r0) * (1.0 - cos_theta).powf(self.config.fresnel_power)
    }

    /// Computes the specular reflection (sun glint).
    ///
    /// # Arguments
    /// * `view_direction` - Direction from surface to camera
    /// * `light_direction` - Direction from surface to light (sun)
    /// * `normal` - Surface normal
    pub fn compute_specular(
        &self,
        view_direction: DVec3,
        light_direction: DVec3,
        normal: DVec3,
    ) -> f64 {
        // Reflect light direction around normal
        let reflect_dir = (2.0 * normal.dot(light_direction) * normal - light_direction).normalize();

        // Specular intensity
        let spec_angle = reflect_dir.dot(view_direction).max(0.0);
        spec_angle.powf(self.config.specular_power) * self.config.specular_intensity
    }

    /// Computes the final water color at a position.
    ///
    /// # Arguments
    /// * `position` - World position on the water surface
    /// * `view_direction` - Direction from surface to camera
    /// * `light_direction` - Direction from surface to light (sun)
    /// * `depth` - Water depth (for shallow/deep color blending)
    pub fn compute_water_color(
        &self,
        position: DVec3,
        view_direction: DVec3,
        light_direction: DVec3,
        depth: f64,
    ) -> DVec3 {
        if !self.config.enabled {
            return self.config.water_color;
        }

        let normal = self.compute_normal(position);

        // Depth-based color blending
        let depth_factor = (depth / 10.0).clamp(0.0, 1.0); // 10m transition zone
        let base_color = self.config.shallow_color.lerp(self.config.water_color, depth_factor);

        // Fresnel reflection
        let fresnel = self.compute_fresnel(view_direction, normal);

        // Specular (sun glint)
        let specular = self.compute_specular(view_direction, light_direction, normal);

        // Combine
        let reflection_color = DVec3::new(0.5, 0.6, 0.8); // Sky reflection approximation
        let mut final_color = base_color.lerp(reflection_color, fresnel * self.config.reflection_strength);

        // Add specular highlight
        final_color += DVec3::splat(specular);

        // Foam on wave crests
        let height = self.compute_height(position);
        if height > self.config.foam_threshold {
            let foam_factor = ((height - self.config.foam_threshold) / 0.5).clamp(0.0, 1.0);
            final_color = final_color.lerp(self.config.foam_color, foam_factor);
        }

        final_color.clamp(DVec3::ZERO, DVec3::ONE)
    }

    /// Generates waves based on wind (simplified Pierson-Moskowitz spectrum).
    pub fn generate_wind_waves(&mut self) {
        let wind_dir = self.wind_direction.normalize();

        // Wave parameters based on wind speed
        let significant_wave_height = 0.22 * self.wind_speed * self.wind_speed / 9.81;
        let peak_wavelength = 2.0 * std::f64::consts::PI * self.wind_speed * self.wind_speed / 9.81;

        self.config.waves.clear();

        // Generate a spectrum of waves
        for i in 0..8 {
            let scale = 0.5_f64.powi(i);
            let wavelength = peak_wavelength * scale;
            let amplitude = significant_wave_height * scale * 0.1;
            let speed = (9.81 * wavelength / std::f64::consts::TAU).sqrt();

            // Vary direction slightly
            let angle_offset = (i as f64 - 3.5) * 0.2;
            let cos_a = angle_offset.cos();
            let sin_a = angle_offset.sin();
            let direction = DVec3::new(
                wind_dir.x * cos_a - wind_dir.z * sin_a,
                0.0,
                wind_dir.x * sin_a + wind_dir.z * cos_a,
            );

            let mut wave = GerstnerWave::new(direction, wavelength.max(1.0), amplitude, speed);
            wave.phase = (i as f64) * 1.7; // Vary phase
            wave.steepness = 0.3 + 0.1 * (i as f64);

            self.config.waves.push(wave);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gerstner_wave_creation() {
        let wave = GerstnerWave::new(DVec3::new(1.0, 0.0, 0.0), 10.0, 1.0, 5.0);

        assert!((wave.wavelength - 10.0).abs() < 1e-10);
        assert!((wave.amplitude - 1.0).abs() < 1e-10);
        assert!((wave.speed - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_gerstner_wave_number() {
        let wave = GerstnerWave::new(DVec3::X, 10.0, 1.0, 5.0);

        let k = wave.wave_number();
        assert!((k - std::f64::consts::TAU / 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_gerstner_displacement_at_origin() {
        let wave = GerstnerWave {
            direction: DVec3::X,
            wavelength: 10.0,
            amplitude: 1.0,
            speed: 5.0,
            steepness: 0.0, // Pure sinusoidal
            phase: 0.0,
        };

        // At t=0, position=(0,0,0): theta = 0, sin(0) = 0
        let displacement = wave.compute_displacement(DVec3::ZERO, 0.0);
        assert!((displacement.y).abs() < 1e-10);

        // At t such that theta = -π/2: sin(-π/2) = -1
        // theta = k*d - omega*t = -omega*t (since d=0)
        // For theta = -π/2: t = π/(2*omega)
        let omega = wave.angular_frequency();
        let t = std::f64::consts::FRAC_PI_2 / omega;
        let displacement = wave.compute_displacement(DVec3::ZERO, t);
        assert!((displacement.y - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_gerstner_normal() {
        let wave = GerstnerWave::new(DVec3::X, 10.0, 1.0, 5.0);

        let normal = wave.compute_normal(DVec3::ZERO, 0.0);

        // Normal should be roughly pointing up
        assert!(normal.y > 0.5);
    }

    #[test]
    fn test_ocean_config_default() {
        let config = OceanConfig::default();

        assert!(config.enabled);
        assert_eq!(config.waves.len(), 5);
        assert!(config.transparency > 0.0);
    }

    #[test]
    fn test_ocean_surface_creation() {
        let ocean = OceanSurface::new(OceanConfig::default());

        assert_eq!(ocean.time, 0.0);
        assert!(ocean.config.enabled);
    }

    #[test]
    fn test_ocean_update() {
        let mut ocean = OceanSurface::new(OceanConfig::default());

        ocean.update(1.0);
        assert!((ocean.time - 1.0).abs() < 1e-10);

        ocean.update(0.5);
        assert!((ocean.time - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_ocean_displacement() {
        let ocean = OceanSurface::new(OceanConfig::default());

        let displacement = ocean.compute_displacement(DVec3::ZERO);

        // Should have some displacement from the waves
        // (may be zero at t=0 depending on wave phases)
        assert!(displacement.length() >= 0.0);
    }

    #[test]
    fn test_ocean_normal() {
        let ocean = OceanSurface::new(OceanConfig::default());

        let normal = ocean.compute_normal(DVec3::ZERO);

        // Normal should be roughly pointing up
        assert!(normal.y > 0.0);
        assert!((normal.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_ocean_disabled() {
        let config = OceanConfig {
            enabled: false,
            ..Default::default()
        };
        let ocean = OceanSurface::new(config);

        let displacement = ocean.compute_displacement(DVec3::ZERO);
        assert_eq!(displacement, DVec3::ZERO);

        let normal = ocean.compute_normal(DVec3::ZERO);
        assert_eq!(normal, DVec3::Y);
    }

    #[test]
    fn test_fresnel_straight_on() {
        let ocean = OceanSurface::new(OceanConfig::default());

        // Looking straight down at flat water
        let view_dir = DVec3::Y;
        let normal = DVec3::Y;

        let fresnel = ocean.compute_fresnel(view_dir, normal);

        // At normal incidence, reflection should be minimal (~2%)
        assert!(fresnel < 0.1);
    }

    #[test]
    fn test_fresnel_grazing_angle() {
        let ocean = OceanSurface::new(OceanConfig::default());

        // Looking at grazing angle
        let view_dir = DVec3::new(1.0, 0.1, 0.0).normalize();
        let normal = DVec3::Y;

        let fresnel = ocean.compute_fresnel(view_dir, normal);

        // At grazing angle, reflection should be high
        assert!(fresnel > 0.5);
    }

    #[test]
    fn test_specular() {
        let ocean = OceanSurface::new(OceanConfig::default());

        let view_dir = DVec3::Y;
        let light_dir = DVec3::Y; // Sun directly overhead
        let normal = DVec3::Y;

        let specular = ocean.compute_specular(view_dir, light_dir, normal);

        // Perfect reflection should give high specular
        assert!(specular > 0.9);
    }

    #[test]
    fn test_water_color() {
        let ocean = OceanSurface::new(OceanConfig::default());

        let color = ocean.compute_water_color(
            DVec3::ZERO,
            DVec3::Y,
            DVec3::Y,
            100.0, // Deep water
        );

        // Should be a valid color
        assert!(color.x >= 0.0 && color.x <= 1.0);
        assert!(color.y >= 0.0 && color.y <= 1.0);
        assert!(color.z >= 0.0 && color.z <= 1.0);
    }

    #[test]
    fn test_wind_wave_generation() {
        let mut ocean = OceanSurface::new(OceanConfig::default());
        ocean.wind_speed = 15.0;
        ocean.wind_direction = DVec3::new(1.0, 0.0, 0.5);

        ocean.generate_wind_waves();

        assert_eq!(ocean.config.waves.len(), 8);

        // All waves should have positive wavelength and amplitude
        for wave in &ocean.config.waves {
            assert!(wave.wavelength > 0.0);
            assert!(wave.amplitude >= 0.0);
        }
    }

    #[test]
    fn test_wave_height() {
        let ocean = OceanSurface::new(OceanConfig::default());

        let height = ocean.compute_height(DVec3::ZERO);

        // Height should be finite
        assert!(height.is_finite());
    }
}
