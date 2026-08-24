//! Ported from `packages/engine/Source/Core/CorridorGeometry.js`.
//!
//! A description of a corridor (triangulated polyline with width).
//!
//! DEVIATION: JS `computePositionsExtruded` passes `height`/`extrudedHeight`
//! in the `params` object to `CorridorGeometryLibrary.computePositions`.
//! The Rust `CorridorComputePositionsParams` struct lacks those fields, so
//! extrusion is handled separately via `PolygonPipeline::scale_to_geodetic_height`
//! after the initial surface-level `combine` call.

use std::collections::HashMap;

use crate::array_remove_duplicates::array_remove_duplicates;
use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::component_datatype::ComponentDatatype;
use crate::corner_type::CornerType;
use crate::corridor_geometry_library::{
    CorridorComputePositionsParams, CorridorComputePositionsResult, CorridorCorner,
    CorridorGeometryLibrary,
};
use crate::ellipsoid::Ellipsoid;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_offset_attribute::GeometryOffsetAttribute;
use crate::geometry_type::GeometryType;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::math::CesiumMath;
use crate::polygon_pipeline::PolygonPipeline;
use crate::primitive_type::PrimitiveType;
use crate::rectangle::Rectangle;
use crate::vertex_format::VertexFormat;

/// A description of a corridor. Corridor geometry can be rendered with both
/// `Primitive` and `GroundPrimitive`.
#[derive(Debug, Clone)]
pub struct CorridorGeometry {
    positions: Vec<Cartesian3>,
    ellipsoid: Ellipsoid,
    vertex_format: VertexFormat,
    width: f64,
    height: f64,
    extruded_height: f64,
    corner_type: CornerType,
    granularity: f64,
    shadow_volume: bool,
    offset_attribute: Option<GeometryOffsetAttribute>,
}

impl CorridorGeometry {
    /// Creates a new `CorridorGeometry`.
    pub fn new(
        positions: Vec<Cartesian3>,
        width: f64,
        ellipsoid: Option<Ellipsoid>,
        vertex_format: Option<VertexFormat>,
        height: Option<f64>,
        extruded_height: Option<f64>,
        corner_type: Option<CornerType>,
        granularity: Option<f64>,
        shadow_volume: Option<bool>,
        offset_attribute: Option<GeometryOffsetAttribute>,
    ) -> Self {
        let height = height.unwrap_or(0.0);
        let extruded_height = extruded_height.unwrap_or(height);
        Self {
            positions,
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84),
            vertex_format: vertex_format.unwrap_or_default(),
            width,
            height: height.max(extruded_height),
            extruded_height: height.min(extruded_height),
            corner_type: corner_type.unwrap_or(CornerType::Rounded),
            granularity: granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE),
            shadow_volume: shadow_volume.unwrap_or(false),
            offset_attribute,
        }
    }

    /// The number of elements used to pack the object into an array.
    ///
    /// DEVIATION: JS `packedLength` is an instance property computed in the
    /// constructor; Rust exposes it as `packed_length(&self)`.
    pub fn packed_length(&self) -> usize {
        1 + self.positions.len() * Cartesian3::PACKED_LENGTH
            + Ellipsoid::PACKED_LENGTH
            + VertexFormat::PACKED_LENGTH
            + 7
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

        self.vertex_format.pack(array, si);
        si += VertexFormat::PACKED_LENGTH;

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
        array[si] = if self.shadow_volume { 1.0 } else { 0.0 };
        si += 1;
        array[si] = match &self.offset_attribute {
            Some(v) => *v as i32 as f64,
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

        let vertex_format = VertexFormat::unpack(array, si, None);
        si += VertexFormat::PACKED_LENGTH;

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
        let shadow_volume = array[si] == 1.0;
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
                Some(vertex_format),
                Some(height),
                Some(extruded_height),
                Some(corner_type),
                Some(granularity),
                Some(shadow_volume),
                offset_attribute,
            ),
            Some(r) => {
                r.positions = positions;
                r.ellipsoid = ellipsoid;
                r.vertex_format = vertex_format;
                r.width = width;
                r.height = height;
                r.extruded_height = extruded_height;
                r.corner_type = corner_type;
                r.granularity = granularity;
                r.shadow_volume = shadow_volume;
                r.offset_attribute = offset_attribute;
                r.clone()
            }
        }
    }

    /// Computes the bounding rectangle given the provided options
    /// (JS static `CorridorGeometry.computeRectangle`).
    pub fn compute_rectangle_from_options(
        positions: Vec<Cartesian3>,
        width: f64,
        ellipsoid: Option<Ellipsoid>,
        corner_type: Option<CornerType>,
        result: Option<Rectangle>,
    ) -> Rectangle {
        let ellipsoid = ellipsoid.unwrap_or(Ellipsoid::WGS84);
        let corner_type = corner_type.unwrap_or(CornerType::Rounded);
        compute_rectangle(positions, &ellipsoid, width, corner_type, result)
    }

    /// JS `Object.defineProperties` `rectangle` getter (computed lazily in
    /// JS; recomputed on each call in Rust).
    pub fn rectangle(&self) -> Rectangle {
        compute_rectangle(
            self.positions.clone(),
            &self.ellipsoid,
            self.width,
            self.corner_type,
            None,
        )
    }

    /// For remapping texture coordinates when rendering CorridorGeometries
    /// as GroundPrimitives. Corridors don't support stRotation, so just
    /// return the corners of the original system.
    pub fn texture_coordinate_rotation_points() -> [f64; 6] {
        [0.0, 0.0, 0.0, 1.0, 1.0, 0.0]
    }

    /// Creates a shadow volume corridor geometry from this geometry
    /// (JS private `CorridorGeometry.createShadowVolume`).
    pub fn create_shadow_volume(
        corridor_geometry: &Self,
        min_height_func: &dyn Fn(f64, &Ellipsoid) -> f64,
        max_height_func: &dyn Fn(f64, &Ellipsoid) -> f64,
    ) -> Self {
        let granularity = corridor_geometry.granularity;
        let ellipsoid = corridor_geometry.ellipsoid.clone();

        let min_height = min_height_func(granularity, &ellipsoid);
        let max_height = max_height_func(granularity, &ellipsoid);

        Self::new(
            corridor_geometry.positions.clone(),
            corridor_geometry.width,
            Some(ellipsoid),
            Some(VertexFormat::position_only()),
            Some(max_height),
            Some(min_height),
            Some(corridor_geometry.corner_type),
            Some(granularity),
            Some(true),
            None,
        )
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

    pub fn vertex_format(&self) -> &VertexFormat {
        &self.vertex_format
    }
}

