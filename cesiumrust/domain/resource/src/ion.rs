//! Cesium Ion asset endpoint URL construction.
//!
//! Maps to CesiumJS `Core/IonResource.js` + `Core/Ion.js`:
//! - `IonResource.fromAssetId(assetId, options)` — builds the endpoint URL.
//! - `IonResource._createEndpointResource(assetId, options)` — constructs the
//!   Resource used to fetch the ion endpoint JSON.
//! - Token injection via `access_token` query parameter and/or
//!   `Authorization: Bearer <token>` header.
//!
//! **This module is pure URL/header construction — it does NOT make network
//! requests.** The actual HTTP fetch is the caller's responsibility (via the
//! adapter layer).
//!
//! # Cesium Ion REST API
//!
//! The ion endpoint for an asset is:
//! ```text
//! GET {server}/v1/assets/{assetId}/endpoint?access_token={token}
//! ```
//!
//! The response contains:
//! - `url` — the actual asset content URL
//! - `accessToken` — a short-lived token for the content URL
//! - `externalType` — if this is an external asset ("3DTILES", "STK_TERRAIN_SERVER", etc.)
//! - `options.url` — for external assets, the URL of the external resource
//! - `attributions` — credit information

use std::collections::HashMap;

/// Default Cesium Ion API server URL.
///
/// Maps to CesiumJS `Ion.defaultServer` = `"https://api.cesium.com/"`.
pub const DEFAULT_ION_SERVER: &str = "https://api.cesium.com/";

/// Options for constructing an Ion asset endpoint resource.
///
/// Maps to CesiumJS `IonResource.fromAssetId(assetId, options)`.
#[derive(Debug, Clone, Default)]
pub struct IonAssetOptions {
    /// The access token to use. If None, no token is injected.
    ///
    /// Maps to `options.accessToken` (falls back to `Ion.defaultAccessToken`).
    pub access_token: Option<String>,

    /// The url of the Cesium ion API server.
    ///
    /// Maps to `options.server` (falls back to `Ion.defaultServer`).
    pub server: Option<String>,

    /// Additional query parameters for the endpoint request.
    ///
    /// Maps to `options.queryParameters` (merged into the endpoint URL).
    pub query_parameters: Option<HashMap<String, String>>,

    /// Whether to inject the `Authorization: Bearer` header in addition to
    /// (or instead of) the `access_token` query parameter.
    ///
    /// CesiumJS uses both: the query parameter for the endpoint request and
    /// the Bearer header for subsequent content requests. Default: `true`.
    pub use_bearer_header: Option<bool>,
}

/// The asset endpoint data returned from the Cesium Ion endpoint service.
///
/// This is the **parsed response** structure. Construction from JSON is the
/// caller's responsibility (the domain layer does not depend on serde_json
/// for this; the adapter or application layer deserializes).
///
/// Maps to the ion endpoint JSON response fields used by `IonResource`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IonEndpoint {
    /// The URL of the asset content (tiles, terrain, imagery).
    pub url: String,

    /// The external asset type (`"3DTILES"`, `"STK_TERRAIN_SERVER"`, etc.),
    /// if this is an external asset. `None` for native ion assets.
    pub external_type: Option<String>,

    /// A short-lived access token for requests against the content URL.
    ///
    /// Maps to `endpoint.accessToken`.
    pub access_token: Option<String>,

    /// For external assets: `endpoint.options.url` (the actual external URL).
    pub options_url: Option<String>,

    /// Attribution/credit HTML strings.
    ///
    /// Maps to `endpoint.attributions`.
    pub attributions: Vec<IonAttribution>,
}

/// A single attribution entry from the ion endpoint response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IonAttribution {
    /// HTML content of the attribution.
    pub html: String,
    /// Whether this attribution is collapsible.
    pub collapsible: bool,
}

