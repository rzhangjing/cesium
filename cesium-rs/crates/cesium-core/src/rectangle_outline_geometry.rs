//! Ported from `packages/engine/Source/Core/RectangleOutlineGeometry.js`.
//!
//! A description of the outline of a cartographic rectangle on an ellipsoid
//! centered at the origin.

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::component_datatype::ComponentDatatype;
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
use crate::rectangle_geometry_library::{self as rectangle_geometry_library, ComputedOptions};

/// A description of the outline of a cartographic rectangle on an ellipsoid
/// centered at the origin.
#[derive(Debug, Clone)]
pub struct RectangleOutlineGeometry {
    rectangle: Rectangle,
    granularity: f64,
    ellipsoid: Ellipsoid,
    surface_height: f64,
    rotation: f64,
    extruded_height: f64,
    offset_attribute: Option<GeometryOffsetAttribute>,
}

impl RectangleOutlineGeometry {
    /// Creates a new `RectangleOutlineGeometry`.
    ///
    /// Retained for spec compatibility; the JS constructor takes an options
    /// object (see [`RectangleOutlineGeometry::from_options`]).
    pub fn new(
        rectangle: Rectangle,
        height: Option<f64>,
        extruded_height: Option<f64>,
        granularity: Option<f64>,
    ) -> Self {
        let height = height.unwrap_or(0.0);
        let extruded_height = extruded_height.unwrap_or(height);
        Self {
            rectangle,
            granularity: granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE),
            ellipsoid: Ellipsoid::WGS84,
            surface_height: height.max(extruded_height),
            rotation: 0.0,
            extruded_height: height.min(extruded_height),
            offset_attribute: None,
        }
    }

    /// JS constructor equivalent: `new RectangleOutlineGeometry(options)`.
    pub fn from_options(
        rectangle: Rectangle,
        ellipsoid: Option<Ellipsoid>,
        granularity: Option<f64>,
        height: Option<f64>,
        rotation: Option<f64>,
        extruded_height: Option<f64>,
        offset_attribute: Option<GeometryOffsetAttribute>,
    ) -> Self {
        let granularity = granularity.unwrap_or(CesiumMath::RADIANS_PER_DEGREE);
        let rotation = rotation.unwrap_or(0.0);
        let height = height.unwrap_or(0.0);
        let extruded_height = extruded_height.unwrap_or(height);

        if cfg!(debug_assertions) {
            assert!(
                rectangle.north >= rectangle.south,
                "options.rectangle.north must be greater than options.rectangle.south"
            );
        }

        Self {
            rectangle,
            granularity,
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84),
            surface_height: height.max(extruded_height),
            rotation,
            extruded_height: height.min(extruded_height),
            offset_attribute,
        }
    }

    /// The number of elements used to pack the object into an array.
    pub const PACKED_LENGTH: usize =
        Rectangle::PACKED_LENGTH + Ellipsoid::PACKED_LENGTH + 5;

    /// Stores the provided instance into the provided array.
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        let mut si = starting_index.unwrap_or(0);

        Rectangle::pack(&self.rectangle, array, Some(si));
        si += Rectangle::PACKED_LENGTH;

        Ellipsoid::pack(&self.ellipsoid, array, Some(si));
        si += Ellipsoid::PACKED_LENGTH;

        array[si] = self.granularity;
        si += 1;
        array[si] = self.surface_height;
        si += 1;
        array[si] = self.rotation;
        si += 1;
        array[si] = self.extruded_height;
        si += 1;
        array[si] = match &self.offset_attribute {
            Some(v) => *v as i32 as f64,
            None => -1.0,
        };
    }

    /// Retrieves an instance from a packed array.
    pub fn unpack(array: &[f64], starting_index: Option<usize>, result: Option<&mut Self>) -> Self {
        let mut si = starting_index.unwrap_or(0);

        let rectangle = Rectangle::unpack(array, Some(si));
        si += Rectangle::PACKED_LENGTH;

        let ellipsoid = Ellipsoid::unpack(array, Some(si));
        si += Ellipsoid::PACKED_LENGTH;

        let granularity = array[si];
        si += 1;
        let height = array[si];
        si += 1;
        let rotation = array[si];
        si += 1;
        let extruded_height = array[si];
        si += 1;
        let offset_attribute = array[si];

        let offset_attribute = if offset_attribute == -1.0 {
            None
        } else {
            GeometryOffsetAttribute::try_from_u32(offset_attribute as u32)
        };

        match result {
            None => Self {
                rectangle,
                granularity,
                ellipsoid,
                surface_height: height,
                rotation,
                extruded_height,
                offset_attribute,
            },
            Some(r) => {
                // Mirrors JS: granularity is intentionally not restored on the
                // result path (only via the options path).
                r.rectangle = rectangle;
                r.ellipsoid = ellipsoid;
                r.surface_height = height;
                r.rotation = rotation;
                r.extruded_height = extruded_height;
                r.offset_attribute = offset_attribute;
                r.clone()
            }
        }
    }

    /// Computes the geometric representation of an outline of a rectangle,
    /// including its vertices, indices, and a bounding sphere.
    pub fn create_geometry(rectangle_geometry: &Self) -> Option<Geometry> {
        let rectangle = &rectangle_geometry.rectangle;
        let ellipsoid = &rectangle_geometry.ellipsoid;
        let computed_options = rectangle_geometry_library::compute_options(
            rectangle,
            rectangle_geometry.granularity,
            rectangle_geometry.rotation,
            0.0,
        );

        if CesiumMath::equals_epsilon(
            rectangle.north,
            rectangle.south,
            Some(CesiumMath::EPSILON10),
            None,
        ) || CesiumMath::equals_epsilon(
            rectangle.east,
            rectangle.west,
            Some(CesiumMath::EPSILON10),
            None,
        ) {
            return None;
        }

        let surface_height = rectangle_geometry.surface_height;
        let extruded_height = rectangle_geometry.extruded_height;
        let extrude = !CesiumMath::equals_epsilon(
            surface_height,
            extruded_height,
            None,
            Some(CesiumMath::EPSILON2),
        );

        let (mut attributes, indices, bounding_sphere) = if extrude {
            let (attributes, indices) =
                construct_extruded_rectangle(rectangle_geometry, &computed_options);
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
            (attributes, indices, bounding_sphere)
        } else {
            let (mut attributes, indices) =
                construct_rectangle(rectangle_geometry, &computed_options);
            if let Some(positions) = attributes.get_mut("position") {
                PolygonPipeline::scale_to_geodetic_height(
                    Some(&mut positions.values),
                    Some(surface_height),
                    Some(ellipsoid),
                    Some(false),
                );
            }
            let bounding_sphere = BoundingSphere::from_rectangle_3d(
                Some(rectangle),
                Some(ellipsoid),
                surface_height,
                None,
            );
            (attributes, indices, bounding_sphere)
        };

        if let Some(offset_attribute) = &rectangle_geometry.offset_attribute {
            let length = attributes.get("position").map(|p| p.values.len()).unwrap_or(0);
            let size = if extrude { length / 6 } else { length / 3 };
            let apply_offset: Vec<f64> = match offset_attribute {
                GeometryOffsetAttribute::Top if extrude => {
                    let mut v = vec![0.0f64; size];
                    for item in v.iter_mut().take(size / 2) {
                        *item = 1.0;
                    }
                    v
                }
                GeometryOffsetAttribute::None => vec![0.0f64; size],
                _ => vec![1.0f64; size],
            };
            attributes.insert(
                "applyOffset".to_string(),
                GeometryAttribute::new(ComponentDatatype::UnsignedByte, 1, false, apply_offset),
            );
        }

        let offset_attribute_name = rectangle_geometry
            .offset_attribute
            .as_ref()
            .map(|_| "offsetAttribute".to_string());

        Some(Geometry::with_all(
            attributes,
            Some(indices),
            Some(PrimitiveType::Lines),
            Some(bounding_sphere),
            GeometryType::None,
            None,
            offset_attribute_name,
        ))
    }
}

