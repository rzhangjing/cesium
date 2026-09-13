//! Material properties: properties which represent [`Material`] uniforms.
//!
//! Maps to CesiumJS `DataSources/MaterialProperty.js` and the concrete
//! implementations `ColorMaterialProperty`, `ImageMaterialProperty`,
//! `CheckerboardMaterialProperty`, `GridMaterialProperty`,
//! `StripeMaterialProperty`, `PolylineArrowMaterialProperty`,
//! `PolylineDashMaterialProperty`, `PolylineGlowMaterialProperty`,
//! `PolylineOutlineMaterialProperty` and `CompositeMaterialProperty`.
//!
//! Each material property evaluates to a material type string (e.g. `"Color"`,
//! `"Grid"`) plus a set of named uniform values. The Fabric material system
//! (P1.3) consumes these to build actual shader materials.

use crate::property_system::property::{ConstantProperty, DynProperty};
use crate::property_system::value::PropertyValue;
use cesium_time::{JulianDate, TimeInterval, TimeIntervalCollection, TimeIntervalData};
use glam::DVec2;
use std::any::Any;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Material uniform values keyed by uniform name.
///
/// Maps to the `result` object filled by CesiumJS
/// `MaterialProperty.prototype.getValue(time, result)`.
pub type MaterialUniforms = BTreeMap<String, PropertyValue>;

/// `Color.WHITE` (maps to CesiumJS `Color.WHITE`).
pub const COLOR_WHITE: [f64; 4] = [1.0, 1.0, 1.0, 1.0];
/// `Color.BLACK` (maps to CesiumJS `Color.BLACK`).
pub const COLOR_BLACK: [f64; 4] = [0.0, 0.0, 0.0, 1.0];
/// `Color.TRANSPARENT` (maps to CesiumJS `Color.TRANSPARENT`).
pub const COLOR_TRANSPARENT: [f64; 4] = [0.0, 0.0, 0.0, 0.0];

/// The interface for all properties that represent material uniforms.
///
/// Maps to CesiumJS `DataSources/MaterialProperty.js`.
pub trait MaterialProperty: Send + Sync {
    /// Whether `get_value` always returns the same result for the current
    /// definition. Maps to `isConstant`.
    fn is_constant(&self) -> bool;

    /// Gets the material type at the provided time.
    /// Maps to `MaterialProperty.prototype.getType`.
    fn get_type(&self, time: &JulianDate) -> Option<String>;

    /// Gets the uniform values of the property at the provided time.
    /// Maps to `MaterialProperty.prototype.getValue(time, result)`.
    fn get_value(&self, time: &JulianDate) -> MaterialUniforms;

    /// Compares this property to another.
    /// Maps to `MaterialProperty.prototype.equals`.
    fn equals(&self, other: &dyn MaterialProperty) -> bool;

    /// Enables downcasting to the concrete type.
    fn as_any(&self) -> &dyn Any;
}

/// Compares two trait-object material properties for equality, treating an
/// `Arc` pointer match as equal. Mirrors `Property.equals(left, right)` as
/// used with material properties.
pub fn arc_material_property_equals(
    left: &Arc<dyn MaterialProperty>,
    right: &Arc<dyn MaterialProperty>,
) -> bool {
    Arc::ptr_eq(left, right) || left.equals(right.as_ref())
}

/// Wraps a raw value into a constant property.
fn to_constant(value: PropertyValue) -> Arc<dyn DynProperty> {
    Arc::new(ConstantProperty::new(value))
}

/// Maps to `Property.getValueOrClonedDefault` / `Property.getValueOrDefault`:
/// evaluates the property at `time`, falling back to `default` when the
/// property is absent or yields undefined.
fn value_or_default(
    property: &Option<Arc<dyn DynProperty>>,
    time: &JulianDate,
    default: PropertyValue,
) -> PropertyValue {
    match property {
        Some(p) => {
            let v = p.get_value(time);
            if v.is_undefined() {
                default
            } else {
                v
            }
        }
        None => default,
    }
}

/// Maps to `Property.getValueOrUndefined`.
fn value_or_undefined(
    property: &Option<Arc<dyn DynProperty>>,
    time: &JulianDate,
) -> PropertyValue {
    match property {
        Some(p) => p.get_value(time),
        None => PropertyValue::Undefined,
    }
}

/// Maps to `Property.isConstant(property)` for optional properties.
fn option_is_constant(property: &Option<Arc<dyn DynProperty>>) -> bool {
    match property {
        None => true,
        Some(p) => p.is_constant(),
    }
}

/// Maps to `Property.equals(left, right)` for optional properties.
fn option_equals(
    left: &Option<Arc<dyn DynProperty>>,
    right: &Option<Arc<dyn DynProperty>>,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(l), Some(r)) => Arc::ptr_eq(l, r) || l.equals(r.as_ref()),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// ColorMaterialProperty
// ---------------------------------------------------------------------------

/// A material property that maps to solid color material uniforms.
///
/// Maps to CesiumJS `DataSources/ColorMaterialProperty.js`.
#[derive(Clone)]
pub struct ColorMaterialProperty {
    color: Option<Arc<dyn DynProperty>>,
}

impl ColorMaterialProperty {
    /// Creates a new color material property. `color` may be a
    /// `PropertyValue::Color`; other property kinds can be assigned via
    /// [`set_color_property`](Self::set_color_property).
    /// Maps to `new ColorMaterialProperty(color)`.
    pub fn new(color: Option<PropertyValue>) -> Self {
        Self {
            color: color.map(to_constant),
        }
    }

    /// Creates a color material property from a constant RGBA color.
    pub fn from_color(color: [f64; 4]) -> Self {
        Self::new(Some(PropertyValue::Color(color)))
    }

    /// The color property. Maps to `color`.
    pub fn color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.color.as_ref()
    }

    /// Sets the color property. Maps to the `color` setter.
    pub fn set_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.color = property;
    }

    /// Sets the color as a constant value.
    pub fn set_color(&mut self, color: Option<PropertyValue>) {
        self.color = color.map(to_constant);
    }
}

impl MaterialProperty for ColorMaterialProperty {
    fn is_constant(&self) -> bool {
        option_is_constant(&self.color)
    }

    fn get_type(&self, _time: &JulianDate) -> Option<String> {
        Some("Color".to_string())
    }

    fn get_value(&self, time: &JulianDate) -> MaterialUniforms {
        let mut uniforms = MaterialUniforms::new();
        uniforms.insert(
            "color".to_string(),
            value_or_default(&self.color, time, PropertyValue::Color(COLOR_WHITE)),
        );
        uniforms
    }

