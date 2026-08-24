//! Ported from `packages/engine/Source/Core/TrustedServers.js` (147 lines).
//!
//! A singleton that contains all of the servers that are trusted. Credentials
//! will be sent with any requests to these servers.
//!
//! # Method-level alignment table (JS `TrustedServers` -> Rust)
//!
//! | CesiumJS                  | Rust                                        |
//! | ------------------------- | ------------------------------------------- |
//! | `TrustedServers.add`      | [`TrustedServers::add_server`]              |
//! | `TrustedServers.remove`   | [`TrustedServers::remove_server`]           |
//! | `TrustedServers.contains` | [`TrustedServers::contains`]                |
//! | `TrustedServers.clear`    | [`TrustedServers::clear`]                   |
//!
//! DEVIATION: legacy url-string API (`add`/`remove`/`is_trusted`/`reset`)
//! is retained for backward compatibility with earlier specs.

use std::collections::HashSet;
use std::sync::Mutex;

use crate::developer_error::throw_developer_error;

static TRUSTED_SERVERS: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// Utilities for managing trusted servers.
///
/// Mirrors the `TrustedServers` namespace singleton.
pub struct TrustedServers;

impl TrustedServers {
    /// Adds a trusted server to the registry.
    ///
    /// Mirrors `TrustedServers.add(host, port)`.
    ///
    /// # Panics
    /// In debug builds, panics with `DeveloperError` when `port <= 0`.
    pub fn add_server(host: &str, port: u32) {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) && port == 0 {
            throw_developer_error("port is required to be greater than 0.");
        }
        //>>includeEnd('debug');

        let authority = format!("{}:{port}", host.to_lowercase());
        let mut servers = TRUSTED_SERVERS.lock().unwrap();
        let set = servers.get_or_insert_with(HashSet::new);
        set.insert(authority);
    }

    /// Removes a trusted server from the registry.
    ///
    /// Mirrors `TrustedServers.remove(host, port)`.
    ///
    /// # Panics
    /// In debug builds, panics with `DeveloperError` when `port <= 0`.
    pub fn remove_server(host: &str, port: u32) {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) && port == 0 {
            throw_developer_error("port is required to be greater than 0.");
        }
        //>>includeEnd('debug');

        let authority = format!("{}:{port}", host.to_lowercase());
        let mut servers = TRUSTED_SERVERS.lock().unwrap();
        if let Some(set) = servers.as_mut() {
            set.remove(&authority);
        }
    }

    /// Tests whether a server is trusted or not. The server must have been
    /// added with the port if it is included in the url.
    ///
    /// Mirrors `TrustedServers.contains(url)`.
    pub fn contains(url: &str) -> bool {
        let Some(authority) = get_authority(url) else {
            return false;
        };
        let servers = TRUSTED_SERVERS.lock().unwrap();
        servers
            .as_ref()
            .is_some_and(|set| set.contains(&authority))
    }

    /// Clears the registry.
    ///
    /// Mirrors `TrustedServers.clear()`.
    pub fn clear() {
        let mut servers = TRUSTED_SERVERS.lock().unwrap();
        *servers = None;
    }

    // ── Legacy url-string API (backward compatibility) ───────────────

    /// Adds a server to the trusted list (legacy url-string form).
    pub fn add(url: &str) {
        let mut servers = TRUSTED_SERVERS.lock().unwrap();
        let set = servers.get_or_insert_with(HashSet::new);
        set.insert(url.to_string());
    }

    /// Removes a server from the trusted list (legacy url-string form).
    pub fn remove(url: &str) {
        let mut servers = TRUSTED_SERVERS.lock().unwrap();
        if let Some(set) = servers.as_mut() {
            set.remove(url);
        }
    }

    /// Checks if a URL's server is trusted (legacy exact-match form).
    pub fn is_trusted(url: &str) -> bool {
        let servers = TRUSTED_SERVERS.lock().unwrap();
        if let Some(set) = servers.as_ref() {
            set.contains(url)
        } else {
            false
        }
    }

    /// Clears all trusted servers (legacy alias of [`Self::clear`]).
    pub fn reset() {
        Self::clear();
    }
}

/// Mirrors private `getAuthority(url)` in TrustedServers.js.
///
/// DEVIATION: JS falls back to `window.location.protocol` for scheme-less
/// urls; the native port treats them as untrusted (returns `None`).
fn get_authority(url: &str) -> Option<String> {
    let uri = url::Url::parse(url).ok()?;

    let host = uri.host_str()?.to_string();
    let mut authority = host;

    // If the port is missing add one based on the scheme
    match uri.port_or_known_default() {
        Some(port) => authority = format!("{authority}:{port}"),
        None => return None,
    }

    Some(authority)
}
