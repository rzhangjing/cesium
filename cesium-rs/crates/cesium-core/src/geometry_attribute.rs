//! Ported from `packages/engine/Source/Core/GeometryAttribute.js`.
//!
//! Values and type information for geometry attributes.

use crate::component_datatype::ComponentDatatype;

/// Values and type information for a geometry attribute. A [`Geometry`](crate::Geometry)
/// generally contains one or more attributes. All attributes together form the
/// geometry's vertices.
#[derive(Debug, Clone)]
pub struct GeometryAttribute {
    /// The datatype of each component in the attribute.
    pub component_datatype: ComponentDatatype,
    /// A number between 1 and 4 that defines the number of components in an attribute.
    pub components_per_attribute: u32,
    /// When `true` and `component_datatype` is an integer format, the components
    /// should be mapped to [0, 1] (unsigned) or [-1, 1] (signed) when accessed.
    pub normalize: bool,
    /// The values for the attribute stored as `f64` components.
    ///
    /// DEVIATION: JS uses typed arrays (`Float32Array`, `Float64Array`, etc.).
    /// We use `Vec<f64>` uniformly at the domain layer; conversion to the
    /// appropriate GPU format happens at the adapter boundary.
    pub values: Vec<f64>,
}

impl GeometryAttribute {
    /// Creates a new `GeometryAttribute`.
    ///
    /// # Panics (debug)
    /// Panics if `components_per_attribute` is not in `1..=4`.
    pub fn new(
        component_datatype: ComponentDatatype,
        components_per_attribute: u32,
        normalize: bool,
        values: Vec<f64>,
    ) -> Self {
        debug_assert!(
            (1..=4).contains(&components_per_attribute),
            "components_per_attribute must be between 1 and 4"
        );
        Self {
            component_datatype,
            components_per_attribute,
            normalize,
            values,
        }
    }
}
