//! Ported from `packages/engine/Source/DataSources/CorridorGraphics.js`.
//!
//! DEVIATION: the Rust value model materializes the JS `Property` objects as
//! plain constants; `shadows`/`distanceDisplayCondition`/`classificationType`
//! sub-properties are not materialized (updaters apply the JS defaults).

use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;
use cesium_core::corner_type::CornerType;
use cesium_scene::height_reference::HeightReference;

/// Graphics properties for a corridor.
#[derive(Clone)]
pub struct CorridorGraphics {
    /// Whether this corridor is shown (JS `show`, default `true`).
    pub show: bool,
    /// The positions of the corridor center line (JS `positions`).
    pub positions: Vec<Cartesian3>,
    /// The distance between the edges of the corridor (JS `width`).
    pub width: Option<f64>,
    /// The style of the corners (JS `cornerType`, default `ROUNDED`).
    pub corner_type: CornerType,
    /// The height above the ellipsoid (JS `height`).
    pub height: Option<f64>,
    /// The extruded height (JS `extrudedHeight`).
    pub extruded_height: Option<f64>,
    /// Whether the corridor is filled (JS `fill`, default `true`).
    pub fill: bool,
    /// Whether the corridor is outlined (JS `outline`, default `false`).
    pub outline: bool,
    /// The outline color (JS `outlineColor`, default `Color.BLACK`).
    pub outline_color: Color,
    /// The outline width in pixels (JS `outlineWidth`, default `1.0`).
    pub outline_width: f64,
    /// The material color (JS `material`, default `Color.WHITE`).
    pub material_color: Color,
    /// The sampling granularity (JS `granularity`).
    pub granularity: Option<f64>,
    /// The z-index for ground corridors (JS `zIndex`).
    pub z_index: Option<f64>,
    /// The height reference (JS `heightReference`, default `NONE`).
    pub height_reference: HeightReference,
    /// The extruded height reference (JS `extrudedHeightReference`, default
    /// `NONE`).
    pub extruded_height_reference: HeightReference,
}

impl CorridorGraphics {
    /// Creates a new corridor graphics with default values.
    pub fn new() -> Self {
        Self {
            show: true,
            positions: Vec::new(),
            width: None,
            corner_type: CornerType::Rounded,
            height: None,
            extruded_height: None,
            fill: true,
            outline: false,
            outline_color: Color::BLACK,
            outline_width: 1.0,
            material_color: Color::WHITE,
            granularity: None,
            z_index: None,
            height_reference: HeightReference::None,
            extruded_height_reference: HeightReference::None,
        }
    }
}

impl Default for CorridorGraphics {
    fn default() -> Self { Self::new() }
}
