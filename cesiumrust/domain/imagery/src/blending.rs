//! Multi-layer imagery blending.
//!
//! Implements color compositing for multiple imagery layers with support for
//! different blending modes, day/night alpha, and split direction.
//! Maps to CesiumJS `Scene/ImageryLayer.js` blending logic.

use crate::imagery_layer::ImageryLayer;
use crate::AlphaBlendingMode;

/// A pixel color in linear RGBA space (each channel 0.0..1.0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PixelColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl PixelColor {
    /// Transparent black.
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    /// Opaque black.
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    /// Opaque white.
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };

    /// Creates a new pixel color.
    pub fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    /// Creates an opaque color from RGB.
    pub fn opaque(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b, a: 1.0 }
    }
}

/// Computes the effective alpha for a layer given day/night conditions.
///
/// Maps to CesiumJS `ImageryLayer._computeAlpha`
///
/// # Arguments
/// * `layer` - The imagery layer
/// * `is_day` - Whether the tile is on the day side of the globe
///
/// # Returns
/// The effective alpha value
pub fn compute_effective_alpha(layer: &ImageryLayer, is_day: bool) -> f64 {
    let mut alpha = layer.alpha;

    // Apply day/night alpha modulation
    if is_day {
        alpha *= layer.day_alpha;
    } else {
        alpha *= layer.night_alpha;
    }

    alpha.clamp(0.0, 1.0)
}

/// Applies brightness, contrast, hue, saturation, and gamma adjustments to a pixel.
///
/// Maps to CesiumJS imagery layer color adjustments.
///
/// # Arguments
/// * `color` - The input pixel color
/// * `layer` - The imagery layer with adjustment parameters
///
/// # Returns
/// The adjusted pixel color
pub fn apply_color_adjustments(color: PixelColor, layer: &ImageryLayer) -> PixelColor {
    let mut r = color.r;
    let mut g = color.g;
    let mut b = color.b;

    // Apply brightness
    if (layer.brightness - 1.0).abs() > 1e-10 {
        r *= layer.brightness;
        g *= layer.brightness;
        b *= layer.brightness;
    }

    // Apply contrast
    if (layer.contrast - 1.0).abs() > 1e-10 {
        r = apply_contrast(r, layer.contrast);
        g = apply_contrast(g, layer.contrast);
        b = apply_contrast(b, layer.contrast);
    }

    // Apply saturation
    if (layer.saturation - 1.0).abs() > 1e-10 {
        let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        r = luminance + (r - luminance) * layer.saturation;
        g = luminance + (g - luminance) * layer.saturation;
        b = luminance + (b - luminance) * layer.saturation;
    }

    // Apply gamma
    if (layer.gamma - 1.0).abs() > 1e-10 {
        let inv_gamma = 1.0 / layer.gamma;
        r = r.max(0.0).powf(inv_gamma);
        g = g.max(0.0).powf(inv_gamma);
        b = b.max(0.0).powf(inv_gamma);
    }

    PixelColor {
        r: r.clamp(0.0, 1.0),
        g: g.clamp(0.0, 1.0),
        b: b.clamp(0.0, 1.0),
        a: color.a,
    }
}

/// Applies contrast adjustment to a single channel.
fn apply_contrast(value: f64, contrast: f64) -> f64 {
    ((value - 0.5) * contrast + 0.5).clamp(0.0, 1.0)
}

