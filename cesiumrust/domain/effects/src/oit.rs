//! Order-Independent Transparency (OIT).
//!
//! Maps to CesiumJS `Scene/OIT.js`:
//! - Weighted blended OIT (accumulation + revealage)
//! - Translucent multipass support
//! - MRT (Multiple Render Target) support detection
//!
//! Domain layer — pure Rust, f64 precision.

use glam::DVec4;

/// Blend equation for OIT compositing.
///
/// Maps to CesiumJS `BlendEquation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendEquation {
    /// Source + Destination.
    #[default]
    Add,
    /// Source - Destination.
    Subtract,
    /// Destination - Source.
    ReverseSubtract,
    /// Min(Source, Destination).
    Min,
    /// Max(Source, Destination).
    Max,
}

/// Blend function for OIT.
///
/// Maps to CesiumJS `BlendFunction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendFunction {
    /// Zero.
    Zero,
    /// One.
    #[default]
    One,
    /// Source color.
    SourceColor,
    /// One minus source color.
    OneMinusSourceColor,
    /// Destination color.
    DestinationColor,
    /// One minus destination color.
    OneMinusDestinationColor,
    /// Source alpha.
    SourceAlpha,
    /// One minus source alpha.
    OneMinusSourceAlpha,
    /// Destination alpha.
    DestinationAlpha,
    /// One minus destination alpha.
    OneMinusDestinationAlpha,
}

/// OIT support capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OitCapabilities {
    /// Whether MRT (Multiple Render Targets) is supported.
    pub mrt_supported: bool,
    /// Whether float blend is supported.
    pub float_blend_supported: bool,
    /// Whether depth texture is supported.
    pub depth_texture_supported: bool,
    /// Whether color buffer float is supported.
    pub color_buffer_float: bool,
}

impl OitCapabilities {
    /// Returns whether weighted blended OIT via MRT is supported.
    pub fn translucent_mrt_supported(&self) -> bool {
        self.mrt_supported
            && self.color_buffer_float
            && self.depth_texture_supported
            && self.float_blend_supported
    }

    /// Returns whether multipass OIT is supported (fallback when MRT unavailable).
    pub fn translucent_multipass_supported(&self) -> bool {
        !self.translucent_mrt_supported()
            && self.color_buffer_float
            && self.depth_texture_supported
            && self.float_blend_supported
    }

    /// Returns whether any OIT mode is supported.
    pub fn is_supported(&self) -> bool {
        self.translucent_mrt_supported() || self.translucent_multipass_supported()
    }
}

/// OIT rendering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OitMode {
    /// No OIT (standard alpha blending).
    #[default]
    None,
    /// Weighted blended OIT using MRT (2 render targets).
    WeightedBlendedMrt,
    /// Weighted blended OIT using multipass (fallback).
    WeightedBlendedMultipass,
}

/// OIT configuration and state.
///
/// Maps to CesiumJS `OIT`.
#[derive(Debug, Clone)]
pub struct OitConfig {
    /// The OIT mode to use.
    pub mode: OitMode,
    /// Number of MSAA samples.
    pub num_samples: u32,
    /// Whether to use HDR rendering.
    pub use_hdr: bool,
    /// Blend equation for the accumulation buffer.
    pub blend_equation: BlendEquation,
    /// Source blend function.
    pub source_blend: BlendFunction,
    /// Destination blend function.
    pub destination_blend: BlendFunction,
}

impl Default for OitConfig {
    fn default() -> Self {
        Self {
            mode: OitMode::None,
            num_samples: 1,
            use_hdr: false,
            blend_equation: BlendEquation::Add,
            source_blend: BlendFunction::One,
            destination_blend: BlendFunction::One,
        }
    }
}

impl OitConfig {
    /// Creates an OIT config based on device capabilities.
    pub fn from_capabilities(caps: &OitCapabilities) -> Self {
        let mode = if caps.translucent_mrt_supported() {
            OitMode::WeightedBlendedMrt
        } else if caps.translucent_multipass_supported() {
            OitMode::WeightedBlendedMultipass
        } else {
            OitMode::None
        };

        Self {
            mode,
            ..Default::default()
        }
    }

    /// Returns whether OIT is active.
    pub fn is_active(&self) -> bool {
        self.mode != OitMode::None
    }

    /// Computes the weight for a fragment based on its depth and alpha.
    ///
    /// Uses the weighting function from McGuire & Bavoil (2013):
    /// w = alpha * clamp(0.03 / (1e-5 + pow(depth/200, 4)), 0.01, 3000)
    pub fn compute_weight(&self, alpha: f64, depth: f64) -> f64 {
        let depth_term = (depth / 200.0).powi(4);
        alpha * (0.03 / (1e-5 + depth_term)).clamp(0.01, 3000.0)
    }

    /// Accumulates a translucent fragment into the OIT buffers.
    ///
    /// # Arguments
    /// * `color` - The fragment color (RGBA, premultiplied alpha expected)
    /// * `depth` - The fragment depth (view space, positive)
    ///
    /// # Returns
    /// (accumulation, revealage) — the values to add to the respective buffers.
    pub fn accumulate_fragment(&self, color: DVec4, depth: f64) -> (DVec4, f64) {
        let weight = self.compute_weight(color.w, depth);

        // Accumulation: color * weight
        let accumulation = DVec4::new(
            color.x * weight,
            color.y * weight,
            color.z * weight,
            color.w * weight,
        );

        // Revealage: 1 - alpha (product of all revealages)
        let revealage = 1.0 - color.w;

        (accumulation, revealage)
    }

