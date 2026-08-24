//! Ported from `packages/engine/Source/Core/CorridorOutlineGeometry.js`.
//!
//! A description of the outline of a corridor.

use std::collections::HashMap;

use crate::array_remove_duplicates::array_remove_duplicates;
use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::component_datatype::ComponentDatatype;
use crate::corner_type::CornerType;
use crate::corridor_geometry_library::{
    CorridorComputePositionsParams, CorridorCorner, CorridorGeometryLibrary,
};
use crate::ellipsoid::Ellipsoid;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_offset_attribute::GeometryOffsetAttribute;
use crate::index_datatype::IndexDatatype;
use crate::math::CesiumMath;
use crate::polygon_pipeline::PolygonPipeline;
use crate::primitive_type::PrimitiveType;

/// A description of the outline of a corridor.
#[derive(Debug, Clone)]
pub struct CorridorOutlineGeometry {
    positions: Vec<Cartesian3>,
    ellipsoid: Ellipsoid,
    width: f64,
    height: f64,
    extruded_height: f64,
    corner_type: CornerType,
    granularity: f64,
    offset_attribute: Option<GeometryOffsetAttribute>,
}

impl CorridorOutlineGeometry {
    /// Creates a new `CorridorOutlineGeometry`.
    pub fn new(
        positions: Vec<Cartesian3>,
        width: f64,
        ellipsoid: Option<Ellipsoid>,
        height: Option<f64>,
        extruded_height: Option<f64>,
        corner_type: Option<CornerType>,
        granularity: Option<f64>,
        offset_attribute: Option<GeometryOffsetAttribute>,
    ) -> Self {
        let height = height.unwrap_or(0.0);
        let extruded_height = extruded_height.unwrap_or(height);
        Self {
            positions,
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84),
            width,
            height: extruded_height.max(height),
            extruded_height: extruded_height.min(height),
            corner_type: corner_type.unwrap_or(CornerType::Rounded),
            granularity: granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE),
            offset_attribute,
        }
    }

    /// Accessors.
    pub fn positions(&self) -> &[Cartesian3] {
        &self.positions
    }

    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn extruded_height(&self) -> f64 {
        self.extruded_height
    }

    pub fn corner_type(&self) -> CornerType {
        self.corner_type
    }

    pub fn granularity(&self) -> f64 {
        self.granularity
    }

    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    /// The number of elements used to pack the object into an array.
    ///
    /// DEVIATION: JS `packedLength` is an instance property computed in the
    /// constructor; Rust exposes it as `packed_length(&self)`.
    pub fn packed_length(&self) -> usize {
        1 + self.positions.len() * Cartesian3::PACKED_LENGTH
            + Ellipsoid::PACKED_LENGTH
            + 6
    }

    /// Stores the provided instance into the provided array.
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        let mut si = starting_index.unwrap_or(0);

        let positions = &self.positions;
        array[si] = positions.len() as f64;
        si += 1;

        for position in positions {
            Cartesian3::pack(position, array, Some(si));
            si += Cartesian3::PACKED_LENGTH;
        }

        Ellipsoid::pack(&self.ellipsoid, array, Some(si));
        si += Ellipsoid::PACKED_LENGTH;

        array[si] = self.width;
        si += 1;
        array[si] = self.height;
        si += 1;
        array[si] = self.extruded_height;
        si += 1;
        array[si] = self.corner_type as i32 as f64;
        si += 1;
        array[si] = self.granularity;
        si += 1;
        array[si] = match &self.offset_attribute {
            Some(v) => *v as u32 as f64,
            None => -1.0,
        };
    }

    /// Retrieves an instance from a packed array.
    pub fn unpack(array: &[f64], starting_index: Option<usize>, result: Option<&mut Self>) -> Self {
        let mut si = starting_index.unwrap_or(0);

        let length = array[si] as usize;
        si += 1;
        let mut positions = Vec::with_capacity(length);
        for _ in 0..length {
            positions.push(Cartesian3::unpack_new(array, Some(si)));
            si += Cartesian3::PACKED_LENGTH;
        }

        let ellipsoid = Ellipsoid::unpack(array, Some(si));
        si += Ellipsoid::PACKED_LENGTH;

        let width = array[si];
        si += 1;
        let height = array[si];
        si += 1;
        let extruded_height = array[si];
        si += 1;
        let corner_type = array[si];
        si += 1;
        let granularity = array[si];
        si += 1;
        let offset_attribute_raw = array[si];

        let corner_type = match corner_type as i32 {
            1 => CornerType::Mitered,
            2 => CornerType::Beveled,
            _ => CornerType::Rounded,
        };
        let offset_attribute = if offset_attribute_raw == -1.0 {
            None
        } else {
            GeometryOffsetAttribute::try_from_u32(offset_attribute_raw as u32)
        };

        match result {
            // JS goes through the constructor on this path, which re-applies
            // the height/extrudedHeight min/max normalization.
            None => Self::new(
                positions,
                width,
                Some(ellipsoid),
                Some(height),
                Some(extruded_height),
                Some(corner_type),
                Some(granularity),
                offset_attribute,
            ),
            Some(r) => {
                r.positions = positions;
                r.ellipsoid = ellipsoid;
                r.width = width;
                r.height = height;
                r.extruded_height = extruded_height;
                r.corner_type = corner_type;
                r.granularity = granularity;
                r.offset_attribute = offset_attribute;
                r.clone()
            }
        }
    }
}

