//! Ported from `packages/engine/Source/Core/PlaneOutlineGeometry.js`.
//!
//! Outline of a plane centered at the origin with unit width and length.

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::component_datatype::ComponentDatatype;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::index_datatype::IndexStorage;
use crate::primitive_type::PrimitiveType;

/// Describes the outline of a plane centered at the origin, with unit width and length.
#[derive(Debug, Clone, Default)]
pub struct PlaneOutlineGeometry;

impl PlaneOutlineGeometry {
    /// The number of `f64` elements used to pack/unpack.
    pub const PACKED_LENGTH: usize = 0;

    /// Computes the geometric representation of the plane outline.
    pub fn create_geometry() -> Geometry {
        let positions: Vec<f64> = vec![
            -0.5, -0.5, 0.0,
             0.5, -0.5, 0.0,
             0.5,  0.5, 0.0,
            -0.5,  0.5, 0.0,
        ];

        let indices: Vec<u16> = vec![0, 1, 1, 2, 2, 3, 3, 0];

        let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();
        attributes.insert(
            "position".to_string(),
            GeometryAttribute::new(ComponentDatatype::Double, 3, false, positions),
        );

        Geometry::new(
            attributes,
            Some(IndexStorage::U16(indices)),
            Some(PrimitiveType::Lines),
            Some(BoundingSphere::new(Cartesian3::ZERO, 2.0f64.sqrt())),
        )
    }
}
