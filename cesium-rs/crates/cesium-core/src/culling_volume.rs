//! Ported from `packages/engine/Source/Core/CullingVolume.js`.

use crate::axis_aligned_bounding_box::AxisAlignedBoundingBox;
use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::cartesian4::Cartesian4;
use crate::intersect::Intersect;
use crate::oriented_bounding_box::OrientedBoundingBox;
use crate::plane::Plane;

/// A culling volume defined by planes.
///
/// Each plane is represented by a Cartesian4, where x, y, z define the unit
/// normal and w is the signed distance from the origin.
#[derive(Clone, Debug)]
pub struct CullingVolume {
    /// The clipping planes.
    pub planes: Vec<Cartesian4>,
}

impl Default for CullingVolume {
    fn default() -> Self {
        Self { planes: Vec::new() }
    }
}

/// For plane masks, represents entirely outside the culling volume.
///
/// Port-local spelling of `CullingVolume.MASK_OUTSIDE`; the associated const
/// below is the JS name.
pub const MASK_OUTSIDE: u32 = 0xFFFFFFFF;
/// For plane masks, represents entirely inside the culling volume.
pub const MASK_INSIDE: u32 = 0x00000000;
/// For plane masks, represents possibly intersecting all planes.
pub const MASK_INDETERMINATE: u32 = 0x7FFFFFFF;

/// The contract CesiumJS `CullingVolume.computeVisibility` duck-types on.
///
/// The JS reads `boundingVolume.intersectPlane(plane)` and nothing else, so
/// any of `BoundingSphere`, `AxisAlignedBoundingBox` or `OrientedBoundingBox`
/// can be passed. Hard-coding `&BoundingSphere` — as an earlier revision of
/// this file did — silently dropped the box cases, which is exactly what
/// `CullingVolumeSpec`'s `"can contain an axis aligned bounding box"` block
/// exercises and what `Cesium3DTile` relies on for region-bounded tiles.
///
/// The impls live here rather than in the three bounding-volume modules so
/// those stay faithful to their own JS files, none of which mention
/// `CullingVolume`.
pub trait BoundingVolume {
    /// Mirrors `boundingVolume.intersectPlane(plane)`.
    fn intersect_plane(&self, plane: &Plane) -> Intersect;
}

impl BoundingVolume for BoundingSphere {
    fn intersect_plane(&self, plane: &Plane) -> Intersect {
        BoundingSphere::intersect_plane(self, plane)
    }
}

impl BoundingVolume for AxisAlignedBoundingBox {
    fn intersect_plane(&self, plane: &Plane) -> Intersect {
        self.intersect_plane_instance(plane)
    }
}

impl BoundingVolume for OrientedBoundingBox {
    fn intersect_plane(&self, plane: &Plane) -> Intersect {
        self.intersect_plane_instance(plane)
    }
}

const FACE_NORMALS: [Cartesian3; 3] = [
    Cartesian3 { x: 1.0, y: 0.0, z: 0.0 },
    Cartesian3 { x: 0.0, y: 1.0, z: 0.0 },
    Cartesian3 { x: 0.0, y: 0.0, z: 1.0 },
];

impl CullingVolume {
    /// Mirrors `CullingVolume.MASK_OUTSIDE`.
    pub const MASK_OUTSIDE: u32 = MASK_OUTSIDE;
    /// Mirrors `CullingVolume.MASK_INSIDE`.
    pub const MASK_INSIDE: u32 = MASK_INSIDE;
    /// Mirrors `CullingVolume.MASK_INDETERMINATE`.
    pub const MASK_INDETERMINATE: u32 = MASK_INDETERMINATE;

    /// Constructs a culling volume from a bounding sphere. Creates six planes
    /// that create a box containing the sphere, aligned to x/y/z axes.
    ///
    /// When `result` is supplied it is written into *and* returned, matching
    /// the JS `expect(result).toBe(returnedResult)` identity contract. An
    /// earlier revision cloned `result`, mutated the clone and never wrote
    /// back, so a caller-provided volume stayed untouched.
    pub fn from_bounding_sphere(
        bounding_sphere: &BoundingSphere,
        result: Option<&mut Self>,
    ) -> Self {
        match result {
            Some(r) => {
                Self::build_from_bounding_sphere(bounding_sphere, r);
                r.clone()
            }
            None => {
                let mut r = Self::default();
                Self::build_from_bounding_sphere(bounding_sphere, &mut r);
                r
            }
        }
    }

