//! Ported from `packages/engine/Source/Core/EllipseGeometry.js`.
//!
//! A description of an ellipse on an ellipsoid. Ellipse geometry can be
//! rendered with both `Primitive` and `GroundPrimitive`.
//!
//! DEVIATION: JS `computeExtrudedEllipse` uses
//! `GeometryPipeline.combineInstances` to merge top/bottom and wall
//! geometry. The Rust port merges attributes and indices manually.
//!
//! DEVIATION: JS `raisePositionsToHeight` takes an `options` object;
//! the Rust `raise_positions_to_height` takes individual parameters.

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::component_datatype::ComponentDatatype;
use crate::ellipsoid::Ellipsoid;
use crate::ellipse_geometry_library::{
    raise_positions_to_height, EllipseGeometryLibrary, EllipseGeometryOptions,
};
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_offset_attribute::GeometryOffsetAttribute;
use crate::geometry_type::GeometryType;
use crate::geographic_projection::GeographicProjection;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::math::CesiumMath;
use crate::matrix3::Matrix3;
use crate::primitive_type::PrimitiveType;
use crate::quaternion::Quaternion;
use crate::vertex_format::VertexFormat;

/// A description of an ellipse on an ellipsoid.
#[derive(Debug, Clone)]
pub struct EllipseGeometry {
    center: Cartesian3,
    semi_major_axis: f64,
    semi_minor_axis: f64,
    ellipsoid: Ellipsoid,
    rotation: f64,
    st_rotation: f64,
    height: f64,
    extruded_height: f64,
    granularity: f64,
    vertex_format: VertexFormat,
    shadow_volume: bool,
    offset_attribute: Option<GeometryOffsetAttribute>,
}

impl EllipseGeometry {
    /// Creates a new `EllipseGeometry`.
    pub fn new(
        center: Cartesian3,
        semi_major_axis: f64,
        semi_minor_axis: f64,
        ellipsoid: Option<Ellipsoid>,
        rotation: Option<f64>,
        st_rotation: Option<f64>,
        height: Option<f64>,
        extruded_height: Option<f64>,
        granularity: Option<f64>,
        vertex_format: Option<VertexFormat>,
        shadow_volume: Option<bool>,
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
            st_rotation: st_rotation.unwrap_or(0.0),
            height: height.max(extruded_height),
            extruded_height: height.min(extruded_height),
            granularity: granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE),
            vertex_format: vertex_format.unwrap_or_default(),
            shadow_volume: shadow_volume.unwrap_or(false),
            offset_attribute,
        }
    }
}

/// Options used by internal functions (mirrors JS `options` object).
struct EllipseOptions {
    center: Cartesian3,
    semi_major_axis: f64,
    semi_minor_axis: f64,
    ellipsoid: Ellipsoid,
    rotation: f64,
    height: f64,
    extruded_height: f64,
    granularity: f64,
    vertex_format: VertexFormat,
    st_rotation: f64,
    shadow_volume: bool,
    offset_attribute: Option<GeometryOffsetAttribute>,
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

/// Port of `computeTopBottomAttributes`.
fn compute_top_bottom_attributes(
    positions: &[f64],
    options: &EllipseOptions,
    extrude: bool,
) -> HashMap<String, GeometryAttribute> {
    let vertex_format = &options.vertex_format;
    let center = &options.center;
    let semi_major_axis = options.semi_major_axis;
    let semi_minor_axis = options.semi_minor_axis;
    let ellipsoid = &options.ellipsoid;
    let st_rotation = options.st_rotation;
    let size = if extrude {
        (positions.len() / 3) * 2
    } else {
        positions.len() / 3
    };
    let shadow_volume = options.shadow_volume;

    let mut texture_coordinates: Vec<f64> = if vertex_format.st {
        vec![0.0f64; size * 2]
    } else {
        Vec::new()
    };
    let mut normals: Vec<f64> = if vertex_format.normal {
        vec![0.0f64; size * 3]
    } else {
        Vec::new()
    };
    let mut tangents: Vec<f64> = if vertex_format.tangent {
        vec![0.0f64; size * 3]
    } else {
        Vec::new()
    };
    let mut bitangents: Vec<f64> = if vertex_format.bitangent {
        vec![0.0f64; size * 3]
    } else {
        Vec::new()
    };
    let mut extrude_normals: Vec<f64> = if shadow_volume {
        vec![0.0f64; size * 3]
    } else {
        Vec::new()
    };

    let mut texture_coord_index = 0usize;

    let projection = GeographicProjection::new(Some(ellipsoid.clone()));
    let mut scratch_carto = Cartographic::default();
    ellipsoid.cartesian_to_cartographic(center, &mut scratch_carto);
    let projected_center = projection.project(&scratch_carto);

    let mut geodetic_normal = Cartesian3::default();
    ellipsoid.scale_to_geodetic_surface(center, &mut geodetic_normal);
    { let gn = geodetic_normal; ellipsoid.geodetic_surface_normal(&gn, &mut geodetic_normal); }

    let texture_matrix = if st_rotation != 0.0 {
        let rot = Quaternion::from_axis_angle_new(&geodetic_normal, st_rotation);
        Matrix3::from_quaternion_new(&rot)
    } else {
        Matrix3::IDENTITY
    };
    let tangent_matrix = if st_rotation != 0.0 {
        let rot = Quaternion::from_axis_angle_new(&geodetic_normal, -st_rotation);
        Matrix3::from_quaternion_new(&rot)
    } else {
        Matrix3::IDENTITY
    };

    let mut min_tex_coord = Cartesian2::new(f64::INFINITY, f64::INFINITY);
    let mut max_tex_coord = Cartesian2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);

