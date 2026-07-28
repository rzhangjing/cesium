//! Off-center frustum specs - ported from:
//! - packages/engine/Specs/Core/PerspectiveOffCenterFrustumSpec.js (31 it())
//! - packages/engine/Specs/Core/OrthographicOffCenterFrustumSpec.js (30 it())
//!
//! A-class tests: 29 (15 perspective + 14 orthographic).
//!
//! Omitted (C-class): all `throws` tests (near/far out of range, left>right,
//! bottom>top, undefined params, getPixelDimensions arg validation — Rust type
//! safety / debug_assert), `equals undefined` (JS undefined handling), and
//! `clone with result parameter` (JS result-param API).
//!
//! Note on "constructs": CesiumJS asserts `f.width === options.width` and
//! `f.aspectRatio === options.aspectRatio`, but both are `undefined` on an
//! off-center frustum (it has no width/aspectRatio properties), so those two
//! comparisons are trivially `undefined === undefined`. The Rust port therefore
//! only asserts the meaningful `near`/`far` values.

use cesium_geospatial::frustum::{OrthographicOffCenterFrustum, PerspectiveOffCenterFrustum};
use glam::{DMat4, DVec3};

const EPSILON15: f64 = 1e-15;
const EPSILON6: f64 = 1e-6;
const EPSILON4: f64 = 1e-4;
const EPSILON1: f64 = 1e-1;
const EPSILON2: f64 = 1e-2;
const EPSILON7: f64 = 1e-7;

fn assert_approx(a: f64, b: f64, eps: f64, msg: &str) {
    assert!(
        (a - b).abs() < eps,
        "{}: got {}, expected {} (eps={})",
        msg,
        a,
        b,
        eps
    );
}

/// Compares two DMat4 elementwise within epsilon.
fn assert_mat4_approx(actual: &DMat4, expected: &DMat4, eps: f64, msg: &str) {
    for col in 0..4 {
        for row in 0..4 {
            assert_approx(
                actual.col(col)[row],
                expected.col(col)[row],
                eps,
                &format!("{}[{}][{}]", msg, col, row),
            );
        }
    }
}

// ============================================================================
// PerspectiveOffCenterFrustum (from PerspectiveOffCenterFrustumSpec.js)
// Setup: left=-1, right=1, bottom=-1, top=1, near=1, far=2
// ============================================================================

fn make_perspective_off_center() -> PerspectiveOffCenterFrustum {
    PerspectiveOffCenterFrustum::from_bounds(-1.0, 1.0, -1.0, 1.0, 1.0, 2.0)
}

fn perspective_off_center_planes() -> [(DVec3, f64); 6] {
    let f = make_perspective_off_center();
    let cv = f.compute_culling_volume(DVec3::ZERO, -DVec3::Z, DVec3::Y);
    [
        (cv.planes[0].normal, cv.planes[0].distance),
        (cv.planes[1].normal, cv.planes[1].distance),
        (cv.planes[2].normal, cv.planes[2].distance),
        (cv.planes[3].normal, cv.planes[3].distance),
        (cv.planes[4].normal, cv.planes[4].distance),
        (cv.planes[5].normal, cv.planes[5].distance),
    ]
}

#[test]
fn test_perspective_off_center_constructs() {
    // Ported from: PerspectiveOffCenterFrustumSpec "constructs"
    // width/aspectRatio are undefined on an off-center frustum (trivially equal),
    // so only near/far are asserted.
    let f = PerspectiveOffCenterFrustum::from_bounds(-1.0, 2.0, -1.0, 5.0, 3.0, 4.0);
    assert_eq!(f.near, 3.0);
    assert_eq!(f.far, 4.0);
}

#[test]
fn test_perspective_off_center_default_constructs() {
    // Ported from: PerspectiveOffCenterFrustumSpec "default constructs"
    let f = PerspectiveOffCenterFrustum::new();
    assert!(f.left.is_none());
    assert!(f.right.is_none());
    assert!(f.top.is_none());
    assert!(f.bottom.is_none());
    assert_eq!(f.near, 1.0);
    assert_eq!(f.far, 500_000_000.0);
}

