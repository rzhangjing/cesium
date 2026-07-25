//! Image-Based Lighting (IBL).
//!
//! Maps to CesiumJS `Scene/ImageBasedLighting.js`:
//! - Spherical harmonic coefficients for diffuse IBL
//! - Specular environment maps
//! - IBL factor scaling
//!
//! Domain layer — pure Rust, f64 precision.

use glam::DVec3;

/// Number of spherical harmonic coefficients (3rd order = 9 coefficients).
pub const SH_COEFFICIENT_COUNT: usize = 9;

/// Image-based lighting configuration.
///
/// Maps to CesiumJS `ImageBasedLighting`.
#[derive(Debug, Clone)]
pub struct ImageBasedLighting {
    /// Scales diffuse and specular IBL contribution.
    /// x = diffuse factor, y = specular factor. Both in [0, 1].
    pub image_based_lighting_factor: [f64; 2],
    /// Third-order spherical harmonic coefficients for diffuse IBL.
    /// 9 coefficients, each an RGB triple.
    pub spherical_harmonic_coefficients: Option<[[f64; 3]; SH_COEFFICIENT_COUNT]>,
    /// URL to a KTX2 specular environment map.
    pub specular_environment_maps: Option<String>,
    /// Whether to use default spherical harmonics.
    pub use_default_spherical_harmonics: bool,
    /// Whether to use default specular maps.
    pub use_default_specular_maps: bool,
}

impl Default for ImageBasedLighting {
    fn default() -> Self {
        Self {
            image_based_lighting_factor: [1.0, 1.0],
            spherical_harmonic_coefficients: None,
            specular_environment_maps: None,
            use_default_spherical_harmonics: false,
            use_default_specular_maps: false,
        }
    }
}

impl ImageBasedLighting {
    /// Creates a new IBL configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the IBL factor (diffuse and specular scaling).
    ///
    /// # Panics
    /// Panics if values are outside [0, 1].
    pub fn set_factor(&mut self, diffuse: f64, specular: f64) {
        assert!((0.0..=1.0).contains(&diffuse), "diffuse factor must be in [0, 1]");
        assert!((0.0..=1.0).contains(&specular), "specular factor must be in [0, 1]");
        self.image_based_lighting_factor = [diffuse, specular];
    }

    /// Sets the spherical harmonic coefficients.
    ///
    /// # Panics
    /// Panics if the array doesn't have exactly 9 coefficients.
    pub fn set_spherical_harmonics(&mut self, coefficients: [[f64; 3]; SH_COEFFICIENT_COUNT]) {
        self.spherical_harmonic_coefficients = Some(coefficients);
        self.use_default_spherical_harmonics = false;
    }

    /// Returns whether custom SH coefficients are set.
    pub fn has_spherical_harmonics(&self) -> bool {
        self.spherical_harmonic_coefficients.is_some()
    }

    /// Returns whether a specular environment map is set.
    pub fn has_specular_environment_maps(&self) -> bool {
        self.specular_environment_maps.is_some()
    }

    /// Returns whether shaders need regeneration due to IBL changes.
    pub fn needs_shader_regeneration(&self) -> bool {
        self.spherical_harmonic_coefficients.is_some() || self.specular_environment_maps.is_some()
    }

    /// Computes the diffuse IBL contribution for a given normal direction.
    ///
    /// Uses the spherical harmonic coefficients to evaluate irradiance.
    ///
    /// # Arguments
    /// * `normal` - Surface normal (normalized)
    ///
    /// # Returns
    /// Diffuse irradiance color [R, G, B], scaled by the diffuse IBL factor.
    pub fn compute_diffuse_ibl(&self, normal: DVec3) -> [f64; 3] {
        let coefficients = match &self.spherical_harmonic_coefficients {
            Some(c) => c,
            None => return [0.0; 3],
        };

        let diffuse_factor = self.image_based_lighting_factor[0];
        if diffuse_factor == 0.0 {
            return [0.0; 3];
        }

        // Evaluate spherical harmonics
        let sh = evaluate_sh(coefficients, normal);

        [sh[0] * diffuse_factor, sh[1] * diffuse_factor, sh[2] * diffuse_factor]
    }

    /// Computes the specular IBL contribution.
    ///
    /// In a full implementation, this would sample the specular environment map
    /// at the reflection direction with the appropriate mip level based on roughness.
    ///
    /// # Arguments
    /// * `reflection` - Reflection direction (normalized)
    /// * `roughness` - Surface roughness [0, 1]
    ///
    /// # Returns
    /// Specular color [R, G, B], scaled by the specular IBL factor.
    pub fn compute_specular_ibl(&self, _reflection: DVec3, _roughness: f64) -> [f64; 3] {
        let specular_factor = self.image_based_lighting_factor[1];
        if specular_factor == 0.0 {
            return [0.0; 3];
        }

        // Placeholder: in a real implementation, sample the environment map
        // For now, return a neutral environment contribution
        [0.1 * specular_factor, 0.1 * specular_factor, 0.12 * specular_factor]
    }
}

