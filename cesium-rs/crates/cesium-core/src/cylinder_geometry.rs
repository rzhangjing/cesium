//! Ported from `packages/engine/Source/Core/CylinderGeometry.js`.
//!
//! A cylinder centered at the origin.

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::component_datatype::ComponentDatatype;
use crate::cylinder_geometry_library;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_offset_attribute::GeometryOffsetAttribute;
use crate::index_datatype::IndexStorage;
use crate::math::CesiumMath;
use crate::primitive_type::PrimitiveType;
use crate::vertex_format::VertexFormat;

/// A description of a cylinder centered at the origin.
#[derive(Debug, Clone)]
pub struct CylinderGeometry {
    length: f64,
    top_radius: f64,
    bottom_radius: f64,
    slices: usize,
    vertex_format: VertexFormat,
    offset_attribute: Option<GeometryOffsetAttribute>,
}

impl CylinderGeometry {
    /// Creates a new `CylinderGeometry`.
    pub fn new(
        length: f64,
        top_radius: f64,
        bottom_radius: f64,
        slices: Option<usize>,
        vertex_format: Option<VertexFormat>,
        offset_attribute: Option<GeometryOffsetAttribute>,
    ) -> Self {
        Self {
            length,
            top_radius,
            bottom_radius,
            slices: slices.unwrap_or(128),
            vertex_format: vertex_format.unwrap_or_default(),
            offset_attribute,
        }
    }

    /// The number of `f64` elements used to pack/unpack.
    pub const PACKED_LENGTH: usize = VertexFormat::PACKED_LENGTH + 5;