    /// Composites the OIT buffers with the opaque scene.
    ///
    /// # Arguments
    /// * `opaque_color` - The opaque scene color (RGB)
    /// * `accumulation` - The accumulated translucent color (RGBA)
    /// * `revealage` - The revealage value (0 = fully transparent, 1 = fully opaque)
    ///
    /// # Returns
    /// The final composited color (RGB).
    pub fn composite(
        &self,
        opaque_color: DVec4,
        accumulation: DVec4,
        revealage: f64,
    ) -> DVec4 {
        if revealage >= 1.0 {
            // No translucent contribution
            return opaque_color;
        }

        // Average translucent color
        let avg_color = if accumulation.w > 1e-5 {
            DVec4::new(
                accumulation.x / accumulation.w,
                accumulation.y / accumulation.w,
                accumulation.z / accumulation.w,
                accumulation.w,
            )
        } else {
            DVec4::ZERO
        };

        // Blend: opaque * revealage + translucent * (1 - revealage)
        let translucent_alpha = (1.0 - revealage) * avg_color.w;
        DVec4::new(
            opaque_color.x * revealage + avg_color.x * translucent_alpha,
            opaque_color.y * revealage + avg_color.y * translucent_alpha,
            opaque_color.z * revealage + avg_color.z * translucent_alpha,
            1.0 - revealage * (1.0 - opaque_color.w),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oit_capabilities_mrt() {
        let caps = OitCapabilities {
            mrt_supported: true,
            float_blend_supported: true,
            depth_texture_supported: true,
            color_buffer_float: true,
        };

        assert!(caps.translucent_mrt_supported());
        assert!(!caps.translucent_multipass_supported());
        assert!(caps.is_supported());
    }

    #[test]
    fn test_oit_capabilities_multipass() {
        let caps = OitCapabilities {
            mrt_supported: false,
            float_blend_supported: true,
            depth_texture_supported: true,
            color_buffer_float: true,
        };

        assert!(!caps.translucent_mrt_supported());
        assert!(caps.translucent_multipass_supported());
        assert!(caps.is_supported());
    }

    #[test]
    fn test_oit_capabilities_unsupported() {
        let caps = OitCapabilities {
            mrt_supported: false,
            float_blend_supported: false,
            depth_texture_supported: true,
            color_buffer_float: true,
        };

        assert!(!caps.is_supported());
    }

    #[test]
    fn test_oit_config_from_capabilities() {
        let caps = OitCapabilities {
            mrt_supported: true,
            float_blend_supported: true,
            depth_texture_supported: true,
            color_buffer_float: true,
        };

        let config = OitConfig::from_capabilities(&caps);
        assert_eq!(config.mode, OitMode::WeightedBlendedMrt);
        assert!(config.is_active());
    }

    #[test]
    fn test_oit_config_unsupported() {
        let caps = OitCapabilities::default();
        let config = OitConfig::from_capabilities(&caps);
        assert_eq!(config.mode, OitMode::None);
        assert!(!config.is_active());
    }

    #[test]
    fn test_compute_weight() {
        let config = OitConfig::default();

        // Near fragments should have higher weight
        let near_weight = config.compute_weight(1.0, 1.0);
        let far_weight = config.compute_weight(1.0, 100.0);
        assert!(near_weight > far_weight);

        // Zero alpha → zero weight
        let zero_alpha = config.compute_weight(0.0, 10.0);
        assert!((zero_alpha).abs() < 1e-10);
    }

    #[test]
    fn test_accumulate_fragment() {
        let config = OitConfig::default();
        let color = DVec4::new(1.0, 0.0, 0.0, 0.5); // Semi-transparent red
        let depth = 10.0;

        let (accumulation, revealage) = config.accumulate_fragment(color, depth);

        // Accumulation should be non-zero
        assert!(accumulation.x > 0.0);
        // Revealage should be 1 - alpha = 0.5
        assert!((revealage - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_composite_no_translucent() {
        let config = OitConfig::default();
        let opaque = DVec4::new(0.5, 0.5, 0.5, 1.0);
        let accumulation = DVec4::ZERO;
        let revealage = 1.0; // Fully opaque (no translucent)

        let result = config.composite(opaque, accumulation, revealage);
        assert!((result - opaque).length() < 1e-10);
    }

    #[test]
    fn test_composite_with_translucent() {
        let config = OitConfig::default();
        let opaque = DVec4::new(0.0, 0.0, 1.0, 1.0); // Blue background
        let accumulation = DVec4::new(0.5, 0.0, 0.0, 0.5); // Red contribution
        let revealage = 0.5; // 50% transparent

        let result = config.composite(opaque, accumulation, revealage);

        // Result should be a blend of blue and red
        assert!(result.x > 0.0); // Has red
        assert!(result.z > 0.0); // Has blue
    }

    #[test]
    fn test_blend_equation_default() {
        assert_eq!(BlendEquation::default(), BlendEquation::Add);
    }

    #[test]
    fn test_blend_function_default() {
        assert_eq!(BlendFunction::default(), BlendFunction::One);
    }
}