// ─── helpers ────────────────────────────────────────────────────────────────

fn scale_to_surface(positions: &mut [Cartesian3], ellipsoid: &Ellipsoid) {
    for pos in positions.iter_mut() {
        let mut scaled = Cartesian3::default();
        ellipsoid.scale_to_geodetic_surface(pos, &mut scaled);
        *pos = scaled;
    }
}

/// Mirrors JS `computeOffsetPoints`: expands `min`/`max` cartographic bounds
/// with the two width-offset points of the segment `position1`->`position2`.
fn compute_offset_points(
    position1: &Cartesian3,
    position2: &Cartesian3,
    ellipsoid: &Ellipsoid,
    half_width: f64,
    min: &mut Cartographic,
    max: &mut Cartographic,
) {
    // Compute direction of offset the point
    let mut direction = Cartesian3::default();
    Cartesian3::subtract(position2, position1, &mut direction);
    let mut tmp = Cartesian3::default();
    Cartesian3::normalize(&direction, &mut tmp);
    direction = tmp;

    let mut normal = Cartesian3::default();
    ellipsoid.geodetic_surface_normal(position1, &mut normal);

    let mut offset_direction = Cartesian3::default();
    Cartesian3::cross(&direction, &normal, &mut offset_direction);
    Cartesian3::multiply_by_scalar(&offset_direction, half_width, &mut tmp);
    offset_direction = tmp;

    let min_lat = min.latitude;
    let min_lon = min.longitude;
    let max_lat = max.latitude;
    let max_lon = max.longitude;

    let mut min_lat = min_lat;
    let mut min_lon = min_lon;
    let mut max_lat = max_lat;
    let mut max_lon = max_lon;

    // Compute 2 offset points
    let mut offset_point = Cartesian3::default();
    Cartesian3::add(position1, &offset_direction, &mut offset_point);
    let mut carto = Cartographic::default();
    ellipsoid.cartesian_to_cartographic(&offset_point, &mut carto);

    min_lat = min_lat.min(carto.latitude);
    min_lon = min_lon.min(carto.longitude);
    max_lat = max_lat.max(carto.latitude);
    max_lon = max_lon.max(carto.longitude);

    Cartesian3::subtract(position1, &offset_direction, &mut offset_point);
    ellipsoid.cartesian_to_cartographic(&offset_point, &mut carto);

    min_lat = min_lat.min(carto.latitude);
    min_lon = min_lon.min(carto.longitude);
    max_lat = max_lat.max(carto.latitude);
    max_lon = max_lon.max(carto.longitude);

    min.latitude = min_lat;
    min.longitude = min_lon;
    max.latitude = max_lat;
    max.longitude = max_lon;
}

/// Mirrors JS module-level `computeRectangle` of CorridorGeometry.js.
fn compute_rectangle(
    mut positions: Vec<Cartesian3>,
    ellipsoid: &Ellipsoid,
    width: f64,
    corner_type: CornerType,
    result: Option<Rectangle>,
) -> Rectangle {
    scale_to_surface(&mut positions, ellipsoid);
    let clean_positions = array_remove_duplicates(
        &positions,
        |a: &Cartesian3, b: &Cartesian3, eps: f64| {
            Cartesian3::equals_epsilon(Some(a), Some(b), Some(eps), None)
        },
        false,
        None,
    )
    .unwrap_or(positions);

    let length = clean_positions.len();
    if length < 2 || width <= 0.0 {
        return Rectangle::default();
    }
    let half_width = width * 0.5;

    let mut min = Cartographic {
        latitude: f64::INFINITY,
        longitude: f64::INFINITY,
        height: 0.0,
    };
    let mut max = Cartographic {
        latitude: f64::NEG_INFINITY,
        longitude: f64::NEG_INFINITY,
        height: 0.0,
    };

    if corner_type == CornerType::Rounded {
        // Compute start cap
        let first = clean_positions[0];
        let mut offset = Cartesian3::default();
        Cartesian3::subtract(&first, &clean_positions[1], &mut offset);
        let mut tmp = Cartesian3::default();
        Cartesian3::normalize(&offset, &mut tmp);
        offset = tmp;
        Cartesian3::multiply_by_scalar(&offset, half_width, &mut tmp);
        offset = tmp;
        let mut ends = Cartesian3::default();
        Cartesian3::add(&first, &offset, &mut ends);

        let mut carto = Cartographic::default();
        ellipsoid.cartesian_to_cartographic(&ends, &mut carto);
        min.latitude = min.latitude.min(carto.latitude);
        min.longitude = min.longitude.min(carto.longitude);
        max.latitude = max.latitude.max(carto.latitude);
        max.longitude = max.longitude.max(carto.longitude);
    }

    // Compute the rest
    for i in 0..length - 1 {
        compute_offset_points(
            &clean_positions[i],
            &clean_positions[i + 1],
            ellipsoid,
            half_width,
            &mut min,
            &mut max,
        );
    }

    // Compute ending point
    let last = clean_positions[length - 1];
    let mut offset = Cartesian3::default();
    Cartesian3::subtract(&last, &clean_positions[length - 2], &mut offset);
    let mut tmp = Cartesian3::default();
    Cartesian3::normalize(&offset, &mut tmp);
    offset = tmp;
    Cartesian3::multiply_by_scalar(&offset, half_width, &mut tmp);
    offset = tmp;
    let mut ends = Cartesian3::default();
    Cartesian3::add(&last, &offset, &mut ends);
    compute_offset_points(&last, &ends, ellipsoid, half_width, &mut min, &mut max);

    if corner_type == CornerType::Rounded {
        // Compute end cap
        let mut carto = Cartographic::default();
        ellipsoid.cartesian_to_cartographic(&ends, &mut carto);
        min.latitude = min.latitude.min(carto.latitude);
        min.longitude = min.longitude.min(carto.longitude);
        max.latitude = max.latitude.max(carto.latitude);
        max.longitude = max.longitude.max(carto.longitude);
    }

    let mut rectangle = result.unwrap_or_default();
    rectangle.north = max.latitude;
    rectangle.south = min.latitude;
    rectangle.east = max.longitude;
    rectangle.west = min.longitude;

    rectangle
}

