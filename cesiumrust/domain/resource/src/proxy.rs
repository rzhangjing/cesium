//! Proxy URL rewriting with trusted-server integration.
//!
//! Maps to CesiumJS `Core/DefaultProxy.js` + the proxy logic in
//! `Resource.prototype.getUrlComponent(query, proxy)` and
//! `Resource.prototype.fetch` (the `_Implementations.loadAndExecuteScript` /
//! `loadWithXhr` proxy branch).
//!
//! A [`DefaultProxy`] prepends a proxy URL to the resource URL so that
//! cross-origin requests can be routed through a same-origin server. When a
//! [`TrustedServers`] registry is configured, the proxy is **only applied to
//! untrusted** servers — trusted servers are accessed directly (credentials
//! flow without CORS preflight).
//!
//! This module is **pure domain logic** — no network IO, no framework
//! dependency.

use crate::trusted_servers::TrustedServers;

/// A simple proxy that appends the desired resource URL as the sole query
/// parameter to the proxy base URL.
///
/// Maps to CesiumJS `DefaultProxy`:
/// ```js
/// function DefaultProxy(proxy) { this.proxy = proxy; }
/// DefaultProxy.prototype.getURL = function(resource) {
///   var prefix = this.proxy.indexOf('?') === -1 ? '?' : '';
///   return this.proxy + prefix + resource;
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultProxy {
    /// The proxy base URL (e.g. `/proxy/` or `https://proxy.example.com/?url=`).
    proxy_url: String,
}

impl DefaultProxy {
    /// Creates a new proxy from the given proxy base URL.
    ///
    /// # Panics
    /// Panics if `proxy_url` is empty (mirrors CesiumJS `Check.typeOf.string`).
    pub fn new(proxy_url: impl Into<String>) -> Self {
        let url = proxy_url.into();
        assert!(!url.is_empty(), "DefaultProxy: proxy URL must not be empty");
        Self { proxy_url: url }
    }

    /// Returns the proxy base URL.
    pub fn proxy_url(&self) -> &str {
        &self.proxy_url
    }

    /// Rewrites `resource_url` to go through this proxy.
    ///
    /// If the proxy URL already contains a `?`, the resource is appended
    /// directly (assumes the proxy expects the URL as the last query param
    /// value). Otherwise a `?` separator is inserted.
    ///
    /// Maps to `DefaultProxy.prototype.getURL`.
    pub fn get_url(&self, resource_url: &str) -> String {
        let prefix = if self.proxy_url.contains('?') { "" } else { "?" };
        format!("{}{}{}", self.proxy_url, prefix, resource_url)
    }
}

/// A proxy policy that decides whether to proxy a given URL based on the
/// trusted-servers registry.
///
/// Maps to CesiumJS `Resource.prototype.getUrlComponent(query, proxy)` where
/// the proxy is applied only when the resource's server is NOT in
/// `TrustedServers`:
/// ```js
/// if (proxy && !TrustedServers.isTrusted(url)) {
///   url = proxy.getURL(url);
/// }
/// ```
///
/// The policy encapsulates this decision so callers don't need to know about
/// the trusted-servers check.
#[derive(Debug, Clone)]
pub struct ProxyPolicy {
    /// The proxy to use for untrusted servers (None = never proxy).
    proxy: Option<DefaultProxy>,
    /// Registry of trusted servers (accessed directly, credentials allowed).
    trusted_servers: TrustedServers,
}

impl ProxyPolicy {
    /// Creates a policy with no proxy (all URLs pass through unchanged).
    pub fn no_proxy() -> Self {
        Self {
            proxy: None,
            trusted_servers: TrustedServers::new(),
        }
    }

    /// Creates a policy that proxies untrusted servers through `proxy`.
    pub fn with_proxy(proxy: DefaultProxy) -> Self {
        Self {
            proxy: Some(proxy),
            trusted_servers: TrustedServers::new(),
        }
    }

    /// Creates a policy with both a proxy and a pre-populated trusted servers
    /// registry.
    pub fn new(proxy: Option<DefaultProxy>, trusted_servers: TrustedServers) -> Self {
        Self {
            proxy,
            trusted_servers,
        }
    }

    /// Returns a mutable reference to the trusted servers registry so hosts
    /// can add/remove entries at runtime.
    pub fn trusted_servers_mut(&mut self) -> &mut TrustedServers {
        &mut self.trusted_servers
    }

    /// Returns a reference to the trusted servers registry.
    pub fn trusted_servers(&self) -> &TrustedServers {
        &self.trusted_servers
    }

    /// Returns a reference to the proxy, if configured.
    pub fn proxy(&self) -> Option<&DefaultProxy> {
        self.proxy.as_ref()
    }

    /// Sets or replaces the proxy.
    pub fn set_proxy(&mut self, proxy: Option<DefaultProxy>) {
        self.proxy = proxy;
    }

