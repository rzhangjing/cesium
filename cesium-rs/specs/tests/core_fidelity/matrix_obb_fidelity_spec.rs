//! Mirrors of the CesiumJS Jasmine spec cases backing the Track A1/A4-A7
//! fidelity fixes:
//!
//! - `packages/engine/Specs/Core/Matrix4Spec.js`
//!   (`multiplyByPoint works` / `multiplyByPointAsVector works`)
//! - `packages/engine/Specs/Core/PerspectiveOffCenterFrustumSpec.js`
//!   (`get perspective projection matrix`, concrete-element variant)
//! - `packages/engine/Specs/Core/OrientedBoundingBoxSpec.js`
//!   (`fromRectangle ...` family)
//!
//! Conventions:
//! - Jasmine `it(...)` titles map to `#[test] fn` names (snake_case).
//! - `toEqualEpsilon` -> `assert_*_epsilon` helpers (relative epsilon).
//! - `toThrowDeveloperError` -> `#[should_panic]` / `catch_unwind` (debug).

use std::panic::{catch_unwind, AssertUnwindSafe};

use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::math::CesiumMath;
use cesium_core::matrix3::Matrix3;
use cesium_core::matrix4::Matrix4;
use cesium_core::oriented_bounding_box::OrientedBoundingBox;
use cesium_core::perspective_off_center_frustum::PerspectiveOffCenterFrustum;
use cesium_core::rectangle::Rectangle;

fn assert_f64_epsilon(actual: f64, expected: f64, epsilon: f64, label: &str) {
    assert!(
        CesiumMath::equals_epsilon(actual, expected, Some(epsilon), None),
        "{label}: expected {expected}, got {actual}"
    );
}

fn assert_cartesian3_epsilon(
    actual: &Cartesian3,
    expected: &Cartesian3,
    epsilon: f64,
    label: &str,
) {
    assert_f64_epsilon(actual.x, expected.x, epsilon, &format!("{label}.x"));
    assert_f64_epsilon(actual.y, expected.y, epsilon, &format!("{label}.y"));
    assert_f64_epsilon(actual.z, expected.z, epsilon, &format!("{label}.z"));
}

fn assert_matrix3_epsilon(actual: &Matrix3, expected: &Matrix3, epsilon: f64, label: &str) {
    for i in 0..9 {
        assert_f64_epsilon(
            actual.elements[i],
            expected.elements[i],
            epsilon,
            &format!("{label}[{i}]"),
        );
    }
}

// ---------------------------------------------------------------------------
// Matrix4Spec.js: "multiplyByPoint works" / "multiplyByPointAsVector works"
// ---------------------------------------------------------------------------

#[test]
fn multiply_by_point_works() {
    let matrix = Matrix4::new(
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
    );
    let point = Cartesian3::new(17.0, 18.0, 19.0);
    let expected = Cartesian3::new(114.0, 334.0, 554.0);
    let mut result = Cartesian3::ZERO;
    Matrix4::multiply_by_point(&matrix, &point, &mut result);
    assert_eq!(result, expected);
}

#[test]
fn multiply_by_point_as_vector_works() {
    let matrix = Matrix4::new(
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
    );
    let point = Cartesian3::new(17.0, 18.0, 19.0);
    // w == 0 semantics: the translation column must not be applied.
    let expected = Cartesian3::new(110.0, 326.0, 542.0);
    let mut result = Cartesian3::ZERO;
    Matrix4::multiply_by_point_as_vector(&matrix, &point, &mut result);
    assert_eq!(result, expected);
}

// ---------------------------------------------------------------------------
// PerspectiveOffCenterFrustumSpec.js: projection matrix element layout
// ---------------------------------------------------------------------------

