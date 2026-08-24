//! Ported from `packages/engine/Source/Core/SphereOutlineGeometry.js`.
//!
//! A description of the outline of a sphere.
//!
//! `SphereOutlineGeometry` is a thin wrapper around `EllipsoidOutlineGeometry`
//! where all radii are equal; all operations delegate to the inner ellipsoid
//! outline geometry, mirroring the JS implementation.

use crate::cartesian3::Cartesian3;
use crate::ellipsoid_outline_geometry::EllipsoidOutlineGeometry;
use crate::geometry::Geometry;

/// A description of the outline of a sphere.
#[derive(Debug, Clone)]
pub struct SphereOutlineGeometry {
    ellipsoid_geometry: EllipsoidOutlineGeometry,
}

impl SphereOutlineGeometry {
    /// Creates a new `SphereOutlineGeometry`.
    pub fn new(
        radius: Option<f64>,
        stack_partitions: Option<f64>,
        slice_partitions: Option<f64>,
        subdivisions: Option<f64>,
    ) -> Self {
        let radius = radius.unwrap_or(1.0);
        let radii = Cartesian3::new(radius, radius, radius);
        let ellipsoid_geometry = EllipsoidOutlineGeometry::new(
            Some(radii),
            None,
            None,
            None,
            None,
            None,
            stack_partitions,
            slice_partitions,
            subdivisions,
            None,
        );
        Self { ellipsoid_geometry }
    }

    /// Access to the inner ellipsoid outline geometry (mirrors JS
    /// `_ellipsoidGeometry`).
    pub fn ellipsoid_geometry(&self) -> &EllipsoidOutlineGeometry {
        &self.ellipsoid_geometry
    }

    /// The sphere radius (mirrors JS `_ellipsoidGeometry._radii.x`).
    pub fn radius(&self) -> f64 {
        self.ellipsoid_geometry.radii().x
    }

    /// The number of `f64` elements needed to pack/unpack a
    /// `SphereOutlineGeometry`.
    pub const PACKED_LENGTH: usize = EllipsoidOutlineGeometry::PACKED_LENGTH;

    /// Packs the sphere outline geometry into `array` starting at
    /// `starting_index`.
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        self.ellipsoid_geometry.pack(array, starting_index);
    }

    /// Unpacks a `SphereOutlineGeometry` from `array`.
    ///
    /// Mirrors the JS semantics faithfully, including the quirk that the
    /// `result` path rebuilds the inner ellipsoid from radii and partition
    /// counts only (clock/cone ranges reset to defaults).
    pub fn unpack(
        array: &[f64],
        starting_index: Option<usize>,
        result: Option<&mut Self>,
    ) -> Self {
        let ellipsoid_geometry = EllipsoidOutlineGeometry::unpack(array, starting_index, None);
        let stack_partitions = ellipsoid_geometry.stack_partitions() as f64;
        let slice_partitions = ellipsoid_geometry.slice_partitions() as f64;
        let subdivisions = ellipsoid_geometry.subdivisions() as f64;

        match result {
            None => {
                let radius = ellipsoid_geometry.radii().x;
                Self::new(
                    Some(radius),
                    Some(stack_partitions),
                    Some(slice_partitions),
                    Some(subdivisions),
                )
            }
            Some(r) => {
                // JS rebuilds result._ellipsoidGeometry with radii and
                // partition counts only; clock/cone fall back to defaults.
                r.ellipsoid_geometry = EllipsoidOutlineGeometry::new(
                    Some(*ellipsoid_geometry.radii()),
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some(stack_partitions),
                    Some(slice_partitions),
                    Some(subdivisions),
                    None,
                );
                r.clone()
            }
        }
    }

    /// Computes the geometric representation of an outline of a sphere,
    /// including its vertices, indices, and a bounding sphere.
    pub fn create_geometry(sphere_geometry: &Self) -> Option<Geometry> {
        sphere_geometry.ellipsoid_geometry.create_geometry()
    }
}
