//! Ported from packages/engine/Source/Core/loadAndExecuteScript.js
//!
//! DEVIATION: the CesiumJS implementation injects a `<script>` element into
//! the DOM and resolves when it has loaded and executed. Native/wgpu builds
//! have no DOM; the function keeps the same async signature and resolves the
//! script through the platform mechanism once one exists (tracked in
//! docs/deferred.md). See docs/deviations.md.

use crate::runtime_error::RuntimeError;

/// @private
///
/// Port of CesiumJS `loadAndExecuteScript(url)`.
///
/// # Errors
/// Always fails on native targets: the DOM-based script injection has no
/// native counterpart yet (see docs/deferred.md).
pub async fn load_and_execute_script(url: &str) -> Result<(), RuntimeError> {
    // DEVIATION: document.createElement("script") pipeline cannot run
    // outside a browser; see module docs.
    Err(RuntimeError::new(Some(&format!(
        "loadAndExecuteScript is not supported on native targets (requested: {url})"
    ))))
}
