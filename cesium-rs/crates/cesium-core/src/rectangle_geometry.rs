//! Ported from `packages/engine/Source/Core/RectangleGeometry.js`.
//!
//! A description of a cartographic rectangle on an ellipsoid centered at the
//! origin. Rectangle geometry can be rendered with both `Primitive` and
//! `GroundPrimitive`.

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::component_datatype::ComponentDatatype;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_instance::{GeometryInstance, GeometryInstanceGeometry};
use crate::geometry_offset_attribute::GeometryOffsetAttribute;
use crate::geometry_pipeline::GeometryPipeline;
use crate::geometry_type::GeometryType;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::math::CesiumMath;
use crate::matrix2::Matrix2;
use crate::matrix3::Matrix3;
use crate::polygon_pipeline::PolygonPipeline;
use crate::primitive_type::PrimitiveType;
use crate::quaternion::Quaternion;
use crate::rectangle::Rectangle;
use crate::rectangle_geometry_library::{self as rectangle_geometry_library, ComputedOptions};
use crate::vertex_format::VertexFormat;

/// Index read helper for [`IndexStorage`].
fn read_index(indices: &IndexStorage, i: usize) -> u32 {
    match indices {
        IndexStorage::U16(v) => v[i] as u32,
        IndexStorage::U32(v) => v[i],
    }
}

/// Index write helper for [`IndexStorage`].
fn write_index(indices: &mut IndexStorage, i: usize, v: u32) {
    match indices {
        IndexStorage::U16(vec) => vec[i] = v as u16,
        IndexStorage::U32(vec) => vec[i] = v,
    }
}

/// A description of a cartographic rectangle on an ellipsoid centered at the
/// origin.
#[derive(Debug, Clone)]
pub struct RectangleGeometry {
    rectangle: Rectangle,
    granularity: f64,
    ellipsoid: Ellipsoid,
    surface_height: f64,
    rotation: f64,
    st_rotation: f64,
    vertex_format: VertexFormat,
    extruded_height: f64,
    shadow_volume: bool,
    offset_attribute: Option<GeometryOffsetAttribute>,
    /// Cache for the `rectangle` getter (JS `_rotatedRectangle`).
    rotated_rectangle: Option<Rectangle>,
    /// Cache for `texture_coordinate_rotation_points`
    /// (JS `_textureCoordinateRotationPoints`).
    texture_coordinate_rotation_points_cache: Option<Vec<f64>>,
}

/// Options for [`RectangleGeometry::from_options`] and
/// [`RectangleGeometry::compute_rectangle`] (mirrors the JS options object).
#[derive(Debug, Clone, Default)]
pub struct RectangleGeometryOptions {
    pub rectangle: Option<Rectangle>,
    pub vertex_format: Option<VertexFormat>,
    pub ellipsoid: Option<Ellipsoid>,
    pub granularity: Option<f64>,
    pub height: Option<f64>,
    pub rotation: Option<f64>,
    pub st_rotation: Option<f64>,
    pub extruded_height: Option<f64>,
    pub shadow_volume: Option<bool>,
    pub offset_attribute: Option<GeometryOffsetAttribute>,
}

impl RectangleGeometry {
    /// Creates a new `RectangleGeometry`.
    ///
    /// Retained positional form for spec compatibility; the JS constructor
    /// takes an options object (see [`RectangleGeometry::from_options`]).
    pub fn new(
        rectangle: Rectangle,
        height: Option<f64>,
        extruded_height: Option<f64>,
        granularity: Option<f64>,
        vertex_format: Option<VertexFormat>,
    ) -> Self {
        Self::from_options(RectangleGeometryOptions {
            rectangle: Some(rectangle),
            height,
            extruded_height,
            granularity,
            vertex_format,
            ..Default::default()
        })
    }

