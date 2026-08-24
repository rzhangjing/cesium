//! Ported from `packages/engine/Source/Core/WallGeometry.js`.
//!
//! A wall, similar to a KML line string, defined by a series of points
//! that extrude down to the ground.

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::component_datatype::ComponentDatatype;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::math::CesiumMath;
use crate::primitive_type::PrimitiveType;
use crate::vertex_format::VertexFormat;
use crate::wall_geometry_library;

/// A description of a wall.
#[derive(Debug, Clone)]
pub struct WallGeometry {
    positions: Vec<Cartesian3>,
    maximum_heights: Option<Vec<f64>>,
    minimum_heights: Option<Vec<f64>>,
    vertex_format: VertexFormat,
    granularity: f64,
    ellipsoid: Ellipsoid,
}

impl WallGeometry {
    /// Creates a new `WallGeometry`.
    pub fn new(
        positions: Vec<Cartesian3>,
        maximum_heights: Option<Vec<f64>>,
        minimum_heights: Option<Vec<f64>>,
        vertex_format: Option<VertexFormat>,
        granularity: Option<f64>,
        ellipsoid: Option<Ellipsoid>,
    ) -> Self {
        debug_assert!(positions.len() >= 2, "At least 2 positions required.");
        if let Some(ref mh) = maximum_heights {
            debug_assert_eq!(mh.len(), positions.len());
        }
        if let Some(ref mh) = minimum_heights {
            debug_assert_eq!(mh.len(), positions.len());
        }
        Self {
            positions,
            maximum_heights,
            minimum_heights,
            vertex_format: vertex_format.unwrap_or_default(),
            granularity: granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE),
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84.clone()),
        }
    }

    /// Creates a wall from constant min/max heights.
    pub fn from_constant_heights(
        positions: Vec<Cartesian3>,
        minimum_height: Option<f64>,
        maximum_height: Option<f64>,
        vertex_format: Option<VertexFormat>,
        ellipsoid: Option<Ellipsoid>,
    ) -> Self {
        let len = positions.len();
        let min_heights = minimum_height.map(|h| vec![h; len]);
        let max_heights = maximum_height.map(|h| vec![h; len]);
        Self::new(positions, max_heights, min_heights, vertex_format, None, ellipsoid)
    }

    /// The positions.
    pub fn positions(&self) -> &[Cartesian3] {
        &self.positions
    }

    /// The minimum heights (JS `_minimumHeights`).
    pub fn minimum_heights(&self) -> Option<&Vec<f64>> {
        self.minimum_heights.as_ref()
    }

    /// The maximum heights (JS `_maximumHeights`).
    pub fn maximum_heights(&self) -> Option<&Vec<f64>> {
        self.maximum_heights.as_ref()
    }

    /// The vertex format (JS `_vertexFormat`).
    pub fn vertex_format(&self) -> &VertexFormat {
        &self.vertex_format
    }

    /// The granularity (JS `_granularity`).
    pub fn granularity(&self) -> f64 {
        self.granularity
    }

    /// The ellipsoid (JS `_ellipsoid`).
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    /// The number of elements used to pack the object into an array.
    ///
    /// DEVIATION: JS exposes `packedLength` as an instance property computed
    /// in the constructor; Rust computes it on demand (always equal).
    pub fn packed_length(&self) -> usize {
        let mut num_components =
            1 + self.positions.len() * Cartesian3::PACKED_LENGTH + 2;
        if let Some(minimum_heights) = &self.minimum_heights {
            num_components += minimum_heights.len();
        }
        if let Some(maximum_heights) = &self.maximum_heights {
            num_components += maximum_heights.len();
        }
        num_components + Ellipsoid::PACKED_LENGTH + VertexFormat::PACKED_LENGTH + 1
    }

    /// Stores this instance into `array` (JS `WallGeometry.pack`).
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        let mut si = starting_index.unwrap_or(0);

        let length = self.positions.len();
        array[si] = length as f64;
        si += 1;

        for position in &self.positions {
            Cartesian3::pack(position, array, Some(si));
            si += Cartesian3::PACKED_LENGTH;
        }

        let min_length = self.minimum_heights.as_ref().map_or(0, |h| h.len());
        array[si] = min_length as f64;
        si += 1;
        if let Some(minimum_heights) = &self.minimum_heights {
            for height in minimum_heights {
                array[si] = *height;
                si += 1;
            }
        }

        let max_length = self.maximum_heights.as_ref().map_or(0, |h| h.len());
        array[si] = max_length as f64;
        si += 1;
        if let Some(maximum_heights) = &self.maximum_heights {
            for height in maximum_heights {
                array[si] = *height;
                si += 1;
            }
        }

        Ellipsoid::pack(&self.ellipsoid, array, Some(si));
        si += Ellipsoid::PACKED_LENGTH;

        self.vertex_format.pack(array, si);
        si += VertexFormat::PACKED_LENGTH;

        array[si] = self.granularity;
    }

    /// Retrieves an instance from a packed array (JS `WallGeometry.unpack`).
    pub fn unpack(array: &[f64], starting_index: Option<usize>, result: Option<&mut Self>) -> Self {
        let mut si = starting_index.unwrap_or(0);

        let mut length = array[si] as usize;
        si += 1;
        let mut positions = Vec::with_capacity(length);
        for _ in 0..length {
            positions.push(Cartesian3::unpack_new(array, Some(si)));
            si += Cartesian3::PACKED_LENGTH;
        }

        length = array[si] as usize;
        si += 1;
        let mut minimum_heights: Option<Vec<f64>> = None;
        if length > 0 {
            let mut heights = Vec::with_capacity(length);
            for _ in 0..length {
                heights.push(array[si]);
                si += 1;
            }
            minimum_heights = Some(heights);
        }

        length = array[si] as usize;
        si += 1;
        let mut maximum_heights: Option<Vec<f64>> = None;
        if length > 0 {
            let mut heights = Vec::with_capacity(length);
            for _ in 0..length {
                heights.push(array[si]);
                si += 1;
            }
            maximum_heights = Some(heights);
        }

        let ellipsoid = Ellipsoid::unpack(array, Some(si));
        si += Ellipsoid::PACKED_LENGTH;

        let vertex_format = VertexFormat::unpack(array, si, None);
        si += VertexFormat::PACKED_LENGTH;

        let granularity = array[si];

        match result {
            None => Self::new(
                positions,
                maximum_heights,
                minimum_heights,
                Some(vertex_format),
                Some(granularity),
                Some(ellipsoid),
            ),
            Some(r) => {
                r.positions = positions;
                r.minimum_heights = minimum_heights;
                r.maximum_heights = maximum_heights;
                r.ellipsoid = ellipsoid;
                r.vertex_format = vertex_format;
                r.granularity = granularity;
                r.clone()
            }
        }
    }

    /// Computes the geometric representation of a wall, including its
    /// vertices, indices, and a bounding sphere (JS
    /// `WallGeometry.createGeometry`).
    pub fn create_geometry(&self) -> Option<Geometry> {
        let wall_positions = &self.positions;
        let vertex_format = &self.vertex_format;
        let ellipsoid = &self.ellipsoid;

        let pos = wall_geometry_library::compute_positions(
            ellipsoid,
            wall_positions,
            self.maximum_heights.as_deref(),
            self.minimum_heights.as_deref(),
            self.granularity,
            true,
        )?;

        let bottom_positions = &pos.bottom_positions;
        let top_positions = &pos.top_positions;
        let num_corners = pos.num_corners;

        let mut length = top_positions.len();
        let size = length * 2;

        let mut positions: Option<Vec<f64>> =
            if vertex_format.position { Some(vec![0.0; size]) } else { None };
        let mut normals: Option<Vec<f64>> =
            if vertex_format.normal { Some(vec![0.0; size]) } else { None };
        let mut tangents: Option<Vec<f64>> =
            if vertex_format.tangent { Some(vec![0.0; size]) } else { None };
        let mut bitangents: Option<Vec<f64>> =
            if vertex_format.bitangent { Some(vec![0.0; size]) } else { None };
        let mut texture_coordinates: Option<Vec<f64>> =
            if vertex_format.st { Some(vec![0.0; (size / 3) * 2]) } else { None };

        let mut position_index = 0usize;
        let mut normal_index = 0usize;
        let mut bitangent_index = 0usize;
        let mut tangent_index = 0usize;
        let mut st_index = 0usize;

        // Add lower and upper points one after the other, lower
        // points being even and upper points being odd.
        let mut normal = Cartesian3::default();
        let mut tangent = Cartesian3::default();
        let mut bitangent = Cartesian3::default();
        let mut recompute_normal = true;
        length /= 3;
        let mut s = 0.0f64;
        let ds = 1.0 / (length - num_corners - 1) as f64;
        for i in 0..length {
            let i3 = i * 3;
            let top_position = Cartesian3::from_array_new(top_positions, Some(i3));
            let bottom_position = Cartesian3::from_array_new(bottom_positions, Some(i3));
            if let Some(positions) = &mut positions {
                // insert the lower point
                positions[position_index] = bottom_position.x;
                position_index += 1;
                positions[position_index] = bottom_position.y;
                position_index += 1;
                positions[position_index] = bottom_position.z;
                position_index += 1;

                // insert the upper point
                positions[position_index] = top_position.x;
                position_index += 1;
                positions[position_index] = top_position.y;
                position_index += 1;
                positions[position_index] = top_position.z;
                position_index += 1;
            }

            if let Some(texture_coordinates) = &mut texture_coordinates {
                texture_coordinates[st_index] = s;
                st_index += 1;
                texture_coordinates[st_index] = 0.0;
                st_index += 1;

                texture_coordinates[st_index] = s;
                st_index += 1;
                texture_coordinates[st_index] = 1.0;
                st_index += 1;
            }

            if vertex_format.normal || vertex_format.tangent || vertex_format.bitangent {
                let mut next_top = Cartesian3::new(0.0, 0.0, 0.0);
                let mut surface_normal = Cartesian3::default();
                ellipsoid.geodetic_surface_normal(&top_position, &mut surface_normal);
                let ground_position = Cartesian3::subtract_new(&top_position, &surface_normal);
                if i + 1 < length {
                    next_top = Cartesian3::from_array_new(top_positions, Some(i3 + 3));
                }

                if recompute_normal {
                    let scaled_next_position = Cartesian3::subtract_new(&next_top, &top_position);
                    let scaled_ground_position =
                        Cartesian3::subtract_new(&ground_position, &top_position);
                    let mut cross = Cartesian3::default();
                    Cartesian3::cross(&scaled_ground_position, &scaled_next_position, &mut cross);
                    Cartesian3::normalize(&cross, &mut normal);
                    recompute_normal = false;
                }

                if Cartesian3::equals_epsilon(
                    Some(&top_position),
                    Some(&next_top),
                    Some(CesiumMath::EPSILON10),
                    Some(CesiumMath::EPSILON10),
                ) {
                    recompute_normal = true;
                } else {
                    s += ds;
                    if vertex_format.tangent {
                        let diff = Cartesian3::subtract_new(&next_top, &top_position);
                        let mut normalized = Cartesian3::default();
                        Cartesian3::normalize(&diff, &mut normalized);
                        tangent = normalized;
                    }
                    if vertex_format.bitangent {
                        let mut cross = Cartesian3::default();
                        Cartesian3::cross(&normal, &tangent, &mut cross);
                        Cartesian3::normalize(&cross, &mut bitangent);
                    }
                }

                if let Some(normals) = &mut normals {
                    normals[normal_index] = normal.x;
                    normal_index += 1;
                    normals[normal_index] = normal.y;
                    normal_index += 1;
                    normals[normal_index] = normal.z;
                    normal_index += 1;

                    normals[normal_index] = normal.x;
                    normal_index += 1;
                    normals[normal_index] = normal.y;
                    normal_index += 1;
                    normals[normal_index] = normal.z;
                    normal_index += 1;
                }

                if let Some(tangents) = &mut tangents {
                    tangents[tangent_index] = tangent.x;
                    tangent_index += 1;
                    tangents[tangent_index] = tangent.y;
                    tangent_index += 1;
                    tangents[tangent_index] = tangent.z;
                    tangent_index += 1;

                    tangents[tangent_index] = tangent.x;
                    tangent_index += 1;
                    tangents[tangent_index] = tangent.y;
                    tangent_index += 1;
                    tangents[tangent_index] = tangent.z;
                    tangent_index += 1;
                }

                if let Some(bitangents) = &mut bitangents {
                    bitangents[bitangent_index] = bitangent.x;
                    bitangent_index += 1;
                    bitangents[bitangent_index] = bitangent.y;
                    bitangent_index += 1;
                    bitangents[bitangent_index] = bitangent.z;
                    bitangent_index += 1;

                    bitangents[bitangent_index] = bitangent.x;
                    bitangent_index += 1;
                    bitangents[bitangent_index] = bitangent.y;
                    bitangent_index += 1;
                    bitangents[bitangent_index] = bitangent.z;
                    bitangent_index += 1;
                }
            }
        }

        let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();

        if let Some(positions) = &positions {
            attributes.insert(
                "position".to_string(),
                GeometryAttribute::new(
                    ComponentDatatype::Double,
                    3,
                    false,
                    positions.clone(),
                ),
            );
        }

        if let Some(normals) = &normals {
            attributes.insert(
                "normal".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 3, false, normals.clone()),
            );
        }

        if let Some(tangents) = &tangents {
            attributes.insert(
                "tangent".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 3, false, tangents.clone()),
            );
        }

        if let Some(bitangents) = &bitangents {
            attributes.insert(
                "bitangent".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 3, false, bitangents.clone()),
            );
        }

        if let Some(texture_coordinates) = &texture_coordinates {
            attributes.insert(
                "st".to_string(),
                GeometryAttribute::new(
                    ComponentDatatype::Float,
                    2,
                    false,
                    texture_coordinates.clone(),
                ),
            );
        }

        // Prepare the side walls, two triangles for each wall.
        let num_vertices = size / 3;
        let mut index_size = size;
        index_size -= 6 * (num_corners + 1);
        let mut indices: IndexStorage =
            IndexDatatype::create_typed_array(num_vertices, index_size);

        // JS skips degenerate quads (`continue`) so fewer indices than
        // allocated may be written; the typed array keeps trailing zeros.
        // The Rust `IndexStorage` cannot hold trailing padding, so only the
        // actually written indices are kept (equivalent primitive set).
        let mut i = 0usize;
        while i + 2 < num_vertices {
            let ll = i;
            let lr = i + 2;
            let positions_ref = positions.as_ref().expect(
                "JS indexes into `positions` here, which is only allocated when \
                 vertexFormat.position is true",
            );
            let pl = Cartesian3::from_array_new(positions_ref, Some(ll * 3));
            let pr = Cartesian3::from_array_new(positions_ref, Some(lr * 3));
            if !Cartesian3::equals_epsilon(
                Some(&pl),
                Some(&pr),
                Some(CesiumMath::EPSILON10),
                Some(CesiumMath::EPSILON10),
            ) {
                let ul = i + 1;
                let ur = i + 3;

                indices.push(ul as u32);
                indices.push(ll as u32);
                indices.push(ur as u32);
                indices.push(ur as u32);
                indices.push(ll as u32);
                indices.push(lr as u32);
            }
            i += 2;
        }

        let positions_for_sphere = positions
            .as_ref()
            .map(|p| p.as_slice())
            .unwrap_or(&[]);

        Some(Geometry::with_all(
            attributes,
            Some(indices),
            Some(PrimitiveType::Triangles),
            Some(BoundingSphere::from_vertices(
                positions_for_sphere,
                None,
                None,
                None,
            )),
            crate::geometry_type::GeometryType::None,
            None,
            None,
        ))
    }
}