#[test]
fn test_perspective_off_center_left_plane() {
    // Ported from: "get frustum left plane"
    // Expected: Cartesian4(x, 0, -x, 0) where x = 1/sqrt(2)
    let planes = perspective_off_center_planes();
    let (normal, distance) = planes[0];
    let x = 1.0 / 2.0_f64.sqrt();
    assert_approx(normal.x, x, EPSILON15, "left.x");
    assert_approx(normal.y, 0.0, EPSILON15, "left.y");
    assert_approx(normal.z, -x, EPSILON15, "left.z");
    assert_approx(distance, 0.0, EPSILON15, "left.d");
}

#[test]
fn test_perspective_off_center_right_plane() {
    // Ported from: "get frustum right plane"
    // Expected: Cartesian4(-x, 0, -x, 0)
    let planes = perspective_off_center_planes();
    let (normal, distance) = planes[1];
    let x = 1.0 / 2.0_f64.sqrt();
    assert_approx(normal.x, -x, EPSILON15, "right.x");
    assert_approx(normal.y, 0.0, EPSILON15, "right.y");
    assert_approx(normal.z, -x, EPSILON15, "right.z");
    assert_approx(distance, 0.0, EPSILON15, "right.d");
}

#[test]
fn test_perspective_off_center_bottom_plane() {
    // Ported from: "get frustum bottom plane"
    // Expected: Cartesian4(0, x, -x, 0)
    let planes = perspective_off_center_planes();
    let (normal, distance) = planes[2];
    let x = 1.0 / 2.0_f64.sqrt();
    assert_approx(normal.x, 0.0, EPSILON15, "bottom.x");
    assert_approx(normal.y, x, EPSILON15, "bottom.y");
    assert_approx(normal.z, -x, EPSILON15, "bottom.z");
    assert_approx(distance, 0.0, EPSILON15, "bottom.d");
}

#[test]
fn test_perspective_off_center_top_plane() {
    // Ported from: "get frustum top plane"
    // Expected: Cartesian4(0, -x, -x, 0)
    let planes = perspective_off_center_planes();
    let (normal, distance) = planes[3];
    let x = 1.0 / 2.0_f64.sqrt();
    assert_approx(normal.x, 0.0, EPSILON15, "top.x");
    assert_approx(normal.y, -x, EPSILON15, "top.y");
    assert_approx(normal.z, -x, EPSILON15, "top.z");
    assert_approx(distance, 0.0, EPSILON15, "top.d");
}

#[test]
fn test_perspective_off_center_near_plane() {
    // Ported from: "get frustum near plane"
    // Expected: Cartesian4(0, 0, -1, -1)
    let planes = perspective_off_center_planes();
    let (normal, distance) = planes[4];
    assert_approx(normal.x, 0.0, EPSILON15, "near.x");
    assert_approx(normal.y, 0.0, EPSILON15, "near.y");
    assert_approx(normal.z, -1.0, EPSILON15, "near.z");
    assert_approx(distance, -1.0, EPSILON15, "near.d");
}

#[test]
fn test_perspective_off_center_far_plane() {
    // Ported from: "get frustum far plane"
    // Expected: Cartesian4(0, 0, 1, 2)
    let planes = perspective_off_center_planes();
    let (normal, distance) = planes[5];
    assert_approx(normal.x, 0.0, EPSILON15, "far.x");
    assert_approx(normal.y, 0.0, EPSILON15, "far.y");
    assert_approx(normal.z, 1.0, EPSILON15, "far.z");
    assert_approx(distance, 2.0, EPSILON15, "far.d");
}

