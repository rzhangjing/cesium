//! Ported from `packages/engine/Source/Core/BoxOutlineGeometry.js`.
//!
//! Outline of a cube centered at the origin.

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::axis_aligned_bounding_box::AxisAlignedBoundingBox;
use crate::cartesian3::Cartesian3;
use crate::component_datatype::ComponentDatatype;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_offset_attribute::GeometryOffsetAttribute;
use crate::index_datatype::IndexStorage;
use crate::primitive_type::PrimitiveType;

/// A description of the outline of a cube centered at the origin.
#[derive(Debug, Clone)]
pub struct BoxOutlineGeometry {
    minimum: Cartesian3,
    maximum: Cartesian3,
    offset_attribute: Option<GeometryOffsetAttribute>,
}

impl BoxOutlineGeometry {
    /// Creates a new `BoxOutlineGeometry` from min/max corners.
    pub fn new(
        minimum: &Cartesian3,
        maximum: &Cartesian3,
        offset_attribute: Option<GeometryOffsetAttribute>,
    ) -> Self {
        Self {
            minimum: *minimum,
            maximum: *maximum,
            offset_attribute,
        }
    }

    /// Creates an outline cube from its dimensions (width, depth, height).
    pub fn from_dimensions(
        dimensions: &Cartesian3,
        offset_attribute: Option<GeometryOffsetAttribute>,
    ) -> Self {
        let half = Cartesian3::multiply_by_scalar_new(dimensions, 0.5);
        let neg_half = Cartesian3::negate_new(&half);
        Self::new(&neg_half, &half, offset_attribute)
    }

    /// Creates an outline cube that encloses an axis-aligned bounding box.
    ///
    /// Port of `BoxOutlineGeometry.fromAxisAlignedBoundingBox`.
    pub fn from_axis_aligned_bounding_box(bounding_box: &AxisAlignedBoundingBox) -> Self {
        Self::new(&bounding_box.minimum, &bounding_box.maximum, None)
    }

    /// The number of `f64` elements needed to pack/unpack.
    pub const PACKED_LENGTH: usize = 2 * Cartesian3::PACKED_LENGTH + 1;

    /// Packs into `array` starting at `starting_index`.
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        let si = starting_index.unwrap_or(0);
        Cartesian3::pack(&self.minimum, array, Some(si));
        Cartesian3::pack(&self.maximum, array, Some(si + Cartesian3::PACKED_LENGTH));
        array[si + Cartesian3::PACKED_LENGTH * 2] =
            self.offset_attribute.map_or(-1.0, |o| o as u32 as f64);
    }

    /// Unpacks from `array`.
    pub fn unpack(
        array: &[f64],
        starting_index: Option<usize>,
        result: Option<&mut Self>,
    ) -> Self {
        let si = starting_index.unwrap_or(0);
        let min = Cartesian3::unpack_new(array, Some(si));
        let max = Cartesian3::unpack_new(array, Some(si + Cartesian3::PACKED_LENGTH));
        let offset_raw = array[si + Cartesian3::PACKED_LENGTH * 2];
        let offset = if offset_raw == -1.0 {
            None
        } else {
            GeometryOffsetAttribute::try_from_u32(offset_raw as u32)
        };

        if let Some(r) = result {
            r.minimum = min;
            r.maximum = max;
            r.offset_attribute = offset;
            r.clone()
        } else {
            Self {
                minimum: min,
                maximum: max,
                offset_attribute: offset,
            }
        }
    }

    /// Computes the geometric representation of the box outline.
    pub fn create_geometry(&self) -> Option<Geometry> {
        let min = &self.minimum;
        let max = &self.maximum;

        if Cartesian3::equals(Some(min), Some(max)) {
            return None;
        }

        let positions: Vec<f64> = vec![
            // bottom face (z = min.z)
            min.x, min.y, min.z,  max.x, min.y, min.z,
            max.x, max.y, min.z,  min.x, max.y, min.z,
            // top face (z = max.z)
            min.x, min.y, max.z,  max.x, min.y, max.z,
            max.x, max.y, max.z,  min.x, max.y, max.z,
        ];

        let indices: Vec<u16> = vec![
            // top
            4, 5, 5, 6, 6, 7, 7, 4,
            // bottom
            0, 1, 1, 2, 2, 3, 3, 0,
            // left
            0, 4, 1, 5,
            // right
            2, 6, 3, 7,
        ];

        let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();
        attributes.insert(
            "position".to_string(),
            GeometryAttribute::new(ComponentDatatype::Double, 3, false, positions),
        );

        let diff = Cartesian3::subtract_new(max, min);
        let radius = Cartesian3::magnitude(&diff) * 0.5;

        if let Some(offset) = self.offset_attribute {
            let num_verts = 8;
            let offset_value = if offset == GeometryOffsetAttribute::None {
                0.0
            } else {
                1.0
            };
            let apply_offset = vec![offset_value; num_verts];
            attributes.insert(
                "applyOffset".to_string(),
                GeometryAttribute::new(ComponentDatatype::UnsignedByte, 1, false, apply_offset),
            );
        }

        Some(Geometry::new(
            attributes,
            Some(IndexStorage::U16(indices)),
            Some(PrimitiveType::Lines),
            Some(BoundingSphere::new(Cartesian3::ZERO, radius)),
        ))
    }
}