fn read_index(storage: &IndexStorage, index: usize) -> u32 {
    match storage {
        IndexStorage::U16(v) => v[index] as u32,
        IndexStorage::U32(v) => v[index],
    }
}

fn write_index(storage: &mut IndexStorage, index: usize, value: u32) {
    match storage {
        IndexStorage::U16(v) => v[index] = value as u16,
        IndexStorage::U32(v) => v[index] = value,
    }
}

/// Port of `addNormals` — writes normal/tangent/bitangent into the attribute
/// arrays at `front` and/or `back` positions.
fn add_normals(
    normals: Option<&mut [f64]>,
    tangents: Option<&mut [f64]>,
    bitangents: Option<&mut [f64]>,
    normal: &Cartesian3,
    left: &Cartesian3,
    front: Option<usize>,
    back: Option<usize>,
    vertex_format: &VertexFormat,
) {
    // forward = normalize(cross(left, normal))
    let cross = Cartesian3::cross_new(left, normal);
    let forward = Cartesian3::normalize_new(&cross);

    if vertex_format.normal {
        if let Some(n) = normals {
            CorridorGeometryLibrary::add_attribute(n, normal, front, back);
        }
    }
    if vertex_format.tangent {
        if let Some(t) = tangents {
            CorridorGeometryLibrary::add_attribute(t, &forward, front, back);
        }
    }
    if vertex_format.bitangent {
        if let Some(b) = bitangents {
            CorridorGeometryLibrary::add_attribute(b, left, front, back);
        }
    }
}

