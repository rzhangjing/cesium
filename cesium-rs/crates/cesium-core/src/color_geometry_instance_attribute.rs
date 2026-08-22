//! Ported from `packages/engine/Source/Core/ColorGeometryInstanceAttribute.js`.
//!
//! Per-instance color attribute.
//!
//! NOTE: `from_color` and `to_value` require the `Color` type which has not
//! yet been ported.  The raw constructor and accessors are available now.

use crate::component_datatype::ComponentDatatype;

/// Per-instance color attribute (RGBA, stored as f64 components in [0,1]).
#[derive(Debug, Clone)]
pub struct ColorGeometryInstanceAttribute {
    /// `[red, green, blue, alpha]` in 0.0–1.0 range.
    pub value: Vec<f64>,
}

impl ColorGeometryInstanceAttribute {
    /// Creates a new instance from RGBA components (0.0–1.0).
    pub fn new(red: Option<f64>, green: Option<f64>, blue: Option<f64>, alpha: Option<f64>) -> Self {
        Self {
            value: vec![
                red.unwrap_or(1.0),
                green.unwrap_or(1.0),
                blue.unwrap_or(1.0),
                alpha.unwrap_or(1.0),
            ],
        }
    }

    /// The component datatype (UnsignedByte).
    pub fn component_datatype() -> ComponentDatatype {
        ComponentDatatype::UnsignedByte
    }

    /// Number of components per attribute.
    pub fn components_per_attribute() -> usize {
        4
    }

    /// Whether to normalize.
    pub fn normalize() -> bool {
        true
    }

    /// Compares two attributes for equality.
    pub fn equals(left: &Self, right: &Self) -> bool {
        left.value == right.value
    }

    // TODO: from_color(color: &Color) — requires Color port
    // TODO: to_value(color: &Color) -> Vec<u8> — requires Color port
}