    /// JS constructor equivalent: `new RectangleGeometry(options)`.
    pub fn from_options(options: RectangleGeometryOptions) -> Self {
        let rectangle = options
            .rectangle
            .expect("options.rectangle is required");

        if cfg!(debug_assertions) {
            // JS: Rectangle._validate(rectangle)
            debug_assert!(
                rectangle.north >= -CesiumMath::PI_OVER_TWO
                    && rectangle.north <= CesiumMath::PI_OVER_TWO,
                "options.rectangle.north must be in the interval [-Pi/2, Pi/2]"
            );
            debug_assert!(
                rectangle.south >= -CesiumMath::PI_OVER_TWO
                    && rectangle.south <= CesiumMath::PI_OVER_TWO,
                "options.rectangle.south must be in the interval [-Pi/2, Pi/2]"
            );
            debug_assert!(
                rectangle.east >= -CesiumMath::PI && rectangle.east <= CesiumMath::PI,
                "options.rectangle.east must be in the interval [-Pi, Pi]"
            );
            debug_assert!(
                rectangle.west >= -CesiumMath::PI && rectangle.west <= CesiumMath::PI,
                "options.rectangle.west must be in the interval [-Pi, Pi]"
            );
            debug_assert!(
                rectangle.north >= rectangle.south,
                "options.rectangle.north must be greater than or equal to options.rectangle.south"
            );
        }

        let height = options.height.unwrap_or(0.0);
        let extruded_height = options.extruded_height.unwrap_or(height);

        Self {
            rectangle,
            granularity: options.granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE),
            ellipsoid: options.ellipsoid.unwrap_or(Ellipsoid::WGS84),
            surface_height: height.max(extruded_height),
            rotation: options.rotation.unwrap_or(0.0),
            st_rotation: options.st_rotation.unwrap_or(0.0),
            // JS: `defaultValue(options.vertexFormat, VertexFormat.DEFAULT)`;
            // `VertexFormat.DEFAULT` is `POSITION_NORMAL_AND_ST`.
            vertex_format: options
                .vertex_format
                .unwrap_or_else(VertexFormat::default_format),
            extruded_height: height.min(extruded_height),
            shadow_volume: options.shadow_volume.unwrap_or(false),
            offset_attribute: options.offset_attribute,
            rotated_rectangle: None,
            texture_coordinate_rotation_points_cache: None,
        }
    }

    /// The number of elements used to pack the object into an array.
    pub const PACKED_LENGTH: usize = Rectangle::PACKED_LENGTH
        + Ellipsoid::PACKED_LENGTH
        + VertexFormat::PACKED_LENGTH
        + 7;

    /// Stores the provided instance into the provided array.
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        let mut si = starting_index.unwrap_or(0);

        Rectangle::pack(&self.rectangle, array, Some(si));
        si += Rectangle::PACKED_LENGTH;

        Ellipsoid::pack(&self.ellipsoid, array, Some(si));
        si += Ellipsoid::PACKED_LENGTH;

        self.vertex_format.pack(array, si);
        si += VertexFormat::PACKED_LENGTH;

        array[si] = self.granularity;
        si += 1;
        array[si] = self.surface_height;
        si += 1;
        array[si] = self.rotation;
        si += 1;
        array[si] = self.st_rotation;
        si += 1;
        array[si] = self.extruded_height;
        si += 1;
        array[si] = if self.shadow_volume { 1.0 } else { 0.0 };
        si += 1;
        array[si] = match &self.offset_attribute {
            Some(v) => *v as i32 as f64,
            None => -1.0,
        };
    }

    /// Retrieves an instance from a packed array.
    pub fn unpack(
        array: &[f64],
        starting_index: Option<usize>,
        result: Option<&mut Self>,
    ) -> Self {
        let mut si = starting_index.unwrap_or(0);

        let rectangle = Rectangle::unpack(array, Some(si));
        si += Rectangle::PACKED_LENGTH;

        let ellipsoid = Ellipsoid::unpack(array, Some(si));
        si += Ellipsoid::PACKED_LENGTH;

        let vertex_format = VertexFormat::unpack(array, si, None);
        si += VertexFormat::PACKED_LENGTH;

        let granularity = array[si];
        si += 1;
        let surface_height = array[si];
        si += 1;
        let rotation = array[si];
        si += 1;
        let st_rotation = array[si];
        si += 1;
        let extruded_height = array[si];
        si += 1;
        let shadow_volume = array[si] == 1.0;
        si += 1;
        let offset_attribute = array[si];

        let offset_attribute = if offset_attribute == -1.0 {
            None
        } else {
            GeometryOffsetAttribute::try_from_u32(offset_attribute as u32)
        };

        match result {
            None => {
                // JS passes these through the constructor (which re-applies
                // max/min; packed data always has surface >= extruded).
                Self::from_options(RectangleGeometryOptions {
                    rectangle: Some(rectangle),
                    ellipsoid: Some(ellipsoid),
                    vertex_format: Some(vertex_format),
                    granularity: Some(granularity),
                    height: Some(surface_height),
                    rotation: Some(rotation),
                    st_rotation: Some(st_rotation),
                    extruded_height: Some(extruded_height),
                    shadow_volume: Some(shadow_volume),
                    offset_attribute,
                })
            }
            Some(r) => {
                r.rectangle = rectangle;
                r.ellipsoid = ellipsoid;
                r.vertex_format = vertex_format;
                r.granularity = granularity;
                r.surface_height = surface_height;
                r.rotation = rotation;
                r.st_rotation = st_rotation;
                r.extruded_height = extruded_height;
                r.shadow_volume = shadow_volume;
                r.offset_attribute = offset_attribute;
                r.clone()
            }
        }
    }

    /// Computes the bounding rectangle based on the provided options.
    /// JS `RectangleGeometry.computeRectangle(options, result)`.
    pub fn compute_rectangle(options: &RectangleGeometryOptions) -> Rectangle {
        let rectangle = options
            .rectangle
            .expect("options.rectangle is required");

        if cfg!(debug_assertions) {
            debug_assert!(
                rectangle.north >= -CesiumMath::PI_OVER_TWO
                    && rectangle.north <= CesiumMath::PI_OVER_TWO,
                "options.rectangle.north must be in the interval [-Pi/2, Pi/2]"
            );
            debug_assert!(
                rectangle.south >= -CesiumMath::PI_OVER_TWO
                    && rectangle.south <= CesiumMath::PI_OVER_TWO,
                "options.rectangle.south must be in the interval [-Pi/2, Pi/2]"
            );
            debug_assert!(
                rectangle.east >= -CesiumMath::PI && rectangle.east <= CesiumMath::PI,
                "options.rectangle.east must be in the interval [-Pi, Pi]"
            );
            debug_assert!(
                rectangle.west >= -CesiumMath::PI && rectangle.west <= CesiumMath::PI,
                "options.rectangle.west must be in the interval [-Pi, Pi]"
            );
            debug_assert!(
                rectangle.north >= rectangle.south,
                "options.rectangle.north must be greater than or equal to options.rectangle.south"
            );
        }

        let granularity = options.granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE);
        let ellipsoid = options.ellipsoid.unwrap_or(Ellipsoid::WGS84);
        let rotation = options.rotation.unwrap_or(0.0);

        compute_rectangle(&rectangle, granularity, rotation, &ellipsoid)
    }

    /// Computes the geometric representation of a rectangle, including its
    /// vertices, indices, and a bounding sphere.
    /// JS `RectangleGeometry.createGeometry`.
    pub fn create_geometry(rectangle_geometry: &Self) -> Option<Geometry> {
        if CesiumMath::equals_epsilon(
            rectangle_geometry.rectangle.north,
            rectangle_geometry.rectangle.south,
            Some(CesiumMath::EPSILON10),
            None,
        ) || CesiumMath::equals_epsilon(
            rectangle_geometry.rectangle.east,
            rectangle_geometry.rectangle.west,
            Some(CesiumMath::EPSILON10),
            None,
        ) {
            return None;
        }

        let rectangle = &rectangle_geometry.rectangle;
        let ellipsoid = &rectangle_geometry.ellipsoid;
        let rotation = rectangle_geometry.rotation;
        let st_rotation = rectangle_geometry.st_rotation;
        let vertex_format = rectangle_geometry.vertex_format.clone();

        let mut computed_options = rectangle_geometry_library::compute_options(
            rectangle,
            rectangle_geometry.granularity,
            rotation,
            st_rotation,
        );

        let tangent_rotation_matrix = if st_rotation != 0.0 || rotation != 0.0 {
            let center = Rectangle::center(rectangle);
            let mut axis = Cartesian3::default();
            ellipsoid.geodetic_surface_normal_cartographic(&center, &mut axis);
            let quaternion = Quaternion::from_axis_angle_new(&axis, -st_rotation);
            let mut m = Matrix3::default();
            Matrix3::from_quaternion(&quaternion, &mut m);
            m
        } else {
            Matrix3::IDENTITY
        };

        let surface_height = rectangle_geometry.surface_height;
        let extruded_height = rectangle_geometry.extruded_height;
        let extrude = !CesiumMath::equals_epsilon(
            surface_height,
            extruded_height,
            None,
            Some(CesiumMath::EPSILON2),
        );

        computed_options.lon_scalar = 1.0 / rectangle.width();
        computed_options.lat_scalar = 1.0 / rectangle.height();

        let (mut geometry, bounding_sphere) = if extrude {
            let geometry = construct_extruded_rectangle(
                rectangle_geometry,
                &computed_options,
                &tangent_rotation_matrix,
            );
            let top_bs = BoundingSphere::from_rectangle_3d(
                Some(rectangle),
                Some(ellipsoid),
                surface_height,
                None,
            );
            let bottom_bs = BoundingSphere::from_rectangle_3d(
                Some(rectangle),
                Some(ellipsoid),
                extruded_height,
                None,
            );
            let bounding_sphere = BoundingSphere::union(&top_bs, &bottom_bs, None);
            (geometry, bounding_sphere)
        } else {
            let mut geometry = construct_rectangle(
                rectangle_geometry,
                &computed_options,
                &tangent_rotation_matrix,
            );
            if let Some(positions) = geometry.attributes.get_mut("position") {
                PolygonPipeline::scale_to_geodetic_height(
                    Some(&mut positions.values),
                    Some(surface_height),
                    Some(ellipsoid),
                    Some(false),
                );
            }

            if let Some(offset_attribute) = &rectangle_geometry.offset_attribute {
                let length = geometry
                    .attributes
                    .get("position")
                    .map(|p| p.values.len())
                    .unwrap_or(0);
                let offset_value =
                    if *offset_attribute == GeometryOffsetAttribute::None { 0.0 } else { 1.0 };
                let apply_offset = vec![offset_value; length / 3];
                geometry.attributes.insert(
                    "applyOffset".to_string(),
                    GeometryAttribute::new(
                        ComponentDatatype::UnsignedByte,
                        1,
                        false,
                        apply_offset,
                    ),
                );
            }

            let bounding_sphere = BoundingSphere::from_rectangle_3d(
                Some(rectangle),
                Some(ellipsoid),
                surface_height,
                None,
            );
            (geometry, bounding_sphere)
        };

        if !vertex_format.position {
            geometry.attributes.remove("position");
        }

        let offset_attribute_name = rectangle_geometry
            .offset_attribute
            .as_ref()
            .map(|_| "applyOffset".to_string());

        Some(Geometry::with_all(
            geometry.attributes,
            geometry.indices,
            Some(geometry.primitive_type),
            Some(bounding_sphere),
            GeometryType::None,
            None,
            offset_attribute_name,
        ))
    }

    /// JS `RectangleGeometry.createShadowVolume` (private).
    pub fn create_shadow_volume(
        rectangle_geometry: &Self,
        min_height_func: &dyn Fn(f64, &Ellipsoid) -> f64,
        max_height_func: &dyn Fn(f64, &Ellipsoid) -> f64,
    ) -> Self {
        let granularity = rectangle_geometry.granularity;
        let ellipsoid = rectangle_geometry.ellipsoid.clone();

        let min_height = min_height_func(granularity, &ellipsoid);
        let max_height = max_height_func(granularity, &ellipsoid);

        Self::from_options(RectangleGeometryOptions {
            rectangle: Some(rectangle_geometry.rectangle),
            rotation: Some(rectangle_geometry.rotation),
            ellipsoid: Some(ellipsoid),
            st_rotation: Some(rectangle_geometry.st_rotation),
            granularity: Some(granularity),
            extruded_height: Some(max_height),
            height: Some(min_height),
            vertex_format: Some(VertexFormat::position_only()),
            shadow_volume: Some(true),
            ..Default::default()
        })
    }

    /// The granularity used when the geometry was constructed
    /// (JS internal `_granularity`; exposed for pack round-trip checks).
    pub fn granularity(&self) -> f64 {
        self.granularity
    }

    /// JS `rectangle` getter: the bounding rectangle after rotation.
    pub fn rectangle(&mut self) -> Rectangle {
        if self.rotated_rectangle.is_none() {
            self.rotated_rectangle = Some(compute_rectangle(
                &self.rectangle,
                self.granularity,
                self.rotation,
                &self.ellipsoid,
            ));
        }
        self.rotated_rectangle.unwrap()
    }

    /// JS `textureCoordinateRotationPoints` getter.
    pub fn texture_coordinate_rotation_points(&mut self) -> Vec<f64> {
        if self.texture_coordinate_rotation_points_cache.is_none() {
            self.texture_coordinate_rotation_points_cache =
                Some(texture_coordinate_rotation_points(self));
        }
        self.texture_coordinate_rotation_points_cache.clone().unwrap()
    }
}

