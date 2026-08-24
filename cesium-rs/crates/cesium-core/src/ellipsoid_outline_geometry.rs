//! Ported from `packages/engine/Source/Core/EllipsoidOutlineGeometry.js`.
//!
//! A description of the outline of an ellipsoid centered at the origin.

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::component_datatype::ComponentDatatype;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_offset_attribute::GeometryOffsetAttribute;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::math::CesiumMath;
use crate::primitive_type::PrimitiveType;

/// A description of the outline of an ellipsoid centered at the origin.
#[derive(Debug, Clone)]
pub struct EllipsoidOutlineGeometry {
    radii: Cartesian3,
    inner_radii: Cartesian3,
    minimum_clock: f64,
    maximum_clock: f64,
    minimum_cone: f64,
    maximum_cone: f64,
    stack_partitions: i32,
    slice_partitions: i32,
    subdivisions: i32,
    offset_attribute: Option<GeometryOffsetAttribute>,
}

impl Default for EllipsoidOutlineGeometry {
    fn default() -> Self {
        Self {
            radii: Cartesian3::new(1.0, 1.0, 1.0),
            inner_radii: Cartesian3::new(1.0, 1.0, 1.0),
            minimum_clock: 0.0,
            maximum_clock: CesiumMath::TWO_PI,
            minimum_cone: 0.0,
            maximum_cone: CesiumMath::PI,
            stack_partitions: 10,
            slice_partitions: 8,
            subdivisions: 128,
            offset_attribute: None,
        }
    }
}

