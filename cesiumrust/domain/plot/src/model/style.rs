//! Element styling: colour, size, fill/outline, icon/text, height-reference and
//! draw-order. Serialisable as-is; the render bridge reads it directly (§5).
//!
//! Colours are plain RGBA `[f32; 4]` (0..1, linear-ish — the bridge does any
//! colour-space conversion) so the core stays engine-free and diffable.

use serde::{Deserialize, Serialize};

/// RGBA colour, components in 0..=1.
pub type Rgba = [f32; 4];

/// Opaque white.
pub const WHITE: Rgba = [1.0, 1.0, 1.0, 1.0];
/// Fully transparent.
pub const TRANSPARENT: Rgba = [0.0, 0.0, 0.0, 0.0];

/// How an element's height is interpreted against the terrain / ellipsoid
/// (Cesium `HeightReference` semantics, plan §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum HeightReference {
    /// Use the stored absolute `height_m` as-is.
    #[default]
    None,
    /// Drape on the terrain / globe surface (height forced to the surface).
    ClampToGround,
    /// `height_m` measured above the terrain surface.
    RelativeToGround,
    /// Explicit metres above the ellipsoid (same as None here, kept for clarity).
    Absolute,
}

/// A polygon outline (stroke) description.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Outline {
    pub color: Rgba,
    pub width_px: f32,
}

/// Icon draw overrides (image is resolved from the geometry's registry key).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IconStyle {
    pub size_px: f32,
    /// Horizontal/vertical anchor within the icon box, 0..1 (0.5, 0.5 == centre).
    pub anchor: [f32; 2],
}

/// Text draw overrides for a [`Label`](super::geometry::LabelGeometry) or a
/// label-bearing icon.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TextStyle {
    pub size_px: f32,
    pub color: Rgba,
    /// Halo/outline colour behind glyphs for legibility on imagery.
    pub halo_color: Rgba,
    pub halo_px: f32,
}

/// The complete style bag for an element. Every field has a sane default so a
/// freshly-drawn element looks reasonable without explicit styling.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Style {
    /// Primary colour (point fill, line stroke, polygon outline base).
    pub color: Rgba,
    /// Global opacity multiplier applied on top of `color`'s alpha.
    pub opacity: f32,
    /// Line / outline width in screen pixels.
    pub width_px: f32,
    /// Polygon fill colour; `None` = outline only.
    pub fill: Option<Rgba>,
    /// Explicit outline override; `None` = derive from `color`/`width_px`.
    pub outline: Option<Outline>,
    /// Point marker diameter in screen pixels.
    pub point_size_px: f32,
    /// Icon overrides (present iff the geometry is an `Icon`).
    pub icon: Option<IconStyle>,
    /// Text overrides (present iff the geometry carries text).
    pub text: Option<TextStyle>,
    /// Height semantics.
    pub height_reference: HeightReference,
    /// Depth-test against terrain (true = correctly occluded by the globe).
    pub depth_test: bool,
    /// Relative draw / pick order within a layer (bigger draws on top, later).
    pub z_order: i32,
    /// Show in the 2D flat view (§10.6).
    pub show_in_flat: bool,
    /// Show in the 3D globe view (§10.6).
    pub show_in_globe: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            color: [0.16, 0.50, 0.86, 1.0],
            opacity: 1.0,
            width_px: 2.0,
            fill: Some([0.16, 0.50, 0.86, 0.25]),
            outline: None,
            point_size_px: 10.0,
            icon: Some(IconStyle {
                size_px: 32.0,
                anchor: [0.5, 0.5],
            }),
            text: Some(TextStyle {
                size_px: 16.0,
                color: WHITE,
                halo_color: [0.0, 0.0, 0.0, 0.85],
                halo_px: 2.0,
            }),
            height_reference: HeightReference::default(),
            // Plan §17.3 default: new elements are terrain-occluded.
            depth_test: true,
            z_order: 0,
            show_in_flat: true,
            show_in_globe: true,
        }
    }
}

impl Style {
    /// `color` scaled by `opacity` (the effective draw colour).
    pub fn effective_color(&self) -> Rgba {
        [
            self.color[0],
            self.color[1],
            self.color[2],
            self.color[3] * self.opacity,
        ]
    }

    /// A copy with a different primary colour (keeps everything else).
    pub fn with_color(mut self, color: Rgba) -> Self {
        self.color = color;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_visible_in_both_views_with_depth_test() {
        let s = Style::default();
        assert!(s.show_in_flat && s.show_in_globe);
        assert!(s.depth_test, "plan §17.3: new elements occluded by default");
        assert_eq!(s.height_reference, HeightReference::None);
    }

    #[test]
    fn effective_color_scales_alpha_only() {
        let s = Style::default();
        let c = s.effective_color();
        assert_eq!(c[0], s.color[0]);
        assert!((c[3] - s.color[3] * s.opacity).abs() < 1e-9);
    }

    #[test]
    fn style_serde_roundtrips() {
        let s = Style::default();
        let j = serde_json::to_string(&s).unwrap();
        let back: Style = serde_json::from_str(&j).unwrap();
        assert_eq!(s, back);
    }
}