/// Port of `combine` — assembles position, normal, tangent, bitangent, st,
/// and index data from the corridor library output.
fn combine(
    computed_positions: &CorridorComputePositionsResult,
    vertex_format: &VertexFormat,
    ellipsoid: &Ellipsoid,
) -> CombineResult {
    let positions = &computed_positions.positions;
    let corners = &computed_positions.corners;
    let end_positions = computed_positions.end_positions.as_ref();
    let computed_lefts = computed_positions.lefts.as_deref().unwrap_or(&[]);
    let computed_normals = computed_positions.normals.as_deref().unwrap_or(&[]);

    let mut left_count = 0usize;
    let mut right_count = 0usize;
    let mut indices_length = 0usize;
    let mut length;

    for i in (0..positions.len()).step_by(2) {
        length = positions[i].len() - 3;
        left_count += length;
        indices_length += length * 2;
        right_count += positions[i + 1].len() - 3;
    }
    left_count += 3;
    right_count += 3;

    for corner in corners {
        match corner {
            CorridorCorner::LeftPositions(l) => {
                left_count += l.len();
                indices_length += l.len();
            }
            CorridorCorner::RightPositions(r) => {
                right_count += r.len();
                indices_length += r.len();
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
        half_length = end_position_length / 6; // ep[0].len()/6  →  half of endPositionLength/3
        end_position_length /= 3;
        indices_length += end_position_length * 6;
    }

    let size = left_count + right_count;
    let mut final_positions = vec![0.0f64; size];
    let mut normals: Vec<f64> = if vertex_format.normal {
        vec![0.0f64; size]
    } else {
        Vec::new()
    };
    let mut tangents: Vec<f64> = if vertex_format.tangent {
        vec![0.0f64; size]
    } else {
        Vec::new()
    };
    let mut bitangents: Vec<f64> = if vertex_format.bitangent {
        vec![0.0f64; size]
    } else {
        Vec::new()
    };

    let mut front = 0isize;
    let mut back = size as isize - 1;
    let mut ul;
    let mut ll;
    let mut ur;
    let mut lr;
    let mut normal = Cartesian3::default();
    let mut left = Cartesian3::default();
    let mut right_pos = Cartesian3::default();
    let mut left_pos = Cartesian3::default();

    let mut indices = IndexDatatype::create_typed_array(size / 3, indices_length);
    let mut index = 0usize;

    if add_end_positions {
        let ep = end_positions.unwrap();
        let first_end_positions = &ep[0];
        Cartesian3::from_array(computed_normals, Some(0), &mut normal);
        Cartesian3::from_array(computed_lefts, Some(0), &mut left);
        for i in 0..half_length {
            Cartesian3::from_array(
                first_end_positions,
                Some((half_length - 1 - i) * 3),
                &mut left_pos,
            );
            Cartesian3::from_array(
                first_end_positions,
                Some((half_length + i) * 3),
                &mut right_pos,
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
            add_normals(
                Some(&mut normals),
                Some(&mut tangents),
                Some(&mut bitangents),
                &normal,
                &left,
                Some(front as usize),
                Some(back as usize),
                vertex_format,
            );

            ll = front / 3;
            lr = ll + 1;
            ul = (back - 2) / 3;
            ur = ul - 1;
            write_index(&mut indices, index, ul as u32);
            index += 1;
            write_index(&mut indices, index, ll as u32);
            index += 1;
            write_index(&mut indices, index, ur as u32);
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

    // --- main body ---
    let mut pos_index = 0usize;
    let mut comp_index = 0usize;
    let mut right_edge = positions[pos_index].clone();
    pos_index += 1;
    let mut left_edge = positions[pos_index].clone();
    pos_index += 1;

    // finalPositions.set(rightEdge, front)  → copy right_edge into front
    for k in 0..right_edge.len() {
        final_positions[front as usize + k] = right_edge[k];
    }
    // finalPositions.set(leftEdge, back - leftEdge.length + 1)
    let left_start = (back as usize).saturating_sub(left_edge.len()).saturating_add(1);
    for k in 0..left_edge.len() {
        final_positions[left_start + k] = left_edge[k];
    }

    Cartesian3::from_array(computed_lefts, Some(comp_index), &mut left);
    length = left_edge.len() - 3;

    let mut scratch1 = Cartesian3::default();
    let mut scratch2 = Cartesian3::default();

    for i in (0..length).step_by(3) {
        // rightNormal = ellipsoid.geodeticSurfaceNormal(rightEdge[i])
        Cartesian3::from_array(&right_edge, Some(i), &mut scratch1);
        { let sn1 = scratch1; ellipsoid.geodetic_surface_normal(&sn1, &mut scratch1); }
        // leftNormal = ellipsoid.geodeticSurfaceNormal(leftEdge[length - i])
        Cartesian3::from_array(&left_edge, Some(length - i), &mut scratch2);
        { let sn2 = scratch2; ellipsoid.geodetic_surface_normal(&sn2, &mut scratch2); }
        // normal = normalize(add(rightNormal, leftNormal))
        let sum = Cartesian3::add_new(&scratch1, &scratch2);
        normal = Cartesian3::normalize_new(&sum);
        add_normals(
            Some(&mut normals),
            Some(&mut tangents),
            Some(&mut bitangents),
            &normal,
            &left,
            Some(front as usize),
            Some(back as usize),
            vertex_format,
        );

        ll = front / 3;
        lr = ll + 1;
        ul = (back - 2) / 3;
        ur = ul - 1;
        write_index(&mut indices, index, ul as u32);
        index += 1;
        write_index(&mut indices, index, ll as u32);
        index += 1;
        write_index(&mut indices, index, ur as u32);
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

    // rightNormal / leftNormal at the last position of this segment
    Cartesian3::from_array(&right_edge, Some(length), &mut scratch1);
    { let sn1 = scratch1; ellipsoid.geodetic_surface_normal(&sn1, &mut scratch1); }
    Cartesian3::from_array(&left_edge, Some(length), &mut scratch2);
    { let sn2 = scratch2; ellipsoid.geodetic_surface_normal(&sn2, &mut scratch2); }
    let sum = Cartesian3::add_new(&scratch1, &scratch2);
    normal = Cartesian3::normalize_new(&sum);

    comp_index += 3;

    // --- corners ---
    for corner in corners {
        let l = match corner {
            CorridorCorner::LeftPositions(l) => Some(l),
            CorridorCorner::RightPositions(_) => None,
        };
        let r = match corner {
            CorridorCorner::RightPositions(r) => Some(r),
            CorridorCorner::LeftPositions(_) => None,
        };

        let mut outside_point = Cartesian3::default();
        let mut previous_point = Cartesian3::default();
        let mut next_point = Cartesian3::default();

        Cartesian3::from_array(computed_normals, Some(comp_index), &mut normal);

        if let Some(l) = l {
            add_normals(
                Some(&mut normals),
                Some(&mut tangents),
                Some(&mut bitangents),
                &normal,
                &left,
                None,
                Some(back as usize),
                vertex_format,
            );
            back -= 3;
            // JS: pivot = LR; start = UR; re-compute from current front/back.
            let pivot = front / 3 + 1; // LR
            let start = (back - 2) / 3 - 1; // UR
            for j in 0..l.len() / 3 {
                Cartesian3::from_array(l, Some(j * 3), &mut outside_point);
                write_index(&mut indices, index, pivot as u32);
                index += 1;
                write_index(&mut indices, index, (start - j as isize - 1) as u32);
                index += 1;
                write_index(&mut indices, index, (start - j as isize) as u32);
                index += 1;
                CorridorGeometryLibrary::add_attribute(
                    &mut final_positions,
                    &outside_point,
                    None,
                    Some(back as usize),
                );
                Cartesian3::from_array(
                    &final_positions,
                    Some(((start - j as isize - 1) as usize) * 3),
                    &mut previous_point,
                );
                Cartesian3::from_array(
                    &final_positions,
                    Some(pivot as usize * 3),
                    &mut next_point,
                );
                let diff = Cartesian3::subtract_new(&previous_point, &next_point);
                left = Cartesian3::normalize_new(&diff);
                add_normals(
                    Some(&mut normals),
                    Some(&mut tangents),
                    Some(&mut bitangents),
                    &normal,
                    &left,
                    None,
                    Some(back as usize),
                    vertex_format,
                );
                back -= 3;
            }
            // final normal adjustment after corner
            Cartesian3::from_array(
                &final_positions,
                Some(pivot as usize * 3),
                &mut outside_point,
            );
            Cartesian3::from_array(
                &final_positions,
                Some(start as usize * 3),
                &mut previous_point,
            );
            previous_point = Cartesian3::subtract_new(&previous_point, &outside_point);
            let s_minus_j = start - (l.len() / 3) as isize;
            Cartesian3::from_array(
                &final_positions,
                Some(s_minus_j as usize * 3),
                &mut next_point,
            );
            next_point = Cartesian3::subtract_new(&next_point, &outside_point);
            let sum = Cartesian3::add_new(&previous_point, &next_point);
            left = Cartesian3::normalize_new(&sum);
            add_normals(
                Some(&mut normals),
                Some(&mut tangents),
                Some(&mut bitangents),
                &normal,
                &left,
                Some(front as usize),
                None,
                vertex_format,
            );
            front += 3;
        } else if let Some(r) = r {
            add_normals(
                Some(&mut normals),
                Some(&mut tangents),
                Some(&mut bitangents),
                &normal,
                &left,
                Some(front as usize),
                None,
                vertex_format,
            );
            front += 3;
            let ur_val = (back - 2) / 3 - 1;
            let lr_val = front / 3 - 1;
            let pivot = ur_val;
            let start = lr_val;
            for j in 0..r.len() / 3 {
                Cartesian3::from_array(r, Some(j * 3), &mut outside_point);
                write_index(&mut indices, index, pivot as u32);
                index += 1;
                write_index(&mut indices, index, (start + j as isize) as u32);
                index += 1;
                write_index(&mut indices, index, (start + j as isize + 1) as u32);
                index += 1;
                CorridorGeometryLibrary::add_attribute(
                    &mut final_positions,
                    &outside_point,
                    Some(front as usize),
                    None,
                );
                Cartesian3::from_array(
                    &final_positions,
                    Some(pivot as usize * 3),
                    &mut previous_point,
                );
                Cartesian3::from_array(
                    &final_positions,
                    Some(((start + j as isize) as usize) * 3),
                    &mut next_point,
                );
                let diff = Cartesian3::subtract_new(&previous_point, &next_point);
                left = Cartesian3::normalize_new(&diff);
                add_normals(
                    Some(&mut normals),
                    Some(&mut tangents),
                    Some(&mut bitangents),
                    &normal,
                    &left,
                    Some(front as usize),
                    None,
                    vertex_format,
                );
                front += 3;
            }
            // final normal adjustment
            Cartesian3::from_array(
                &final_positions,
                Some(pivot as usize * 3),
                &mut outside_point,
            );
            let s_plus_j = start + (r.len() / 3) as isize;
            Cartesian3::from_array(
                &final_positions,
                Some(s_plus_j as usize * 3),
                &mut previous_point,
            );
            previous_point = Cartesian3::subtract_new(&previous_point, &outside_point);
            Cartesian3::from_array(
                &final_positions,
                Some(start as usize * 3),
                &mut next_point,
            );
            next_point = Cartesian3::subtract_new(&next_point, &outside_point);
            let sum = Cartesian3::add_new(&next_point, &previous_point);
            let neg = Cartesian3::negate_new(&sum);
            left = Cartesian3::normalize_new(&neg);
            add_normals(
                Some(&mut normals),
                Some(&mut tangents),
                Some(&mut bitangents),
                &normal,
                &left,
                None,
                Some(back as usize),
                vertex_format,
            );
            back -= 3;
        }

        // load next segment
        right_edge = positions[pos_index].clone();
        pos_index += 1;
        left_edge = positions[pos_index].clone();
        pos_index += 1;
        // rightEdge.splice(0, 3) — remove first 3 elements
        right_edge.drain(0..3);
        // leftEdge.splice(leftEdge.length - 3, 3) — remove last 3
        let le_len = left_edge.len();
        left_edge.truncate(le_len - 3);

        for k in 0..right_edge.len() {
            final_positions[front as usize + k] = right_edge[k];
        }
        let lstart = (back as usize).saturating_sub(left_edge.len()).saturating_add(1);
        for k in 0..left_edge.len() {
            final_positions[lstart + k] = left_edge[k];
        }

        length = left_edge.len() - 3;
        comp_index += 3;
        Cartesian3::from_array(computed_lefts, Some(comp_index), &mut left);

        for j in (0..left_edge.len()).step_by(3) {
            Cartesian3::from_array(&right_edge, Some(j), &mut scratch1);
            { let sn1 = scratch1; ellipsoid.geodetic_surface_normal(&sn1, &mut scratch1); }
            Cartesian3::from_array(&left_edge, Some(length - j), &mut scratch2);
            { let sn2 = scratch2; ellipsoid.geodetic_surface_normal(&sn2, &mut scratch2); }
            let sum = Cartesian3::add_new(&scratch1, &scratch2);
            normal = Cartesian3::normalize_new(&sum);
            add_normals(
                Some(&mut normals),
                Some(&mut tangents),
                Some(&mut bitangents),
                &normal,
                &left,
                Some(front as usize),
                Some(back as usize),
                vertex_format,
            );

            lr = front / 3;
            ll = lr - 1;
            ur = (back - 2) / 3;
            ul = ur + 1;
            write_index(&mut indices, index, ul as u32);
            index += 1;
            write_index(&mut indices, index, ll as u32);
            index += 1;
            write_index(&mut indices, index, ur as u32);
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
    }

    // last normal
    Cartesian3::from_array(
        computed_normals,
        Some(computed_normals.len() - 3),
        &mut normal,
    );
    add_normals(
        Some(&mut normals),
        Some(&mut tangents),
        Some(&mut bitangents),
        &normal,
        &left,
        Some(front as usize),
        Some(back as usize),
        vertex_format,
    );

    // --- end cap (rounded end) ---
    if add_end_positions {
        front += 3;
        back -= 3;
        let ep = end_positions.unwrap();
        let last_end_positions = &ep[1];
        for i in 0..half_length {
            Cartesian3::from_array(
                last_end_positions,
                Some((end_position_length - i - 1) * 3),
                &mut left_pos,
            );
            Cartesian3::from_array(last_end_positions, Some(i * 3), &mut right_pos);
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
            add_normals(
                Some(&mut normals),
                Some(&mut tangents),
                Some(&mut bitangents),
                &normal,
                &left,
                Some(front as usize),
                Some(back as usize),
                vertex_format,
            );

            lr = front / 3;
            ll = lr - 1;
            ur = (back - 2) / 3;
            ul = ur + 1;
            write_index(&mut indices, index, ul as u32);
            index += 1;
            write_index(&mut indices, index, ll as u32);
            index += 1;
            write_index(&mut indices, index, ur as u32);
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

    // --- build attributes map ---
    let mut attributes = HashMap::new();
    attributes.insert(
        "position".to_string(),
        GeometryAttribute::new(ComponentDatatype::Double, 3, false, final_positions),
    );

    // ST coordinates
    if vertex_format.st {
        let mut st = vec![0.0f64; (size / 3) * 2];
        let mut st_index = 0usize;
        if add_end_positions {
            let lc3 = left_count / 3;
            let rc3 = right_count / 3;
            let theta = std::f64::consts::PI / (end_position_length + 1) as f64;
            let left_st = 1.0 / (lc3 - end_position_length + 1) as f64;
            let right_st = 1.0 / (rc3 - end_position_length + 1) as f64;
            let half_end_pos = end_position_length / 2;
            for i in half_end_pos + 1..end_position_length + 1 {
                let a = CesiumMath::PI_OVER_TWO + theta * i as f64;
                st[st_index] = right_st * (1.0 + a.cos());
                st[st_index + 1] = 0.5 * (1.0 + a.sin());
                st_index += 2;
            }
            for i in 1..rc3 - end_position_length + 1 {
                st[st_index] = i as f64 * right_st;
                st[st_index + 1] = 0.0;
                st_index += 2;
            }
            for i in (half_end_pos + 1..end_position_length + 1).rev() {
                let a = CesiumMath::PI_OVER_TWO - i as f64 * theta;
                st[st_index] = 1.0 - right_st * (1.0 + a.cos());
                st[st_index + 1] = 0.5 * (1.0 + a.sin());
                st_index += 2;
            }
            for i in (1..half_end_pos + 1).rev() {
                let a = CesiumMath::PI_OVER_TWO - theta * i as f64;
                st[st_index] = 1.0 - left_st * (1.0 + a.cos());
                st[st_index + 1] = 0.5 * (1.0 + a.sin());
                st_index += 2;
            }
            for i in (1..lc3 - end_position_length + 1).rev() {
                st[st_index] = i as f64 * left_st;
                st[st_index + 1] = 1.0;
                st_index += 2;
            }
            for i in 1..half_end_pos + 1 {
                let a = CesiumMath::PI_OVER_TWO + theta * i as f64;
                st[st_index] = left_st * (1.0 + a.cos());
                st[st_index + 1] = 0.5 * (1.0 + a.sin());
                st_index += 2;
            }
        } else {
            let lc3 = left_count / 3;
            let rc3 = right_count / 3;
            let left_st = 1.0 / (lc3 - 1) as f64;
            let right_st = 1.0 / (rc3 - 1) as f64;
            for i in 0..rc3 {
                st[st_index] = i as f64 * right_st;
                st[st_index + 1] = 0.0;
                st_index += 2;
            }
            for i in (1..lc3 + 1).rev() {
                st[st_index] = (i - 1) as f64 * left_st;
                st[st_index + 1] = 1.0;
                st_index += 2;
            }
        }
        attributes.insert(
            "st".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 2, false, st),
        );
    }

    if vertex_format.normal {
        attributes.insert(
            "normal".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 3, false, normals),
        );
    }
    if vertex_format.tangent {
        attributes.insert(
            "tangent".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 3, false, tangents),
        );
    }
    if vertex_format.bitangent {
        attributes.insert(
            "bitangent".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 3, false, bitangents),
        );
    }

    CombineResult {
        attributes,
        indices,
    }
}

struct CombineResult {
    attributes: HashMap<String, GeometryAttribute>,
    indices: IndexStorage,
}

/// Port of `addWallPositions` — duplicates internal positions for wall geometry.
fn add_wall_positions(positions: &[f64], start: usize, wall_positions: &mut [f64]) -> usize {
    let mut index = start;
    wall_positions[index] = positions[0];
    wall_positions[index + 1] = positions[1];
    wall_positions[index + 2] = positions[2];
    index += 3;
    let mut i = 3;
    while i < positions.len() {
        let x = positions[i];
        let y = positions[i + 1];
        let z = positions[i + 2];
        wall_positions[index] = x;
        wall_positions[index + 1] = y;
        wall_positions[index + 2] = z;
        index += 3;
        wall_positions[index] = x;
        wall_positions[index + 1] = y;
        wall_positions[index + 2] = z;
        index += 3;
        i += 3;
    }
    wall_positions[index] = positions[0];
    wall_positions[index + 1] = positions[1];
    wall_positions[index + 2] = positions[2];
    index + 3
}

/// Port of `extrudedAttributes` — expands top-face normals/tangents/bitangents/st
/// to include bottom face and walls.
fn extruded_attributes(
    attributes: &mut HashMap<String, GeometryAttribute>,
    vertex_format: &VertexFormat,
) {
    if !vertex_format.normal
        && !vertex_format.tangent
        && !vertex_format.bitangent
        && !vertex_format.st
    {
        return;
    }

    let positions = attributes.get("position").unwrap().values.clone();
    let top_normals = if vertex_format.normal || vertex_format.bitangent {
        attributes.get("normal").map(|a| a.values.clone())
    } else {
        None
    };
    let top_bitangents = if vertex_format.bitangent {
        attributes.get("bitangent").map(|a| a.values.clone())
    } else {
        None
    };

    let size = positions.len() / 18;
    let three_size = size * 3;
    let two_size = size * 2;
    let six_size = three_size * 2;

    if vertex_format.normal || vertex_format.bitangent || vertex_format.tangent {
        let mut normals: Vec<f64> = if vertex_format.normal {
            vec![0.0f64; three_size * 6]
        } else {
            Vec::new()
        };
        let mut tangents: Vec<f64> = if vertex_format.tangent {
            vec![0.0f64; three_size * 6]
        } else {
            Vec::new()
        };
        let mut bitangents: Vec<f64> = if vertex_format.bitangent {
            vec![0.0f64; three_size * 6]
        } else {
            Vec::new()
        };

        let mut top_position = Cartesian3::default();
        let mut bottom_position = Cartesian3::default();
        let mut previous_position = Cartesian3::default();
        let mut normal = Cartesian3::default();
        let mut tangent = Cartesian3::default();
        let mut bitangent = Cartesian3::default();

        let mut attr_index = six_size;
        for i in (0..three_size).step_by(3) {
            let attr_index_offset = attr_index + six_size;
            Cartesian3::from_array(&positions, Some(i), &mut top_position);
            Cartesian3::from_array(
                &positions,
                Some(i + three_size),
                &mut bottom_position,
            );
            Cartesian3::from_array(
                &positions,
                Some((i + 3) % three_size),
                &mut previous_position,
            );
            bottom_position = Cartesian3::subtract_new(&bottom_position, &top_position);
            previous_position = Cartesian3::subtract_new(&previous_position, &top_position);
            let cross = Cartesian3::cross_new(&bottom_position, &previous_position);
            normal = Cartesian3::normalize_new(&cross);

            if vertex_format.normal {
                CorridorGeometryLibrary::add_attribute(&mut normals, &normal, Some(attr_index_offset), None);
                CorridorGeometryLibrary::add_attribute(&mut normals, &normal, Some(attr_index_offset + 3), None);
                CorridorGeometryLibrary::add_attribute(&mut normals, &normal, Some(attr_index), None);
                CorridorGeometryLibrary::add_attribute(&mut normals, &normal, Some(attr_index + 3), None);
            }

            if vertex_format.tangent || vertex_format.bitangent {
                if let Some(tn) = &top_normals {
                    Cartesian3::from_array(tn, Some(i), &mut bitangent);
                }
                if vertex_format.bitangent {
                    CorridorGeometryLibrary::add_attribute(&mut bitangents, &bitangent, Some(attr_index_offset), None);
                    CorridorGeometryLibrary::add_attribute(&mut bitangents, &bitangent, Some(attr_index_offset + 3), None);
                    CorridorGeometryLibrary::add_attribute(&mut bitangents, &bitangent, Some(attr_index), None);
                    CorridorGeometryLibrary::add_attribute(&mut bitangents, &bitangent, Some(attr_index + 3), None);
                }
                if vertex_format.tangent {
                    let cross = Cartesian3::cross_new(&bitangent, &normal);
                    tangent = Cartesian3::normalize_new(&cross);
                    CorridorGeometryLibrary::add_attribute(&mut tangents, &tangent, Some(attr_index_offset), None);
                    CorridorGeometryLibrary::add_attribute(&mut tangents, &tangent, Some(attr_index_offset + 3), None);
                    CorridorGeometryLibrary::add_attribute(&mut tangents, &tangent, Some(attr_index), None);
                    CorridorGeometryLibrary::add_attribute(&mut tangents, &tangent, Some(attr_index + 3), None);
                }
            }
            attr_index += 6;
        }

        if vertex_format.normal {
            if let Some(tn) = &top_normals {
                for i in 0..three_size {
                    normals[i] = tn[i];
                }
                // bottom normals = -top
                for i in 0..three_size {
                    normals[i + three_size] = -tn[i];
                }
            }
            attributes.insert(
                "normal".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 3, false, normals),
            );
        }

        if vertex_format.bitangent {
            if let Some(tb) = &top_bitangents {
                for i in 0..three_size {
                    bitangents[i] = tb[i];
                }
                for i in 0..three_size {
                    bitangents[i + three_size] = tb[i];
                }
            }
            attributes.insert(
                "bitangent".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 3, false, bitangents),
            );
        }

        if vertex_format.tangent {
            let top_tangents = attributes.get("tangent").unwrap().values.clone();
            for i in 0..three_size {
                tangents[i] = top_tangents[i];
            }
            for i in 0..three_size {
                tangents[i + three_size] = top_tangents[i];
            }
            attributes.insert(
                "tangent".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 3, false, tangents),
            );
        }
    }

    if vertex_format.st {
        let top_st = attributes.get("st").unwrap().values.clone();
        let mut st = vec![0.0f64; two_size * 6];
        // top
        for i in 0..two_size {
            st[i] = top_st[i];
        }
        // bottom
        for i in 0..two_size {
            st[i + two_size] = top_st[i];
        }
        let mut index = two_size * 2;
        for _j in 0..2 {
            st[index] = top_st[0];
            st[index + 1] = top_st[1];
            index += 2;
            let mut i = 2;
            while i < two_size {
                let s = top_st[i];
                let t = top_st[i + 1];
                st[index] = s;
                st[index + 1] = t;
                index += 2;
                st[index] = s;
                st[index + 1] = t;
                index += 2;
                i += 2;
            }
            st[index] = top_st[0];
            st[index + 1] = top_st[1];
            index += 2;
        }
        attributes.insert(
            "st".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 2, false, st),
        );
    }
}

/// Port of `computePositionsExtruded`.
fn compute_positions_extruded(
    params: &CorridorComputePositionsParams,
    vertex_format: &VertexFormat,
    height: f64,
    extruded_height: f64,
    shadow_volume: bool,
    offset_attribute: Option<GeometryOffsetAttribute>,
    ellipsoid: &Ellipsoid,
) -> CombineResult {
    let top_vertex_format = VertexFormat {
        position: vertex_format.position,
        normal: vertex_format.normal || vertex_format.bitangent || shadow_volume,
        tangent: vertex_format.tangent,
        bitangent: vertex_format.normal || vertex_format.bitangent,
        st: vertex_format.st,
        color: false,
    };

    let computed_positions = CorridorGeometryLibrary::compute_positions(params);
    let result = combine(&computed_positions, &top_vertex_format, ellipsoid);
    let mut attributes = result.attributes;
    let indices = result.indices;

    let mut positions = attributes.get("position").unwrap().values.clone();
    let length = positions.len();
    let mut new_positions = vec![0.0f64; length * 6];
    let mut extruded_positions = positions.clone();
    let mut wall_positions = vec![0.0f64; length * 4];

    PolygonPipeline::scale_to_geodetic_height(
        Some(&mut positions),
        Some(height),
        Some(ellipsoid),
        Some(true),
    );
    let _ = add_wall_positions(&positions, 0, &mut wall_positions);
    PolygonPipeline::scale_to_geodetic_height(
        Some(&mut extruded_positions),
        Some(extruded_height),
        Some(ellipsoid),
        Some(true),
    );
    let _ = add_wall_positions(&extruded_positions, length * 2, &mut wall_positions);

    new_positions[..length].copy_from_slice(&positions);
    new_positions[length..length * 2].copy_from_slice(&extruded_positions);
    new_positions[length * 2..].copy_from_slice(&wall_positions);
    attributes.get_mut("position").unwrap().values = new_positions.clone();

    extruded_attributes(&mut attributes, vertex_format);

    let size = length / 3;

    // shadow volume
    if shadow_volume {
        let top_normals = attributes.get("normal").unwrap().values.clone();
        let tn_length = top_normals.len();
        let mut extrude_normals = vec![0.0f64; tn_length * 6];
        for i in 0..tn_length {
            extrude_normals[i] = -top_normals[i];
        }
        // bottom face
        for i in 0..tn_length {
            extrude_normals[i + tn_length] = -top_normals[i];
        }
        // bottom wall
        let _ = add_wall_positions(&extrude_normals[..tn_length].to_vec(), tn_length * 4, &mut extrude_normals);
        attributes.insert(
            "extrudeDirection".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 3, false, extrude_normals),
        );
        if !vertex_format.normal {
            attributes.remove("normal");
        }
    }

    // offset attribute
    if let Some(offset_attr) = offset_attribute {
        let apply_offset: Vec<f64> = if offset_attr == GeometryOffsetAttribute::Top {
            let mut v = vec![0.0f64; size * 6];
            for i in 0..size {
                v[i] = 1.0; // top face
            }
            for i in size * 2..size * 4 {
                v[i] = 1.0; // top wall
            }
            v
        } else {
            let offset_value = if offset_attr == GeometryOffsetAttribute::None { 0 } else { 1 };
            vec![offset_value as f64; size * 6]
        };
        attributes.insert(
            "applyOffset".to_string(),
            GeometryAttribute::new(ComponentDatatype::UnsignedByte, 1, false, apply_offset),
        );
    }

    // indices
    let i_length = indices.len();
    let two_size = size + size;
    let mut new_indices = IndexDatatype::create_typed_array(
        new_positions.len() / 3,
        i_length * 2 + two_size * 3,
    );
    for i in 0..i_length {
        write_index(&mut new_indices, i, read_index(&indices, i));
    }
    let mut index = i_length;
    // bottom indices (mirrored)
    for i in (0..i_length).step_by(3) {
        let v0 = read_index(&indices, i);
        let v1 = read_index(&indices, i + 1);
        let v2 = read_index(&indices, i + 2);
        write_index(&mut new_indices, index, v2 + size as u32);
        index += 1;
        write_index(&mut new_indices, index, v1 + size as u32);
        index += 1;
        write_index(&mut new_indices, index, v0 + size as u32);
        index += 1;
    }
    // wall indices
    for i in (0..two_size).step_by(2) {
        let ul = i + two_size;
        let ll = ul + two_size;
        let ur = ul + 1;
        let lr = ll + 1;
        write_index(&mut new_indices, index, ul as u32);
        index += 1;
        write_index(&mut new_indices, index, ll as u32);
        index += 1;
        write_index(&mut new_indices, index, ur as u32);
        index += 1;
        write_index(&mut new_indices, index, ur as u32);
        index += 1;
        write_index(&mut new_indices, index, ll as u32);
        index += 1;
        write_index(&mut new_indices, index, lr as u32);
        index += 1;
    }

    CombineResult {
        attributes,
        indices: new_indices,
    }
}

