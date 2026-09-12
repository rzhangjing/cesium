//! Tests for `cesium_core::CullingVolume`.
//!
//! The box cases mirror `packages/engine/Specs/Core/CullingVolumeSpec.js`
//! (`describe("box intersections")`), which is the coverage that forced
//! `computeVisibility` to be generic: CesiumJS duck-types on
//! `boundingVolume.intersectPlane`, so an `AxisAlignedBoundingBox` is a legal
//! argument and not just a `BoundingSphere`.

use cesium_core::axis_aligned_bounding_box::AxisAlignedBoundingBox;
use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::culling_volume::{BoundingVolume, CullingVolume, MASK_INSIDE, MASK_OUTSIDE};
use cesium_core::intersect::Intersect;
use cesium_core::perspective_frustum::PerspectiveFrustum;

#[test]
fn from_bounding_sphere_creates_six_planes() {
    let bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 1.0);
    let cv = CullingVolume::from_bounding_sphere(&bs, None);
    assert_eq!(cv.planes.len(), 6);
}

#[test]
fn sphere_inside_culling_volume_is_inside() {
    // Create a culling volume from a large sphere
    let big_bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 100.0);
    let cv = CullingVolume::from_bounding_sphere(&big_bs, None);

    // A small sphere at the center should be inside
    let small_bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 1.0);
    let result = cv.compute_visibility(&small_bs);
    assert_eq!(result, Intersect::Inside);
}

#[test]
fn sphere_outside_culling_volume_is_outside() {
    let big_bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 10.0);
    let cv = CullingVolume::from_bounding_sphere(&big_bs, None);

    // A sphere far away should be outside
    let far_bs = BoundingSphere::new(Cartesian3::new(1000.0, 0.0, 0.0), 1.0);
    let result = cv.compute_visibility(&far_bs);
    assert_eq!(result, Intersect::Outside);
}

#[test]
fn sphere_intersecting_plane_is_intersecting() {
    let big_bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 10.0);
    let cv = CullingVolume::from_bounding_sphere(&big_bs, None);

    // A sphere near the edge should be intersecting
    let edge_bs = BoundingSphere::new(Cartesian3::new(9.5, 0.0, 0.0), 1.0);
    let result = cv.compute_visibility(&edge_bs);
    assert_eq!(result, Intersect::Intersecting);
}

#[test]
fn compute_visibility_with_plane_mask_outside_returns_outside() {
    let big_bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 100.0);
    let cv = CullingVolume::from_bounding_sphere(&big_bs, None);

    let small_bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 1.0);
    let result = cv.compute_visibility_with_plane_mask(&small_bs, MASK_OUTSIDE);
    assert_eq!(result, MASK_OUTSIDE);
}

#[test]
fn compute_visibility_with_plane_mask_inside_returns_inside() {
    let big_bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 100.0);
    let cv = CullingVolume::from_bounding_sphere(&big_bs, None);

    let small_bs = BoundingSphere::new(Cartesian3::new(0.0, 0.0, 0.0), 1.0);
    let result = cv.compute_visibility_with_plane_mask(&small_bs, MASK_INSIDE);
    assert_eq!(result, MASK_INSIDE);
}

#[test]
fn default_has_no_planes() {
    let cv = CullingVolume::default();
    assert!(cv.planes.is_empty());
}

// ---- Axis aligned bounding box intersections ----

/// The JS spec's `beforeEach`: a 60° square frustum spanning z = -1 … -2,
/// built through `PerspectiveFrustum.computeCullingVolume` so the six planes
/// come from the same code path the renderer uses.
fn frustum_culling_volume() -> CullingVolume {
    let mut frustum = PerspectiveFrustum::new();
    frustum.near = 1.0;
    frustum.far = 2.0;
    frustum.fov = Some(std::f64::consts::FRAC_PI_3);
    frustum.aspect_ratio = Some(1.0);
    let direction = Cartesian3::new(0.0, 0.0, -1.0);
    frustum
        .compute_culling_volume(&Cartesian3::ZERO, &direction, &Cartesian3::UNIT_Y)
        .clone()
}

/// Port of the JS spec's `testWithAndWithoutPlaneMask` helper: the plain
/// visibility and the plane-masked visibility must agree, and feeding the
/// returned mask back in must be a fixed point.
fn test_with_and_without_plane_mask(
    culling: &CullingVolume,
    bound: &impl BoundingVolume,
    intersect: Intersect,
    label: &str,
) {
    assert_eq!(culling.compute_visibility(bound), intersect, "{label}");

    let mask =
        culling.compute_visibility_with_plane_mask(bound, CullingVolume::MASK_INDETERMINATE);
    match intersect {
        Intersect::Inside => assert_eq!(mask, MASK_INSIDE, "{label}"),
        Intersect::Outside => assert_eq!(mask, MASK_OUTSIDE, "{label}"),
        Intersect::Intersecting => {
            assert_ne!(mask, MASK_INSIDE, "{label}");
            assert_ne!(mask, MASK_OUTSIDE, "{label}");
        }
    }
    assert_eq!(
        culling.compute_visibility_with_plane_mask(bound, mask),
        mask,
        "{label}"
    );
}

fn aabb(points: &[[f64; 3]]) -> AxisAlignedBoundingBox {
    let positions: Vec<Cartesian3> = points
        .iter()
        .map(|p| Cartesian3::new(p[0], p[1], p[2]))
        .collect();
    AxisAlignedBoundingBox::from_points(Some(&positions))
}

