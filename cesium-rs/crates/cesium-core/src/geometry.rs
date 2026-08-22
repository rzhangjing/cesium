//! Ported from `packages/engine/Source/Core/Geometry.js`.
//!
//! A geometry representation with attributes forming vertices and optional index
//! data defining primitives.

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_type::GeometryType;
use crate::index_datatype::IndexStorage;
use crate::primitive_type::PrimitiveType;

/// A geometry representation with attributes forming vertices and optional index
/// data defining primitives. Geometries and an `Appearance`, which describes the
/// shading, can be assigned to a `Primitive` for visualization.
#[derive(Debug, Clone)]
pub struct Geometry {
    /// Attributes which make up the geometry's vertices. Each property
    /// corresponds to a [`GeometryAttribute`] containing the attribute's data.
    pub attributes: HashMap<String, GeometryAttribute>,
    /// Optional index data that — along with `primitive_type` — determines the
    /// primitives in the geometry.
    pub indices: Option<IndexStorage>,
    /// The type of primitives in the geometry.
    pub primitive_type: PrimitiveType,
    /// An optional bounding sphere that fully encloses the geometry.
    pub bounding_sphere: Option<BoundingSphere>,
    /// Internal geometry type identifier (private).
    pub geometry_type: GeometryType,
    /// Bounding sphere in Columbus View (private).
    pub bounding_sphere_cv: Option<BoundingSphere>,
    /// Used for computing the bounding sphere for geometry using the applyOffset
    /// vertex attribute (private).
    pub offset_attribute: Option<String>,
}

impl Geometry {
    /// Creates a new `Geometry` from the given options.
    pub fn new(
        attributes: HashMap<String, GeometryAttribute>,
        indices: Option<IndexStorage>,
        primitive_type: Option<PrimitiveType>,
        bounding_sphere: Option<BoundingSphere>,
    ) -> Self {
        debug_assert!(
            !attributes.is_empty(),
            "options.attributes is required and must not be empty"
        );
        Self {
            attributes,
            indices,
            primitive_type: primitive_type.unwrap_or(PrimitiveType::Triangles),
            bounding_sphere,
            geometry_type: GeometryType::None,
            bounding_sphere_cv: None,
            offset_attribute: None,
        }
    }

    /// Creates a new `Geometry` with all options.
    pub fn with_all(
        attributes: HashMap<String, GeometryAttribute>,
        indices: Option<IndexStorage>,
        primitive_type: Option<PrimitiveType>,
        bounding_sphere: Option<BoundingSphere>,
        geometry_type: GeometryType,
        bounding_sphere_cv: Option<BoundingSphere>,
        offset_attribute: Option<String>,
    ) -> Self {
        Self {
            attributes,
            indices,
            primitive_type: primitive_type.unwrap_or(PrimitiveType::Triangles),
            bounding_sphere,
            geometry_type,
            bounding_sphere_cv,
            offset_attribute,
        }
    }

    /// Computes the number of vertices in a geometry. The runtime is linear with
    /// respect to the number of attributes in a vertex, not the number of vertices.
    ///
    /// Returns `None` if there are no valid attributes.
    ///
    /// # Panics (debug)
    /// Panics if attribute lists have inconsistent vertex counts.
    pub fn compute_number_of_vertices(&self) -> Option<usize> {
        let mut number_of_vertices: Option<usize> = None;

        for (name, attr) in &self.attributes {
            if attr.values.is_empty() {
                continue;
            }
            let num = attr.values.len() / attr.components_per_attribute as usize;
            if let Some(prev) = number_of_vertices {
                debug_assert!(
                    prev == num,
                    "All attribute lists must have the same number of attributes (mismatch on '{name}')"
                );
            }
            number_of_vertices = Some(num);
        }

        number_of_vertices
    }
}
