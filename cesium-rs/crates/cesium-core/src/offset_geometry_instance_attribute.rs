//! Ported from `packages/engine/Source/Core/OffsetGeometryInstanceAttribute.js`.
//!
//! Per-instance attribute that applies a Cartesian offset to the geometry.

use crate::cartesian3::Cartesian3;
use crate::component_datatype::ComponentDatatype;

/// Per-instance offset attribute.
#[derive(Debug, Clone)]
pub struct OffsetGeometryInstanceAttribute {
    /// `[x, y, z]` offset values.
    pub value: Vec<f64>,
}

impl OffsetGeometryInstanceAttribute {
    /// Creates a new instance.
    pub fn new(x: Option<f64>, y: Option<f64>, z: Option<f64>) -> Self {
        Self {
            value: vec![x.unwrap_or(0.0), y.unwrap_or(0.0), z.unwrap_or(0.0)],
        }
    }

    /// Creates from a `Cartesian3` offset.
    pub fn from_cartesian3(offset: &Cartesian3) -> Self {
        Self {
            value: vec![offset.x, offset.y, offset.z],
        }
    }

    /// The component datatype (Float).
    pub fn component_datatype() -> ComponentDatatype {
        ComponentDatatype::Float
    }

    /// Number of components per attribute.
    pub fn components_per_attribute() -> usize {
        3
    }

    /// Whether to normalize.
    pub fn normalize() -> bool {
        false
    }

    /// Converts a Cartesian3 to a value array.
    pub fn to_value(offset: &Cartesian3) -> Vec<f64> {
        vec![offset.x, offset.y, offset.z]
    }
}
