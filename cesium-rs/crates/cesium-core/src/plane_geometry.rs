//! Ported from `packages/engine/Source/Core/PlaneGeometry.js`.
//!
//! A plane centered at the origin with unit width and length.

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::component_datatype::ComponentDatatype;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::index_datatype::IndexStorage;
use crate::primitive_type::PrimitiveType;
use crate::vertex_format::VertexFormat;

/// Describes a plane centered at the origin, with unit width and length.
#[derive(Debug, Clone)]
pub struct PlaneGeometry {
    vertex_format: VertexFormat,
}

impl PlaneGeometry {
    /// Creates a new `PlaneGeometry`.
    pub fn new(vertex_format: Option<VertexFormat>) -> Self {
        Self {
            vertex_format: vertex_format.unwrap_or_default(),
        }
    }

    /// The number of `f64` elements used to pack/unpack.
    pub const PACKED_LENGTH: usize = VertexFormat::PACKED_LENGTH;

    /// Packs into `array`.
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        let si = starting_index.unwrap_or(0);
        self.vertex_format.pack(array, si);
    }

    /// Unpacks from `array`.
    pub fn unpack(array: &[f64], starting_index: Option<usize>) -> Self {
        let si = starting_index.unwrap_or(0);
        let vf = VertexFormat::unpack(array, si, None);
        Self { vertex_format: vf }
    }

    /// Computes the geometric representation of the plane.
    pub fn create_geometry(&self) -> Geometry {
        let vf = &self.vertex_format;

        let positions: Vec<f64> = vec![
            -0.5, -0.5, 0.0,
             0.5, -0.5, 0.0,
             0.5,  0.5, 0.0,
            -0.5,  0.5, 0.0,
        ];

        let indices: Vec<u16> = vec![0, 1, 2, 0, 2, 3];

        let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();
        attributes.insert(
            "position".to_string(),
            GeometryAttribute::new(ComponentDatatype::Double, 3, false, positions),
        );

        if vf.normal {
            let normals: Vec<f64> = vec![
                0.,0.,1., 0.,0.,1., 0.,0.,1., 0.,0.,1.,
            ];
            attributes.insert(
                "normal".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 3, false, normals),
            );
        }

        if vf.st {
            let tex: Vec<f64> = vec![
                0.,0., 1.,0., 1.,1., 0.,1.,
            ];
            attributes.insert(
                "st".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 2, false, tex),
            );
        }

        if vf.tangent {
            let tangents: Vec<f64> = vec![
                1.,0.,0., 1.,0.,0., 1.,0.,0., 1.,0.,0.,
            ];
            attributes.insert(
                "tangent".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 3, false, tangents),
            );
        }

        if vf.bitangent {
            let bitangents: Vec<f64> = vec![
                0.,1.,0., 0.,1.,0., 0.,1.,0., 0.,1.,0.,
            ];
            attributes.insert(
                "bitangent".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 3, false, bitangents),
            );
        }

        Geometry::new(
            attributes,
            Some(IndexStorage::U16(indices)),
            Some(PrimitiveType::Triangles),
            Some(BoundingSphere::new(Cartesian3::ZERO, 2.0f64.sqrt())),
        )
    }
}
