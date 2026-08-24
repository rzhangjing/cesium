//! Mirror of `packages/engine/Specs/Core/PolylinePipelineSpec.js`.
//!
//! DEVIATION: the JS specs "generateArc throws without positions" and
//! "generateRhumbArc throws without positions" rely on `options.positions`
//! being `undefined`; the Rust port models positions as a non-optional
//! `Vec<Cartesian3>` (see DEVIATION in `polyline_pipeline.rs`), so those two
//! cases are mirrored as "absent options behave like an empty position list".

use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::math::CesiumMath;
use cesium_core::matrix4::Matrix4;
use cesium_core::polyline_pipeline::{
    GenerateArcHeight, GenerateArcOptions, PolylinePipeline,
};
use cesium_core::transforms::east_north_up_to_fixed_frame;

#[test]
fn wrap_longitude() {
    let positions = Cartesian3::from_degrees_array(
        &[-75.163789, 39.952335, -80.2264393, 25.7889689],
        None,
        None,
    );
    let segments = PolylinePipeline::wrap_longitude(Some(&positions), None);
    assert_eq!(segments.lengths.len(), 1);
    assert_eq!(segments.lengths[0], 2);
}

#[test]
fn wrap_longitude_works_with_empty_array() {
    let segments = PolylinePipeline::wrap_longitude(Some(&[]), None);
    assert_eq!(segments.lengths.len(), 0);
}

#[test]
fn wrap_longitude_breaks_polyline_into_segments() {
    let positions = Cartesian3::from_degrees_array(&[-179.0, 39.0, 2.0, 25.0], None, None);
    let segments = PolylinePipeline::wrap_longitude(Some(&positions), None);
    assert_eq!(segments.lengths.len(), 2);
    assert_eq!(segments.lengths[0], 2);
    assert_eq!(segments.lengths[1], 2);
}

#[test]
fn wrap_longitude_breaks_polyline_into_segments_with_model_matrix() {
    let center = Cartesian3::from_degrees_new(-179.0, 39.0, None, None);
    let mut matrix = Matrix4::default();
    east_north_up_to_fixed_frame(&center, Some(&Ellipsoid::WGS84), &mut matrix);

    let positions = [
        Cartesian3::new(0.0, 0.0, 0.0),
        Cartesian3::new(0.0, 100000000.0, 0.0),
    ];
    let segments = PolylinePipeline::wrap_longitude(Some(&positions), Some(&matrix));
    assert_eq!(segments.lengths.len(), 2);
    assert_eq!(segments.lengths[0], 2);
    assert_eq!(segments.lengths[1], 2);
}

/// DEVIATION mirror of "generateArc throws without positions": the Rust API
/// cannot receive undefined positions, so absent options fall back to the
/// default (empty) options instead of throwing.
#[test]
fn generate_arc_without_options_behaves_like_empty_positions() {
    let new_positions = PolylinePipeline::generate_arc(None);
    assert_eq!(new_positions.len(), 0);
}

#[test]
fn generate_arc_accepts_a_height_array_for_single_value() {
    let positions = vec![Cartesian3::from_degrees_new(0.0, 0.0, None, None)];
    let height = vec![30.0];

    let options = GenerateArcOptions {
        positions,
        height: Some(GenerateArcHeight::Array(height)),
        ..Default::default()
    };
    let new_positions = PolylinePipeline::generate_arc(Some(&options));

    assert_eq!(new_positions.len(), 3);
    let actual = Cartesian3::from_array_new(&new_positions, Some(0));
    let expected = Cartesian3::from_degrees_new(0.0, 0.0, Some(30.0), None);
    assert!(Cartesian3::equals_epsilon(
        Some(&actual),
        Some(&expected),
        Some(CesiumMath::EPSILON6),
        None
    ));
}

#[test]
fn generate_arc_subdivides_in_half() {
    let p1 = Cartesian3::from_degrees_new(0.0, 0.0, None, None);
    let p2 = Cartesian3::from_degrees_new(90.0, 0.0, None, None);
    let p3 = Cartesian3::from_degrees_new(45.0, 0.0, None, None);
    let positions = vec![p1, p2];

    let options = GenerateArcOptions {
        positions,
        granularity: Some(CesiumMath::PI_OVER_TWO / 2.0),
        ellipsoid: Some(Ellipsoid::WGS84),
        ..Default::default()
    };
    let new_positions = PolylinePipeline::generate_arc(Some(&options));

    assert_eq!(new_positions.len(), 3 * 3);
    let p1n = Cartesian3::from_array_new(&new_positions, Some(0));
    let p3n = Cartesian3::from_array_new(&new_positions, Some(3));
    let p2n = Cartesian3::from_array_new(&new_positions, Some(6));
    assert!(Cartesian3::equals_epsilon(
        Some(&p1),
        Some(&p1n),
        Some(CesiumMath::EPSILON4),
        None
    ));
    assert!(Cartesian3::equals_epsilon(
        Some(&p2),
        Some(&p2n),
        Some(CesiumMath::EPSILON4),
        None
    ));
    assert!(Cartesian3::equals_epsilon(
        Some(&p3),
        Some(&p3n),
        Some(CesiumMath::EPSILON4),
        None
    ));
}