/// Mirrors JS `constructRectangle`: builds the outline positions/indices for
/// a flat rectangle ring.
fn construct_rectangle(
    geometry: &RectangleOutlineGeometry,
    computed_options: &ComputedOptions,
) -> (HashMap<String, GeometryAttribute>, IndexStorage) {
    let ellipsoid = &geometry.ellipsoid;
    let height = computed_options.height;
    let width = computed_options.width;
    let north_cap = computed_options.north_cap;
    let south_cap = computed_options.south_cap;

    let mut row_height = height;
    let mut width_multiplier = 2;
    let mut size = 0usize;
    let mut corners = 4usize;
    if north_cap {
        width_multiplier -= 1;
        row_height -= 1;
        size += 1;
        corners -= 2;
    }
    if south_cap {
        width_multiplier -= 1;
        row_height -= 1;
        size += 1;
        corners -= 2;
    }
    size += width_multiplier * width + 2 * row_height - corners;

    let mut positions = vec![0.0f64; size * 3];

    let mut pos_index = 0usize;
    let mut row = 0usize;
    let mut position = Cartesian3::default();
    let mut st = Cartesian2::default();
    if north_cap {
        rectangle_geometry_library::compute_position(
            computed_options,
            ellipsoid,
            false,
            row as f64,
            0.0,
            &mut position,
            &mut st,
        );
        positions[pos_index] = position.x;
        positions[pos_index + 1] = position.y;
        positions[pos_index + 2] = position.z;
        pos_index += 3;
    } else {
        for col in 0..width {
            rectangle_geometry_library::compute_position(
                computed_options,
                ellipsoid,
                false,
                row as f64,
                col as f64,
                &mut position,
                &mut st,
            );
            positions[pos_index] = position.x;
            positions[pos_index + 1] = position.y;
            positions[pos_index + 2] = position.z;
            pos_index += 3;
        }
    }

    let col = width - 1;
    for r in 1..height {
        rectangle_geometry_library::compute_position(
            computed_options,
            ellipsoid,
            false,
            r as f64,
            col as f64,
            &mut position,
            &mut st,
        );
        positions[pos_index] = position.x;
        positions[pos_index + 1] = position.y;
        positions[pos_index + 2] = position.z;
        pos_index += 3;
    }

    row = height - 1;
    if !south_cap {
        // If southCap is true, we don't need to add any more points because
        // the south pole point was added by the iteration above.
        for c in (0..=width - 2).rev() {
            rectangle_geometry_library::compute_position(
                computed_options,
                ellipsoid,
                false,
                row as f64,
                c as f64,
                &mut position,
                &mut st,
            );
            positions[pos_index] = position.x;
            positions[pos_index + 1] = position.y;
            positions[pos_index + 2] = position.z;
            pos_index += 3;
        }
    }

    let col = 0;
    for r in (1..=height - 2).rev() {
        rectangle_geometry_library::compute_position(
            computed_options,
            ellipsoid,
            false,
            r as f64,
            col as f64,
            &mut position,
            &mut st,
        );
        positions[pos_index] = position.x;
        positions[pos_index + 1] = position.y;
        positions[pos_index + 2] = position.z;
        pos_index += 3;
    }

    let num_vertices = positions.len() / 3;
    let indices_size = num_vertices * 2;
    let mut indices = IndexDatatype::create_typed_array(num_vertices, indices_size);

    for i in 0..num_vertices - 1 {
        indices.push(i as u32);
        indices.push((i + 1) as u32);
    }
    indices.push((num_vertices - 1) as u32);
    indices.push(0);

    let mut attributes = HashMap::new();
    attributes.insert(
        "position".to_string(),
        GeometryAttribute::new(ComponentDatatype::Double, 3, false, positions),
    );

    (attributes, indices)
}

