//! Tests for EarthOrientationParameters, EarthOrientationParametersSample,
//! Iau2006XysData, Iau2006XysSample, IauOrientationParameters,
//! IauOrientationAxes, SplinePoint, find_time_interval, wrap_time,
//! clamp_time, IonGeocodeProviderType, GoogleMaps.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::earth_orientation_parameters::{EarthOrientationParameters, EopData};
use cesium_core::earth_orientation_parameters_sample::EarthOrientationParametersSample;
use cesium_core::google_maps::{GoogleMaps, MAP_TILES_API_ENDPOINT, STREET_VIEW_STATIC_API_ENDPOINT};
use cesium_core::iau2006_xys_data::{Iau2006XysData, Iau2006XysDataOptions};
use cesium_core::iau2006_xys_sample::Iau2006XysSample;
use cesium_core::iau_orientation_axes::IauOrientationAxes;
use cesium_core::iau_orientation_parameters::IauOrientationParameters;
use cesium_core::ion_geocode_provider_type::IonGeocodeProviderType;
use cesium_core::julian_date::JulianDate;
use cesium_core::spline::{clamp_time, find_time_interval, wrap_time, SplinePoint};
use cesium_core::time_standard::TimeStandard;

// --- EarthOrientationParametersSample ---
#[test]
fn eop_sample_new() {
    let s = EarthOrientationParametersSample::new(0.1, 0.2, 0.3, 0.4, 0.5);
    assert_eq!(s.x_pole_wander, 0.1);
    assert_eq!(s.y_pole_wander, 0.2);
    assert_eq!(s.x_pole_offset, 0.3);
    assert_eq!(s.y_pole_offset, 0.4);
    assert_eq!(s.ut1_minus_utc, 0.5);
}

#[test]
fn eop_sample_clone_eq() {
    let s = EarthOrientationParametersSample::new(1.0, 2.0, 3.0, 4.0, 5.0);
    let c = s;
    assert_eq!(c, s);
}

// --- EarthOrientationParameters ---
#[test]
fn eop_default_no_data() {
    let eop = EarthOrientationParameters::new(None, None);
    let date = JulianDate::new(2451545.0, 0.0, TimeStandard::TAI);
    let mut result = EarthOrientationParametersSample::new(99.0, 99.0, 99.0, 99.0, 99.0);
    eop.compute(&date, &mut result);
    assert_eq!(result.x_pole_wander, 0.0);
    assert_eq!(result.y_pole_wander, 0.0);
    assert_eq!(result.x_pole_offset, 0.0);
    assert_eq!(result.y_pole_offset, 0.0);
    assert_eq!(result.ut1_minus_utc, 0.0);
}

#[test]
fn eop_with_data_interpolates() {
    let data = EopData {
        column_names: vec![
            "modifiedJulianDateUtc".into(),
            "xPoleWanderRadians".into(),
            "yPoleWanderRadians".into(),
            "ut1MinusUtcSeconds".into(),
            "lengthOfDayCorrectionSeconds".into(),
            "xCelestialPoleOffsetRadians".into(),
            "yCelestialPoleOffsetRadians".into(),
            "taiMinusUtcSeconds".into(),
        ],
        samples: vec![
            50000.0, 0.001, 0.002, 0.5, 0.0, 0.0001, 0.0002, 37.0,
            50001.0, 0.003, 0.004, 0.7, 0.0, 0.0003, 0.0004, 37.0,
        ],
    };
    let eop = EarthOrientationParameters::new(Some(data), None);
    let date = JulianDate::new(50000.0 + 2400000.5, 37.0, TimeStandard::TAI);
    let mut result = EarthOrientationParametersSample::new(0.0, 0.0, 0.0, 0.0, 0.0);
    eop.compute(&date, &mut result);
    // Should have non-zero values from the data
    assert!(result.x_pole_wander != 0.0 || result.ut1_minus_utc != 0.0);
}

// --- Iau2006XysSample ---
#[test]
fn xys_sample_new() {
    let s = Iau2006XysSample::new(1.0, 2.0, 3.0);
    assert_eq!(s.x, 1.0);
    assert_eq!(s.y, 2.0);
    assert_eq!(s.s, 3.0);
}

