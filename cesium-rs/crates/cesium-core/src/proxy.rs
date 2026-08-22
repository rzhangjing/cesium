//! Ported from `packages/engine/Source/Core/Proxy.js`.

/// Base class for proxying requests made by `Resource`.
pub trait Proxy {
    /// Get the final URL to use to request a given resource.
    fn get_url(&self, resource: &str) -> String;
}
