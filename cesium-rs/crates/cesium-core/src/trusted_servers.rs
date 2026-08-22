//! Ported from `packages/engine/Source/Core/TrustedServers.js`.
//!
//! Manages a list of trusted servers for CORS handling.

use std::collections::HashSet;
use std::sync::Mutex;

static TRUSTED_SERVERS: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// Utilities for managing trusted servers.
pub struct TrustedServers;

impl TrustedServers {
    /// Adds a server to the trusted list.
    pub fn add(url: &str) {
        let mut servers = TRUSTED_SERVERS.lock().unwrap();
        let set = servers.get_or_insert_with(HashSet::new);
        set.insert(url.to_string());
    }

    /// Removes a server from the trusted list.
    pub fn remove(url: &str) {
        let mut servers = TRUSTED_SERVERS.lock().unwrap();
        if let Some(set) = servers.as_mut() {
            set.remove(url);
        }
    }

    /// Checks if a URL's server is trusted.
    pub fn is_trusted(url: &str) -> bool {
        let servers = TRUSTED_SERVERS.lock().unwrap();
        if let Some(set) = servers.as_ref() {
            set.contains(url)
        } else {
            false
        }
    }

    /// Clears all trusted servers.
    pub fn reset() {
        let mut servers = TRUSTED_SERVERS.lock().unwrap();
        *servers = None;
    }
}