#[test]
fn compute_projection_matrix_element_layout() {
    let mut frustum = PerspectiveOffCenterFrustum::new();
    frustum.left = Some(-1.0);
    frustum.right = Some(3.0);
    frustum.bottom = Some(-2.0);
    frustum.top = Some(4.0);
    frustum.near = 1.0;
    frustum.far = 2.0;

    let m = frustum.compute_projection_matrix();

    let col0x = 2.0 * frustum.near / (3.0 - -1.0); // 0.5
    let col1y = 2.0 * frustum.near / (4.0 - -2.0); // 1/3
    let col2x = (3.0 + -1.0) / (3.0 - -1.0); // 0.5
    let col2y = (4.0 + -2.0) / (4.0 - -2.0); // 1/3
    let col2z = -(frustum.far + frustum.near) / (frustum.far - frustum.near); // -3
    let col3z = -2.0 * frustum.far * frustum.near / (frustum.far - frustum.near); // -4

    // Column-major: col0 = [col0x,0,0,0], col1 = [0,col1y,0,0],
    // col2 = [col2x,col2y,col2z,-1], col3 = [0,0,col3z,0].
    assert_f64_epsilon(m.elements[0], col0x, CesiumMath::EPSILON15, "m[0]");
    assert_f64_epsilon(m.elements[5], col1y, CesiumMath::EPSILON15, "m[5]");
    assert_f64_epsilon(m.elements[8], col2x, CesiumMath::EPSILON15, "m[8]");
    assert_f64_epsilon(m.elements[9], col2y, CesiumMath::EPSILON15, "m[9]");
    assert_f64_epsilon(m.elements[10], col2z, CesiumMath::EPSILON15, "m[10]");
    assert_f64_epsilon(m.elements[11], -1.0, CesiumMath::EPSILON15, "m[11]");
    assert_f64_epsilon(m.elements[14], col3z, CesiumMath::EPSILON15, "m[14]");
    for i in [1, 2, 3, 4, 6, 7, 12, 13, 15] {
        assert_f64_epsilon(m.elements[i], 0.0, CesiumMath::EPSILON15, &format!("m[{i}]"));
    }
}

// ---------------------------------------------------------------------------
// OrientedBoundingBoxSpec.js: fromRectangle family
// ---------------------------------------------------------------------------

#[test]
fn from_rectangle_sets_correct_default_ellipsoid() {
    let rectangle = Rectangle::new(-0.9, -1.2, 0.5, 0.7);
    let box1 = OrientedBoundingBox::from_rectangle(Some(&rectangle), Some(0.0), Some(0.0), None, None);
    let box2 = OrientedBoundingBox::from_rectangle(
        Some(&rectangle),
        Some(0.0),
        Some(0.0),
        Some(Ellipsoid::WGS84),
        None,
    );

    assert_cartesian3_epsilon(&box1.center, &box2.center, CesiumMath::EPSILON15, "center");
    assert_matrix3_epsilon(&box1.half_axes, &box2.half_axes, CesiumMath::EPSILON15, "halfAxes");
}

#[test]
fn from_rectangle_sets_correct_default_heights() {
    let rectangle = Rectangle::new(0.0, 0.0, 0.0, 0.0);
    let obb =
        OrientedBoundingBox::from_rectangle(Some(&rectangle), None, None, Some(Ellipsoid::UNIT_SPHERE), None);

    assert_cartesian3_epsilon(
        &obb.center,
        &Cartesian3::new(1.0, 0.0, 0.0),
        CesiumMath::EPSILON15,
        "center",
    );
    assert_matrix3_epsilon(&obb.half_axes, &Matrix3::ZERO, CesiumMath::EPSILON15, "halfAxes");
}

#[test]
#[should_panic]
fn from_rectangle_throws_without_rectangle() {
    OrientedBoundingBox::from_rectangle(None, Some(0.0), Some(0.0), Some(Ellipsoid::UNIT_SPHERE), None);
}