    let length = positions.len();
    let bottom_offset = if extrude { length } else { 0 };
    let st_offset = (bottom_offset / 3) * 2;

    let mut normal = Cartesian3::default();
    let mut tangent = Cartesian3::default();
    let mut bitangent = Cartesian3::default();

    for i in (0..length).step_by(3) {
        let i1 = i + 1;
        let i2 = i + 2;
        let position = Cartesian3::from_array_new(positions, Some(i));

        if vertex_format.st {
            let rotated_point = Matrix3::multiply_by_vector_new(&texture_matrix, &position);
            ellipsoid.cartesian_to_cartographic(&rotated_point, &mut scratch_carto);
            let projected_point = projection.project(&scratch_carto);
            let pp = Cartesian3::subtract_new(&projected_point, &projected_center);

            let tex_x = (pp.x + semi_major_axis) / (2.0 * semi_major_axis);
            let tex_y = (pp.y + semi_minor_axis) / (2.0 * semi_minor_axis);

            min_tex_coord.x = min_tex_coord.x.min(tex_x);
            min_tex_coord.y = min_tex_coord.y.min(tex_y);
            max_tex_coord.x = max_tex_coord.x.max(tex_x);
            max_tex_coord.y = max_tex_coord.y.max(tex_y);

            if extrude {
                texture_coordinates[texture_coord_index + st_offset] = tex_x;
                texture_coordinates[texture_coord_index + 1 + st_offset] = tex_y;
            }
            texture_coordinates[texture_coord_index] = tex_x;
            texture_coord_index += 1;
            texture_coordinates[texture_coord_index] = tex_y;
            texture_coord_index += 1;
        }

        if vertex_format.normal || vertex_format.tangent || vertex_format.bitangent || shadow_volume {
            { let p = position; ellipsoid.geodetic_surface_normal(&p, &mut normal); }

            if shadow_volume {
                extrude_normals[i + bottom_offset] = -normal.x;
                extrude_normals[i1 + bottom_offset] = -normal.y;
                extrude_normals[i2 + bottom_offset] = -normal.z;
            }

            if vertex_format.normal || vertex_format.tangent || vertex_format.bitangent {
                if vertex_format.tangent || vertex_format.bitangent {
                    let cross = Cartesian3::cross_new(&Cartesian3::UNIT_Z, &normal);
                    tangent = Cartesian3::normalize_new(&cross);
                    let tm = tangent_matrix;
                    let t = tangent;
                    Matrix3::multiply_by_vector(&tm, &t, &mut tangent);
                }
                if vertex_format.normal {
                    normals[i] = normal.x;
                    normals[i1] = normal.y;
                    normals[i2] = normal.z;
                    if extrude {
                        normals[i + bottom_offset] = -normal.x;
                        normals[i1 + bottom_offset] = -normal.y;
                        normals[i2 + bottom_offset] = -normal.z;
                    }
                }
                if vertex_format.tangent {
                    tangents[i] = tangent.x;
                    tangents[i1] = tangent.y;
                    tangents[i2] = tangent.z;
                    if extrude {
                        tangents[i + bottom_offset] = -tangent.x;
                        tangents[i1 + bottom_offset] = -tangent.y;
                        tangents[i2 + bottom_offset] = -tangent.z;
                    }
                }
                if vertex_format.bitangent {
                    let cross = Cartesian3::cross_new(&normal, &tangent);
                    bitangent = Cartesian3::normalize_new(&cross);
                    bitangents[i] = bitangent.x;
                    bitangents[i1] = bitangent.y;
                    bitangents[i2] = bitangent.z;
                    if extrude {
                        bitangents[i + bottom_offset] = bitangent.x;
                        bitangents[i1 + bottom_offset] = bitangent.y;
                        bitangents[i2 + bottom_offset] = bitangent.z;
                    }
                }
            }
        }
    }

