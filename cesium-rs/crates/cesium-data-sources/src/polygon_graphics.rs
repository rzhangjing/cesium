//! Ported from `packages/engine/Source/DataSources/PolygonGraphics.js`.
//!
//! DEVIATION (simplified value model): the JS time-dynamic `Property`
//! fields are stored as plain constant values, mirroring the rest of the
//! data-sources port. Sub-properties `shadows`, `distanceDisplayCondition`,
//! `classificationType`, `textureCoordinates` and `z` are not materialized;
//! the updaters apply the JS defaults for them.

use cesium_core::arc_type::ArcType;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;
use cesium_scene::height_reference::HeightReference;

/// Graphics properties for a polygon.
#[derive(Clone)]
pub struct PolygonGraphics {
    /// Whether this polygon is shown.
    pub show: bool,
    /// The polygon hierarchy (outer ring + holes).
    pub hierarchy: Vec<Cartesian3>,
    /// The height above the ellipsoid.
    pub height: Option<f64>,
    /// The extruded height.
    pub extruded_height: Option<f64>,
    /// The material color.
    pub material_color: Color,
    /// Whether to show the outline.
    pub outline: bool,
    /// The outline color.
    pub outline_color: Color,
    /// Whether to extrude the polygon to the ground.
    pub extrude: bool,
    /// Whether to fill the polygon.
    pub fill: bool,
    /// The width of the outline in pixels.
    pub outline_width: f64,
    /// Whether the polygon uses the height of each position
    /// (mirrors `perPositionHeight`; `None` means unset).
    pub per_position_height: Option<bool>,
    /// The holes of the polygon (each hole is a ring of positions).
    pub holes: Vec<Vec<Cartesian3>>,
    /// The type of arc used to connect the positions.
    pub arc_type: ArcType,
    /// The sampling granularity (mirrors `granularity`).
    pub granularity: Option<f64>,
    /// The texture rotation, in radians (mirrors `stRotation`).
    pub st_rotation: Option<f64>,
    /// Whether the top of an extruded polygon is closed (mirrors
    /// `closeTop`, JS default `true`).
    pub close_top: bool,
    /// Whether the bottom of an extruded polygon is closed (mirrors
    /// `closeBottom`, JS default `true`).
    pub close_bottom: bool,
    /// The draw order (mirrors `zIndex`, JS default `0`).
    pub z_index: Option<f64>,
    /// The height reference (mirrors `heightReference`, JS default NONE).
    pub height_reference: HeightReference,
    /// The extruded height reference (mirrors `extrudedHeightReference`,
    /// JS default NONE).
    pub extruded_height_reference: HeightReference,
}

impl PolygonGraphics {
    /// Creates a new polygon graphics with default values.
    pub fn new() -> Self {
        Self {
            show: true,
            hierarchy: Vec::new(),
            height: None,
            extruded_height: None,
            material_color: Color::new(1.0, 1.0, 1.0, 1.0),
            outline: false,
            outline_color: Color::new(0.0, 0.0, 0.0, 1.0),
            extrude: false,
            fill: true,
            outline_width: 1.0,
            per_position_height: None,
            holes: Vec::new(),
            arc_type: ArcType::Geodesic,
            granularity: None,
            st_rotation: None,
            close_top: true,
            close_bottom: true,
            z_index: None,
            height_reference: HeightReference::None,
            extruded_height_reference: HeightReference::None,
        }
    }
}

impl Default for PolygonGraphics {
    fn default() -> Self { Self::new() }
}
