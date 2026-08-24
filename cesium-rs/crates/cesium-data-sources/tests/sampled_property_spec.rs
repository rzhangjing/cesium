//! Spec mirror of `packages/engine/Specs/DataSources/SampledPropertySpec.js`.
//!
//! One `#[test]` per Jasmine `it(...)` (snake_case direct translation).
//!
//! Time convention: CesiumJS `JulianDate(day, seconds)` maps to plain `f64`
//! seconds (`day * 86400 + seconds`), matching the crate-wide time
//! convention. `definitionChanged` event assertions from the JS spec are
//! skipped: the event system is owned by a separate work item (#34).

use cesium_core::extrapolation_type::ExtrapolationType;
use cesium_data_sources::property::{Property, PropertyResult};
use cesium_data_sources::sampled_property::{
    InterpolationAlgorithmKind, PackableType, SampledProperty,
};
use cesium_test_utils::assert_approx_eq_f64;

const DAY: f64 = 86400.0;

const EPSILON7: f64 = 1e-7;
const EPSILON12: f64 = 1e-12;

fn number(value: f64) -> PropertyResult {
    PropertyResult::Number(value)
}

fn cartesian3(x: f64, y: f64, z: f64) -> PropertyResult {
    PropertyResult::Cartesian3(x, y, z)
}

fn expect_number(result: PropertyResult) -> f64 {
    match result {
        PropertyResult::Number(value) => value,
        other => panic!("expected Number, got {:?}", other),
    }
}

#[test]
fn constructor_sets_expected_defaults() {
    let property = SampledProperty::new(PackableType::Cartesian3);
    assert_eq!(property.interpolation_degree(), 1);
    assert_eq!(
        property.interpolation_algorithm(),
        InterpolationAlgorithmKind::Linear
    );
    assert!(property.is_constant());
    assert_eq!(property.property_type(), PackableType::Cartesian3);
    assert!(property.derivative_types().is_none());
    assert_eq!(
        property.forward_extrapolation_type(),
        ExtrapolationType::None
    );
    assert_eq!(property.forward_extrapolation_duration(), 0.0);
    assert_eq!(
        property.backward_extrapolation_type(),
        ExtrapolationType::None
    );
    assert_eq!(property.backward_extrapolation_duration(), 0.0);

    let derivatives = vec![PackableType::Cartesian3, PackableType::Cartesian3];
    let property =
        SampledProperty::with_derivative_types(PackableType::Quaternion, Some(derivatives.clone()));
    assert_eq!(property.interpolation_degree(), 1);
    assert_eq!(
        property.interpolation_algorithm(),
        InterpolationAlgorithmKind::Linear
    );
    assert!(property.is_constant());
    assert_eq!(property.property_type(), PackableType::Quaternion);
    assert_eq!(property.derivative_types(), Some(derivatives.as_slice()));
    assert_eq!(
        property.forward_extrapolation_type(),
        ExtrapolationType::None
    );
    assert_eq!(property.forward_extrapolation_duration(), 0.0);
    assert_eq!(
        property.backward_extrapolation_type(),
        ExtrapolationType::None
    );
    assert_eq!(property.backward_extrapolation_duration(), 0.0);
}

#[test]
fn is_constant_works() {
    let mut property = SampledProperty::new(PackableType::Number);
    assert!(property.is_constant());
    property.add_sample(0.0, &number(1.0));
    assert!(!property.is_constant());
}

#[test]
fn add_samples_packed_array_works() {
    let data = [0.0, 7.0, 1.0, 8.0, 2.0, 9.0];
    let epoch = 0.0; // JulianDate(0, 0)

    let mut property = SampledProperty::new(PackableType::Number);
    // JS: definitionChanged listener fires here (event system: work item #34).
    property.add_samples_packed_array(&data, Some(epoch));

    assert_eq!(expect_number(property.get_value(epoch)), 7.0);
    assert_eq!(expect_number(property.get_value(epoch + 0.5)), 7.5);
}

#[test]
fn add_sample_works() {
    let times = [0.0, DAY, 2.0 * DAY]; // JulianDate(0/1/2, 0)
    let values = [7.0, 8.0, 9.0];

    let mut property = SampledProperty::new(PackableType::Number);
    // JS: definitionChanged listener fires on each addSample (work item #34).
    property.add_sample(times[0], &number(values[0]));
    property.add_sample(times[1], &number(values[1]));
    property.add_sample(times[2], &number(values[2]));

    assert_eq!(expect_number(property.get_value(times[0])), values[0]);
    assert_eq!(expect_number(property.get_value(times[1])), values[1]);
    assert_eq!(expect_number(property.get_value(times[2])), values[2]);
    // JS: getValue(new JulianDate(0.5, 0)) — half a day after the start.
    assert_eq!(expect_number(property.get_value(0.5 * DAY)), 7.5);
}

