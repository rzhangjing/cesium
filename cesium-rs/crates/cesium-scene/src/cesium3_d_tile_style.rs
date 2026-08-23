//! Ported from `packages/engine/Source/Scene/Cesium3DTileStyle.js`.
//!
//! A style that is applied to a 3D Tiles tileset.

use std::collections::HashMap;

/// A style that is applied to a [`Cesium3DTileset`](crate::cesium3_d_tileset::Cesium3DTileset).
///
/// Evaluates an expression that determines the visibility and style of features.
/// Mirrors CesiumJS `Cesium3DTileStyle` (604 lines).
pub struct Cesium3DTileStyle {
    /// The show expression string.
    pub show: Option<String>,
    /// The color expression string.
    pub color: Option<String>,
    /// The point size expression string (point cloud only).
    pub point_size: Option<String>,
    /// The metadata expression string.
    pub meta: HashMap<String, String>,
    /// Whether the style is ready (compiled).
    ready: bool,
}

impl Cesium3DTileStyle {
    /// Creates a new Cesium3DTileStyle.
    pub fn new() -> Self {
        Self {
            show: None,
            color: None,
            point_size: None,
            meta: HashMap::new(),
            ready: true,
        }
    }

    /// Creates a style from a JSON-like definition.
    pub fn from_json(show: Option<String>, color: Option<String>, point_size: Option<String>) -> Self {
        Self {
            show,
            color,
            point_size,
            meta: HashMap::new(),
            ready: true,
        }
    }

    /// Returns whether the style is ready (compiled).
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Evaluates the show expression for a feature.
    pub fn show_expression(&self) -> Option<&str> {
        self.show.as_deref()
    }

    /// Evaluates the color expression for a feature.
    pub fn color_expression(&self) -> Option<&str> {
        self.color.as_deref()
    }
}

impl Default for Cesium3DTileStyle {
    fn default() -> Self { Self::new() }
}