    // Normalize texture coordinates
    if vertex_format.st {
        let len = texture_coordinates.len();
        let mut k = 0;
        while k < len {
            let denom_x = max_tex_coord.x - min_tex_coord.x;
            let denom_y = max_tex_coord.y - min_tex_coord.y;
            texture_coordinates[k] = if denom_x.abs() > f64::EPSILON {
                (texture_coordinates[k] - min_tex_coord.x) / denom_x
            } else {
                0.0
            };
            texture_coordinates[k + 1] = if denom_y.abs() > f64::EPSILON {
                (texture_coordinates[k + 1] - min_tex_coord.y) / denom_y
            } else {
                0.0
            };
            k += 2;
        }
    }

    let mut attributes = HashMap::new();

    if vertex_format.position {
        let final_positions = raise_positions_to_height(
            positions,
            ellipsoid,
            options.height,
            options.extruded_height,
            extrude,
        );
        attributes.insert(
            "position".to_string(),
            GeometryAttribute::new(ComponentDatatype::Double, 3, false, final_positions),
        );
    }

    if vertex_format.st {
        attributes.insert(
            "st".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 2, false, texture_coordinates),
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
    if shadow_volume {
        attributes.insert(
            "extrudeDirection".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 3, false, extrude_normals),
        );
    }

    if extrude {
        if let Some(offset_attr) = options.offset_attribute {
            let apply_offset: Vec<f64> = if offset_attr == GeometryOffsetAttribute::Top {
                let mut v = vec![0.0f64; size];
                for i in 0..size / 2 {
                    v[i] = 1.0;
                }
                v
            } else {
                let offset_value = if offset_attr == GeometryOffsetAttribute::None { 0 } else { 1 };
                vec![offset_value as f64; size]
            };
            attributes.insert(
                "applyOffset".to_string(),
                GeometryAttribute::new(ComponentDatatype::UnsignedByte, 1, false, apply_offset),
            );
        }
    }

    attributes
}