/// Blends a source pixel onto a destination pixel using the specified blending mode.
///
/// # Arguments
/// * `dst` - The destination (background) pixel
/// * `src` - The source (foreground) pixel
/// * `mode` - The alpha blending mode
/// * `layer_alpha` - The effective layer alpha
///
/// # Returns
/// The blended pixel
pub fn blend_pixel(
    dst: PixelColor,
    src: PixelColor,
    mode: AlphaBlendingMode,
    layer_alpha: f64,
) -> PixelColor {
    let src_alpha = src.a * layer_alpha;

    match mode {
        AlphaBlendingMode::Standard => {
            // Standard alpha compositing: result = src * src_alpha + dst * (1 - src_alpha)
            let inv_alpha = 1.0 - src_alpha;
            PixelColor {
                r: src.r * src_alpha + dst.r * inv_alpha,
                g: src.g * src_alpha + dst.g * inv_alpha,
                b: src.b * src_alpha + dst.b * inv_alpha,
                a: src_alpha + dst.a * inv_alpha,
            }
        }
        AlphaBlendingMode::Additive => {
            // Additive blending: result = src * src_alpha + dst
            PixelColor {
                r: (dst.r + src.r * src_alpha).clamp(0.0, 1.0),
                g: (dst.g + src.g * src_alpha).clamp(0.0, 1.0),
                b: (dst.b + src.b * src_alpha).clamp(0.0, 1.0),
                a: (dst.a + src_alpha).clamp(0.0, 1.0),
            }
        }
        AlphaBlendingMode::Multiplicative => {
            // Multiplicative blending: result = src * dst (modulated by alpha)
            let inv_alpha = 1.0 - src_alpha;
            PixelColor {
                r: src.r * dst.r * src_alpha + dst.r * inv_alpha,
                g: src.g * dst.g * src_alpha + dst.g * inv_alpha,
                b: src.b * dst.b * src_alpha + dst.b * inv_alpha,
                a: src_alpha + dst.a * inv_alpha,
            }
        }
    }
}

/// Composites multiple imagery layers from bottom to top.
///
/// # Arguments
/// * `layers` - The layers in bottom-to-top order
/// * `layer_colors` - The pixel color from each layer's texture
/// * `is_day` - Whether the tile is on the day side
/// * `base_color` - The base (terrain) color before any imagery
///
/// # Returns
/// The final composited pixel color
pub fn composite_layers(
    layers: &[&ImageryLayer],
    layer_colors: &[PixelColor],
    is_day: bool,
    base_color: PixelColor,
) -> PixelColor {
    let mut result = base_color;

    for (layer, &src_color) in layers.iter().zip(layer_colors.iter()) {
        if !layer.show {
            continue;
        }

        let effective_alpha = compute_effective_alpha(layer, is_day);
        if effective_alpha < 1e-10 {
            continue;
        }

        // Apply color adjustments
        let adjusted = apply_color_adjustments(src_color, layer);

        // Blend onto result
        result = blend_pixel(result, adjusted, layer.alpha_blending_mode, effective_alpha);
    }

    result
}

