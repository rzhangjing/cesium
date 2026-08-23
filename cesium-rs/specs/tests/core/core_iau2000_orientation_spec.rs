//! Port of `Core/Iau2000OrientationSpec.js`.

use cesium_core::iau2000_orientation;
use cesium_core::julian_date::JulianDate;
use cesium_core::time_standard::TimeStandard;

#[test]
fn compute_moon() {
    let date = JulianDate::new(2451545.0, -32.184, TimeStandard::TAI);
    let mut result = cesium_core::iau_orientation_parameters::IauOrientationParameters::default();
    iau2000_orientation::compute_moon(&date, &mut result);

    // Expected results from STK Components (Iau2000Orientation.ComputeMoon(TimeConstants.J2000))
    cesium_test_utils::assert_approx_eq_f64!(result.right_ascension, 4.6575460830237914, 1e-12);
    cesium_test_utils::assert_approx_eq_f64!(result.declination, 1.1456533675897986, 1e-12);
    cesium_test_utils::assert_approx_eq_f64!(result.rotation, 0.71899299269222972, 1e-12);
    cesium_test_utils::assert_approx_eq_f64!(result.rotation_rate, 0.0000026518066425764541, 1e-15);
}
