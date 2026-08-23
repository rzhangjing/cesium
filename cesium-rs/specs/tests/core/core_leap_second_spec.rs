//! Port of `Core/LeapSecondSpec.js`.

use cesium_core::julian_date::JulianDate;
use cesium_core::leap_second::LeapSecond;
use cesium_core::time_standard::TimeStandard;

#[test]
fn default_constructor_sets_expected_values() {
    let ls = LeapSecond::new(JulianDate::default_date(), 0.0);
    // JulianDate default is a valid date, offset is 0.0
    assert_eq!(ls.offset, 0.0);
}

#[test]
fn constructor_sets_expected_values() {
    let date = JulianDate::new(2451545.0, 0.0, TimeStandard::UTC);
    let offset = 12.0;
    let ls = LeapSecond::new(date.clone(), offset);
    assert_eq!(ls.julian_date, date);
    assert_eq!(ls.offset, 12.0);
}