    fn equals(&self, other: &dyn MaterialProperty) -> bool {
        match other.as_any().downcast_ref::<ColorMaterialProperty>() {
            Some(o) => option_equals(&self.color, &o.color),
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// ImageMaterialProperty
// ---------------------------------------------------------------------------

/// A material property that maps to image material uniforms.
///
/// Maps to CesiumJS `DataSources/ImageMaterialProperty.js`.
#[derive(Clone, Default)]
pub struct ImageMaterialProperty {
    image: Option<Arc<dyn DynProperty>>,
    repeat: Option<Arc<dyn DynProperty>>,
    color: Option<Arc<dyn DynProperty>>,
    transparent: Option<Arc<dyn DynProperty>>,
}

impl ImageMaterialProperty {
    /// Creates a new image material property with all values defaulted.
    pub fn new() -> Self {
        Self::default()
    }

    /// The image property (URL/canvas/etc). Maps to `image`.
    pub fn image_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.image.as_ref()
    }

    /// Sets the image property. Maps to the `image` setter.
    pub fn set_image_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.image = property;
    }

    /// Sets the image as a constant value (typically a URL string).
    pub fn set_image(&mut self, image: Option<PropertyValue>) {
        self.image = image.map(to_constant);
    }

    /// The repeat property. Maps to `repeat`.
    pub fn repeat_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.repeat.as_ref()
    }

    /// Sets the repeat property. Maps to the `repeat` setter.
    pub fn set_repeat_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.repeat = property;
    }

    /// Sets the repeat as a constant value.
    pub fn set_repeat(&mut self, repeat: Option<PropertyValue>) {
        self.repeat = repeat.map(to_constant);
    }

    /// The color property. Maps to `color`.
    pub fn color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.color.as_ref()
    }

    /// Sets the color property. Maps to the `color` setter.
    pub fn set_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.color = property;
    }

    /// Sets the color as a constant value.
    pub fn set_color(&mut self, color: Option<PropertyValue>) {
        self.color = color.map(to_constant);
    }

    /// The transparent property. Maps to `transparent`.
    pub fn transparent_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.transparent.as_ref()
    }

    /// Sets the transparent property. Maps to the `transparent` setter.
    pub fn set_transparent_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.transparent = property;
    }

    /// Sets the transparent flag as a constant value.
    pub fn set_transparent(&mut self, transparent: Option<PropertyValue>) {
        self.transparent = transparent.map(to_constant);
    }
}

impl MaterialProperty for ImageMaterialProperty {
    fn is_constant(&self) -> bool {
        option_is_constant(&self.image) && option_is_constant(&self.repeat)
    }

    fn get_type(&self, _time: &JulianDate) -> Option<String> {
        Some("Image".to_string())
    }

    fn get_value(&self, time: &JulianDate) -> MaterialUniforms {
        let mut uniforms = MaterialUniforms::new();
        uniforms.insert(
            "image".to_string(),
            value_or_undefined(&self.image, time),
        );
        uniforms.insert(
            "repeat".to_string(),
            value_or_default(
                &self.repeat,
                time,
                PropertyValue::Cartesian2(DVec2::new(1.0, 1.0)),
            ),
        );
        let mut color = value_or_default(&self.color, time, PropertyValue::Color(COLOR_WHITE));
        let transparent = value_or_default(
            &self.transparent,
            time,
            PropertyValue::Boolean(false),
        );
        if matches!(transparent, PropertyValue::Boolean(true)) {
            if let PropertyValue::Color(ref mut c) = color {
                c[3] = c[3].min(0.99);
            }
        }
        uniforms.insert("color".to_string(), color);
        uniforms
    }

    fn equals(&self, other: &dyn MaterialProperty) -> bool {
        match other.as_any().downcast_ref::<ImageMaterialProperty>() {
            Some(o) => {
                option_equals(&self.image, &o.image)
                    && option_equals(&self.repeat, &o.repeat)
                    && option_equals(&self.color, &o.color)
                    && option_equals(&self.transparent, &o.transparent)
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// CheckerboardMaterialProperty
// ---------------------------------------------------------------------------

/// A material property that maps to checkerboard material uniforms.
///
/// Maps to CesiumJS `DataSources/CheckerboardMaterialProperty.js`.
#[derive(Clone, Default)]
pub struct CheckerboardMaterialProperty {
    even_color: Option<Arc<dyn DynProperty>>,
    odd_color: Option<Arc<dyn DynProperty>>,
    repeat: Option<Arc<dyn DynProperty>>,
}

impl CheckerboardMaterialProperty {
    /// Creates a new checkerboard material property with all values defaulted.
    pub fn new() -> Self {
        Self::default()
    }

    /// The even color property. Maps to `evenColor`.
    pub fn even_color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.even_color.as_ref()
    }

    /// Sets the even color property. Maps to the `evenColor` setter.
    pub fn set_even_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.even_color = property;
    }

    /// Sets the even color as a constant value.
    pub fn set_even_color(&mut self, color: Option<PropertyValue>) {
        self.even_color = color.map(to_constant);
    }

    /// The odd color property. Maps to `oddColor`.
    pub fn odd_color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.odd_color.as_ref()
    }

    /// Sets the odd color property. Maps to the `oddColor` setter.
    pub fn set_odd_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.odd_color = property;
    }

    /// Sets the odd color as a constant value.
    pub fn set_odd_color(&mut self, color: Option<PropertyValue>) {
        self.odd_color = color.map(to_constant);
    }

    /// The repeat property. Maps to `repeat`.
    pub fn repeat_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.repeat.as_ref()
    }

    /// Sets the repeat property. Maps to the `repeat` setter.
    pub fn set_repeat_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.repeat = property;
    }

    /// Sets the repeat as a constant value.
    pub fn set_repeat(&mut self, repeat: Option<PropertyValue>) {
        self.repeat = repeat.map(to_constant);
    }
}

impl MaterialProperty for CheckerboardMaterialProperty {
    fn is_constant(&self) -> bool {
        option_is_constant(&self.even_color)
            && option_is_constant(&self.odd_color)
            && option_is_constant(&self.repeat)
    }

    fn get_type(&self, _time: &JulianDate) -> Option<String> {
        Some("Checkerboard".to_string())
    }

    fn get_value(&self, time: &JulianDate) -> MaterialUniforms {
        let mut uniforms = MaterialUniforms::new();
        uniforms.insert(
            "lightColor".to_string(),
            value_or_default(&self.even_color, time, PropertyValue::Color(COLOR_WHITE)),
        );
        uniforms.insert(
            "darkColor".to_string(),
            value_or_default(&self.odd_color, time, PropertyValue::Color(COLOR_BLACK)),
        );
        uniforms.insert(
            "repeat".to_string(),
            value_or_default(
                &self.repeat,
                time,
                PropertyValue::Cartesian2(DVec2::new(2.0, 2.0)),
            ),
        );
        uniforms
    }

