//! Ported from `packages/engine/Source/Core/EllipseOutlineGeometry.js`.
//!
//! A description of the outline of an ellipse on an ellipsoid.

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::component_datatype::ComponentDatatype;
use crate::ellipse_geometry_library::{
    EllipseGeometryLibrary, EllipseGeometryOptions, raise_positions_to_height,
};
use crate::ellipsoid::Ellipsoid;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_offset_attribute::GeometryOffsetAttribute;
use crate::index_datatype::IndexDatatype;
use crate::math::CesiumMath;
use crate::primitive_type::PrimitiveType;

/// A description of the outline of an ellipse on an ellipsoid.
#[derive(Debug, Clone)]
pub struct EllipseOutlineGeometry {
    center: Cartesian3,
    semi_major_axis: f64,
    semi_minor_axis: f64,
    ellipsoid: Ellipsoid,
    rotation: f64,
    height: f64,
    granularity: f64,
    extruded_height: f64,
    number_of_vertical_lines: usize,
    offset_attribute: Option<GeometryOffsetAttribute>,
}

impl EllipseOutlineGeometry {
    /// Creates a new `EllipseOutlineGeometry`.
    pub fn new(
        center: Cartesian3,
        semi_major_axis: f64,
        semi_minor_axis: f64,
        ellipsoid: Option<Ellipsoid>,
        height: Option<f64>,
        extruded_height: Option<f64>,
        rotation: Option<f64>,
        granularity: Option<f64>,
        number_of_vertical_lines: Option<usize>,
        offset_attribute: Option<GeometryOffsetAttribute>,
    ) -> Self {
        let height = height.unwrap_or(0.0);
        let extruded_height = extruded_height.unwrap_or(height);
        Self {
            center,
            semi_major_axis,
            semi_minor_axis,
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84),
            rotation: rotation.unwrap_or(0.0),
            height: extruded_height.max(height),
            granularity: granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE),
            extruded_height: extruded_height.min(height),
            number_of_vertical_lines: number_of_vertical_lines.unwrap_or(16).max(0),
            offset_attribute,
        }
    }

    /// The number of elements used to pack the object into an array.
    pub const PACKED_LENGTH: usize =
        Cartesian3::PACKED_LENGTH + Ellipsoid::PACKED_LENGTH + 8;
}

/// Computes the geometric representation of an outline of an ellipse.
///
/// Port of `EllipseOutlineGeometry.createGeometry`.
pub fn create_geometry(ellipse_geometry: &EllipseOutlineGeometry) -> Option<Geometry> {
    if ellipse_geometry.semi_major_axis <= 0.0 || ellipse_geometry.semi_minor_axis <= 0.0 {
        return None;
    }

    let height = ellipse_geometry.height;
    let extruded_height = ellipse_geometry.extruded_height;
    let extrude = !CesiumMath::equals_epsilon(height, extruded_height, Some(0.0), Some(CesiumMath::EPSILON2));

    let mut center = ellipse_geometry.center;
    ellipse_geometry.ellipsoid.scale_to_geodetic_surface(&ellipse_geometry.center, &mut center);

    let options = EllipseOptions {
        center,
        semi_major_axis: ellipse_geometry.semi_major_axis,
        semi_minor_axis: ellipse_geometry.semi_minor_axis,
        rotation: ellipse_geometry.rotation,
        granularity: ellipse_geometry.granularity,
        ellipsoid: &ellipse_geometry.ellipsoid,
        height,
        extruded_height,
    };

    let result = if extrude {
        compute_extruded_ellipse(&options, ellipse_geometry.number_of_vertical_lines, ellipse_geometry.offset_attribute)
    } else {
        let mut r = compute_ellipse(&options);
        if let Some(offset_attribute) = ellipse_geometry.offset_attribute {
            let length = r.position_attribute_values.len();
            let offset_value = if offset_attribute == GeometryOffsetAttribute::None { 0 } else { 1 };
            let apply_offset = vec![offset_value as f64; length / 3];
            r.attributes.insert(
                "applyOffset".to_string(),
                GeometryAttribute::new(ComponentDatatype::UnsignedByte, 1, false, apply_offset),
            );
        }
        r
    };

    let bounding_sphere = result.bounding_sphere;
    let attributes = result.attributes;
    let indices = result.indices;

    Some(Geometry::with_all(
        attributes,
        Some(indices),
        Some(PrimitiveType::Lines),
        Some(bounding_sphere),
        crate::geometry_type::GeometryType::None,
        None,
        ellipse_geometry.offset_attribute.map(|_| "applyOffset".to_string()),
    ))
}

struct EllipseOptions<'a> {
    center: Cartesian3,
    semi_major_axis: f64,
    semi_minor_axis: f64,
    rotation: f64,
    granularity: f64,
    ellipsoid: &'a Ellipsoid,
    height: f64,
    extruded_height: f64,
}

struct ComputeResult {
    bounding_sphere: BoundingSphere,
    attributes: HashMap<String, GeometryAttribute>,
    indices: crate::index_datatype::IndexStorage,
    position_attribute_values: Vec<f64>,
}

