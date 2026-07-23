//! Post-processing effects pipeline.
//!
//! Maps to CesiumJS `Scene/PostProcessStageLibrary.js`:
//! - Bloom
//! - Ambient Occlusion
//! - Fog
//! - Tone Mapping

use glam::DVec3;

/// A post-processing stage identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PostProcessStageType {
    /// Bloom (HDR glow) effect.
    Bloom,
    /// Screen-space ambient occlusion.
    AmbientOcclusion,
    /// Distance fog.
    Fog,
    /// Tone mapping (HDR → LDR).
    ToneMapping,
    /// Color correction / grading.
    ColorCorrection,
}

/// Bloom effect parameters.
/// Maps to CesiumJS `PostProcessStageLibrary.createBloomStage()`
#[derive(Debug, Clone, PartialEq)]
pub struct BloomConfig {
    /// Whether bloom is enabled.
    pub enabled: bool,
    /// Bloom intensity (0.0 = no bloom).
    pub intensity: f64,
    /// Luminance threshold for bloom (pixels brighter than this glow).
    pub threshold: f64,
    /// Blur radius in pixels.
    pub blur_radius: f64,
    /// Number of blur passes (more = smoother but slower).
    pub blur_passes: u32,
}

impl Default for BloomConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 1.0,
            threshold: 0.8,
            blur_radius: 4.0,
            blur_passes: 4,
        }
    }
}

impl BloomConfig {
    /// Computes the bloom contribution for a given pixel luminance.
    ///
    /// Returns the bloom intensity multiplier (0.0 if below threshold).
    pub fn compute_bloom(&self, luminance: f64) -> f64 {
        if !self.enabled || luminance <= self.threshold {
            return 0.0;
        }
        let excess = luminance - self.threshold;
        excess * self.intensity
    }
}

/// Ambient Occlusion parameters.
/// Maps to CesiumJS `PostProcessStageLibrary.createAmbientOcclusionStage()`
#[derive(Debug, Clone, PartialEq)]
pub struct AmbientOcclusionConfig {
    /// Whether AO is enabled.
    pub enabled: bool,
    /// AO intensity (0.0 = no darkening, 1.0 = full darkening).
    pub intensity: f64,
    /// Sample radius in world units.
    pub sample_radius: f64,
    /// Number of samples per pixel.
    pub sample_count: u32,
    /// Bias to avoid self-occlusion artifacts.
    pub bias: f64,
    /// Length cap for AO rays.
    pub length_cap: f64,
}

impl Default for AmbientOcclusionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 3.0,
            sample_radius: 0.5,
            sample_count: 16,
            bias: 0.001,
            length_cap: 0.26,
        }
    }
}

impl AmbientOcclusionConfig {
    /// Computes the AO factor for a given occlusion ratio (0.0 = no occlusion, 1.0 = fully occluded).
    ///
    /// Returns a multiplier in [0.0, 1.0] to apply to the pixel color.
    pub fn compute_ao(&self, occlusion_ratio: f64) -> f64 {
        if !self.enabled {
            return 1.0;
        }
        let ao = 1.0 - occlusion_ratio.clamp(0.0, 1.0) * self.intensity;
        ao.clamp(0.0, 1.0)
    }
}

/// Fog effect parameters.
/// Maps to CesiumJS `Scene/Fog.js`
#[derive(Debug, Clone, PartialEq)]
pub struct FogConfig {
    /// Whether fog is enabled.
    pub enabled: bool,
    /// Fog density at the surface.
    pub density: f64,
    /// Fog color (RGB, 0-1 range).
    pub color: DVec3,
    /// Minimum visibility distance (meters).
    pub minimum_distance: f64,
    /// Maximum visibility distance (meters, fog is fully opaque beyond this).
    pub maximum_distance: f64,
    /// Whether to use screen-space error based fog density.
    pub use_sse_based_density: bool,
}

impl Default for FogConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            density: 2.0e-4,
            color: DVec3::new(0.8, 0.85, 0.9),
            minimum_distance: 100.0,
            maximum_distance: 100_000_000.0,
            use_sse_based_density: true,
        }
    }
}

