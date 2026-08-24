//! Tests for Resource: URL manipulation, query parameters, headers,
//! derived resources, retry, proxy, ion endpoint, error display,
//! and MockResourceBackend.

use cesium_core::default_proxy::DefaultProxy;
use cesium_core::resource::{
    MockResourceBackend, Resource, ResourceBackend, ResourceError, RequestOptions, ResponseType,
};
use std::collections::HashMap;

// --- Resource construction ---
#[test]
fn resource_new_url() {
    let r = Resource::new("https://example.com/data".to_string());
    assert_eq!(r.url(), "https://example.com/data");
}

#[test]
fn resource_new_default_retry() {
    let r = Resource::new("https://example.com".to_string());
    // Aligned with CesiumJS: retryAttempts defaults to 0.
    assert_eq!(r.retry_attempts(), 0);
    assert_eq!(r.retry_count(), 0);
}

#[test]
fn resource_default_empty_url() {
    let r = Resource::default();
    assert_eq!(r.url(), "");
}

#[test]
fn resource_from_url_with_params() {
    let mut params = HashMap::new();
    params.insert("key".to_string(), "value".to_string());
    let r = Resource::from_url_with_params("https://example.com".to_string(), params);
    assert_eq!(r.get_query_parameter("key"), Some("value"));
}

// --- URL manipulation ---
#[test]
fn resource_set_url() {
    let mut r = Resource::new("https://old.com".to_string());
    r.set_url("https://new.com".to_string());
    assert_eq!(r.url(), "https://new.com");
}

#[test]
fn resource_append_forward_slash_adds() {
    let mut r = Resource::new("https://example.com/path".to_string());
    r.append_forward_slash();
    assert_eq!(r.url(), "https://example.com/path/");
}

#[test]
fn resource_append_forward_slash_idempotent() {
    let mut r = Resource::new("https://example.com/path/".to_string());
    r.append_forward_slash();
    assert_eq!(r.url(), "https://example.com/path/");
}

#[test]
fn resource_get_derived_resource() {
    let r = Resource::new("https://example.com/base".to_string());
    let derived = r.get_derived_resource("child/file.json");
    assert_eq!(derived.url(), "https://example.com/base/child/file.json");
}

#[test]
fn resource_get_derived_resource_with_existing_slash() {
    let r = Resource::new("https://example.com/base/".to_string());
    let derived = r.get_derived_resource("child.json");
    assert_eq!(derived.url(), "https://example.com/base/child.json");
}

#[test]
fn resource_get_derived_resource_preserves_headers() {
    let mut r = Resource::new("https://example.com".to_string());
    r.set_header("X-Test".to_string(), "yes".to_string());
    let derived = r.get_derived_resource("path");
    assert_eq!(derived.get_header("X-Test"), Some("yes"));
}

#[test]
fn resource_clone_resource() {
    let mut r = Resource::new("https://example.com".to_string());
    r.set_query_parameter("a".to_string(), "1".to_string());
    r.set_header("X-Key".to_string(), "val".to_string());
    r.set_retry_attempts(5);
    let cloned = r.clone_resource();
    // Mirrors Resource.js: the `url` property is `getUrlComponent(true, true)`,
    // so the query string is included; the raw stored url excludes it.
    assert_eq!(cloned.url(), "https://example.com?a=1");
    assert_eq!(cloned.raw_url(), "https://example.com");
    assert_eq!(cloned.get_query_parameter("a"), Some("1"));
    assert_eq!(cloned.get_header("X-Key"), Some("val"));
    assert_eq!(cloned.retry_attempts(), 5);
}

// --- Query parameters ---
#[test]
fn resource_query_params_empty() {
    let r = Resource::new("https://example.com".to_string());
    assert_eq!(r.get_url_with_query_parameters(), "https://example.com");
}

#[test]
fn resource_set_and_get_query_parameter() {
    let mut r = Resource::new("https://example.com".to_string());
    r.set_query_parameter("token".to_string(), "abc".to_string());
    assert_eq!(r.get_query_parameter("token"), Some("abc"));
    assert_eq!(r.get_query_parameter("missing"), None);
}

#[test]
fn resource_query_url_contains_separator() {
    let mut r = Resource::new("https://example.com".to_string());
    r.set_query_parameter("k".to_string(), "v".to_string());
    let url = r.get_url_with_query_parameters();
    assert!(url.contains('?'));
    assert!(url.contains("k=v"));
}

#[test]
fn resource_add_query_parameters() {
    let mut r = Resource::new("https://example.com".to_string());
    let mut params = HashMap::new();
    params.insert("a".to_string(), "1".to_string());
    params.insert("b".to_string(), "2".to_string());
    r.add_query_parameters(&params);
    assert_eq!(r.get_query_parameter("a"), Some("1"));
    assert_eq!(r.get_query_parameter("b"), Some("2"));
}