#[test]
fn from_rectangle_throws_with_invalid_rectangles() {
    let ellipsoid = Ellipsoid::UNIT_SPHERE;
    let invalid = [
        Rectangle::new(-1.0, 1.0, 1.0, -1.0),
        Rectangle::new(-2.0, 2.0, -1.0, 1.0),
        Rectangle::new(-4.0, -2.0, 4.0, 1.0),
        Rectangle::new(-2.0, -2.0, 1.0, 2.0),
        Rectangle::new(-1.0, -2.0, 2.0, 2.0),
        Rectangle::new(-4.0, -1.0, 4.0, 2.0),
    ];
    for rectangle in &invalid {
        let ellipsoid = ellipsoid.clone();
        let rectangle = rectangle.clone();
        let result = catch_unwind(AssertUnwindSafe(move || {
            OrientedBoundingBox::from_rectangle(
                Some(&rectangle),
                Some(0.0),
                Some(0.0),
                Some(ellipsoid),
                None,
            )
        }));
        assert!(result.is_err(), "expected DeveloperError for {rectangle:?}");
    }
}

#[test]
fn from_rectangle_throws_with_non_revolution_ellipsoids() {
    let rectangle = Rectangle::new(0.0, 0.0, 0.0, 0.0);
    for ellipsoid in [Ellipsoid::new(1.01, 1.0, 1.01), Ellipsoid::new(1.0, 1.01, 1.01)] {
        let rectangle = rectangle.clone();
        let result = catch_unwind(AssertUnwindSafe(move || {
            OrientedBoundingBox::from_rectangle(Some(&rectangle), Some(0.0), Some(0.0), Some(ellipsoid), None)
        }));
        assert!(result.is_err(), "expected DeveloperError for {ellipsoid:?}");
    }
}

#[test]
fn from_rectangle_creates_a_box_without_a_result_parameter() {
    let rectangle = Rectangle::new(0.0, 0.0, 0.0, 0.0);
    let obb = OrientedBoundingBox::from_rectangle(
        Some(&rectangle),
        Some(0.0),
        Some(0.0),
        Some(Ellipsoid::UNIT_SPHERE),
        None,
    );

    assert_cartesian3_epsilon(
        &obb.center,
        &Cartesian3::new(1.0, 0.0, 0.0),
        CesiumMath::EPSILON15,
        "center",
    );
    assert_matrix3_epsilon(&obb.half_axes, &Matrix3::ZERO, CesiumMath::EPSILON15, "halfAxes");
}

#[test]
fn from_rectangle_creates_a_box_with_a_result_parameter() {
    let rectangle = Rectangle::new(0.0, 0.0, 0.0, 0.0);
    let mut result = OrientedBoundingBox::default();
    let obb = OrientedBoundingBox::from_rectangle(
        Some(&rectangle),
        Some(0.0),
        Some(0.0),
        Some(Ellipsoid::UNIT_SPHERE),
        Some(&mut result),
    );

    // JS: `expect(box).toBe(result)` -- in Rust the `result` slot is written
    // through; verify it carries the same values.
    assert_cartesian3_epsilon(
        &result.center,
        &Cartesian3::new(1.0, 0.0, 0.0),
        CesiumMath::EPSILON15,
        "result.center",
    );
    assert_matrix3_epsilon(&result.half_axes, &Matrix3::ZERO, CesiumMath::EPSILON15, "result.halfAxes");
    assert_cartesian3_epsilon(&obb.center, &result.center, CesiumMath::EPSILON15, "obb.center");
    assert_matrix3_epsilon(&obb.half_axes, &result.half_axes, CesiumMath::EPSILON15, "box.halfAxes");
}

