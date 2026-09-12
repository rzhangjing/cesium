use cesium_core::cartesian3::Cartesian3;
use cesium_core::intersect::Intersect;
use cesium_core::math::CesiumMath;
use cesium_core::matrix3::Matrix3;
use cesium_core::matrix4::Matrix4;
use cesium_core::oriented_bounding_box::OrientedBoundingBox;
use cesium_core::plane::Plane;
use cesium_core::quaternion::Quaternion;

/// The `positions` fixture of `OrientedBoundingBoxSpec.js` (L20-27): the six
/// axis-aligned extremes, whose bounding box is `fromScale((2, 3, 4))` centred
/// on the origin.
fn spec_positions() -> Vec<Cartesian3> {
    vec![
        Cartesian3::new(2.0, 0.0, 0.0),
        Cartesian3::new(0.0, 3.0, 0.0),
        Cartesian3::new(0.0, 0.0, 4.0),
        Cartesian3::new(-2.0, 0.0, 0.0),
        Cartesian3::new(0.0, -3.0, 0.0),
        Cartesian3::new(0.0, 0.0, -4.0),
    ]
}

/// A deliberately non-default box, used as the `result` out-parameter so a
/// missing write-back is visible: the old code left the caller holding either
/// the stale values or, worse, the `Self::default()` that the buggy
/// `*r = Self::default()` reset had just written.
fn dirty_box() -> OrientedBoundingBox {
    OrientedBoundingBox::new(
        Some(&Cartesian3::new(-7.0, -8.0, -9.0)),
        Some(&Matrix3::from_scale_new(&Cartesian3::new(5.0, 6.0, 7.0))),
    )
}

#[test]
fn default_constructor() {
    let obb = OrientedBoundingBox::default();
    assert_eq!(obb.center, Cartesian3::ZERO);
    assert_eq!(obb.half_axes, Matrix3::ZERO);
}

#[test]
fn constructor_with_parameters() {
    let center = Cartesian3::new(1.0, 2.0, 3.0);
    let half_axes = Matrix3::IDENTITY;
    let obb = OrientedBoundingBox::new(Some(&center), Some(&half_axes));
    assert_eq!(obb.center, center);
    assert_eq!(obb.half_axes, half_axes);
}

#[test]
fn constructor_defaults_to_zero() {
    let obb = OrientedBoundingBox::new(None, None);
    assert_eq!(obb.center, Cartesian3::ZERO);
}

#[test]
fn equals_works() {
    let center = Cartesian3::new(1.0, 2.0, 3.0);
    let half_axes = Matrix3::IDENTITY;
    let obb1 = OrientedBoundingBox::new(Some(&center), Some(&half_axes));
    let obb2 = OrientedBoundingBox::new(Some(&center), Some(&half_axes));
    assert!(obb1.equals(Some(&obb2)));

    let obb3 = OrientedBoundingBox::new(Some(&Cartesian3::ZERO), Some(&half_axes));
    assert!(!obb1.equals(Some(&obb3)));
}

#[test]
fn intersect_plane_splits() {
    // An OBB centered at origin with unit half-axes
    let center = Cartesian3::ZERO;
    let half_axes = Matrix3::new(
        1.0, 0.0, 0.0,
        0.0, 1.0, 0.0,
        0.0, 0.0, 1.0,
    );
    let obb = OrientedBoundingBox::new(Some(&center), Some(&half_axes));

    // Plane at x=0.5 should split the box
    let plane = Plane::new(&Cartesian3::UNIT_X, -0.5);
    let result = OrientedBoundingBox::intersect_plane(&obb, &plane);
    assert_eq!(result, Intersect::Intersecting);
}

#[test]
fn intersect_plane_in_front() {
    let center = Cartesian3::ZERO;
    let half_axes = Matrix3::new(
        1.0, 0.0, 0.0,
        0.0, 1.0, 0.0,
        0.0, 0.0, 1.0,
    );
    let obb = OrientedBoundingBox::new(Some(&center), Some(&half_axes));

    // Plane at x=5 should be in front of the box
    let plane = Plane::new(&Cartesian3::UNIT_X, -5.0);
    let result = OrientedBoundingBox::intersect_plane(&obb, &plane);
    assert_eq!(result, Intersect::Outside);
}

