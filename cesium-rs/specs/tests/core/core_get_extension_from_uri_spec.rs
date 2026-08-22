//! Mirrors packages/engine/Specs/Core/getExtensionFromUriSpec.js

use cesium_core::get_extension_from_uri::get_extension_from_uri;
use cesium_test_utils::expect_to_throw_dev_error;

// describe("Core/getExtensionFromUri")

#[test]
fn works_as_expected() {
    let result = get_extension_from_uri(Some(
        "http://www.mysite.com/awesome?makeitawesome=true",
    ));
    assert_eq!(result, "");

    let result = get_extension_from_uri(Some(
        "http://www.mysite.com/somefolder/awesome.png#makeitawesome",
    ));
    assert_eq!(result, "png");

    let result = get_extension_from_uri(Some("awesome.png"));
    assert_eq!(result, "png");
}

#[test]
fn throws_with_undefined_parameter() {
    expect_to_throw_dev_error(|| {
        let _ = get_extension_from_uri(None);
    });
}