    /// Packs into `array`.
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        let mut si = starting_index.unwrap_or(0);
        self.vertex_format.pack(array, si);
        si += VertexFormat::PACKED_LENGTH;
        array[si] = self.length; si += 1;
        array[si] = self.top_radius; si += 1;
        array[si] = self.bottom_radius; si += 1;
        array[si] = self.slices as f64; si += 1;
        array[si] = self.offset_attribute.map_or(-1.0, |o| o as u32 as f64);
    }

    /// Unpacks from `array`.
    pub fn unpack(array: &[f64], starting_index: Option<usize>) -> Self {
        let mut si = starting_index.unwrap_or(0);
        let vf = VertexFormat::unpack(array, si, None);
        si += VertexFormat::PACKED_LENGTH;
        let length = array[si]; si += 1;
        let top_radius = array[si]; si += 1;
        let bottom_radius = array[si]; si += 1;
        let slices = array[si] as usize; si += 1;
        let offset_raw = array[si];
        let offset = if offset_raw == -1.0 {
            None
        } else {
            GeometryOffsetAttribute::try_from_u32(offset_raw as u32)
        };
        Self {
            length,
            top_radius,
            bottom_radius,
            slices,
            vertex_format: vf,
            offset_attribute: offset,
        }
    }

    /// Computes the geometric representation of the cylinder.
    pub fn create_geometry(&self) -> Option<Geometry> {
        let length = self.length;
        let top_radius = self.top_radius;
        let bottom_radius = self.bottom_radius;
        let vf = &self.vertex_format;
        let slices = self.slices;

        if length <= 0.0 || top_radius < 0.0 || bottom_radius < 0.0 || (top_radius == 0.0 && bottom_radius == 0.0) {
            return None;
        }

        let two_slices = slices + slices;
        let three_slices = slices + two_slices;
        let num_vertices = two_slices + two_slices;

        let positions = cylinder_geometry_library::compute_positions(length, top_radius, bottom_radius, slices, true);

        let mut normals_data: Option<Vec<f64>> = if vf.normal { Some(vec![0.0; num_vertices * 3]) } else { None };
        let mut tangents_data: Option<Vec<f64>> = if vf.tangent { Some(vec![0.0; num_vertices * 3]) } else { None };
        let mut bitangents_data: Option<Vec<f64>> = if vf.bitangent { Some(vec![0.0; num_vertices * 3]) } else { None };
        let mut st_data: Option<Vec<f64>> = if vf.st { Some(vec![0.0; num_vertices * 2]) } else { None };

        let compute_normal = vf.normal || vf.tangent || vf.bitangent;

        if compute_normal {
            let compute_tangent = vf.tangent || vf.bitangent;

            let mut normal_index = 0;
            let mut tangent_index = 0;
            let mut bitangent_index = 0;

            let theta = (bottom_radius - top_radius).atan2(length);
            let mut normal = Cartesian3::new(0.0, 0.0, theta.sin());
            let normal_scale = theta.cos();

            // Side normals
            for i in 0..slices {
                let angle = (i as f64 / slices as f64) * CesiumMath::TWO_PI;
                let x = normal_scale * angle.cos();
                let y = normal_scale * angle.sin();
                normal.x = x;
                normal.y = y;

                let tangent = if compute_tangent {
                    let t = Cartesian3::cross_new(&Cartesian3::UNIT_Z, &normal);
                    Cartesian3::normalize_new(&t)
                } else {
                    Cartesian3::ZERO
                };

                if vf.normal {
                    normals_data.as_mut().unwrap()[normal_index] = normal.x;
                    normals_data.as_mut().unwrap()[normal_index + 1] = normal.y;
                    normals_data.as_mut().unwrap()[normal_index + 2] = normal.z;
                    normals_data.as_mut().unwrap()[normal_index + 3] = normal.x;
                    normals_data.as_mut().unwrap()[normal_index + 4] = normal.y;
                    normals_data.as_mut().unwrap()[normal_index + 5] = normal.z;
                    normal_index += 6;
                }

                if vf.tangent {
                    tangents_data.as_mut().unwrap()[tangent_index] = tangent.x;
                    tangents_data.as_mut().unwrap()[tangent_index + 1] = tangent.y;
                    tangents_data.as_mut().unwrap()[tangent_index + 2] = tangent.z;
                    tangents_data.as_mut().unwrap()[tangent_index + 3] = tangent.x;
                    tangents_data.as_mut().unwrap()[tangent_index + 4] = tangent.y;
                    tangents_data.as_mut().unwrap()[tangent_index + 5] = tangent.z;
                    tangent_index += 6;
                }

                if vf.bitangent {
                    let bitangent = Cartesian3::cross_new(&normal, &tangent);
                    let bitangent = Cartesian3::normalize_new(&bitangent);
                    bitangents_data.as_mut().unwrap()[bitangent_index] = bitangent.x;
                    bitangents_data.as_mut().unwrap()[bitangent_index + 1] = bitangent.y;
                    bitangents_data.as_mut().unwrap()[bitangent_index + 2] = bitangent.z;
                    bitangents_data.as_mut().unwrap()[bitangent_index + 3] = bitangent.x;
                    bitangents_data.as_mut().unwrap()[bitangent_index + 4] = bitangent.y;
                    bitangents_data.as_mut().unwrap()[bitangent_index + 5] = bitangent.z;
                    bitangent_index += 6;
                }
            }

            // Bottom cap normals
            for _ in 0..slices {
                if vf.normal { normals_data.as_mut().unwrap()[normal_index] = 0.0; normals_data.as_mut().unwrap()[normal_index+1] = 0.0; normals_data.as_mut().unwrap()[normal_index+2] = -1.0; normal_index += 3; }
                if vf.tangent { tangents_data.as_mut().unwrap()[tangent_index] = 1.0; tangents_data.as_mut().unwrap()[tangent_index+1] = 0.0; tangents_data.as_mut().unwrap()[tangent_index+2] = 0.0; tangent_index += 3; }
                if vf.bitangent { bitangents_data.as_mut().unwrap()[bitangent_index] = 0.0; bitangents_data.as_mut().unwrap()[bitangent_index+1] = -1.0; bitangents_data.as_mut().unwrap()[bitangent_index+2] = 0.0; bitangent_index += 3; }
            }

            // Top cap normals
            for _ in 0..slices {
                if vf.normal { normals_data.as_mut().unwrap()[normal_index] = 0.0; normals_data.as_mut().unwrap()[normal_index+1] = 0.0; normals_data.as_mut().unwrap()[normal_index+2] = 1.0; normal_index += 3; }
                if vf.tangent { tangents_data.as_mut().unwrap()[tangent_index] = 1.0; tangents_data.as_mut().unwrap()[tangent_index+1] = 0.0; tangents_data.as_mut().unwrap()[tangent_index+2] = 0.0; tangent_index += 3; }
                if vf.bitangent { bitangents_data.as_mut().unwrap()[bitangent_index] = 0.0; bitangents_data.as_mut().unwrap()[bitangent_index+1] = 1.0; bitangents_data.as_mut().unwrap()[bitangent_index+2] = 0.0; bitangent_index += 3; }
            }
        }

        // Indices
        let num_indices = 12 * slices - 12;
        let mut indices: Vec<u16> = Vec::with_capacity(num_indices);
        let mut _index = 0;
        let mut j = 0;

        // Side
        for _ in 0..slices - 1 {
            indices.push(j as u16);
            indices.push((j + 2) as u16);
            indices.push((j + 3) as u16);
            indices.push(j as u16);
            indices.push((j + 3) as u16);
            indices.push((j + 1) as u16);
            _index += 6;
            j += 2;
        }

        indices.push((two_slices - 2) as u16);
        indices.push(0);
        indices.push(1);
        indices.push((two_slices - 2) as u16);
        indices.push(1);
        indices.push((two_slices - 1) as u16);
        _index += 6;

        // Bottom cap
        for i in 1..slices - 1 {
            indices.push((two_slices + i + 1) as u16);
            indices.push((two_slices + i) as u16);
            indices.push(two_slices as u16);
            _index += 3;
        }

        // Top cap
        for i in 1..slices - 1 {
            indices.push(three_slices as u16);
            indices.push((three_slices + i) as u16);
            indices.push((three_slices + i + 1) as u16);
            _index += 3;
        }

        // Texture coordinates
        if vf.st {
            let rad = top_radius.max(bottom_radius);
            let mut st_index = 0;
            for i in 0..num_vertices {
                let px = positions[i * 3];
                let py = positions[i * 3 + 1];
                st_data.as_mut().unwrap()[st_index] = (px + rad) / (2.0 * rad);
                st_data.as_mut().unwrap()[st_index + 1] = (py + rad) / (2.0 * rad);
                st_index += 2;
            }
        }

        let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();
        if vf.position {
            attributes.insert("position".to_string(), GeometryAttribute::new(ComponentDatatype::Double, 3, false, positions));
        }
        if vf.normal { attributes.insert("normal".to_string(), GeometryAttribute::new(ComponentDatatype::Float, 3, false, normals_data.unwrap())); }
        if vf.tangent { attributes.insert("tangent".to_string(), GeometryAttribute::new(ComponentDatatype::Float, 3, false, tangents_data.unwrap())); }
        if vf.bitangent { attributes.insert("bitangent".to_string(), GeometryAttribute::new(ComponentDatatype::Float, 3, false, bitangents_data.unwrap())); }
        if vf.st { attributes.insert("st".to_string(), GeometryAttribute::new(ComponentDatatype::Float, 2, false, st_data.unwrap())); }

        let radius = Cartesian2::new(length * 0.5, bottom_radius.max(top_radius));
        let bs = BoundingSphere::new(Cartesian3::ZERO, Cartesian2::magnitude(&radius));

        if let Some(offset) = self.offset_attribute {
            let num_verts = attributes["position"].values.len() / 3;
            let offset_value = if offset == GeometryOffsetAttribute::None { 0.0 } else { 1.0 };
            attributes.insert("applyOffset".to_string(), GeometryAttribute::new(ComponentDatatype::UnsignedByte, 1, false, vec![offset_value; num_verts]));
        }

        Some(Geometry::new(
            attributes,
            Some(IndexStorage::U16(indices)),
            Some(PrimitiveType::Triangles),
            Some(bs),
        ))
    }

    /// Returns the geometry for a unit cylinder.
    pub fn get_unit_cylinder() -> Geometry {
        let geom = Self::new(1.0, 1.0, 1.0, None, Some(VertexFormat::position_only()), None);
        geom.create_geometry().unwrap()
    }
}
