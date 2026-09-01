use cesium_core::cartesian3::Cartesian3;
use cesium_core::catmull_rom_spline::CatmullRomSpline;
use cesium_core::hermite_spline::HermiteSpline;
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

#[test]
#[should_panic]
fn constructor_panics_when_control_points_length_is_less_than_2() {
    let _ = CatmullRomSpline::new(
        vec![0.0],
        vec![SplinePoint::Cartesian3(Cartesian3::ZERO)],
    );
}

#[test]
#[should_panic]
fn constructor_panics_when_times_length_is_not_equal_to_points_length() {
    let _ = CatmullRomSpline::new(vec![0.0, 1.0], make_points());
}

#[test]
fn sets_start_and_end_tangents() {
    let points = make_points();
    let start = spline_point_sub(&points[1], &points[0]);
    let end = spline_point_sub(&points[points.len() - 1], &points[points.len() - 2]);

    let crs = CatmullRomSpline::new_with_tangents(
        make_times(),
        points,
        Some(start.clone()),
        Some(end.clone()),
    );

    assert_spline_point_eq(crs.first_tangent().unwrap(), &start);
    assert_spline_point_eq(crs.last_tangent().unwrap(), &end);
}

#[test]
fn computes_start_and_end_tangents() {
    let points = make_points();

    // start = 0.5 * (2*points[1] - points[2] - points[0])
    let start = spline_point_scale(
        &spline_point_sub(
            &spline_point_sub(&spline_point_scale(&points[1], 2.0), &points[2]),
            &points[0],
        ),
        0.5,
    );

    let n = points.len() - 1;
    // end = 0.5 * (points[n] - 2*points[n-1] + points[n-2])
    let end = spline_point_scale(
        &spline_point_add(
            &spline_point_sub(&points[n], &spline_point_scale(&points[n - 1], 2.0)),
            &points[n - 2],
        ),
        0.5,
    );

    let crs = CatmullRomSpline::new(make_times(), points);

    assert_spline_point_eq(crs.first_tangent().unwrap(), &start);
    assert_spline_point_eq(crs.last_tangent().unwrap(), &end);
}

#[test]
fn check_catmull_rom_spline_against_a_hermite_spline() {
    let points = make_points();
    let times = make_times();
    let mut crs = CatmullRomSpline::new(times.clone(), points.clone());

    // tangents = [firstTangent, 0.5*(points[i+1]-points[i-1])..., lastTangent]
    let mut tangents = vec![crs.first_tangent().unwrap().clone_point()];
    for i in 1..points.len() - 1 {
        tangents.push(spline_point_scale(&spline_point_sub(&points[i + 1], &points[i - 1]), 0.5));
    }
    tangents.push(crs.last_tangent().unwrap().clone_point());

    // createC1: outTangents = tangents[0..n-1], inTangents = tangents[1..n];
    // the Rust HermiteSpline indexes in_tangents[i + 1], so pass the full
    // array for both (unit spacing makes the dt scaling a no-op).
    let mut hs = HermiteSpline::new(times.clone(), points, tangents.clone(), tangents);

    let granularity = 0.5;
    let mut j = times[0];
    while j <= times[times.len() - 1] {
        let expected = hs.evaluate(j).unwrap();
        let actual = crs.evaluate(j).unwrap();
        assert_spline_point_eq_epsilon(&actual, &expected, CesiumMath::EPSILON4);
        j += granularity;
    }
}

// --- helpers mirroring the Cartesian3 ops used by the JS spec --------------

fn spline_point_sub(a: &SplinePoint, b: &SplinePoint) -> SplinePoint {
    match (a, b) {
        (SplinePoint::Cartesian3(va), SplinePoint::Cartesian3(vb)) => {
            let mut result = Cartesian3::ZERO;
            Cartesian3::subtract(va, vb, &mut result);
            SplinePoint::Cartesian3(result)
        }
        (SplinePoint::Scalar(va), SplinePoint::Scalar(vb)) => SplinePoint::Scalar(va - vb),
        _ => panic!("SplinePoint variant mismatch"),
    }
}

fn spline_point_add(a: &SplinePoint, b: &SplinePoint) -> SplinePoint {
    match (a, b) {
        (SplinePoint::Cartesian3(va), SplinePoint::Cartesian3(vb)) => {
            let mut result = Cartesian3::ZERO;
            Cartesian3::add(va, vb, &mut result);
            SplinePoint::Cartesian3(result)
        }
        (SplinePoint::Scalar(va), SplinePoint::Scalar(vb)) => SplinePoint::Scalar(va + vb),
        _ => panic!("SplinePoint variant mismatch"),
    }
}

fn spline_point_scale(a: &SplinePoint, s: f64) -> SplinePoint {
    match a {
        SplinePoint::Cartesian3(v) => {
            let mut result = Cartesian3::ZERO;
            Cartesian3::multiply_by_scalar(v, s, &mut result);
            SplinePoint::Cartesian3(result)
        }
        SplinePoint::Scalar(v) => SplinePoint::Scalar(v * s),
    }
}

fn assert_spline_point_eq(actual: &SplinePoint, expected: &SplinePoint) {
    match (actual, expected) {
        (SplinePoint::Cartesian3(a), SplinePoint::Cartesian3(e)) => {
            assert_eq!(a.x, e.x);
            assert_eq!(a.y, e.y);
            assert_eq!(a.z, e.z);
        }
        (SplinePoint::Scalar(a), SplinePoint::Scalar(e)) => assert_eq!(a, e),
        _ => panic!("SplinePoint variant mismatch"),
    }
}

fn assert_spline_point_eq_epsilon(actual: &SplinePoint, expected: &SplinePoint, epsilon: f64) {
    match (actual, expected) {
        (SplinePoint::Cartesian3(a), SplinePoint::Cartesian3(e)) => {
            assert!((a.x - e.x).abs() < epsilon, "x: {} vs {}", a.x, e.x);
            assert!((a.y - e.y).abs() < epsilon, "y: {} vs {}", a.y, e.y);
            assert!((a.z - e.z).abs() < epsilon, "z: {} vs {}", a.z, e.z);
        }
        (SplinePoint::Scalar(a), SplinePoint::Scalar(e)) => {
            assert!((a - e).abs() < epsilon);
        }
        _ => panic!("SplinePoint variant mismatch"),
    }
}