    fn equals(&self, other: &dyn MaterialProperty) -> bool {
        match other.as_any().downcast_ref::<CheckerboardMaterialProperty>() {
            Some(o) => {
                option_equals(&self.even_color, &o.even_color)
                    && option_equals(&self.odd_color, &o.odd_color)
                    && option_equals(&self.repeat, &o.repeat)
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// GridMaterialProperty
// ---------------------------------------------------------------------------

/// A material property that maps to grid material uniforms.
///
/// Maps to CesiumJS `DataSources/GridMaterialProperty.js`.
#[derive(Clone, Default)]
pub struct GridMaterialProperty {
    color: Option<Arc<dyn DynProperty>>,
    cell_alpha: Option<Arc<dyn DynProperty>>,
    line_count: Option<Arc<dyn DynProperty>>,
    line_thickness: Option<Arc<dyn DynProperty>>,
    line_offset: Option<Arc<dyn DynProperty>>,
}

impl GridMaterialProperty {
    /// Creates a new grid material property with all values defaulted.
    pub fn new() -> Self {
        Self::default()
    }

    /// The color property. Maps to `color`.
    pub fn color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.color.as_ref()
    }

    /// Sets the color property. Maps to the `color` setter.
    pub fn set_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.color = property;
    }

    /// Sets the color as a constant value.
    pub fn set_color(&mut self, color: Option<PropertyValue>) {
        self.color = color.map(to_constant);
    }

    /// The cell alpha property. Maps to `cellAlpha`.
    pub fn cell_alpha_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.cell_alpha.as_ref()
    }

    /// Sets the cell alpha property. Maps to the `cellAlpha` setter.
    pub fn set_cell_alpha_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.cell_alpha = property;
    }

    /// Sets the cell alpha as a constant value.
    pub fn set_cell_alpha(&mut self, cell_alpha: Option<PropertyValue>) {
        self.cell_alpha = cell_alpha.map(to_constant);
    }

    /// The line count property. Maps to `lineCount`.
    pub fn line_count_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.line_count.as_ref()
    }

    /// Sets the line count property. Maps to the `lineCount` setter.
    pub fn set_line_count_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.line_count = property;
    }

    /// Sets the line count as a constant value.
    pub fn set_line_count(&mut self, line_count: Option<PropertyValue>) {
        self.line_count = line_count.map(to_constant);
    }

    /// The line thickness property. Maps to `lineThickness`.
    pub fn line_thickness_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.line_thickness.as_ref()
    }

    /// Sets the line thickness property. Maps to the `lineThickness` setter.
    pub fn set_line_thickness_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.line_thickness = property;
    }

    /// Sets the line thickness as a constant value.
    pub fn set_line_thickness(&mut self, line_thickness: Option<PropertyValue>) {
        self.line_thickness = line_thickness.map(to_constant);
    }

    /// The line offset property. Maps to `lineOffset`.
    pub fn line_offset_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.line_offset.as_ref()
    }

    /// Sets the line offset property. Maps to the `lineOffset` setter.
    pub fn set_line_offset_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.line_offset = property;
    }

    /// Sets the line offset as a constant value.
    pub fn set_line_offset(&mut self, line_offset: Option<PropertyValue>) {
        self.line_offset = line_offset.map(to_constant);
    }
}

impl MaterialProperty for GridMaterialProperty {
    fn is_constant(&self) -> bool {
        option_is_constant(&self.color)
            && option_is_constant(&self.cell_alpha)
            && option_is_constant(&self.line_count)
            && option_is_constant(&self.line_thickness)
            && option_is_constant(&self.line_offset)
    }

    fn get_type(&self, _time: &JulianDate) -> Option<String> {
        Some("Grid".to_string())
    }

    fn get_value(&self, time: &JulianDate) -> MaterialUniforms {
        let mut uniforms = MaterialUniforms::new();
        uniforms.insert(
            "color".to_string(),
            value_or_default(&self.color, time, PropertyValue::Color(COLOR_WHITE)),
        );
        uniforms.insert(
            "cellAlpha".to_string(),
            value_or_default(&self.cell_alpha, time, PropertyValue::Number(0.1)),
        );
        uniforms.insert(
            "lineCount".to_string(),
            value_or_default(
                &self.line_count,
                time,
                PropertyValue::Cartesian2(DVec2::new(8.0, 8.0)),
            ),
        );
        uniforms.insert(
            "lineThickness".to_string(),
            value_or_default(
                &self.line_thickness,
                time,
                PropertyValue::Cartesian2(DVec2::new(1.0, 1.0)),
            ),
        );
        uniforms.insert(
            "lineOffset".to_string(),
            value_or_default(
                &self.line_offset,
                time,
                PropertyValue::Cartesian2(DVec2::new(0.0, 0.0)),
            ),
        );
        uniforms
    }

    fn equals(&self, other: &dyn MaterialProperty) -> bool {
        match other.as_any().downcast_ref::<GridMaterialProperty>() {
            Some(o) => {
                option_equals(&self.color, &o.color)
                    && option_equals(&self.cell_alpha, &o.cell_alpha)
                    && option_equals(&self.line_count, &o.line_count)
                    && option_equals(&self.line_thickness, &o.line_thickness)
                    && option_equals(&self.line_offset, &o.line_offset)
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// StripeMaterialProperty
// ---------------------------------------------------------------------------

/// The orientation of stripes in a `StripeMaterialProperty`.
///
/// Maps to CesiumJS `DataSources/StripeOrientation.js`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StripeOrientation {
    /// Horizontal orientation (`StripeOrientation.HORIZONTAL` = 0).
    #[default]
    Horizontal,
    /// Vertical orientation (`StripeOrientation.VERTICAL` = 1).
    Vertical,
}

impl StripeOrientation {
    /// Converts to the numeric representation used by CesiumJS.
    pub fn to_number(self) -> f64 {
        match self {
            StripeOrientation::Horizontal => 0.0,
            StripeOrientation::Vertical => 1.0,
        }
    }

    /// Converts to a `PropertyValue::Number`.
    pub fn to_value(self) -> PropertyValue {
        PropertyValue::Number(self.to_number())
    }

    /// Parses from a property value. Anything other than the number `1.0`
    /// yields `Horizontal` (matching CesiumJS's `=== StripeOrientation.HORIZONTAL`
    /// comparison semantics where the default applies).
    pub fn from_value(value: &PropertyValue) -> Self {
        match value {
            PropertyValue::Number(n) if *n == 1.0 => StripeOrientation::Vertical,
            _ => StripeOrientation::Horizontal,
        }
    }
}

/// A material property that maps to stripe material uniforms.
///
/// Maps to CesiumJS `DataSources/StripeMaterialProperty.js`.
#[derive(Clone, Default)]
pub struct StripeMaterialProperty {
    orientation: Option<Arc<dyn DynProperty>>,
    even_color: Option<Arc<dyn DynProperty>>,
    odd_color: Option<Arc<dyn DynProperty>>,
    offset: Option<Arc<dyn DynProperty>>,
    repeat: Option<Arc<dyn DynProperty>>,
}

impl StripeMaterialProperty {
    /// Creates a new stripe material property with all values defaulted.
    pub fn new() -> Self {
        Self::default()
    }

    /// The orientation property. Maps to `orientation`.
    pub fn orientation_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.orientation.as_ref()
    }

    /// Sets the orientation property. Maps to the `orientation` setter.
    pub fn set_orientation_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.orientation = property;
    }

    /// Sets the orientation as a constant value.
    pub fn set_orientation(&mut self, orientation: StripeOrientation) {
        self.orientation = Some(to_constant(orientation.to_value()));
    }

    /// The even color property. Maps to `evenColor`.
    pub fn even_color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.even_color.as_ref()
    }

