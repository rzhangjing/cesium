//! Ported from `packages/engine/Source/Core/GeometryInstanceAttribute.js`.
//!
//! Values and type information for per-instance geometry attributes.

use crate::component_datatype::ComponentDatatype;

/// Per-instance geometry attribute value + metadata.
#[derive(Debug, Clone)]
pub struct GeometryInstanceAttribute {
    component_datatype: ComponentDatatype,
    components_per_attribute: usize,
    normalize: bool,
    value: Vec<f64>,
}

impl GeometryInstanceAttribute {
    /// Creates a new `GeometryInstanceAttribute`.
    pub fn new(
        component_datatype: ComponentDatatype,
        components_per_attribute: usize,
        normalize: Option<bool>,
        value: Vec<f64>,
    ) -> Self {
        debug_assert!(
            (1..=4).contains(&components_per_attribute),
            "componentsPerAttribute must be between 1 and 4"
        );
        Self {
            component_datatype,
            components_per_attribute,
            normalize: normalize.unwrap_or(false),
            value,
        }
    }

    /// The datatype of each component.
    pub fn component_datatype(&self) -> ComponentDatatype {
        self.component_datatype
    }

    /// The number of components per attribute (1–4).
    pub fn components_per_attribute(&self) -> usize {
        self.components_per_attribute
    }

    /// Whether integer values should be normalized to [0,1] / [-1,1].
    pub fn normalize(&self) -> bool {
        self.normalize
    }

    /// The raw attribute values.
    pub fn value(&self) -> &[f64] {
        &self.value
    }
}
