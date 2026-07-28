//! GeometryInstanceAttribute family.
//! Maps to CesiumJS `Core/GeometryInstanceAttribute.js`,
//! `Core/ColorGeometryInstanceAttribute.js`,
//! `Core/ShowGeometryInstanceAttribute.js`,
//! `Core/DistanceDisplayConditionGeometryInstanceAttribute.js`

use crate::attribute_compression::ComponentDatatype;
use crate::color::Color;

/// Values and type information for per-instance geometry attributes.
/// Maps to CesiumJS `GeometryInstanceAttribute`
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryInstanceAttribute {
    /// The datatype of each component in the attribute.
    pub component_datatype: ComponentDatatype,
    /// A number between 1 and 4 that defines the number of components in an attribute.
    pub components_per_attribute: u32,
    /// When true and componentDatatype is an integer format, indicate that the components
    /// should be mapped to the range [0, 1] (unsigned) or [-1, 1] (signed).
    pub normalize: bool,
    /// The value for the attribute.
    pub value: Vec<f64>,
}

impl GeometryInstanceAttribute {
    /// Creates a new GeometryInstanceAttribute.
    ///
    /// # Panics
    /// Panics if `components_per_attribute` is not between 1 and 4.
    pub fn new(
        component_datatype: ComponentDatatype,
        components_per_attribute: u32,
        normalize: bool,
        value: Vec<f64>,
    ) -> Self {
        assert!(
            (1..=4).contains(&components_per_attribute),
            "components_per_attribute must be between 1 and 4."
        );
        Self {
            component_datatype,
            components_per_attribute,
            normalize,
            value,
        }
    }
}

/// Value and type information for per-instance geometry color.
/// Maps to CesiumJS `ColorGeometryInstanceAttribute`
#[derive(Debug, Clone, PartialEq)]
pub struct ColorGeometryInstanceAttribute {
    /// The values for the attributes stored as [R, G, B, A] bytes.
    pub value: [u8; 4],
}

impl ColorGeometryInstanceAttribute {
    /// Creates a new ColorGeometryInstanceAttribute from floating point RGBA components.
    pub fn new(red: f64, green: f64, blue: f64, alpha: f64) -> Self {
        Self {
            value: [
                Color::float_to_byte(red),
                Color::float_to_byte(green),
                Color::float_to_byte(blue),
                Color::float_to_byte(alpha),
            ],
        }
    }

    /// The datatype of each component: UNSIGNED_BYTE.
    pub fn component_datatype(&self) -> ComponentDatatype {
        ComponentDatatype::UnsignedByte
    }

    /// The number of components: 4.
    pub fn components_per_attribute(&self) -> u32 {
        4
    }

    /// Normalize: true.
    pub fn normalize(&self) -> bool {
        true
    }

    /// Creates a new ColorGeometryInstanceAttribute from a Color.
    /// Maps to CesiumJS `ColorGeometryInstanceAttribute.fromColor`
    pub fn from_color(color: &Color) -> Self {
        Self {
            value: color.to_bytes(),
        }
    }

    /// Converts a color to a byte array that can be used to assign a color attribute.
    /// Maps to CesiumJS `ColorGeometryInstanceAttribute.toValue`
    pub fn to_value(color: &Color) -> [u8; 4] {
        color.to_bytes()
    }

    /// Compares two ColorGeometryInstanceAttributes for equality.
    /// Maps to CesiumJS `ColorGeometryInstanceAttribute.equals`
    pub fn equals(
        left: Option<&ColorGeometryInstanceAttribute>,
        right: Option<&ColorGeometryInstanceAttribute>,
    ) -> bool {
        match (left, right) {
            (Some(l), Some(r)) => l.value == r.value,
            _ => false,
        }
    }
}

/// Value and type information for per-instance geometry attribute that determines
/// if the geometry instance will be shown.
/// Maps to CesiumJS `ShowGeometryInstanceAttribute`
#[derive(Debug, Clone, PartialEq)]
pub struct ShowGeometryInstanceAttribute {
    /// The values for the attributes stored as [show] byte.
    pub value: [u8; 1],
}

impl ShowGeometryInstanceAttribute {
    /// Creates a new ShowGeometryInstanceAttribute.
    pub fn new(show: bool) -> Self {
        Self {
            value: Self::to_value(show),
        }
    }

    /// The datatype of each component: UNSIGNED_BYTE.
    pub fn component_datatype(&self) -> ComponentDatatype {
        ComponentDatatype::UnsignedByte
    }

    /// The number of components: 1.
    pub fn components_per_attribute(&self) -> u32 {
        1
    }

    /// Normalize: false.
    pub fn normalize(&self) -> bool {
        false
    }

    /// Converts a boolean show to a typed array.
    /// Maps to CesiumJS `ShowGeometryInstanceAttribute.toValue`
    pub fn to_value(show: bool) -> [u8; 1] {
        [show as u8]
    }
}

/// Value and type information for per-instance geometry attribute that determines
/// if the geometry instance has a distance display condition.
/// Maps to CesiumJS `DistanceDisplayConditionGeometryInstanceAttribute`
#[derive(Debug, Clone, PartialEq)]
pub struct DistanceDisplayConditionGeometryInstanceAttribute {
    /// The values for the attributes stored as [near, far] floats.
    pub value: [f32; 2],
}

impl DistanceDisplayConditionGeometryInstanceAttribute {
    /// Creates a new DistanceDisplayConditionGeometryInstanceAttribute.
    ///
    /// # Panics
    /// Panics if far <= near.
    pub fn new(near: f32, far: f32) -> Self {
        assert!(
            far > near,
            "far distance must be greater than near distance."
        );
        Self {
            value: [near, far],
        }
    }

    /// Creates with default values: near=0.0, far=f32::MAX.
    pub fn default_value() -> Self {
        Self {
            value: [0.0, f32::MAX],
        }
    }

    /// The datatype of each component: FLOAT.
    pub fn component_datatype(&self) -> ComponentDatatype {
        ComponentDatatype::Float
    }

    /// The number of components: 2.
    pub fn components_per_attribute(&self) -> u32 {
        2
    }

    /// Normalize: false.
    pub fn normalize(&self) -> bool {
        false
    }

    /// Creates from a DistanceDisplayCondition (near, far pair).
    /// Maps to CesiumJS `DistanceDisplayConditionGeometryInstanceAttribute.fromDistanceDisplayCondition`
    pub fn from_distance_display_condition(near: f32, far: f32) -> Self {
        assert!(
            far > near,
            "distanceDisplayCondition.far distance must be greater than distanceDisplayCondition.near distance."
        );
        Self {
            value: [near, far],
        }
    }

    /// Converts a distance display condition to a float array.
    /// Maps to CesiumJS `DistanceDisplayConditionGeometryInstanceAttribute.toValue`
    pub fn to_value(near: f32, far: f32) -> [f32; 2] {
        [near, far]
    }
}