/// Evaluates 3rd-order spherical harmonics for a given direction.
///
/// The 9 SH basis functions for order 0, 1, 2:
/// - Y_0^0 = 0.282095
/// - Y_1^{-1} = 0.488603 * y
/// - Y_1^0 = 0.488603 * z
/// - Y_1^1 = 0.488603 * x
/// - Y_2^{-2} = 1.092548 * x * y
/// - Y_2^{-1} = 1.092548 * y * z
/// - Y_2^0 = 0.315392 * (3z² - 1)
/// - Y_2^1 = 1.092548 * x * z
/// - Y_2^2 = 0.546274 * (x² - y²)
fn evaluate_sh(coefficients: &[[f64; 3]; 9], direction: DVec3) -> [f64; 3] {
    let x = direction.x;
    let y = direction.y;
    let z = direction.z;

    // SH basis functions
    let basis = [
        0.282095,                        // Y_0^0
        0.488603 * y,                    // Y_1^{-1}
        0.488603 * z,                    // Y_1^0
        0.488603 * x,                    // Y_1^1
        1.092548 * x * y,               // Y_2^{-2}
        1.092548 * y * z,               // Y_2^{-1}
        0.315392 * (3.0 * z * z - 1.0), // Y_2^0
        1.092548 * x * z,               // Y_2^1
        0.546274 * (x * x - y * y),     // Y_2^2
    ];

    let mut result = [0.0f64; 3];
    for (i, b) in basis.iter().enumerate() {
        for c in 0..3 {
            result[c] += coefficients[i][c] * b;
        }
    }

    result
}

/// Default spherical harmonic coefficients for a neutral sky environment.
///
/// These approximate a simple sky/ground environment.
pub fn default_spherical_harmonics() -> [[f64; 3]; SH_COEFFICIENT_COUNT] {
    [
        [0.3, 0.3, 0.35],   // DC term (ambient)
        [0.0, 0.0, 0.0],    // Y_1^{-1}
        [0.1, 0.1, 0.15],   // Y_1^0 (sky/ground gradient)
        [0.0, 0.0, 0.0],    // Y_1^1
        [0.0, 0.0, 0.0],    // Y_2^{-2}
        [0.0, 0.0, 0.0],    // Y_2^{-1}
        [0.05, 0.05, 0.08], // Y_2^0
        [0.0, 0.0, 0.0],    // Y_2^1
        [0.0, 0.0, 0.0],    // Y_2^2
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ibl_default() {
        let ibl = ImageBasedLighting::default();
        assert_eq!(ibl.image_based_lighting_factor, [1.0, 1.0]);
        assert!(!ibl.has_spherical_harmonics());
        assert!(!ibl.has_specular_environment_maps());
    }

    #[test]
    fn test_ibl_set_factor() {
        let mut ibl = ImageBasedLighting::default();
        ibl.set_factor(0.5, 0.8);
        assert_eq!(ibl.image_based_lighting_factor, [0.5, 0.8]);
    }

    #[test]
    #[should_panic(expected = "diffuse factor must be in [0, 1]")]
    fn test_ibl_set_factor_invalid() {
        let mut ibl = ImageBasedLighting::default();
        ibl.set_factor(1.5, 0.5);
    }

    #[test]
    fn test_ibl_set_spherical_harmonics() {
        let mut ibl = ImageBasedLighting::default();
        let sh = default_spherical_harmonics();
        ibl.set_spherical_harmonics(sh);

        assert!(ibl.has_spherical_harmonics());
        assert!(ibl.needs_shader_regeneration());
    }

    #[test]
    fn test_ibl_specular_maps() {
        let mut ibl = ImageBasedLighting::default();
        assert!(!ibl.has_specular_environment_maps());

        ibl.specular_environment_maps = Some("environment.ktx2".to_string());
        assert!(ibl.has_specular_environment_maps());
    }

    #[test]
    fn test_compute_diffuse_ibl_no_coefficients() {
        let ibl = ImageBasedLighting::default();
        let result = ibl.compute_diffuse_ibl(DVec3::Y);
        assert_eq!(result, [0.0; 3]);
    }

    #[test]
    fn test_compute_diffuse_ibl_with_coefficients() {
        let mut ibl = ImageBasedLighting::default();
        ibl.set_spherical_harmonics(default_spherical_harmonics());

        let result = ibl.compute_diffuse_ibl(DVec3::Y);

        // Should produce non-zero result
        assert!(result[0] > 0.0 || result[1] > 0.0 || result[2] > 0.0);
    }

    #[test]
    fn test_compute_diffuse_ibl_zero_factor() {
        let mut ibl = ImageBasedLighting::default();
        ibl.set_spherical_harmonics(default_spherical_harmonics());
        ibl.set_factor(0.0, 1.0); // Zero diffuse

        let result = ibl.compute_diffuse_ibl(DVec3::Y);
        assert_eq!(result, [0.0; 3]);
    }

    #[test]
    fn test_compute_specular_ibl() {
        let ibl = ImageBasedLighting::default();
        let result = ibl.compute_specular_ibl(DVec3::Y, 0.5);

        // Default returns neutral contribution
        assert!(result[0] > 0.0);
    }

    #[test]
    fn test_compute_specular_ibl_zero_factor() {
        let mut ibl = ImageBasedLighting::default();
        ibl.set_factor(1.0, 0.0); // Zero specular

        let result = ibl.compute_specular_ibl(DVec3::Y, 0.5);
        assert_eq!(result, [0.0; 3]);
    }

    #[test]
    fn test_evaluate_sh_dc_only() {
        // Only DC term set
        let mut coefficients = [[0.0; 3]; 9];
        coefficients[0] = [1.0, 1.0, 1.0];

        // DC term should be constant regardless of direction
        let up = evaluate_sh(&coefficients, DVec3::Y);
        let down = evaluate_sh(&coefficients, DVec3::new(0.0, -1.0, 0.0));

        assert!((up[0] - down[0]).abs() < 1e-10);
        assert!((up[0] - 0.282095).abs() < 1e-5);
    }

    #[test]
    fn test_default_spherical_harmonics() {
        let sh = default_spherical_harmonics();
        // DC term should be the ambient contribution
        assert!(sh[0][0] > 0.0);
        assert!(sh[0][1] > 0.0);
        assert!(sh[0][2] > 0.0);
    }
}