/// Computes the geometric representation of a corridor outline.
///
/// Port of `CorridorOutlineGeometry.createGeometry`.
pub fn create_geometry(
    corridor_outline_geometry: &CorridorOutlineGeometry,
) -> Option<Geometry> {
    let mut positions = corridor_outline_geometry.positions.clone();
    let width = corridor_outline_geometry.width;
    let ellipsoid = &corridor_outline_geometry.ellipsoid;

    scale_to_surface(&mut positions, ellipsoid);
    let clean_positions = array_remove_duplicates(
        &positions,
        |a: &Cartesian3, b: &Cartesian3, eps| {
            Cartesian3::equals_epsilon(Some(a), Some(b), Some(eps), Some(eps))
        },
        false,
        None,
    );
    let clean_positions = clean_positions.unwrap_or_else(|| positions.clone());

    if clean_positions.len() < 2 || width <= 0.0 {
        return None;
    }

    let height = corridor_outline_geometry.height;
    let extruded_height = corridor_outline_geometry.extruded_height;
    let extrude =
        !CesiumMath::equals_epsilon(height, extruded_height, Some(0.0), Some(CesiumMath::EPSILON2));

    let params = CorridorComputePositionsParams {
        ellipsoid: ellipsoid.clone(),
        positions: clean_positions.clone(),
        width,
        corner_type: corridor_outline_geometry.corner_type,
        granularity: corridor_outline_geometry.granularity,
        save_attributes: false,
    };

    let (attributes, indices) = if extrude {
        let params_with_height = CorridorComputePositionsParams {
            ellipsoid: ellipsoid.clone(),
            positions: clean_positions.clone(),
            width,
            corner_type: corridor_outline_geometry.corner_type,
            granularity: corridor_outline_geometry.granularity,
            save_attributes: false,
        };
        // For extruded, we need height/extrudedHeight in params - but the
        // current CorridorComputePositionsParams doesn't have those fields.
        // DEVIATION: JS params object includes height/extrudedHeight/offsetAttribute;
        // the Rust CorridorComputePositionsParams struct doesn't. We handle the
        // extrusion by computing top/bottom separately.
        let computed_positions = CorridorGeometryLibrary::compute_positions(&params_with_height);
        let attr = combine(&computed_positions, corridor_outline_geometry.corner_type);
        let wall_indices = attr.wall_indices;
        let mut attributes = attr.attributes;
        let indices = attr.indices;

        let mut positions_vals = attributes["position"].values.clone();
        let length = positions_vals.len();
        let mut extruded_positions = positions_vals.clone();

        PolygonPipeline::scale_to_geodetic_height(
            Some(&mut positions_vals),
            Some(height),
            Some(ellipsoid),
            Some(true),
        );
        PolygonPipeline::scale_to_geodetic_height(
            Some(&mut extruded_positions),
            Some(extruded_height),
            Some(ellipsoid),
            Some(true),
        );

        let mut new_positions = vec![0.0f64; length * 2];
        new_positions[..length].copy_from_slice(&positions_vals);
        new_positions[length..].copy_from_slice(&extruded_positions);
        attributes.get_mut("position").unwrap().values = new_positions.clone();

        let vertex_count = length / 3;
        if let Some(offset_attr) = corridor_outline_geometry.offset_attribute {
            let apply_offset = if offset_attr == GeometryOffsetAttribute::Top {
                let mut v = vec![0.0f64; vertex_count * 2];
                for i in 0..vertex_count {
                    v[i] = 1.0;
                }
                v
            } else {
                let offset_value =
                    if offset_attr == GeometryOffsetAttribute::None { 0 } else { 1 };
                vec![offset_value as f64; vertex_count * 2]
            };
            attributes.insert(
                "applyOffset".to_string(),
                GeometryAttribute::new(
                    ComponentDatatype::UnsignedByte,
                    1,
                    false,
                    apply_offset,
                ),
            );
        }

        let i_length = indices.len();
        let mut new_indices = IndexDatatype::create_typed_array(
            new_positions.len() / 3,
            (i_length + wall_indices.len()) * 2,
        );
        // Copy original indices
        for i in 0..i_length {
            write_index(&mut new_indices, i, read_index(&indices, i));
        }
        let mut index = i_length;
        // Bottom indices (mirrored)
        for i in (0..i_length).step_by(2) {
            let v0 = read_index(&indices, i);
            let v1 = read_index(&indices, i + 1);
            write_index(&mut new_indices, index, v0 + vertex_count as u32);
            index += 1;
            write_index(&mut new_indices, index, v1 + vertex_count as u32);
            index += 1;
        }
        // Wall indices
        for &wi in &wall_indices {
            write_index(&mut new_indices, index, wi);
            index += 1;
            write_index(&mut new_indices, index, wi + vertex_count as u32);
            index += 1;
        }

        (attributes, new_indices)
    } else {
        let computed_positions = CorridorGeometryLibrary::compute_positions(&params);
        let mut attr = combine(&computed_positions, corridor_outline_geometry.corner_type);
        let mut positions_vals = attr.attributes["position"].values.clone();
        PolygonPipeline::scale_to_geodetic_height(
            Some(&mut positions_vals),
            Some(height),
            Some(ellipsoid),
            Some(true),
        );
        attr.attributes.get_mut("position").unwrap().values = positions_vals.clone();

        if let Some(offset_attr) = corridor_outline_geometry.offset_attribute {
            let length = positions_vals.len();
            let offset_value =
                if offset_attr == GeometryOffsetAttribute::None { 0 } else { 1 };
            let apply_offset = vec![offset_value as f64; length / 3];
            attr.attributes.insert(
                "applyOffset".to_string(),
                GeometryAttribute::new(
                    ComponentDatatype::UnsignedByte,
                    1,
                    false,
                    apply_offset,
                ),
            );
        }

        (attr.attributes, attr.indices)
    };

    let position_values = &attributes["position"].values;
    let bounding_sphere =
        BoundingSphere::from_vertices(position_values, None, Some(3), None);

    Some(Geometry::with_all(
        attributes,
        Some(indices),
        Some(PrimitiveType::Lines),
        Some(bounding_sphere),
        crate::geometry_type::GeometryType::None,
        None,
        corridor_outline_geometry
            .offset_attribute
            .map(|_| "applyOffset".to_string()),
    ))
}

