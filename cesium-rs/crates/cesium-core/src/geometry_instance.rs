//! Ported from `packages/engine/Source/Core/GeometryInstance.js`.
//!
//! Geometry instancing allows one Geometry object to be placed at several
//! different locations and colored uniquely.

use std::collections::HashMap;

use crate::geometry_instance_attribute::GeometryInstanceAttribute;
use crate::matrix4::Matrix4;

/// A single instance of a geometry, with its own model matrix and attributes.
#[derive(Debug, Clone)]
pub struct GeometryInstance {
    /// Opaque tag for the underlying geometry type (packed/unpacked separately).
    /// In JS this is `Geometry | GeometryFactory`; we use an enum wrapper.
    pub geometry: GeometryInstanceGeometry,

    /// 4×4 transform from model to world coordinates.
    pub model_matrix: Matrix4,

    /// User-defined id for picking.
    pub id: Option<String>,

    /// Per-instance attributes (color, show, …).
    pub attributes: HashMap<String, GeometryInstanceAttribute>,
}

/// Enum wrapping the two possible geometry sources for an instance.
#[derive(Debug, Clone)]
pub enum GeometryInstanceGeometry {
    /// A pre-built geometry.
    // TODO: replace with actual Geometry reference when pipeline is ready
    Placeholder,
}

impl GeometryInstance {
    /// Creates a new `GeometryInstance`.
    pub fn new(
        geometry: GeometryInstanceGeometry,
        model_matrix: Option<Matrix4>,
        id: Option<String>,
        attributes: Option<HashMap<String, GeometryInstanceAttribute>>,
    ) -> Self {
        Self {
            geometry,
            model_matrix: model_matrix.unwrap_or(Matrix4::IDENTITY.clone()),
            id,
            attributes: attributes.unwrap_or_default(),
        }
    }
}