/// Describes a fully-constructed Ion resource request (URL + headers).
///
/// This is the output of the pure URL-building logic — it tells the caller
/// *what* to request without actually requesting it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IonResourceRequest {
    /// The fully-constructed endpoint URL (with query parameters).
    pub url: String,

    /// HTTP headers to send with the request (may include Authorization).
    pub headers: HashMap<String, String>,

    /// The asset ID this request is for.
    pub asset_id: u64,

    /// The ion server base URL used.
    pub server: String,
}

/// Builds the Ion endpoint URL for a given asset ID.
///
/// Maps to `IonResource._createEndpointResource(assetId, options)`.
///
/// This function is **pure** — it constructs the URL and headers without
/// making any network request. The caller (adapter layer) is responsible for
/// actually fetching the endpoint.
///
/// # Examples
/// ```
/// use cesium_resource::ion::{build_endpoint_request, IonAssetOptions};
///
/// let req = build_endpoint_request(1234, IonAssetOptions {
///     access_token: Some("my-token".to_string()),
///     ..Default::default()
/// });
/// assert!(req.url.contains("v1/assets/1234/endpoint"));
/// assert!(req.url.contains("access_token=my-token"));
/// assert_eq!(req.headers.get("Authorization").unwrap(), "Bearer my-token");
/// ```
pub fn build_endpoint_request(asset_id: u64, options: IonAssetOptions) -> IonResourceRequest {
    let server = options
        .server
        .unwrap_or_else(|| DEFAULT_ION_SERVER.to_string());

    // Normalize server URL: ensure trailing slash for path joining.
    let server_base = if server.ends_with('/') {
        server.clone()
    } else {
        format!("{}/", server)
    };

    // Build the endpoint path: v1/assets/{assetId}/endpoint
    let endpoint_path = format!("v1/assets/{}/endpoint", asset_id);

    // Build query parameters.
    let mut query_parts: Vec<String> = Vec::new();

    if let Some(ref token) = options.access_token {
        if !token.is_empty() {
            query_parts.push(format!("access_token={}", percent_encode_value(token)));
        }
    }

    // Merge additional query parameters.
    if let Some(ref extra) = options.query_parameters {
        let mut sorted_keys: Vec<&String> = extra.keys().collect();
        sorted_keys.sort();
        for key in sorted_keys {
            if let Some(value) = extra.get(key) {
                query_parts.push(format!(
                    "{}={}",
                    percent_encode_value(key),
                    percent_encode_value(value)
                ));
            }
        }
    }

    let url = if query_parts.is_empty() {
        format!("{}{}", server_base, endpoint_path)
    } else {
        format!(
            "{}{}?{}",
            server_base,
            endpoint_path,
            query_parts.join("&")
        )
    };

    // Build headers.
    let mut headers = HashMap::new();

    // CesiumJS client identification header.
    // Maps to `addClientHeaders(headers)` in IonResource.js.
    headers.insert(
        "X-Cesium-Client".to_string(),
        "cesium-rust".to_string(),
    );

    // Authorization: Bearer header (if configured and token available).
    let use_bearer = options.use_bearer_header.unwrap_or(true);
    if use_bearer {
        if let Some(ref token) = options.access_token {
            if !token.is_empty() {
                headers.insert(
                    "Authorization".to_string(),
                    format!("Bearer {}", token),
                );
            }
        }
    }

    IonResourceRequest {
        url,
        headers,
        asset_id,
        server: server_base,
    }
}

/// Builds the content URL for an Ion asset given the endpoint response.
///
/// After the endpoint is fetched, the content URL needs the endpoint's
/// `accessToken` appended as a query parameter (CesiumJS does this in
/// `IonResource.fromEndpoint`).
///
/// Maps to the token injection in `new IonResource(endpoint, endpointResource)`:
/// ```js
/// resource = new Resource({ url: endpoint.url });
/// resource.setQueryParameters({ access_token: endpoint.accessToken });
/// ```
pub fn build_content_url(endpoint: &IonEndpoint) -> String {
    let base_url = &endpoint.url;

    match &endpoint.access_token {
        Some(token) if !token.is_empty() => {
            let separator = if base_url.contains('?') { '&' } else { '?' };
            format!(
                "{}{}access_token={}",
                base_url,
                separator,
                percent_encode_value(token)
            )
        }
        _ => base_url.clone(),
    }
}