#[test]
fn test_perspective_off_center_projection_matrix() {
    // Ported from: "get perspective projection matrix"
    // Expected: Matrix4.computePerspectiveOffCenter(-1, 1, -1, 1, 1, 2)
    let f = make_perspective_off_center();
    let proj = f.projection_matrix();
    let expected = DMat4::from_cols_array(&[
        1.0, 0.0, 0.0, 0.0, // col0: 2*near/(right-left) = 1
        0.0, 1.0, 0.0, 0.0, // col1: 2*near/(top-bottom) = 1
        0.0, 0.0, -3.0, -1.0, // col2: -(far+near)/(far-near) = -3, -1
        0.0, 0.0, -4.0, 0.0, // col3: -2*far*near/(far-near) = -4
    ]);
    assert_mat4_approx(&proj, &expected, EPSILON6, "perspective proj");
}

#[test]
fn test_perspective_off_center_infinite_projection_matrix() {
    // Ported from: "get infinite perspective matrix"
    // Expected: Matrix4.computeInfinitePerspectiveOffCenter(-1, 1, -1, 1, 1)
    let f = make_perspective_off_center();
    let proj = f.infinite_projection_matrix();
    let expected = DMat4::from_cols_array(&[
        1.0, 0.0, 0.0, 0.0, // col0
        0.0, 1.0, 0.0, 0.0, // col1
        0.0, 0.0, -1.0, -1.0, // col2: -1, -1
        0.0, 0.0, -2.0, 0.0, // col3: -2*near = -2
    ]);
    assert_mat4_approx(&proj, &expected, EPSILON6, "infinite perspective proj");
}

#[test]
fn test_perspective_off_center_pixel_dimensions() {
    // Ported from: "get pixel dimensions"
    let f = make_perspective_off_center();
    let (pw, ph) = f.pixel_dimensions(1.0, 1.0, 1.0, 1.0);
    assert_eq!(pw, 2.0);
    assert_eq!(ph, 2.0);
}

#[test]
fn test_perspective_off_center_pixel_dimensions_with_pixel_ratio() {
    // Ported from: "get pixel dimensions with pixel ratio"
    let f = make_perspective_off_center();
    let (pw, ph) = f.pixel_dimensions(1.0, 1.0, 1.0, 2.0);
    assert_eq!(pw, 4.0);
    assert_eq!(ph, 4.0);
}

#[test]
fn test_perspective_off_center_equals() {
    // Ported from: "equals"
    let f = make_perspective_off_center();
    let f2 = PerspectiveOffCenterFrustum::from_bounds(-1.0, 1.0, -1.0, 1.0, 1.0, 2.0);
    assert!(f.equals(&f2));
}

#[test]
fn test_perspective_off_center_equals_epsilon() {
    // Ported from: "equals epsilon"
    let f = make_perspective_off_center();

    let f2 = PerspectiveOffCenterFrustum::from_bounds(-1.0, 1.0, -1.0, 1.0, 1.0, 2.0);
    assert!(f.equals_epsilon(&f2, EPSILON7, EPSILON7));

    let f3 = PerspectiveOffCenterFrustum::from_bounds(-1.0, 1.01, -1.0, 1.01, 1.01, 1.99);
    assert!(f.equals_epsilon(&f3, EPSILON1, EPSILON1));

    let f4 = PerspectiveOffCenterFrustum::from_bounds(-1.0, 1.1, -1.0, 1.0, 1.0, 2.0);
    assert!(!f.equals_epsilon(&f4, EPSILON2, EPSILON2));
}

#[test]
fn test_perspective_off_center_clone() {
    // Ported from: "clone"
    let f = make_perspective_off_center();
    let f2 = f; // Copy semantics mirror CesiumJS clone()
    assert!(f.equals(&f2));
}

// ============================================================================
// OrthographicOffCenterFrustum (from OrthographicOffCenterFrustumSpec.js)
// Setup: left=-1, right=1, bottom=-1, top=1, near=1, far=3
// ============================================================================

fn make_orthographic_off_center() -> OrthographicOffCenterFrustum {
    OrthographicOffCenterFrustum::from_bounds(-1.0, 1.0, -1.0, 1.0, 1.0, 3.0)
}

