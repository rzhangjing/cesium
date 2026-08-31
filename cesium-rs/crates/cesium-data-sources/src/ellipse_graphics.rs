//! Ported from `packages/engine/Source/DataSources/EllipseGraphics.js`.
//!
//! DEVIATION: the Rust value model materializes the JS `Property` objects as
//! plain constants; `shadows`/`distanceDisplayCondition`/`classificationType`
//! sub-properties are not materialized (updaters apply the JS defaults).

use cesium_core::color::Color;
use cesium_scene::height_reference::HeightReference;

/// Graphics properties for an ellipse.
#[derive(Clone)]
pub struct EllipseGraphics {
    /// Whether this ellipse is shown (JS `show`, default `true`).
    pub show: bool,
    /// The semi-major axis (JS `semiMajorAxis`).
    pub semi_major_axis: Option<f64>,
    /// The semi-minor axis (JS `semiMinorAxis`).
    pub semi_minor_axis: Option<f64>,
    /// The height above the ellipsoid (JS `height`).
    pub height: Option<f64>,
    /// The extruded height (JS `extrudedHeight`).
    pub extruded_height: Option<f64>,
    /// The rotation of the ellipse about its center (JS `rotation`).
    pub rotation: Option<f64>,
    /// The texture coordinate rotation (JS `stRotation`).
    pub st_rotation: Option<f64>,
    /// The sampling granularity (JS `granularity`).
    pub granularity: Option<f64>,
    /// The number of vertical lines for extruded outlines
    /// (JS `numberOfVerticalLines`).
    pub number_of_vertical_lines: Option<f64>,
    /// Whether the ellipse is filled (JS `fill`, default `true`).
    pub fill: bool,
    /// Whether the ellipse is outlined (JS `outline`, default `false`).
    pub outline: bool,
    /// The outline color (JS `outlineColor`, default `Color.BLACK`).
    pub outline_color: Color,
    /// The outline width in pixels (JS `outlineWidth`, default `1.0`).
    pub outline_width: f64,
    /// The material color (JS `material`, default `Color.WHITE`).
    pub material_color: Color,
    /// The z-index for ground ellipses (JS `zIndex`).
    pub z_index: Option<f64>,
    /// The height reference (JS `heightReference`, default `NONE`).
    pub height_reference: HeightReference,
    /// The extruded height reference (JS `extrudedHeightReference`, default
    /// `NONE`).
    pub extruded_height_reference: HeightReference,
}

impl EllipseGraphics {
    /// Creates a new ellipse graphics with default values.
    pub fn new() -> Self {
        Self {
            show: true,
            semi_major_axis: None,
            semi_minor_axis: None,
            height: None,
            extruded_height: None,
            rotation: None,
            st_rotation: None,
            granularity: None,
            number_of_vertical_lines: None,
            fill: true,
            outline: false,
            outline_color: Color::BLACK,
            outline_width: 1.0,
            material_color: Color::WHITE,
            z_index: None,
            height_reference: HeightReference::None,
            extruded_height_reference: HeightReference::None,
        }
    }
}

impl Default for EllipseGraphics {
    fn default() -> Self { Self::new() }
}