#[test]
fn add_samples_works() {
    let times = [0.0, DAY, 2.0 * DAY];
    let values = [7.0, 8.0, 9.0];

    let mut property = SampledProperty::new(PackableType::Number);
    property.add_samples(
        &times,
        &[number(values[0]), number(values[1]), number(values[2])],
    );

    // JS: definitionChanged listener fires once (work item #34).
    assert_eq!(expect_number(property.get_value(times[0])), values[0]);
    assert_eq!(expect_number(property.get_value(times[1])), values[1]);
    assert_eq!(expect_number(property.get_value(times[2])), values[2]);
    assert_eq!(expect_number(property.get_value(0.5 * DAY)), 7.5);
}

#[test]
fn get_sample_works() {
    let times = [0.0, DAY, 2.0 * DAY];
    let values = [7.0, 8.0, 9.0];

    let mut property = SampledProperty::new(PackableType::Number);
    property.add_samples(
        &times,
        &[number(values[0]), number(values[1]), number(values[2])],
    );

    assert_eq!(property.get_sample(0), Some(times[0]));
    assert_eq!(property.get_sample(1), Some(times[1]));
    assert_eq!(property.get_sample(2), Some(times[2]));
    assert_eq!(property.get_sample(3), None);

    assert_eq!(property.get_sample(-1), Some(times[2]));
    assert_eq!(property.get_sample(-2), Some(times[1]));
    assert_eq!(property.get_sample(-3), Some(times[0]));
    assert_eq!(property.get_sample(-4), None);
}

#[test]
fn can_remove_a_sample_at_a_date() {
    let times = [0.0, DAY, 2.0 * DAY];
    let values = [1.0, 8.0, 3.0];

    let mut property = SampledProperty::new(PackableType::Number);
    property.add_samples(
        &times,
        &[number(values[0]), number(values[1]), number(values[2])],
    );
    assert_eq!(expect_number(property.get_value(times[0])), values[0]);
    assert_eq!(expect_number(property.get_value(times[1])), values[1]);
    assert_eq!(expect_number(property.get_value(times[2])), values[2]);

    // JS: definitionChanged listener attached here (work item #34).

    let result = property.remove_sample(4.0 * DAY);
    assert!(!result);

    let result = property.remove_sample(times[1]);

    assert!(result);
    assert_eq!(property.times_for_testing().len(), 2);
    assert_eq!(property.values_for_testing().len(), 2);

    assert_eq!(expect_number(property.get_value(times[0])), values[0]);
    // by deleting the sample at times[1] we now linearly interpolate from
    // the remaining samples
    assert_eq!(
        expect_number(property.get_value(times[1])),
        (values[0] + values[2]) / 2.0
    );
    assert_eq!(expect_number(property.get_value(times[2])), values[2]);
}

fn array_subset_f64(array: &[f64], start_index: usize, count: usize) -> Vec<f64> {
    let mut copy = array.to_vec();
    copy.drain(start_index..start_index + count);
    copy
}

#[test]
fn can_remove_samples_for_a_time_interval() {
    let times = [0.0, DAY, 2.0 * DAY, 3.0 * DAY, 4.0 * DAY];
    let values = [1.0, 8.0, 13.0, 1.0, 3.0];

    let mut create_property = || {
        let mut property = SampledProperty::new(PackableType::Number);
        property.add_samples(
            &times,
            &values.iter().map(|value| number(*value)).collect::<Vec<_>>(),
        );
        for (time, value) in times.iter().zip(values.iter()) {
            assert_eq!(expect_number(property.get_value(*time)), *value);
        }
        property
    };

    let mut property = create_property();
    property.remove_samples_interval(times[1], times[3], true, true);

    // JS: definitionChanged listener fires (work item #34).
    assert_eq!(property.times_for_testing().len(), 2);
    assert_eq!(property.values_for_testing().len(), 2);
    assert_eq!(
        property.times_for_testing(),
        array_subset_f64(&times, 1, 3).as_slice()
    );
    assert_eq!(
        property.values_for_testing(),
        array_subset_f64(&values, 1, 3).as_slice()
    );

    assert_eq!(expect_number(property.get_value(times[0])), values[0]);
    // by deleting the samples we now linearly interpolate from the
    // remaining samples
    assert_eq!(
        expect_number(property.get_value(times[2])),
        (values[0] + values[4]) / 2.0
    );
    assert_eq!(expect_number(property.get_value(times[4])), values[4]);

    // remove using a start time just after a sample
    let mut property = create_property();
    property.remove_samples_interval(times[1] + 4.0, times[3], true, true);

    assert_eq!(property.times_for_testing().len(), 3);
    assert_eq!(property.values_for_testing().len(), 3);
    assert_eq!(
        property.times_for_testing(),
        array_subset_f64(&times, 2, 2).as_slice()
    );
    assert_eq!(
        property.values_for_testing(),
        array_subset_f64(&values, 2, 2).as_slice()
    );

    // remove using a stop time just before a sample
    let mut property = create_property();
    property.remove_samples_interval(times[1] + 4.0, times[3] - 4.0, true, true);

    assert_eq!(property.times_for_testing().len(), 4);
    assert_eq!(property.values_for_testing().len(), 4);
    assert_eq!(
        property.times_for_testing(),
        array_subset_f64(&times, 2, 1).as_slice()
    );
    assert_eq!(
        property.values_for_testing(),
        array_subset_f64(&values, 2, 1).as_slice()
    );
}

