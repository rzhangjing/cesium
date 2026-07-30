//! TrustedServers spec - ported from packages/engine/Specs/Core/TrustedServersSpec.js
//!
//! A-class tests: 8 (pure logic, no browser/DOM)

use cesium_resource::trusted_servers::TrustedServers;

#[cfg(test)]
mod tests {
    use super::*;

    /// "http without a port"
    #[test]
    fn http_without_a_port() {
        let mut ts = TrustedServers::new();
        ts.add("cesiumjs.org", 80);
        assert!(ts.is_trusted("http://cesiumjs.org/index.html"));
        assert!(!ts.is_trusted("https://cesiumjs.org/index.html"));
    }

    /// "https without a port"
    #[test]
    fn https_without_a_port() {
        let mut ts = TrustedServers::new();
        ts.add("cesiumjs.org", 443);
        assert!(ts.is_trusted("https://cesiumjs.org/index.html"));
        assert!(!ts.is_trusted("http://cesiumjs.org/index.html"));
    }

    /// "add"
    #[test]
    fn add_with_explicit_port() {
        let mut ts = TrustedServers::new();
        assert!(!ts.is_trusted("http://cesiumjs.org:81/index.html"));
        ts.add("cesiumjs.org", 81);
        // Default port 80 should NOT match explicit port 81
        assert!(!ts.is_trusted("http://cesiumjs.org/index.html"));
        assert!(ts.is_trusted("http://cesiumjs.org:81/index.html"));
    }

    /// "remove"
    #[test]
    fn remove_server() {
        let mut ts = TrustedServers::new();
        ts.add("cesiumjs.org", 81);
        assert!(ts.is_trusted("http://cesiumjs.org:81/index.html"));
        // Removing wrong port should not affect
        ts.remove("cesiumjs.org", 8080);
        assert!(ts.is_trusted("http://cesiumjs.org:81/index.html"));
        // Removing correct port
        ts.remove("cesiumjs.org", 81);
        assert!(!ts.is_trusted("http://cesiumjs.org:81/index.html"));
    }

    /// "handles username/password credentials"
    #[test]
    fn handles_credentials() {
        let mut ts = TrustedServers::new();
        ts.add("cesiumjs.org", 81);
        assert!(ts.is_trusted("http://user:pass@cesiumjs.org:81/index.html"));
    }

    /// "always returns false for relative paths"
    #[test]
    fn relative_paths_return_false() {
        let ts = TrustedServers::new();
        assert!(!ts.is_trusted("./data/index.html"));
    }

    /// "handles protocol relative URLs"
    #[test]
    fn protocol_relative_urls() {
        let mut ts = TrustedServers::new();
        ts.add("cesiumjs.org", 80);
        // Protocol-relative URL without port → can't determine default port
        // CesiumJS uses window.location.protocol, we return false
        // But with explicit port it should work
        assert!(!ts.is_trusted("//cesiumjs.org/index.html"));
        // With explicit port
        ts.add("cesiumjs.org", 8080);
        assert!(ts.is_trusted("//cesiumjs.org:8080/index.html"));
    }

    /// "clear"
    #[test]
    fn clear_all() {
        let mut ts = TrustedServers::new();
        ts.add("cesiumjs.org", 80);
        assert!(ts.is_trusted("http://cesiumjs.org/index.html"));
        ts.clear();
        assert!(!ts.is_trusted("http://cesiumjs.org/index.html"));
        // Can add again after clear
        ts.add("cesiumjs.org", 80);
        assert!(ts.is_trusted("http://cesiumjs.org/index.html"));
    }
}