/// Computes the geometric representation of a corridor, including vertices,
/// indices, and a bounding sphere.
///
/// Port of `CorridorGeometry.createGeometry`.
pub fn create_geometry(corridor_geometry: &CorridorGeometry) -> Option<Geometry> {
    let mut positions = corridor_geometry.positions.clone();
    let width = corridor_geometry.width;
    let ellipsoid = &corridor_geometry.ellipsoid;

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

    let height = corridor_geometry.height;
    let extruded_height = corridor_geometry.extruded_height;
    let extrude = !CesiumMath::equals_epsilon(
        height,
        extruded_height,
        Some(0.0),
        Some(CesiumMath::EPSILON2),
    );

    let vertex_format = &corridor_geometry.vertex_format;
    let params = CorridorComputePositionsParams {
        granularity: corridor_geometry.granularity,
        positions: clean_positions.clone(),
        ellipsoid: ellipsoid.clone(),
        width,
        corner_type: corridor_geometry.corner_type,
        save_attributes: true,
    };

    let result = if extrude {
        compute_positions_extruded(
            &params,
            vertex_format,
            height,
            extruded_height,
            corridor_geometry.shadow_volume,
            corridor_geometry.offset_attribute,
            ellipsoid,
        )
    } else {
        let computed_positions = CorridorGeometryLibrary::compute_positions(&params);
        let mut r = combine(&computed_positions, vertex_format, ellipsoid);
        // scale positions to height
        let mut pos_vals = r.attributes.get_mut("position").unwrap().values.clone();
        PolygonPipeline::scale_to_geodetic_height(
            Some(&mut pos_vals),
            Some(height),
            Some(ellipsoid),
            Some(true),
        );
        r.attributes.get_mut("position").unwrap().values = pos_vals;

        if let Some(offset_attr) = corridor_geometry.offset_attribute {
            let offset_value = if offset_attr == GeometryOffsetAttribute::None { 0 } else { 1 };
            let length = r.attributes["position"].values.len();
            let apply_offset = vec![offset_value as f64; length / 3];
            r.attributes.insert(
                "applyOffset".to_string(),
                GeometryAttribute::new(
                    ComponentDatatype::UnsignedByte,
                    1,
                    false,
                    apply_offset,
                ),
            );
        }
        r
    };

    let attributes = &result.attributes;
    let bounding_sphere = BoundingSphere::from_vertices(
        &attributes["position"].values,
        None,
        Some(3),
        None,
    );

    let mut final_attributes = result.attributes;
    if !vertex_format.position {
        final_attributes.remove("position");
    }

    let offset_attr_str = corridor_geometry
        .offset_attribute
        .map(|_| "applyOffset".to_string());

    Some(Geometry::with_all(
        final_attributes,
        Some(result.indices),
        Some(PrimitiveType::Triangles),
        Some(bounding_sphere),
        GeometryType::None,
        None,
        offset_attr_str,
    ))
}
