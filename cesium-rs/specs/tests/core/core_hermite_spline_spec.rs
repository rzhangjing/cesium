use cesium_core::cartesian3::Cartesian3;
use cesium_core::hermite_spline::HermiteSpline;
use cesium_core::math::CesiumMath;
use cesium_core::spline::SplinePoint;

fn make_scalar_spline() -> HermiteSpline {
    let times = vec![0.0, 1.0, 2.0, 3.0];
    let points = vec![
        SplinePoint::Scalar(0.0),
        SplinePoint::Scalar(1.0),
        SplinePoint::Scalar(4.0),
        SplinePoint::Scalar(9.0),
    ];
    let out_tangents = vec![
        SplinePoint::Scalar(1.0),
        SplinePoint::Scalar(3.0),
        SplinePoint::Scalar(5.0),
        SplinePoint::Scalar(7.0),
    ];
    let in_tangents = vec![
        SplinePoint::Scalar(1.0),
        SplinePoint::Scalar(3.0),
        SplinePoint::Scalar(5.0),
        SplinePoint::Scalar(7.0),
    ];
    HermiteSpline::new(times, points, in_tangents, out_tangents)
}

fn make_cartesian3_spline() -> HermiteSpline {
    let times = vec![0.0, 1.0, 2.0, 3.0];
    let points = vec![
        SplinePoint::Cartesian3(Cartesian3::new(-1.0, -1.0, 0.0)),
        SplinePoint::Cartesian3(Cartesian3::new(-0.5, -0.125, 0.0)),
        SplinePoint::Cartesian3(Cartesian3::new(0.5, 0.125, 0.0)),
        SplinePoint::Cartesian3(Cartesian3::new(1.0, 1.0, 0.0)),
    ];
    let out_tangents = vec![
        SplinePoint::Cartesian3(Cartesian3::new(1.5, 1.5, 0.0)),
        SplinePoint::Cartesian3(Cartesian3::new(1.0, 0.5, 0.0)),
        SplinePoint::Cartesian3(Cartesian3::new(1.0, 0.5, 0.0)),
        SplinePoint::Cartesian3(Cartesian3::new(1.5, 1.5, 0.0)),
    ];
    let in_tangents = vec![
        SplinePoint::Cartesian3(Cartesian3::new(1.5, 1.5, 0.0)),
        SplinePoint::Cartesian3(Cartesian3::new(1.0, 0.5, 0.0)),
        SplinePoint::Cartesian3(Cartesian3::new(1.0, 0.5, 0.0)),
        SplinePoint::Cartesian3(Cartesian3::new(1.5, 1.5, 0.0)),
    ];
    HermiteSpline::new(times, points, in_tangents, out_tangents)
}

#[test]
fn evaluate_scalar_at_start() {
    let mut spline = make_scalar_spline();
    let result = spline.evaluate(0.0).unwrap();
    match result {
        SplinePoint::Scalar(v) => assert!((v - 0.0).abs() < CesiumMath::EPSILON10),
        _ => panic!("expected Scalar"),
    }
}

#[test]
fn evaluate_scalar_at_midpoint() {
    let mut spline = make_scalar_spline();
    let result = spline.evaluate(0.5).unwrap();
    match result {
        SplinePoint::Scalar(v) => {
            // Hermite interpolation at t=0.5 between p0=0, p1=1 with m0=1, m1=3, dt=1
            // h00=0.5, h10=-0.125 (scaled by dt=1 => -0.125), h01=0.5, h11=0.125 (scaled => 0.125*1)
            // Actually: h00=2*0.125-3*0.25+1=0.5, h10=0.125-2*0.25+0.5=0.125, h01=-2*0.125+3*0.25=0.5, h11=0.125-0.25=-0.125
            // result = 0.5*0 + 0.125*1*1 + 0.5*1 + (-0.125)*3*1 = 0 + 0.125 + 0.5 - 0.375 = 0.25
            assert!((v - 0.25).abs() < CesiumMath::EPSILON10, "expected ~0.25, got {}", v);
        }
        _ => panic!("expected Scalar"),
    }
}

#[test]
fn evaluate_cartesian3_at_start() {
    let mut spline = make_cartesian3_spline();
    let result = spline.evaluate(0.0).unwrap();
    match result {
        SplinePoint::Cartesian3(v) => {
            assert!((v.x - (-1.0)).abs() < CesiumMath::EPSILON10);
            assert!((v.y - (-1.0)).abs() < CesiumMath::EPSILON10);
            assert!((v.z - 0.0).abs() < CesiumMath::EPSILON10);
        }
        _ => panic!("expected Cartesian3"),
    }
}

#[test]
fn evaluate_cartesian3_at_end() {
    let mut spline = make_cartesian3_spline();
    // evaluate at last time: find_time_interval returns length-2 for time == times[last]
    let result = spline.evaluate(3.0).unwrap();
    match result {
        SplinePoint::Cartesian3(v) => {
            assert!((v.x - 1.0).abs() < CesiumMath::EPSILON10);
            assert!((v.y - 1.0).abs() < CesiumMath::EPSILON10);
            assert!((v.z - 0.0).abs() < CesiumMath::EPSILON10);
        }
        _ => panic!("expected Cartesian3"),
    }
}

#[test]
fn evaluate_out_of_range_returns_none() {
    let mut spline = make_scalar_spline();
    assert!(spline.evaluate(-1.0).is_none());
    assert!(spline.evaluate(4.0).is_none());
}

#[test]
fn clamp_time_works() {
    let spline = make_scalar_spline();
    assert_eq!(spline.clamp_time(-5.0), 0.0);
    assert_eq!(spline.clamp_time(10.0), 3.0);
    assert_eq!(spline.clamp_time(1.5), 1.5);
}

#[test]
fn wrap_time_works() {
    let spline = make_scalar_spline();
    // times = [0, 1, 2, 3], stretch = 3
    let wrapped = spline.wrap_time(4.0);
    // 4.0 > 3.0: divs = floor((4-3)/3) + 1 = 0+1 = 1, t = 4 - 1*3 = 1.0
    assert!((wrapped - 1.0).abs() < CesiumMath::EPSILON10);

    let wrapped_neg = spline.wrap_time(-1.0);
    // -1.0 < 0.0: divs = floor((0-(-1))/3) + 1 = floor(0.333)+1 = 0+1 = 1, t = -1 + 1*3 = 2.0
    assert!((wrapped_neg - 2.0).abs() < CesiumMath::EPSILON10);
}
