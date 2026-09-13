//! Tests ported from CesiumJS:
//! - isLeapYearSpec.js (1 A-class test)
//! - getStringFromTypedArraySpec.js (5 A-class tests)
//! Total: 6 tests

// ===== isLeapYear =====

#[test]
fn is_leap_year_valid_years() {
    // Ported from: "Check for valid leap years"
    use cesium_time::is_leap_year;

    // Standard leap years (divisible by 4, not by 100)
    assert!(is_leap_year(2000)); // divisible by 400
    assert!(is_leap_year(2004));
    assert!(is_leap_year(2008));
    assert!(is_leap_year(2012));
    assert!(is_leap_year(2016));
    assert!(is_leap_year(2020));
    assert!(is_leap_year(2024));
    assert!(is_leap_year(1600)); // divisible by 400
    assert!(is_leap_year(1200)); // divisible by 400

    // Non-leap years
    assert!(!is_leap_year(2001));
    assert!(!is_leap_year(2002));
    assert!(!is_leap_year(2003));
    assert!(!is_leap_year(2005));
    assert!(!is_leap_year(1900)); // divisible by 100 but not 400
    assert!(!is_leap_year(2100)); // divisible by 100 but not 400
    assert!(!is_leap_year(1800)); // divisible by 100 but not 400
    assert!(!is_leap_year(1700)); // divisible by 100 but not 400
}

// ===== getStringFromTypedArray =====
// In Rust, this maps to String::from_utf8 / std::str::from_utf8
// We test the equivalent behavior

/// Converts a byte slice (UTF-8) to a String.
/// Maps to CesiumJS `getStringFromTypedArray(array, byteOffset, byteLength)`
fn get_string_from_typed_array(data: &[u8], byte_offset: usize, byte_length: Option<usize>) -> String {
    let len = byte_length.unwrap_or(data.len() - byte_offset);
    let slice = &data[byte_offset..byte_offset + len];
    String::from_utf8(slice.to_vec()).expect("Invalid UTF-8")
}

#[test]
fn converts_typed_array_to_string() {
    // Ported from: "converts a typed array to string"
    let arr: &[u8] = &[67, 101, 115, 105, 117, 109]; // "Cesium"
    let string = get_string_from_typed_array(arr, 0, None);
    assert_eq!(string, "Cesium");

    // Empty array
    let arr: &[u8] = &[];
    let string = get_string_from_typed_array(arr, 0, None);
    assert_eq!(string, "");
}

#[test]
fn converts_sub_region_of_typed_array_to_string() {
    // Ported from: "converts a sub-region of a typed array to a string"
    let arr: &[u8] = &[67, 101, 115, 105, 117, 109]; // "Cesium"
    let string = get_string_from_typed_array(arr, 1, Some(3));
    assert_eq!(string, "esi");
}

#[test]
fn unicode_2_byte_characters_work() {
    // Ported from: "Unicode 2-byte characters work"
    // "Zürich" in UTF-8: Z=90, ü=195,188, r=114, i=105, c=99, h=104
    let arr: &[u8] = &[90, 195, 188, 114, 105, 99, 104];
    let string = get_string_from_typed_array(arr, 0, None);
    assert_eq!(string, "Zürich");
}

#[test]
fn unicode_3_byte_characters_work() {
    // Ported from: "Unicode 3-byte characters work"
    // U+08A0 (ࢠ) in UTF-8: 224, 162, 160
    let arr: &[u8] = &[224, 162, 160];
    let string = get_string_from_typed_array(arr, 0, None);
    assert_eq!(string, "ࢠ");
}

#[test]
fn unicode_4_byte_characters_work() {
    // Ported from: "Unicode 4-byte characters work"
    // U+10281 (𐊁) in UTF-8: 240, 144, 138, 129
    let arr: &[u8] = &[240, 144, 138, 129];
    let string = get_string_from_typed_array(arr, 0, None);
    assert_eq!(string, "𐊁");
}
