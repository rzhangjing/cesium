//! Cesium Ion 资产端点 URL 构造。
//!
//! 映射到 CesiumJS `Core/IonResource.js` + `Core/Ion.js`：
//! - `IonResource.fromAssetId(assetId, options)` —— 构建端点 URL。
//! - `IonResource._createEndpointResource(assetId, options)` —— 构造用于
//!   获取 ion 端点 JSON 的 Resource。
//! - 通过 `access_token` 查询参数和/或
//!   `Authorization: Bearer <token>` 头部注入令牌。
//!
//! **本模块是纯 URL/头部构造 —— 它不会发出网络请求。**
//! 实际的 HTTP 获取由调用方负责（通过适配层）。
//!
//! # Cesium Ion REST API
//!
//! 某个资产的 ion 端点为：
//! ```text
//! GET {server}/v1/assets/{assetId}/endpoint?access_token={token}
//! ```
//!
//! 响应包含：
//! - `url` —— 实际的资产内容 URL
//! - `accessToken` —— 用于内容 URL 的短期令牌
//! - `externalType` —— 若这是一个外部资产（"3DTILES"、"STK_TERRAIN_SERVER" 等）
//! - `options.url` —— 对于外部资产，外部资源的 URL
//! - `attributions` —— 致谢信息

use std::collections::HashMap;

/// 默认的 Cesium Ion API 服务器 URL。
///
/// 映射到 CesiumJS `Ion.defaultServer` = `"https://api.cesium.com/"`。
pub const DEFAULT_ION_SERVER: &str = "https://api.cesium.com/";

/// 构造 Ion 资产端点资源的选项。
///
/// 映射到 CesiumJS `IonResource.fromAssetId(assetId, options)`。
#[derive(Debug, Clone, Default)]
pub struct IonAssetOptions {
    /// 要使用的访问令牌。若为 None 则不注入令牌。
    ///
    /// 映射到 `options.accessToken`（回退到 `Ion.defaultAccessToken`）。
    pub access_token: Option<String>,

    /// Cesium ion API 服务器的 url。
    ///
    /// 映射到 `options.server`（回退到 `Ion.defaultServer`）。
    pub server: Option<String>,

    /// 端点请求的额外查询参数。
    ///
    /// 映射到 `options.queryParameters`（合并进端点 URL）。
    pub query_parameters: Option<HashMap<String, String>>,

    /// 除了在 `access_token` 查询参数之外（或替代它），
    /// 是否注入 `Authorization: Bearer` 头部。
    ///
    /// CesiumJS 两者都用：端点请求用查询参数，后续的
    /// 内容请求用 Bearer 头部。默认：`true`。
    pub use_bearer_header: Option<bool>,
}

/// 从 Cesium Ion 端点服务返回的资产端点数据。
///
/// 这是 **解析后的响应** 结构。从 JSON 构造由调用方负责
/// （领域层不为此依赖 serde_json；适配器或应用层
/// 进行反序列化）。
///
/// 映射到 `IonResource` 使用的 ion 端点 JSON 响应字段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IonEndpoint {
    /// 资产内容的 URL（瓦片、地形、影像）。
    pub url: String,

    /// 外部资产类型（`"3DTILES"`、`"STK_TERRAIN_SERVER"` 等），
    /// 仅当这是一个外部资产时。对于原生 ion 资产为 `None`。
    pub external_type: Option<String>,

    /// 用于针对内容 URL 请求的短期访问令牌。
    ///
    /// 映射到 `endpoint.accessToken`。
    pub access_token: Option<String>,

    /// 对于外部资产：`endpoint.options.url`（实际的外部 URL）。
    pub options_url: Option<String>,

    /// 致谢/信用 HTML 字符串。
    ///
    /// 映射到 `endpoint.attributions`。
    pub attributions: Vec<IonAttribution>,
}

/// 来自 ion 端点响应的单条致谢条目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IonAttribution {
    /// 致谢的 HTML 内容。
    pub html: String,
    /// 该致谢是否可折叠。
    pub collapsible: bool,
}