fn orthographic_off_center_planes() -> [(DVec3, f64); 6] {
    let f = make_orthographic_off_center();
    let cv = f.compute_culling_volume(DVec3::ZERO, -DVec3::Z, DVec3::Y);
    [
        (cv.planes[0].normal, cv.planes[0].distance),
        (cv.planes[1].normal, cv.planes[1].distance),
        (cv.planes[2].normal, cv.planes[2].distance),
        (cv.planes[3].normal, cv.planes[3].distance),
        (cv.planes[4].normal, cv.planes[4].distance),
        (cv.planes[5].normal, cv.planes[5].distance),
    ]
}

#[test]
fn test_orthographic_off_center_constructs() {
    // Ported from: OrthographicOffCenterFrustumSpec "constructs"
    // width/aspectRatio are undefined on an off-center frustum (trivially equal),
    // so only near/far are asserted.
    let f = OrthographicOffCenterFrustum::from_bounds(-1.0, 2.0, -1.0, 5.0, 3.0, 4.0);
    assert_eq!(f.near, 3.0);
    assert_eq!(f.far, 4.0);
}

#[test]
fn test_orthographic_off_center_default_constructs() {
    // Ported from: OrthographicOffCenterFrustumSpec "default constructs"
    let f = OrthographicOffCenterFrustum::new();
    assert!(f.left.is_none());
    assert!(f.right.is_none());
    assert!(f.top.is_none());
    assert!(f.bottom.is_none());
    assert_eq!(f.near, 1.0);
    assert_eq!(f.far, 500_000_000.0);
}

#[test]
fn test_orthographic_off_center_left_plane() {
    // Ported from: "get frustum left plane"
    // Expected: Cartesian4(1, 0, 0, 1)
    let planes = orthographic_off_center_planes();
    let (normal, distance) = planes[0];
    assert_approx(normal.x, 1.0, EPSILON4, "left.x");
    assert_approx(normal.y, 0.0, EPSILON4, "left.y");
    assert_approx(normal.z, 0.0, EPSILON4, "left.z");
    assert_approx(distance, 1.0, EPSILON4, "left.d");
}

#[test]
fn test_orthographic_off_center_right_plane() {
    // Ported from: "get frustum right plane"
    // Expected: Cartesian4(-1, 0, 0, 1)
    let planes = orthographic_off_center_planes();
    let (normal, distance) = planes[1];
    assert_approx(normal.x, -1.0, EPSILON4, "right.x");
    assert_approx(normal.y, 0.0, EPSILON4, "right.y");
    assert_approx(normal.z, 0.0, EPSILON4, "right.z");
    assert_approx(distance, 1.0, EPSILON4, "right.d");
}

#[test]
fn test_orthographic_off_center_bottom_plane() {
    // Ported from: "get frustum bottom plane"
    // Expected: Cartesian4(0, 1, 0, 1)
    let planes = orthographic_off_center_planes();
    let (normal, distance) = planes[2];
    assert_approx(normal.x, 0.0, EPSILON4, "bottom.x");
    assert_approx(normal.y, 1.0, EPSILON4, "bottom.y");
    assert_approx(normal.z, 0.0, EPSILON4, "bottom.z");
    assert_approx(distance, 1.0, EPSILON4, "bottom.d");
}

#[test]
fn test_orthographic_off_center_top_plane() {
    // Ported from: "get frustum top plane"
    // Expected: Cartesian4(0, -1, 0, 1)
    let planes = orthographic_off_center_planes();
    let (normal, distance) = planes[3];
    assert_approx(normal.x, 0.0, EPSILON4, "top.x");
    assert_approx(normal.y, -1.0, EPSILON4, "top.y");
    assert_approx(normal.z, 0.0, EPSILON4, "top.z");
    assert_approx(distance, 1.0, EPSILON4, "top.d");
}