#[test]
fn can_contain_an_axis_aligned_bounding_box() {
    let culling = frustum_culling_volume();
    let box1 = aabb(&[
        [-0.5, 0.0, -1.25],
        [0.5, 0.0, -1.25],
        [-0.5, 0.0, -1.75],
        [0.5, 0.0, -1.75],
    ]);
    test_with_and_without_plane_mask(&culling, &box1, Intersect::Inside, "box1");
}

#[test]
fn can_partially_contain_an_axis_aligned_bounding_box() {
    let culling = frustum_culling_volume();
    let cases: [(&str, [[f64; 3]; 4]); 6] = [
        (
            "on the far plane",
            [[-0.5, 0.0, -1.5], [0.5, 0.0, -1.5], [-0.5, 0.0, -2.5], [0.5, 0.0, -2.5]],
        ),
        (
            "on the near plane",
            [[-0.5, 0.0, -0.5], [0.5, 0.0, -0.5], [-0.5, 0.0, -1.5], [0.5, 0.0, -1.5]],
        ),
        (
            "on the left plane",
            [[-1.5, 0.0, -1.25], [0.0, 0.0, -1.25], [-1.5, 0.0, -1.5], [0.0, 0.0, -1.5]],
        ),
        (
            "on the right plane",
            [[0.0, 0.0, -1.25], [1.5, 0.0, -1.25], [0.0, 0.0, -1.5], [1.5, 0.0, -1.5]],
        ),
        (
            "on the top plane",
            [[-0.5, 0.0, -1.25], [0.5, 0.0, -1.25], [-0.5, 2.0, -1.75], [0.5, 2.0, -1.75]],
        ),
        (
            "on the bottom plane",
            [[-0.5, -2.0, -1.25], [0.5, 0.0, -1.25], [-0.5, -2.0, -1.5], [0.5, 0.0, -1.5]],
        ),
    ];
    for (label, points) in cases {
        let bound = aabb(&points);
        test_with_and_without_plane_mask(&culling, &bound, Intersect::Intersecting, label);
    }
}

#[test]
fn can_not_contain_an_axis_aligned_bounding_box() {
    let culling = frustum_culling_volume();
    let cases: [(&str, [[f64; 3]; 4]); 6] = [
        (
            "past the far plane",
            [[-0.5, 0.0, -2.25], [0.5, 0.0, -2.25], [-0.5, 0.0, -2.75], [0.5, 0.0, -2.75]],
        ),
        (
            "before the near plane",
            [[-0.5, 0.0, -0.25], [0.5, 0.0, -0.25], [-0.5, 0.0, -0.75], [0.5, 0.0, -0.75]],
        ),
        (
            "past the left plane",
            [[-5.0, 0.0, -1.25], [-3.0, 0.0, -1.25], [-5.0, 0.0, -1.75], [-3.0, 0.0, -1.75]],
        ),
        (
            "past the right plane",
            [[3.0, 0.0, -1.25], [5.0, 0.0, -1.25], [3.0, 0.0, -1.75], [5.0, 0.0, -1.75]],
        ),
        (
            "past the top plane",
            [[-0.5, 3.0, -1.25], [0.5, 3.0, -1.25], [-0.5, 5.0, -1.75], [0.5, 5.0, -1.75]],
        ),
        (
            "past the bottom plane",
            [[-0.5, -3.0, -1.25], [0.5, -3.0, -1.25], [-0.5, -5.0, -1.75], [0.5, -5.0, -1.75]],
        ),
    ];
    for (label, points) in cases {
        let bound = aabb(&points);
        test_with_and_without_plane_mask(&culling, &bound, Intersect::Outside, label);
    }
}

/// The sphere path still resolves through the same generic entry points.
#[test]
fn can_contain_a_bounding_sphere() {
    let culling = frustum_culling_volume();
    let positions = [
        Cartesian3::new(0.0, 0.0, -1.25),
        Cartesian3::new(0.0, 0.0, -1.75),
    ];
    let sphere = BoundingSphere::from_points(&positions, None);
    test_with_and_without_plane_mask(&culling, &sphere, Intersect::Inside, "sphere1");
}

/// `CullingVolume.MASK_*` is reachable under the JS spelling as well as the
/// port's historical module-level consts.
#[test]
fn mask_constants_match_the_js_spellings() {
    assert_eq!(CullingVolume::MASK_OUTSIDE, MASK_OUTSIDE);
    assert_eq!(CullingVolume::MASK_INSIDE, MASK_INSIDE);
    assert_eq!(CullingVolume::MASK_INDETERMINATE, 0x7FFF_FFFF);
}

/// JS writes into the caller-supplied `result` and returns that same object
/// (`expect(result).toBe(returnedResult)`); the earlier port mutated a clone.
#[test]
fn from_bounding_sphere_writes_back_into_the_result() {
    let bs = BoundingSphere::new(Cartesian3::new(1.0, 2.0, 3.0), 4.0);
    let mut result = CullingVolume::default();
    let returned = CullingVolume::from_bounding_sphere(&bs, Some(&mut result));
    assert_eq!(result.planes.len(), 6, "the supplied volume must be filled");
    assert_eq!(result.planes, returned.planes);
}