#[test]
fn can_remove_samples_for_a_time_interval_with_start_or_stop_not_included() {
    let times = [0.0, DAY, 2.0 * DAY, 3.0 * DAY, 4.0 * DAY];
    let values = [1.0, 8.0, 13.0, 1.0, 3.0];

    let mut create_property = || {
        let mut property = SampledProperty::new(PackableType::Number);
        property.add_samples(
            &times,
            &values.iter().map(|value| number(*value)).collect::<Vec<_>>(),
        );
        property
    };

    let mut property = create_property();
    property.remove_samples_interval(times[1], times[3], false, true);
    assert_eq!(
        property.times_for_testing(),
        array_subset_f64(&times, 2, 2).as_slice()
    );
    assert_eq!(
        property.values_for_testing(),
        array_subset_f64(&values, 2, 2).as_slice()
    );

    let mut property = create_property();
    property.remove_samples_interval(times[1], times[3], true, false);
    assert_eq!(
        property.times_for_testing(),
        array_subset_f64(&times, 1, 2).as_slice()
    );
    assert_eq!(
        property.values_for_testing(),
        array_subset_f64(&values, 1, 2).as_slice()
    );

    let mut property = create_property();
    property.remove_samples_interval(times[1], times[3], false, false);
    assert_eq!(
        property.times_for_testing(),
        array_subset_f64(&times, 2, 1).as_slice()
    );
    assert_eq!(
        property.values_for_testing(),
        array_subset_f64(&values, 2, 1).as_slice()
    );
}

#[test]
#[ignore = "DEVIATION: custom PackableForInterpolation types are not supported by the closed PackableType enum"]
fn works_with_packable_for_interpolation() {
    // JS spec defines a CustomType with packedInterpolationLength and
    // convertPackedArrayForInterpolation; the Rust port interpolates all
    // types directly in their packed representation.
}

#[test]
fn can_set_interpolation_algorithm_and_degree() {
    let data = [0.0, 7.0, 2.0, 9.0, 4.0, 11.0];
    let epoch = 0.0;

    let mut property = SampledProperty::new(PackableType::Number);
    property.set_forward_extrapolation_type(ExtrapolationType::Extrapolate);
    property.add_samples_packed_array(&data, Some(epoch));

    assert_eq!(expect_number(property.get_value(epoch)), 7.0);
    assert_eq!(expect_number(property.get_value(epoch + 1.0)), 8.0);

    // DEVIATION: the JS spec installs a MockInterpolation object and
    // asserts the exact xTable/yTable passed to it; the Rust port uses a
    // closed algorithm enum, so behavioral coverage of the windowing logic
    // is provided by the golden-vector tests instead.
    property.set_interpolation_options(Some(InterpolationAlgorithmKind::Lagrange), Some(2));

    // JS: definitionChanged listener fires (work item #34).
    assert_eq!(
        property.interpolation_algorithm(),
        InterpolationAlgorithmKind::Lagrange
    );
    assert_eq!(property.interpolation_degree(), 2);
    assert_eq!(expect_number(property.get_value(epoch)), 7.0);
}

#[test]
fn returns_undefined_if_trying_to_interpolate_with_less_than_enough_samples() {
    let value = 7.0;
    let time = 0.0;

    let mut property = SampledProperty::new(PackableType::Number);
    property.add_sample(time, &number(value));

    assert_eq!(expect_number(property.get_value(time)), value);
    assert!(property.get_value(time + 4.0).is_none());
}

#[test]
fn allows_empty_options_object_without_failing() {
    let mut property = SampledProperty::new(PackableType::Number);

    let interpolation_algorithm = property.interpolation_algorithm();
    let interpolation_degree = property.interpolation_degree();

    property.set_interpolation_options(None, None);

    assert_eq!(property.interpolation_algorithm(), interpolation_algorithm);
    assert_eq!(property.interpolation_degree(), interpolation_degree);
}

#[test]
fn merge_new_samples_works_with_huge_data_sets() {
    let mut times: Vec<f64> = Vec::new();
    let mut values: Vec<f64> = Vec::new();
    let epoch = 0.0;

    let mut data: Vec<f64> = Vec::new();
    let mut expected_times: Vec<f64> = Vec::new();
    let mut expected_values: Vec<f64> = Vec::new();

    for i in 0..200000 {
        let i = i as f64;
        data.push(i);
        data.push(i);
        expected_times.push(epoch + i);
        expected_values.push(i);
    }

    SampledProperty::merge_new_samples_for_testing(Some(epoch), &mut times, &mut values, &data, 1);

    assert_eq!(times, expected_times);
    assert_eq!(values, expected_values);
}

