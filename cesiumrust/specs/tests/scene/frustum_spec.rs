//! Frustum tests ported from CesiumJS PerspectiveFrustumSpec.js + OrthographicFrustumSpec.js
//! PerspectiveFrustum: 16 A-class tests (of 32 total; 14 throws + 2 packable = C-class)
//! OrthographicFrustum: 14 A-class tests (of 30 total; 14 throws + 2 packable = C-class)
//!
//! Omitted (C-class): all throws-with-undefined/negative-arg tests (Rust type safety),
//! result-parameter variants (owned returns), createPackableSpecs (JS API).

use cesium_geospatial::frustum::{OrthographicFrustum, PerspectiveFrustum};
use glam::DVec3;

const EPSILON14: f64 = 1e-14;
const EPSILON6: f64 = 1e-6;
const EPSILON5: f64 = 1e-5;
const EPSILON1: f64 = 1e-1;
const EPSILON2: f64 = 1e-2;

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

// ============================================================================
// PerspectiveFrustum (from PerspectiveFrustumSpec.js)
// Setup: near=1, far=2, aspectRatio=1, fov=PI/3
// ============================================================================

fn make_perspective() -> PerspectiveFrustum {
    PerspectiveFrustum::new(std::f64::consts::FRAC_PI_3, 1.0, 1.0, 2.0)
}

