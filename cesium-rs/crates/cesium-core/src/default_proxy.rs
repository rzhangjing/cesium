//! Ported from `packages/engine/Source/Core/DefaultProxy.js`.

/// A simple proxy that appends the desired resource as the sole query parameter
/// to the given proxy URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultProxy {
    /// The proxy URL that will be used to request all resources.
    pub proxy: String,
}

impl DefaultProxy {
    pub fn new(proxy: &str) -> Self {
        Self {
            proxy: proxy.to_string(),
        }
    }

    /// Get the final URL to use to request a given resource.
    pub fn get_url(&self, resource: &str) -> String {
        let prefix = if self.proxy.contains('?') { "" } else { "?" };
        format!("{}{}{}", self.proxy, prefix, resource)
    }
}