#[test]
fn merge_new_samples_works_for_sorted_non_intersecting_data() {
    let mut times: Vec<f64> = Vec::new();
    let mut values: Vec<f64> = Vec::new();
    let epoch = 0.0;

    // JS uses string values "a".."f"; the Rust packed representation is f64.
    let new_data = [0.0, 1.0, 1.0, 2.0, 2.0, 3.0];
    let new_data2 = [3.0, 4.0, 4.0, 5.0, 5.0, 6.0];

    let expected_times = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let expected_values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];

    SampledProperty::merge_new_samples_for_testing(
        Some(epoch),
        &mut times,
        &mut values,
        &new_data,
        1,
    );
    SampledProperty::merge_new_samples_for_testing(
        Some(epoch),
        &mut times,
        &mut values,
        &new_data2,
        1,
    );

    assert_eq!(times, expected_times);
    assert_eq!(values, expected_values);
}

#[test]
fn merge_new_samples_works_for_iso8601_dates() {
    // DEVIATION: the JS spec passes ISO8601 date strings in the packed
    // array; the Rust port stores f64 seconds, so this case is exercised
    // with absolute offsets from the epoch instead.
    let mut times: Vec<f64> = Vec::new();
    let mut values: Vec<f64> = Vec::new();

    let new_data = [0.0, 1.0, 1.0, 2.0, 2.0, 3.0];
    let new_data2 = [3.0, 4.0, 4.0, 5.0, 5.0, 6.0];

    let expected_times = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let expected_values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];

    SampledProperty::merge_new_samples_for_testing(None, &mut times, &mut values, &new_data, 1);
    SampledProperty::merge_new_samples_for_testing(None, &mut times, &mut values, &new_data2, 1);

    assert_eq!(times, expected_times);
    assert_eq!(values, expected_values);
}

#[test]
fn merge_new_samples_works_for_elements_of_size_2() {
    let mut times: Vec<f64> = Vec::new();
    let mut values: Vec<f64> = Vec::new();
    let epoch = 0.0;

    // JS values "a".."e" mapped to 1..5.
    let new_data = [1.0, 2.0, 2.0, 4.0, 5.0, 5.0, 0.0, 1.0, 1.0];
    let new_data2 = [2.0, 3.0, 3.0, 3.0, 4.0, 4.0];

    let expected_times = [0.0, 1.0, 2.0, 3.0, 4.0];
    let expected_values = [1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 4.0, 4.0, 5.0, 5.0];

    SampledProperty::merge_new_samples_for_testing(
        Some(epoch),
        &mut times,
        &mut values,
        &new_data,
        2,
    );
    SampledProperty::merge_new_samples_for_testing(
        Some(epoch),
        &mut times,
        &mut values,
        &new_data2,
        2,
    );

    assert_eq!(times, expected_times);
    assert_eq!(values, expected_values);
}

#[test]
fn merge_new_samples_works_for_unsorted_intersecting_data() {
    let mut times: Vec<f64> = Vec::new();
    let mut values: Vec<f64> = Vec::new();
    let epoch = 0.0;

    let new_data = [1.0, 2.0, 4.0, 5.0, 0.0, 1.0];
    let new_data2 = [5.0, 6.0, 2.0, 3.0, 3.0, 4.0];

    let expected_times = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let expected_values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];

    SampledProperty::merge_new_samples_for_testing(
        Some(epoch),
        &mut times,
        &mut values,
        &new_data,
        1,
    );
    SampledProperty::merge_new_samples_for_testing(
        Some(epoch),
        &mut times,
        &mut values,
        &new_data2,
        1,
    );

    assert_eq!(times, expected_times);
    assert_eq!(values, expected_values);
}

#[test]
fn merge_new_samples_works_for_data_with_repeated_values() {
    let mut times: Vec<f64> = Vec::new();
    let mut values: Vec<f64> = Vec::new();
    let epoch = 0.0;

    // JS values "a".."f" mapped to 1..6.
    let new_data = [0.0, 1.0, 1.0, 2.0, 1.0, 3.0, 0.0, 4.0, 4.0, 5.0, 5.0, 6.0];
    let expected_times = [0.0, 1.0, 4.0, 5.0];
    let expected_values = [4.0, 3.0, 5.0, 6.0];
    SampledProperty::merge_new_samples_for_testing(
        Some(epoch),
        &mut times,
        &mut values,
        &new_data,
        1,
    );

    assert_eq!(times, expected_times);
    assert_eq!(values, expected_values);
}