    /// The body of `CullingVolume.fromBoundingSphere`.
    ///
    /// JS sizes the plane array to `2 * faces.length` and reuses any
    /// `Cartesian4` already in the slot; `resize` is the equivalent because the
    /// six entries are unconditionally overwritten below.
    fn build_from_bounding_sphere(bounding_sphere: &BoundingSphere, result: &mut Self) {
        let center = bounding_sphere.center;
        let radius = bounding_sphere.radius;

        result.planes.resize(2 * FACE_NORMALS.len(), Cartesian4::ZERO);

        let mut plane_index = 0;
        for face_normal in &FACE_NORMALS {
            // Near plane: center - radius * normal
            let mut scratch_center = Cartesian3::ZERO;
            Cartesian3::multiply_by_scalar(face_normal, -radius, &mut scratch_center);
            let mut plane_center = Cartesian3::ZERO;
            Cartesian3::add(&center, &scratch_center, &mut plane_center);

            result.planes[plane_index] = Cartesian4::new(
                face_normal.x,
                face_normal.y,
                face_normal.z,
                -Cartesian3::dot(face_normal, &plane_center),
            );

            // Far plane: center + radius * normal
            let mut scratch_center2 = Cartesian3::ZERO;
            Cartesian3::multiply_by_scalar(face_normal, radius, &mut scratch_center2);
            let mut plane_center2 = Cartesian3::ZERO;
            Cartesian3::add(&center, &scratch_center2, &mut plane_center2);

            let neg_normal = Cartesian3::new(-face_normal.x, -face_normal.y, -face_normal.z);
            result.planes[plane_index + 1] = Cartesian4::new(
                neg_normal.x,
                neg_normal.y,
                neg_normal.z,
                -Cartesian3::dot(&neg_normal, &plane_center2),
            );

            plane_index += 2;
        }
    }

    /// Determines whether a bounding volume intersects the culling volume.
    ///
    /// Generic over [`BoundingVolume`] to mirror the JS duck typing — see the
    /// trait docs.
    pub fn compute_visibility<V: BoundingVolume>(&self, bounding_volume: &V) -> Intersect {
        let mut intersecting = false;

        for plane4 in &self.planes {
            let plane = Plane::from_cartesian4_new(plane4);
            let result = bounding_volume.intersect_plane(&plane);
            if result == Intersect::Outside {
                return Intersect::Outside;
            } else if result == Intersect::Intersecting {
                intersecting = true;
            }
        }

        if intersecting {
            Intersect::Intersecting
        } else {
            Intersect::Inside
        }
    }

    /// Determines whether a bounding volume intersects the culling volume,
    /// using a plane mask to skip redundant checks.
    ///
    /// Generic over [`BoundingVolume`] for the same reason as
    /// [`Self::compute_visibility`].
    pub fn compute_visibility_with_plane_mask<V: BoundingVolume>(
        &self,
        bounding_volume: &V,
        parent_plane_mask: u32,
    ) -> u32 {
        if parent_plane_mask == MASK_OUTSIDE || parent_plane_mask == MASK_INSIDE {
            // Parent is completely outside or completely inside, so this child
            // is as well.
            return parent_plane_mask;
        }

        // Start with MASK_INSIDE (all zeros) so that after the loop the return
        // value can be compared with MASK_INSIDE — with fewer than 31 planes
        // the upper bits are never touched.
        let mut mask = MASK_INSIDE;

        for (k, plane4) in self.planes.iter().enumerate() {
            // For k greater than 31 (31 being the maximum number of
            // INSIDE/INTERSECTING bits that can be stored) skip the
            // optimisation.
            let flag = if k < 31 { 1u32 << k } else { 0u32 };
            if k < 31 && (parent_plane_mask & flag) == 0 {
                // bounding_volume is known to be INSIDE this plane.
                continue;
            }

            let plane = Plane::from_cartesian4_new(plane4);
            let result = bounding_volume.intersect_plane(&plane);
            if result == Intersect::Outside {
                return MASK_OUTSIDE;
            } else if result == Intersect::Intersecting {
                mask |= flag;
            }
        }

        mask
    }
}
