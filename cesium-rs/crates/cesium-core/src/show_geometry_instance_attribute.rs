//! Ported from `packages/engine/Source/Core/ShowGeometryInstanceAttribute.js`.
//!
//! Per-instance attribute that determines whether the geometry instance is shown.

use crate::component_datatype::ComponentDatatype;

/// Per-instance show attribute.
#[derive(Debug, Clone)]
pub struct ShowGeometryInstanceAttribute {
    /// Raw value: `[1.0]` for shown, `[0.0]` for hidden.
    pub value: Vec<f64>,
}

impl ShowGeometryInstanceAttribute {
    /// Creates a new instance. `show` defaults to `true`.
    pub fn new(show: Option<bool>) -> Self {
        let show = show.unwrap_or(true);
        Self {
            value: vec![if show { 1.0 } else { 0.0 }],
        }
    }

    /// The component datatype (UnsignedByte).
    pub fn component_datatype() -> ComponentDatatype {
        ComponentDatatype::UnsignedByte
    }

    /// Number of components per attribute.
    pub fn components_per_attribute() -> usize {
        1
    }

    /// Whether to normalize.
    pub fn normalize() -> bool {
        false
    }

    /// Converts a boolean to a value array.
    pub fn to_value(show: bool) -> Vec<f64> {
        vec![if show { 1.0 } else { 0.0 }]
    }
}
