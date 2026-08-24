//! Ported from `packages/engine/Source/Core/FrustumOutlineGeometry.js`.
//!
//! A description of the outline of a frustum with the given origin and
//! orientation.

use std::collections::HashMap;

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::cartesian4::Cartesian4;
use crate::component_datatype::ComponentDatatype;
use crate::frustum_geometry::{FrustumGeometry, FrustumKind};
use crate::geometry::Geometry;
use crate::geometry_attribute::GeometryAttribute;
use crate::geometry_type::GeometryType;
use crate::index_datatype::{IndexDatatype, IndexStorage};
use crate::orthographic_frustum::OrthographicFrustum;
use crate::perspective_frustum::PerspectiveFrustum;
use crate::primitive_type::PrimitiveType;
use crate::quaternion::Quaternion;

const PERSPECTIVE: u32 = 0;
#[allow(dead_code)]
const ORTHOGRAPHIC: u32 = 1;

/// A description of the outline of a frustum with the given origin and
/// orientation.
///
/// DEVIATION: JS `packedLength` is an instance property computed in the
/// constructor; Rust exposes it as `packed_length(&self)`.
#[derive(Debug, Clone)]
pub struct FrustumOutlineGeometry {
    frustum_type: u32,
    frustum: FrustumKind,
    origin: Cartesian3,
    orientation: Quaternion,
    draw_near_plane: bool,
}

impl FrustumOutlineGeometry {
    /// Creates a FrustumOutlineGeometry from a perspective frustum described
    /// by `near`, `far`, `fov` and `aspect_ratio`.
    ///
    /// Retained for spec compatibility; the JS constructor takes an options
    /// object with an explicit frustum instance (see
    /// [`FrustumOutlineGeometry::from_frustum`]). `orientation` is a
    /// quaternion stored as a `Cartesian4` (x, y, z, w).
    pub fn new(
        origin: Cartesian3,
        orientation: Cartesian4,
        near: f64,
        far: f64,
        fov: f64,
        aspect_ratio: f64,
    ) -> Self {
        let mut frustum = PerspectiveFrustum::new();
        frustum.fov = Some(fov);
        frustum.aspect_ratio = Some(aspect_ratio);
        frustum.near = near;
        frustum.far = far;

        Self {
            frustum_type: PERSPECTIVE,
            frustum: FrustumKind::Perspective(frustum),
            origin,
            orientation: Quaternion::new(orientation.x, orientation.y, orientation.z, orientation.w),
            draw_near_plane: true,
        }
    }

    /// JS constructor equivalent: `new FrustumOutlineGeometry(options)`.
    pub fn from_frustum(
        frustum: FrustumKind,
        origin: Cartesian3,
        orientation: Quaternion,
        draw_near_plane: Option<bool>,
    ) -> Self {
        let frustum_type = match &frustum {
            FrustumKind::Perspective(_) => PERSPECTIVE,
            FrustumKind::Orthographic(_) => ORTHOGRAPHIC,
        };
        Self {
            frustum_type,
            frustum,
            origin,
            orientation,
            draw_near_plane: draw_near_plane.unwrap_or(true),
        }
    }

    /// The number of elements used to pack the object into an array.
    pub fn packed_length(&self) -> usize {
        2 + match &self.frustum {
            FrustumKind::Perspective(_) => PerspectiveFrustum::PACKED_LENGTH,
            FrustumKind::Orthographic(_) => OrthographicFrustum::PACKED_LENGTH,
        } + Cartesian3::PACKED_LENGTH
            + Quaternion::PACKED_LENGTH
    }

    /// Stores the provided instance into the provided array.
    pub fn pack(&self, array: &mut [f64], starting_index: Option<usize>) {
        let mut i = starting_index.unwrap_or(0);

        array[i] = self.frustum_type as f64;
        i += 1;

        match &self.frustum {
            FrustumKind::Perspective(p) => {
                PerspectiveFrustum::pack(p, array, i);
                i += PerspectiveFrustum::PACKED_LENGTH;
            }
            FrustumKind::Orthographic(o) => {
                OrthographicFrustum::pack(o, array, i);
                i += OrthographicFrustum::PACKED_LENGTH;
            }
        }

        Cartesian3::pack(&self.origin, array, Some(i));
        i += Cartesian3::PACKED_LENGTH;
        Quaternion::pack(&self.orientation, array, i);
        i += Quaternion::PACKED_LENGTH;
        array[i] = if self.draw_near_plane { 1.0 } else { 0.0 };
    }