impl FogConfig {
    /// Computes the fog factor for a given distance from the camera.
    ///
    /// Returns a value in [0.0, 1.0] where 0.0 = no fog, 1.0 = fully fogged.
    pub fn compute_fog_factor(&self, distance: f64) -> f64 {
        if !self.enabled {
            return 0.0;
        }

        if distance <= self.minimum_distance {
            return 0.0;
        }

        // Exponential fog: factor = 1 - exp(-density * distance)
        let fog = 1.0 - (-self.density * distance).exp();
        fog.clamp(0.0, 1.0)
    }

    /// Blends a pixel color with the fog color based on distance.
    ///
    /// # Arguments
    /// * `pixel_color` - The original pixel color (RGB)
    /// * `distance` - Distance from camera to pixel
    ///
    /// # Returns
    /// The fogged pixel color
    pub fn apply_fog(&self, pixel_color: DVec3, distance: f64) -> DVec3 {
        let factor = self.compute_fog_factor(distance);
        pixel_color.lerp(self.color, factor)
    }
}

/// Tone mapping operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToneMappingOperator {
    /// No tone mapping (linear).
    None,
    /// Reinhard tone mapping.
    Reinhard,
    /// ACES Filmic tone mapping.
    AcesFilmic,
    /// Uncharted 2 tone mapping.
    Uncharted2,
}

/// Tone mapping configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct ToneMappingConfig {
    /// The tone mapping operator to use.
    pub operator: ToneMappingOperator,
    /// Exposure value.
    pub exposure: f64,
    /// White point (for Reinhard).
    pub white_point: f64,
}

impl Default for ToneMappingConfig {
    fn default() -> Self {
        Self {
            operator: ToneMappingOperator::AcesFilmic,
            exposure: 1.0,
            white_point: 1.0,
        }
    }
}

impl ToneMappingConfig {
    /// Applies tone mapping to an HDR color value.
    ///
    /// # Arguments
    /// * `hdr_color` - The HDR color (can exceed 1.0)
    ///
    /// # Returns
    /// The tone-mapped LDR color (0.0 to 1.0)
    pub fn apply(&self, hdr_color: DVec3) -> DVec3 {
        let exposed = hdr_color * self.exposure;

        match self.operator {
            ToneMappingOperator::None => exposed,
            ToneMappingOperator::Reinhard => self.reinhard(exposed),
            ToneMappingOperator::AcesFilmic => self.aces_filmic(exposed),
            ToneMappingOperator::Uncharted2 => self.uncharted2(exposed),
        }
    }

    fn reinhard(&self, color: DVec3) -> DVec3 {
        let white_sq = self.white_point * self.white_point;
        DVec3::new(
            color.x * (1.0 + color.x / white_sq) / (1.0 + color.x),
            color.y * (1.0 + color.y / white_sq) / (1.0 + color.y),
            color.z * (1.0 + color.z / white_sq) / (1.0 + color.z),
        )
    }

    fn aces_filmic(&self, color: DVec3) -> DVec3 {
        // ACES approximation by Krzysztof Narkowicz
        const A: f64 = 2.51;
        const B: f64 = 0.03;
        const C: f64 = 2.43;
        const D: f64 = 0.59;
        const E: f64 = 0.14;

        DVec3::new(
            aces_curve(color.x, A, B, C, D, E),
            aces_curve(color.y, A, B, C, D, E),
            aces_curve(color.z, A, B, C, D, E),
        )
    }

    fn uncharted2(&self, color: DVec3) -> DVec3 {
        DVec3::new(
            uncharted2_curve(color.x),
            uncharted2_curve(color.y),
            uncharted2_curve(color.z),
        )
    }
}

fn aces_curve(x: f64, a: f64, b: f64, c: f64, d: f64, e: f64) -> f64 {
    ((x * (a * x + b)) / (x * (c * x + d) + e)).clamp(0.0, 1.0)
}

fn uncharted2_curve(x: f64) -> f64 {
    const A: f64 = 0.15;
    const B: f64 = 0.50;
    const C: f64 = 0.10;
    const D: f64 = 0.20;
    const E: f64 = 0.02;
    const F: f64 = 0.30;

    ((x * (A * x + C * B) + D * E) / (x * (A * x + B) + D * F) - E / F).clamp(0.0, 1.0)
}

