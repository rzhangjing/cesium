//! Mirror of `packages/engine/Specs/Core/ResourceSpec.js` (pure-logic cases).
//!
//! Conventions:
//! - Jasmine `it(...)` titles map to `#[test] fn` names (snake_case).
//! - Async XHR/image/browser-only cases (loadWithXhr spies, `fetchImage`,
//!   XMLHttpRequest header parsing, JSONP, blob object URLs) are not mirrored:
//!   they exercise browser infrastructure that has no Rust counterpart.
//! - DEVIATION (ordering): CesiumJS option objects preserve insertion order;
//!   the Rust port takes `HashMap`s for `queryParameters`/`templateValues`
//!   setters, so url-string assertions that would depend on HashMap iteration
//!   order are relaxed to membership assertions. URL-parsed query order IS
//!   one-to-one (see the `multiple values` tests).
//! - DEVIATION (blob mime): JS blobs carry the mime type; the Rust port
//!   returns raw bytes.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use cesium_core::default_proxy::DefaultProxy;
use cesium_core::resource::{
    FetchParams, MockResourceBackend, QueryValue, Resource, ResourceError, ResourceOptions,
    Response, ResponseType, RetryCallback,
};

fn qp<'a>(resource: &'a Resource, key: &str) -> Option<&'a QueryValue> {
    resource
        .query_parameters()
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
}

fn qp_single<'a>(resource: &'a Resource, key: &str) -> Option<&'a str> {
    match qp(resource, key)? {
        QueryValue::Single(v) => Some(v.as_str()),
        _ => None,
    }
}

fn qp_multiple<'a>(resource: &'a Resource, key: &str) -> Option<&'a Vec<String>> {
    match qp(resource, key)? {
        QueryValue::Multiple(v) => Some(v),
        _ => None,
    }
}