#[test]
fn from_rectangle_for_rectangles_with_heights() {
    let d90 = CesiumMath::PI_OVER_TWO;

    let obb = OrientedBoundingBox::from_rectangle(
        Some(&Rectangle::new(0.0, 0.0, 0.0, 0.0)),
        Some(1.0),
        Some(1.0),
        Some(Ellipsoid::UNIT_SPHERE),
        None,
    );
    assert_cartesian3_epsilon(
        &obb.center,
        &Cartesian3::new(2.0, 0.0, 0.0),
        CesiumMath::EPSILON15,
        "center(1,1)",
    );
    assert_matrix3_epsilon(&obb.half_axes, &Matrix3::ZERO, CesiumMath::EPSILON15, "halfAxes(1,1)");

    let obb = OrientedBoundingBox::from_rectangle(
        Some(&Rectangle::new(0.0, 0.0, 0.0, 0.0)),
        Some(-1.0),
        Some(-1.0),
        Some(Ellipsoid::UNIT_SPHERE),
        None,
    );
    assert_cartesian3_epsilon(
        &obb.center,
        &Cartesian3::new(0.0, 0.0, 0.0),
        CesiumMath::EPSILON15,
        "center(-1,-1)",
    );
    assert_matrix3_epsilon(&obb.half_axes, &Matrix3::ZERO, CesiumMath::EPSILON15, "halfAxes(-1,-1)");

    let obb = OrientedBoundingBox::from_rectangle(
        Some(&Rectangle::new(0.0, 0.0, 0.0, 0.0)),
        Some(-1.0),
        Some(1.0),
        Some(Ellipsoid::UNIT_SPHERE),
        None,
    );
    assert_cartesian3_epsilon(
        &obb.center,
        &Cartesian3::new(1.0, 0.0, 0.0),
        CesiumMath::EPSILON15,
        "center(-1,1)",
    );
    assert_matrix3_epsilon(
        &obb.half_axes,
        &Matrix3::new(0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
        CesiumMath::EPSILON15,
        "halfAxes(-1,1)",
    );

    let obb = OrientedBoundingBox::from_rectangle(
        Some(&Rectangle::new(-d90, -d90, d90, d90)),
        Some(0.0),
        Some(1.0),
        Some(Ellipsoid::UNIT_SPHERE),
        None,
    );
    assert_cartesian3_epsilon(
        &obb.center,
        &Cartesian3::new(1.0, 0.0, 0.0),
        CesiumMath::EPSILON15,
        "center(0,1)",
    );
    assert_matrix3_epsilon(
        &obb.half_axes,
        &Matrix3::new(0.0, 0.0, 1.0, 2.0, 0.0, 0.0, 0.0, 2.0, 0.0),
        CesiumMath::EPSILON15,
        "halfAxes(0,1)",
    );

    let obb = OrientedBoundingBox::from_rectangle(
        Some(&Rectangle::new(-d90, -d90, d90, d90)),
        Some(-1.0),
        Some(-1.0),
        Some(Ellipsoid::UNIT_SPHERE),
        None,
    );
    assert_cartesian3_epsilon(
        &obb.center,
        &Cartesian3::new(0.0, 0.0, 0.0),
        CesiumMath::EPSILON15,
        "center(-1,-1)",
    );
    assert_matrix3_epsilon(&obb.half_axes, &Matrix3::ZERO, CesiumMath::EPSILON15, "halfAxes(-1,-1)");

    let obb = OrientedBoundingBox::from_rectangle(
        Some(&Rectangle::new(-d90, -d90, d90, d90)),
        Some(-1.0),
        Some(0.0),
        Some(Ellipsoid::UNIT_SPHERE),
        None,
    );
    assert_cartesian3_epsilon(
        &obb.center,
        &Cartesian3::new(0.5, 0.0, 0.0),
        CesiumMath::EPSILON15,
        "center(-1,0)",
    );
    assert_matrix3_epsilon(
        &obb.half_axes,
        &Matrix3::new(0.0, 0.0, 0.5, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0),
        CesiumMath::EPSILON15,
        "halfAxes(-1,0)",
    );
}

#[test]
fn from_rectangle_for_rectangles_that_span_over_half_the_ellipsoid() {
    let d90 = CesiumMath::PI_OVER_TWO;
    let d180 = CesiumMath::PI;
    let d135 = (3.0 / 4.0) * CesiumMath::PI;
    let d45 = CesiumMath::PI_OVER_FOUR;
    let one_plus_sqrt_half_div_two = (1.0 + std::f64::consts::FRAC_1_SQRT_2) / 2.0;
    let one_minus_one_plus_sqrt_half_div_two = 1.0 - one_plus_sqrt_half_div_two;
    let sqrt_two_minus_one_div_four = (std::f64::consts::SQRT_2 - 1.0) / 4.0;
    let sqrt_two_plus_one_div_four = (std::f64::consts::SQRT_2 + 1.0) / 4.0;
    let sqrt_half = std::f64::consts::FRAC_1_SQRT_2;

    // Entire ellipsoid
    let obb = OrientedBoundingBox::from_rectangle(
        Some(&Rectangle::new(-d180, -d90, d180, d90)),
        Some(0.0),
        Some(0.0),
        Some(Ellipsoid::UNIT_SPHERE),
        None,
    );
    assert_cartesian3_epsilon(
        &obb.center,
        &Cartesian3::new(0.0, 0.0, 0.0),
        CesiumMath::EPSILON15,
        "full.center",
    );
    assert_matrix3_epsilon(
        &obb.half_axes,
        &Matrix3::new(0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0),
        CesiumMath::EPSILON15,
        "full.halfAxes",
    );

    // 3/4s of longitude, full latitude
    let obb = OrientedBoundingBox::from_rectangle(
        Some(&Rectangle::new(-d135, -d90, d135, d90)),
        Some(0.0),
        Some(0.0),
        Some(Ellipsoid::UNIT_SPHERE),
        None,
    );
    assert_cartesian3_epsilon(
        &obb.center,
        &Cartesian3::new(one_minus_one_plus_sqrt_half_div_two, 0.0, 0.0),
        CesiumMath::EPSILON15,
        "3/4lon.center",
    );
    assert_matrix3_epsilon(
        &obb.half_axes,
        &Matrix3::new(
            0.0,
            0.0,
            one_plus_sqrt_half_div_two,
            1.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
        ),
        CesiumMath::EPSILON15,
        "3/4lon.halfAxes",
    );

    // 3/4s of longitude, 1/2 of latitude centered at equator
    let obb = OrientedBoundingBox::from_rectangle(
        Some(&Rectangle::new(-d135, -d45, d135, d45)),
        Some(0.0),
        Some(0.0),
        Some(Ellipsoid::UNIT_SPHERE),
        None,
    );
    assert_cartesian3_epsilon(
        &obb.center,
        &Cartesian3::new(one_minus_one_plus_sqrt_half_div_two, 0.0, 0.0),
        CesiumMath::EPSILON15,
        "3/4lon-halfLat.center",
    );
    assert_matrix3_epsilon(
        &obb.half_axes,
        &Matrix3::new(
            0.0,
            0.0,
            one_plus_sqrt_half_div_two,
            1.0,
            0.0,
            0.0,
            0.0,
            sqrt_half,
            0.0,
        ),
        CesiumMath::EPSILON15,
        "3/4lon-halfLat.halfAxes",
    );

    // 3/4s of longitude centered at IDL, 1/2 of latitude centered at equator
    let obb = OrientedBoundingBox::from_rectangle(
        Some(&Rectangle::new(d180, -d45, d90, d45)),
        Some(0.0),
        Some(0.0),
        Some(Ellipsoid::UNIT_SPHERE),
        None,
    );
    assert_cartesian3_epsilon(
        &obb.center,
        &Cartesian3::new(sqrt_two_minus_one_div_four, -sqrt_two_minus_one_div_four, 0.0),
        CesiumMath::EPSILON15,
        "IDL.center",
    );
    assert_matrix3_epsilon(
        &obb.half_axes,
        &Matrix3::new(
            sqrt_half,
            0.0,
            sqrt_two_plus_one_div_four,
            sqrt_half,
            0.0,
            -sqrt_two_plus_one_div_four,
            0.0,
            sqrt_half,
            0.0,
        ),
        CesiumMath::EPSILON15,
        "IDL.halfAxes",
    );

    // Full longitude, 1/2 of latitude centered at equator
    let obb = OrientedBoundingBox::from_rectangle(
        Some(&Rectangle::new(-d180, -d45, d180, d45)),
        Some(0.0),
        Some(0.0),
        Some(Ellipsoid::UNIT_SPHERE),
        None,
    );
    assert_cartesian3_epsilon(
        &obb.center,
        &Cartesian3::new(0.0, 0.0, 0.0),
        CesiumMath::EPSILON15,
        "fullLon-halfLat.center",
    );
    assert_matrix3_epsilon(
        &obb.half_axes,
        &Matrix3::new(0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, sqrt_half, 0.0),
        CesiumMath::EPSILON15,
        "fullLon-halfLat.halfAxes",
    );

    // Full longitude, 1/4 of latitude starting from north pole
    let obb = OrientedBoundingBox::from_rectangle(
        Some(&Rectangle::new(-d180, d45, d180, d90)),
        Some(0.0),
        Some(0.0),
        Some(Ellipsoid::UNIT_SPHERE),
        None,
    );
    assert_cartesian3_epsilon(
        &obb.center,
        &Cartesian3::new(0.0, 0.0, one_plus_sqrt_half_div_two),
        CesiumMath::EPSILON15,
        "northQuarter.center",
    );
    assert_matrix3_epsilon(
        &obb.half_axes,
        &Matrix3::new(
            0.0,
            0.0,
            sqrt_half,
            sqrt_half,
            0.0,
            0.0,
            0.0,
            one_minus_one_plus_sqrt_half_div_two,
            0.0,
        ),
        CesiumMath::EPSILON15,
        "northQuarter.halfAxes",
    );

    // Full longitude, 1/4 of latitude starting from south pole
    let obb = OrientedBoundingBox::from_rectangle(
        Some(&Rectangle::new(-d180, -d90, d180, -d45)),
        Some(0.0),
        Some(0.0),
        Some(Ellipsoid::UNIT_SPHERE),
        None,
    );
    assert_cartesian3_epsilon(
        &obb.center,
        &Cartesian3::new(0.0, 0.0, -one_plus_sqrt_half_div_two),
        CesiumMath::EPSILON15,
        "southQuarter.center",
    );
    assert_matrix3_epsilon(
        &obb.half_axes,
        &Matrix3::new(
            0.0,
            0.0,
            sqrt_half,
            sqrt_half,
            0.0,
            0.0,
            0.0,
            one_minus_one_plus_sqrt_half_div_two,
            0.0,
        ),
        CesiumMath::EPSILON15,
        "southQuarter.halfAxes",
    );

    // Completely on north pole
    let obb = OrientedBoundingBox::from_rectangle(
        Some(&Rectangle::new(-d180, d90, d180, d90)),
        Some(0.0),
        Some(0.0),
        Some(Ellipsoid::UNIT_SPHERE),
        None,
    );
    assert_cartesian3_epsilon(
        &obb.center,
        &Cartesian3::new(0.0, 0.0, 1.0),
        CesiumMath::EPSILON15,
        "northPole.center",
    );
    assert_matrix3_epsilon(&obb.half_axes, &Matrix3::ZERO, CesiumMath::EPSILON15, "northPole.halfAxes");

    // Completely on north pole 2
    let obb = OrientedBoundingBox::from_rectangle(
        Some(&Rectangle::new(-d135, d90, d135, d90)),
        Some(0.0),
        Some(0.0),
        Some(Ellipsoid::UNIT_SPHERE),
        None,
    );
    assert_cartesian3_epsilon(
        &obb.center,
        &Cartesian3::new(0.0, 0.0, 1.0),
        CesiumMath::EPSILON15,
        "northPole2.center",
    );
    assert_matrix3_epsilon(&obb.half_axes, &Matrix3::ZERO, CesiumMath::EPSILON15, "northPole2.halfAxes");

    // Completely on south pole
    let obb = OrientedBoundingBox::from_rectangle(
        Some(&Rectangle::new(-d180, -d90, d180, -d90)),
        Some(0.0),
        Some(0.0),
        Some(Ellipsoid::UNIT_SPHERE),
        None,
    );
    assert_cartesian3_epsilon(
        &obb.center,
        &Cartesian3::new(0.0, 0.0, -1.0),
        CesiumMath::EPSILON15,
        "southPole.center",
    );
    assert_matrix3_epsilon(&obb.half_axes, &Matrix3::ZERO, CesiumMath::EPSILON15, "southPole.halfAxes");
}
