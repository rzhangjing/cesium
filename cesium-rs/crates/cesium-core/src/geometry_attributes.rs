//! Ported from `packages/engine/Source/Core/GeometryAttributes.js`.

use crate::geometry_attribute::GeometryAttribute;

/// Attributes which make up a geometry's vertices.
#[derive(Debug, Clone, Default)]
pub struct GeometryAttributes {
    /// The 3D position attribute (64-bit, 3 components).
    pub position: Option<GeometryAttribute>,
    /// The normal attribute (32-bit, 3 components).
    pub normal: Option<GeometryAttribute>,
    /// The 2D texture coordinate attribute (32-bit, 2 components).
    pub st: Option<GeometryAttribute>,
    /// The bitangent attribute (32-bit, 3 components).
    pub bitangent: Option<GeometryAttribute>,
    /// The tangent attribute (32-bit, 3 components).
    pub tangent: Option<GeometryAttribute>,
    /// The color attribute (8-bit unsigned, 4 components).
    pub color: Option<GeometryAttribute>,
}