fn perspective_planes() -> [(DVec3, f64); 6] {
    let f = make_perspective();
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
fn test_perspective_constructs() {
    // Ported from: PerspectiveFrustumSpec "constructs"
    let f = PerspectiveFrustum {
        fov: 1.0,
        aspect_ratio: 2.0,
        near: 3.0,
        far: 4.0,
        x_offset: 5.0,
        y_offset: 6.0,
    };
    assert_eq!(f.fov, 1.0);
    assert_eq!(f.aspect_ratio, 2.0);
    assert_eq!(f.near, 3.0);
    assert_eq!(f.far, 4.0);
    assert_eq!(f.x_offset, 5.0);
    assert_eq!(f.y_offset, 6.0);
}

#[test]
fn test_perspective_default_constructs() {
    // Ported from: PerspectiveFrustumSpec "default constructs"
    // CesiumJS defaults: near=1.0, far=500000000.0, xOffset=0, yOffset=0
    let f = PerspectiveFrustum::new(std::f64::consts::FRAC_PI_3, 1.0, 1.0, 500_000_000.0);
    assert_eq!(f.near, 1.0);
    assert_eq!(f.far, 500_000_000.0);
    assert_eq!(f.x_offset, 0.0);
    assert_eq!(f.y_offset, 0.0);
}

#[test]
fn test_perspective_left_plane() {
    // Ported from: PerspectiveFrustumSpec "get frustum left plane"
    // Expected: Cartesian4(sqrt(3)/2, 0, -0.5, 0)
    let planes = perspective_planes();
    let (normal, distance) = planes[0];
    let sqrt3_2 = 3.0_f64.sqrt() / 2.0;
    assert_approx(normal.x, sqrt3_2, EPSILON14, "left.x");
    assert_approx(normal.y, 0.0, EPSILON14, "left.y");
    assert_approx(normal.z, -0.5, EPSILON14, "left.z");
    assert_approx(distance, 0.0, EPSILON14, "left.d");
}

#[test]
fn test_perspective_right_plane() {
    // Ported from: PerspectiveFrustumSpec "get frustum right plane"
    // Expected: Cartesian4(-sqrt(3)/2, 0, -0.5, 0)
    let planes = perspective_planes();
    let (normal, distance) = planes[1];
    let sqrt3_2 = 3.0_f64.sqrt() / 2.0;
    assert_approx(normal.x, -sqrt3_2, EPSILON14, "right.x");
    assert_approx(normal.y, 0.0, EPSILON14, "right.y");
    assert_approx(normal.z, -0.5, EPSILON14, "right.z");
    assert_approx(distance, 0.0, EPSILON14, "right.d");
}

#[test]
fn test_perspective_bottom_plane() {
    // Ported from: PerspectiveFrustumSpec "get frustum bottom plane"
    // Expected: Cartesian4(0, sqrt(3)/2, -0.5, 0)
    let planes = perspective_planes();
    let (normal, distance) = planes[2];
    let sqrt3_2 = 3.0_f64.sqrt() / 2.0;
    assert_approx(normal.x, 0.0, EPSILON14, "bottom.x");
    assert_approx(normal.y, sqrt3_2, EPSILON14, "bottom.y");
    assert_approx(normal.z, -0.5, EPSILON14, "bottom.z");
    assert_approx(distance, 0.0, EPSILON14, "bottom.d");
}

#[test]
fn test_perspective_top_plane() {
    // Ported from: PerspectiveFrustumSpec "get frustum top plane"
    // Expected: Cartesian4(0, -sqrt(3)/2, -0.5, 0)
    let planes = perspective_planes();
    let (normal, distance) = planes[3];
    let sqrt3_2 = 3.0_f64.sqrt() / 2.0;
    assert_approx(normal.x, 0.0, EPSILON14, "top.x");
    assert_approx(normal.y, -sqrt3_2, EPSILON14, "top.y");
    assert_approx(normal.z, -0.5, EPSILON14, "top.z");
    assert_approx(distance, 0.0, EPSILON14, "top.d");
}

#[test]
fn test_perspective_near_plane() {
    // Ported from: PerspectiveFrustumSpec "get frustum near plane"
    // Expected: Cartesian4(0, 0, -1, -1)
    let planes = perspective_planes();
    let (normal, distance) = planes[4];
    assert_approx(normal.x, 0.0, EPSILON14, "near.x");
    assert_approx(normal.y, 0.0, EPSILON14, "near.y");
    assert_approx(normal.z, -1.0, EPSILON14, "near.z");
    assert_approx(distance, -1.0, EPSILON14, "near.d");
}

#[test]
fn test_perspective_far_plane() {
    // Ported from: PerspectiveFrustumSpec "get frustum far plane"
    // Expected: Cartesian4(0, 0, 1, 2)
    let planes = perspective_planes();
    let (normal, distance) = planes[5];
    assert_approx(normal.x, 0.0, EPSILON14, "far.x");
    assert_approx(normal.y, 0.0, EPSILON14, "far.y");
    assert_approx(normal.z, 1.0, EPSILON14, "far.z");
    assert_approx(distance, 2.0, EPSILON14, "far.d");
}

#[test]
fn test_perspective_sse_denominator() {
    // Ported from: PerspectiveFrustumSpec "get sseDenominator"
    // Expected: ≈ 1.1547 (= 2*tan(PI/6))
    let f = make_perspective();
    let expected = 2.0 * (std::f64::consts::FRAC_PI_3 * 0.5).tan();
    assert_approx(f.sse_denominator(), expected, EPSILON5, "sseDenominator");
    assert_approx(f.sse_denominator(), 1.1547, 1e-4, "sseDenominator approx");
}

#[test]
fn test_perspective_projection_matrix() {
    // Ported from: PerspectiveFrustumSpec "get perspective projection matrix"
    // Verify against computePerspectiveFieldOfView formula
    let f = make_perspective();
    let proj = f.projection_matrix();

    let fovy = f.fov;
    let aspect = f.aspect_ratio;
    let near = f.near;
    let far = f.far;
    let top = near * (fovy * 0.5).tan();
    let bottom = -top;
    let right = top * aspect;
    let left = -right;

    // Expected matrix (column-major, standard OpenGL perspective)
    let e0 = 2.0 * near / (right - left);
    let e5 = 2.0 * near / (top - bottom);
    let e10 = -(far + near) / (far - near);
    let e11 = -1.0;
    let e14 = -2.0 * far * near / (far - near);

    assert_approx(proj.x_axis.x, e0, EPSILON6, "proj[0][0]");
    assert_approx(proj.y_axis.y, e5, EPSILON6, "proj[1][1]");
    assert_approx(proj.z_axis.z, e10, EPSILON6, "proj[2][2]");
    assert_approx(proj.z_axis.w, e11, EPSILON6, "proj[2][3]");
    assert_approx(proj.w_axis.z, e14, EPSILON6, "proj[3][2]");
    assert_approx(proj.w_axis.w, 0.0, EPSILON6, "proj[3][3]");
}

#[test]
fn test_perspective_infinite_projection_matrix() {
    // Ported from: PerspectiveFrustumSpec "get infinite perspective matrix"
    let f = make_perspective();
    let inf_proj = f.infinite_projection_matrix();

    let top = f.near * (f.fov * 0.5).tan();
    let bottom = -top;
    let right = f.aspect_ratio * top;
    let left = -right;
    let near = f.near;
    let e = 1e-10_f64;

    // CesiumJS computeInfinitePerspectiveOffCenter formula
    let e0 = 2.0 * near / (right - left);
    let e5 = 2.0 * near / (top - bottom);
    let e8 = (right + left) / (right - left);
    let e9 = (top + bottom) / (top - bottom);
    let e10 = -1.0 + e;
    let e11 = -1.0;
    let e14 = (-2.0 + e) * near;

    assert_approx(inf_proj.x_axis.x, e0, EPSILON6, "inf[0][0]");
    assert_approx(inf_proj.y_axis.y, e5, EPSILON6, "inf[1][1]");
    assert_approx(inf_proj.z_axis.x, e8, EPSILON6, "inf[2][0]");
    assert_approx(inf_proj.z_axis.y, e9, EPSILON6, "inf[2][1]");
    assert_approx(inf_proj.z_axis.z, e10, EPSILON6, "inf[2][2]");
    assert_approx(inf_proj.z_axis.w, e11, EPSILON6, "inf[2][3]");
    assert_approx(inf_proj.w_axis.z, e14, EPSILON6, "inf[3][2]");
    assert_approx(inf_proj.w_axis.w, 0.0, EPSILON6, "inf[3][3]");
}

#[test]
fn test_perspective_pixel_dimensions() {
    // Ported from: PerspectiveFrustumSpec "get pixel dimensions"
    let f = make_perspective();
    let (pw, ph) = f.pixel_dimensions(1.0, 1.0, 1.0, 1.0);

    // Expected: 2 * distance * tan(fov/2) * aspect / width = 2*tan(PI/6) ≈ 1.1547
    let tan_phi = (f.fov * 0.5).tan();
    let tan_theta = tan_phi * f.aspect_ratio;
    let expected_x = 2.0 * 1.0 * tan_theta / 1.0;
    let expected_y = 2.0 * 1.0 * tan_phi / 1.0;
    assert_approx(pw, expected_x, EPSILON14, "pixelWidth");
    assert_approx(ph, expected_y, EPSILON14, "pixelHeight");
}

#[test]
fn test_perspective_pixel_dimensions_with_pixel_ratio() {
    // Ported from: PerspectiveFrustumSpec "get pixel dimensions with pixel ratio"
    let f = make_perspective();
    let (pw, ph) = f.pixel_dimensions(1.0, 1.0, 1.0, 2.0);

    let tan_phi = (f.fov * 0.5).tan();
    let tan_theta = tan_phi * f.aspect_ratio;
    let expected_x = 2.0 * 2.0 * tan_theta / 1.0;
    let expected_y = 2.0 * 2.0 * tan_phi / 1.0;
    assert_approx(pw, expected_x, EPSILON14, "pixelWidth ratio=2");
    assert_approx(ph, expected_y, EPSILON14, "pixelHeight ratio=2");
}

#[test]
fn test_perspective_equals() {
    // Ported from: PerspectiveFrustumSpec "equals"
    let f1 = make_perspective();
    let f2 = PerspectiveFrustum::new(std::f64::consts::FRAC_PI_3, 1.0, 1.0, 2.0);
    assert_eq!(f1, f2);
}

#[test]
fn test_perspective_equals_epsilon() {
    // Ported from: PerspectiveFrustumSpec "equals epsilon"
    let f1 = make_perspective();

    // Same values → within any epsilon
    let f2 = PerspectiveFrustum::new(std::f64::consts::FRAC_PI_3, 1.0, 1.0, 2.0);
    assert!((f1.fov - f2.fov).abs() < EPSILON6);
    assert!((f1.near - f2.near).abs() < EPSILON6);

    // Slightly different → within EPSILON1
    let f3 = PerspectiveFrustum::new(std::f64::consts::FRAC_PI_3 + 0.01, 1.01, 1.01, 2.01);
    assert!((f1.fov - f3.fov).abs() < EPSILON1);
    assert!((f1.aspect_ratio - f3.aspect_ratio).abs() < EPSILON1);
    assert!((f1.near - f3.near).abs() < EPSILON1);
    assert!((f1.far - f3.far).abs() < EPSILON1);

    // More different → NOT within EPSILON2
    let f4 = PerspectiveFrustum::new(std::f64::consts::FRAC_PI_3, 1.1, 1.0, 2.0);
    assert!((f1.aspect_ratio - f4.aspect_ratio).abs() > EPSILON2);
}

#[test]
fn test_perspective_clone() {
    // Ported from: PerspectiveFrustumSpec "clone"
    let f1 = make_perspective();
    let f2 = f1; // Copy trait = clone
    assert_eq!(f1, f2);
}

// ============================================================================
// OrthographicFrustum (from OrthographicFrustumSpec.js)
// Setup: near=1, far=3, width=2, aspectRatio=1
// ============================================================================

fn make_orthographic() -> OrthographicFrustum {
    OrthographicFrustum::new(2.0, 1.0, 1.0, 3.0)
}

fn orthographic_planes() -> [(DVec3, f64); 6] {
    let f = make_orthographic();
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
fn test_orthographic_constructs() {
    // Ported from: OrthographicFrustumSpec "constructs"
    let f = OrthographicFrustum::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(f.width, 1.0);
    assert_eq!(f.aspect_ratio, 2.0);
    assert_eq!(f.near, 3.0);
    assert_eq!(f.far, 4.0);
}

#[test]
fn test_orthographic_default_constructs() {
    // Ported from: OrthographicFrustumSpec "default constructs"
    // CesiumJS defaults: near=1.0, far=500000000.0
    let f = OrthographicFrustum::new(2.0, 1.0, 1.0, 500_000_000.0);
    assert_eq!(f.near, 1.0);
    assert_eq!(f.far, 500_000_000.0);
}

#[test]
fn test_orthographic_left_plane() {
    // Ported from: OrthographicFrustumSpec "get frustum left plane"
    // Expected: Cartesian4(1, 0, 0, 1)
    let planes = orthographic_planes();
    let (normal, distance) = planes[0];
    assert_approx(normal.x, 1.0, 1e-4, "left.x");
    assert_approx(normal.y, 0.0, 1e-4, "left.y");
    assert_approx(normal.z, 0.0, 1e-4, "left.z");
    assert_approx(distance, 1.0, 1e-4, "left.d");
}

#[test]
fn test_orthographic_right_plane() {
    // Ported from: OrthographicFrustumSpec "get frustum right plane"
    // Expected: Cartesian4(-1, 0, 0, 1)
    let planes = orthographic_planes();
    let (normal, distance) = planes[1];
    assert_approx(normal.x, -1.0, 1e-4, "right.x");
    assert_approx(normal.y, 0.0, 1e-4, "right.y");
    assert_approx(normal.z, 0.0, 1e-4, "right.z");
    assert_approx(distance, 1.0, 1e-4, "right.d");
}

#[test]
fn test_orthographic_bottom_plane() {
    // Ported from: OrthographicFrustumSpec "get frustum bottom plane"
    // Expected: Cartesian4(0, 1, 0, 1)
    let planes = orthographic_planes();
    let (normal, distance) = planes[2];
    assert_approx(normal.x, 0.0, 1e-4, "bottom.x");
    assert_approx(normal.y, 1.0, 1e-4, "bottom.y");
    assert_approx(normal.z, 0.0, 1e-4, "bottom.z");
    assert_approx(distance, 1.0, 1e-4, "bottom.d");
}

#[test]
fn test_orthographic_top_plane() {
    // Ported from: OrthographicFrustumSpec "get frustum top plane"
    // Expected: Cartesian4(0, -1, 0, 1)
    let planes = orthographic_planes();
    let (normal, distance) = planes[3];
    assert_approx(normal.x, 0.0, 1e-4, "top.x");
    assert_approx(normal.y, -1.0, 1e-4, "top.y");
    assert_approx(normal.z, 0.0, 1e-4, "top.z");
    assert_approx(distance, 1.0, 1e-4, "top.d");
}

#[test]
fn test_orthographic_near_plane() {
    // Ported from: OrthographicFrustumSpec "get frustum near plane"
    // Expected: Cartesian4(0, 0, -1, -1)
    let planes = orthographic_planes();
    let (normal, distance) = planes[4];
    assert_approx(normal.x, 0.0, 1e-4, "near.x");
    assert_approx(normal.y, 0.0, 1e-4, "near.y");
    assert_approx(normal.z, -1.0, 1e-4, "near.z");
    assert_approx(distance, -1.0, 1e-4, "near.d");
}

#[test]
fn test_orthographic_far_plane() {
    // Ported from: OrthographicFrustumSpec "get frustum far plane"
    // Expected: Cartesian4(0, 0, 1, 3)
    let planes = orthographic_planes();
    let (normal, distance) = planes[5];
    assert_approx(normal.x, 0.0, 1e-4, "far.x");
    assert_approx(normal.y, 0.0, 1e-4, "far.y");
    assert_approx(normal.z, 1.0, 1e-4, "far.z");
    assert_approx(distance, 3.0, 1e-4, "far.d");
}

#[test]
fn test_orthographic_projection_matrix() {
    // Ported from: OrthographicFrustumSpec "get orthographic projection matrix"
    let f = make_orthographic();
    let proj = f.projection_matrix();

    // Expected: computeOrthographicOffCenter(left=-1, right=1, bottom=-1, top=1, near=1, far=3)
    let left = -1.0_f64;
    let right = 1.0_f64;
    let bottom = -1.0_f64;
    let top = 1.0_f64;
    let near = 1.0_f64;
    let far = 3.0_f64;

    let e0 = 2.0 / (right - left); // = 1
    let e5 = 2.0 / (top - bottom); // = 1
    let e10 = -2.0 / (far - near); // = -1
    let e12 = -(right + left) / (right - left); // = 0
    let e13 = -(top + bottom) / (top - bottom); // = 0
    let e14 = -(far + near) / (far - near); // = -2
    let e15 = 1.0;

    assert_approx(proj.x_axis.x, e0, EPSILON6, "ortho[0][0]");
    assert_approx(proj.y_axis.y, e5, EPSILON6, "ortho[1][1]");
    assert_approx(proj.z_axis.z, e10, EPSILON6, "ortho[2][2]");
    assert_approx(proj.w_axis.x, e12, EPSILON6, "ortho[3][0]");
    assert_approx(proj.w_axis.y, e13, EPSILON6, "ortho[3][1]");
    assert_approx(proj.w_axis.z, e14, EPSILON6, "ortho[3][2]");
    assert_approx(proj.w_axis.w, e15, EPSILON6, "ortho[3][3]");
    // w-row should be (0, 0, 0, 1) for orthographic
    assert_approx(proj.x_axis.w, 0.0, EPSILON6, "ortho[0][3]");
    assert_approx(proj.y_axis.w, 0.0, EPSILON6, "ortho[1][3]");
    assert_approx(proj.z_axis.w, 0.0, EPSILON6, "ortho[2][3]");
}

#[test]
fn test_orthographic_pixel_dimensions() {
    // Ported from: OrthographicFrustumSpec "get pixel dimensions"
    let f = make_orthographic();
    let (pw, ph) = f.pixel_dimensions(1.0, 1.0, 1.0, 1.0);

    // Expected: pixelWidth = pixelRatio * width / drawingBufferWidth = 1*2/1 = 2
    //           pixelHeight = pixelRatio * height / drawingBufferHeight = 1*2/1 = 2
    assert_approx(pw, 2.0, EPSILON14, "ortho pixelWidth");
    assert_approx(ph, 2.0, EPSILON14, "ortho pixelHeight");
}

#[test]
fn test_orthographic_pixel_dimensions_with_pixel_ratio() {
    // Ported from: OrthographicFrustumSpec "get pixel dimensions with pixel ratio"
    let f = make_orthographic();
    let (pw, ph) = f.pixel_dimensions(1.0, 1.0, 1.0, 2.0);

    // Expected: pixelWidth = 2*2/1 = 4, pixelHeight = 2*2/1 = 4
    assert_approx(pw, 4.0, EPSILON14, "ortho pixelWidth ratio=2");
    assert_approx(ph, 4.0, EPSILON14, "ortho pixelHeight ratio=2");
}

#[test]
fn test_orthographic_equals() {
    // Ported from: OrthographicFrustumSpec "equals"
    let f1 = make_orthographic();
    let f2 = OrthographicFrustum::new(2.0, 1.0, 1.0, 3.0);
    assert_eq!(f1, f2);
}

#[test]
fn test_orthographic_equals_epsilon() {
    // Ported from: OrthographicFrustumSpec "equals epsilon"
    let f1 = make_orthographic();

    let f2 = OrthographicFrustum::new(2.0, 1.0, 1.0, 3.0);
    assert!((f1.width - f2.width).abs() < EPSILON6);

    // Slightly different → within EPSILON1
    let f3 = OrthographicFrustum::new(2.01, 1.01, 1.01, 3.01);
    assert!((f1.width - f3.width).abs() < EPSILON1);
    assert!((f1.aspect_ratio - f3.aspect_ratio).abs() < EPSILON1);
    assert!((f1.near - f3.near).abs() < EPSILON1);
    assert!((f1.far - f3.far).abs() < EPSILON1);

    // More different → NOT within EPSILON2
    let f4 = OrthographicFrustum::new(2.0, 1.1, 1.0, 3.0);
    assert!((f1.aspect_ratio - f4.aspect_ratio).abs() > EPSILON2);
}

#[test]
fn test_orthographic_clone() {
    // Ported from: OrthographicFrustumSpec "clone"
    let f1 = make_orthographic();
    let f2 = f1; // Copy trait
    assert_eq!(f1, f2);
}