/// 描述一个已完全构造的 Ion 资源请求（URL + 头部）。
///
/// 这是纯 URL 构建逻辑的输出 —— 它告诉调用方
/// 要请求 *什么*，而不实际发起请求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IonResourceRequest {
    /// 已完全构造的端点 URL（含查询参数）。
    pub url: String,

    /// 随请求发送的 HTTP 头部（可能包含 Authorization）。
    pub headers: HashMap<String, String>,

    /// 本请求对应的资产 ID。
    pub asset_id: u64,

    /// 所使用的 ion 服务器基础 URL。
    pub server: String,
}

/// 为给定的资产 ID 构建 Ion 端点 URL。
///
/// 映射到 `IonResource._createEndpointResource(assetId, options)`。
///
/// 本函数是 **纯函数** —— 它构造 URL 和头部而不
/// 发起任何网络请求。调用方（适配层）负责
/// 实际获取端点。
///
/// # 示例
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

    // 规范化服务器 URL：确保末尾有斜杠以便拼接路径。
    let server_base = if server.ends_with('/') {
        server.clone()
    } else {
        format!("{}/", server)
    };

    // 构建端点路径：v1/assets/{assetId}/endpoint
    let endpoint_path = format!("v1/assets/{}/endpoint", asset_id);

    // 构建查询参数。
    let mut query_parts: Vec<String> = Vec::new();

    if let Some(ref token) = options.access_token {
        if !token.is_empty() {
            query_parts.push(format!("access_token={}", percent_encode_value(token)));
        }
    }

    // 合并额外的查询参数。
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

    // 构建头部。
    let mut headers = HashMap::new();

    // CesiumJS 客户端识别头部。
    // 映射到 IonResource.js 中的 `addClientHeaders(headers)`。
    headers.insert(
        "X-Cesium-Client".to_string(),
        "cesium-rust".to_string(),
    );

    // Authorization: Bearer 头部（若已配置且有可用令牌）。
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

/// 在给定端点响应的情况下，为 Ion 资产构建内容 URL。
///
/// 端点获取后，内容 URL 需要将端点的
/// `accessToken` 作为查询参数追加（CesiumJS 在
/// `IonResource.fromEndpoint` 中完成这一操作）。
///
/// 映射到 `new IonResource(endpoint, endpointResource)` 中的令牌注入：
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

/// 为向 Ion 资产发起的内容请求构建头部。
///
/// 包含来自端点响应的 Bearer 令牌（若可用）以及
/// 标准的 Cesium 客户端识别头部。
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

/// 判断一个 Ion 端点是否代表一个外部资产。
///
/// 映射到 `IonResource._isExternal` 逻辑：当
/// `endpoint.externalType` 已定义时，资产为外部资产。
pub fn is_external_asset(endpoint: &IonEndpoint) -> bool {
    endpoint.external_type.is_some()
}

/// 对于外部资产，返回有效的内容 URL。
///
/// 映射到 `IonResource.fromEndpoint`，其中外部资产使用
/// `endpoint.options.url` 而非 `endpoint.url`。
///
/// 对于非外部资产或缺少 options URL 的情况返回 `None`。
pub fn external_asset_url(endpoint: &IonEndpoint) -> Option<&str> {
    if !is_external_asset(endpoint) {
        return None;
    }
    endpoint.options_url.as_deref()
}

/// 检查某个外部资产类型是否受支持为 Resource。
///
/// 映射到 CesiumJS 的守卫：
/// ```js
/// if (externalType !== '3DTILES' && externalType !== 'STK_TERRAIN_SERVER') {
///   throw new RuntimeError('Ion.createResource does not support external imagery assets...');
/// }
/// ```
pub fn is_supported_external_type(endpoint: &IonEndpoint) -> bool {
    match endpoint.external_type.as_deref() {
        Some("3DTILES") | Some("STK_TERRAIN_SERVER") => true,
        Some(_) => false, // 例如 "IMAGERY" —— 不支持作为 Resource
        None => true,     // 非外部资产始终受支持
    }
}

/// 对查询参数值进行百分号编码。
///
/// 编码除未保留的 RFC 3986 字符
/// （`A-Z a-z 0-9 - _ . ~`）之外的一切字符。
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
        // 令牌仍在查询参数中，但没有 Authorization 头部
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
        // 查询参数已排序
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