/// Port of `topIndices` — generates triangulation indices for the ellipse fill.
fn top_indices(num_pts: usize) -> Vec<u32> {
    let mut indices = vec![0u32; 12 * (num_pts * (num_pts + 1)) - 6];
    let mut indices_index = 0usize;
    let mut prev_index;
    let mut num_interior;
    let mut position_index;

    // Right of north vector
    prev_index = 0;
    position_index = 1;
    for _ in 0..3 {
        indices[indices_index] = position_index as u32;
        position_index += 1;
        indices_index += 1;
        indices[indices_index] = prev_index as u32;
        indices_index += 1;
        indices[indices_index] = position_index as u32;
        indices_index += 1;
    }

    for i in 2..num_pts + 1 {
        position_index = i * (i + 1) - 1;
        prev_index = (i - 1) * i - 1;

        indices[indices_index] = position_index as u32;
        position_index += 1;
        indices_index += 1;
        indices[indices_index] = prev_index as u32;
        indices_index += 1;
        indices[indices_index] = position_index as u32;
        indices_index += 1;

        num_interior = 2 * i;
        for _ in 0..num_interior - 1 {
            indices[indices_index] = position_index as u32;
            indices_index += 1;
            indices[indices_index] = prev_index as u32;
            prev_index += 1;
            indices_index += 1;
            indices[indices_index] = prev_index as u32;
            indices_index += 1;

            indices[indices_index] = position_index as u32;
            position_index += 1;
            indices_index += 1;
            indices[indices_index] = prev_index as u32;
            indices_index += 1;
            indices[indices_index] = position_index as u32;
            indices_index += 1;
        }

        indices[indices_index] = position_index as u32;
        position_index += 1;
        indices_index += 1;
        indices[indices_index] = prev_index as u32;
        indices_index += 1;
        indices[indices_index] = position_index as u32;
        indices_index += 1;
    }

    // Center column
    num_interior = num_pts * 2;
    position_index += 1;
    prev_index += 1;
    for _ in 0..num_interior - 1 {
        indices[indices_index] = position_index as u32;
        indices_index += 1;
        indices[indices_index] = prev_index as u32;
        prev_index += 1;
        indices_index += 1;
        indices[indices_index] = prev_index as u32;
        indices_index += 1;

        indices[indices_index] = position_index as u32;
        position_index += 1;
        indices_index += 1;
        indices[indices_index] = prev_index as u32;
        indices_index += 1;
        indices[indices_index] = position_index as u32;
        indices_index += 1;
    }

    indices[indices_index] = position_index as u32;
    indices_index += 1;
    indices[indices_index] = prev_index as u32;
    prev_index += 1;
    indices_index += 1;
    indices[indices_index] = prev_index as u32;
    indices_index += 1;

    indices[indices_index] = position_index as u32;
    position_index += 1;
    indices_index += 1;
    indices[indices_index] = prev_index as u32;
    prev_index += 1;
    indices_index += 1;
    indices[indices_index] = prev_index as u32;
    indices_index += 1;

    // Left of north vector (reverse)
    prev_index += 1;
    for i in (2..num_pts).rev() {
        indices[indices_index] = prev_index as u32;
        prev_index += 1;
        indices_index += 1;
        indices[indices_index] = prev_index as u32;
        indices_index += 1;
        indices[indices_index] = position_index as u32;
        indices_index += 1;

        num_interior = 2 * i;
        for _ in 0..num_interior - 1 {
            indices[indices_index] = position_index as u32;
            indices_index += 1;
            indices[indices_index] = prev_index as u32;
            prev_index += 1;
            indices_index += 1;
            indices[indices_index] = prev_index as u32;
            indices_index += 1;

            indices[indices_index] = position_index as u32;
            position_index += 1;
            indices_index += 1;
            indices[indices_index] = prev_index as u32;
            indices_index += 1;
            indices[indices_index] = position_index as u32;
            indices_index += 1;
        }

        indices[indices_index] = prev_index as u32;
        prev_index += 1;
        indices_index += 1;
        indices[indices_index] = prev_index as u32;
        prev_index += 1;
        indices_index += 1;
        indices[indices_index] = position_index as u32;
        position_index += 1;
        indices_index += 1;
    }

    for _ in 0..3 {
        indices[indices_index] = prev_index as u32;
        prev_index += 1;
        indices_index += 1;
        indices[indices_index] = prev_index as u32;
        indices_index += 1;
        indices[indices_index] = position_index as u32;
        indices_index += 1;
    }

    indices
}

/// Port of `computeEllipse`.
fn compute_ellipse(options: &EllipseOptions) -> EllipseResult {
    let center = &options.center;
    let ellipsoid = &options.ellipsoid;

    let mut normal = Cartesian3::default();
    ellipsoid.geodetic_surface_normal(center, &mut normal);
    let scaled = Cartesian3::multiply_by_scalar_new(&normal, options.height);
    let bounding_sphere_center = Cartesian3::add_new(center, &scaled);
    let bounding_sphere = BoundingSphere::new(bounding_sphere_center, options.semi_major_axis);

    let lib_options = EllipseGeometryOptions {
        semi_minor_axis: options.semi_minor_axis,
        semi_major_axis: options.semi_major_axis,
        rotation: options.rotation,
        center: options.center.clone(),
        granularity: options.granularity,
    };
    let cep = EllipseGeometryLibrary::compute_ellipse_positions(&lib_options, true, false);
    let positions = cep.positions.unwrap();
    let num_pts = cep.num_pts;
    let attributes = compute_top_bottom_attributes(&positions, options, false);
    let raw_indices = top_indices(num_pts);
    let indices = IndexDatatype::create_typed_array(positions.len() / 3, raw_indices.len());

    EllipseResult {
        bounding_sphere,
        attributes,
        indices,
    }
}

