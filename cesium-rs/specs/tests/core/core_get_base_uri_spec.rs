//! Mirrors packages/engine/Specs/Core/getBaseUriSpec.js

use cesium_core::get_base_uri::get_base_uri;
use cesium_test_utils::expect_to_throw_dev_error;

// describe("Core/getBaseUri")

#[test]
fn works_as_expected() {
    let result = get_base_uri(
        Some("http://www.mysite.com/awesome?makeitawesome=true"),
        None,
    );
    assert_eq!(result, "http://www.mysite.com/");

    let result = get_base_uri(
        Some("http://www.mysite.com/somefolder/awesome.png#makeitawesome"),
        None,
    );
    assert_eq!(result, "http://www.mysite.com/somefolder/");
}

#[test]
fn works_with_include_query_flag() {
    let result = get_base_uri(
        Some("http://www.mysite.com/awesome?makeitawesome=true"),
        Some(true),
    );
    assert_eq!(result, "http://www.mysite.com/?makeitawesome=true");

    let result = get_base_uri(
        Some("http://www.mysite.com/somefolder/awesome.png#makeitawesome"),
        Some(true),
    );
    assert_eq!(result, "http://www.mysite.com/somefolder/#makeitawesome");
}

#[test]
fn throws_with_undefined_parameter() {
    expect_to_throw_dev_error(|| {
        let _ = get_base_uri(None, None);
    });
}