#[test]
fn merge_new_samples_works_with_interwoven_data() {
    // JS uses two ISO8601 epochs 465.9 s apart
    // ("20130205T150405.704999999999927Z" / "20130205T151151.60499999999956Z");
    // the exact offset does not affect the interwoven ordering.
    let epoch1 = 0.0;
    let epoch2 = 465.9;
    let values1 = [
        0.0, 1.0, 120.0, 2.0, 240.0, 3.0, 360.0, 4.0, 480.0, 6.0, 600.0, 8.0, 720.0, 10.0, 840.0,
        12.0, 960.0, 14.0, 1080.0, 16.0,
    ];
    let values2 = [
        0.0, 5.0, 120.0, 7.0, 240.0, 9.0, 360.0, 11.0, 480.0, 13.0, 600.0, 15.0, 720.0, 17.0,
        840.0, 18.0, 960.0, 19.0, 1080.0, 20.0,
    ];

    let mut times: Vec<f64> = Vec::new();
    let mut values: Vec<f64> = Vec::new();
    SampledProperty::merge_new_samples_for_testing(
        Some(epoch1),
        &mut times,
        &mut values,
        &values1,
        1,
    );
    SampledProperty::merge_new_samples_for_testing(
        Some(epoch2),
        &mut times,
        &mut values,
        &values2,
        1,
    );
    for (i, value) in values.iter().enumerate() {
        assert_eq!(*value, (i + 1) as f64);
    }
}

#[test]
#[ignore = "DEVIATION: the type parameter is enforced at compile time by the PackableType enum"]
fn constructor_throws_without_type_parameter() {}

#[test]
fn equals_works_when_interpolators_differ() {
    let left = SampledProperty::new(PackableType::Number);
    let mut right = SampledProperty::new(PackableType::Number);

    assert!(left.equals(&right));
    right.set_interpolation_options(Some(InterpolationAlgorithmKind::Lagrange), None);
    assert!(!left.equals(&right));
}

#[test]
fn equals_works_when_interpolator_degree_differ() {
    let mut left = SampledProperty::new(PackableType::Number);
    left.set_interpolation_options(Some(InterpolationAlgorithmKind::Lagrange), Some(2));

    let mut right = SampledProperty::new(PackableType::Number);
    right.set_interpolation_options(Some(InterpolationAlgorithmKind::Lagrange), Some(2));

    assert!(left.equals(&right));
    right.set_interpolation_options(None, Some(3));

    assert!(!left.equals(&right));
}

#[test]
fn equals_works_when_samples_differ() {
    let mut left = SampledProperty::new(PackableType::Number);
    let mut right = SampledProperty::new(PackableType::Number);
    assert!(left.equals(&right));

    let time = 0.0;
    left.add_sample(time, &number(5.0));
    assert!(!left.equals(&right));

    right.add_sample(time, &number(5.0));
    assert!(left.equals(&right));
}

#[test]
fn equals_works_when_samples_differ_with_quaternion() {
    let mut left = SampledProperty::new(PackableType::Quaternion);
    let mut right = SampledProperty::new(PackableType::Quaternion);
    assert!(left.equals(&right));

    let time = 0.0;
    left.add_sample(time, &PropertyResult::Quaternion(1.0, 2.0, 3.0, 4.0));
    right.add_sample(time, &PropertyResult::Quaternion(1.0, 2.0, 3.0, 5.0));
    assert!(!left.equals(&right));
}

#[test]
fn equals_works_when_derivatives_differ() {
    let left = SampledProperty::with_derivative_types(
        PackableType::Number,
        Some(vec![PackableType::Number]),
    );
    let right = SampledProperty::new(PackableType::Number);
    assert!(!left.equals(&right));

    let left = SampledProperty::with_derivative_types(
        PackableType::Number,
        Some(vec![PackableType::Number]),
    );
    let right = SampledProperty::with_derivative_types(
        PackableType::Number,
        Some(vec![PackableType::Number]),
    );
    assert!(left.equals(&right));

    let left = SampledProperty::with_derivative_types(
        PackableType::Number,
        Some(vec![PackableType::Number]),
    );
    let right = SampledProperty::with_derivative_types(
        PackableType::Number,
        Some(vec![PackableType::Number, PackableType::Number]),
    );
    assert!(!left.equals(&right));

    let left = SampledProperty::with_derivative_types(
        PackableType::Cartesian3,
        Some(vec![PackableType::Cartesian3, PackableType::Number]),
    );
    let right = SampledProperty::with_derivative_types(
        PackableType::Cartesian3,
        Some(vec![PackableType::Number, PackableType::Number]),
    );
    assert!(!left.equals(&right));
}

// The remaining tests were verified with STK Components in the JS spec.

