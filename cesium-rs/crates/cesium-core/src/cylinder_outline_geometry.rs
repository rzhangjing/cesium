//! Ported from `packages/engine/Source/Core/CylinderOutlineGeometry.js`.
//!
//! A description of the outline of a cylinder.

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
use crate::primitive_type::PrimitiveType;

/// A description of the outline of a cylinder.
#[derive(Debug, Clone)]
pub struct CylinderOutlineGeometry {
    length: f64,
    top_radius: f64,
    bottom_radius: f64,
    slices: usize,
    number_of_vertical_lines: usize,
    offset_attribute: Option<GeometryOffsetAttribute>,
}

impl CylinderOutlineGeometry {
    /// Creates a new `CylinderOutlineGeometry`.
    pub fn new(
        length: f64,
        top_radius: f64,
        bottom_radius: f64,
        slices: Option<usize>,
        number_of_vertical_lines: Option<usize>,
        offset_attribute: Option<GeometryOffsetAttribute>,
    ) -> Self {
        let sl = slices.unwrap_or(128);
        let nvl = number_of_vertical_lines.unwrap_or(16).max(0);
        Self {
            length,
            top_radius,
            bottom_radius,
            slices: sl,
            number_of_vertical_lines: nvl,
            offset_attribute,
        }
    }

    /// The number of `f64` elements used to pack/unpack.
    pub const PACKED_LENGTH: usize = 6;

    /// Packs into `array`.
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        let mut si = starting_index.unwrap_or(0);
        array[si] = self.length; si += 1;
        array[si] = self.top_radius; si += 1;
        array[si] = self.bottom_radius; si += 1;
        array[si] = self.slices as f64; si += 1;
        array[si] = self.number_of_vertical_lines as f64; si += 1;
        array[si] = self.offset_attribute.map_or(-1.0, |o| o as u32 as f64);
    }

    /// Unpacks from `array`.
    pub fn unpack(array: &[f64], starting_index: Option<usize>) -> Self {
        let mut si = starting_index.unwrap_or(0);
        let length = array[si]; si += 1;
        let top_radius = array[si]; si += 1;
        let bottom_radius = array[si]; si += 1;
        let slices = array[si] as usize; si += 1;
        let number_of_vertical_lines = array[si] as usize; si += 1;
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
            number_of_vertical_lines,
            offset_attribute: offset,
        }
    }

    /// Computes the geometric representation of an outline of a cylinder.
    pub fn create_geometry(&self) -> Option<Geometry> {
        let length = self.length;
        let top_radius = self.top_radius;
        let bottom_radius = self.bottom_radius;
        let slices = self.slices;
        let number_of_vertical_lines = self.number_of_vertical_lines;

        if length <= 0.0
            || top_radius < 0.0
            || bottom_radius < 0.0
            || (top_radius == 0.0 && bottom_radius == 0.0)
        {
            return None;
        }

        let _num_vertices = slices * 2;
        let positions = cylinder_geometry_library::compute_positions(
            length, top_radius, bottom_radius, slices, false,
        );

        let mut num_indices = slices * 2;
        let mut num_side: usize = 0;
        if number_of_vertical_lines > 0 {
            let num_side_lines = number_of_vertical_lines.min(slices);
            num_side = (slices as f64 / num_side_lines as f64).round() as usize;
            num_indices += num_side_lines;
        }

        let mut indices: Vec<u16> = Vec::with_capacity(num_indices * 2);

        // Top and bottom rings
        for i in 0..(slices - 1) {
            indices.push(i as u16);
            indices.push((i + 1) as u16);
            indices.push((i + slices) as u16);
            indices.push((i + 1 + slices) as u16);
        }
        // Close the ring
        indices.push((slices - 1) as u16);
        indices.push(0);
        indices.push((slices + slices - 1) as u16);
        indices.push(slices as u16);

        // Vertical lines
        if number_of_vertical_lines > 0 && num_side > 0 {
            let mut i = 0;
            while i < slices {
                indices.push(i as u16);
                indices.push((i + slices) as u16);
                i += num_side;
            }
        }

        let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();
        attributes.insert(
            "position".to_string(),
            GeometryAttribute::new(ComponentDatatype::Double, 3, false, positions.clone()),
        );

        let radius = Cartesian2::new(length * 0.5, bottom_radius.max(top_radius));
        let bs = BoundingSphere::new(Cartesian3::ZERO, Cartesian2::magnitude(&radius));

        if let Some(offset) = self.offset_attribute {
            let num_verts = positions.len() / 3;
            let offset_value = if offset == GeometryOffsetAttribute::None {
                0.0
            } else {
                1.0
            };
            attributes.insert(
                "applyOffset".to_string(),
                GeometryAttribute::new(
                    ComponentDatatype::UnsignedByte,
                    1,
                    false,
                    vec![offset_value; num_verts],
                ),
            );
        }

        Some(Geometry::new(
            attributes,
            Some(IndexStorage::U16(indices)),
            Some(PrimitiveType::Lines),
            Some(bs),
        ))
    }
}