/// Mirrors the module-level JS `computeRectangle(rectangle, granularity,
/// rotation, ellipsoid, result)`.
fn compute_rectangle(
    rectangle: &Rectangle,
    granularity: f64,
    rotation: f64,
    ellipsoid: &Ellipsoid,
) -> Rectangle {
    if rotation == 0.0 {
        return *rectangle;
    }

    let computed_options =
        rectangle_geometry_library::compute_options(rectangle, granularity, rotation, 0.0);

    let height = computed_options.height;
    let width = computed_options.width;

    let mut positions = [Cartesian3::default(); 4];
    let mut st = Cartesian2::default();
    rectangle_geometry_library::compute_position(
        &computed_options,
        ellipsoid,
        false,
        0.0,
        0.0,
        &mut positions[0],
        &mut st,
    );
    rectangle_geometry_library::compute_position(
        &computed_options,
        ellipsoid,
        false,
        0.0,
        (width - 1) as f64,
        &mut positions[1],
        &mut st,
    );
    rectangle_geometry_library::compute_position(
        &computed_options,
        ellipsoid,
        false,
        (height - 1) as f64,
        0.0,
        &mut positions[2],
        &mut st,
    );
    rectangle_geometry_library::compute_position(
        &computed_options,
        ellipsoid,
        false,
        (height - 1) as f64,
        (width - 1) as f64,
        &mut positions[3],
        &mut st,
    );

    Rectangle::from_cartesian_array(&positions, Some(ellipsoid))
}