/// Color correction / grading parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorCorrectionConfig {
    /// Whether color correction is enabled.
    pub enabled: bool,
    /// Brightness adjustment (-1 to 1).
    pub brightness: f64,
    /// Contrast adjustment (0 = flat, 1 = normal, 2 = high contrast).
    pub contrast: f64,
    /// Saturation adjustment (0 = grayscale, 1 = normal, 2 = oversaturated).
    pub saturation: f64,
    /// Hue rotation in radians.
    pub hue: f64,
}

impl Default for ColorCorrectionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            brightness: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            hue: 0.0,
        }
    }
}

impl ColorCorrectionConfig {
    /// Applies color correction to a pixel color.
    pub fn apply(&self, color: DVec3) -> DVec3 {
        if !self.enabled {
            return color;
        }

        let mut result = color;

        // Brightness
        result += DVec3::splat(self.brightness);

        // Contrast (around 0.5 midpoint)
        result = (result - DVec3::splat(0.5)) * self.contrast + DVec3::splat(0.5);

        // Saturation
        let luminance = 0.2126 * result.x + 0.7152 * result.y + 0.0722 * result.z;
        result = DVec3::splat(luminance).lerp(result, self.saturation);

        // Clamp to valid range
        result.clamp(DVec3::ZERO, DVec3::ONE)
    }
}

/// The complete post-processing pipeline configuration.
#[derive(Debug, Clone, Default)]
pub struct PostProcessPipeline {
    /// Bloom stage.
    pub bloom: BloomConfig,
    /// Ambient occlusion stage.
    pub ambient_occlusion: AmbientOcclusionConfig,
    /// Fog stage.
    pub fog: FogConfig,
    /// Tone mapping stage.
    pub tone_mapping: ToneMappingConfig,
    /// Color correction stage.
    pub color_correction: ColorCorrectionConfig,
}

