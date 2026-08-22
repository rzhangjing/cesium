//! Mirrors packages/engine/Specs/Core/getAbsoluteUriSpec.js

use cesium_core::get_absolute_uri::{
    get_absolute_uri, get_absolute_uri_implementation, DocumentLike,
};
use cesium_test_utils::expect_to_throw_dev_error;

// describe("Core/getAbsoluteUri")

#[test]
fn works_as_expected() {
    let result = get_absolute_uri(
        Some("http://www.mysite.com/awesome?makeitawesome=true"),
        None,
    );
    assert_eq!(
        result,
        "http://www.mysite.com/awesome?makeitawesome=true"
    );

    let result = get_absolute_uri(Some("awesome.png"), Some("http://test.com"));
    assert_eq!(result, "http://test.com/awesome.png");

    // JS third assertion resolves against `document.location.href`; native
    // builds have no document, so the relative URI is returned unchanged
    // (DEVIATION, see docs/deviations.md).
    let result = get_absolute_uri(Some("awesome.png"), None);
    assert_eq!(result, "awesome.png");
}

#[test]
#[ignore = "depends on document.location.href, which does not exist in native builds (DEVIATION)"]
fn resolves_against_document_location_href() {}

#[test]
fn document_base_uri_is_respected() {
    struct FakeDocument;
    impl DocumentLike for FakeDocument {
        fn base_uri(&self) -> Option<String> {
            Some("http://test.com/index.html".to_owned())
        }
        fn location_href(&self) -> Option<String> {
            // JS used the real document.location here; only baseURI matters.
            None
        }
    }

    let result = get_absolute_uri_implementation(Some("awesome.png"), None, Some(&FakeDocument));
    assert_eq!(result, "http://test.com/awesome.png");
}

#[test]
fn throws_with_undefined_parameter() {
    expect_to_throw_dev_error(|| {
        let _ = get_absolute_uri(None, None);
    });
}
