//! Ported from `packages/engine/Source/Core/buildModuleUrl.js` (154 lines).
//!
//! Given a relative URL under the Cesium base URL, returns an absolute URL.
//!
//! # Method-level alignment table (JS `buildModuleUrl` -> Rust)
//!
//! | CesiumJS (buildModuleUrl.js)            | Rust                                    |
//! | ---------------------------------------- | --------------------------------------- |
//! | `buildModuleUrl(relativeUrl)`            | [`build_module_url`]                    |
//! | `buildModuleUrl.setBaseUrl(value)`       | [`set_base_url`]                        |
//! | `buildModuleUrl.getCesiumBaseUrl()`      | [`get_cesium_base_url`]                 |
//! | `buildModuleUrl._clearBaseResource()`    | [`clear_base_resource`]                 |
//! | `buildModuleUrlFromBaseUrl(moduleID)`    | [`build_module_url_from_base_url`]      |
//! | `buildModuleUrlFromRequireToUrl`         | DEVIATION: no AMD/require.toUrl in Rust |
//! | `getBaseUrlFromCesiumScript` (DOM)       | DEVIATION: no DOM in Rust               |
//! | `CESIUM_BASE_URL` / `import.meta.url`    | DEVIATION: base url via [`set_base_url`]|

use std::sync::{Mutex, OnceLock};

use crate::developer_error::throw_developer_error;
use crate::resource::{DerivedResourceOptions, Resource};

fn base_resource_slot() -> &'static Mutex<Option<Resource>> {
    static BASE_RESOURCE: OnceLock<Mutex<Option<Resource>>> = OnceLock::new();
    BASE_RESOURCE.get_or_init(|| Mutex::new(None))
}

/// Locks the slot, recovering from poisoning (a DeveloperError panic raised
/// by [`get_cesium_base_url`] must not permanently poison the global).
fn lock_slot() -> std::sync::MutexGuard<'static, Option<Resource>> {
    base_resource_slot()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Gets the base URL for resolving modules.
///
/// Mirrors `buildModuleUrl.getCesiumBaseUrl()`: returns the cached base
/// [`Resource`], determining it automatically if not set yet.
///
/// DEVIATION: CesiumJS auto-detects the base url from `CESIUM_BASE_URL`,
/// `import.meta.url`, `require.toUrl` or the `Cesium.js` script tag; none of
/// those exist in the Rust port, so the base url must have been configured
/// with [`set_base_url`] first.
///
/// # Panics
/// Panics with `DeveloperError` when no base URL has been set (JS debug:
/// "Unable to determine Cesium base URL automatically, try defining a global
/// variable called CESIUM_BASE_URL.").
#[must_use]
pub fn get_cesium_base_url() -> Resource {
    let existing = lock_slot().as_ref().map(Resource::clone_resource);
    if let Some(resource) = existing {
        return resource;
    }

    // >>includeStart('debug', pragmas.debug);
    // No CESIUM_BASE_URL global / import.meta / DOM detection is available
    // in Rust, so an unset base url always fails the JS debug check.
    throw_developer_error(
        "Unable to determine Cesium base URL automatically, try defining a global variable called CESIUM_BASE_URL.",
    );
    // >>includeEnd('debug')
}

/// Sets the base URL for resolving modules.
///
/// Mirrors `buildModuleUrl.setBaseUrl(value)`:
/// `baseResource = Resource.DEFAULT.getDerivedResource({ url: value })`.
pub fn set_base_url(value: &str) {
    let resource = Resource::default_resource()
        .get_derived_resource_with_options(DerivedResourceOptions {
            url: Some(value),
            ..Default::default()
        });
    *lock_slot() = Some(resource);
}

/// Clears the cached base resource (exposed for testing).
///
/// Mirrors `buildModuleUrl._clearBaseResource()`.
pub fn clear_base_resource() {
    *lock_slot() = None;
}

fn build_module_url_from_base_url(module_id: &str) -> String {
    let resource = get_cesium_base_url().get_derived_resource_with_options(
        DerivedResourceOptions {
            url: Some(module_id),
            ..Default::default()
        },
    );
    resource.url()
}

/// Given a relative URL under the Cesium base URL, returns an absolute URL.
///
/// Mirrors `buildModuleUrl(relativeUrl)`.
///
/// DEVIATION: CesiumJS selects between a `require.toUrl` implementation and
/// the base-url implementation; the Rust port always uses the base-url
/// implementation.
#[must_use]
pub fn build_module_url(relative_url: &str) -> String {
    build_module_url_from_base_url(relative_url)
}

/// Legacy stub handle so existing specs can reference the module by type.
///
/// DEVIATION: CesiumJS exports the `buildModuleUrl` function itself (with
/// attached `setBaseUrl` / `getCesiumBaseUrl` members); the Rust port exposes
/// free functions instead and keeps this unit struct only for backwards
/// compatibility with earlier stub specs.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuildModuleUrl {
    _private: (),
}

impl BuildModuleUrl {
    /// Creates a new BuildModuleUrl (legacy stub).
    #[must_use]
    pub fn new() -> Self {
        Self { _private: () }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Single test: the module keeps a process-global base resource, so the
    // cases are sequenced here to avoid cross-test races.
    #[test]
    fn build_module_url_flow() {
        // unset base url -> DeveloperError (JS debug check)
        clear_base_resource();
        let result = std::panic::catch_unwind(get_cesium_base_url);
        assert!(result.is_err());

        // setBaseUrl + buildModuleUrl relative resolution
        set_base_url("https://example.com/Cesium/");
        assert_eq!(
            get_cesium_base_url().url(),
            "https://example.com/Cesium/"
        );
        let url = build_module_url("Assets/Textures/logo.png");
        assert_eq!(url, "https://example.com/Cesium/Assets/Textures/logo.png");

        // setBaseUrl with an absolute file-style url keeps it as-is (JS
        // appendForwardSlash only runs in the auto-detect path)
        set_base_url("https://example.com/Cesium/BaseUrl.js");
        assert_eq!(
            get_cesium_base_url().url(),
            "https://example.com/Cesium/BaseUrl.js"
        );

        clear_base_resource();
    }
}