fn compute_ellipse(options: &EllipseOptions) -> ComputeResult {
    let ellipsoid = options.ellipsoid;
    let mut normal = Cartesian3::ZERO;
    ellipsoid.geodetic_surface_normal(&options.center, &mut normal);
    let scaled = Cartesian3::multiply_by_scalar_new(&normal, options.height);
    let bounding_sphere_center = Cartesian3::add_new(&options.center, &scaled);
    let bounding_sphere = BoundingSphere::new(bounding_sphere_center, options.semi_major_axis);

    let ellipse_options = EllipseGeometryOptions {
        semi_minor_axis: options.semi_minor_axis,
        semi_major_axis: options.semi_major_axis,
        rotation: options.rotation,
        center: options.center,
        granularity: options.granularity,
    };
    let positions = EllipseGeometryLibrary::compute_ellipse_positions(&ellipse_options, false, true)
        .outer_positions
        .unwrap_or_default();

    let raised_positions = raise_positions_to_height(
        &positions,
        ellipsoid,
        options.height,
        options.extruded_height,
        false,
    );

    let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();
    attributes.insert(
        "position".to_string(),
        GeometryAttribute::new(ComponentDatatype::Double, 3, false, raised_positions.clone()),
    );

    let length = positions.len() / 3;
    let mut indices = IndexDatatype::create_typed_array(length, length * 2);
    let mut index = 0usize;
    for i in 0..length {
        write_index(&mut indices, index, i as u32);
        index += 1;
        write_index(&mut indices, index, ((i + 1) % length) as u32);
        index += 1;
    }

    ComputeResult {
        bounding_sphere,
        attributes,
        indices,
        position_attribute_values: raised_positions,
    }
}

fn compute_extruded_ellipse(
    options: &EllipseOptions,
    number_of_vertical_lines: usize,
    offset_attribute: Option<GeometryOffsetAttribute>,
) -> ComputeResult {
    let ellipsoid = options.ellipsoid;
    let center = &options.center;
    let semi_major_axis = options.semi_major_axis;

    let mut normal = Cartesian3::ZERO;
    ellipsoid.geodetic_surface_normal(center, &mut normal);
    let mut scaled_normal = Cartesian3::multiply_by_scalar_new(&normal, options.height);
    let top_center = Cartesian3::add_new(center, &scaled_normal);
    let top_bounding_sphere = BoundingSphere::new(top_center, semi_major_axis);

    scaled_normal = Cartesian3::multiply_by_scalar_new(&normal, options.extruded_height);
    let bottom_center = Cartesian3::add_new(center, &scaled_normal);
    let bottom_bounding_sphere = BoundingSphere::new(bottom_center, semi_major_axis);

    let ellipse_options = EllipseGeometryOptions {
        semi_minor_axis: options.semi_minor_axis,
        semi_major_axis: options.semi_major_axis,
        rotation: options.rotation,
        center: *center,
        granularity: options.granularity,
    };
    let positions = EllipseGeometryLibrary::compute_ellipse_positions(&ellipse_options, false, true)
        .outer_positions
        .unwrap_or_default();

    let raised_positions = raise_positions_to_height(
        &positions,
        ellipsoid,
        options.height,
        options.extruded_height,
        true,
    );

    let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();
    attributes.insert(
        "position".to_string(),
        GeometryAttribute::new(ComponentDatatype::Double, 3, false, raised_positions.clone()),
    );

    let positions = raised_positions;
    let bounding_sphere =
        BoundingSphere::union(&top_bounding_sphere, &bottom_bounding_sphere, None);

    let mut length = positions.len() / 3;

    if let Some(offset_attr) = offset_attribute {
        let apply_offset = if offset_attr == GeometryOffsetAttribute::Top {
            let mut v = vec![0.0f64; length];
            for i in 0..length / 2 {
                v[i] = 1.0;
            }
            v
        } else {
            let offset_value = if offset_attr == GeometryOffsetAttribute::None { 0 } else { 1 };
            vec![offset_value as f64; length]
        };
        attributes.insert(
            "applyOffset".to_string(),
            GeometryAttribute::new(ComponentDatatype::UnsignedByte, 1, false, apply_offset),
        );
    }

    let mut num_vertical_lines = number_of_vertical_lines;
    num_vertical_lines = CesiumMath::clamp(num_vertical_lines as f64, 0.0, length as f64 / 2.0) as usize;

    let mut indices = IndexDatatype::create_typed_array(length, length * 2 + num_vertical_lines * 2);

    length /= 2;
    let mut index = 0usize;
    for i in 0..length {
        write_index(&mut indices, index, i as u32);
        index += 1;
        write_index(&mut indices, index, ((i + 1) % length) as u32);
        index += 1;
        write_index(&mut indices, index, (i + length) as u32);
        index += 1;
        write_index(&mut indices, index, (((i + 1) % length) + length) as u32);
        index += 1;
    }

    if num_vertical_lines > 0 {
        let num_side_lines = num_vertical_lines.min(length);
        let num_side = (length as f64 / num_side_lines as f64).round() as usize;
        let max_i = (num_side * num_vertical_lines).min(length);
        let mut i = 0;
        while i < max_i {
            write_index(&mut indices, index, i as u32);
            index += 1;
            write_index(&mut indices, index, (i + length) as u32);
            index += 1;
            i += num_side;
        }
    }

    ComputeResult {
        bounding_sphere,
        attributes,
        indices,
        position_attribute_values: positions,
    }
}

fn write_index(storage: &mut crate::index_datatype::IndexStorage, index: usize, value: u32) {
    use crate::index_datatype::IndexStorage;
    match storage {
        IndexStorage::U16(v) => v[index] = value as u16,
        IndexStorage::U32(v) => v[index] = value,
    }
}