/// Determines if a layer should be rendered for a given split position.
///
/// # Arguments
/// * `layer` - The imagery layer
/// * `split_position` - The normalized split position (0.0 to 1.0)
/// * `tile_center_x` - The normalized X center of the tile (0.0 to 1.0)
///
/// # Returns
/// True if the layer should render for this tile
pub fn should_render_for_split(
    layer: &ImageryLayer,
    split_position: f64,
    tile_center_x: f64,
) -> bool {
    use crate::SplitDirection;

    match layer.split_direction {
        SplitDirection::None => true,
        SplitDirection::Left => tile_center_x <= split_position,
        SplitDirection::Right => tile_center_x > split_position,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_geospatial::rectangle::Rectangle;

    fn create_test_layer() -> ImageryLayer {
        ImageryLayer::new(1, Rectangle::MAX_VALUE)
    }

    #[test]
    fn test_effective_alpha_day() {
        let layer = create_test_layer();
        let alpha = compute_effective_alpha(&layer, true);
        assert!((alpha - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_effective_alpha_night() {
        let mut layer = create_test_layer();
        layer.night_alpha = 0.5;
        let alpha = compute_effective_alpha(&layer, false);
        assert!((alpha - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_effective_alpha_combined() {
        let mut layer = create_test_layer();
        layer.alpha = 0.8;
        layer.day_alpha = 0.5;
        let alpha = compute_effective_alpha(&layer, true);
        assert!((alpha - 0.4).abs() < 1e-10);
    }

    #[test]
    fn test_blend_standard() {
        let dst = PixelColor::opaque(0.0, 0.0, 1.0); // blue background
        let src = PixelColor::opaque(1.0, 0.0, 0.0); // red foreground

        let result = blend_pixel(dst, src, AlphaBlendingMode::Standard, 0.5);

        // 50% red + 50% blue = purple
        assert!((result.r - 0.5).abs() < 1e-10);
        assert!((result.b - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_blend_additive() {
        let dst = PixelColor::opaque(0.3, 0.3, 0.3);
        let src = PixelColor::opaque(0.5, 0.0, 0.0);

        let result = blend_pixel(dst, src, AlphaBlendingMode::Additive, 1.0);

        assert!((result.r - 0.8).abs() < 1e-10);
        assert!((result.g - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_blend_multiplicative() {
        let dst = PixelColor::opaque(0.5, 0.8, 1.0);
        let src = PixelColor::opaque(0.5, 0.5, 0.5);

        let result = blend_pixel(dst, src, AlphaBlendingMode::Multiplicative, 1.0);

        assert!((result.r - 0.25).abs() < 1e-10);
        assert!((result.g - 0.4).abs() < 1e-10);
        assert!((result.b - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_composite_multiple_layers() {
        let layer1 = create_test_layer();
        let layer2 = create_test_layer();

        let layers = vec![&layer1, &layer2];
        let colors = vec![
            PixelColor::opaque(1.0, 0.0, 0.0), // red
            PixelColor::opaque(0.0, 0.0, 1.0), // blue
        ];

        let result = composite_layers(&layers, &colors, true, PixelColor::TRANSPARENT);

        // Layer1 (red, alpha=1) on transparent → red
        // Layer2 (blue, alpha=1) on red → blue (fully covers)
        assert!((result.r - 0.0).abs() < 1e-10);
        assert!((result.b - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_composite_with_alpha() {
        let mut layer1 = create_test_layer();
        layer1.alpha = 0.5;
        let layer2 = create_test_layer();

        let layers = vec![&layer1, &layer2];
        let colors = vec![
            PixelColor::opaque(1.0, 0.0, 0.0), // red at 50%
            PixelColor::new(0.0, 0.0, 1.0, 0.5), // blue at 50% pixel alpha
        ];

        let result = composite_layers(&layers, &colors, true, PixelColor::BLACK);

        // After layer1: 0.5*red + 0.5*black = (0.5, 0, 0)
        // After layer2: 0.5*blue + 0.5*(0.5, 0, 0) = (0.25, 0, 0.5)
        assert!((result.r - 0.25).abs() < 1e-10);
        assert!((result.b - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_color_adjustments_brightness() {
        let mut layer = create_test_layer();
        layer.brightness = 1.5;

        let color = PixelColor::opaque(0.4, 0.2, 0.6);
        let adjusted = apply_color_adjustments(color, &layer);

        assert!((adjusted.r - 0.6).abs() < 1e-10);
        assert!((adjusted.g - 0.3).abs() < 1e-10);
        assert!((adjusted.b - 0.9).abs() < 1e-10);
    }

    #[test]
    fn test_color_adjustments_saturation() {
        let mut layer = create_test_layer();
        layer.saturation = 0.0; // fully desaturate

        let color = PixelColor::opaque(1.0, 0.0, 0.0);
        let adjusted = apply_color_adjustments(color, &layer);

        // Luminance of pure red = 0.2126
        let lum = 0.2126;
        assert!((adjusted.r - lum).abs() < 1e-6);
        assert!((adjusted.g - lum).abs() < 1e-6);
        assert!((adjusted.b - lum).abs() < 1e-6);
    }

    #[test]
    fn test_split_direction() {
        use crate::SplitDirection;

        let mut layer = create_test_layer();

        // No split - always render
        layer.split_direction = SplitDirection::None;
        assert!(should_render_for_split(&layer, 0.5, 0.3));
        assert!(should_render_for_split(&layer, 0.5, 0.7));

        // Left split - render only left of split
        layer.split_direction = SplitDirection::Left;
        assert!(should_render_for_split(&layer, 0.5, 0.3));
        assert!(!should_render_for_split(&layer, 0.5, 0.7));

        // Right split - render only right of split
        layer.split_direction = SplitDirection::Right;
        assert!(!should_render_for_split(&layer, 0.5, 0.3));
        assert!(should_render_for_split(&layer, 0.5, 0.7));
    }
}