    /// Determines whether `url` should be proxied.
    ///
    /// Returns `true` when:
    /// 1. A proxy is configured, AND
    /// 2. The URL's server is NOT in the trusted registry.
    ///
    /// Data URIs and blob URIs are never proxied (they have no server).
    pub fn should_proxy(&self, url: &str) -> bool {
        if self.proxy.is_none() {
            return false;
        }
        // Data URIs and blob URIs bypass proxying.
        if url.starts_with("data:") || url.starts_with("blob:") {
            return false;
        }
        !self.trusted_servers.is_trusted(url)
    }

    /// Applies the proxy to `url` if the policy says it should be proxied.
    /// Otherwise returns the URL unchanged.
    ///
    /// This is the single entry point for Resource's URL-building pipeline:
    /// ```ignore
    /// let final_url = policy.apply(&resource.build_url());
    /// ```
    pub fn apply(&self, url: &str) -> String {
        if self.should_proxy(url) {
            // Safe to unwrap: should_proxy returns true only when proxy is Some.
            self.proxy.as_ref().unwrap().get_url(url)
        } else {
            url.to_string()
        }
    }

    /// Applies the proxy and also passes through additional query parameters
    /// that should be appended to the *original* (pre-proxy) URL.
    ///
    /// This supports the CesiumJS pattern where `Resource.queryParameters` are
    /// appended before the proxy wraps the URL:
    /// ```text
    /// proxy.getURL(baseUrl + "?" + queryString)
    /// ```
    pub fn apply_with_query(&self, base_url: &str, query_string: &str) -> String {
        let url = if query_string.is_empty() {
            base_url.to_string()
        } else if base_url.contains('?') {
            format!("{}&{}", base_url, query_string)
        } else {
            format!("{}?{}", base_url, query_string)
        };
        self.apply(&url)
    }
}

impl Default for ProxyPolicy {
    fn default() -> Self {
        Self::no_proxy()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_proxy_appends_with_question_mark() {
        let proxy = DefaultProxy::new("https://proxy.example.com/");
        assert_eq!(
            proxy.get_url("https://tiles.example.com/0/0/0.png"),
            "https://proxy.example.com/?https://tiles.example.com/0/0/0.png"
        );
    }

    #[test]
    fn default_proxy_appends_without_extra_question_mark() {
        let proxy = DefaultProxy::new("https://proxy.example.com/?url=");
        assert_eq!(
            proxy.get_url("https://tiles.example.com/a.png"),
            "https://proxy.example.com/?url=https://tiles.example.com/a.png"
        );
    }

    #[test]
    #[should_panic(expected = "proxy URL must not be empty")]
    fn default_proxy_panics_on_empty() {
        DefaultProxy::new("");
    }

    #[test]
    fn policy_no_proxy_passes_through() {
        let policy = ProxyPolicy::no_proxy();
        assert_eq!(
            policy.apply("https://example.com/tile.png"),
            "https://example.com/tile.png"
        );
    }

    #[test]
    fn policy_proxies_untrusted_server() {
        let proxy = DefaultProxy::new("/proxy");
        let policy = ProxyPolicy::with_proxy(proxy);
        assert_eq!(
            policy.apply("https://untrusted.com/data"),
            "/proxy?https://untrusted.com/data"
        );
    }

    #[test]
    fn policy_skips_proxy_for_trusted_server() {
        let proxy = DefaultProxy::new("/proxy");
        let mut ts = TrustedServers::new();
        ts.add("trusted.com", 443);
        let policy = ProxyPolicy::new(Some(proxy), ts);

        // Trusted: no proxy
        assert_eq!(
            policy.apply("https://trusted.com/data"),
            "https://trusted.com/data"
        );
        // Untrusted: proxied
        assert_eq!(
            policy.apply("https://other.com/data"),
            "/proxy?https://other.com/data"
        );
    }

    #[test]
    fn policy_never_proxies_data_uri() {
        let proxy = DefaultProxy::new("/proxy");
        let policy = ProxyPolicy::with_proxy(proxy);
        assert_eq!(
            policy.apply("data:text/plain,hello"),
            "data:text/plain,hello"
        );
    }

    #[test]
    fn policy_never_proxies_blob_uri() {
        let proxy = DefaultProxy::new("/proxy");
        let policy = ProxyPolicy::with_proxy(proxy);
        assert_eq!(
            policy.apply("blob:https://example.com/uuid"),
            "blob:https://example.com/uuid"
        );
    }

    #[test]
    fn apply_with_query_builds_correct_url() {
        let policy = ProxyPolicy::no_proxy();
        assert_eq!(
            policy.apply_with_query("https://example.com/api", "key=value&foo=bar"),
            "https://example.com/api?key=value&foo=bar"
        );
        // Already has query
        assert_eq!(
            policy.apply_with_query("https://example.com/api?existing=1", "key=value"),
            "https://example.com/api?existing=1&key=value"
        );
        // Empty query
        assert_eq!(
            policy.apply_with_query("https://example.com/api", ""),
            "https://example.com/api"
        );
    }

    #[test]
    fn apply_with_query_proxies_result() {
        let proxy = DefaultProxy::new("/proxy");
        let policy = ProxyPolicy::with_proxy(proxy);
        assert_eq!(
            policy.apply_with_query("https://untrusted.com/api", "key=value"),
            "/proxy?https://untrusted.com/api?key=value"
        );
    }
}
