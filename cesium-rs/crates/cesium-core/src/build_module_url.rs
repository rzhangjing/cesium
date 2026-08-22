//! Ported from `packages/engine/Source/Core/buildModuleUrl.js`.
//!
//! Given a relative URL under the Cesium base URL, returns an absolute URL.
//! Skeleton implementation for Rust.

use std::sync::Mutex;

static BASE_URL: Mutex<Option<String>> = Mutex::new(None);

/// Given a relative URL under the Cesium base URL, returns an absolute URL.
pub fn build_module_url(relative_url: &str) -> String {
    let base = BASE_URL.lock().unwrap();
    if let Some(base) = base.as_ref() {
        format!("{base}{relative_url}")
    } else {
        relative_url.to_string()
    }
}

/// Sets the base URL for resolving modules.
pub fn set_base_url(value: &str) {
    let mut base = BASE_URL.lock().unwrap();
    let mut url = value.to_string();
    if !url.ends_with('/') {
        url.push('/');
    }
    *base = Some(url);
}

/// Gets the configured base URL.
pub fn get_cesium_base_url() -> String {
    let base = BASE_URL.lock().unwrap();
    base.clone().unwrap_or_default()
}