#[test]
fn add_sample_works_with_multiple_derivatives() {
    let results = [
        0.0,
        -3.39969163485071,
        0.912945250727628,
        -6.17439797860995,
        0.745113160479349,
        -1.63963048028446,
        -0.304810621102217,
        4.83619040459681,
        -0.993888653923375,
        169.448966391543,
    ];

    let mut property = SampledProperty::with_derivative_types(
        PackableType::Number,
        Some(vec![PackableType::Number, PackableType::Number]),
    );
    property.set_forward_extrapolation_type(ExtrapolationType::Extrapolate);
    property.set_interpolation_options(Some(InterpolationAlgorithmKind::Hermite), Some(1));

    // Sample inputs as produced by V8's Math.sin/Math.cos for the JS spec;
    // hardcoded so the Rust port operates on bit-identical inputs.
    let sin_cos: [(f64, f64); 5] = [
        (0.0, 1.0),
        (0.9129452507276277, 0.40808206181339196),
        (0.7451131604793488, -0.6669380616522619),
        (-0.3048106211022167, -0.9524129804151563),
        (-0.9938886539233752, -0.11038724383904756),
    ];

    let mut x = 0usize;
    for (sin_x, cos_x) in sin_cos {
        property.add_sample_with_derivatives(
            x as f64,
            &number(sin_x),
            &[number(cos_x), number(-sin_x)],
        );
        x += 20;
    }

    let mut result_index = 0;
    let mut i = 0usize;
    while i < 100 {
        let result = expect_number(property.get_value(i as f64));
        assert_approx_eq_f64!(result, results[result_index], EPSILON12, 1e-10);
        result_index += 1;
        i += 10;
    }
}

// JS spec dataset: 8 samples at 60 s spacing starting 2014-01-01T00:00:00.
const HERMITE_TIMES: [f64; 8] = [0.0, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0, 420.0];

const HERMITE_POSITIONS: [(f64, f64, f64); 8] = [
    (13378137.0, 0.0, 1.0),
    (13374128.3576279, 327475.593690065, 2.0),
    (13362104.8328212, 654754.936954423, 3.0),
    (13342073.6310691, 981641.896976832, 4.0),
    (13314046.7567223, 1307940.57608951, 5.0),
    (13278041.005799, 1633455.42917117, 6.0),
    (13234077.9559193, 1957991.38083385, 7.0),
    (13182183.953374, 2281353.94232816, 8.0),
];

const HERMITE_DERIVATIVES: [(f64, f64, f64); 8] = [
    (0.0, 5458.47176691947, 0.0),
    (-133.614738921601, 5456.83618333919, 0.0),
    (-267.149404854867, 5451.93041277513, 0.0),
    (-400.523972797808, 5443.75739517027, 0.0),
    (-533.658513692378, 5432.32202847183, 0.0),
    (-666.473242324565, 5417.63116569613, 0.0),
    (-798.888565138278, 5399.69361082164, 0.0),
    (-930.82512793439, 5378.52011351288, 0.0),
];

const ORDER0_RESULTS: [(f64, f64, f64); 22] = [
    (13378137.0, 0.0, 1.0),
    (13376800.785876, 109158.531230022, 1.33333333333333),
    (13375464.5717519, 218317.062460043, 1.66666666666667),
    (13374128.3576279, 327475.593690065, 2.0),
    (13370120.5160257, 436568.708111518, 2.33333333333333),
    (13366112.6744234, 545661.82253297, 2.66666666666667),
    (13362104.8328212, 654754.936954423, 3.0),
    (13355427.7655705, 763717.256961893, 3.33333333333333),
    (13348750.6983198, 872679.576969362, 3.66666666666667),
    (13342073.6310691, 981641.896976832, 4.0),
    (13332731.3396202, 1090408.12334772, 4.33333333333333),
    (13323389.0481712, 1199174.34971862, 4.66666666666667),
    (13314046.7567223, 1307940.57608951, 5.0),
    (13302044.8397479, 1416445.52711673, 5.33333333333333),
    (13290042.9227734, 1524950.47814395, 5.66666666666667),
    (13278041.005799, 1633455.42917117, 6.0),
    (13263386.6558391, 1741634.0797254, 6.33333333333333),
    (13248732.3058792, 1849812.73027962, 6.66666666666667),
    (13234077.9559193, 1957991.38083385, 7.0),
    (13216779.9550709, 2065778.90133195, 7.33333333333333),
    (13199481.9542224, 2173566.42183006, 7.66666666666667),
    (13182183.953374, 2281353.94232816, 8.0),
];

