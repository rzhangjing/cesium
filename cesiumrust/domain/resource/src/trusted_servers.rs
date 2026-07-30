//! Trusted servers registry.
//!
//! Maps to CesiumJS `Core/TrustedServers.js`.
//!
//! A registry of servers that are trusted. Credentials will be sent
//! with any requests to these servers.

use std::collections::HashSet;

/// A registry of trusted servers.
///
/// Maps to CesiumJS `TrustedServers`.
#[derive(Debug, Default)]
pub struct TrustedServers {
    servers: HashSet<String>,
}

impl TrustedServers {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            servers: HashSet::new(),
        }
    }

    /// Adds a trusted server to the registry.
    ///
    /// Maps to `TrustedServers.add`.
    pub fn add(&mut self, host: &str, port: u16) {
        let authority = format!("{}:{}", host.to_lowercase(), port);
        self.servers.insert(authority);
    }

    /// Removes a trusted server from the registry.
    ///
    /// Maps to `TrustedServers.remove`.
    pub fn remove(&mut self, host: &str, port: u16) {
        let authority = format!("{}:{}", host.to_lowercase(), port);
        self.servers.remove(&authority);
    }

    /// Returns whether a URL is trusted.
    ///
    /// Maps to `TrustedServers.isTrusted`.
    pub fn is_trusted(&self, url: &str) -> bool {
        match Self::get_authority(url) {
            Some(authority) => self.servers.contains(&authority),
            None => false,
        }
    }

    /// Clears all trusted servers.
    pub fn clear(&mut self) {
        self.servers.clear();
    }

    /// Returns the number of trusted servers.
    pub fn len(&self) -> usize {
        self.servers.len()
    }

    /// Returns whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.servers.is_empty()
    }

    /// Extracts the authority (host:port) from a URL.
    ///
    /// Handles:
    /// - http/https schemes with default ports (80/443)
    /// - Username:password@ prefix stripping
    /// - Protocol-relative URLs (//host/path)
    ///
    /// Returns None for relative URLs or unknown schemes.
    fn get_authority(url: &str) -> Option<String> {
        let url = url.trim();

        // Handle protocol-relative URLs
        if url.starts_with("//") {
            let rest = &url[2..];
            let authority = rest.split('/').next().unwrap_or("");
            if authority.is_empty() {
                return None;
            }
            let authority = Self::strip_credentials(authority);
            // No scheme → can't determine default port
            if authority.contains(':') {
                return Some(authority.to_lowercase());
            }
            return None;
        }

        // Parse scheme
        let scheme_end = url.find("://")?;
        let scheme = &url[..scheme_end].to_lowercase();
        let rest = &url[scheme_end + 3..];

        // Extract authority (before first /)
        let authority = rest.split('/').next().unwrap_or("");
        if authority.is_empty() {
            return None;
        }

        let authority = Self::strip_credentials(authority);

        // Add default port if missing
        if authority.contains(':') {
            Some(authority.to_lowercase())
        } else {
            match scheme.as_str() {
                "http" => Some(format!("{}:80", authority.to_lowercase())),
                "https" => Some(format!("{}:443", authority.to_lowercase())),
                _ => None,
            }
        }
    }

    /// Strips username:password@ from an authority string.
    fn strip_credentials(authority: &str) -> &str {
        if let Some(at_pos) = authority.find('@') {
            &authority[at_pos + 1..]
        } else {
            authority
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_is_trusted() {
        let mut ts = TrustedServers::new();
        ts.add("example.com", 80);
        assert!(ts.is_trusted("http://example.com/path"));
        assert!(!ts.is_trusted("http://other.com/path"));
    }

    #[test]
    fn test_remove() {
        let mut ts = TrustedServers::new();
        ts.add("example.com", 80);
        ts.remove("example.com", 80);
        assert!(!ts.is_trusted("http://example.com/path"));
    }

    #[test]
    fn test_default_port_https() {
        let mut ts = TrustedServers::new();
        ts.add("secure.com", 443);
        assert!(ts.is_trusted("https://secure.com/api"));
    }

    #[test]
    fn test_credentials_stripped() {
        let mut ts = TrustedServers::new();
        ts.add("example.com", 80);
        assert!(ts.is_trusted("http://user:pass@example.com/path"));
    }

    #[test]
    fn test_clear() {
        let mut ts = TrustedServers::new();
        ts.add("a.com", 80);
        ts.add("b.com", 443);
        ts.clear();
        assert!(ts.is_empty());
    }
}
