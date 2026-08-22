//! Mirrors packages/engine/Specs/Core/isDataUriSpec.js

use cesium_core::is_data_uri::is_data_uri;
use cesium_test_utils::expect_to_throw_dev_error;

// describe("Core/isDataUri")

#[test]
fn throws_if_url_is_undefined() {
    expect_to_throw_dev_error(|| {
        let _ = is_data_uri(None);
    });
}

#[test]
fn determines_that_a_uri_is_not_a_data_uri() {
    assert!(!is_data_uri(Some("http://cesiumjs.org/")));
}

#[test]
fn determines_that_a_uri_is_a_data_uri() {
    // JS: `data:text/plain;base64,${btoa("a data uri")}`
    // btoa("a data uri") === "YSBkYXRhIHVyaQ=="
    let uri = "data:text/plain;base64,YSBkYXRhIHVyaQ==";
    assert!(is_data_uri(Some(uri)));
}
