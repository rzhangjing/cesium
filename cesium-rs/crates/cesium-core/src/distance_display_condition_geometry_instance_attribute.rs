//! Ported from `packages/engine/Source/Core/DistanceDisplayConditionGeometryInstanceAttribute.js`.
//!
//! Per-instance attribute that controls visibility based on camera distance.

use crate::component_datatype::ComponentDatatype;

/// Per-instance distance display condition attribute.
#[derive(Debug, Clone)]
pub struct DistanceDisplayConditionGeometryInstanceAttribute {
    /// `[near, far]` distances.
    pub value: Vec<f64>,
}

impl DistanceDisplayConditionGeometryInstanceAttribute {
    /// Creates a new instance.
    pub fn new(near: Option<f64>, far: Option<f64>) -> Self {
        let n = near.unwrap_or(0.0);
        let f = far.unwrap_or(f64::MAX);
        debug_assert!(f > n, "far distance must be greater than near distance");
        Self { value: vec![n, f] }
    }

    /// The component datatype (Float).
    pub fn component_datatype() -> ComponentDatatype {
        ComponentDatatype::Float
    }

    /// Number of components per attribute.
    pub fn components_per_attribute() -> usize {
        2
    }

    /// Whether to normalize.
    pub fn normalize() -> bool {
        false
    }

    /// Converts near/far to a value array.
    pub fn to_value(near: f64, far: f64) -> Vec<f64> {
        vec![near, far]
    }
}