/// Builds headers for content requests to an Ion asset.
///
/// Includes the Bearer token from the endpoint response (if available) and
/// the standard Cesium client identification header.
pub fn build_content_headers(endpoint: &IonEndpoint) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    headers.insert(
        "X-Cesium-Client".to_string(),
        "cesium-rust".to_string(),
    );

    if let Some(ref token) = endpoint.access_token {
        if !token.is_empty() {
            headers.insert(
                "Authorization".to_string(),
                format!("Bearer {}", token),
            );
        }
    }

    headers
}

/// Determines whether an Ion endpoint represents an external asset.
///
/// Maps to `IonResource._isExternal` logic: an asset is external when
/// `endpoint.externalType` is defined.
pub fn is_external_asset(endpoint: &IonEndpoint) -> bool {
    endpoint.external_type.is_some()
}

/// For external assets, returns the effective content URL.
///
/// Maps to `IonResource.fromEndpoint` where external assets use
/// `endpoint.options.url` instead of `endpoint.url`.
///
/// Returns `None` for non-external assets or when the options URL is missing.
pub fn external_asset_url(endpoint: &IonEndpoint) -> Option<&str> {
    if !is_external_asset(endpoint) {
        return None;
    }
    endpoint.options_url.as_deref()
}

/// Checks if an external asset type is supported as a Resource.
///
/// Maps to the CesiumJS guard:
/// ```js
/// if (externalType !== '3DTILES' && externalType !== 'STK_TERRAIN_SERVER') {
///   throw new RuntimeError('Ion.createResource does not support external imagery assets...');
/// }
/// ```
pub fn is_supported_external_type(endpoint: &IonEndpoint) -> bool {
    match endpoint.external_type.as_deref() {
        Some("3DTILES") | Some("STK_TERRAIN_SERVER") => true,
        Some(_) => false, // e.g. "IMAGERY" — not supported as Resource
        None => true,     // Non-external assets are always supported
    }
}