/// Port of `computeWallAttributes`.
fn compute_wall_attributes(
    positions: &[f64],
    options: &EllipseOptions,
) -> HashMap<String, GeometryAttribute> {
    let vertex_format = &options.vertex_format;
    let center = &options.center;
    let semi_major_axis = options.semi_major_axis;
    let semi_minor_axis = options.semi_minor_axis;
    let ellipsoid = &options.ellipsoid;
    let height = options.height;
    let extruded_height = options.extruded_height;
    let st_rotation = options.st_rotation;
    let size = (positions.len() / 3) * 2;

    let mut final_positions = vec![0.0f64; size * 3];
    let mut texture_coordinates: Vec<f64> = if vertex_format.st {
        vec![0.0f64; size * 2]
    } else {
        Vec::new()
    };
    let mut normals: Vec<f64> = if vertex_format.normal {
        vec![0.0f64; size * 3]
    } else {
        Vec::new()
    };
    let mut tangents: Vec<f64> = if vertex_format.tangent {
        vec![0.0f64; size * 3]
    } else {
        Vec::new()
    };
    let mut bitangents: Vec<f64> = if vertex_format.bitangent {
        vec![0.0f64; size * 3]
    } else {
        Vec::new()
    };
    let mut extrude_normals: Vec<f64> = if options.shadow_volume {
        vec![0.0f64; size * 3]
    } else {
        Vec::new()
    };

    let mut texture_coord_index = 0usize;
    let projection = GeographicProjection::new(Some(ellipsoid.clone()));
    let mut scratch_carto = Cartographic::default();
    ellipsoid.cartesian_to_cartographic(center, &mut scratch_carto);
    let projected_center = projection.project(&scratch_carto);

    let mut geodetic_normal = Cartesian3::default();
    ellipsoid.scale_to_geodetic_surface(center, &mut geodetic_normal);
    { let gn = geodetic_normal; ellipsoid.geodetic_surface_normal(&gn, &mut geodetic_normal); }
    let rot = Quaternion::from_axis_angle_new(&geodetic_normal, st_rotation);
    let texture_matrix = Matrix3::from_quaternion_new(&rot);

    let mut min_tex_coord = Cartesian2::new(f64::INFINITY, f64::INFINITY);
    let mut max_tex_coord = Cartesian2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);

    let length = positions.len();
    let st_offset = (length / 3) * 2;

    let mut normal = Cartesian3::default();
    let mut tangent = Cartesian3::default();
    let mut bitangent = Cartesian3::default();

    for i in (0..length).step_by(3) {
        let i1 = i + 1;
        let i2 = i + 2;
        let mut position = Cartesian3::from_array_new(positions, Some(i));

        if vertex_format.st {
            let rotated_point = Matrix3::multiply_by_vector_new(&texture_matrix, &position);
            ellipsoid.cartesian_to_cartographic(&rotated_point, &mut scratch_carto);
            let projected_point = projection.project(&scratch_carto);
            let pp = Cartesian3::subtract_new(&projected_point, &projected_center);

            let tex_x = (pp.x + semi_major_axis) / (2.0 * semi_major_axis);
            let tex_y = (pp.y + semi_minor_axis) / (2.0 * semi_minor_axis);

            min_tex_coord.x = min_tex_coord.x.min(tex_x);
            min_tex_coord.y = min_tex_coord.y.min(tex_y);
            max_tex_coord.x = max_tex_coord.x.max(tex_x);
            max_tex_coord.y = max_tex_coord.y.max(tex_y);

            texture_coordinates[texture_coord_index + st_offset] = tex_x;
            texture_coordinates[texture_coord_index + 1 + st_offset] = tex_y;
            texture_coordinates[texture_coord_index] = tex_x;
            texture_coord_index += 1;
            texture_coordinates[texture_coord_index] = tex_y;
            texture_coord_index += 1;
        }

        // Scale to surface
        { let p = position; ellipsoid.scale_to_geodetic_surface(&p, &mut position); }
        let mut extruded_position = position;
        { let p = position; ellipsoid.geodetic_surface_normal(&p, &mut normal); }

        if options.shadow_volume {
            extrude_normals[i + length] = -normal.x;
            extrude_normals[i1 + length] = -normal.y;
            extrude_normals[i2 + length] = -normal.z;
        }

        let mut scaled_normal = Cartesian3::multiply_by_scalar_new(&normal, height);
        position = Cartesian3::add_new(&position, &scaled_normal);
        scaled_normal = Cartesian3::multiply_by_scalar_new(&normal, extruded_height);
        extruded_position = Cartesian3::add_new(&extruded_position, &scaled_normal);

        if vertex_format.position {
            final_positions[i + length] = extruded_position.x;
            final_positions[i1 + length] = extruded_position.y;
            final_positions[i2 + length] = extruded_position.z;
            final_positions[i] = position.x;
            final_positions[i1] = position.y;
            final_positions[i2] = position.z;
        }

        if vertex_format.normal || vertex_format.tangent || vertex_format.bitangent {
            bitangent = normal;
            let next_idx = (i + 3) % length;
            let mut next = Cartesian3::from_array_new(positions, Some(next_idx));
            next = Cartesian3::subtract_new(&next, &position);
            let bottom = Cartesian3::subtract_new(&extruded_position, &position);
            let cross = Cartesian3::cross_new(&bottom, &next);
            normal = Cartesian3::normalize_new(&cross);

            if vertex_format.normal {
                normals[i] = normal.x;
                normals[i1] = normal.y;
                normals[i2] = normal.z;
                normals[i + length] = normal.x;
                normals[i1 + length] = normal.y;
                normals[i2 + length] = normal.z;
            }
            if vertex_format.tangent {
                let cross2 = Cartesian3::cross_new(&bitangent, &normal);
                tangent = Cartesian3::normalize_new(&cross2);
                tangents[i] = tangent.x;
                tangents[i1] = tangent.y;
                tangents[i2] = tangent.z;
                tangents[i + length] = tangent.x;
                tangents[i + 1 + length] = tangent.y;
                tangents[i + 2 + length] = tangent.z;
            }
            if vertex_format.bitangent {
                bitangents[i] = bitangent.x;
                bitangents[i1] = bitangent.y;
                bitangents[i2] = bitangent.z;
                bitangents[i + length] = bitangent.x;
                bitangents[i1 + length] = bitangent.y;
                bitangents[i2 + length] = bitangent.z;
            }
        }
    }

    // Normalize texture coordinates
    if vertex_format.st {
        let len = texture_coordinates.len();
        let mut k = 0;
        while k < len {
            let denom_x = max_tex_coord.x - min_tex_coord.x;
            let denom_y = max_tex_coord.y - min_tex_coord.y;
            texture_coordinates[k] = if denom_x.abs() > f64::EPSILON {
                (texture_coordinates[k] - min_tex_coord.x) / denom_x
            } else {
                0.0
            };
            texture_coordinates[k + 1] = if denom_y.abs() > f64::EPSILON {
                (texture_coordinates[k + 1] - min_tex_coord.y) / denom_y
            } else {
                0.0
            };
            k += 2;
        }
    }

    let mut attributes = HashMap::new();
    if vertex_format.position {
        attributes.insert(
            "position".to_string(),
            GeometryAttribute::new(ComponentDatatype::Double, 3, false, final_positions),
        );
    }
    if vertex_format.st {
        attributes.insert(
            "st".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 2, false, texture_coordinates),
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
    if options.shadow_volume {
        attributes.insert(
            "extrudeDirection".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 3, false, extrude_normals),
        );
    }
    if let Some(offset_attr) = options.offset_attribute {
        let apply_offset: Vec<f64> = if offset_attr == GeometryOffsetAttribute::Top {
            let mut v = vec![0.0f64; size];
            for i in 0..size / 2 {
                v[i] = 1.0;
            }
            v
        } else {
            let offset_value = if offset_attr == GeometryOffsetAttribute::None { 0 } else { 1 };
            vec![offset_value as f64; size]
        };
        attributes.insert(
            "applyOffset".to_string(),
            GeometryAttribute::new(ComponentDatatype::UnsignedByte, 1, false, apply_offset),
        );
    }

    attributes
}