#[test]
fn xys_sample_eq() {
    let a = Iau2006XysSample::new(1.0, 2.0, 3.0);
    let b = Iau2006XysSample::new(1.0, 2.0, 3.0);
    assert_eq!(a, b);
}

// --- Iau2006XysData ---
#[test]
fn xys_data_default_construction() {
    let xys = Iau2006XysData::new(None);
    let _ = xys;
}

#[test]
fn xys_data_with_options() {
    let opts = Iau2006XysDataOptions {
        interpolation_order: Some(3),
        step_size_days: Some(0.5),
        ..Default::default()
    };
    let xys = Iau2006XysData::new(Some(opts));
    let _ = xys;
}

#[test]
fn xys_data_compute_returns_none_without_data() {
    let mut xys = Iau2006XysData::new(None);
    let result = xys.compute_xys_radians(0, 0.0, &mut None);
    // Without loaded data, samples are all None → returns None
    assert!(result.is_none());
}

// --- IauOrientationParameters ---
#[test]
fn iau_params_default() {
    let p = IauOrientationParameters::default();
    assert_eq!(p.right_ascension, 0.0);
    assert_eq!(p.declination, 0.0);
    assert_eq!(p.rotation, 0.0);
    assert_eq!(p.rotation_rate, 0.0);
}

#[test]
fn iau_params_new() {
    let p = IauOrientationParameters::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(p.right_ascension, 1.0);
    assert_eq!(p.declination, 2.0);
    assert_eq!(p.rotation, 3.0);
    assert_eq!(p.rotation_rate, 4.0);
}

#[test]
fn iau_params_eq() {
    let a = IauOrientationParameters::new(1.0, 2.0, 3.0, 4.0);
    let b = IauOrientationParameters::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(a, b);
}

// --- IauOrientationAxes ---
#[test]
fn iau_axes_default_compute() {
    let axes = IauOrientationAxes::new(None);
    let date = JulianDate::new(2451545.0, 0.0, TimeStandard::TAI);
    let mut result = cesium_core::matrix3::Matrix3::default();
    axes.evaluate(&date, &mut result);
    // Should produce a non-zero rotation matrix
    let det = result.elements[0] * (result.elements[4] * result.elements[8] - result.elements[5] * result.elements[7])
        - result.elements[1] * (result.elements[3] * result.elements[8] - result.elements[5] * result.elements[6])
        + result.elements[2] * (result.elements[3] * result.elements[7] - result.elements[4] * result.elements[6]);
    assert!((det.abs() - 1.0).abs() < 0.01);
}

// --- SplinePoint ---
#[test]
fn spline_point_scalar_lerp() {
    let a = SplinePoint::Scalar(0.0);
    let b = SplinePoint::Scalar(10.0);
    match SplinePoint::lerp(&a, &b, 0.5) {
        SplinePoint::Scalar(v) => assert!((v - 5.0).abs() < 1e-10),
        _ => panic!("expected Scalar"),
    }
}

#[test]
fn spline_point_scalar_lerp_endpoints() {
    let a = SplinePoint::Scalar(3.0);
    let b = SplinePoint::Scalar(7.0);
    match SplinePoint::lerp(&a, &b, 0.0) {
        SplinePoint::Scalar(v) => assert!((v - 3.0).abs() < 1e-10),
        _ => panic!("expected Scalar"),
    }
    match SplinePoint::lerp(&a, &b, 1.0) {
        SplinePoint::Scalar(v) => assert!((v - 7.0).abs() < 1e-10),
        _ => panic!("expected Scalar"),
    }
}

#[test]
fn spline_point_cartesian3_lerp() {
    let a = SplinePoint::Cartesian3(Cartesian3::new(0.0, 0.0, 0.0));
    let b = SplinePoint::Cartesian3(Cartesian3::new(10.0, 20.0, 30.0));
    match SplinePoint::lerp(&a, &b, 0.5) {
        SplinePoint::Cartesian3(v) => {
            assert!((v.x - 5.0).abs() < 1e-10);
            assert!((v.y - 10.0).abs() < 1e-10);
            assert!((v.z - 15.0).abs() < 1e-10);
        }
        _ => panic!("expected Cartesian3"),
    }
}

#[test]
fn spline_point_clone_point() {
    let p = SplinePoint::Scalar(42.0);
    let c = p.clone_point();
    match c {
        SplinePoint::Scalar(v) => assert_eq!(v, 42.0),
        _ => panic!("expected Scalar"),
    }
}

