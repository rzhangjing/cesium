//! Ported from `packages/engine/Source/Core/EllipsoidGeometry.js`.
//!
//! An ellipsoid centered at the origin.

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::component_datatype::ComponentDatatype;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_offset_attribute::GeometryOffsetAttribute;
use crate::index_datatype::IndexStorage;
use crate::math::CesiumMath;
use crate::primitive_type::PrimitiveType;
use crate::vertex_format::VertexFormat;

/// A description of an ellipsoid centered at the origin.
#[derive(Debug, Clone)]
pub struct EllipsoidGeometry {
    radii: Cartesian3,
    inner_radii: Cartesian3,
    minimum_clock: f64,
    maximum_clock: f64,
    minimum_cone: f64,
    maximum_cone: f64,
    stack_partitions: i32,
    slice_partitions: i32,
    vertex_format: VertexFormat,
    offset_attribute: Option<GeometryOffsetAttribute>,
}

impl Default for EllipsoidGeometry {
    fn default() -> Self {
        Self {
            radii: Cartesian3::new(1.0, 1.0, 1.0),
            inner_radii: Cartesian3::new(1.0, 1.0, 1.0),
            minimum_clock: 0.0,
            maximum_clock: CesiumMath::TWO_PI,
            minimum_cone: 0.0,
            maximum_cone: CesiumMath::PI,
            stack_partitions: 64,
            slice_partitions: 64,
            vertex_format: VertexFormat::default(),
            offset_attribute: None,
        }
    }
}

impl EllipsoidGeometry {
    /// Creates a new `EllipsoidGeometry`.
    pub fn new(
        radii: Option<Cartesian3>,
        inner_radii: Option<Cartesian3>,
        minimum_clock: Option<f64>,
        maximum_clock: Option<f64>,
        minimum_cone: Option<f64>,
        maximum_cone: Option<f64>,
        stack_partitions: Option<i32>,
        slice_partitions: Option<i32>,
        vertex_format: Option<VertexFormat>,
        offset_attribute: Option<GeometryOffsetAttribute>,
    ) -> Self {
        let radii = radii.unwrap_or(Cartesian3::new(1.0, 1.0, 1.0));
        Self {
            radii,
            inner_radii: inner_radii.unwrap_or(radii),
            minimum_clock: minimum_clock.unwrap_or(0.0),
            maximum_clock: maximum_clock.unwrap_or(CesiumMath::TWO_PI),
            minimum_cone: minimum_cone.unwrap_or(0.0),
            maximum_cone: maximum_cone.unwrap_or(CesiumMath::PI),
            stack_partitions: stack_partitions.unwrap_or(64),
            slice_partitions: slice_partitions.unwrap_or(64),
            vertex_format: vertex_format.unwrap_or_default(),
            offset_attribute,
        }
    }

    /// The number of `f64` elements used to pack/unpack.
    pub const PACKED_LENGTH: usize = 2 * Cartesian3::PACKED_LENGTH + VertexFormat::PACKED_LENGTH + 7;