    /// Sets the even color property. Maps to the `evenColor` setter.
    pub fn set_even_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.even_color = property;
    }

    /// Sets the even color as a constant value.
    pub fn set_even_color(&mut self, color: Option<PropertyValue>) {
        self.even_color = color.map(to_constant);
    }

    /// The odd color property. Maps to `oddColor`.
    pub fn odd_color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.odd_color.as_ref()
    }

    /// Sets the odd color property. Maps to the `oddColor` setter.
    pub fn set_odd_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.odd_color = property;
    }

    /// Sets the odd color as a constant value.
    pub fn set_odd_color(&mut self, color: Option<PropertyValue>) {
        self.odd_color = color.map(to_constant);
    }

    /// The offset property. Maps to `offset`.
    pub fn offset_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.offset.as_ref()
    }

    /// Sets the offset property. Maps to the `offset` setter.
    pub fn set_offset_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.offset = property;
    }

    /// Sets the offset as a constant value.
    pub fn set_offset(&mut self, offset: Option<PropertyValue>) {
        self.offset = offset.map(to_constant);
    }

    /// The repeat property. Maps to `repeat`.
    pub fn repeat_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.repeat.as_ref()
    }

    /// Sets the repeat property. Maps to the `repeat` setter.
    pub fn set_repeat_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.repeat = property;
    }

    /// Sets the repeat as a constant value.
    pub fn set_repeat(&mut self, repeat: Option<PropertyValue>) {
        self.repeat = repeat.map(to_constant);
    }
}

impl MaterialProperty for StripeMaterialProperty {
    fn is_constant(&self) -> bool {
        option_is_constant(&self.orientation)
            && option_is_constant(&self.even_color)
            && option_is_constant(&self.odd_color)
            && option_is_constant(&self.offset)
            && option_is_constant(&self.repeat)
    }

    fn get_type(&self, _time: &JulianDate) -> Option<String> {
        Some("Stripe".to_string())
    }

    fn get_value(&self, time: &JulianDate) -> MaterialUniforms {
        let mut uniforms = MaterialUniforms::new();
        let orientation_value = value_or_default(
            &self.orientation,
            time,
            StripeOrientation::Horizontal.to_value(),
        );
        let horizontal =
            StripeOrientation::from_value(&orientation_value) == StripeOrientation::Horizontal;
        uniforms.insert("horizontal".to_string(), PropertyValue::Boolean(horizontal));
        uniforms.insert(
            "evenColor".to_string(),
            value_or_default(&self.even_color, time, PropertyValue::Color(COLOR_WHITE)),
        );
        uniforms.insert(
            "oddColor".to_string(),
            value_or_default(&self.odd_color, time, PropertyValue::Color(COLOR_BLACK)),
        );
        uniforms.insert(
            "offset".to_string(),
            value_or_default(&self.offset, time, PropertyValue::Number(0.0)),
        );
        uniforms.insert(
            "repeat".to_string(),
            value_or_default(&self.repeat, time, PropertyValue::Number(1.0)),
        );
        uniforms
    }

    fn equals(&self, other: &dyn MaterialProperty) -> bool {
        match other.as_any().downcast_ref::<StripeMaterialProperty>() {
            Some(o) => {
                option_equals(&self.orientation, &o.orientation)
                    && option_equals(&self.even_color, &o.even_color)
                    && option_equals(&self.odd_color, &o.odd_color)
                    && option_equals(&self.offset, &o.offset)
                    && option_equals(&self.repeat, &o.repeat)
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// PolylineArrowMaterialProperty
// ---------------------------------------------------------------------------

/// A material property that maps to PolylineArrow material uniforms.
///
/// Maps to CesiumJS `DataSources/PolylineArrowMaterialProperty.js`.
#[derive(Clone)]
pub struct PolylineArrowMaterialProperty {
    color: Option<Arc<dyn DynProperty>>,
}

impl PolylineArrowMaterialProperty {
    /// Creates a new polyline arrow material property.
    /// Maps to `new PolylineArrowMaterialProperty(color)`.
    pub fn new(color: Option<PropertyValue>) -> Self {
        Self {
            color: color.map(to_constant),
        }
    }

    /// The color property. Maps to `color`.
    pub fn color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.color.as_ref()
    }

    /// Sets the color property. Maps to the `color` setter.
    pub fn set_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.color = property;
    }

    /// Sets the color as a constant value.
    pub fn set_color(&mut self, color: Option<PropertyValue>) {
        self.color = color.map(to_constant);
    }
}

impl MaterialProperty for PolylineArrowMaterialProperty {
    fn is_constant(&self) -> bool {
        option_is_constant(&self.color)
    }

    fn get_type(&self, _time: &JulianDate) -> Option<String> {
        Some("PolylineArrow".to_string())
    }

    fn get_value(&self, time: &JulianDate) -> MaterialUniforms {
        let mut uniforms = MaterialUniforms::new();
        uniforms.insert(
            "color".to_string(),
            value_or_default(&self.color, time, PropertyValue::Color(COLOR_WHITE)),
        );
        uniforms
    }

    fn equals(&self, other: &dyn MaterialProperty) -> bool {
        match other.as_any().downcast_ref::<PolylineArrowMaterialProperty>() {
            Some(o) => option_equals(&self.color, &o.color),
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// PolylineDashMaterialProperty
// ---------------------------------------------------------------------------

/// A material property that maps to polyline dash material uniforms.
///
/// Maps to CesiumJS `DataSources/PolylineDashMaterialProperty.js`.
#[derive(Clone, Default)]
pub struct PolylineDashMaterialProperty {
    color: Option<Arc<dyn DynProperty>>,
    gap_color: Option<Arc<dyn DynProperty>>,
    dash_length: Option<Arc<dyn DynProperty>>,
    dash_pattern: Option<Arc<dyn DynProperty>>,
}

impl PolylineDashMaterialProperty {
    /// Creates a new polyline dash material property with all values defaulted.
    pub fn new() -> Self {
        Self::default()
    }

    /// The color property. Maps to `color`.
    pub fn color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.color.as_ref()
    }

    /// Sets the color property. Maps to the `color` setter.
    pub fn set_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.color = property;
    }

    /// Sets the color as a constant value.
    pub fn set_color(&mut self, color: Option<PropertyValue>) {
        self.color = color.map(to_constant);
    }

    /// The gap color property. Maps to `gapColor`.
    pub fn gap_color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.gap_color.as_ref()
    }

    /// Sets the gap color property. Maps to the `gapColor` setter.
    pub fn set_gap_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.gap_color = property;
    }

    /// Sets the gap color as a constant value.
    pub fn set_gap_color(&mut self, color: Option<PropertyValue>) {
        self.gap_color = color.map(to_constant);
    }

    /// The dash length property. Maps to `dashLength`.
    pub fn dash_length_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.dash_length.as_ref()
    }