/// Mirrors JS `constructExtrudedRectangle`: duplicates the ring at the
/// extruded height and connects top/bottom with vertical lines.
fn construct_extruded_rectangle(
    rectangle_geometry: &RectangleOutlineGeometry,
    computed_options: &ComputedOptions,
) -> (HashMap<String, GeometryAttribute>, IndexStorage) {
    let max_height = rectangle_geometry.surface_height;
    let min_height = rectangle_geometry.extruded_height;
    let ellipsoid = &rectangle_geometry.ellipsoid;
    let (mut attributes, _) = construct_rectangle(rectangle_geometry, computed_options);

    let height = computed_options.height;
    let width = computed_options.width;

    let mut top_positions = attributes
        .get("position")
        .map(|p| p.values.clone())
        .unwrap_or_default();
    PolygonPipeline::scale_to_geodetic_height(
        Some(&mut top_positions),
        Some(max_height),
        Some(ellipsoid),
        Some(false),
    );
    let length = top_positions.len();
    let mut positions = vec![0.0f64; length * 2];
    positions[..length].copy_from_slice(&top_positions);
    let mut bottom_positions = attributes
        .get("position")
        .map(|p| p.values.clone())
        .unwrap_or_default();
    PolygonPipeline::scale_to_geodetic_height(
        Some(&mut bottom_positions),
        Some(min_height),
        Some(ellipsoid),
        None,
    );
    positions[length..].copy_from_slice(&bottom_positions);
    if let Some(position_attribute) = attributes.get_mut("position") {
        position_attribute.values = positions.clone();
    }

    let north_cap = computed_options.north_cap;
    let south_cap = computed_options.south_cap;
    let mut corners = 4usize;
    if north_cap {
        corners -= 1;
    }
    if south_cap {
        corners -= 1;
    }

    let indices_size = (positions.len() / 3 + corners) * 2;
    let mut indices = IndexDatatype::create_typed_array(positions.len() / 3, indices_size);
    let length = positions.len() / 6;
    for i in 0..length - 1 {
        indices.push(i as u32);
        indices.push((i + 1) as u32);
        indices.push((i + length) as u32);
        indices.push((i + length + 1) as u32);
    }
    indices.push((length - 1) as u32);
    indices.push(0);
    indices.push((length + length - 1) as u32);
    indices.push(length as u32);

    indices.push(0);
    indices.push(length as u32);

    let bottom_corner;
    if north_cap {
        bottom_corner = height - 1;
    } else {
        let top_right_corner = width - 1;
        indices.push(top_right_corner as u32);
        indices.push((top_right_corner + length) as u32);
        bottom_corner = width + height - 2;
    }

    indices.push(bottom_corner as u32);
    indices.push((bottom_corner + length) as u32);

    if !south_cap {
        let bottom_left_corner = width + bottom_corner - 1;
        indices.push(bottom_left_corner as u32);
        indices.push((bottom_left_corner + length) as u32);
    }

    (attributes, indices)
}