    /// Retrieves an instance from a packed array.
    pub fn unpack(array: &[f64], starting_index: Option<usize>, result: Option<&mut Self>) -> Self {
        let mut i = starting_index.unwrap_or(0);

        let frustum_type = array[i] as u32;
        i += 1;

        let frustum = if frustum_type == PERSPECTIVE {
            let f = PerspectiveFrustum::unpack(array, i, None);
            i += PerspectiveFrustum::PACKED_LENGTH;
            FrustumKind::Perspective(f)
        } else {
            let f = OrthographicFrustum::unpack(array, i, None);
            i += OrthographicFrustum::PACKED_LENGTH;
            FrustumKind::Orthographic(f)
        };

        let origin = Cartesian3::unpack_new(array, Some(i));
        i += Cartesian3::PACKED_LENGTH;
        let orientation = Quaternion::unpack_new(array, i);
        i += Quaternion::PACKED_LENGTH;
        let draw_near_plane = array[i] == 1.0;

        match result {
            Some(r) => {
                r.frustum_type = frustum_type;
                r.frustum = frustum;
                r.origin = origin;
                r.orientation = orientation;
                r.draw_near_plane = draw_near_plane;
                r.clone()
            }
            None => Self {
                frustum_type,
                frustum,
                origin,
                orientation,
                draw_near_plane,
            },
        }
    }

    /// Computes the geometric representation of a frustum outline, including
    /// its vertices, indices, and a bounding sphere.
    pub fn create_geometry(frustum_geometry: &Self) -> Option<Geometry> {
        let frustum_type = frustum_geometry.frustum_type;
        let draw_near_plane = frustum_geometry.draw_near_plane;

        let mut positions = vec![0.0f64; 3 * 4 * 2];
        let frustum = &mut frustum_geometry.frustum.clone();
        FrustumGeometry::compute_near_far_planes(
            &frustum_geometry.origin,
            &frustum_geometry.orientation,
            frustum_type,
            frustum,
            &mut positions,
        );

        let mut attributes = HashMap::new();
        attributes.insert(
            "position".to_string(),
            GeometryAttribute::new(ComponentDatatype::Double, 3, false, positions.clone()),
        );

        let number_of_planes = if draw_near_plane { 2 } else { 1 };
        let mut indices: IndexStorage =
            IndexDatatype::create_typed_array(8, 8 * (number_of_planes + 1));

        // Build the near/far planes
        let start = if draw_near_plane { 0 } else { 1 };
        for i in start..2usize {
            let offset = if draw_near_plane { i * 8 } else { 0 };
            let index = i * 4;

            // JS writes at `offset` into a pre-sized array; Rust pushes in order.
            let _ = offset;
            indices.push(index as u32);
            indices.push((index + 1) as u32);
            indices.push((index + 1) as u32);
            indices.push((index + 2) as u32);
            indices.push((index + 2) as u32);
            indices.push((index + 3) as u32);
            indices.push((index + 3) as u32);
            indices.push(index as u32);
        }

        // Build the sides of the frustum
        for i in 0..2usize {
            let index = i * 4;

            indices.push(index as u32);
            indices.push((index + 4) as u32);
            indices.push((index + 1) as u32);
            indices.push((index + 5) as u32);
            indices.push((index + 2) as u32);
            indices.push((index + 6) as u32);
            indices.push((index + 3) as u32);
            indices.push((index + 7) as u32);
        }

        let bounding_sphere = BoundingSphere::from_vertices(&positions, None, None, None);

        Some(Geometry::with_all(
            attributes,
            Some(indices),
            Some(PrimitiveType::Lines),
            Some(bounding_sphere),
            GeometryType::None,
            None,
            None,
        ))
    }
}
