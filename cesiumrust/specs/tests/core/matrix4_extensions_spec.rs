//! Core/Matrix4Spec.js (CesiumJS-specific extensions) → Rust integration tests
//! Covers: fromRotationTranslation, fromTranslation, fromScale, fromUniformScale,
//! getTranslation, getScale, getMaximumScale, getRotation, multiplyByTranslation,
//! multiplyByScale, computePerspectiveFieldOfView, pack/unpack, equalsEpsilon

use cesium_geospatial::matrix4_ext;
use glam::{DMat3, DMat4, DVec3};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

const EPS: f64 = 1e-10;

fn assert_mat4_eq(actual: &DMat4, expected: &DMat4, msg: &str) {
    let a = actual.to_cols_array();
    let e = expected.to_cols_array();
    for i in 0..16 {
        assert!(
            (a[i] - e[i]).abs() < EPS,
            "{msg}: element[{i}] expected {}, got {}",
            e[i],
            a[i]
        );
    }
}

// ─── fromRotationTranslation ────────────────────────────────────────────────

#[test]
fn from_rotation_translation_basic() {
    let rotation = DMat3::IDENTITY;
    let translation = DVec3::new(1.0, 2.0, 3.0);
    let result = matrix4_ext::from_rotation_translation(&rotation, translation);

    let expected = DMat4::from_cols_array(&[
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        1.0, 2.0, 3.0, 1.0,
    ]);
    assert_mat4_eq(&result, &expected, "fromRotationTranslation identity");
}

#[test]
fn from_rotation_translation_with_rotation() {
    // 90° rotation around Z axis
    let rotation = DMat3::from_rotation_z(FRAC_PI_2);
    let translation = DVec3::new(10.0, 20.0, 30.0);
    let result = matrix4_ext::from_rotation_translation(&rotation, translation);

    // Check translation column
    assert!((result.w_axis.x - 10.0).abs() < EPS);
    assert!((result.w_axis.y - 20.0).abs() < EPS);
    assert!((result.w_axis.z - 30.0).abs() < EPS);
    assert!((result.w_axis.w - 1.0).abs() < EPS);

    // Check rotation part (cos90≈0, sin90≈1)
    assert!(result.x_axis.x.abs() < EPS); // cos(90) ≈ 0
    assert!((result.x_axis.y - 1.0).abs() < EPS); // sin(90) ≈ 1
}

// ─── fromTranslation ────────────────────────────────────────────────────────

#[test]
fn from_translation_basic() {
    let t = DVec3::new(5.0, 6.0, 7.0);
    let result = matrix4_ext::from_translation(t);
    let expected = DMat4::from_translation(t);
    assert_mat4_eq(&result, &expected, "fromTranslation");
}

// ─── fromScale / fromUniformScale ───────────────────────────────────────────

#[test]
fn from_scale_basic() {
    let s = DVec3::new(2.0, 3.0, 4.0);
    let result = matrix4_ext::from_scale(s);
    let expected = DMat4::from_scale(s);
    assert_mat4_eq(&result, &expected, "fromScale");
}

#[test]
fn from_uniform_scale_basic() {
    let result = matrix4_ext::from_uniform_scale(3.0);
    let expected = DMat4::from_scale(DVec3::splat(3.0));
    assert_mat4_eq(&result, &expected, "fromUniformScale");
}

// ─── getTranslation ─────────────────────────────────────────────────────────

#[test]
fn get_translation_basic() {
    let m = DMat4::from_translation(DVec3::new(1.0, 2.0, 3.0));
    let t = matrix4_ext::get_translation(&m);
    assert!((t.x - 1.0).abs() < EPS);
    assert!((t.y - 2.0).abs() < EPS);
    assert!((t.z - 3.0).abs() < EPS);
}

#[test]
fn get_translation_from_composed() {
    let rotation = DMat3::from_rotation_z(FRAC_PI_4);
    let translation = DVec3::new(10.0, 20.0, 30.0);
    let m = matrix4_ext::from_rotation_translation(&rotation, translation);
    let t = matrix4_ext::get_translation(&m);
    assert!((t.x - 10.0).abs() < EPS);
    assert!((t.y - 20.0).abs() < EPS);
    assert!((t.z - 30.0).abs() < EPS);
}

