//! Ported from `packages/engine/Source/Core/SphereGeometry.js`.
//!
//! A sphere centered at the origin — a thin wrapper around [`EllipsoidGeometry`].

use crate::cartesian3::Cartesian3;
use crate::geometry::Geometry;
use crate::geometry_offset_attribute::GeometryOffsetAttribute;
use crate::vertex_format::VertexFormat;

use crate::ellipsoid_geometry::EllipsoidGeometry;

/// A description of a sphere centered at the origin.
#[derive(Debug, Clone)]
pub struct SphereGeometry {
    ellipsoid_geometry: EllipsoidGeometry,
}

impl SphereGeometry {
    /// Creates a new `SphereGeometry`.
    pub fn new(
        radius: Option<f64>,
        stack_partitions: Option<i32>,
        slice_partitions: Option<i32>,
        vertex_format: Option<VertexFormat>,
    ) -> Self {
        let r = radius.unwrap_or(1.0);
        let radii = Cartesian3::new(r, r, r);
        Self {
            ellipsoid_geometry: EllipsoidGeometry::new(
                Some(radii),
                None,
                None,
                None,
                None,
                None,
                stack_partitions,
                slice_partitions,
                vertex_format,
                None,
            ),
        }
    }

    /// Creates a `SphereGeometry` with an offset attribute.
    pub fn with_offset(
        radius: Option<f64>,
        stack_partitions: Option<i32>,
        slice_partitions: Option<i32>,
        vertex_format: Option<VertexFormat>,
        offset_attribute: Option<GeometryOffsetAttribute>,
    ) -> Self {
        let r = radius.unwrap_or(1.0);
        let radii = Cartesian3::new(r, r, r);
        Self {
            ellipsoid_geometry: EllipsoidGeometry::new(
                Some(radii),
                None,
                None,
                None,
                None,
                None,
                stack_partitions,
                slice_partitions,
                vertex_format,
                offset_attribute,
            ),
        }
    }

    /// The number of `f64` elements used to pack/unpack.
    pub const PACKED_LENGTH: usize = EllipsoidGeometry::PACKED_LENGTH;

    /// Packs into `array`.
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        self.ellipsoid_geometry.pack(array, starting_index);
    }

    /// Unpacks from `array`.
    pub fn unpack(array: &[f64], starting_index: Option<usize>) -> Self {
        let eg = EllipsoidGeometry::unpack(array, starting_index);
        Self { ellipsoid_geometry: eg }
    }

    /// Computes the geometric representation of the sphere.
    pub fn create_geometry(&self) -> Option<Geometry> {
        self.ellipsoid_geometry.create_geometry()
    }
}
