//! Ported from `packages/engine/Source/Core/ColorGeometryInstanceAttribute.js`.
//!
//! Per-instance color attribute.

use crate::color::Color;
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

    /// Creates a new instance from a [`Color`].
    ///
    /// Port of `ColorGeometryInstanceAttribute.fromColor`.
    pub fn from_color(color: &Color) -> Self {
        Self::new(
            Some(color.red),
            Some(color.green),
            Some(color.blue),
            Some(color.alpha),
        )
    }

    /// Converts a color to a byte array that can be used to assign a color
    /// attribute.
    ///
    /// Port of `ColorGeometryInstanceAttribute.toValue`. Mirrors the JS
    /// `Uint8Array(color.toBytes())` semantics: byte conversion wraps
    /// out-of-range values modulo 256 (see `Color::to_bytes`, which returns
    /// the unclamped JS `floatToByte` results).
    pub fn to_value(color: &Color, result: Option<&mut [u8; 4]>) -> [u8; 4] {
        let bytes = color.to_bytes();
        let mut out = [0u8; 4];
        for i in 0..4 {
            out[i] = bytes[i].rem_euclid(256) as u8;
        }
        if let Some(r) = result {
            *r = out;
        }
        out
    }
}