// ─── getScale ───────────────────────────────────────────────────────────────

#[test]
fn get_scale_identity() {
    let m = DMat4::IDENTITY;
    let s = matrix4_ext::get_scale(&m);
    assert!((s.x - 1.0).abs() < EPS);
    assert!((s.y - 1.0).abs() < EPS);
    assert!((s.z - 1.0).abs() < EPS);
}

#[test]
fn get_scale_non_uniform() {
    let m = DMat4::from_scale(DVec3::new(2.0, 3.0, 4.0));
    let s = matrix4_ext::get_scale(&m);
    assert!((s.x - 2.0).abs() < EPS);
    assert!((s.y - 3.0).abs() < EPS);
    assert!((s.z - 4.0).abs() < EPS);
}

#[test]
fn get_scale_with_rotation() {
    let rotation = DMat3::from_rotation_z(FRAC_PI_4);
    let m = matrix4_ext::from_rotation_translation(&rotation, DVec3::ZERO);
    let m = m * DMat4::from_scale(DVec3::new(2.0, 3.0, 4.0));
    let s = matrix4_ext::get_scale(&m);
    assert!((s.x - 2.0).abs() < 1e-9);
    assert!((s.y - 3.0).abs() < 1e-9);
    assert!((s.z - 4.0).abs() < 1e-9);
}

// ─── getMaximumScale ────────────────────────────────────────────────────────

#[test]
fn get_maximum_scale_basic() {
    let m = DMat4::from_scale(DVec3::new(2.0, 5.0, 3.0));
    let max_s = matrix4_ext::get_maximum_scale(&m);
    assert!((max_s - 5.0).abs() < EPS);
}

// ─── getRotation ────────────────────────────────────────────────────────────

#[test]
fn get_rotation_identity() {
    let m = DMat4::IDENTITY;
    let r = matrix4_ext::get_rotation(&m);
    let expected = DMat3::IDENTITY;
    let r_arr = r.to_cols_array();
    let e_arr = expected.to_cols_array();
    for i in 0..9 {
        assert!((r_arr[i] - e_arr[i]).abs() < EPS, "getRotation identity[{i}]");
    }
}

#[test]
fn get_rotation_removes_scale() {
    let rotation = DMat3::from_rotation_z(FRAC_PI_4);
    let m = matrix4_ext::from_rotation_translation(&rotation, DVec3::new(1.0, 2.0, 3.0));
    // Apply scale
    let scaled = DMat4::from_cols(
        m.x_axis * 2.0,
        m.y_axis * 3.0,
        m.z_axis * 4.0,
        m.w_axis,
    );
    let r = matrix4_ext::get_rotation(&scaled);
    // Should recover the original rotation
    let r_arr = r.to_cols_array();
    let rot_arr = rotation.to_cols_array();
    for i in 0..9 {
        assert!(
            (r_arr[i] - rot_arr[i]).abs() < 1e-9,
            "getRotation removes scale[{i}]: expected {}, got {}",
            rot_arr[i],
            r_arr[i]
        );
    }
}

// ─── multiplyByTranslation ──────────────────────────────────────────────────

#[test]
fn multiply_by_translation_identity() {
    let m = DMat4::IDENTITY;
    let t = DVec3::new(1.0, 2.0, 3.0);
    let result = matrix4_ext::multiply_by_translation(&m, t);
    let expected = DMat4::from_translation(t);
    assert_mat4_eq(&result, &expected, "multiplyByTranslation identity");
}

#[test]
fn multiply_by_translation_composed() {
    let m = DMat4::from_translation(DVec3::new(10.0, 0.0, 0.0));
    let t = DVec3::new(0.0, 5.0, 0.0);
    let result = matrix4_ext::multiply_by_translation(&m, t);
    // Should be equivalent to m * fromTranslation(t)
    let expected = m * DMat4::from_translation(t);
    assert_mat4_eq(&result, &expected, "multiplyByTranslation composed");
}

// ─── multiplyByScale ────────────────────────────────────────────────────────

#[test]
fn multiply_by_scale_basic() {
    let m = DMat4::IDENTITY;
    let s = DVec3::new(2.0, 3.0, 4.0);
    let result = matrix4_ext::multiply_by_scale(&m, s);
    let expected = DMat4::from_scale(s);
    assert_mat4_eq(&result, &expected, "multiplyByScale identity");
}