impl EllipsoidOutlineGeometry {
    /// Creates a new `EllipsoidOutlineGeometry`.
    ///
    /// Mirrors the JS constructor: partition counts are rounded via
    /// `Math.round` (here the caller passes the already-rounded value, see
    /// [`Self::new_with_rounding`] for the faithful variant).
    ///
    /// # Panics (debug)
    /// Mirrors the JS `DeveloperError` checks behind `debug_assertions`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        radii: Option<Cartesian3>,
        inner_radii: Option<Cartesian3>,
        minimum_clock: Option<f64>,
        maximum_clock: Option<f64>,
        minimum_cone: Option<f64>,
        maximum_cone: Option<f64>,
        stack_partitions: Option<f64>,
        slice_partitions: Option<f64>,
        subdivisions: Option<f64>,
        offset_attribute: Option<GeometryOffsetAttribute>,
    ) -> Self {
        let radii = radii.unwrap_or(Cartesian3::new(1.0, 1.0, 1.0));
        let inner_radii = inner_radii.unwrap_or(radii);
        // JS `Math.round` semantics: round half toward +infinity.
        let stack_partitions = js_round(stack_partitions.unwrap_or(10.0));
        let slice_partitions = js_round(slice_partitions.unwrap_or(8.0));
        let subdivisions = js_round(subdivisions.unwrap_or(128.0));

        if cfg!(debug_assertions) {
            if stack_partitions < 1 {
                crate::developer_error::throw_developer_error(
                    "options.stackPartitions cannot be less than 1",
                );
            }
            if slice_partitions < 0 {
                crate::developer_error::throw_developer_error(
                    "options.slicePartitions cannot be less than 0",
                );
            }
            if subdivisions < 0 {
                crate::developer_error::throw_developer_error(
                    "options.subdivisions must be greater than or equal to zero.",
                );
            }
            if let Some(o) = offset_attribute {
                if o == GeometryOffsetAttribute::Top {
                    crate::developer_error::throw_developer_error(
                        "GeometryOffsetAttribute.TOP is not a supported options.offsetAttribute for this geometry.",
                    );
                }
            }
        }

        Self {
            radii,
            inner_radii,
            minimum_clock: minimum_clock.unwrap_or(0.0),
            maximum_clock: maximum_clock.unwrap_or(CesiumMath::TWO_PI),
            minimum_cone: minimum_cone.unwrap_or(0.0),
            maximum_cone: maximum_cone.unwrap_or(CesiumMath::PI),
            stack_partitions,
            slice_partitions,
            subdivisions,
            offset_attribute,
        }
    }

    /// The number of `f64` elements used to pack/unpack.
    pub const PACKED_LENGTH: usize = 2 * Cartesian3::PACKED_LENGTH + 8;

    /// Packs the ellipsoid outline geometry into `array` starting at
    /// `starting_index`.
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        let mut si = starting_index.unwrap_or(0);
        Cartesian3::pack(&self.radii, array, Some(si));
        si += Cartesian3::PACKED_LENGTH;
        Cartesian3::pack(&self.inner_radii, array, Some(si));
        si += Cartesian3::PACKED_LENGTH;
        array[si] = self.minimum_clock;
        si += 1;
        array[si] = self.maximum_clock;
        si += 1;
        array[si] = self.minimum_cone;
        si += 1;
        array[si] = self.maximum_cone;
        si += 1;
        array[si] = self.stack_partitions as f64;
        si += 1;
        array[si] = self.slice_partitions as f64;
        si += 1;
        array[si] = self.subdivisions as f64;
        si += 1;
        array[si] = self.offset_attribute.map_or(-1.0, |o| o as u32 as f64);
    }

    /// Unpacks an `EllipsoidOutlineGeometry` from `array`.
    ///
    /// Mirrors the JS semantics: when `result` is `None` the values run
    /// through the constructor; when `result` is provided the fields are
    /// assigned verbatim.
    pub fn unpack(
        array: &[f64],
        starting_index: Option<usize>,
        result: Option<&mut Self>,
    ) -> Self {
        let mut si = starting_index.unwrap_or(0);
        let radii = Cartesian3::unpack_new(array, Some(si));
        si += Cartesian3::PACKED_LENGTH;
        let inner_radii = Cartesian3::unpack_new(array, Some(si));
        si += Cartesian3::PACKED_LENGTH;
        let minimum_clock = array[si];
        si += 1;
        let maximum_clock = array[si];
        si += 1;
        let minimum_cone = array[si];
        si += 1;
        let maximum_cone = array[si];
        si += 1;
        let stack_partitions = array[si];
        si += 1;
        let slice_partitions = array[si];
        si += 1;
        let subdivisions = array[si];
        si += 1;
        let offset_raw = array[si];
        let offset_attribute = if offset_raw == -1.0 {
            None
        } else {
            GeometryOffsetAttribute::try_from_u32(offset_raw as u32)
        };

        match result {
            None => Self::new(
                Some(radii),
                Some(inner_radii),
                Some(minimum_clock),
                Some(maximum_clock),
                Some(minimum_cone),
                Some(maximum_cone),
                Some(stack_partitions),
                Some(slice_partitions),
                Some(subdivisions),
                offset_attribute,
            ),
            Some(r) => {
                r.radii = radii;
                r.inner_radii = inner_radii;
                r.minimum_clock = minimum_clock;
                r.maximum_clock = maximum_clock;
                r.minimum_cone = minimum_cone;
                r.maximum_cone = maximum_cone;
                r.stack_partitions = stack_partitions as i32;
                r.slice_partitions = slice_partitions as i32;
                r.subdivisions = subdivisions as i32;
                r.offset_attribute = offset_attribute;
                r.clone()
            }
        }
    }

    /// The radii of the ellipsoid in the x, y, and z directions.
    pub fn radii(&self) -> &Cartesian3 {
        &self.radii
    }

    /// The inner radii of the ellipsoid in the x, y, and z directions.
    pub fn inner_radii(&self) -> &Cartesian3 {
        &self.inner_radii
    }

    /// The count of stacks for the ellipsoid.
    pub fn stack_partitions(&self) -> i32 {
        self.stack_partitions
    }

    /// The count of slices for the ellipsoid.
    pub fn slice_partitions(&self) -> i32 {
        self.slice_partitions
    }

    /// The number of points per line.
    pub fn subdivisions(&self) -> i32 {
        self.subdivisions
    }

    /// Computes the geometric representation of an outline of an ellipsoid,
    /// including its vertices, indices, and a bounding sphere.
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
        let subdivisions = self.subdivisions as usize;
        let ellipsoid = Ellipsoid::from_cartesian3(Some(radii));

        // Add an extra slice and stack to remain consistent with
        // EllipsoidGeometry.
        let mut slice_partitions =
            ((self.slice_partitions + 1) as f64 * (maximum_clock - minimum_clock).abs()
                / CesiumMath::TWO_PI)
                .round() as usize;
        let mut stack_partitions =
            ((self.stack_partitions + 1) as f64 * (maximum_cone - minimum_cone).abs()
                / CesiumMath::PI)
                .round() as usize;

        if slice_partitions < 2 {
            slice_partitions = 2;
        }
        if stack_partitions < 2 {
            stack_partitions = 2;
        }

        let mut extra_indices = 0usize;
        let mut vertex_multiplier = 1usize;
        let has_inner_surface = inner_radii.x != radii.x
            || inner_radii.y != radii.y
            || inner_radii.z != radii.z;
        let mut is_top_open = false;
        let mut is_bot_open = false;
        if has_inner_surface {
            vertex_multiplier = 2;
            // Add 2x slicePartitions to connect the top/bottom of the outer
            // to the top/bottom of the inner.
            if minimum_cone > 0.0 {
                is_top_open = true;
                extra_indices += slice_partitions;
            }
            if maximum_cone < CesiumMath::PI {
                is_bot_open = true;
                extra_indices += slice_partitions;
            }
        }

        let vertex_count =
            subdivisions * vertex_multiplier * (stack_partitions + slice_partitions);
        let mut positions = vec![0.0f64; vertex_count * 3];

        // Multiply by two because two points define each line segment.
        let num_indices = 2 * (vertex_count + extra_indices
            - (slice_partitions + stack_partitions) * vertex_multiplier);
        let mut indices = IndexDatatype::create_typed_array(vertex_count, num_indices);

        let mut index = 0usize;

        // Calculate sin/cos phi (stacks).
        let mut sin_phi = vec![0.0f64; stack_partitions];
        let mut cos_phi = vec![0.0f64; stack_partitions];
        for i in 0..stack_partitions {
            let phi = minimum_cone
                + (i as f64 * (maximum_cone - minimum_cone)) / (stack_partitions - 1) as f64;
            sin_phi[i] = phi.sin();
            cos_phi[i] = phi.cos();
        }

        // Calculate sin/cos theta (subdivisions).
        let mut sin_theta = vec![0.0f64; subdivisions];
        let mut cos_theta = vec![0.0f64; subdivisions];
        for i in 0..subdivisions {
            let theta = minimum_clock
                + (i as f64 * (maximum_clock - minimum_clock)) / (subdivisions - 1) as f64;
            sin_theta[i] = theta.sin();
            cos_theta[i] = theta.cos();
        }

        // Calculate the latitude lines on the outer surface.
        for i in 0..stack_partitions {
            for j in 0..subdivisions {
                positions[index] = radii.x * sin_phi[i] * cos_theta[j];
                positions[index + 1] = radii.y * sin_phi[i] * sin_theta[j];
                positions[index + 2] = radii.z * cos_phi[i];
                index += 3;
            }
        }

        // Calculate the latitude lines on the inner surface.
        if has_inner_surface {
            for i in 0..stack_partitions {
                for j in 0..subdivisions {
                    positions[index] = inner_radii.x * sin_phi[i] * cos_theta[j];
                    positions[index + 1] = inner_radii.y * sin_phi[i] * sin_theta[j];
                    positions[index + 2] = inner_radii.z * cos_phi[i];
                    index += 3;
                }
            }
        }

        // Recalculate sin/cos phi over the subdivisions.
        sin_phi.resize(subdivisions, 0.0);
        cos_phi.resize(subdivisions, 0.0);
        for i in 0..subdivisions {
            let phi = minimum_cone
                + (i as f64 * (maximum_cone - minimum_cone)) / (subdivisions - 1) as f64;
            sin_phi[i] = phi.sin();
            cos_phi[i] = phi.cos();
        }

        // Calculate sin/cos theta for each slice partition.
        sin_theta.resize(slice_partitions, 0.0);
        cos_theta.resize(slice_partitions, 0.0);
        for i in 0..slice_partitions {
            let theta = minimum_clock
                + (i as f64 * (maximum_clock - minimum_clock)) / (slice_partitions - 1) as f64;
            sin_theta[i] = theta.sin();
            cos_theta[i] = theta.cos();
        }

        // Calculate the longitude lines on the outer surface.
        for i in 0..subdivisions {
            for j in 0..slice_partitions {
                positions[index] = radii.x * sin_phi[i] * cos_theta[j];
                positions[index + 1] = radii.y * sin_phi[i] * sin_theta[j];
                positions[index + 2] = radii.z * cos_phi[i];
                index += 3;
            }
        }

        // Calculate the longitude lines on the inner surface.
        if has_inner_surface {
            for i in 0..subdivisions {
                for j in 0..slice_partitions {
                    positions[index] = inner_radii.x * sin_phi[i] * cos_theta[j];
                    positions[index + 1] = inner_radii.y * sin_phi[i] * sin_theta[j];
                    positions[index + 2] = inner_radii.z * cos_phi[i];
                    index += 3;
                }
            }
        }

        // Create indices for the latitude lines.
        index = 0;
        for i in 0..stack_partitions * vertex_multiplier {
            let top_offset = i * subdivisions;
            for j in 0..subdivisions - 1 {
                write_index(&mut indices, index, (top_offset + j) as u32);
                write_index(&mut indices, index + 1, (top_offset + j + 1) as u32);
                index += 2;
            }
        }

        // Create indices for the outer longitude lines.
        let mut offset = stack_partitions * subdivisions * vertex_multiplier;
        for i in 0..slice_partitions {
            for j in 0..subdivisions - 1 {
                write_index(&mut indices, index, (offset + i + j * slice_partitions) as u32);
                write_index(
                    &mut indices,
                    index + 1,
                    (offset + i + (j + 1) * slice_partitions) as u32,
                );
                index += 2;
            }
        }

        // Create indices for the inner longitude lines.
        if has_inner_surface {
            offset = stack_partitions * subdivisions * vertex_multiplier
                + slice_partitions * subdivisions;
            for i in 0..slice_partitions {
                for j in 0..subdivisions - 1 {
                    write_index(
                        &mut indices,
                        index,
                        (offset + i + j * slice_partitions) as u32,
                    );
                    write_index(
                        &mut indices,
                        index + 1,
                        (offset + i + (j + 1) * slice_partitions) as u32,
                    );
                    index += 2;
                }
            }
        }

        if has_inner_surface {
            let mut outer_offset = stack_partitions * subdivisions * vertex_multiplier;
            let mut inner_offset = outer_offset + subdivisions * slice_partitions;
            if is_top_open {
                // Draw lines from the top of the inner surface to the top of
                // the outer surface.
                for i in 0..slice_partitions {
                    write_index(&mut indices, index, (outer_offset + i) as u32);
                    write_index(&mut indices, index + 1, (inner_offset + i) as u32);
                    index += 2;
                }
            }

            if is_bot_open {
                // Draw lines from the bottom of the inner surface to the
                // bottom of the outer surface.
                outer_offset += subdivisions * slice_partitions - slice_partitions;
                inner_offset += subdivisions * slice_partitions - slice_partitions;
                for i in 0..slice_partitions {
                    write_index(&mut indices, index, (outer_offset + i) as u32);
                    write_index(&mut indices, index + 1, (inner_offset + i) as u32);
                    index += 2;
                }
            }
        }

        let mut attributes: HashMap<String, GeometryAttribute> = HashMap::new();
        attributes.insert(
            "position".to_string(),
            GeometryAttribute::new(ComponentDatatype::Double, 3, false, positions.clone()),
        );

        if let Some(offset_attribute) = self.offset_attribute {
            let length = positions.len();
            let offset_value = if offset_attribute == GeometryOffsetAttribute::None {
                0
            } else {
                1
            };
            let apply_offset = vec![offset_value as f64; length / 3];
            attributes.insert(
                "applyOffset".to_string(),
                GeometryAttribute::new(ComponentDatatype::UnsignedByte, 1, false, apply_offset),
            );
        }

        Some(Geometry::with_all(
            attributes,
            Some(indices),
            Some(PrimitiveType::Lines),
            Some(BoundingSphere::from_ellipsoid(&ellipsoid, None)),
            crate::geometry_type::GeometryType::None,
            None,
            None,
        ))
    }
}

/// Mirrors JS `Math.round` (round half toward +infinity), returning `i32`.
fn js_round(value: f64) -> i32 {
    (value + 0.5).floor() as i32
}

fn write_index(storage: &mut IndexStorage, index: usize, value: u32) {
    match storage {
        IndexStorage::U16(v) => v[index] = value as u16,
        IndexStorage::U32(v) => v[index] = value,
    }
}