/// Port of `fromPoints correct scale` (OrientedBoundingBoxSpec.js L74-80).
#[test]
fn from_points_correct_scale() {
    let positions = spec_positions();
    // `box` is a reserved Rust keyword, hence `obb` — the name the pre-existing
    // tests in this file already use.
    let obb = OrientedBoundingBox::from_points(Some(&positions), None);
    assert_eq!(
        obb.half_axes,
        Matrix3::from_scale_new(&Cartesian3::new(2.0, 3.0, 4.0))
    );
    assert_eq!(obb.center, Cartesian3::ZERO);
}

/// `fromPoints` must write into the caller's box, not just into a clone of it.
///
/// The JS contract is `expect(result).toBe(returnedResult)` — the out-parameter
/// and the return value are the same object. An earlier revision of the port
/// cloned `result`, mutated the clone and dropped it, so this assertion is the
/// regression guard.
#[test]
fn from_points_writes_back_into_the_result() {
    let positions = spec_positions();
    let mut result = dirty_box();
    let returned = OrientedBoundingBox::from_points(Some(&positions), Some(&mut result));

    let expected = OrientedBoundingBox::from_points(Some(&positions), None);
    // `OrientedBoundingBox` has no `PartialEq` — comparison goes through
    // `equals`, mirroring the JS.
    assert!(
        result.equals(Some(&expected)),
        "the out-parameter must be overwritten"
    );
    assert!(
        returned.equals(Some(&expected)),
        "and the return value must agree with it"
    );
}

/// The empty-input branch returns early in the JS, after zeroing
/// `result.halfAxes` / `result.center`. Splitting the body into a private
/// builder risks losing the write-back on *that* path only, so it gets its own
/// guard.
///
/// Note this is a forward guard rather than a regression catcher: the old bug
/// happened to be *masked* here, because its `*r = Self::default()` reset left
/// the caller's box zeroed — which coincides with the correct empty-input
/// result. Only the non-empty paths above actually discriminate.
#[test]
fn from_points_writes_back_on_the_empty_positions_early_return() {
    let mut result = dirty_box();
    let returned = OrientedBoundingBox::from_points(Some(&[]), Some(&mut result));

    assert_eq!(result.half_axes, Matrix3::ZERO);
    assert_eq!(result.center, Cartesian3::ZERO);
    assert!(returned.equals(Some(&result)));

    // `undefined` positions take the same branch.
    let mut result = dirty_box();
    OrientedBoundingBox::from_points(None, Some(&mut result));
    assert_eq!(result.half_axes, Matrix3::ZERO);
    assert_eq!(result.center, Cartesian3::ZERO);
}

/// Port of `fromTransformation works with a result parameter`
/// (OrientedBoundingBoxSpec.js L899-921). The JS asserts on `box` — the
/// out-parameter — and discards the return value entirely, which is exactly
/// what makes it able to catch a clone-instead-of-write-back.
#[test]
fn from_transformation_works_with_a_result_parameter() {
    let translation = Cartesian3::new(1.0, 2.0, 3.0);
    let rotation = Quaternion::from_axis_angle_new(&Cartesian3::UNIT_Z, 0.4);
    let scale = Cartesian3::new(1.0, 2.0, 3.0);
    let mut transformation = Matrix4::default();
    Matrix4::from_translation_quaternion_rotation_scale(
        &translation,
        &rotation,
        &scale,
        &mut transformation,
    );

    let mut obb = dirty_box();
    OrientedBoundingBox::from_transformation(&transformation, Some(&mut obb));

    assert_eq!(obb.center, translation);
    let matrix3 = Matrix4::get_matrix3_new(&transformation);
    let mut expected = Matrix3::default();
    Matrix3::multiply_by_uniform_scale(&matrix3, 0.5, &mut expected);
    assert!(Matrix3::equals_epsilon(
        &obb.half_axes,
        &expected,
        CesiumMath::EPSILON14
    ));
}

/// `unpack` writes straight into `result.center` / `result.halfAxes` in the JS;
/// those are the only two fields, so the port must reach the caller's box too.
#[test]
fn unpack_writes_back_into_the_result() {
    let original = OrientedBoundingBox::new(
        Some(&Cartesian3::new(1.0, 2.0, 3.0)),
        Some(&Matrix3::from_scale_new(&Cartesian3::new(4.0, 5.0, 6.0))),
    );
    let mut array = [0.0; 12];
    OrientedBoundingBox::pack(&original, &mut array, Some(0));

    let mut result = dirty_box();
    let returned = OrientedBoundingBox::unpack(&array, Some(0), Some(&mut result));

    assert!(
        result.equals(Some(&original)),
        "the out-parameter must be overwritten"
    );
    assert!(returned.equals(Some(&original)));
}