// --- Headers ---
#[test]
fn resource_set_has_get_delete_header() {
    let mut r = Resource::new("https://example.com".to_string());
    assert!(!r.has_header("Authorization"));
    r.set_header("Authorization".to_string(), "Bearer tok".to_string());
    assert!(r.has_header("Authorization"));
    assert_eq!(r.get_header("Authorization"), Some("Bearer tok"));
    r.delete_header("Authorization");
    assert!(!r.has_header("Authorization"));
    assert_eq!(r.get_header("Authorization"), None);
}

// --- Retry ---
#[test]
fn resource_set_retry_attempts() {
    let mut r = Resource::new("https://example.com".to_string());
    r.set_retry_attempts(10);
    assert_eq!(r.retry_attempts(), 10);
}

// --- Proxy ---
#[test]
fn resource_proxy_default_none() {
    let r = Resource::new("https://example.com".to_string());
    assert_eq!(r.proxy(), None);
}

#[test]
fn resource_set_proxy() {
    let mut r = Resource::new("https://example.com".to_string());
    r.set_proxy(DefaultProxy::new("http://proxy.com"));
    assert_eq!(r.proxy().map(|p| p.proxy.as_str()), Some("http://proxy.com"));
}

// --- Ion endpoint ---
#[test]
fn resource_from_ion_asset_id() {
    let r = Resource::from_ion_asset_id(12345, "my_token");
    assert!(r.url().contains("api.cesium.com"));
    assert!(r.url().contains("12345"));
    assert!(r.url().contains("my_token"));
    assert_eq!(r.get_header("Authorization"), Some("Bearer my_token"));
}

// --- RequestOptions ---
#[test]
fn request_options_default() {
    let opts = RequestOptions::default();
    assert_eq!(opts.method, "GET");
    assert!(opts.data.is_none());
    assert!(opts.content_type.is_none());
    assert_eq!(opts.response_type, ResponseType::Json);
}

// --- ResponseType ---
#[test]
fn response_type_variants() {
    assert_eq!(ResponseType::Arraybuffer, ResponseType::Arraybuffer);
    assert_ne!(ResponseType::Json, ResponseType::Text);
}

// --- ResourceError Display ---
#[test]
fn resource_error_request_failed_display() {
    let e = ResourceError::RequestFailed("connection refused".to_string());
    let msg = format!("{e}");
    assert!(msg.contains("connection refused"));
}

#[test]
fn resource_error_http_error_display() {
    let e = ResourceError::HttpError {
        status: 404,
        message: "not found".to_string(),
    };
    let msg = format!("{e}");
    assert!(msg.contains("404"));
    assert!(msg.contains("not found"));
}

#[test]
fn resource_error_json_parse_display() {
    let e = ResourceError::JsonParseError("invalid json".to_string());
    let msg = format!("{e}");
    assert!(msg.contains("invalid json"));
}

#[test]
fn resource_error_retry_exceeded_display() {
    let e = ResourceError::RetryExceeded { attempts: 3 };
    let msg = format!("{e}");
    assert!(msg.contains("3"));
}

#[test]
fn resource_error_invalid_url_display() {
    let e = ResourceError::InvalidUrl("bad url".to_string());
    let msg = format!("{e}");
    assert!(msg.contains("bad url"));
}

#[test]
fn resource_error_is_error_trait() {
    let e: Box<dyn std::error::Error> =
        Box::new(ResourceError::RequestFailed("test".to_string()));
    assert!(e.source().is_none() || e.source().is_some());
}

// --- MockResourceBackend ---
#[test]
fn mock_backend_default() {
    let _mock = MockResourceBackend::default();
}

#[tokio::test]
async fn mock_backend_register_and_fetch() {
    let mut mock = MockResourceBackend::new();
    mock.register_response("https://example.com/data", b"hello".to_vec());
    let text = mock
        .fetch_text("https://example.com/data", &HashMap::new())
        .await
        .unwrap();
    assert_eq!(text, "hello");
}

#[tokio::test]
async fn mock_backend_unknown_url_errors() {
    let mock = MockResourceBackend::new();
    let result = mock.fetch_bytes("https://unknown.com", &HashMap::new()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn mock_backend_register_json_response() {
    let mut mock = MockResourceBackend::new();
    mock.register_json_response("https://api.com/data", r#"{"key":"val"}"#);
    let text = mock
        .fetch_text("https://api.com/data", &HashMap::new())
        .await
        .unwrap();
    assert_eq!(text, r#"{"key":"val"}"#);
}
