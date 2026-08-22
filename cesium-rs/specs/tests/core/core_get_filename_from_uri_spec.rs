//! Mirrors packages/engine/Specs/Core/getFilenameFromUriSpec.js

use cesium_core::get_filename_from_uri::get_filename_from_uri;
use cesium_test_utils::expect_to_throw_dev_error;

// describe("Core/getFilenameFromUri")

#[test]
fn works_as_expected() {
    let result = get_filename_from_uri(Some(
        "http://www.mysite.com/awesome?makeitawesome=true",
    ));
    assert_eq!(result, "awesome");

    let result = get_filename_from_uri(Some(
        "http://www.mysite.com/somefolder/awesome.png#makeitawesome",
    ));
    assert_eq!(result, "awesome.png");
}

#[test]
fn throws_with_undefined_parameter() {
    expect_to_throw_dev_error(|| {
        let _ = get_filename_from_uri(None);
    });
}