/// Port of `computeWallIndices`.
fn compute_wall_indices(positions: &[f64]) -> IndexStorage {
    let length = positions.len() / 3;
    let mut indices = IndexDatatype::create_typed_array(length, length * 6);
    let mut index = 0usize;
    for i in 0..length {
        let ul = i as u32;
        let ll = (i + length) as u32;
        let ur = ((i + 1) % length) as u32;
        let lr = ur + length as u32;
        write_index(&mut indices, index, ul);
        index += 1;
        write_index(&mut indices, index, ll);
        index += 1;
        write_index(&mut indices, index, ur);
        index += 1;
        write_index(&mut indices, index, ur);
        index += 1;
        write_index(&mut indices, index, ll);
        index += 1;
        write_index(&mut indices, index, lr);
        index += 1;
    }
    indices
}

/// Result of compute_ellipse / compute_extruded_ellipse.
struct EllipseResult {
    bounding_sphere: BoundingSphere,
    attributes: HashMap<String, GeometryAttribute>,
    indices: IndexStorage,
}

/// Port of `computeExtrudedEllipse`.
///
/// DEVIATION: JS uses `GeometryPipeline.combineInstances` to merge top/bottom
/// and wall geometry. Rust merges manually.
fn compute_extruded_ellipse(options: &EllipseOptions) -> EllipseResult {
    let center = &options.center;
    let ellipsoid = &options.ellipsoid;
    let semi_major_axis = options.semi_major_axis;

    let mut normal = Cartesian3::default();
    ellipsoid.geodetic_surface_normal(center, &mut normal);
    let scaled = Cartesian3::multiply_by_scalar_new(&normal, options.height);
    let top_center = Cartesian3::add_new(center, &scaled);
    let top_bs = BoundingSphere::new(top_center, semi_major_axis);

    let scaled2 = Cartesian3::multiply_by_scalar_new(&normal, options.extruded_height);
    let bottom_center = Cartesian3::add_new(center, &scaled2);
    let bottom_bs = BoundingSphere::new(bottom_center, semi_major_axis);

    let lib_options = EllipseGeometryOptions {
        semi_minor_axis: options.semi_minor_axis,
        semi_major_axis: options.semi_major_axis,
        rotation: options.rotation,
        center: options.center.clone(),
        granularity: options.granularity,
    };
    let cep = EllipseGeometryLibrary::compute_ellipse_positions(&lib_options, true, true);
    let positions = cep.positions.unwrap();
    let num_pts = cep.num_pts;
    let outer_positions = cep.outer_positions.unwrap();
    let bounding_sphere = BoundingSphere::union(&top_bs, &bottom_bs, None);

    let top_bottom_attrs = compute_top_bottom_attributes(&positions, options, true);
    let mut raw_indices = top_indices(num_pts);
    let tb_length = raw_indices.len();
    raw_indices.resize(tb_length * 2, 0);
    let pos_length = positions.len() / 3;
    for i in (0..tb_length).step_by(3) {
        raw_indices[i + tb_length] = raw_indices[i + 2] + pos_length as u32;
        raw_indices[i + 1 + tb_length] = raw_indices[i + 1] + pos_length as u32;
        raw_indices[i + 2 + tb_length] = raw_indices[i] + pos_length as u32;
    }
    let top_bottom_indices = IndexDatatype::create_typed_array((pos_length * 2) / 3, raw_indices.len());
    for i in 0..raw_indices.len() {
        write_index(&mut top_bottom_indices.clone(), i, raw_indices[i]);
    }

    let wall_attrs = compute_wall_attributes(&outer_positions, options);
    let wall_indices = compute_wall_indices(&outer_positions);

    // Manually merge top/bottom + wall
    let mut merged_attrs = HashMap::new();
    let tb_pos_len = top_bottom_attrs.get("position").map(|a| a.values.len()).unwrap_or(0);
    let wall_pos_len = wall_attrs.get("position").map(|a| a.values.len()).unwrap_or(0);

    // Merge positions
    let mut merged_positions = Vec::with_capacity(tb_pos_len + wall_pos_len);
    if let Some(tb_pos) = top_bottom_attrs.get("position") {
        merged_positions.extend_from_slice(&tb_pos.values);
    }
    if let Some(w_pos) = wall_attrs.get("position") {
        merged_positions.extend_from_slice(&w_pos.values);
    }
    if !merged_positions.is_empty() {
        merged_attrs.insert(
            "position".to_string(),
            GeometryAttribute::new(ComponentDatatype::Double, 3, false, merged_positions),
        );
    }

    // Merge other attributes (st, normal, tangent, bitangent, extrudeDirection, applyOffset)
    for key in &["st", "normal", "tangent", "bitangent", "extrudeDirection", "applyOffset"] {
        let tb = top_bottom_attrs.get(*key);
        let w = wall_attrs.get(*key);
        if tb.is_none() && w.is_none() {
            continue;
        }
        let comp = if *key == "st" { 2u32 } else if *key == "applyOffset" { 1u32 } else { 3u32 };
        let dt = if *key == "applyOffset" {
            ComponentDatatype::UnsignedByte
        } else if *key == "position" {
            ComponentDatatype::Double
        } else {
            ComponentDatatype::Float
        };
        let tb_vals = tb.map(|a| &a.values[..]).unwrap_or(&[]);
        let w_vals = w.map(|a| &a.values[..]).unwrap_or(&[]);
        let mut merged = Vec::with_capacity(tb_vals.len() + w_vals.len());
        merged.extend_from_slice(tb_vals);
        merged.extend_from_slice(w_vals);
        merged_attrs.insert(
            key.to_string(),
            GeometryAttribute::new(dt, comp, false, merged),
        );
    }

    // Merge indices: top/bottom first, then wall (offset by tb vertex count)
    let tb_vertex_count = tb_pos_len / 3;
    let total_indices = top_bottom_indices.len() + wall_indices.len();
    let mut merged_indices = IndexDatatype::create_typed_array(
        (tb_pos_len + wall_pos_len) / 3,
        total_indices,
    );
    let mut idx = 0usize;
    for i in 0..top_bottom_indices.len() {
        write_index(&mut merged_indices, idx, read_index(&top_bottom_indices, i));
        idx += 1;
    }
    for i in 0..wall_indices.len() {
        write_index(&mut merged_indices, idx, read_index(&wall_indices, i) + tb_vertex_count as u32);
        idx += 1;
    }

    EllipseResult {
        bounding_sphere,
        attributes: merged_attrs,
        indices: merged_indices,
    }
}