const ORDER1_RESULTS: [(f64, f64, f64); 22] = [
    (13378137.0, 0.0, 1.0),
    (13377691.5656321, 109168.223625571, 1.25925925925926),
    (13376355.3218481, 218329.177845564, 1.74074074074074),
    (13374128.3576279, 327475.593690065, 2.0),
    (13371010.7916129, 436600.202479654, 2.25925925925926),
    (13367002.8610487, 545695.738439022, 2.74074074074074),
    (13362104.8328212, 654754.936954423, 3.0),
    (13356317.0034622, 763770.534428588, 3.25925925925926),
    (13349639.7880007, 872735.273070732, 3.74074074074074),
    (13342073.6310691, 981641.896976832, 4.0),
    (13333619.0069115, 1090483.15198472, 4.25925925925926),
    (13324276.5080919, 1199251.7926376, 4.74074074074074),
    (13314046.7567223, 1307940.57608951, 5.0),
    (13302930.4044753, 1416542.26196067, 5.25925925925926),
    (13290928.2210945, 1525049.62147035, 5.74074074074074),
    (13278041.005799, 1633455.42917117, 6.0),
    (13264269.587299, 1741752.46280477, 6.25925925925926),
    (13249614.9120568, 1849933.51459858, 6.74074074074074),
    (13234077.9559193, 1957991.38083385, 7.0),
    (13217659.7241379, 2065918.86170184, 7.25925925925926),
    (13200361.339326, 2173708.77475762, 7.74074074074074),
    (13182183.953374, 2281353.94232816, 8.0),
];

fn assert_cartesian3_approx(result: PropertyResult, expected: (f64, f64, f64)) {
    match result {
        PropertyResult::Cartesian3(x, y, z) => {
            assert_approx_eq_f64!(x, expected.0, EPSILON7);
            assert_approx_eq_f64!(y, expected.1, EPSILON7);
            assert_approx_eq_f64!(z, expected.2, EPSILON7);
        }
        other => panic!("expected Cartesian3, got {:?}", other),
    }
}

fn check_hermite_series(property: &SampledProperty, expected: &[(f64, f64, f64); 22]) {
    let mut result_index = 0;
    let mut i = 0.0_f64;
    while i < 420.0 {
        let result = property.get_value(i);
        assert_cartesian3_approx(result, expected[result_index]);
        result_index += 1;
        i += 20.0;
    }
}

#[test]
fn add_sample_works_with_derivatives() {
    let mut property = SampledProperty::with_derivative_types(
        PackableType::Cartesian3,
        Some(vec![PackableType::Cartesian3]),
    );
    property.set_interpolation_options(Some(InterpolationAlgorithmKind::Hermite), Some(1));

    for (x, time) in HERMITE_TIMES.iter().enumerate() {
        let (px, py, pz) = HERMITE_POSITIONS[x];
        let (dx, dy, dz) = HERMITE_DERIVATIVES[x];
        property.add_sample_with_derivatives(
            *time,
            &cartesian3(px, py, pz),
            &[cartesian3(dx, dy, dz)],
        );
    }
    check_hermite_series(&property, &ORDER1_RESULTS);
}

#[test]
fn add_sample_works_without_derivatives() {
    let mut property = SampledProperty::new(PackableType::Cartesian3);
    property.set_interpolation_options(Some(InterpolationAlgorithmKind::Hermite), Some(1));

    for (x, time) in HERMITE_TIMES.iter().enumerate() {
        let (px, py, pz) = HERMITE_POSITIONS[x];
        property.add_sample(*time, &cartesian3(px, py, pz));
    }
    check_hermite_series(&property, &ORDER0_RESULTS);
}

#[test]
fn add_samples_works_with_derivatives() {
    let mut property = SampledProperty::with_derivative_types(
        PackableType::Cartesian3,
        Some(vec![PackableType::Cartesian3]),
    );
    property.set_interpolation_options(Some(InterpolationAlgorithmKind::Hermite), Some(1));

    let values: Vec<PropertyResult> = HERMITE_POSITIONS
        .iter()
        .map(|(x, y, z)| cartesian3(*x, *y, *z))
        .collect();
    let derivative_values: Vec<Vec<PropertyResult>> = HERMITE_DERIVATIVES
        .iter()
        .map(|(x, y, z)| vec![cartesian3(*x, *y, *z)])
        .collect();
    property.add_samples_with_derivatives(&HERMITE_TIMES, &values, &derivative_values);
    check_hermite_series(&property, &ORDER1_RESULTS);
}

#[test]
fn add_samples_works_without_derivatives() {
    let mut property = SampledProperty::new(PackableType::Cartesian3);
    property.set_interpolation_options(Some(InterpolationAlgorithmKind::Hermite), Some(1));

    let values: Vec<PropertyResult> = HERMITE_POSITIONS
        .iter()
        .map(|(x, y, z)| cartesian3(*x, *y, *z))
        .collect();
    property.add_samples(&HERMITE_TIMES, &values);
    check_hermite_series(&property, &ORDER0_RESULTS);
}

#[test]
fn add_samples_packed_array_works_with_derivatives() {
    let mut property = SampledProperty::with_derivative_types(
        PackableType::Cartesian3,
        Some(vec![PackableType::Cartesian3]),
    );
    property.set_interpolation_options(Some(InterpolationAlgorithmKind::Hermite), Some(1));

    let mut data: Vec<f64> = Vec::new();
    for x in 0..HERMITE_TIMES.len() {
        data.push(HERMITE_TIMES[x]);
        data.extend_from_slice(&[
            HERMITE_POSITIONS[x].0,
            HERMITE_POSITIONS[x].1,
            HERMITE_POSITIONS[x].2,
        ]);
        data.extend_from_slice(&[
            HERMITE_DERIVATIVES[x].0,
            HERMITE_DERIVATIVES[x].1,
            HERMITE_DERIVATIVES[x].2,
        ]);
    }
    property.add_samples_packed_array(&data, None);
    check_hermite_series(&property, &ORDER1_RESULTS);
}