#[test]
fn multiply_by_scale_unity_is_clone() {
    let m = DMat4::from_translation(DVec3::new(1.0, 2.0, 3.0));
    let result = matrix4_ext::multiply_by_scale(&m, DVec3::ONE);
    assert_mat4_eq(&result, &m, "multiplyByScale unity");
}

#[test]
fn multiply_by_scale_composed() {
    let m = DMat4::from_translation(DVec3::new(5.0, 6.0, 7.0));
    let s = DVec3::new(2.0, 3.0, 4.0);
    let result = matrix4_ext::multiply_by_scale(&m, s);
    let expected = m * DMat4::from_scale(s);
    assert_mat4_eq(&result, &expected, "multiplyByScale composed");
}

// ─── computePerspectiveFieldOfView ──────────────────────────────────────────

#[test]
fn compute_perspective_fov_basic() {
    let fov_y = FRAC_PI_2; // 90 degrees
    let aspect = 1.0;
    let near = 1.0;
    let far = 100.0;
    let result = matrix4_ext::compute_perspective_field_of_view(fov_y, aspect, near, far);

    let bottom = (fov_y * 0.5).tan();
    let col1_row1 = 1.0 / bottom;
    let col0_row0 = col1_row1 / aspect;
    let col2_row2 = (far + near) / (near - far);
    let col3_row2 = (2.0 * far * near) / (near - far);

    let arr = result.to_cols_array();
    assert!((arr[0] - col0_row0).abs() < EPS, "col0Row0");
    assert!((arr[5] - col1_row1).abs() < EPS, "col1Row1");
    assert!((arr[10] - col2_row2).abs() < EPS, "col2Row2");
    assert!((arr[11] - (-1.0)).abs() < EPS, "col2Row3 = -1");
    assert!((arr[14] - col3_row2).abs() < EPS, "col3Row2");
    assert!((arr[15] - 0.0).abs() < EPS, "col3Row3 = 0");
}

// ─── pack / unpack ──────────────────────────────────────────────────────────

#[test]
fn pack_basic() {
    let m = DMat4::from_translation(DVec3::new(1.0, 2.0, 3.0));
    let mut array = vec![0.0; 16];
    matrix4_ext::pack(&m, &mut array, 0);
    let unpacked = matrix4_ext::unpack(&array, 0);
    assert_mat4_eq(&unpacked, &m, "pack roundtrip");
}

#[test]
fn unpack_with_offset() {
    let m = DMat4::from_scale(DVec3::new(2.0, 3.0, 4.0));
    let mut array = vec![0.0; 32];
    matrix4_ext::pack(&m, &mut array, 16);
    let result = matrix4_ext::unpack(&array, 16);
    assert_mat4_eq(&result, &m, "unpack with offset");
}

#[test]
fn pack_unpack_roundtrip() {
    let rotation = DMat3::from_rotation_z(0.7);
    let m = matrix4_ext::from_rotation_translation(&rotation, DVec3::new(-1.0, 2.5, 3.14));
    let mut array = vec![0.0; 16];
    matrix4_ext::pack(&m, &mut array, 0);
    let result = matrix4_ext::unpack(&array, 0);
    assert_mat4_eq(&result, &m, "pack/unpack roundtrip");
}

// ─── equalsEpsilon ──────────────────────────────────────────────────────────

#[test]
fn equals_epsilon_exact() {
    let m = DMat4::from_translation(DVec3::new(1.0, 2.0, 3.0));
    assert!(matrix4_ext::equals_epsilon(&m, &m, 0.0));
}

#[test]
fn equals_epsilon_within() {
    let m1 = DMat4::IDENTITY;
    let m2 = DMat4::from_translation(DVec3::new(1e-12, 0.0, 0.0));
    assert!(matrix4_ext::equals_epsilon(&m1, &m2, 1e-10));
}

#[test]
fn equals_epsilon_outside() {
    let m1 = DMat4::IDENTITY;
    let m2 = DMat4::from_translation(DVec3::new(1.0, 0.0, 0.0));
    assert!(!matrix4_ext::equals_epsilon(&m1, &m2, 0.5));
}