#[test]
fn generate_arc_works_with_empty_array() {
    let options = GenerateArcOptions {
        positions: vec![],
        ..Default::default()
    };
    let new_positions = PolylinePipeline::generate_arc(Some(&options));

    assert_eq!(new_positions.len(), 0);
}

#[test]
fn generate_arc_works_one_position() {
    let options = GenerateArcOptions {
        positions: vec![Cartesian3::UNIT_Z],
        ellipsoid: Some(Ellipsoid::UNIT_SPHERE),
        ..Default::default()
    };
    let new_positions = PolylinePipeline::generate_arc(Some(&options));

    assert_eq!(new_positions.len(), 3);
    assert_eq!(new_positions, vec![0.0, 0.0, 1.0]);
}

/// DEVIATION mirror of "generateRhumbArc throws without positions": see the
/// note on `generate_arc_without_options_behaves_like_empty_positions`.
#[test]
fn generate_rhumb_arc_without_options_behaves_like_empty_positions() {
    let new_positions = PolylinePipeline::generate_rhumb_arc(None);
    assert_eq!(new_positions.len(), 0);
}

#[test]
fn generate_rhumb_arc_accepts_a_height_array_for_single_value() {
    let positions = vec![Cartesian3::from_degrees_new(0.0, 0.0, None, None)];
    let height = vec![30.0];

    let options = GenerateArcOptions {
        positions,
        height: Some(GenerateArcHeight::Array(height)),
        ..Default::default()
    };
    let new_positions = PolylinePipeline::generate_rhumb_arc(Some(&options));

    assert_eq!(new_positions.len(), 3);
    let actual = Cartesian3::from_array_new(&new_positions, Some(0));
    let expected = Cartesian3::from_degrees_new(0.0, 0.0, Some(30.0), None);
    assert!(Cartesian3::equals_epsilon(
        Some(&actual),
        Some(&expected),
        Some(CesiumMath::EPSILON6),
        None
    ));
}

#[test]
fn generate_rhumb_arc_subdivides_in_half() {
    let p1 = Cartesian3::from_degrees_new(0.0, 30.0, None, None);
    let p2 = Cartesian3::from_degrees_new(90.0, 30.0, None, None);
    let p3 = Cartesian3::from_degrees_new(45.0, 30.0, None, None);
    let positions = vec![p1, p2];

    let options = GenerateArcOptions {
        positions,
        granularity: Some(CesiumMath::PI_OVER_FOUR),
        ellipsoid: Some(Ellipsoid::WGS84),
        ..Default::default()
    };
    let new_positions = PolylinePipeline::generate_rhumb_arc(Some(&options));

    assert_eq!(new_positions.len(), 3 * 3);
    let p1n = Cartesian3::from_array_new(&new_positions, Some(0));
    let p3n = Cartesian3::from_array_new(&new_positions, Some(3));
    let p2n = Cartesian3::from_array_new(&new_positions, Some(6));
    assert!(Cartesian3::equals_epsilon(
        Some(&p1),
        Some(&p1n),
        Some(CesiumMath::EPSILON4),
        None
    ));
    assert!(Cartesian3::equals_epsilon(
        Some(&p2),
        Some(&p2n),
        Some(CesiumMath::EPSILON4),
        None
    ));
    assert!(Cartesian3::equals_epsilon(
        Some(&p3),
        Some(&p3n),
        Some(CesiumMath::EPSILON4),
        None
    ));
}

#[test]
fn generate_rhumb_arc_works_with_empty_array() {
    let options = GenerateArcOptions {
        positions: vec![],
        ..Default::default()
    };
    let new_positions = PolylinePipeline::generate_rhumb_arc(Some(&options));

    assert_eq!(new_positions.len(), 0);
}

#[test]
fn generate_rhumb_arc_works_one_position() {
    let options = GenerateArcOptions {
        positions: vec![Cartesian3::UNIT_Z],
        ellipsoid: Some(Ellipsoid::UNIT_SPHERE),
        ..Default::default()
    };
    let new_positions = PolylinePipeline::generate_rhumb_arc(Some(&options));

    assert_eq!(new_positions.len(), 3);
    assert_eq!(new_positions, vec![0.0, 0.0, 1.0]);
}

#[test]
fn generate_rhumb_arc_return_values_for_each_position() {
    let positions = Cartesian3::from_degrees_array(&[0.0, 0.0, 10.0, 0.0, 10.0, 5.0], None, None);
    let options = GenerateArcOptions {
        positions,
        ..Default::default()
    };
    let new_positions = PolylinePipeline::generate_rhumb_arc(Some(&options));
    for value in &new_positions {
        assert!(!value.is_nan(), "expected a defined (non-NaN) value");
    }
}