#[test]
fn add_samples_packed_array_works_without_derivatives() {
    let mut property = SampledProperty::new(PackableType::Cartesian3);
    property.set_interpolation_options(Some(InterpolationAlgorithmKind::Hermite), Some(1));

    let mut data: Vec<f64> = Vec::new();
    for x in 0..HERMITE_TIMES.len() {
        data.push(HERMITE_TIMES[x]);
        data.extend_from_slice(&[
            HERMITE_POSITIONS[x].0,
            HERMITE_POSITIONS[x].1,
            HERMITE_POSITIONS[x].2,
        ]);
    }
    property.add_samples_packed_array(&data, None);
    check_hermite_series(&property, &ORDER0_RESULTS);
}

#[test]
fn obeys_extrapolation_options() {
    let mut property = SampledProperty::new(PackableType::Number);

    let time0 = 0.99;
    let time1 = 1.0;
    let time2 = 2.0;
    let time3 = 3.0;
    let time4 = 4.0;
    let time5 = 4.01;

    property.add_sample(time2, &number(1.0));
    property.add_sample(time3, &number(2.0));

    // Default is no extrapolation
    assert!(property.get_value(time0).is_none());
    assert!(property.get_value(time1).is_none());
    assert_eq!(expect_number(property.get_value(time2)), 1.0);
    assert_eq!(expect_number(property.get_value(time3)), 2.0);
    assert!(property.get_value(time4).is_none());
    assert!(property.get_value(time5).is_none());

    // No backward, hold forward for up to 1 second
    property.set_forward_extrapolation_type(ExtrapolationType::Hold);
    property.set_forward_extrapolation_duration(1.0);
    property.set_backward_extrapolation_type(ExtrapolationType::None);
    property.set_backward_extrapolation_duration(1.0);

    assert!(property.get_value(time1).is_none());
    assert_eq!(expect_number(property.get_value(time2)), 1.0);
    assert_eq!(expect_number(property.get_value(time3)), 2.0);
    assert_eq!(expect_number(property.get_value(time4)), 2.0);
    assert!(property.get_value(time5).is_none());

    // No backward, extrapolate forward for up to 1 second
    property.set_forward_extrapolation_type(ExtrapolationType::Extrapolate);
    property.set_forward_extrapolation_duration(1.0);
    property.set_backward_extrapolation_type(ExtrapolationType::None);
    property.set_backward_extrapolation_duration(1.0);

    assert!(property.get_value(time1).is_none());
    assert_eq!(expect_number(property.get_value(time2)), 1.0);
    assert_eq!(expect_number(property.get_value(time3)), 2.0);
    assert_eq!(expect_number(property.get_value(time4)), 3.0);
    assert!(property.get_value(time5).is_none());

    // No forward, hold backward for up to 1 second
    property.set_forward_extrapolation_type(ExtrapolationType::None);
    property.set_forward_extrapolation_duration(1.0);
    property.set_backward_extrapolation_type(ExtrapolationType::Hold);
    property.set_backward_extrapolation_duration(1.0);

    assert!(property.get_value(time0).is_none());
    assert_eq!(expect_number(property.get_value(time1)), 1.0);
    assert_eq!(expect_number(property.get_value(time2)), 1.0);
    assert_eq!(expect_number(property.get_value(time3)), 2.0);
    assert!(property.get_value(time4).is_none());

    // No forward, extrapolate backward for up to 1 second
    property.set_forward_extrapolation_type(ExtrapolationType::None);
    property.set_forward_extrapolation_duration(1.0);
    property.set_backward_extrapolation_type(ExtrapolationType::Extrapolate);
    property.set_backward_extrapolation_duration(1.0);

    assert!(property.get_value(time0).is_none());
    assert_eq!(expect_number(property.get_value(time1)), 0.0);
    assert_eq!(expect_number(property.get_value(time2)), 1.0);
    assert_eq!(expect_number(property.get_value(time3)), 2.0);
    assert!(property.get_value(time4).is_none());
}

#[test]
fn get_value_returns_undefined_for_empty_extrapolated_property() {
    let mut sampled_position = SampledProperty::new(PackableType::Cartesian3);
    sampled_position.set_backward_extrapolation_type(ExtrapolationType::Hold);
    sampled_position.set_forward_extrapolation_type(ExtrapolationType::Hold);
    let result = sampled_position.get_value(0.0);
    assert!(result.is_none());
}

#[test]
#[ignore = "definitionChanged event is owned by work item #34"]
fn raises_definition_changed_when_extrapolation_options_change() {}
