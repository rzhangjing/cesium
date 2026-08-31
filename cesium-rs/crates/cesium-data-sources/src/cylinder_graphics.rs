//! Ported from `packages/engine/Source/DataSources/CylinderGraphics.js`.
//!
//! DEVIATION: the Rust value model materializes the JS `Property` objects as
//! plain constants; `shadows`/`distanceDisplayCondition` sub-properties are
//! not materialized (updaters apply the JS defaults).

use cesium_core::color::Color;
use cesium_scene::height_reference::HeightReference;

/// Graphics properties for a cylinder.
#[derive(Clone)]
pub struct CylinderGraphics {
    /// Whether this cylinder is shown (JS `show`, default `true`).
    pub show: bool,
    /// The length of the cylinder (JS `length`).
    pub length: Option<f64>,
    /// The radius of the top of the cylinder (JS `topRadius`).
    pub top_radius: Option<f64>,
    /// The radius of the bottom of the cylinder (JS `bottomRadius`).
    pub bottom_radius: Option<f64>,
    /// The number of edges around the perimeter (JS `slices`).
    pub slices: Option<f64>,
    /// The number of vertical lines (JS `numberOfVerticalLines`).
    pub number_of_vertical_lines: Option<f64>,
    /// Whether the cylinder is filled (JS `fill`, default `true`).
    pub fill: bool,
    /// Whether the cylinder is outlined (JS `outline`, default `false`).
    pub outline: bool,
    /// The outline color (JS `outlineColor`, default `Color.BLACK`).
    pub outline_color: Color,
    /// The outline width in pixels (JS `outlineWidth`, default `1.0`).
    pub outline_width: f64,
    /// The material color (JS `material`, default `Color.WHITE`).
    pub material_color: Color,
    /// The height reference (JS `heightReference`, default `NONE`).
    pub height_reference: HeightReference,
}

impl CylinderGraphics {
    /// Creates a new cylinder graphics with default values.
    pub fn new() -> Self {
        Self {
            show: true,
            length: None,
            top_radius: None,
            bottom_radius: None,
            slices: None,
            number_of_vertical_lines: None,
            fill: true,
            outline: false,
            outline_color: Color::BLACK,
            outline_width: 1.0,
            material_color: Color::WHITE,
            height_reference: HeightReference::None,
        }
    }
}

impl Default for CylinderGraphics {
    fn default() -> Self { Self::new() }
}