#[test]
fn test_orthographic_off_center_near_plane() {
    // Ported from: "get frustum near plane"
    // Expected: Cartesian4(0, 0, -1, -1)
    let planes = orthographic_off_center_planes();
    let (normal, distance) = planes[4];
    assert_approx(normal.x, 0.0, EPSILON4, "near.x");
    assert_approx(normal.y, 0.0, EPSILON4, "near.y");
    assert_approx(normal.z, -1.0, EPSILON4, "near.z");
    assert_approx(distance, -1.0, EPSILON4, "near.d");
}

#[test]
fn test_orthographic_off_center_far_plane() {
    // Ported from: "get frustum far plane"
    // Expected: Cartesian4(0, 0, 1, 3)
    let planes = orthographic_off_center_planes();
    let (normal, distance) = planes[5];
    assert_approx(normal.x, 0.0, EPSILON4, "far.x");
    assert_approx(normal.y, 0.0, EPSILON4, "far.y");
    assert_approx(normal.z, 1.0, EPSILON4, "far.z");
    assert_approx(distance, 3.0, EPSILON4, "far.d");
}

#[test]
fn test_orthographic_off_center_projection_matrix() {
    // Ported from: "get orthographic projection matrix"
    // Expected: Matrix4.computeOrthographicOffCenter(-1, 1, -1, 1, 1, 3)
    let f = make_orthographic_off_center();
    let proj = f.projection_matrix();
    let expected = DMat4::from_cols_array(&[
        1.0, 0.0, 0.0, 0.0, // col0: 2/(right-left) = 1
        0.0, 1.0, 0.0, 0.0, // col1: 2/(top-bottom) = 1
        0.0, 0.0, -1.0, 0.0, // col2: -2/(far-near) = -1
        0.0, 0.0, -2.0, 1.0, // col3: tz = -(far+near)/(far-near) = -2
    ]);
    assert_mat4_approx(&proj, &expected, EPSILON6, "orthographic proj");
}

#[test]
fn test_orthographic_off_center_pixel_dimensions() {
    // Ported from: "get pixel dimensions"
    let f = make_orthographic_off_center();
    let (pw, ph) = f.pixel_dimensions(1.0, 1.0, 0.0, 1.0);
    assert_eq!(pw, 2.0);
    assert_eq!(ph, 2.0);
}

#[test]
fn test_orthographic_off_center_pixel_dimensions_with_pixel_ratio() {
    // Ported from: "get pixel dimensions with pixel ratio"
    let f = make_orthographic_off_center();
    let (pw, ph) = f.pixel_dimensions(1.0, 1.0, 0.0, 2.0);
    assert_eq!(pw, 4.0);
    assert_eq!(ph, 4.0);
}

#[test]
fn test_orthographic_off_center_equals() {
    // Ported from: "equals"
    let f = make_orthographic_off_center();
    let f2 = OrthographicOffCenterFrustum::from_bounds(-1.0, 1.0, -1.0, 1.0, 1.0, 3.0);
    assert!(f.equals(&f2));
}

#[test]
fn test_orthographic_off_center_equals_epsilon() {
    // Ported from: "equals epsilon"
    let f = make_orthographic_off_center();

    let f2 = OrthographicOffCenterFrustum::from_bounds(-1.0, 1.0, -1.0, 1.0, 1.0, 3.0);
    assert!(f.equals_epsilon(&f2, EPSILON7, EPSILON7));

    let f3 = OrthographicOffCenterFrustum::from_bounds(-0.99, 1.02, -1.05, 0.99, 1.01, 2.98);
    assert!(f.equals_epsilon(&f3, EPSILON1, EPSILON1));

    let f4 = OrthographicOffCenterFrustum::from_bounds(-1.02, 0.0, -1.005, 1.02, 1.1, 2.9);
    assert!(!f.equals_epsilon(&f4, EPSILON2, EPSILON2));
}

#[test]
fn test_orthographic_off_center_clone() {
    // Ported from: "clone"
    let f = make_orthographic_off_center();
    let f2 = f; // Copy semantics mirror CesiumJS clone()
    assert!(f.equals(&f2));
}