fn map_of(entries: &[(&str, &str)]) -> HashMap<String, String> {
    entries
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

// ── Constructor ──────────────────────────────────────────────────────────

#[test]
fn constructor_sets_correct_properties() {
    let proxy = DefaultProxy::new("/proxy/");

    let resource = Resource::with_options(ResourceOptions {
        url: Some("http://test.com/tileset".to_string()),
        query_parameters: Some(map_of(&[("key1", "value1"), ("key2", "value2")])),
        template_values: Some(map_of(&[("key3", "value3"), ("key4", "value4")])),
        headers: Some(map_of(&[("Accept", "application/test-type")])),
        proxy: Some(proxy.clone()),
        retry_callback: Some(Box::new(|_error| true)),
        retry_attempts: Some(4),
        ..Default::default()
    });

    assert_eq!(
        resource.get_url_component(false, false),
        "http://test.com/tileset"
    );
    // HashMap iteration order is unspecified (JS preserves insertion order);
    // assert membership of both pairs instead of an exact string.
    let with_query = resource.get_url_component(true, false);
    assert!(with_query.starts_with("http://test.com/tileset?"));
    assert!(with_query.contains("key1=value1"));
    assert!(with_query.contains("key2=value2"));
    assert_eq!(
        resource.get_url_component(false, true),
        proxy.get_url("http://test.com/tileset")
    );
    assert_eq!(resource.url(), resource.get_url_component(true, true));
    assert_eq!(resource.url(), format!("{resource}"));
    assert_eq!(qp_single(&resource, "key1"), Some("value1"));
    assert_eq!(qp_single(&resource, "key2"), Some("value2"));
    assert_eq!(resource.template_values().get("key3").map(String::as_str), Some("value3"));
    assert_eq!(resource.template_values().get("key4").map(String::as_str), Some("value4"));
    assert_eq!(resource.headers.get("Accept").map(String::as_str), Some("application/test-type"));
    assert_eq!(resource.proxy(), Some(&proxy));
    assert!(resource.has_retry_callback());
    assert_eq!(resource.retry_attempts(), 4);
    assert_eq!(resource.retry_count(), 0);
}

#[test]
fn constructor_sets_correct_properties_from_url_string() {
    let url = "http://invalid.domain.com/tileset";
    let resource = Resource::new(url.to_string());
    assert_eq!(resource.url(), url);
    assert_eq!(format!("{resource}"), url);
    assert!(resource.query_parameters().is_empty());
    assert!(resource.template_values().is_empty());
    assert!(resource.headers.is_empty());
    assert_eq!(resource.proxy(), None);
    assert!(!resource.has_retry_callback());
    assert_eq!(resource.retry_attempts(), 0);
}

// ── url helpers ──────────────────────────────────────────────────────────

#[test]
fn append_forward_slash_appends_a_slash() {
    let mut resource = Resource::with_options(ResourceOptions {
        url: Some("http://test.com/tileset".to_string()),
        ..Default::default()
    });
    assert_eq!(resource.url(), "http://test.com/tileset");
    resource.append_forward_slash();
    assert_eq!(resource.url(), "http://test.com/tileset/");
}

#[test]
fn setting_a_url_with_a_query_string_sets_query_parameters_correctly() {
    let resource = Resource::with_options(ResourceOptions {
        url: Some("http://test.com/tileset?foo=bar&baz=foo".to_string()),
        ..Default::default()
    });
    assert_eq!(resource.get_url_component(false, false), "http://test.com/tileset");
    assert_eq!(
        resource.get_url_component(true, false),
        "http://test.com/tileset?foo=bar&baz=foo"
    );
    assert_eq!(qp_single(&resource, "foo"), Some("bar"));
    assert_eq!(qp_single(&resource, "baz"), Some("foo"));
}

#[test]
fn constructing_with_parse_url_false_does_not_strip_query_parameters_from_url() {
    let resource = Resource::with_options(ResourceOptions {
        url: Some("http://test.com/tileset?foo=bar&baz=foo".to_string()),
        parse_url: Some(false),
        ..Default::default()
    });
    assert_eq!(
        resource.get_url_component(false, false),
        "http://test.com/tileset?foo=bar&baz=foo"
    );
    assert_eq!(
        resource.get_url_component(true, false),
        "http://test.com/tileset?foo=bar&baz=foo"
    );
    assert!(resource.query_parameters().is_empty());
}

#[test]
fn create_if_needed_returns_resource_if_parameter_is_a_string() {
    let resource = Resource::create_if_needed("http://test.com/tileset");
    assert_eq!(resource.url(), "http://test.com/tileset");
}

// ── Multiple query values (order-sensitive, URL-parsed) ──────────────────

#[test]
fn multiple_values_for_query_parameters_are_allowed() {
    let resource = Resource::new("http://test.com/tileset/endpoint?a=1&a=2&b=3&a=4".to_string());
    assert_eq!(
        qp_multiple(&resource, "a"),
        Some(&vec!["1".to_string(), "2".to_string(), "4".to_string()])
    );
    assert_eq!(qp_single(&resource, "b"), Some("3"));

    assert_eq!(
        resource.url(),
        "http://test.com/tileset/endpoint?a=1&a=2&a=4&b=3"
    );
}

#[test]
fn multiple_values_for_query_parameters_work_with_get_derived_resource_without_preserve() {
    let resource = Resource::new("http://test.com/tileset/endpoint?a=1&a=2&b=3&a=4".to_string());

    let derived = resource.get_derived_resource_with_options(
        cesium_core::resource::DerivedResourceOptions {
            url: Some("other_endpoint?a=5&b=6&a=7"),
            ..Default::default()
        },
    );

    assert_eq!(
        qp_multiple(&derived, "a"),
        Some(&vec!["5".to_string(), "7".to_string()])
    );
    assert_eq!(qp_single(&derived, "b"), Some("6"));

    assert_eq!(
        derived.url(),
        "http://test.com/tileset/other_endpoint?a=5&a=7&b=6"
    );
}

#[test]
fn multiple_values_for_query_parameters_work_with_get_derived_resource_with_preserve() {
    let resource = Resource::new("http://test.com/tileset/endpoint?a=1&a=2&b=3&a=4".to_string());

    let derived = resource.get_derived_resource_with_options(
        cesium_core::resource::DerivedResourceOptions {
            url: Some("other_endpoint?a=5&b=6&a=7"),
            preserve_query_parameters: true,
            ..Default::default()
        },
    );

    assert_eq!(
        qp_multiple(&derived, "a"),
        Some(&vec![
            "5".to_string(),
            "7".to_string(),
            "1".to_string(),
            "2".to_string(),
            "4".to_string()
        ])
    );
    assert_eq!(
        qp_multiple(&derived, "b"),
        Some(&vec!["6".to_string(), "3".to_string()])
    );

    assert_eq!(
        derived.url(),
        "http://test.com/tileset/other_endpoint?a=5&a=7&a=1&a=2&a=4&b=6&b=3"
    );
}

// ── Template values ──────────────────────────────────────────────────────

#[test]
fn replaces_template_values_in_the_url() {
    let resource = Resource::with_options(ResourceOptions {
        url: Some("http://test.com/tileset/{foo}/{bar}".to_string()),
        template_values: Some(map_of(&[("foo", "test1"), ("bar", "test2")])),
        ..Default::default()
    });

    assert_eq!(resource.url(), "http://test.com/tileset/test1/test2");
}

#[test]
fn replaces_numeric_template_values() {
    let resource = Resource::with_options(ResourceOptions {
        url: Some("http://test.com/tileset/{0}/{1}".to_string()),
        template_values: Some(map_of(&[("0", "test1"), ("1", "test2")])),
        ..Default::default()
    });

    assert_eq!(resource.url(), "http://test.com/tileset/test1/test2");
}

#[test]
fn leaves_template_values_unchanged_that_are_not_provided() {
    let resource = Resource::with_options(ResourceOptions {
        url: Some("http://test.com/tileset/{foo}/{bar}".to_string()),
        ..Default::default()
    });

    assert_eq!(resource.url(), "http://test.com/tileset/{foo}/{bar}");
}

#[test]
fn url_encodes_replacement_template_values_in_the_url() {
    let resource = Resource::with_options(ResourceOptions {
        url: Some("http://test.com/tileset/{foo}/{bar}".to_string()),
        template_values: Some(map_of(&[("foo", "a/b"), ("bar", "x$y#")])),
        ..Default::default()
    });

    assert_eq!(resource.url(), "http://test.com/tileset/a%2Fb/x%24y%23");
}

// ── getDerivedResource ───────────────────────────────────────────────────

#[test]
fn get_derived_resource_sets_correct_properties() {
    let proxy = DefaultProxy::new("/proxy/");

    let mut parent = Resource::with_options(ResourceOptions {
        url: Some("http://test.com/tileset?key=value".to_string()),
        query_parameters: Some(map_of(&[("foo", "bar")])),
        template_values: Some(map_of(&[("key5", "value5"), ("key6", "value6")])),
        ..Default::default()
    });
    parent.append_forward_slash();

    let resource = parent.get_derived_resource_with_options(
        cesium_core::resource::DerivedResourceOptions {
            url: Some("tileset.json"),
            query_parameters: Some(&map_of(&[("key1", "value1"), ("key2", "value2")])),
            template_values: Some(&map_of(&[("key3", "value3"), ("key4", "value4")])),
            headers: Some(&map_of(&[("Accept", "application/test-type")])),
            proxy: Some(proxy.clone()),
            retry_attempts: Some(4),
            ..Default::default()
        },
    );

    assert_eq!(
        resource.get_url_component(false, false),
        "http://test.com/tileset/tileset.json"
    );
    let with_query = resource.get_url_component(true, false);
    for pair in ["key1=value1", "key2=value2", "key=value", "foo=bar"] {
        assert!(with_query.contains(pair), "missing {pair} in {with_query}");
    }
    assert_eq!(
        resource.get_url_component(false, true),
        proxy.get_url("http://test.com/tileset/tileset.json")
    );
    assert_eq!(resource.url(), resource.get_url_component(true, true));
    assert_eq!(qp_single(&resource, "foo"), Some("bar"));
    assert_eq!(qp_single(&resource, "key"), Some("value"));
    assert_eq!(qp_single(&resource, "key1"), Some("value1"));
    assert_eq!(qp_single(&resource, "key2"), Some("value2"));
    assert_eq!(resource.template_values().get("key5").map(String::as_str), Some("value5"));
    assert_eq!(resource.template_values().get("key6").map(String::as_str), Some("value6"));
    assert_eq!(resource.template_values().get("key3").map(String::as_str), Some("value3"));
    assert_eq!(resource.template_values().get("key4").map(String::as_str), Some("value4"));
    assert_eq!(resource.headers.get("Accept").map(String::as_str), Some("application/test-type"));
    assert_eq!(resource.proxy(), Some(&proxy));
    assert_eq!(resource.retry_attempts(), 4);
    assert_eq!(resource.retry_count(), 0);
}

#[test]
fn get_derived_resource_works_with_directory_parent_resource() {
    let parent = Resource::with_options(ResourceOptions {
        url: Some("http://test.com/tileset/".to_string()),
        ..Default::default()
    });

    assert_eq!(parent.url(), "http://test.com/tileset/");

    let resource = parent.get_derived_resource_with_options(
        cesium_core::resource::DerivedResourceOptions {
            url: Some("tileset.json"),
            ..Default::default()
        },
    );

    assert_eq!(resource.url(), "http://test.com/tileset/tileset.json");
}

#[test]
fn get_derived_resource_works_with_file_parent_resource() {
    let parent = Resource::with_options(ResourceOptions {
        url: Some("http://test.com/tileset/tileset.json".to_string()),
        ..Default::default()
    });

    assert_eq!(parent.url(), "http://test.com/tileset/tileset.json");

    let resource = parent.get_derived_resource_with_options(
        cesium_core::resource::DerivedResourceOptions {
            url: Some("0/0/0.b3dm"),
            ..Default::default()
        },
    );

    assert_eq!(resource.url(), "http://test.com/tileset/0/0/0.b3dm");
}

#[test]
fn get_derived_resource_works_with_only_template_values() {
    let parent = Resource::with_options(ResourceOptions {
        url: Some("http://test.com/terrain/{z}/{x}/{y}.terrain".to_string()),
        ..Default::default()
    });

    assert_eq!(parent.url(), "http://test.com/terrain/{z}/{x}/{y}.terrain");

    let resource = parent.get_derived_resource_with_options(
        cesium_core::resource::DerivedResourceOptions {
            template_values: Some(&map_of(&[("x", "1"), ("y", "2"), ("z", "0")])),
            ..Default::default()
        },
    );

    assert_eq!(resource.url(), "http://test.com/terrain/0/1/2.terrain");
}

#[test]
fn get_derived_resource_works_with_only_query_parameters() {
    let parent = Resource::with_options(ResourceOptions {
        url: Some("http://test.com/terrain".to_string()),
        ..Default::default()
    });

    assert_eq!(parent.url(), "http://test.com/terrain");

    let resource = parent.get_derived_resource_with_options(
        cesium_core::resource::DerivedResourceOptions {
            query_parameters: Some(&map_of(&[("x", "1"), ("y", "2"), ("z", "0")])),
            ..Default::default()
        },
    );

    // HashMap iteration order is unspecified (JS: "?x=1&y=2&z=0").
    let url = resource.url();
    assert!(url.starts_with("http://test.com/terrain?"));
    for pair in ["x=1", "y=2", "z=0"] {
        assert!(url.contains(pair), "missing {pair} in {url}");
    }
}

// ── setQueryParameters / appendQueryParameters ───────────────────────────

#[test]
fn set_query_parameters_with_use_as_default_set_to_true() {
    let mut resource = Resource::with_options(ResourceOptions {
        url: Some("http://test.com/terrain".to_string()),
        query_parameters: Some(map_of(&[("x", "1"), ("y", "2")])),
        ..Default::default()
    });

    assert_eq!(qp_single(&resource, "x"), Some("1"));
    assert_eq!(qp_single(&resource, "y"), Some("2"));

    resource.set_query_parameters(&map_of(&[("x", "3"), ("y", "4"), ("z", "0")]), true);

    assert_eq!(qp_single(&resource, "x"), Some("1"));
    assert_eq!(qp_single(&resource, "y"), Some("2"));
    assert_eq!(qp_single(&resource, "z"), Some("0"));
}

#[test]
fn set_query_parameters_with_use_as_default_set_to_false() {
    let mut resource = Resource::with_options(ResourceOptions {
        url: Some("http://test.com/terrain".to_string()),
        query_parameters: Some(map_of(&[("x", "1"), ("y", "2")])),
        ..Default::default()
    });

    assert_eq!(qp_single(&resource, "x"), Some("1"));
    assert_eq!(qp_single(&resource, "y"), Some("2"));

    resource.set_query_parameters(&map_of(&[("x", "3"), ("y", "4"), ("z", "0")]), false);

    assert_eq!(qp_single(&resource, "x"), Some("3"));
    assert_eq!(qp_single(&resource, "y"), Some("4"));
    assert_eq!(qp_single(&resource, "z"), Some("0"));
}

#[test]
fn append_query_parameters_works_with_non_arrays() {
    let mut resource = Resource::with_options(ResourceOptions {
        url: Some("http://test.com/terrain".to_string()),
        query_parameters: Some(map_of(&[("x", "1"), ("y", "2")])),
        ..Default::default()
    });

    resource.append_query_parameters(&map_of(&[("x", "3"), ("y", "4"), ("z", "0")]));

    // JS: x -> [3, 1], y -> [4, 2], z -> 0 (new values precede existing).
    assert_eq!(
        qp_multiple(&resource, "x"),
        Some(&vec!["3".to_string(), "1".to_string()])
    );
    assert_eq!(
        qp_multiple(&resource, "y"),
        Some(&vec!["4".to_string(), "2".to_string()])
    );
    assert_eq!(qp_single(&resource, "z"), Some("0"));
}

// DEVIATION: the JS "appendQueryParameters works with arrays/non-arrays"
// case passes array values in the input object; the Rust API takes
// `HashMap<String, String>` so array inputs cannot be expressed. Array
// accumulation itself is covered by the URL-parsed multiple-values tests.

// ── setTemplateValues ────────────────────────────────────────────────────

#[test]
fn set_template_values_with_use_as_default_set_to_true() {
    let mut resource = Resource::with_options(ResourceOptions {
        url: Some("http://test.com/terrain/{z}/{x}/{y}.terrain".to_string()),
        template_values: Some(map_of(&[("x", "1"), ("y", "2"), ("map", "my map")])),
        ..Default::default()
    });

    resource.set_template_values(
        &map_of(&[("x", "3"), ("y", "4"), ("z", "0"), ("style", "my style")]),
        true,
    );

    let template = resource.template_values();
    assert_eq!(template.get("x").map(String::as_str), Some("1"));
    assert_eq!(template.get("y").map(String::as_str), Some("2"));
    assert_eq!(template.get("map").map(String::as_str), Some("my map"));
    assert_eq!(template.get("z").map(String::as_str), Some("0"));
    assert_eq!(template.get("style").map(String::as_str), Some("my style"));
}

#[test]
fn set_template_values_with_use_as_default_set_to_false() {
    let mut resource = Resource::with_options(ResourceOptions {
        url: Some("http://test.com/terrain/{z}/{x}/{y}.terrain".to_string()),
        template_values: Some(map_of(&[("x", "1"), ("y", "2"), ("map", "my map")])),
        ..Default::default()
    });

    resource.set_template_values(
        &map_of(&[("x", "3"), ("y", "4"), ("z", "0"), ("style", "my style")]),
        false,
    );

    let template = resource.template_values();
    assert_eq!(template.get("x").map(String::as_str), Some("3"));
    assert_eq!(template.get("y").map(String::as_str), Some("4"));
    assert_eq!(template.get("map").map(String::as_str), Some("my map"));
    assert_eq!(template.get("z").map(String::as_str), Some("0"));
    assert_eq!(template.get("style").map(String::as_str), Some("my style"));
}

// ── retryOnError ─────────────────────────────────────────────────────────

#[test]
fn retry_on_fail_does_not_exceed_retry_attempts() {
    let calls = Arc::new(AtomicU32::new(0));
    let calls_cb = Arc::clone(&calls);
    let callback: RetryCallback = Box::new(move |_error| {
        calls_cb.fetch_add(1, Ordering::SeqCst);
        true
    });

    let mut resource = Resource::with_options(ResourceOptions {
        url: Some("http://test.com/terrain".to_string()),
        retry_callback: Some(callback),
        retry_attempts: Some(3),
        ..Default::default()
    });

    let error = ResourceError::RequestFailed("test".to_string());
    let mut results = Vec::new();
    for _ in 0..6 {
        results.push(resource.retry_on_error(&error));
    }

    assert_eq!(results, vec![true, true, true, false, false, false]);
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(resource.retry_count(), 3);
}

#[test]
fn retry_on_fail_returns_value_from_callback() {
    let result = Arc::new(AtomicBool::new(true));
    let calls = Arc::new(AtomicU32::new(0));
    let result_cb = Arc::clone(&result);
    let calls_cb = Arc::clone(&calls);
    let callback: RetryCallback = Box::new(move |_error| {
        calls_cb.fetch_add(1, Ordering::SeqCst);
        let next = !result_cb.load(Ordering::SeqCst);
        result_cb.store(next, Ordering::SeqCst);
        next
    });

    let mut resource = Resource::with_options(ResourceOptions {
        url: Some("http://test.com/terrain".to_string()),
        retry_callback: Some(callback),
        retry_attempts: Some(4),
        ..Default::default()
    });

    let error = ResourceError::RequestFailed("test".to_string());
    let mut results = Vec::new();
    for _ in 0..6 {
        results.push(resource.retry_on_error(&error));
    }

    assert_eq!(results, vec![false, true, false, true, false, false]);
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    assert_eq!(resource.retry_count(), 4);
}

// ── isDataUri / isBlobUri ────────────────────────────────────────────────

#[test]
fn is_data_uri_returns_correct_values() {
    let data_resource = Resource::with_options(ResourceOptions {
        url: Some("data:text/plain;base64,SGVsbG8sIFdvcmxkIQ%3D%3".to_string()),
        ..Default::default()
    });
    assert!(data_resource.is_data_uri());

    let resource = Resource::with_options(ResourceOptions {
        url: Some("http://invalid.uri/tileset".to_string()),
        ..Default::default()
    });
    assert!(!resource.is_data_uri());
}

#[test]
fn is_blob_uri_returns_correct_values() {
    let blob_resource = Resource::with_options(ResourceOptions {
        url: Some("blob:d3958f5c-0777-0845-9dcf-2cb28783acaf".to_string()),
        ..Default::default()
    });
    assert!(blob_resource.is_blob_uri());

    let resource = Resource::with_options(ResourceOptions {
        url: Some("http://invalid.uri/tileset".to_string()),
        ..Default::default()
    });
    assert!(!resource.is_blob_uri());
}

// ── fetch (mock backend mirrors the loadWithXhr spy) ─────────────────────

#[tokio::test]
async fn fetch_calls_correct_method() {
    let expected_url = "http://test.com/endpoint";

    let mut backend = MockResourceBackend::new();
    backend.register_json_response(expected_url, "{\"status\":\"success\"}");

    let mut resource = Resource::new(expected_url.to_string());
    let result = resource
        .fetch(
            &backend,
            Some(FetchParams {
                response_type: Some(ResponseType::Json),
                ..Default::default()
            }),
        )
        .await
        .expect("fetch");

    match result {
        Response::Json(value) => assert_eq!(value["status"], "success"),
        other => panic!("expected Json response, got {other:?}"),
    }
}

// DEVIATION: the JS post/put/patch/delete/head/options "calls with correct
// method" cases assert the HTTP verb passed to loadWithXhr. The Rust
// `ResourceBackend` trait abstracts over transport with fetch_bytes/fetch_text
// only (no verb parameter), so verb capture is not expressible; the fetch
// plumbing (url/headers combine, response mapping) is covered above.

// ── data URI loading (mirrors the "data URI loading" describe block) ─────

async fn fetch_data_uri(url: &str, response_type: Option<ResponseType>) -> Response {
    let backend = MockResourceBackend::new();
    let mut resource = Resource::new(url.to_string());
    resource
        .fetch(
            &backend,
            Some(FetchParams {
                response_type,
                ..Default::default()
            }),
        )
        .await
        .expect("data uri fetch")
}

fn expect_text(response: Response, expected: &str) {
    match response {
        Response::Text(text) => assert_eq!(text, expected),
        other => panic!("expected Text response, got {other:?}"),
    }
}

#[tokio::test]
async fn can_load_uri_escaped_text_with_default_response_type() {
    let result = fetch_data_uri("data:,Hello%2C%20World!", None).await;
    expect_text(result, "Hello, World!");
}

#[tokio::test]
async fn can_load_uri_escaped_text_with_response_type_text() {
    let result = fetch_data_uri("data:,Hello%2C%20World!", Some(ResponseType::Text)).await;
    expect_text(result, "Hello, World!");
}

#[tokio::test]
async fn can_load_base64_encoded_text_with_default_response_type() {
    let result = fetch_data_uri("data:text/plain;base64,SGVsbG8sIFdvcmxkIQ==", None).await;
    expect_text(result, "Hello, World!");
}

#[tokio::test]
async fn can_load_base64_encoded_text_with_response_type_text() {
    let result = fetch_data_uri(
        "data:text/plain;base64,SGVsbG8sIFdvcmxkIQ==",
        Some(ResponseType::Text),
    )
    .await;
    expect_text(result, "Hello, World!");
}

#[tokio::test]
async fn can_load_base64_and_uri_encoded_text_with_default_response_type() {
    let result = fetch_data_uri("data:text/plain;base64,SGVsbG8sIFdvcmxkIQ%3D%3D", None).await;
    expect_text(result, "Hello, World!");
}

#[tokio::test]
async fn can_load_base64_and_uri_encoded_text_with_response_type_text() {
    let result = fetch_data_uri(
        "data:text/plain;base64,SGVsbG8sIFdvcmxkIQ%3D%3D",
        Some(ResponseType::Text),
    )
    .await;
    expect_text(result, "Hello, World!");
}

#[tokio::test]
async fn can_load_uri_escaped_html_as_text_with_default_response_type() {
    let result = fetch_data_uri("data:text/html,%3Ch1%3EHello%2C%20World!%3C%2Fh1%3E", None).await;
    expect_text(result, "<h1>Hello, World!</h1>");
}

#[tokio::test]
async fn can_load_uri_escaped_html_as_text_with_response_type_text() {
    let result = fetch_data_uri(
        "data:text/html,%3Ch1%3EHello%2C%20World!%3C%2Fh1%3E",
        Some(ResponseType::Text),
    )
    .await;
    expect_text(result, "<h1>Hello, World!</h1>");
}

#[tokio::test]
async fn can_load_uri_escaped_text_as_json() {
    let result = fetch_data_uri(
        "data:application/json,%7B%22key%22%3A%22value%22%7D",
        Some(ResponseType::Json),
    )
    .await;
    match result {
        Response::Json(value) => assert_eq!(value["key"], "value"),
        other => panic!("expected Json response, got {other:?}"),
    }
}

#[tokio::test]
async fn can_load_base64_encoded_text_as_json() {
    let result = fetch_data_uri(
        "data:application/json;base64,eyJrZXkiOiJ2YWx1ZSJ9",
        Some(ResponseType::Json),
    )
    .await;
    match result {
        Response::Json(value) => assert_eq!(value["key"], "value"),
        other => panic!("expected Json response, got {other:?}"),
    }
}

// Mirrors the JS `tile` fixture ("can load Base64 encoded data as
// arraybuffer": byteLength 3914).
const TILE_DATA_URI: &str = "data:;base64,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DwABABIAAgAAAAIAEwACAAEABgADAAIAEwAAABMAFAACAAEAAgAAAAAAAgAEAAYAAwAEAAEACAACAAYACAAHAAAACQAAAAQAAgABAAoAAAAKAAAACwAOAAAABQAMAAMAAgAMAAEADwAQAAAAAgAQAAEAAAACABIAEgAUAAEAAAADAAIAAAAEAAIABAABAAUABQABAAYAAAAIAAcABwACAAEACAAAAAAAAwACAAoAAQAMAAoAAAAMAA0AAQANAAIAAAANAAAAAAASAA8AAwABAA8ADwAEAAIAEQASAAEAAAASAAAAEwACABUAAwABABMAAAAEAAYAAQACAAQAAAAAAAgACAABAAMABwACAAgAAgAHAAAAAQAIAAoACgAAAAIADAABAAsADQABAAwADQAOAAAAAgAOAAAAAQAPAAAAAwABABAAEgAAABIABAASAAEAEwAAAAIAFQABABQAAQAAAAMAAwAAAAcABAACAAEAAQAFAAcABQAAAAAABwAAAAkAAQAIAAIAAwAIAAAAAQAJAAUAAgAAAAsACwABAAAADAABAA4ADwAOAAEAAAASABAAAgABABAAAQARABIAAAASAAIAEgABAAAAFQATAAEAAAAEAAAABgAHAAUAAQAFAAcABQACAAQABwAAAAIACAAKAAEACgAAAAIAAQALAAAAAQAMAA0ADgABAA0AAAACAA8AAQAPAAAADQABABAAEQASAA0AEgABAA0AAQASAAAAAgABAAAAAQAEAAMAAAAFAAAAAAAHAAMAAQAIAAcAAgAHAAQACAAAAAoAAQAJAAAACgADAAEACwAAAA0ADAADAAAAAQACAA0ADwAOAAIAAgAQAA8AAgAAABEAEwARAAAAAQASAAIAAQAAABUAFAAVAAEAAQAAABUAFQABACkAAgAAAAAAAgAEAAUAAwAEAAAAAQAFAAIABwAAAAcABwABAAQAAAAAAAoAAwAKAAEACgAMAAIAAAANAAwADQABAAMAAAACAA0ADQAQAAEAAAAAAAQABgAEAAEAAwACAAQAAQAFAAYABQAAAAAABwADAAIABwAAAAoAAQAIAAIAAAAOAAsACwACAAEADQAOAAEAAAAAAA8ADwABAA4AAgAPAAAAAQAQAAQAAAATABAAEAADAAEAPAA7ABMAAQA8ABMAPAABAAAAAQACAAAAAwAFAAEABQAGAAAAAAAHAAIAAwAHAAEACAAAAAAACAACAAoAAQAEAAoACAAMAAAAAQADAAkADAANAAkADgABAA0ADwAAAA8AAgAPAAAAAQAQAAIAAAASAAAAEwAUAAEABAATAAIAFAAVAAEAAQAVAAAAAwACAAEAAQAAAAQAAQAGAAQAAAAHAAAACAACAAcAAQAIAAMAAAAKAAgACAADAAEADAAKAAEADAAAAAwAAgAAAA4AAgAOAAEAAAARAA4ADgADAAEAAAAAABMAAwACABMAEwABABIAAAAVABMAAQATAAIAAgAAAAAABAABAAMABQACAAQABgAIAAUAAAADAAYABgAJAAEAAAACAAoACQABAAoAAAAKAAAAAwALAAAAAQAMAAMAAgAMAA4ADgAQAAIAAAADABEAEQAQAAEAAAAAABIAFAACABIAAwASAAEAFAAVAAIAAgAAAAIAAwAWAAAAAAADAAUAAgABAAUAAAAFAAQABQABAAcAAAAKAAgACAACAAEACgABAAsACwABAAAACgAMAAEACgAAAAAAAQAPAAwAAwACAAwAAAAQAAIAEAABABEAAQATABEAAQAAAAAAAAAEAAIAAAAXAAUABQACAAEABAAFAAYABgAHAAQAAAAIAAkACQAAAAIACwABAAoACwAMAAEAAQAMAAAAEAABAA0AEAAOAAAAAgARAAEAAAAQABEAAQARAAAAAgABAAAAAgATAAAAAwAAAAMABAAAAAAABgAEAAIAAQADAAYABABCAAAAQwAFAAAABAAGAAIAAQAGABkABAAAAAQAVQAFAAMABQBVAAEAAAAHAAUABQACAAEABwABAAAAAgADAFoAAgAAAAIAAQADAFsAWwAEAFkAWQAEAFcABABYAFcAWAAGAEkABgBIAEkAMwBIAAUAHQAzAAUAWwBaAAEAAQBaAAAAAABcAF4AXAABAAIAAAACAGAAXwBgAAIAAAAAAAMAAgADAGIAYgB0AAIAAAACAAMAdQABAAMAAgABAAAAAgAAAAIAAwB3AHgAAQADAAAAAQAEAHkAAAADAAIAAwABAAAAewCPAAMAAwCPAAIAAAADAAAAkQCSAAQABAACAAMAAQAEAJIAAQAAAAMAkwCmAAIApgABAAIApgClAAEAIQAAACEAIQABAAAAAQA4ACIAAAACAAMAOQACADgAOAACAAAAAAACAAQABAADAAEAAwAAAAIAAAAEAAMAAwACAAEABAABAAAAPQAFAD8AAQAAAAYAQAAGAAEAAAAEAAAABAAFAAAABgAHAAIAAQAGAAMABwAAAAMAAQAEAAMAvQC+AAQABAABAL0ABAC+AAIAvgAGAAIAAAAGAAcAAQAHALsAuwAHAL8ABgABAAAABwBaAEYAWgAHAAEAAQACAAAAvgADAL0AAQADAL4AAgBcAFsAAQCZAAIAAgCZAFwArACZAAEAvgCsAAEAXACZAJgAwAAAAMMAwQAFAAAAAgDCAAEA2AACANYAxAACANgAEQAAAAEAnwCoANUA2QAFANoAzgDLAMkApwCmAKMAqgCsAK0AqwACAAAAAQAEABEAAAAEAK4ADAAkAMYAxQDIAMIAwQC/ALsAugC5ALIAsAC8AL0AAgAAAJ8ArgA=";

#[tokio::test]
async fn can_load_base64_encoded_data_as_arraybuffer() {
    let result = fetch_data_uri(TILE_DATA_URI, Some(ResponseType::Arraybuffer)).await;
    match result {
        Response::Bytes(bytes) => assert_eq!(bytes.len(), 3914),
        other => panic!("expected Bytes response, got {other:?}"),
    }
}

// DEVIATION: the JS "can load Base64 encoded data as blob" case asserts the
// blob mime type and round-trips through URL.createObjectURL/fetchImage
// (browser-only); the Rust port returns raw bytes without a mime wrapper.
