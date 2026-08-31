//! Ported from `packages/engine/Source/DataSources/EllipsoidGraphics.js`.
//!
//! DEVIATION: the Rust value model materializes the JS `Property` objects as
//! plain constants; `shadows`/`distanceDisplayCondition` sub-properties are
//! not materialized (updaters apply the JS defaults).

use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;
use cesium_scene::height_reference::HeightReference;

/// Graphics properties for an ellipsoid.
#[derive(Clone)]
pub struct EllipsoidGraphics {
    /// Whether this ellipsoid is shown (JS `show`, default `true`).
    pub show: bool,
    /// The radii of the ellipsoid (JS `radii`).
    pub radii: Option<Cartesian3>,
    /// The inner radii (JS `innerRadii`; a cut-away ellipsoid when set).
    pub inner_radii: Option<Cartesian3>,
    /// The minimum clock angle (JS `minimumClock`).
    pub minimum_clock: Option<f64>,
    /// The maximum clock angle (JS `maximumClock`).
    pub maximum_clock: Option<f64>,
    /// The minimum cone angle (JS `minimumCone`).
    pub minimum_cone: Option<f64>,
    /// The maximum cone angle (JS `maximumCone`).
    pub maximum_cone: Option<f64>,
    /// The number of stacked partitions (JS `stackPartitions`).
    pub stack_partitions: Option<f64>,
    /// The number of radial partitions (JS `slicePartitions`).
    pub slice_partitions: Option<f64>,
    /// The number of samples per outline ring (JS `subdivisions`).
    pub subdivisions: Option<f64>,
    /// Whether the ellipsoid is filled (JS `fill`, default `true`).
    pub fill: bool,
    /// Whether the ellipsoid is outlined (JS `outline`, default `false`).
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

impl EllipsoidGraphics {
    /// Creates a new ellipsoid graphics with default values.
    pub fn new() -> Self {
        Self {
            show: true,
            radii: None,
            inner_radii: None,
            minimum_clock: None,
            maximum_clock: None,
            minimum_cone: None,
            maximum_cone: None,
            stack_partitions: None,
            slice_partitions: None,
            subdivisions: None,
            fill: true,
            outline: false,
            outline_color: Color::BLACK,
            outline_width: 1.0,
            material_color: Color::WHITE,
            height_reference: HeightReference::None,
        }
    }
}

impl Default for EllipsoidGraphics {
    fn default() -> Self { Self::new() }
}