    /// Sets the dash length property. Maps to the `dashLength` setter.
    pub fn set_dash_length_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.dash_length = property;
    }

    /// Sets the dash length as a constant value.
    pub fn set_dash_length(&mut self, dash_length: Option<PropertyValue>) {
        self.dash_length = dash_length.map(to_constant);
    }

    /// The dash pattern property. Maps to `dashPattern`.
    pub fn dash_pattern_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.dash_pattern.as_ref()
    }

    /// Sets the dash pattern property. Maps to the `dashPattern` setter.
    pub fn set_dash_pattern_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.dash_pattern = property;
    }

    /// Sets the dash pattern as a constant value.
    pub fn set_dash_pattern(&mut self, dash_pattern: Option<PropertyValue>) {
        self.dash_pattern = dash_pattern.map(to_constant);
    }
}

impl MaterialProperty for PolylineDashMaterialProperty {
    fn is_constant(&self) -> bool {
        option_is_constant(&self.color)
            && option_is_constant(&self.gap_color)
            && option_is_constant(&self.dash_length)
            && option_is_constant(&self.dash_pattern)
    }

    fn get_type(&self, _time: &JulianDate) -> Option<String> {
        Some("PolylineDash".to_string())
    }

    fn get_value(&self, time: &JulianDate) -> MaterialUniforms {
        let mut uniforms = MaterialUniforms::new();
        uniforms.insert(
            "color".to_string(),
            value_or_default(&self.color, time, PropertyValue::Color(COLOR_WHITE)),
        );
        uniforms.insert(
            "gapColor".to_string(),
            value_or_default(
                &self.gap_color,
                time,
                PropertyValue::Color(COLOR_TRANSPARENT),
            ),
        );
        uniforms.insert(
            "dashLength".to_string(),
            value_or_default(&self.dash_length, time, PropertyValue::Number(16.0)),
        );
        uniforms.insert(
            "dashPattern".to_string(),
            value_or_default(&self.dash_pattern, time, PropertyValue::Number(255.0)),
        );
        uniforms
    }

    fn equals(&self, other: &dyn MaterialProperty) -> bool {
        match other.as_any().downcast_ref::<PolylineDashMaterialProperty>() {
            Some(o) => {
                option_equals(&self.color, &o.color)
                    && option_equals(&self.gap_color, &o.gap_color)
                    && option_equals(&self.dash_length, &o.dash_length)
                    && option_equals(&self.dash_pattern, &o.dash_pattern)
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// PolylineGlowMaterialProperty
// ---------------------------------------------------------------------------

/// A material property that maps to polyline glow material uniforms.
///
/// Maps to CesiumJS `DataSources/PolylineGlowMaterialProperty.js`.
#[derive(Clone, Default)]
pub struct PolylineGlowMaterialProperty {
    color: Option<Arc<dyn DynProperty>>,
    glow_power: Option<Arc<dyn DynProperty>>,
    taper_power: Option<Arc<dyn DynProperty>>,
}

impl PolylineGlowMaterialProperty {
    /// Creates a new polyline glow material property with all values defaulted.
    pub fn new() -> Self {
        Self::default()
    }

    /// The color property. Maps to `color`.
    pub fn color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.color.as_ref()
    }

    /// Sets the color property. Maps to the `color` setter.
    pub fn set_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.color = property;
    }

    /// Sets the color as a constant value.
    pub fn set_color(&mut self, color: Option<PropertyValue>) {
        self.color = color.map(to_constant);
    }

    /// The glow power property. Maps to `glowPower`.
    pub fn glow_power_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.glow_power.as_ref()
    }

    /// Sets the glow power property. Maps to the `glowPower` setter.
    pub fn set_glow_power_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.glow_power = property;
    }

    /// Sets the glow power as a constant value.
    pub fn set_glow_power(&mut self, glow_power: Option<PropertyValue>) {
        self.glow_power = glow_power.map(to_constant);
    }

    /// The taper power property. Maps to `taperPower`.
    pub fn taper_power_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.taper_power.as_ref()
    }

    /// Sets the taper power property. Maps to the `taperPower` setter.
    pub fn set_taper_power_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.taper_power = property;
    }

    /// Sets the taper power as a constant value.
    pub fn set_taper_power(&mut self, taper_power: Option<PropertyValue>) {
        self.taper_power = taper_power.map(to_constant);
    }
}

impl MaterialProperty for PolylineGlowMaterialProperty {
    fn is_constant(&self) -> bool {
        // Note: CesiumJS checks `Property.isConstant(this._glow)` here, which
        // references a nonexistent field and thus always passes; the intended
        // semantics (and the fields compared in `equals`) are color, glowPower
        // and taperPower, which is what we implement.
        option_is_constant(&self.color)
            && option_is_constant(&self.glow_power)
            && option_is_constant(&self.taper_power)
    }

    fn get_type(&self, _time: &JulianDate) -> Option<String> {
        Some("PolylineGlow".to_string())
    }

    fn get_value(&self, time: &JulianDate) -> MaterialUniforms {
        let mut uniforms = MaterialUniforms::new();
        uniforms.insert(
            "color".to_string(),
            value_or_default(&self.color, time, PropertyValue::Color(COLOR_WHITE)),
        );
        uniforms.insert(
            "glowPower".to_string(),
            value_or_default(&self.glow_power, time, PropertyValue::Number(0.25)),
        );
        uniforms.insert(
            "taperPower".to_string(),
            value_or_default(&self.taper_power, time, PropertyValue::Number(1.0)),
        );
        uniforms
    }

    fn equals(&self, other: &dyn MaterialProperty) -> bool {
        match other.as_any().downcast_ref::<PolylineGlowMaterialProperty>() {
            Some(o) => {
                option_equals(&self.color, &o.color)
                    && option_equals(&self.glow_power, &o.glow_power)
                    && option_equals(&self.taper_power, &o.taper_power)
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// PolylineOutlineMaterialProperty
// ---------------------------------------------------------------------------

/// A material property that maps to polyline outline material uniforms.
///
/// Maps to CesiumJS `DataSources/PolylineOutlineMaterialProperty.js`.
#[derive(Clone, Default)]
pub struct PolylineOutlineMaterialProperty {
    color: Option<Arc<dyn DynProperty>>,
    outline_color: Option<Arc<dyn DynProperty>>,
    outline_width: Option<Arc<dyn DynProperty>>,
}

impl PolylineOutlineMaterialProperty {
    /// Creates a new polyline outline material property with all values
    /// defaulted.
    pub fn new() -> Self {
        Self::default()
    }

    /// The color property. Maps to `color`.
    pub fn color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.color.as_ref()
    }

    /// Sets the color property. Maps to the `color` setter.
    pub fn set_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.color = property;
    }

    /// Sets the color as a constant value.
    pub fn set_color(&mut self, color: Option<PropertyValue>) {
        self.color = color.map(to_constant);
    }

    /// The outline color property. Maps to `outlineColor`.
    pub fn outline_color_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.outline_color.as_ref()
    }

    /// Sets the outline color property. Maps to the `outlineColor` setter.
    pub fn set_outline_color_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.outline_color = property;
    }

