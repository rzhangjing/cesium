//! Mirrors packages/engine/Specs/Core/isBlobUriSpec.js

use cesium_core::is_blob_uri::is_blob_uri;
use cesium_test_utils::expect_to_throw_dev_error;

// describe("Core/isBlobUri")

#[test]
fn throws_if_url_is_undefined() {
    expect_to_throw_dev_error(|| {
        let _ = is_blob_uri(None);
    });
}

#[test]
fn determines_that_a_uri_is_not_a_blob_uri() {
    assert!(!is_blob_uri(Some("http://cesiumjs.org/")));
}

#[test]
fn determines_that_a_uri_is_a_blob_uri() {
    // DEVIATION: the JS spec builds a Blob and calls
    // `window.URL.createObjectURL(blob)`; native builds have no Blob/URL
    // registry, so the canonical blob URI shape produced by that API
    // ("blob:<origin>/<uuid>") is tested with a literal instead. See
    // docs/deviations.md.
    let blob_url = "blob:http://localhost/3d9c8e6a-1b2f-4a5d-9c7e-0f6a1b2c3d4e";
    assert!(is_blob_uri(Some(blob_url)));
}