/// Mirrors JS `textureCoordinateRotationPoints(rectangleGeometry)`.
fn texture_coordinate_rotation_points(rectangle_geometry: &mut RectangleGeometry) -> Vec<f64> {
    if rectangle_geometry.st_rotation == 0.0 {
        return vec![0.0, 0.0, 0.0, 1.0, 1.0, 0.0];
    }

    let rectangle = rectangle_geometry.rectangle;
    let granularity = rectangle_geometry.granularity;
    let ellipsoid = rectangle_geometry.ellipsoid.clone();

    // Rotate to align the texture coordinates with ENU
    let rotation = rectangle_geometry.rotation - rectangle_geometry.st_rotation;

    let unrotated_texture_rectangle =
        compute_rectangle(&rectangle, granularity, rotation, &ellipsoid);

    let mut points_2d = [
        Cartesian2::new(unrotated_texture_rectangle.west, unrotated_texture_rectangle.south),
        Cartesian2::new(unrotated_texture_rectangle.west, unrotated_texture_rectangle.north),
        Cartesian2::new(unrotated_texture_rectangle.east, unrotated_texture_rectangle.south),
    ];

    let bounding_rectangle = rectangle_geometry.rectangle();
    let to_desired_in_computed = Matrix2::from_rotation_new(rectangle_geometry.st_rotation);
    let bounding_rectangle_center = Rectangle::center(&bounding_rectangle);

    for point_2d in points_2d.iter_mut() {
        point_2d.x -= bounding_rectangle_center.longitude;
        point_2d.y -= bounding_rectangle_center.latitude;
        let mut rotated = Cartesian2::default();
        Matrix2::multiply_by_vector(&to_desired_in_computed, point_2d, &mut rotated);
        *point_2d = rotated;
        point_2d.x += bounding_rectangle_center.longitude;
        point_2d.y += bounding_rectangle_center.latitude;

        // Convert point into east-north texture coordinate space
        point_2d.x = (point_2d.x - bounding_rectangle.west) / bounding_rectangle.width();
        point_2d.y = (point_2d.y - bounding_rectangle.south) / bounding_rectangle.height();
    }

    let mut result = vec![0.0f64; 6];
    Cartesian2::pack(&points_2d[0], &mut result, None);
    Cartesian2::pack(&points_2d[1], &mut result, Some(2));
    Cartesian2::pack(&points_2d[2], &mut result, Some(4));
    result
}

/// Mirrors JS `createAttributes(vertexFormat, attributes)`.
fn create_attributes(
    vertex_format: &VertexFormat,
    positions: Vec<f64>,
    normals: Option<Vec<f64>>,
    tangents: Option<Vec<f64>>,
    bitangents: Option<Vec<f64>>,
) -> Geometry {
    let mut attributes = HashMap::new();
    attributes.insert(
        "position".to_string(),
        GeometryAttribute::new(ComponentDatatype::Double, 3, false, positions),
    );
    if vertex_format.normal {
        if let Some(normals) = normals {
            attributes.insert(
                "normal".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 3, false, normals),
            );
        }
    }
    if vertex_format.tangent {
        if let Some(tangents) = tangents {
            attributes.insert(
                "tangent".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 3, false, tangents),
            );
        }
    }
    if vertex_format.bitangent {
        if let Some(bitangents) = bitangents {
            attributes.insert(
                "bitangent".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 3, false, bitangents),
            );
        }
    }
    Geometry::with_all(
        attributes,
        None,
        Some(PrimitiveType::Triangles),
        None,
        GeometryType::None,
        None,
        None,
    )
}

/// Mirrors JS `calculateAttributes(positions, vertexFormat, ellipsoid,
/// tangentRotationMatrix)`.
fn calculate_attributes(
    positions: &[f64],
    vertex_format: &VertexFormat,
    ellipsoid: &Ellipsoid,
    tangent_rotation_matrix: &Matrix3,
) -> Geometry {
    let length = positions.len();

    let mut normals = if vertex_format.normal { Some(vec![0.0f64; length]) } else { None };
    let mut tangents = if vertex_format.tangent { Some(vec![0.0f64; length]) } else { None };
    let mut bitangents =
        if vertex_format.bitangent { Some(vec![0.0f64; length]) } else { None };

    let mut attr_index = 0usize;
    if vertex_format.normal || vertex_format.tangent || vertex_format.bitangent {
        let mut i = 0usize;
        while i < length {
            let p = Cartesian3::new(positions[i], positions[i + 1], positions[i + 2]);
            let attr_index1 = attr_index + 1;
            let attr_index2 = attr_index + 2;

            let mut normal = Cartesian3::default();
            ellipsoid.geodetic_surface_normal(&p, &mut normal);

            let mut tangent = Cartesian3::default();
            let mut bitangent = Cartesian3::default();
            if vertex_format.tangent || vertex_format.bitangent {
                let mut tmp = Cartesian3::default();
                Cartesian3::cross(&Cartesian3::UNIT_Z, &normal, &mut tangent);
                Matrix3::multiply_by_vector(tangent_rotation_matrix, &tangent, &mut tmp);
                tangent = tmp;
                Cartesian3::normalize(&tangent, &mut tmp);
                tangent = tmp;

                if vertex_format.bitangent {
                    Cartesian3::cross(&normal, &tangent, &mut bitangent);
                    Cartesian3::normalize(&bitangent, &mut tmp);
                    bitangent = tmp;
                }
            }

            if let Some(normals) = normals.as_mut() {
                normals[attr_index] = normal.x;
                normals[attr_index1] = normal.y;
                normals[attr_index2] = normal.z;
            }
            if let Some(tangents) = tangents.as_mut() {
                tangents[attr_index] = tangent.x;
                tangents[attr_index1] = tangent.y;
                tangents[attr_index2] = tangent.z;
            }
            if let Some(bitangents) = bitangents.as_mut() {
                bitangents[attr_index] = bitangent.x;
                bitangents[attr_index1] = bitangent.y;
                bitangents[attr_index2] = bitangent.z;
            }
            attr_index += 3;
            i += 3;
        }
    }

    create_attributes(vertex_format, positions.to_vec(), normals, tangents, bitangents)
}