    /// Sets the outline color as a constant value.
    pub fn set_outline_color(&mut self, color: Option<PropertyValue>) {
        self.outline_color = color.map(to_constant);
    }

    /// The outline width property. Maps to `outlineWidth`.
    pub fn outline_width_property(&self) -> Option<&Arc<dyn DynProperty>> {
        self.outline_width.as_ref()
    }

    /// Sets the outline width property. Maps to the `outlineWidth` setter.
    pub fn set_outline_width_property(&mut self, property: Option<Arc<dyn DynProperty>>) {
        self.outline_width = property;
    }

    /// Sets the outline width as a constant value.
    pub fn set_outline_width(&mut self, width: Option<PropertyValue>) {
        self.outline_width = width.map(to_constant);
    }
}

impl MaterialProperty for PolylineOutlineMaterialProperty {
    fn is_constant(&self) -> bool {
        option_is_constant(&self.color)
            && option_is_constant(&self.outline_color)
            && option_is_constant(&self.outline_width)
    }

    fn get_type(&self, _time: &JulianDate) -> Option<String> {
        Some("PolylineOutline".to_string())
    }

    fn get_value(&self, time: &JulianDate) -> MaterialUniforms {
        let mut uniforms = MaterialUniforms::new();
        uniforms.insert(
            "color".to_string(),
            value_or_default(&self.color, time, PropertyValue::Color(COLOR_WHITE)),
        );
        uniforms.insert(
            "outlineColor".to_string(),
            value_or_default(
                &self.outline_color,
                time,
                PropertyValue::Color(COLOR_BLACK),
            ),
        );
        uniforms.insert(
            "outlineWidth".to_string(),
            value_or_default(&self.outline_width, time, PropertyValue::Number(1.0)),
        );
        uniforms
    }