impl PostProcessPipeline {
    /// Creates a new pipeline with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a list of enabled stage types in execution order.
    pub fn enabled_stages(&self) -> Vec<PostProcessStageType> {
        let mut stages = Vec::new();

        if self.ambient_occlusion.enabled {
            stages.push(PostProcessStageType::AmbientOcclusion);
        }
        if self.bloom.enabled {
            stages.push(PostProcessStageType::Bloom);
        }
        if self.fog.enabled {
            stages.push(PostProcessStageType::Fog);
        }
        if self.tone_mapping.operator != ToneMappingOperator::None {
            stages.push(PostProcessStageType::ToneMapping);
        }
        if self.color_correction.enabled {
            stages.push(PostProcessStageType::ColorCorrection);
        }

        stages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_below_threshold() {
        let bloom = BloomConfig {
            enabled: true,
            threshold: 0.8,
            intensity: 1.0,
            ..Default::default()
        };

        assert_eq!(bloom.compute_bloom(0.5), 0.0);
        assert_eq!(bloom.compute_bloom(0.8), 0.0);
    }

    #[test]
    fn test_bloom_above_threshold() {
        let bloom = BloomConfig {
            enabled: true,
            threshold: 0.8,
            intensity: 2.0,
            ..Default::default()
        };

        let result = bloom.compute_bloom(1.0);
        assert!((result - 0.4).abs() < 1e-10); // (1.0 - 0.8) * 2.0
    }

    #[test]
    fn test_bloom_disabled() {
        let bloom = BloomConfig::default(); // disabled by default
        assert_eq!(bloom.compute_bloom(10.0), 0.0);
    }

    #[test]
    fn test_ao_no_occlusion() {
        let ao = AmbientOcclusionConfig {
            enabled: true,
            intensity: 3.0,
            ..Default::default()
        };

        assert_eq!(ao.compute_ao(0.0), 1.0);
    }

    #[test]
    fn test_ao_full_occlusion() {
        let ao = AmbientOcclusionConfig {
            enabled: true,
            intensity: 3.0,
            ..Default::default()
        };

        // Full occlusion with intensity 3.0 → clamped to 0.0
        assert_eq!(ao.compute_ao(1.0), 0.0);
    }

    #[test]
    fn test_ao_partial() {
        let ao = AmbientOcclusionConfig {
            enabled: true,
            intensity: 1.0,
            ..Default::default()
        };

        let result = ao.compute_ao(0.5);
        assert!((result - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_fog_near() {
        let fog = FogConfig::default();
        assert_eq!(fog.compute_fog_factor(50.0), 0.0); // Below minimum distance
    }

    #[test]
    fn test_fog_far() {
        let fog = FogConfig {
            enabled: true,
            density: 1.0e-3,
            ..Default::default()
        };

        let factor = fog.compute_fog_factor(10000.0);
        assert!(factor > 0.99); // Nearly fully fogged
    }

    #[test]
    fn test_fog_apply() {
        let fog = FogConfig {
            enabled: true,
            density: 1.0e-3,
            color: DVec3::new(1.0, 1.0, 1.0),
            ..Default::default()
        };

        let pixel = DVec3::new(0.0, 0.0, 0.0);
        let result = fog.apply_fog(pixel, 10000.0);

        // Should be mostly white (fog color)
        assert!(result.x > 0.9);
        assert!(result.y > 0.9);
        assert!(result.z > 0.9);
    }

    #[test]
    fn test_tone_mapping_reinhard() {
        let config = ToneMappingConfig {
            operator: ToneMappingOperator::Reinhard,
            exposure: 1.0,
            white_point: 100.0, // Large white point ≈ simple Reinhard
        };

        let hdr = DVec3::new(2.0, 2.0, 2.0);
        let ldr = config.apply(hdr);

        // Simple Reinhard: x / (1 + x) = 2 / 3 ≈ 0.667
        assert!((ldr.x - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_tone_mapping_aces() {
        let config = ToneMappingConfig {
            operator: ToneMappingOperator::AcesFilmic,
            exposure: 1.0,
            white_point: 1.0,
        };

        let hdr = DVec3::new(1.0, 1.0, 1.0);
        let ldr = config.apply(hdr);

        // ACES should map 1.0 to something less than 1.0
        assert!(ldr.x < 1.0);
        assert!(ldr.x > 0.0);
    }

    #[test]
    fn test_tone_mapping_none() {
        let config = ToneMappingConfig {
            operator: ToneMappingOperator::None,
            exposure: 1.0,
            white_point: 1.0,
        };

        let hdr = DVec3::new(0.5, 0.7, 0.9);
        let ldr = config.apply(hdr);

        assert!((ldr - hdr).length() < 1e-10);
    }

    #[test]
    fn test_color_correction_brightness() {
        let cc = ColorCorrectionConfig {
            enabled: true,
            brightness: 0.1,
            contrast: 1.0,
            saturation: 1.0,
            hue: 0.0,
        };

        let color = DVec3::new(0.5, 0.5, 0.5);
        let result = cc.apply(color);

        assert!((result.x - 0.6).abs() < 1e-10);
    }

    #[test]
    fn test_color_correction_saturation_zero() {
        let cc = ColorCorrectionConfig {
            enabled: true,
            brightness: 0.0,
            contrast: 1.0,
            saturation: 0.0, // Grayscale
            hue: 0.0,
        };

        let color = DVec3::new(1.0, 0.0, 0.0);
        let result = cc.apply(color);

        // Should be grayscale (all channels equal)
        assert!((result.x - result.y).abs() < 1e-10);
        assert!((result.y - result.z).abs() < 1e-10);
    }

    #[test]
    fn test_pipeline_enabled_stages() {
        let mut pipeline = PostProcessPipeline::new();
        pipeline.bloom.enabled = true;
        pipeline.fog.enabled = true;

        let stages = pipeline.enabled_stages();

        assert!(stages.contains(&PostProcessStageType::Bloom));
        assert!(stages.contains(&PostProcessStageType::Fog));
        assert!(stages.contains(&PostProcessStageType::ToneMapping)); // Default is ACES
    }

    #[test]
    fn test_pipeline_default_stages() {
        let pipeline = PostProcessPipeline::new();
        let stages = pipeline.enabled_stages();

        // By default: only fog and tone mapping are enabled
        assert_eq!(stages.len(), 2);
        assert!(stages.contains(&PostProcessStageType::Fog));
        assert!(stages.contains(&PostProcessStageType::ToneMapping));
    }
}
