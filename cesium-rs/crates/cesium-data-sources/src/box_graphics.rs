//! Ported from `packages/engine/Source/DataSources/BoxGraphics.js`.
//!
//! DEVIATION: the Rust value model materializes the JS `Property` objects as
//! plain constants (see docs/deviations.md), so `getValue` is implicit and
//! `isConstant` is always `true`. Sub-properties that the JS class exposes
//! (`shadows`, `distanceDisplayCondition`, `zIndex`) are not materialized;
//! the updaters apply the JS defaults for them.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;
use cesium_scene::height_reference::HeightReference;

/// Graphics properties for a box.
#[derive(Clone)]
pub struct BoxGraphics {
    /// Whether this box is shown (JS `show`, default `true`).
    pub show: bool,
    /// The width, depth, and height of the box (JS `dimensions`).
    pub dimensions: Option<Cartesian3>,
    /// Whether the box is filled (JS `fill`, default `true`).
    pub fill: bool,
    /// The material color (JS `material`, default `Color.WHITE`
    /// `ColorMaterialProperty`).
    pub material_color: Color,
    /// Whether the box is outlined (JS `outline`, default `false`).
    pub outline: bool,
    /// The outline color (JS `outlineColor`, default `Color.BLACK`).
    pub outline_color: Color,
    /// The outline width in pixels (JS `outlineWidth`, default `1.0`).
    pub outline_width: f64,
    /// The height reference (JS `heightReference`, default `NONE`).
    pub height_reference: HeightReference,
}

impl BoxGraphics {
    /// Creates a new box graphics with default values.
    pub fn new() -> Self {
        Self {
            show: true,
            dimensions: None,
            fill: true,
            material_color: Color::WHITE,
            outline: false,
            outline_color: Color::BLACK,
            outline_width: 1.0,
            height_reference: HeightReference::None,
        }
    }
}

impl Default for BoxGraphics {
    fn default() -> Self { Self::new() }
}