    fn equals(&self, other: &dyn MaterialProperty) -> bool {
        match other.as_any().downcast_ref::<PolylineOutlineMaterialProperty>() {
            Some(o) => {
                option_equals(&self.color, &o.color)
                    && option_equals(&self.outline_color, &o.outline_color)
                    && option_equals(&self.outline_width, &o.outline_width)
            }
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// CompositeMaterialProperty
// ---------------------------------------------------------------------------

fn material_same_data(
    left: &Arc<dyn MaterialProperty>,
    right: &Arc<dyn MaterialProperty>,
) -> bool {
    arc_material_property_equals(left, right)
}

/// A `CompositeProperty` which is also a `MaterialProperty`.
///
/// Each interval's data is itself a material property; evaluation delegates
/// to the inner property.
///
/// Maps to CesiumJS `DataSources/CompositeMaterialProperty.js`.
#[derive(Clone, Default)]
pub struct CompositeMaterialProperty {
    intervals: TimeIntervalCollection<Arc<dyn MaterialProperty>>,
}

impl CompositeMaterialProperty {
    /// Creates an empty composite material property.
    pub fn new() -> Self {
        Self::default()
    }

    /// The underlying interval collection. Maps to `intervals`.
    pub fn intervals(&self) -> &TimeIntervalCollection<Arc<dyn MaterialProperty>> {
        &self.intervals
    }

    /// Adds an interval whose data is another material property.
    pub fn add_interval(
        &mut self,
        interval: TimeInterval,
        data: Option<Arc<dyn MaterialProperty>>,
    ) {
        let tid = TimeIntervalData::new(interval, data);
        self.intervals.add_interval(tid, &material_same_data);
    }
}

impl MaterialProperty for CompositeMaterialProperty {
    fn is_constant(&self) -> bool {
        self.intervals.is_empty()
    }

    fn get_type(&self, time: &JulianDate) -> Option<String> {
        self.intervals
            .find_data_for_interval_containing_date(time)?
            .get_type(time)
    }

    fn get_value(&self, time: &JulianDate) -> MaterialUniforms {
        match self.intervals.find_data_for_interval_containing_date(time) {
            Some(inner) => inner.get_value(time),
            None => MaterialUniforms::new(),
        }
    }

    fn equals(&self, other: &dyn MaterialProperty) -> bool {
        match other.as_any().downcast_ref::<CompositeMaterialProperty>() {
            Some(o) => self.intervals.equals(&o.intervals, &material_same_data),
            None => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::property_system::property::SampledProperty;
    use crate::property_system::value::PackableType;

    fn t(seconds: f64) -> JulianDate {
        JulianDate::new(2451545.0, seconds)
    }

    fn uniform<'a>(uniforms: &'a MaterialUniforms, name: &str) -> &'a PropertyValue {
        uniforms.get(name).unwrap_or_else(|| panic!("missing uniform {name}"))
    }

    #[test]
    fn test_color_material_defaults() {
        let prop = ColorMaterialProperty::new(None);
        assert!(prop.is_constant());
        assert_eq!(prop.get_type(&t(0.0)), Some("Color".to_string()));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "color"),
            &PropertyValue::Color(COLOR_WHITE)
        );
    }

    #[test]
    fn test_color_material_custom_color() {
        let red = [1.0, 0.0, 0.0, 1.0];
        let prop = ColorMaterialProperty::from_color(red);
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(uniform(&uniforms, "color"), &PropertyValue::Color(red));
    }

    #[test]
    fn test_color_material_dynamic() {
        let mut sampled = SampledProperty::new(PackableType::Color);
        sampled.add_sample(t(0.0), &PropertyValue::Color([1.0, 0.0, 0.0, 1.0]), &[]);
        sampled.add_sample(t(10.0), &PropertyValue::Color([0.0, 0.0, 1.0, 1.0]), &[]);

        let mut prop = ColorMaterialProperty::new(None);
        prop.set_color_property(Some(Arc::new(sampled)));
        assert!(!prop.is_constant());

        let uniforms = prop.get_value(&t(5.0));
        assert_eq!(
            uniform(&uniforms, "color"),
            &PropertyValue::Color([0.5, 0.0, 0.5, 1.0])
        );
    }

    #[test]
    fn test_color_material_equals() {
        let a = ColorMaterialProperty::from_color([1.0, 0.0, 0.0, 1.0]);
        let b = ColorMaterialProperty::from_color([1.0, 0.0, 0.0, 1.0]);
        let c = ColorMaterialProperty::from_color([0.0, 1.0, 0.0, 1.0]);
        assert!(a.equals(&b));
        assert!(!a.equals(&c));
    }

    #[test]
    fn test_image_material_defaults() {
        let prop = ImageMaterialProperty::new();
        assert!(prop.is_constant());
        assert_eq!(prop.get_type(&t(0.0)), Some("Image".to_string()));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(uniform(&uniforms, "image"), &PropertyValue::Undefined);
        assert_eq!(
            uniform(&uniforms, "repeat"),
            &PropertyValue::Cartesian2(DVec2::new(1.0, 1.0))
        );
        assert_eq!(
            uniform(&uniforms, "color"),
            &PropertyValue::Color(COLOR_WHITE)
        );
    }

    #[test]
    fn test_image_material_transparent_caps_alpha() {
        let mut prop = ImageMaterialProperty::new();
        prop.set_image(Some(PropertyValue::Text("test.png".to_string())));
        prop.set_transparent(Some(PropertyValue::Boolean(true)));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "image"),
            &PropertyValue::Text("test.png".to_string())
        );
        // Default WHITE alpha 1.0 is capped to 0.99 when transparent.
        assert_eq!(
            uniform(&uniforms, "color"),
            &PropertyValue::Color([1.0, 1.0, 1.0, 0.99])
        );
    }

    #[test]
    fn test_image_material_not_transparent_keeps_alpha() {
        let mut prop = ImageMaterialProperty::new();
        prop.set_color(Some(PropertyValue::Color([1.0, 1.0, 1.0, 0.5])));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "color"),
            &PropertyValue::Color([1.0, 1.0, 1.0, 0.5])
        );
    }

    #[test]
    fn test_checkerboard_defaults() {
        let prop = CheckerboardMaterialProperty::new();
        assert!(prop.is_constant());
        assert_eq!(prop.get_type(&t(0.0)), Some("Checkerboard".to_string()));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "lightColor"),
            &PropertyValue::Color(COLOR_WHITE)
        );
        assert_eq!(
            uniform(&uniforms, "darkColor"),
            &PropertyValue::Color(COLOR_BLACK)
        );
        assert_eq!(
            uniform(&uniforms, "repeat"),
            &PropertyValue::Cartesian2(DVec2::new(2.0, 2.0))
        );
    }

    #[test]
    fn test_checkerboard_custom() {
        let mut prop = CheckerboardMaterialProperty::new();
        prop.set_even_color(Some(PropertyValue::Color([1.0, 0.0, 0.0, 1.0])));
        prop.set_odd_color(Some(PropertyValue::Color([0.0, 1.0, 0.0, 1.0])));
        prop.set_repeat(Some(PropertyValue::Cartesian2(DVec2::new(4.0, 4.0))));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "lightColor"),
            &PropertyValue::Color([1.0, 0.0, 0.0, 1.0])
        );
        assert_eq!(
            uniform(&uniforms, "darkColor"),
            &PropertyValue::Color([0.0, 1.0, 0.0, 1.0])
        );
        assert_eq!(
            uniform(&uniforms, "repeat"),
            &PropertyValue::Cartesian2(DVec2::new(4.0, 4.0))
        );
    }

    #[test]
    fn test_grid_defaults() {
        let prop = GridMaterialProperty::new();
        assert!(prop.is_constant());
        assert_eq!(prop.get_type(&t(0.0)), Some("Grid".to_string()));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "color"),
            &PropertyValue::Color(COLOR_WHITE)
        );
        assert_eq!(uniform(&uniforms, "cellAlpha"), &PropertyValue::Number(0.1));
        assert_eq!(
            uniform(&uniforms, "lineCount"),
            &PropertyValue::Cartesian2(DVec2::new(8.0, 8.0))
        );
        assert_eq!(
            uniform(&uniforms, "lineThickness"),
            &PropertyValue::Cartesian2(DVec2::new(1.0, 1.0))
        );
        assert_eq!(
            uniform(&uniforms, "lineOffset"),
            &PropertyValue::Cartesian2(DVec2::new(0.0, 0.0))
        );
    }

    #[test]
    fn test_grid_custom_and_constancy() {
        let mut prop = GridMaterialProperty::new();
        prop.set_cell_alpha(Some(PropertyValue::Number(0.5)));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(uniform(&uniforms, "cellAlpha"), &PropertyValue::Number(0.5));
        assert!(prop.is_constant());

        // A dynamic sub-property makes the whole material non-constant.
        let mut sampled = SampledProperty::new(PackableType::Number);
        sampled.add_sample(t(0.0), &PropertyValue::Number(0.0), &[]);
        sampled.add_sample(t(10.0), &PropertyValue::Number(1.0), &[]);
        prop.set_cell_alpha_property(Some(Arc::new(sampled)));
        assert!(!prop.is_constant());
        let uniforms = prop.get_value(&t(5.0));
        assert_eq!(uniform(&uniforms, "cellAlpha"), &PropertyValue::Number(0.5));
    }

    #[test]
    fn test_stripe_defaults() {
        let prop = StripeMaterialProperty::new();
        assert!(prop.is_constant());
        assert_eq!(prop.get_type(&t(0.0)), Some("Stripe".to_string()));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(uniform(&uniforms, "horizontal"), &PropertyValue::Boolean(true));
        assert_eq!(
            uniform(&uniforms, "evenColor"),
            &PropertyValue::Color(COLOR_WHITE)
        );
        assert_eq!(
            uniform(&uniforms, "oddColor"),
            &PropertyValue::Color(COLOR_BLACK)
        );
        assert_eq!(uniform(&uniforms, "offset"), &PropertyValue::Number(0.0));
        assert_eq!(uniform(&uniforms, "repeat"), &PropertyValue::Number(1.0));
    }

    #[test]
    fn test_stripe_vertical() {
        let mut prop = StripeMaterialProperty::new();
        prop.set_orientation(StripeOrientation::Vertical);
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "horizontal"),
            &PropertyValue::Boolean(false)
        );
    }

    #[test]
    fn test_stripe_orientation_value_roundtrip() {
        assert_eq!(
            StripeOrientation::from_value(&StripeOrientation::Horizontal.to_value()),
            StripeOrientation::Horizontal
        );
        assert_eq!(
            StripeOrientation::from_value(&StripeOrientation::Vertical.to_value()),
            StripeOrientation::Vertical
        );
        assert_eq!(StripeOrientation::Horizontal.to_number(), 0.0);
        assert_eq!(StripeOrientation::Vertical.to_number(), 1.0);
    }

    #[test]
    fn test_polyline_arrow() {
        let prop = PolylineArrowMaterialProperty::new(None);
        assert!(prop.is_constant());
        assert_eq!(prop.get_type(&t(0.0)), Some("PolylineArrow".to_string()));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "color"),
            &PropertyValue::Color(COLOR_WHITE)
        );
    }

    #[test]
    fn test_polyline_dash_defaults() {
        let prop = PolylineDashMaterialProperty::new();
        assert!(prop.is_constant());
        assert_eq!(prop.get_type(&t(0.0)), Some("PolylineDash".to_string()));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "color"),
            &PropertyValue::Color(COLOR_WHITE)
        );
        assert_eq!(
            uniform(&uniforms, "gapColor"),
            &PropertyValue::Color(COLOR_TRANSPARENT)
        );
        assert_eq!(uniform(&uniforms, "dashLength"), &PropertyValue::Number(16.0));
        assert_eq!(
            uniform(&uniforms, "dashPattern"),
            &PropertyValue::Number(255.0)
        );
    }

    #[test]
    fn test_polyline_dash_custom() {
        let mut prop = PolylineDashMaterialProperty::new();
        prop.set_dash_length(Some(PropertyValue::Number(32.0)));
        prop.set_gap_color(Some(PropertyValue::Color([1.0, 0.0, 0.0, 0.5])));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(uniform(&uniforms, "dashLength"), &PropertyValue::Number(32.0));
        assert_eq!(
            uniform(&uniforms, "gapColor"),
            &PropertyValue::Color([1.0, 0.0, 0.0, 0.5])
        );
    }

    #[test]
    fn test_polyline_glow_defaults() {
        let prop = PolylineGlowMaterialProperty::new();
        assert!(prop.is_constant());
        assert_eq!(prop.get_type(&t(0.0)), Some("PolylineGlow".to_string()));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "color"),
            &PropertyValue::Color(COLOR_WHITE)
        );
        assert_eq!(uniform(&uniforms, "glowPower"), &PropertyValue::Number(0.25));
        assert_eq!(uniform(&uniforms, "taperPower"), &PropertyValue::Number(1.0));
    }

    #[test]
    fn test_polyline_glow_dynamic_not_constant() {
        // Corrects CesiumJS's `isConstant` bug (it checks nonexistent `_glow`):
        // a dynamic glowPower must make the property non-constant.
        let mut prop = PolylineGlowMaterialProperty::new();
        let mut sampled = SampledProperty::new(PackableType::Number);
        sampled.add_sample(t(0.0), &PropertyValue::Number(0.1), &[]);
        sampled.add_sample(t(10.0), &PropertyValue::Number(0.9), &[]);
        prop.set_glow_power_property(Some(Arc::new(sampled)));
        assert!(!prop.is_constant());
        let uniforms = prop.get_value(&t(5.0));
        assert_eq!(uniform(&uniforms, "glowPower"), &PropertyValue::Number(0.5));
    }

    #[test]
    fn test_polyline_outline_defaults() {
        let prop = PolylineOutlineMaterialProperty::new();
        assert!(prop.is_constant());
        assert_eq!(prop.get_type(&t(0.0)), Some("PolylineOutline".to_string()));
        let uniforms = prop.get_value(&t(0.0));
        assert_eq!(
            uniform(&uniforms, "color"),
            &PropertyValue::Color(COLOR_WHITE)
        );
        assert_eq!(
            uniform(&uniforms, "outlineColor"),
            &PropertyValue::Color(COLOR_BLACK)
        );
        assert_eq!(
            uniform(&uniforms, "outlineWidth"),
            &PropertyValue::Number(1.0)
        );
    }

    #[test]
    fn test_composite_material() {
        let mut prop = CompositeMaterialProperty::new();
        assert!(prop.is_constant()); // empty → constant

        let color_mat = Arc::new(ColorMaterialProperty::from_color([1.0, 0.0, 0.0, 1.0]))
            as Arc<dyn MaterialProperty>;
        let grid_mat = Arc::new(GridMaterialProperty::new()) as Arc<dyn MaterialProperty>;

        prop.add_interval(TimeInterval::new(t(0.0), t(10.0), true, false), Some(color_mat));
        prop.add_interval(TimeInterval::new(t(10.0), t(20.0), true, true), Some(grid_mat));
        assert!(!prop.is_constant());

        assert_eq!(prop.get_type(&t(5.0)), Some("Color".to_string()));
        assert_eq!(
            uniform(&prop.get_value(&t(5.0)), "color"),
            &PropertyValue::Color([1.0, 0.0, 0.0, 1.0])
        );

        assert_eq!(prop.get_type(&t(15.0)), Some("Grid".to_string()));
        assert!(prop.get_value(&t(15.0)).contains_key("cellAlpha"));

        // Outside all intervals: no type, empty uniforms.
        assert_eq!(prop.get_type(&t(30.0)), None);
        assert!(prop.get_value(&t(30.0)).is_empty());
    }

    #[test]
    fn test_composite_material_equals() {
        let mut a = CompositeMaterialProperty::new();
        let mut b = CompositeMaterialProperty::new();
        let mat_a = Arc::new(ColorMaterialProperty::from_color([1.0, 0.0, 0.0, 1.0]))
            as Arc<dyn MaterialProperty>;
        let mat_b = Arc::new(ColorMaterialProperty::from_color([1.0, 0.0, 0.0, 1.0]))
            as Arc<dyn MaterialProperty>;
        a.add_interval(TimeInterval::new(t(0.0), t(10.0), true, true), Some(mat_a));
        b.add_interval(TimeInterval::new(t(0.0), t(10.0), true, true), Some(mat_b));
        assert!(a.equals(&b));

        let mut c = CompositeMaterialProperty::new();
        let mat_c = Arc::new(ColorMaterialProperty::from_color([0.0, 0.0, 1.0, 1.0]))
            as Arc<dyn MaterialProperty>;
        c.add_interval(TimeInterval::new(t(0.0), t(10.0), true, true), Some(mat_c));
        assert!(!a.equals(&c));
    }

    #[test]
    fn test_material_type_names() {
        assert_eq!(
            ColorMaterialProperty::new(None).get_type(&t(0.0)).unwrap(),
            "Color"
        );
        assert_eq!(
            ImageMaterialProperty::new().get_type(&t(0.0)).unwrap(),
            "Image"
        );
        assert_eq!(
            CheckerboardMaterialProperty::new().get_type(&t(0.0)).unwrap(),
            "Checkerboard"
        );
        assert_eq!(GridMaterialProperty::new().get_type(&t(0.0)).unwrap(), "Grid");
        assert_eq!(
            StripeMaterialProperty::new().get_type(&t(0.0)).unwrap(),
            "Stripe"
        );
        assert_eq!(
            PolylineArrowMaterialProperty::new(None).get_type(&t(0.0)).unwrap(),
            "PolylineArrow"
        );
        assert_eq!(
            PolylineDashMaterialProperty::new().get_type(&t(0.0)).unwrap(),
            "PolylineDash"
        );
        assert_eq!(
            PolylineGlowMaterialProperty::new().get_type(&t(0.0)).unwrap(),
            "PolylineGlow"
        );
        assert_eq!(
            PolylineOutlineMaterialProperty::new().get_type(&t(0.0)).unwrap(),
            "PolylineOutline"
        );
    }
}