/// Computes the geometric representation of an ellipse on an ellipsoid,
/// including vertices, indices, and a bounding sphere.
///
/// Port of `EllipseGeometry.createGeometry`.
pub fn create_geometry(ellipse_geometry: &EllipseGeometry) -> Option<Geometry> {
    if ellipse_geometry.semi_major_axis <= 0.0 || ellipse_geometry.semi_minor_axis <= 0.0 {
        return None;
    }

    let height = ellipse_geometry.height;
    let extruded_height = ellipse_geometry.extruded_height;
    let extrude = !CesiumMath::equals_epsilon(
        height,
        extruded_height,
        Some(0.0),
        Some(CesiumMath::EPSILON2),
    );

    let mut center = ellipse_geometry.center;
    let ellipsoid = &ellipse_geometry.ellipsoid;
    { let c = center; ellipsoid.scale_to_geodetic_surface(&c, &mut center); }

    let options = EllipseOptions {
        center: center.clone(),
        semi_major_axis: ellipse_geometry.semi_major_axis,
        semi_minor_axis: ellipse_geometry.semi_minor_axis,
        ellipsoid: ellipsoid.clone(),
        rotation: ellipse_geometry.rotation,
        height,
        extruded_height,
        granularity: ellipse_geometry.granularity,
        vertex_format: ellipse_geometry.vertex_format.clone(),
        st_rotation: ellipse_geometry.st_rotation,
        shadow_volume: ellipse_geometry.shadow_volume,
        offset_attribute: ellipse_geometry.offset_attribute,
    };

    let geometry = if extrude {
        compute_extruded_ellipse(&options)
    } else {
        let result = compute_ellipse(&options);
        let mut attrs = result.attributes;
        if let Some(offset_attr) = ellipse_geometry.offset_attribute {
            let length = attrs.get("position").map(|a| a.values.len()).unwrap_or(0);
            let offset_value = if offset_attr == GeometryOffsetAttribute::None { 0 } else { 1 };
            let apply_offset = vec![offset_value as f64; length / 3];
            attrs.insert(
                "applyOffset".to_string(),
                GeometryAttribute::new(ComponentDatatype::UnsignedByte, 1, false, apply_offset),
            );
        }
        EllipseResult {
            bounding_sphere: result.bounding_sphere,
            attributes: attrs,
            indices: result.indices,
        }
    };

    let offset_attr_str = ellipse_geometry
        .offset_attribute
        .map(|_| "applyOffset".to_string());

    Some(Geometry::with_all(
        geometry.attributes,
        Some(geometry.indices),
        Some(PrimitiveType::Triangles),
        Some(geometry.bounding_sphere),
        GeometryType::None,
        None,
        offset_attr_str,
    ))
}
