//! Imagery layer configuration.
//! Maps to CesiumJS `Scene/ImageryLayer.js`

use cesium_geospatial::rectangle::Rectangle;
use serde::{Deserialize, Serialize};

use crate::{AlphaBlendingMode, SplitDirection};

/// Configuration for an imagery layer.
///
/// This contains all the visual properties that can be applied to an imagery layer.
/// Maps to CesiumJS `ImageryLayer`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageryLayer {
    /// Unique identifier for this layer.
    pub id: u64,

    /// The rectangle covered by this layer.
    pub rectangle: Rectangle,

    /// Alpha blending value (0.0 to 1.0).
    #[serde(default = "default_alpha")]
    pub alpha: f64,

    /// Alpha on the night side of the globe (0.0 to 1.0).
    #[serde(default = "default_alpha")]
    pub night_alpha: f64,

    /// Alpha on the day side of the globe (0.0 to 1.0).
    #[serde(default = "default_alpha")]
    pub day_alpha: f64,

    /// Brightness adjustment (1.0 = unmodified).
    #[serde(default = "default_one")]
    pub brightness: f64,

    /// Contrast adjustment (1.0 = unmodified).
    #[serde(default = "default_one")]
    pub contrast: f64,

    /// Hue rotation in radians (0.0 = unmodified).
    #[serde(default)]
    pub hue: f64,

    /// Saturation adjustment (1.0 = unmodified).
    #[serde(default = "default_one")]
    pub saturation: f64,

    /// Gamma correction (1.0 = unmodified).
    #[serde(default = "default_one")]
    pub gamma: f64,

    /// Whether the layer is visible.
    #[serde(default = "default_true")]
    pub show: bool,

    /// The alpha blending mode.
    #[serde(default)]
    pub alpha_blending_mode: AlphaBlendingMode,

    /// The split direction for split-screen comparison.
    #[serde(default)]
    pub split_direction: SplitDirection,

    /// Minimum zoom level.
    #[serde(default)]
    pub minimum_level: u32,

    /// Maximum zoom level.
    #[serde(default = "default_max_level")]
    pub maximum_level: u32,

    /// Tile width in pixels.
    #[serde(default = "default_tile_size")]
    pub tile_width: u32,

    /// Tile height in pixels.
    #[serde(default = "default_tile_size")]
    pub tile_height: u32,
}

fn default_alpha() -> f64 {
    1.0
}

fn default_one() -> f64 {
    1.0
}

fn default_true() -> bool {
    true
}

fn default_max_level() -> u32 {
    25
}

fn default_tile_size() -> u32 {
    256
}

impl ImageryLayer {
    /// Creates a new imagery layer with default settings.
    pub fn new(id: u64, rectangle: Rectangle) -> Self {
        Self {
            id,
            rectangle,
            alpha: 1.0,
            night_alpha: 1.0,
            day_alpha: 1.0,
            brightness: 1.0,
            contrast: 1.0,
            hue: 0.0,
            saturation: 1.0,
            gamma: 1.0,
            show: true,
            alpha_blending_mode: AlphaBlendingMode::Standard,
            split_direction: SplitDirection::None,
            minimum_level: 0,
            maximum_level: 25,
            tile_width: 256,
            tile_height: 256,
        }
    }

    /// Sets the alpha value.
    pub fn with_alpha(mut self, alpha: f64) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Sets the brightness.
    pub fn with_brightness(mut self, brightness: f64) -> Self {
        self.brightness = brightness.max(0.0);
        self
    }

    /// Sets the contrast.
    pub fn with_contrast(mut self, contrast: f64) -> Self {
        self.contrast = contrast.max(0.0);
        self
    }

    /// Sets the saturation.
    pub fn with_saturation(mut self, saturation: f64) -> Self {
        self.saturation = saturation.max(0.0);
        self
    }

    /// Sets the gamma.
    pub fn with_gamma(mut self, gamma: f64) -> Self {
        self.gamma = gamma.max(0.001);
        self
    }

    /// Sets visibility.
    pub fn with_show(mut self, show: bool) -> Self {
        self.show = show;
        self
    }

    /// Sets the alpha blending mode.
    pub fn with_alpha_blending_mode(mut self, mode: AlphaBlendingMode) -> Self {
        self.alpha_blending_mode = mode;
        self
    }

    /// Sets the split direction.
    pub fn with_split_direction(mut self, direction: SplitDirection) -> Self {
        self.split_direction = direction;
        self
    }

    /// Sets the zoom level range.
    pub fn with_level_range(mut self, min: u32, max: u32) -> Self {
        self.minimum_level = min;
        self.maximum_level = max;
        self
    }

    /// Sets the tile size.
    pub fn with_tile_size(mut self, width: u32, height: u32) -> Self {
        self.tile_width = width;
        self.tile_height = height;
        self
    }

    /// Computes the effective alpha for a given lighting condition.
    ///
    /// # Arguments
    /// * `is_night` - Whether the tile is on the night side of the globe
    pub fn effective_alpha(&self, is_night: bool) -> f64 {
        let base_alpha = if is_night { self.night_alpha } else { self.day_alpha };
        base_alpha * self.alpha
    }

    /// Checks if a level is within the valid range for this layer.
    pub fn is_level_valid(&self, level: u32) -> bool {
        level >= self.minimum_level && level <= self.maximum_level
    }

    /// Checks if a rectangle intersects with this layer's rectangle.
    pub fn intersects(&self, rectangle: &Rectangle) -> bool {
        self.rectangle.intersection(rectangle).is_some()
    }
}

impl Default for ImageryLayer {
    fn default() -> Self {
        Self::new(0, Rectangle::MAX_VALUE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_layer() {
        let layer = ImageryLayer::new(1, Rectangle::from_degrees(-180.0, -90.0, 180.0, 90.0));
        assert_eq!(layer.id, 1);
        assert_eq!(layer.alpha, 1.0);
        assert!(layer.show);
    }

    #[test]
    fn test_builder_pattern() {
        let layer = ImageryLayer::new(1, Rectangle::MAX_VALUE)
            .with_alpha(0.5)
            .with_brightness(1.2)
            .with_show(false);

        assert_eq!(layer.alpha, 0.5);
        assert_eq!(layer.brightness, 1.2);
        assert!(!layer.show);
    }

    #[test]
    fn test_alpha_clamping() {
        let layer = ImageryLayer::new(1, Rectangle::MAX_VALUE).with_alpha(1.5);
        assert_eq!(layer.alpha, 1.0);

        let layer = ImageryLayer::new(1, Rectangle::MAX_VALUE).with_alpha(-0.5);
        assert_eq!(layer.alpha, 0.0);
    }

    #[test]
    fn test_effective_alpha() {
        let layer = ImageryLayer::new(1, Rectangle::MAX_VALUE)
            .with_alpha(0.8);

        assert_eq!(layer.effective_alpha(false), 0.8); // day
        assert_eq!(layer.effective_alpha(true), 0.8); // night (same by default)
    }

    #[test]
    fn test_level_validation() {
        let layer = ImageryLayer::new(1, Rectangle::MAX_VALUE)
            .with_level_range(2, 10);

        assert!(!layer.is_level_valid(1));
        assert!(layer.is_level_valid(2));
        assert!(layer.is_level_valid(10));
        assert!(!layer.is_level_valid(11));
    }
}