    /// Packs into `array`.
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        let mut si = starting_index.unwrap_or(0);
        Cartesian3::pack(&self.radii, array, Some(si));
        si += Cartesian3::PACKED_LENGTH;
        Cartesian3::pack(&self.inner_radii, array, Some(si));
        si += Cartesian3::PACKED_LENGTH;
        self.vertex_format.pack(array, si);
        si += VertexFormat::PACKED_LENGTH;
        array[si] = self.minimum_clock; si += 1;
        array[si] = self.maximum_clock; si += 1;
        array[si] = self.minimum_cone; si += 1;
        array[si] = self.maximum_cone; si += 1;
        array[si] = self.stack_partitions as f64; si += 1;
        array[si] = self.slice_partitions as f64; si += 1;
        array[si] = self.offset_attribute.map_or(-1.0, |o| o as u32 as f64);
    }

    /// Unpacks from `array`.
    pub fn unpack(array: &[f64], starting_index: Option<usize>) -> Self {
        let mut si = starting_index.unwrap_or(0);
        let radii = Cartesian3::unpack_new(array, Some(si));
        si += Cartesian3::PACKED_LENGTH;
        let inner_radii = Cartesian3::unpack_new(array, Some(si));
        si += Cartesian3::PACKED_LENGTH;
        let vertex_format = VertexFormat::unpack(array, si, None);
        si += VertexFormat::PACKED_LENGTH;
        let minimum_clock = array[si]; si += 1;
        let maximum_clock = array[si]; si += 1;
        let minimum_cone = array[si]; si += 1;
        let maximum_cone = array[si]; si += 1;
        let stack_partitions = array[si] as i32; si += 1;
        let slice_partitions = array[si] as i32; si += 1;
        let offset_raw = array[si];
        let offset_attribute = if offset_raw == -1.0 {
            None
        } else {
            GeometryOffsetAttribute::try_from_u32(offset_raw as u32)
        };

        Self {
            radii,
            inner_radii,
            minimum_clock,
            maximum_clock,
            minimum_cone,
            maximum_cone,
            stack_partitions,
            slice_partitions,
            vertex_format,
            offset_attribute,
        }
    }

    /// Computes the geometric representation of the ellipsoid.
    pub fn create_geometry(&self) -> Option<Geometry> {
        let radii = &self.radii;
        if radii.x <= 0.0 || radii.y <= 0.0 || radii.z <= 0.0 {
            return None;
        }
        let inner_radii = &self.inner_radii;
        if inner_radii.x <= 0.0 || inner_radii.y <= 0.0 || inner_radii.z <= 0.0 {
            return None;
        }

        let minimum_clock = self.minimum_clock;
        let maximum_clock = self.maximum_clock;
        let minimum_cone = self.minimum_cone;
        let maximum_cone = self.maximum_cone;
        let vertex_format = &self.vertex_format;

        let mut slice_partitions = (self.slice_partitions + 1) as f64;
        let mut stack_partitions = (self.stack_partitions + 1) as f64;

        slice_partitions = ((slice_partitions * (maximum_clock - minimum_clock).abs()) / CesiumMath::TWO_PI).round();
        stack_partitions = ((stack_partitions * (maximum_cone - minimum_cone).abs()) / CesiumMath::PI).round();

        if slice_partitions < 2.0 { slice_partitions = 2.0; }
        if stack_partitions < 2.0 { stack_partitions = 2.0; }

        let slice_partitions = slice_partitions as usize;
        let stack_partitions = stack_partitions as usize;

        // Build phi and theta arrays
        let mut phis = vec![minimum_cone];
        for i in 0..stack_partitions {
            phis.push(minimum_cone + (i as f64 * (maximum_cone - minimum_cone)) / (stack_partitions as f64 - 1.0));
        }
        phis.push(maximum_cone);

        let mut thetas = vec![minimum_clock];
        for j in 0..slice_partitions {
            thetas.push(minimum_clock + (j as f64 * (maximum_clock - minimum_clock)) / (slice_partitions as f64 - 1.0));
        }
        thetas.push(maximum_clock);

        let num_phis = phis.len();
        let num_thetas = thetas.len();

        let has_inner_surface = inner_radii.x != radii.x || inner_radii.y != radii.y || inner_radii.z != radii.z;
        let vertex_multiplier = if has_inner_surface { 2 } else { 1 };

        let mut extra_indices: usize = 0;
        let mut is_top_open = false;
        let mut is_bot_open = false;
        let is_clock_open;
        if has_inner_surface {
            if minimum_cone > 0.0 {
                is_top_open = true;
                extra_indices += slice_partitions - 1;
            }
            if maximum_cone < std::f64::consts::PI {
                is_bot_open = true;
                extra_indices += slice_partitions - 1;
            }
            if (maximum_clock - minimum_clock) % CesiumMath::TWO_PI != 0.0 {
                is_clock_open = true;
                extra_indices += (stack_partitions - 1) * 2 + 1;
            } else {
                is_clock_open = false;
                extra_indices += 1;
            }
        } else {
            is_top_open = false;
            is_bot_open = false;
            is_clock_open = false;
        }

        let vertex_count = num_thetas * num_phis * vertex_multiplier;
        let mut positions = vec![0.0f64; vertex_count * 3];
        let mut is_inner = vec![false; vertex_count];
        let mut negate_normal = vec![false; vertex_count];

        let index_count = slice_partitions * stack_partitions * vertex_multiplier;
        let num_indices = 6 * (index_count + extra_indices + 1 - (slice_partitions + stack_partitions) * vertex_multiplier);

        let mut indices: Vec<usize> = vec![0; num_indices];

        let normals: Option<Vec<f32>> = if vertex_format.normal { Some(vec![0.0; vertex_count * 3]) } else { None };
        let tangents: Option<Vec<f32>> = if vertex_format.tangent { Some(vec![0.0; vertex_count * 3]) } else { None };
        let bitangents: Option<Vec<f32>> = if vertex_format.bitangent { Some(vec![0.0; vertex_count * 3]) } else { None };
        let st: Option<Vec<f32>> = if vertex_format.st { Some(vec![0.0; vertex_count * 2]) } else { None };

        // Precompute sin/cos
        let sin_phi: Vec<f64> = phis.iter().map(|p| p.sin()).collect();
        let cos_phi: Vec<f64> = phis.iter().map(|p| p.cos()).collect();
        let sin_theta: Vec<f64> = thetas.iter().map(|t| t.sin()).collect();
        let cos_theta: Vec<f64> = thetas.iter().map(|t| t.cos()).collect();

        // Outer surface positions
        let mut index = 0;
        for i in 0..num_phis {
            for j in 0..num_thetas {
                positions[index] = radii.x * sin_phi[i] * cos_theta[j];
                positions[index + 1] = radii.y * sin_phi[i] * sin_theta[j];
                positions[index + 2] = radii.z * cos_phi[i];
                index += 3;
            }
        }

        // Inner surface positions
        if has_inner_surface {
            let mut vertex_index = vertex_count / 2;
            for i in 0..num_phis {
                for j in 0..num_thetas {
                    positions[index] = inner_radii.x * sin_phi[i] * cos_theta[j];
                    positions[index + 1] = inner_radii.y * sin_phi[i] * sin_theta[j];
                    positions[index + 2] = inner_radii.z * cos_phi[i];
                    index += 3;

                    is_inner[vertex_index] = true;
                    if i > 0 && i != num_phis - 1 && j != 0 && j != num_thetas - 1 {
                        negate_normal[vertex_index] = true;
                    }
                    vertex_index += 1;
                }
            }
        }

        // Outer surface indices
        index = 0;
        for i in 1..num_phis - 2 {
            let top_offset = i * num_thetas;
            let bottom_offset = (i + 1) * num_thetas;
            for j in 1..num_thetas - 2 {
                indices[index] = bottom_offset + j;
                indices[index + 1] = bottom_offset + j + 1;
                indices[index + 2] = top_offset + j + 1;
                indices[index + 3] = bottom_offset + j;
                indices[index + 4] = top_offset + j + 1;
                indices[index + 5] = top_offset + j;
                index += 6;
            }
        }

        // Inner surface indices
        if has_inner_surface {
            let offset = num_phis * num_thetas;
            for i in 1..num_phis - 2 {
                let top_offset = offset + i * num_thetas;
                let bottom_offset = offset + (i + 1) * num_thetas;
                for j in 1..num_thetas - 2 {
                    indices[index] = bottom_offset + j;
                    indices[index + 1] = top_offset + j;
                    indices[index + 2] = top_offset + j + 1;
                    indices[index + 3] = bottom_offset + j;
                    indices[index + 4] = top_offset + j + 1;
                    indices[index + 5] = bottom_offset + j + 1;
                    index += 6;
                }
            }
        }

        // Connect top/bottom if open
        if has_inner_surface {
            if is_top_open {
                let inner_offset = num_phis * num_thetas;
                for i in 1..num_thetas - 2 {
                    indices[index] = i;
                    indices[index + 1] = i + 1;
                    indices[index + 2] = inner_offset + i + 1;
                    indices[index + 3] = i;
                    indices[index + 4] = inner_offset + i + 1;
                    indices[index + 5] = inner_offset + i;
                    index += 6;
                }
            }
            if is_bot_open {
                let outer_offset = num_phis * num_thetas - num_thetas;
                let inner_offset = num_phis * num_thetas * vertex_multiplier - num_thetas;
                for i in 1..num_thetas - 2 {
                    indices[index] = outer_offset + i + 1;
                    indices[index + 1] = outer_offset + i;
                    indices[index + 2] = inner_offset + i;
                    indices[index + 3] = outer_offset + i + 1;
                    indices[index + 4] = inner_offset + i;
                    indices[index + 5] = inner_offset + i + 1;
                    index += 6;
                }
            }
        }

        // Connect edges if clock is not closed
        if is_clock_open {
            for i in 1..num_phis - 2 {
                let inner_offset = num_thetas * num_phis + num_thetas * i;
                let outer_offset = num_thetas * i;
                indices[index] = inner_offset;
                indices[index + 1] = outer_offset + num_thetas;
                indices[index + 2] = outer_offset;
                indices[index + 3] = inner_offset;
                indices[index + 4] = inner_offset + num_thetas;
                indices[index + 5] = outer_offset + num_thetas;
                index += 6;
            }
            for i in 1..num_phis - 2 {
                let inner_offset = num_thetas * num_phis + num_thetas * (i + 1) - 1;
                let outer_offset = num_thetas * (i + 1) - 1;
                indices[index] = outer_offset + num_thetas;
                indices[index + 1] = inner_offset;
                indices[index + 2] = outer_offset;
                indices[index + 3] = outer_offset + num_thetas;
                indices[index + 4] = inner_offset + num_thetas;
                indices[index + 5] = inner_offset;
                index += 6;
            }
        }

        // Convert indices to the right storage type
        let max_index = *indices.iter().max().unwrap_or(&0);
        let index_storage = if max_index <= 65535 {
            IndexStorage::U16(indices.iter().map(|&i| i as u16).collect())
        } else {
            IndexStorage::U32(indices.iter().map(|&i| i as u32).collect())
        };

        // Build attributes
        let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();
        if vertex_format.position {
            attributes.insert(
                "position".to_string(),
                GeometryAttribute::new(ComponentDatatype::Double, 3, false, positions.clone()),
            );
        }

        let ellipsoid_outer = Ellipsoid::from_cartesian3(Some(radii));
        let ellipsoid_inner = Ellipsoid::from_cartesian3(Some(inner_radii));
        let vertex_count_half = vertex_count / 2;

        if vertex_format.st || vertex_format.normal || vertex_format.tangent || vertex_format.bitangent {
            let mut normals = normals.unwrap_or_default();
            let mut tangents = tangents.unwrap_or_default();
            let mut bitangents = bitangents.unwrap_or_default();
            let mut st = st.unwrap_or_default();

            let mut st_index = 0;
            let mut normal_index = 0;
            let mut tangent_index = 0;
            let mut bitangent_index = 0;

            for i in 0..vertex_count {
                let ellipsoid = if is_inner[i] { &ellipsoid_inner } else { &ellipsoid_outer };
                let position = Cartesian3::new(
                    positions[i * 3],
                    positions[i * 3 + 1],
                    positions[i * 3 + 2],
                );
                let mut normal = Cartesian3::ZERO;
                ellipsoid.geodetic_surface_normal(&position, &mut normal);
                if negate_normal[i] {
                    normal = Cartesian3::negate_new(&normal);
                }

                if vertex_format.st {
                    let neg_normal = Cartesian2::new(-normal.x, -normal.y);
                    st[st_index] = (neg_normal.y.atan2(neg_normal.x) / CesiumMath::TWO_PI + 0.5) as f32;
                    st[st_index + 1] = (normal.z.asin() / std::f64::consts::PI + 0.5) as f32;
                    st_index += 2;
                }

                if vertex_format.normal {
                    normals[normal_index] = normal.x as f32;
                    normals[normal_index + 1] = normal.y as f32;
                    normals[normal_index + 2] = normal.z as f32;
                    normal_index += 3;
                }

                if vertex_format.tangent || vertex_format.bitangent {
                    let tangent_offset = if is_inner[i] { vertex_count_half } else { 0 };
                    let unit = if !is_top_open && i >= tangent_offset && i < tangent_offset + num_thetas * 2 {
                        Cartesian3::UNIT_X
                    } else {
                        Cartesian3::UNIT_Z
                    };
                    let mut tangent = Cartesian3::cross_new(&unit, &normal);
                    tangent = Cartesian3::normalize_new(&tangent);

                    if vertex_format.tangent {
                        tangents[tangent_index] = tangent.x as f32;
                        tangents[tangent_index + 1] = tangent.y as f32;
                        tangents[tangent_index + 2] = tangent.z as f32;
                        tangent_index += 3;
                    }

                    if vertex_format.bitangent {
                        let mut bitangent = Cartesian3::cross_new(&normal, &tangent);
                        bitangent = Cartesian3::normalize_new(&bitangent);
                        bitangents[bitangent_index] = bitangent.x as f32;
                        bitangents[bitangent_index + 1] = bitangent.y as f32;
                        bitangents[bitangent_index + 2] = bitangent.z as f32;
                        bitangent_index += 3;
                    }
                }
            }

            if vertex_format.st {
                attributes.insert("st".to_string(), GeometryAttribute::new(ComponentDatatype::Float, 2, false, st.iter().map(|&v| v as f64).collect()));
            }
            if vertex_format.normal {
                attributes.insert("normal".to_string(), GeometryAttribute::new(ComponentDatatype::Float, 3, false, normals.iter().map(|&v| v as f64).collect()));
            }
            if vertex_format.tangent {
                attributes.insert("tangent".to_string(), GeometryAttribute::new(ComponentDatatype::Float, 3, false, tangents.iter().map(|&v| v as f64).collect()));
            }
            if vertex_format.bitangent {
                attributes.insert("bitangent".to_string(), GeometryAttribute::new(ComponentDatatype::Float, 3, false, bitangents.iter().map(|&v| v as f64).collect()));
            }
        }

        if let Some(offset) = self.offset_attribute {
            let num_verts = positions.len() / 3;
            let offset_value = if offset == GeometryOffsetAttribute::None { 0.0 } else { 1.0 };
            let apply_offset = vec![offset_value; num_verts];
            attributes.insert(
                "applyOffset".to_string(),
                GeometryAttribute::new(ComponentDatatype::UnsignedByte, 1, false, apply_offset),
            );
        }

        Some(Geometry::new(
            attributes,
            Some(index_storage),
            Some(PrimitiveType::Triangles),
            Some(BoundingSphere::from_ellipsoid(&ellipsoid_outer, None)),
        ))
    }

    /// Returns the geometry for a unit ellipsoid.
    pub fn get_unit_ellipsoid() -> Geometry {
        let geom = Self::new(
            Some(Cartesian3::new(1.0, 1.0, 1.0)),
            None, None, None, None, None, None, None,
            Some(VertexFormat::position_only()),
            None,
        );
        geom.create_geometry().unwrap()
    }
}
