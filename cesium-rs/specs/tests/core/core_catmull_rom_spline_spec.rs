use cesium_core::cartesian3::Cartesian3;
use cesium_core::catmull_rom_spline::CatmullRomSpline;
use cesium_core::math::CesiumMath;
use cesium_core::spline::SplinePoint;

fn make_points() -> Vec<SplinePoint> {
    vec![
        SplinePoint::Cartesian3(Cartesian3::new(-1.0, -1.0, 0.0)),
        SplinePoint::Cartesian3(Cartesian3::new(-0.5, -0.125, 0.0)),
        SplinePoint::Cartesian3(Cartesian3::new(0.5, 0.125, 0.0)),
        SplinePoint::Cartesian3(Cartesian3::new(1.0, 1.0, 0.0)),
    ]
}

fn make_times() -> Vec<f64> {
    vec![0.0, 1.0, 2.0, 3.0]
}

fn spline_point_as_cartesian(sp: &SplinePoint) -> Cartesian3 {
    match sp {
        SplinePoint::Cartesian3(c) => *c,
        _ => panic!("Expected Cartesian3 SplinePoint"),
    }
}

#[test]
fn evaluate_at_start_time() {
    let points = make_points();
    let times = make_times();
    let mut crs = CatmullRomSpline::new(times, points.clone());
    let result = crs.evaluate(0.0).unwrap();
    let c = spline_point_as_cartesian(&result);
    assert!((c.x - (-1.0)).abs() < CesiumMath::EPSILON4);
    assert!((c.y - (-1.0)).abs() < CesiumMath::EPSILON4);
    assert!((c.z - 0.0).abs() < CesiumMath::EPSILON4);
}

#[test]
fn evaluate_at_end_time() {
    let points = make_points();
    let times = make_times();
    let mut crs = CatmullRomSpline::new(times, points);
    let result = crs.evaluate(3.0).unwrap();
    let c = spline_point_as_cartesian(&result);
    assert!((c.x - 1.0).abs() < CesiumMath::EPSILON4);
    assert!((c.y - 1.0).abs() < CesiumMath::EPSILON4);
    assert!((c.z - 0.0).abs() < CesiumMath::EPSILON4);
}

#[test]
fn evaluate_at_midpoint() {
    let points = make_points();
    let times = make_times();
    let mut crs = CatmullRomSpline::new(times, points);
    // Evaluate at t=1.5 (midpoint between times[1] and times[2])
    let result = crs.evaluate(1.5).unwrap();
    let c = spline_point_as_cartesian(&result);
    // Should be between (-0.5, -0.125) and (0.5, 0.125), roughly near (0, 0, 0)
    assert!(c.x.abs() < 0.5);
    assert!(c.y.abs() < 0.5);
}

#[test]
fn evaluate_out_of_range_returns_none() {
    let points = make_points();
    let times = make_times();
    let mut crs = CatmullRomSpline::new(times, points);
    // Time before start
    assert!(crs.evaluate(-1.0).is_none());
    // Time after end
    assert!(crs.evaluate(4.0).is_none());
}

#[test]
fn spline_with_2_control_points_defaults_to_lerp() {
    let points = vec![
        SplinePoint::Cartesian3(Cartesian3::new(-1.0, -1.0, 0.0)),
        SplinePoint::Cartesian3(Cartesian3::new(-0.5, -0.125, 0.0)),
    ];
    let times = vec![0.0, 1.0];
    let mut crs = CatmullRomSpline::new(times.clone(), points.clone());

    let t = (times[0] + times[1]) * 0.5;
    let result = crs.evaluate(t).unwrap();
    let c = spline_point_as_cartesian(&result);

    // Lerp at t=0.5 between (-1,-1,0) and (-0.5,-0.125,0)
    let expected_x = -1.0 + 0.5 * (-0.5 - (-1.0));
    let expected_y = -1.0 + 0.5 * (-0.125 - (-1.0));
    assert!((c.x - expected_x).abs() < CesiumMath::EPSILON4);
    assert!((c.y - expected_y).abs() < CesiumMath::EPSILON4);
}

#[test]
fn wrap_time_works() {
    let points = make_points();
    let times = make_times();
    let crs = CatmullRomSpline::new(times, points);
    // wrap_time should wrap a time outside [0, 3] back into range
    let wrapped = crs.wrap_time(4.0);
    assert!(wrapped >= 0.0 && wrapped <= 3.0);
}

#[test]
fn clamp_time_works() {
    let points = make_points();
    let times = make_times();
    let crs = CatmullRomSpline::new(times, points);
    assert_eq!(crs.clamp_time(-1.0), 0.0);
    assert_eq!(crs.clamp_time(5.0), 3.0);
    assert_eq!(crs.clamp_time(1.5), 1.5);
}