#[test]
fn spline_point_mixed_types_returns_first() {
    let a = SplinePoint::Scalar(1.0);
    let b = SplinePoint::Cartesian3(Cartesian3::new(0.0, 0.0, 0.0));
    match SplinePoint::lerp(&a, &b, 0.5) {
        SplinePoint::Scalar(v) => assert_eq!(v, 1.0),
        _ => panic!("expected Scalar for mixed types"),
    }
}

// --- find_time_interval ---
#[test]
fn find_time_interval_basic() {
    let times = [0.0, 1.0, 2.0, 3.0, 4.0];
    assert_eq!(find_time_interval(&times, 1.5, None), Some(1));
}

#[test]
fn find_time_interval_at_start() {
    let times = [0.0, 1.0, 2.0];
    assert_eq!(find_time_interval(&times, 0.0, None), Some(0));
}

#[test]
fn find_time_interval_at_end() {
    let times = [0.0, 1.0, 2.0];
    // time == last element → out of range (time > times[length-1] is false, but time >= times[length-1])
    let result = find_time_interval(&times, 2.0, None);
    assert!(result.is_some());
}

#[test]
fn find_time_interval_before_start() {
    let times = [0.0, 1.0, 2.0];
    assert_eq!(find_time_interval(&times, -1.0, None), None);
}

#[test]
fn find_time_interval_too_short() {
    let times = [0.0];
    assert_eq!(find_time_interval(&times, 0.0, None), None);
}

#[test]
fn find_time_interval_with_hint() {
    let times = [0.0, 1.0, 2.0, 3.0, 4.0];
    assert_eq!(find_time_interval(&times, 2.5, Some(2)), Some(2));
}

// --- wrap_time ---
#[test]
fn wrap_time_in_range() {
    let times = [0.0, 10.0];
    assert_eq!(wrap_time(&times, 5.0), 5.0);
}

#[test]
fn wrap_time_below_range() {
    let times = [0.0, 10.0];
    let wrapped = wrap_time(&times, -5.0);
    assert!(wrapped >= 0.0 && wrapped <= 10.0);
}

#[test]
fn wrap_time_above_range() {
    let times = [0.0, 10.0];
    let wrapped = wrap_time(&times, 15.0);
    assert!(wrapped >= 0.0 && wrapped <= 10.0);
}

// --- clamp_time ---
#[test]
fn clamp_time_in_range() {
    let times = [0.0, 10.0];
    assert_eq!(clamp_time(&times, 5.0), 5.0);
}

#[test]
fn clamp_time_below() {
    let times = [0.0, 10.0];
    assert_eq!(clamp_time(&times, -5.0), 0.0);
}

#[test]
fn clamp_time_above() {
    let times = [0.0, 10.0];
    assert_eq!(clamp_time(&times, 15.0), 10.0);
}

// --- IonGeocodeProviderType ---
#[test]
fn ion_geocode_provider_type_variants() {
    assert_eq!(IonGeocodeProviderType::Google, IonGeocodeProviderType::Google);
    assert_ne!(IonGeocodeProviderType::Google, IonGeocodeProviderType::Bing);
}

#[test]
fn ion_geocode_provider_type_as_str() {
    assert_eq!(IonGeocodeProviderType::Google.as_str(), "GOOGLE");
    assert_eq!(IonGeocodeProviderType::Bing.as_str(), "BING");
    assert_eq!(IonGeocodeProviderType::Default.as_str(), "DEFAULT");
}

// --- GoogleMaps ---
#[test]
fn google_maps_default() {
    let gm = GoogleMaps::default();
    assert!(gm.default_api_key.is_none());
    assert_eq!(gm.map_tiles_api_endpoint, MAP_TILES_API_ENDPOINT);
    assert!(gm.default_street_view_static_api_key.is_none());
    assert_eq!(gm.street_view_static_api_endpoint, STREET_VIEW_STATIC_API_ENDPOINT);
}

#[test]
fn google_maps_constants() {
    assert!(MAP_TILES_API_ENDPOINT.contains("googleapis"));
    assert!(STREET_VIEW_STATIC_API_ENDPOINT.contains("streetview"));
}
