use cesium_core::heading_pitch_roll::HeadingPitchRoll;
use cesium_core::math::CesiumMath;
use cesium_core::quaternion::Quaternion;

const DEG2RAD: f64 = CesiumMath::RADIANS_PER_DEGREE;

#[test]
fn construct_with_default_values() {
    let hpr = HeadingPitchRoll::default();
    assert_eq!(hpr.heading, 0.0);
    assert_eq!(hpr.pitch, 0.0);
    assert_eq!(hpr.roll, 0.0);
}

#[test]
fn construct_with_all_values() {
    let hpr = HeadingPitchRoll::new(1.0 * DEG2RAD, 2.0 * DEG2RAD, 3.0 * DEG2RAD);
    assert_eq!(hpr.heading, 1.0 * DEG2RAD);
    assert_eq!(hpr.pitch, 2.0 * DEG2RAD);
    assert_eq!(hpr.roll, 3.0 * DEG2RAD);
}

#[test]
fn conversion_from_quaternion() {
    let testing_tab: [[f64; 3]; 9] = [
        [0.0, 0.0, 0.0],
        [90.0 * DEG2RAD, 0.0, 0.0],
        [-90.0 * DEG2RAD, 0.0, 0.0],
        [0.0, 89.0 * DEG2RAD, 0.0],
        [0.0, -89.0 * DEG2RAD, 0.0],
        [0.0, 0.0, 90.0 * DEG2RAD],
        [0.0, 0.0, -90.0 * DEG2RAD],
        [30.0 * DEG2RAD, 30.0 * DEG2RAD, 30.0 * DEG2RAD],
        [-30.0 * DEG2RAD, -30.0 * DEG2RAD, 45.0 * DEG2RAD],
    ];

    for init in &testing_tab {
        let hpr_input = HeadingPitchRoll::new(init[0], init[1], init[2]);
        let quat = Quaternion::from_heading_pitch_roll_new(&hpr_input);
        let result = HeadingPitchRoll::from_quaternion_new(&quat);

        assert!(
            (init[0] - result.heading).abs() < CesiumMath::EPSILON11,
            "heading mismatch: expected {}, got {}",
            init[0],
            result.heading
        );
        assert!(
            (init[1] - result.pitch).abs() < CesiumMath::EPSILON11,
            "pitch mismatch: expected {}, got {}",
            init[1],
            result.pitch
        );
        assert!(
            (init[2] - result.roll).abs() < CesiumMath::EPSILON11,
            "roll mismatch: expected {}, got {}",
            init[2],
            result.roll
        );
    }
}

#[test]
fn correct_pitch_with_quaternion_rounding_error() {
    let q = Quaternion::new(
        8.801218199179452e-17,
        -0.7071067801637715,
        -8.801218315071006e-17,
        -0.7071067822093238,
    );
    let result = HeadingPitchRoll::from_quaternion_new(&q);
    assert!(
        (result.pitch - (-std::f64::consts::PI / 2.0)).abs() < 1e-10,
        "expected -PI/2, got {}",
        result.pitch
    );
}

#[test]
fn equals() {
    let hpr = HeadingPitchRoll::new(1.0, 2.0, 3.0);
    assert_eq!(hpr, HeadingPitchRoll::new(1.0, 2.0, 3.0));
    assert_ne!(hpr, HeadingPitchRoll::new(2.0, 2.0, 3.0));
    assert_ne!(hpr, HeadingPitchRoll::new(2.0, 1.0, 3.0));
    assert_ne!(hpr, HeadingPitchRoll::new(1.0, 2.0, 4.0));
}

#[test]
fn equals_epsilon_within_relative_tolerance() {
    // Mirrors the CesiumJS HeadingPitchRoll.equalsEpsilon spec and the
    // Phase 2 diff golden (hpr.equalsEpsilon.eq).
    let left = HeadingPitchRoll::new(1.0, 2.0, 3.0);
    let right = HeadingPitchRoll::new(1.001, 2.001, 2.999);
    assert!(HeadingPitchRoll::equals_epsilon(
        Some(&left),
        Some(&right),
        Some(0.01),
        None
    ));
}

#[test]
fn equals_epsilon_outside_tolerance() {
    // Phase 2 diff golden (hpr.equalsEpsilon.neq).
    let left = HeadingPitchRoll::new(1.0, 2.0, 3.0);
    let right = HeadingPitchRoll::new(1.5, 2.0, 3.0);
    assert!(!HeadingPitchRoll::equals_epsilon(
        Some(&left),
        Some(&right),
        Some(0.01),
        None
    ));
}

#[test]
fn equals_epsilon_handles_undefined_like_js() {
    let hpr = HeadingPitchRoll::new(1.0, 2.0, 3.0);
    assert!(HeadingPitchRoll::equals_epsilon(None, None, Some(0.0), None));
    assert!(!HeadingPitchRoll::equals_epsilon(Some(&hpr), None, Some(0.0), None));
    assert!(!HeadingPitchRoll::equals_epsilon(None, Some(&hpr), Some(0.0), None));
}

#[test]
fn equals_epsilon_method_variant() {
    let hpr = HeadingPitchRoll::new(1.0, 2.0, 3.0);
    assert!(hpr.equals_epsilon_method(&HeadingPitchRoll::new(1.0, 2.0, 3.0), Some(0.0), None));
    assert!(!hpr.equals_epsilon_method(&HeadingPitchRoll::new(1.0, 2.0, 4.0), Some(0.1), None));
}