/// Percent-encodes a query parameter value.
///
/// Encodes all characters except unreserved RFC 3986 chars
/// (`A-Z a-z 0-9 - _ . ~`).
fn percent_encode_value(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_endpoint_request_default_server() {
        let req = build_endpoint_request(
            1234,
            IonAssetOptions {
                access_token: Some("test-token".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(
            req.url,
            "https://api.cesium.com/v1/assets/1234/endpoint?access_token=test-token"
        );
        assert_eq!(req.asset_id, 1234);
        assert_eq!(
            req.headers.get("Authorization").unwrap(),
            "Bearer test-token"
        );
        assert_eq!(
            req.headers.get("X-Cesium-Client").unwrap(),
            "cesium-rust"
        );
    }

    #[test]
    fn build_endpoint_request_custom_server() {
        let req = build_endpoint_request(
            42,
            IonAssetOptions {
                server: Some("https://custom-ion.example.com".to_string()),
                access_token: Some("tok".to_string()),
                ..Default::default()
            },
        );
        assert!(req.url.starts_with("https://custom-ion.example.com/v1/assets/42/endpoint"));
    }

    #[test]
    fn build_endpoint_request_no_token() {
        let req = build_endpoint_request(99, IonAssetOptions::default());
        assert_eq!(req.url, "https://api.cesium.com/v1/assets/99/endpoint");
        assert!(!req.headers.contains_key("Authorization"));
    }

    #[test]
    fn build_endpoint_request_bearer_disabled() {
        let req = build_endpoint_request(
            1,
            IonAssetOptions {
                access_token: Some("tok".to_string()),
                use_bearer_header: Some(false),
                ..Default::default()
            },
        );
        // Token still in query param but no Authorization header
        assert!(req.url.contains("access_token=tok"));
        assert!(!req.headers.contains_key("Authorization"));
    }

    #[test]
    fn build_endpoint_request_extra_query_params() {
        let mut extra = HashMap::new();
        extra.insert("foo".to_string(), "bar".to_string());
        extra.insert("baz".to_string(), "qux".to_string());

        let req = build_endpoint_request(
            5,
            IonAssetOptions {
                access_token: Some("t".to_string()),
                query_parameters: Some(extra),
                ..Default::default()
            },
        );
        // Query params are sorted
        assert!(req.url.contains("access_token=t"));
        assert!(req.url.contains("baz=qux"));
        assert!(req.url.contains("foo=bar"));
    }

    #[test]
    fn build_content_url_with_token() {
        let endpoint = IonEndpoint {
            url: "https://assets.ion.cesium.com/us-east-1/1234/tileset.json".to_string(),
            external_type: None,
            access_token: Some("short-lived-token".to_string()),
            options_url: None,
            attributions: Vec::new(),
        };
        let url = build_content_url(&endpoint);
        assert_eq!(
            url,
            "https://assets.ion.cesium.com/us-east-1/1234/tileset.json?access_token=short-lived-token"
        );
    }

    #[test]
    fn build_content_url_existing_query() {
        let endpoint = IonEndpoint {
            url: "https://example.com/tile?v=2".to_string(),
            external_type: None,
            access_token: Some("tok".to_string()),
            options_url: None,
            attributions: Vec::new(),
        };
        assert_eq!(
            build_content_url(&endpoint),
            "https://example.com/tile?v=2&access_token=tok"
        );
    }

    #[test]
    fn build_content_url_no_token() {
        let endpoint = IonEndpoint {
            url: "https://example.com/data".to_string(),
            external_type: None,
            access_token: None,
            options_url: None,
            attributions: Vec::new(),
        };
        assert_eq!(build_content_url(&endpoint), "https://example.com/data");
    }

    #[test]
    fn build_content_headers_includes_bearer() {
        let endpoint = IonEndpoint {
            url: String::new(),
            external_type: None,
            access_token: Some("tok123".to_string()),
            options_url: None,
            attributions: Vec::new(),
        };
        let headers = build_content_headers(&endpoint);
        assert_eq!(headers.get("Authorization").unwrap(), "Bearer tok123");
        assert_eq!(headers.get("X-Cesium-Client").unwrap(), "cesium-rust");
    }

    #[test]
    fn external_asset_detection() {
        let native = IonEndpoint {
            url: "https://example.com".to_string(),
            external_type: None,
            access_token: None,
            options_url: None,
            attributions: Vec::new(),
        };
        assert!(!is_external_asset(&native));
        assert_eq!(external_asset_url(&native), None);
        assert!(is_supported_external_type(&native));

        let external_3dtiles = IonEndpoint {
            url: String::new(),
            external_type: Some("3DTILES".to_string()),
            access_token: None,
            options_url: Some("https://external.com/tileset.json".to_string()),
            attributions: Vec::new(),
        };
        assert!(is_external_asset(&external_3dtiles));
        assert_eq!(
            external_asset_url(&external_3dtiles),
            Some("https://external.com/tileset.json")
        );
        assert!(is_supported_external_type(&external_3dtiles));

        let external_imagery = IonEndpoint {
            url: String::new(),
            external_type: Some("IMAGERY".to_string()),
            access_token: None,
            options_url: Some("https://imagery.com".to_string()),
            attributions: Vec::new(),
        };
        assert!(is_external_asset(&external_imagery));
        assert!(!is_supported_external_type(&external_imagery));
    }

    #[test]
    fn percent_encode_value_special_chars() {
        assert_eq!(percent_encode_value("hello world"), "hello%20world");
        assert_eq!(percent_encode_value("a+b=c&d"), "a%2Bb%3Dc%26d");
        assert_eq!(percent_encode_value("simple-token_123"), "simple-token_123");
    }
}