fn scale_to_surface(positions: &mut [Cartesian3], ellipsoid: &Ellipsoid) {
    for pos in positions.iter_mut() {
        let mut scaled = Cartesian3::default();
        ellipsoid.scale_to_geodetic_surface(pos, &mut scaled);
        *pos = scaled;
    }
}

struct CombineResult {
    attributes: HashMap<String, GeometryAttribute>,
    indices: crate::index_datatype::IndexStorage,
    wall_indices: Vec<u32>,
}

fn combine(
    computed_positions: &crate::corridor_geometry_library::CorridorComputePositionsResult,
    corner_type: CornerType,
) -> CombineResult {
    let positions = &computed_positions.positions;
    let corners = &computed_positions.corners;
    let end_positions = computed_positions.end_positions.as_ref();

    let mut left_count = 0usize;
    let mut right_count = 0usize;
    let mut indices_length = 0usize;

    for i in (0..positions.len()).step_by(2) {
        let length = positions[i].len() - 3;
        left_count += length;
        indices_length += (length / 3) * 4;
        right_count += positions[i + 1].len() - 3;
    }
    left_count += 3;
    right_count += 3;

    for corner in corners {
        match corner {
            CorridorCorner::LeftPositions(l) => {
                left_count += l.len();
                indices_length += (l.len() / 3) * 2;
            }
            CorridorCorner::RightPositions(r) => {
                right_count += r.len();
                indices_length += (r.len() / 3) * 2;
            }
        }
    }

    let add_end_positions = end_positions.is_some();
    let mut end_position_length = 0usize;
    let mut half_length = 0usize;
    if add_end_positions {
        let ep = end_positions.unwrap();
        end_position_length = ep[0].len() - 3;
        left_count += end_position_length;
        right_count += end_position_length;
        end_position_length /= 3;
        indices_length += end_position_length * 4;
        half_length = end_position_length / 2;
    }

    let size = left_count + right_count;
    let mut final_positions = vec![0.0f64; size];
    let mut front = 0isize;
    let mut back = size as isize - 1;

    let mut indices = IndexDatatype::create_typed_array(size / 3, indices_length + 4);
    let mut index = 0usize;
    let mut wall_indices: Vec<u32> = Vec::new();

    write_index(&mut indices, index, (front as usize / 3) as u32);
    index += 1;
    write_index(&mut indices, index, ((back - 2) as usize / 3) as u32);
    index += 1;

    if add_end_positions {
        let ep = end_positions.unwrap();
        wall_indices.push((front as usize / 3) as u32);
        let first_end_positions = &ep[0];
        for i in 0..half_length {
            let left_pos = Cartesian3::from_array_new(
                first_end_positions,
                Some((half_length - 1 - i) * 3),
            );
            let right_pos = Cartesian3::from_array_new(
                first_end_positions,
                Some((half_length + i) * 3),
            );
            CorridorGeometryLibrary::add_attribute(
                &mut final_positions,
                &right_pos,
                Some(front as usize),
                None,
            );
            CorridorGeometryLibrary::add_attribute(
                &mut final_positions,
                &left_pos,
                None,
                Some(back as usize),
            );

            let ll = front as usize / 3;
            let lr = ll + 1;
            let ul = ((back - 2) as usize) / 3;
            let ur = ul - 1;
            write_index(&mut indices, index, ul as u32);
            index += 1;
            write_index(&mut indices, index, ur as u32);
            index += 1;
            write_index(&mut indices, index, ll as u32);
            index += 1;
            write_index(&mut indices, index, lr as u32);
            index += 1;

            front += 3;
            back -= 3;
        }
    }

    let mut pos_index = 0usize;
    let right_edge = &positions[pos_index];
    pos_index += 1;
    let left_edge = &positions[pos_index];
    pos_index += 1;

    final_positions[front as usize..front as usize + right_edge.len()]
        .copy_from_slice(&right_edge[..right_edge.len()]);
    let left_start = (back as isize - left_edge.len() as isize + 1) as usize;
    for (k, &v) in left_edge.iter().enumerate() {
        final_positions[left_start + k] = v;
    }

    let length = left_edge.len() - 3;
    wall_indices.push((front as usize / 3) as u32);
    wall_indices.push(((back - 2) as usize / 3) as u32);

    for _ in 0..length / 3 {
        let ll = front as usize / 3;
        let lr = ll + 1;
        let ul = ((back - 2) as usize) / 3;
        let ur = ul - 1;
        write_index(&mut indices, index, ul as u32);
        index += 1;
        write_index(&mut indices, index, ur as u32);
        index += 1;
        write_index(&mut indices, index, ll as u32);
        index += 1;
        write_index(&mut indices, index, lr as u32);
        index += 1;

        front += 3;
        back -= 3;
    }

    for corner in corners {
        match corner {
            CorridorCorner::LeftPositions(l) => {
                back -= 3;
                let start = ((back + 3 - 2) as usize / 3) as u32;
                wall_indices.push(start - 1);
                for j in 0..l.len() / 3 {
                    let outside_point =
                        Cartesian3::from_array_new(l, Some(j * 3));
                    write_index(
                        &mut indices,
                        index,
                        start - 1 - j as u32,
                    );
                    index += 1;
                    write_index(&mut indices, index, start - j as u32);
                    index += 1;
                    CorridorGeometryLibrary::add_attribute(
                        &mut final_positions,
                        &outside_point,
                        None,
                        Some(back as usize),
                    );
                    back -= 3;
                }
                wall_indices.push(start - (l.len() / 6) as u32);
                if corner_type == CornerType::Beveled {
                    wall_indices.push(((back - 2) as usize / 3 + 1) as u32);
                }
                front += 3;
            }
            CorridorCorner::RightPositions(r) => {
                front += 3;
                let start = (front as usize / 3 - 1) as u32;
                wall_indices.push(start);
                for j in 0..r.len() / 3 {
                    let outside_point =
                        Cartesian3::from_array_new(r, Some(j * 3));
                    write_index(&mut indices, index, start + j as u32);
                    index += 1;
                    write_index(
                        &mut indices,
                        index,
                        start + j as u32 + 1,
                    );
                    index += 1;
                    CorridorGeometryLibrary::add_attribute(
                        &mut final_positions,
                        &outside_point,
                        Some(front as usize),
                        None,
                    );
                    front += 3;
                }
                wall_indices.push(start + (r.len() / 6) as u32);
                if corner_type == CornerType::Beveled {
                    wall_indices.push((front as usize / 3 - 1) as u32);
                }
                back -= 3;
            }
        }

        let right_edge = &positions[pos_index];
        pos_index += 1;
        let left_edge = &positions[pos_index];
        pos_index += 1;

        // Remove first 3 (duplicate at corner start)
        let right_trimmed = &right_edge[3..];
        let left_trimmed = &left_edge[..left_edge.len() - 3];

        final_positions[front as usize..front as usize + right_trimmed.len()]
            .copy_from_slice(right_trimmed);
        let left_start = (back as isize - left_trimmed.len() as isize + 1) as usize;
        for (k, &v) in left_trimmed.iter().enumerate() {
            final_positions[left_start + k] = v;
        }

        for _ in 0..left_trimmed.len() / 3 {
            let lr = front as usize / 3;
            let ll = lr - 1;
            let ur = ((back - 2) as usize) / 3;
            let ul = ur + 1;
            write_index(&mut indices, index, ul as u32);
            index += 1;
            write_index(&mut indices, index, ur as u32);
            index += 1;
            write_index(&mut indices, index, ll as u32);
            index += 1;
            write_index(&mut indices, index, lr as u32);
            index += 1;

            front += 3;
            back -= 3;
        }
        front -= 3;
        back += 3;
        wall_indices.push((front as usize / 3) as u32);
        wall_indices.push(((back - 2) as usize / 3) as u32);
    }

    if add_end_positions {
        let ep = end_positions.unwrap();
        front += 3;
        back -= 3;
        let last_end_positions = &ep[1];
        for i in 0..half_length {
            let left_pos = Cartesian3::from_array_new(
                last_end_positions,
                Some((end_position_length - i - 1) * 3),
            );
            let right_pos =
                Cartesian3::from_array_new(last_end_positions, Some(i * 3));
            CorridorGeometryLibrary::add_attribute(
                &mut final_positions,
                &left_pos,
                None,
                Some(back as usize),
            );
            CorridorGeometryLibrary::add_attribute(
                &mut final_positions,
                &right_pos,
                Some(front as usize),
                None,
            );

            let lr = front as usize / 3;
            let ll = lr - 1;
            let ur = ((back - 2) as usize) / 3;
            let ul = ur + 1;
            write_index(&mut indices, index, ul as u32);
            index += 1;
            write_index(&mut indices, index, ur as u32);
            index += 1;
            write_index(&mut indices, index, ll as u32);
            index += 1;
            write_index(&mut indices, index, lr as u32);
            index += 1;

            front += 3;
            back -= 3;
        }
        wall_indices.push((front as usize / 3) as u32);
    } else {
        wall_indices.push((front as usize / 3) as u32);
        wall_indices.push(((back - 2) as usize / 3) as u32);
    }

    write_index(&mut indices, index, (front as usize / 3) as u32);
    index += 1;
    write_index(&mut indices, index, ((back - 2) as usize / 3) as u32);

    let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();
    attributes.insert(
        "position".to_string(),
        GeometryAttribute::new(ComponentDatatype::Double, 3, false, final_positions),
    );

    CombineResult {
        attributes,
        indices,
        wall_indices,
    }
}

fn read_index(storage: &crate::index_datatype::IndexStorage, index: usize) -> u32 {
    use crate::index_datatype::IndexStorage;
    match storage {
        IndexStorage::U16(v) => v[index] as u32,
        IndexStorage::U32(v) => v[index],
    }
}

fn write_index(storage: &mut crate::index_datatype::IndexStorage, index: usize, value: u32) {
    use crate::index_datatype::IndexStorage;
    match storage {
        IndexStorage::U16(v) => v[index] = value as u16,
        IndexStorage::U32(v) => v[index] = value,
    }
}