/// Mirrors JS `calculateAttributesWall(positions, vertexFormat, ellipsoid)`.
fn calculate_attributes_wall(
    positions: &[f64],
    vertex_format: &VertexFormat,
    ellipsoid: &Ellipsoid,
) -> Geometry {
    let length = positions.len();

    let mut normals = if vertex_format.normal { Some(vec![0.0f64; length]) } else { None };
    let mut tangents = if vertex_format.tangent { Some(vec![0.0f64; length]) } else { None };
    let mut bitangents =
        if vertex_format.bitangent { Some(vec![0.0f64; length]) } else { None };

    let mut normal_index = 0usize;
    let mut tangent_index = 0usize;
    let mut bitangent_index = 0usize;
    let mut recompute_normal = true;

    if vertex_format.normal || vertex_format.tangent || vertex_format.bitangent {
        // JS keeps `normal` in a scratch variable persisting across loop
        // iterations: pairs with `recompute_normal == false` reuse the
        // previously computed wall normal (including the second of a
        // duplicated corner pair, whose own edge delta would be zero).
        let mut normal = Cartesian3::default();
        let mut i = 0usize;
        while i < length {
            let p = Cartesian3::new(positions[i], positions[i + 1], positions[i + 2]);
            let p1_offset = (i + 6) % length;
            let mut p1 = Cartesian3::new(
                positions[p1_offset],
                positions[p1_offset + 1],
                positions[p1_offset + 2],
            );

            if recompute_normal {
                let p2_offset = (i + 3) % length;
                let p2 = Cartesian3::new(
                    positions[p2_offset],
                    positions[p2_offset + 1],
                    positions[p2_offset + 2],
                );
                p1 = Cartesian3::subtract_new(&p1, &p);
                let p2_delta = Cartesian3::subtract_new(&p2, &p);
                let mut tmp = Cartesian3::default();
                Cartesian3::cross(&p2_delta, &p1, &mut normal);
                Cartesian3::normalize(&normal, &mut tmp);
                normal = tmp;
                recompute_normal = false;
            }

            if Cartesian3::equals_epsilon(
                Some(&p1),
                Some(&p),
                Some(CesiumMath::EPSILON10),
                None,
            ) {
                // if we've reached a corner
                recompute_normal = true;
            }

            let mut tangent = Cartesian3::default();
            let mut bitangent = Cartesian3::default();
            if vertex_format.tangent || vertex_format.bitangent {
                ellipsoid.geodetic_surface_normal(&p, &mut bitangent);
                if vertex_format.tangent {
                    let mut tmp = Cartesian3::default();
                    Cartesian3::cross(&bitangent, &normal, &mut tangent);
                    Cartesian3::normalize(&tangent, &mut tmp);
                    tangent = tmp;
                }
            }

            if let Some(normals) = normals.as_mut() {
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
            if let Some(tangents) = tangents.as_mut() {
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
            if let Some(bitangents) = bitangents.as_mut() {
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
            i += 6;
        }
    }

    create_attributes(vertex_format, positions.to_vec(), normals, tangents, bitangents)
}

/// Mirrors JS `constructRectangle(rectangleGeometry, computedOptions)`.
fn construct_rectangle(
    rectangle_geometry: &RectangleGeometry,
    computed_options: &ComputedOptions,
    tangent_rotation_matrix: &Matrix3,
) -> Geometry {
    let vertex_format = &rectangle_geometry.vertex_format;
    let ellipsoid = &rectangle_geometry.ellipsoid;
    let height = computed_options.height;
    let width = computed_options.width;
    let north_cap = computed_options.north_cap;
    let south_cap = computed_options.south_cap;

    let row_start = if north_cap { 1 } else { 0 };
    let row_end = if south_cap { height - 1 } else { height };
    let mut row_height = height;
    let mut size = 0usize;
    if north_cap {
        row_height -= 1;
        size += 1;
    }
    if south_cap {
        row_height -= 1;
        size += 1;
    }
    size += width * row_height;

    let mut positions = if vertex_format.position { Some(vec![0.0f64; size * 3]) } else { None };
    let mut texture_coordinates =
        if vertex_format.st { Some(vec![0.0f64; size * 2]) } else { None };

    let mut pos_index = 0usize;
    let mut st_index = 0usize;

    let mut position = Cartesian3::default();
    let mut st = Cartesian2::default();

    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for row in row_start..row_end {
        for col in 0..width {
            rectangle_geometry_library::compute_position(
                computed_options,
                ellipsoid,
                vertex_format.st,
                row as f64,
                col as f64,
                &mut position,
                &mut st,
            );

            if let Some(positions) = positions.as_mut() {
                positions[pos_index] = position.x;
                positions[pos_index + 1] = position.y;
                positions[pos_index + 2] = position.z;
            }
            pos_index += 3;

            if let Some(tex_coords) = texture_coordinates.as_mut() {
                tex_coords[st_index] = st.x;
                tex_coords[st_index + 1] = st.y;

                min_x = min_x.min(st.x);
                min_y = min_y.min(st.y);
                max_x = max_x.max(st.x);
                max_y = max_y.max(st.y);
            }
            st_index += 2;
        }
    }

    if north_cap {
        rectangle_geometry_library::compute_position(
            computed_options,
            ellipsoid,
            vertex_format.st,
            0.0,
            0.0,
            &mut position,
            &mut st,
        );

        if let Some(positions) = positions.as_mut() {
            positions[pos_index] = position.x;
            positions[pos_index + 1] = position.y;
            positions[pos_index + 2] = position.z;
        }
        pos_index += 3;

        if let Some(tex_coords) = texture_coordinates.as_mut() {
            tex_coords[st_index] = st.x;
            tex_coords[st_index + 1] = st.y;

            min_x = st.x;
            min_y = st.y;
            max_x = st.x;
            max_y = st.y;
        }
        st_index += 2;
    }

    if south_cap {
        rectangle_geometry_library::compute_position(
            computed_options,
            ellipsoid,
            vertex_format.st,
            (height - 1) as f64,
            0.0,
            &mut position,
            &mut st,
        );

        if let Some(positions) = positions.as_mut() {
            positions[pos_index] = position.x;
            positions[pos_index + 1] = position.y;
            positions[pos_index + 2] = position.z;
        }

        if let Some(tex_coords) = texture_coordinates.as_mut() {
            tex_coords[st_index] = st.x;
            tex_coords[st_index + 1] = st.y;

            min_x = min_x.min(st.x);
            min_y = min_y.min(st.y);
            max_x = max_x.max(st.x);
            max_y = max_y.max(st.y);
        }
    }

    if vertex_format.st && (min_x < 0.0 || min_y < 0.0 || max_x > 1.0 || max_y > 1.0) {
        if let Some(tex_coords) = texture_coordinates.as_mut() {
            let mut k = 0usize;
            while k < tex_coords.len() {
                tex_coords[k] = (tex_coords[k] - min_x) / (max_x - min_x);
                tex_coords[k + 1] = (tex_coords[k + 1] - min_y) / (max_y - min_y);
                k += 2;
            }
        }
    }

    let mut geo = calculate_attributes(
        positions.as_deref().unwrap_or(&[]),
        vertex_format,
        ellipsoid,
        tangent_rotation_matrix,
    );

    let mut indices_size = 6 * (width - 1) * (row_height - 1);
    if north_cap {
        indices_size += 3 * (width - 1);
    }
    if south_cap {
        indices_size += 3 * (width - 1);
    }
    let mut indices = IndexDatatype::create_typed_array(size, indices_size);
    let mut index = 0usize;
    let mut indices_index = 0usize;
    for _i in 0..row_height - 1 {
        for _j in 0..width - 1 {
            let upper_left = index;
            let lower_left = upper_left + width;
            let lower_right = lower_left + 1;
            let upper_right = upper_left + 1;
            write_index(&mut indices, indices_index, upper_left as u32);
            indices_index += 1;
            write_index(&mut indices, indices_index, lower_left as u32);
            indices_index += 1;
            write_index(&mut indices, indices_index, upper_right as u32);
            indices_index += 1;
            write_index(&mut indices, indices_index, upper_right as u32);
            indices_index += 1;
            write_index(&mut indices, indices_index, lower_left as u32);
            indices_index += 1;
            write_index(&mut indices, indices_index, lower_right as u32);
            indices_index += 1;
            index += 1;
        }
        index += 1;
    }
    if north_cap || south_cap {
        let mut north_index = size - 1;
        let south_index = size - 1;
        if north_cap && south_cap {
            north_index = size - 2;
        }

        index = 0;

        if north_cap {
            for _i in 0..width - 1 {
                let p1 = index;
                let p2 = p1 + 1;
                write_index(&mut indices, indices_index, north_index as u32);
                indices_index += 1;
                write_index(&mut indices, indices_index, p1 as u32);
                indices_index += 1;
                write_index(&mut indices, indices_index, p2 as u32);
                indices_index += 1;
                index += 1;
            }
        }
        if south_cap {
            index = (row_height - 1) * width;
            for _i in 0..width - 1 {
                let p1 = index;
                let p2 = p1 + 1;
                write_index(&mut indices, indices_index, p1 as u32);
                indices_index += 1;
                write_index(&mut indices, indices_index, south_index as u32);
                indices_index += 1;
                write_index(&mut indices, indices_index, p2 as u32);
                indices_index += 1;
                index += 1;
            }
        }
    }

    geo.indices = Some(indices);
    if vertex_format.st {
        if let Some(tex_coords) = texture_coordinates {
            geo.attributes.insert(
                "st".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 2, false, tex_coords),
            );
        }
    }

    geo
}

/// Mirrors JS `addWallPositions`.
fn add_wall_positions(
    wall_positions: &mut [f64],
    pos_index: usize,
    i: usize,
    top_positions: &[f64],
    bottom_positions: &[f64],
) {
    wall_positions[pos_index] = top_positions[i];
    wall_positions[pos_index + 1] = top_positions[i + 1];
    wall_positions[pos_index + 2] = top_positions[i + 2];
    wall_positions[pos_index + 3] = bottom_positions[i];
    wall_positions[pos_index + 4] = bottom_positions[i + 1];
    wall_positions[pos_index + 5] = bottom_positions[i + 2];
}

/// Mirrors JS `addWallTextureCoordinates`.
fn add_wall_texture_coordinates(
    wall_textures: &mut [f64],
    st_index: usize,
    i: usize,
    st: &[f64],
) {
    wall_textures[st_index] = st[i];
    wall_textures[st_index + 1] = st[i + 1];
    wall_textures[st_index + 2] = st[i];
    wall_textures[st_index + 3] = st[i + 1];
}

/// Mirrors JS `constructExtrudedRectangle(rectangleGeometry, computedOptions)`.
fn construct_extruded_rectangle(
    rectangle_geometry: &RectangleGeometry,
    computed_options: &ComputedOptions,
    tangent_rotation_matrix: &Matrix3,
) -> Geometry {
    let shadow_volume = rectangle_geometry.shadow_volume;
    let offset_attribute_value = rectangle_geometry.offset_attribute;
    let vertex_format = rectangle_geometry.vertex_format.clone();
    let min_height = rectangle_geometry.extruded_height;
    let max_height = rectangle_geometry.surface_height;
    let ellipsoid = rectangle_geometry.ellipsoid.clone();

    let height = computed_options.height;
    let width = computed_options.width;

    // DEVIATION: JS temporarily mutates `rectangleGeometry._vertexFormat`
    // (adds `normal`) while building the top/bottom geometry when
    // `shadowVolume` is true. This port threads an effective vertex format
    // through instead of mutating `self`.
    let mut effective_vertex_format = vertex_format.clone();
    if shadow_volume {
        effective_vertex_format.normal = true;
    }
    let mut shadow_geometry = RectangleGeometry {
        vertex_format: effective_vertex_format.clone(),
        ..rectangle_geometry.clone()
    };

    let mut top_bottom_geo =
        construct_rectangle(&shadow_geometry, computed_options, tangent_rotation_matrix);
    if shadow_volume {
        shadow_geometry.vertex_format = vertex_format.clone();
    }
    let _ = &shadow_geometry;

    let flat_positions = top_bottom_geo
        .attributes
        .get("position")
        .map(|p| p.values.clone())
        .unwrap_or_default();

    let mut top_positions = flat_positions.clone();
    PolygonPipeline::scale_to_geodetic_height(
        Some(&mut top_positions),
        Some(max_height),
        Some(&ellipsoid),
        Some(false),
    );
    let length = top_positions.len();
    let new_length = length * 2;
    let mut positions = vec![0.0f64; new_length];
    positions[..length].copy_from_slice(&top_positions);
    let mut bottom_positions = flat_positions;
    PolygonPipeline::scale_to_geodetic_height(
        Some(&mut bottom_positions),
        Some(min_height),
        Some(&ellipsoid),
        None,
    );
    positions[length..].copy_from_slice(&bottom_positions);
    top_bottom_geo
        .attributes
        .get_mut("position")
        .unwrap()
        .values = positions;

    let mut normals = if vertex_format.normal { Some(vec![0.0f64; new_length]) } else { None };
    let mut tangents = if vertex_format.tangent { Some(vec![0.0f64; new_length]) } else { None };
    let mut bitangents =
        if vertex_format.bitangent { Some(vec![0.0f64; new_length]) } else { None };
    let mut textures = if vertex_format.st {
        Some(vec![0.0f64; (new_length / 3) * 2])
    } else {
        None
    };

    let mut top_st: Option<Vec<f64>> = None;
    let mut top_normals: Option<Vec<f64>> = None;

    if vertex_format.normal {
        let mut top_normals_local = top_bottom_geo
            .attributes
            .get("normal")
            .map(|a| a.values.clone())
            .unwrap_or_default();
        if let Some(normals) = normals.as_mut() {
            normals[..length].copy_from_slice(&top_normals_local);
        }
        for i in 0..length {
            top_normals_local[i] = -top_normals_local[i];
        }
        if let Some(normals) = normals.as_mut() {
            normals[length..].copy_from_slice(&top_normals_local);
        }
        top_bottom_geo
            .attributes
            .get_mut("normal")
            .unwrap()
            // JS: `topBottomGeo.attributes.normal.values = normals`
            // (the fully duplicated top/bottom array).
            .values = normals.as_ref().unwrap().clone();
        top_normals = Some(top_normals_local);
    }
    if shadow_volume {
        let mut top_normals_local = top_normals.take().unwrap_or_else(|| {
            top_bottom_geo
                .attributes
                .get("normal")
                .map(|a| a.values.clone())
                .unwrap_or_default()
        });
        if !vertex_format.normal {
            top_bottom_geo.attributes.remove("normal");
        }
        let mut extrude_normals = vec![0.0f64; new_length];
        for i in 0..length {
            top_normals_local[i] = -top_normals_local[i];
        }
        // only get normals for bottom layer that's going to be pushed down
        extrude_normals[length..].copy_from_slice(&top_normals_local[..length]);
        top_bottom_geo.attributes.insert(
            "extrudeDirection".to_string(),
            GeometryAttribute::new(ComponentDatatype::Float, 3, false, extrude_normals),
        );
        top_normals = Some(top_normals_local);
    }

    let has_offsets = offset_attribute_value.is_some();
    if has_offsets {
        let size = (length / 3) * 2;
        let mut offset_attribute = vec![0.0f64; size];
        if offset_attribute_value == Some(GeometryOffsetAttribute::Top) {
            for v in offset_attribute.iter_mut().take(size / 2) {
                *v = 1.0;
            }
        } else {
            let offset_value =
                if offset_attribute_value == Some(GeometryOffsetAttribute::None) { 0.0 } else { 1.0 };
            for v in offset_attribute.iter_mut() {
                *v = offset_value;
            }
        }

        top_bottom_geo.attributes.insert(
            "applyOffset".to_string(),
            GeometryAttribute::new(ComponentDatatype::UnsignedByte, 1, false, offset_attribute),
        );
    }

    if vertex_format.tangent {
        let mut top_tangents = top_bottom_geo
            .attributes
            .get("tangent")
            .map(|a| a.values.clone())
            .unwrap_or_default();
        if let Some(tangents) = tangents.as_mut() {
            tangents[..length].copy_from_slice(&top_tangents);
        }
        for i in 0..length {
            top_tangents[i] = -top_tangents[i];
        }
        if let Some(tangents) = tangents.as_mut() {
            tangents[length..].copy_from_slice(&top_tangents);
        }
        top_bottom_geo
            .attributes
            .get_mut("tangent")
            .unwrap()
            // JS: `topBottomGeo.attributes.tangent.values = tangents`
            // (the fully duplicated top/bottom array).
            .values = tangents.as_ref().unwrap().clone();
    }
    if vertex_format.bitangent {
        let top_bitangents = top_bottom_geo
            .attributes
            .get("bitangent")
            .map(|a| a.values.clone())
            .unwrap_or_default();
        if let Some(bitangents) = bitangents.as_mut() {
            bitangents[..length].copy_from_slice(&top_bitangents);
            bitangents[length..].copy_from_slice(&top_bitangents);
        }
        top_bottom_geo
            .attributes
            .get_mut("bitangent")
            .unwrap()
            // JS: `topBottomGeo.attributes.bitangent.values = bitangents`
            // (the fully duplicated top/bottom array).
            .values = bitangents.as_ref().unwrap().clone();
    }
    if vertex_format.st {
        let top_st_local = top_bottom_geo
            .attributes
            .get("st")
            .map(|a| a.values.clone())
            .unwrap_or_default();
        if let Some(textures) = textures.as_mut() {
            let st_len = (length / 3) * 2;
            textures[..st_len].copy_from_slice(&top_st_local);
            textures[st_len..].copy_from_slice(&top_st_local);
        }
        top_bottom_geo
            .attributes
            .get_mut("st")
            .unwrap()
            // JS: `topBottomGeo.attributes.st.values = textures`
            // (the fully duplicated top/bottom array).
            .values = textures.as_ref().unwrap().clone();
        top_st = Some(top_st_local);
    }

    let indices = top_bottom_geo.indices.clone().unwrap();
    let indices_length = indices.len();
    let pos_length = length / 3;
    let mut new_indices =
        IndexDatatype::create_typed_array(new_length / 3, indices_length * 2);
    for i in 0..indices_length {
        write_index(&mut new_indices, i, read_index(&indices, i));
    }
    let mut i = 0usize;
    while i < indices_length {
        write_index(
            &mut new_indices,
            i + indices_length,
            read_index(&indices, i + 2) + pos_length as u32,
        );
        write_index(
            &mut new_indices,
            i + 1 + indices_length,
            read_index(&indices, i + 1) + pos_length as u32,
        );
        write_index(
            &mut new_indices,
            i + 2 + indices_length,
            read_index(&indices, i) + pos_length as u32,
        );
        i += 3;
    }
    top_bottom_geo.indices = Some(new_indices);

    let north_cap = computed_options.north_cap;
    let south_cap = computed_options.south_cap;

    let mut row_height = height;
    let mut width_multiplier = 2;
    let mut perimeter_positions = 0usize;
    let mut corners = 4usize;
    let mut duplicate_corners = 4usize;
    if north_cap {
        width_multiplier -= 1;
        row_height -= 1;
        perimeter_positions += 1;
        corners -= 2;
        duplicate_corners -= 1;
    }
    if south_cap {
        width_multiplier -= 1;
        row_height -= 1;
        perimeter_positions += 1;
        corners -= 2;
        duplicate_corners -= 1;
    }
    perimeter_positions += width_multiplier * width + 2 * row_height - corners;

    let wall_count = (perimeter_positions + duplicate_corners) * 2;

    let mut wall_positions = vec![0.0f64; wall_count * 3];
    let mut wall_extrude_normals =
        if shadow_volume { Some(vec![0.0f64; wall_count * 3]) } else { None };
    let mut wall_offset_attribute =
        if has_offsets { Some(vec![0.0f64; wall_count]) } else { None };
    let mut wall_textures =
        if vertex_format.st { Some(vec![0.0f64; wall_count * 2]) } else { None };

    let compute_top_offsets = offset_attribute_value == Some(GeometryOffsetAttribute::Top);
    if has_offsets && !compute_top_offsets {
        let fill_value =
            if offset_attribute_value == Some(GeometryOffsetAttribute::All) { 1.0 } else { 0.0 };
        if let Some(wall_offset) = wall_offset_attribute.as_mut() {
            for v in wall_offset.iter_mut() {
                *v = fill_value;
            }
        }
    }

    let top_positions = &top_positions;
    let top_normals_ref = top_normals.as_deref().unwrap_or(&[]);
    let top_st_ref = top_st.as_deref().unwrap_or(&[]);

    let mut pos_index = 0usize;
    let mut st_index = 0usize;
    let mut extrude_normal_index = 0usize;
    let mut wall_offset_index = 0usize;
    let area = width * row_height;

    // Perimeter traversal (mirrors the four JS loops).
    let mut emit = |three_i: usize,
                    pos_index: &mut usize,
                    st_index: &mut usize,
                    extrude_normal_index: &mut usize,
                    wall_offset_index: &mut usize| {
        add_wall_positions(
            &mut wall_positions,
            *pos_index,
            three_i,
            top_positions,
            &bottom_positions,
        );
        *pos_index += 6;
        if vertex_format.st {
            if let Some(wall_textures) = wall_textures.as_mut() {
                add_wall_texture_coordinates(wall_textures, *st_index, three_i / 3 * 2, top_st_ref);
            }
            *st_index += 4;
        }
        if shadow_volume {
            if let Some(wall_extrude_normals) = wall_extrude_normals.as_mut() {
                *extrude_normal_index += 3;
                wall_extrude_normals[*extrude_normal_index] = top_normals_ref[three_i];
                *extrude_normal_index += 1;
                wall_extrude_normals[*extrude_normal_index] = top_normals_ref[three_i + 1];
                *extrude_normal_index += 1;
                wall_extrude_normals[*extrude_normal_index] = top_normals_ref[three_i + 2];
            }
        }
        if compute_top_offsets {
            if let Some(wall_offset) = wall_offset_attribute.as_mut() {
                wall_offset[*wall_offset_index] = 1.0;
            }
            *wall_offset_index += 1;
            *wall_offset_index += 1;
        }
    };

    let mut i = 0usize;
    while i < area {
        let three_i = i * 3;
        emit(
            three_i,
            &mut pos_index,
            &mut st_index,
            &mut extrude_normal_index,
            &mut wall_offset_index,
        );
        i += width;
    }

    if !south_cap {
        for i in area - width..area {
            let three_i = i * 3;
            emit(
                three_i,
                &mut pos_index,
                &mut st_index,
                &mut extrude_normal_index,
                &mut wall_offset_index,
            );
        }
    } else {
        let south_index = if north_cap { area + 1 } else { area };
        let three_i = south_index * 3;
        for _ in 0..2 {
            // duplicate corner points
            emit(
                three_i,
                &mut pos_index,
                &mut st_index,
                &mut extrude_normal_index,
                &mut wall_offset_index,
            );
        }
    }

    i = area - 1;
    while i > 0 {
        let three_i = i * 3;
        emit(
            three_i,
            &mut pos_index,
            &mut st_index,
            &mut extrude_normal_index,
            &mut wall_offset_index,
        );
        if i < width {
            break;
        }
        i -= width;
    }

    if !north_cap {
        for i in (0..=width - 1).rev() {
            let three_i = i * 3;
            emit(
                three_i,
                &mut pos_index,
                &mut st_index,
                &mut extrude_normal_index,
                &mut wall_offset_index,
            );
        }
    } else {
        let north_index = area;
        let three_i = north_index * 3;
        for _ in 0..2 {
            // duplicate corner points
            emit(
                three_i,
                &mut pos_index,
                &mut st_index,
                &mut extrude_normal_index,
                &mut wall_offset_index,
            );
        }
    }

    let mut geo = calculate_attributes_wall(&wall_positions, &vertex_format, &ellipsoid);

    if vertex_format.st {
        if let Some(wall_textures) = wall_textures {
            geo.attributes.insert(
                "st".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 2, false, wall_textures),
            );
        }
    }
    if shadow_volume {
        if let Some(wall_extrude_normals) = wall_extrude_normals {
            geo.attributes.insert(
                "extrudeDirection".to_string(),
                GeometryAttribute::new(ComponentDatatype::Float, 3, false, wall_extrude_normals),
            );
        }
    }
    if has_offsets {
        if let Some(wall_offset_attribute) = wall_offset_attribute {
            geo.attributes.insert(
                "applyOffset".to_string(),
                GeometryAttribute::new(
                    ComponentDatatype::UnsignedByte,
                    1,
                    false,
                    wall_offset_attribute,
                ),
            );
        }
    }

    let mut wall_indices =
        IndexDatatype::create_typed_array(wall_count, perimeter_positions * 6);

    let length = wall_positions.len() / 3;
    let mut index = 0usize;
    i = 0usize;
    while i < length - 1 {
        let upper_left = i;
        let upper_right = (upper_left + 2) % length;
        let p1 = Cartesian3::new(
            wall_positions[upper_left * 3],
            wall_positions[upper_left * 3 + 1],
            wall_positions[upper_left * 3 + 2],
        );
        let p2 = Cartesian3::new(
            wall_positions[upper_right * 3],
            wall_positions[upper_right * 3 + 1],
            wall_positions[upper_right * 3 + 2],
        );
        if Cartesian3::equals_epsilon(Some(&p1), Some(&p2), Some(CesiumMath::EPSILON10), None) {
            i += 2;
            continue;
        }
        let lower_left = (upper_left + 1) % length;
        let lower_right = (lower_left + 2) % length;
        write_index(&mut wall_indices, index, upper_left as u32);
        index += 1;
        write_index(&mut wall_indices, index, lower_left as u32);
        index += 1;
        write_index(&mut wall_indices, index, upper_right as u32);
        index += 1;
        write_index(&mut wall_indices, index, upper_right as u32);
        index += 1;
        write_index(&mut wall_indices, index, lower_left as u32);
        index += 1;
        write_index(&mut wall_indices, index, lower_right as u32);
        index += 1;
        i += 2;
    }

    geo.indices = Some(wall_indices);

    let geometries = vec![
        GeometryInstance::new(
            GeometryInstanceGeometry::Geometry(Box::new(top_bottom_geo)),
            None,
            None,
            None,
        ),
        GeometryInstance::new(
            GeometryInstanceGeometry::Geometry(Box::new(geo)),
            None,
            None,
            None,
        ),
    ];
    let mut combined = GeometryPipeline::combine_instances(&geometries);
    combined.remove(0)
}
