//! Ported from packages/engine/Source/Core/FeatureDetection.js
//!
//! DEVIATION: browser feature probes (`navigator`, `document`, `Image`,
//! `PointerEvent`, ...) have no native counterpart. The Rust port keeps the
//! full detection logic operating on an injectable user-agent/app-version
//! triple ([`FeatureDetector`]) and exposes module-level functions bound to
//! a native detector (empty user agent → every browser detection returns
//! false; capability probes answer with the native equivalent). DOM-only
//! probes are stubbed and registered in docs/deviations.md.

use std::sync::{LazyLock, OnceLock};

use crate::developer_error::throw_developer_error;

fn extract_version(version_string: &str) -> Vec<f64> {
    version_string
        .split('.')
        .map(|part| {
            // parseInt(part, 10): leading digits, NaN when none.
            let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse::<f64>().unwrap_or(f64::NAN)
        })
        .collect()
}

/// Captures the version number following `token` (e.g. `" Chrome/"`),
/// matching the JS regex `/ Token\/([\.0-9]+)/` semantics.
fn capture_version_after(haystack: &str, token: &str) -> Option<Vec<f64>> {
    let index = haystack.find(token)?;
    let rest = &haystack[index + token.len()..];
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(rest.len());
    let version_string = &rest[..end];
    if version_string.is_empty() {
        return None;
    }
    Some(extract_version(version_string))
}

/// Version of the detected Webkit engine (JS array + `isNightly` flag).
#[derive(Debug, Clone)]
pub struct WebkitVersion {
    /// Dotted version components.
    pub parts: Vec<f64>,
    /// True when the `+` nightly marker was present.
    pub is_nightly: bool,
}

/// A set of functions to detect whether the current browser supports
/// various features.
///
/// Port of the CesiumJS `FeatureDetection` namespace; the detection state
/// is cached per detector exactly like the JS module-level caches.
#[derive(Default)]
pub struct FeatureDetector {
    user_agent: String,
    app_version: String,
    platform: String,

    is_chrome_result: OnceLock<Option<Vec<f64>>>,
    is_safari_result: OnceLock<Option<Vec<f64>>>,
    is_webkit_result: OnceLock<Option<WebkitVersion>>,
    is_edge_result: OnceLock<Option<Vec<f64>>>,
    is_firefox_result: OnceLock<Option<Vec<f64>>>,
    is_windows_result: OnceLock<bool>,
    is_ipad_or_ios_result: OnceLock<bool>,
    has_pointer_events: OnceLock<bool>,
    supports_image_rendering_pixelated_result: OnceLock<Option<String>>,
}

impl FeatureDetector {
    /// Creates a detector for the given browser identification strings.
    #[must_use]
    pub fn new(user_agent: &str, app_version: &str, platform: &str) -> Self {
        Self {
            user_agent: user_agent.to_owned(),
            app_version: app_version.to_owned(),
            platform: platform.to_owned(),
            ..Self::default()
        }
    }

    fn is_chrome_cached(&self) -> Option<Vec<f64>> {
        self.is_chrome_result
            .get_or_init(|| {
                // Edge contains Chrome in the user agent too
                if self.is_edge() {
                    return None;
                }
                capture_version_after(&self.user_agent, " Chrome/")
            })
            .clone()
    }

    /// Port of `FeatureDetection.isChrome`.
    #[must_use]
    pub fn is_chrome(&self) -> bool {
        self.is_chrome_cached().is_some()
    }

    /// Port of `FeatureDetection.chromeVersion` (JS returns `false` or the
    /// version array; the Rust port uses `Option`).
    #[must_use]
    pub fn chrome_version(&self) -> Option<Vec<f64>> {
        self.is_chrome_cached()
    }

    fn is_safari_cached(&self) -> Option<Vec<f64>> {
        self.is_safari_result
            .get_or_init(|| {
                // Chrome and Edge contain Safari in the user agent too
                if self.is_chrome() || self.is_edge() {
                    return None;
                }
                let safari_token = self.user_agent.find(" Safari/").and_then(|i| {
                    let rest = &self.user_agent[i + " Safari/".len()..];
                    if rest
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_digit() || c == '.')
                    {
                        Some(())
                    } else {
                        None
                    }
                });
                if safari_token.is_none() {
                    return None;
                }
                capture_version_after(&self.user_agent, " Version/")
            })
            .clone()
    }

    /// Port of `FeatureDetection.isSafari`.
    #[must_use]
    pub fn is_safari(&self) -> bool {
        self.is_safari_cached().is_some()
    }

    /// Port of `FeatureDetection.safariVersion`.
    #[must_use]
    pub fn safari_version(&self) -> Option<Vec<f64>> {
        self.is_safari_cached()
    }

    fn is_webkit_cached(&self) -> Option<WebkitVersion> {
        self.is_webkit_result
            .get_or_init(|| {
                let index = self.user_agent.find(" AppleWebKit/")?;
                let rest = &self.user_agent[index + " AppleWebKit/".len()..];
                let end = rest
                    .find(|c: char| !(c.is_ascii_digit() || c == '.'))
                    .unwrap_or(rest.len());
                let version_string = &rest[..end];
                if version_string.is_empty() {
                    return None;
                }
                let is_nightly = rest[end..].starts_with('+');
                Some(WebkitVersion {
                    parts: extract_version(version_string),
                    is_nightly,
                })
            })
            .clone()
    }

    /// Port of `FeatureDetection.isWebkit`.
    #[must_use]
    pub fn is_webkit(&self) -> bool {
        self.is_webkit_cached().is_some()
    }

    /// Port of `FeatureDetection.webkitVersion`.
    #[must_use]
    pub fn webkit_version(&self) -> Option<WebkitVersion> {
        self.is_webkit_cached()
    }

    /// Port of `FeatureDetection.isEdge`.
    #[must_use]
    pub fn is_edge(&self) -> bool {
        self.edge_version().is_some()
    }

    /// Port of `FeatureDetection.edgeVersion`.
    #[must_use]
    pub fn edge_version(&self) -> Option<Vec<f64>> {
        self.is_edge_result
            .get_or_init(|| capture_version_after(&self.user_agent, " Edg/"))
            .clone()
    }

    /// Port of `FeatureDetection.isFirefox`.
    #[must_use]
    pub fn is_firefox(&self) -> bool {
        self.firefox_version().is_some()
    }

    /// Port of `FeatureDetection.firefoxVersion`.
    #[must_use]
    pub fn firefox_version(&self) -> Option<Vec<f64>> {
        self.is_firefox_result
            .get_or_init(|| capture_version_after(&self.user_agent, "Firefox/"))
            .clone()
    }

    /// Port of `FeatureDetection.isWindows`.
    #[must_use]
    pub fn is_windows(&self) -> bool {
        *self
            .is_windows_result
            .get_or_init(|| self.app_version.to_ascii_lowercase().contains("windows"))
    }

    /// Port of `FeatureDetection.isIPadOrIOS`.
    #[must_use]
    pub fn is_ipad_or_ios(&self) -> bool {
        *self.is_ipad_or_ios_result.get_or_init(|| {
            self.platform == "iPhone" || self.platform == "iPod" || self.platform == "iPad"
        })
    }

    /// Port of `FeatureDetection.supportsPointerEvents` (native builds have
    /// no `PointerEvent` global → false).
    #[must_use]
    pub fn supports_pointer_events(&self) -> bool {
        *self.has_pointer_events.get_or_init(|| {
            // Firefox disabled because of https://github.com/CesiumGS/cesium/issues/6372
            // DEVIATION: `typeof PointerEvent !== "undefined"` is false in
            // native builds; see docs/deviations.md.
            !self.is_firefox() && false
        })
    }

    /// Port of `FeatureDetection.supportsImageRenderingPixelated` (no DOM →
    /// false).
    #[must_use]
    pub fn supports_image_rendering_pixelated(&self) -> bool {
        self.image_rendering_value().is_some()
    }

    /// Port of `FeatureDetection.imageRenderingValue`.
    #[must_use]
    pub fn image_rendering_value(&self) -> Option<String> {
        self.supports_image_rendering_pixelated_result
            .get_or_init(|| {
                // DEVIATION: needs a DOM canvas to probe CSS support; the
                // native port reports no support. See docs/deviations.md.
                None
            })
            .clone()
    }

    /// Port of `FeatureDetection.supportsEsmWebWorkers`.
    #[must_use]
    pub fn supports_esm_web_workers(&self) -> bool {
        let Some(version) = self.firefox_version() else {
            return true; // !isFirefox()
        };
        version.first().copied().unwrap_or(f64::NAN) >= 114.0
    }
}

/// Native-environment detector (empty user agent).
static NATIVE: LazyLock<FeatureDetector> = LazyLock::new(|| FeatureDetector::default());

/// Port of `FeatureDetection.isChrome` for the current (native) environment.
#[must_use]
pub fn is_chrome() -> bool {
    NATIVE.is_chrome()
}

/// Port of `FeatureDetection.chromeVersion`.
#[must_use]
pub fn chrome_version() -> Option<Vec<f64>> {
    NATIVE.chrome_version()
}

/// Port of `FeatureDetection.isSafari`.
#[must_use]
pub fn is_safari() -> bool {
    NATIVE.is_safari()
}

/// Port of `FeatureDetection.safariVersion`.
#[must_use]
pub fn safari_version() -> Option<Vec<f64>> {
    NATIVE.safari_version()
}

/// Port of `FeatureDetection.isWebkit`.
#[must_use]
pub fn is_webkit() -> bool {
    NATIVE.is_webkit()
}

/// Port of `FeatureDetection.webkitVersion`.
#[must_use]
pub fn webkit_version() -> Option<WebkitVersion> {
    NATIVE.webkit_version()
}

/// Port of `FeatureDetection.isEdge`.
#[must_use]
pub fn is_edge() -> bool {
    NATIVE.is_edge()
}

/// Port of `FeatureDetection.edgeVersion`.
#[must_use]
pub fn edge_version() -> Option<Vec<f64>> {
    NATIVE.edge_version()
}

/// Port of `FeatureDetection.isFirefox`.
#[must_use]
pub fn is_firefox() -> bool {
    NATIVE.is_firefox()
}

/// Port of `FeatureDetection.firefoxVersion`.
#[must_use]
pub fn firefox_version() -> Option<Vec<f64>> {
    NATIVE.firefox_version()
}

/// Port of `FeatureDetection.isWindows`.
#[must_use]
pub fn is_windows() -> bool {
    NATIVE.is_windows()
}

/// Port of `FeatureDetection.isIPadOrIOS`.
#[must_use]
pub fn is_ipad_or_ios() -> bool {
    NATIVE.is_ipad_or_ios()
}

/// Port of `FeatureDetection.hardwareConcurrency`
/// (`theNavigator.hardwareConcurrency ?? 3`).
#[must_use]
pub fn hardware_concurrency() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(3)
}

/// Port of `FeatureDetection.supportsPointerEvents`.
#[must_use]
pub fn supports_pointer_events() -> bool {
    NATIVE.supports_pointer_events()
}

/// Port of `FeatureDetection.supportsImageRenderingPixelated`.
#[must_use]
pub fn supports_image_rendering_pixelated() -> bool {
    NATIVE.supports_image_rendering_pixelated()
}

/// Port of `FeatureDetection.imageRenderingValue`.
#[must_use]
pub fn image_rendering_value() -> Option<String> {
    NATIVE.image_rendering_value()
}

/// Port of `FeatureDetection.typedArrayTypes` (names of the typed array
/// constructors; Rust typed arrays are always available).
#[must_use]
pub fn typed_array_types() -> &'static [&'static str] {
    &[
        "Int8Array",
        "Uint8Array",
        "Int16Array",
        "Uint16Array",
        "Int32Array",
        "Uint32Array",
        "Float32Array",
        "Float64Array",
        "Uint8ClampedArray",
        "Uint8ClampedArray",
        "BigInt64Array",
        "BigUint64Array",
    ]
}

/// Port of `FeatureDetection.supportsFullscreen` (native builds have no
/// Fullscreen API → false; the browser path goes through the Fullscreen
/// module once ported).
#[must_use]
pub fn supports_fullscreen() -> bool {
    // DEVIATION: Fullscreen.js depends on the DOM; see docs/deviations.md.
    false
}

/// Port of `FeatureDetection.supportsTypedArrays`.
#[must_use]
pub fn supports_typed_arrays() -> bool {
    true
}

/// Port of `FeatureDetection.supportsBigInt64Array`.
#[must_use]
pub fn supports_big_int64_array() -> bool {
    true
}

/// Port of `FeatureDetection.supportsBigUint64Array`.
#[must_use]
pub fn supports_big_uint64_array() -> bool {
    true
}

/// Port of `FeatureDetection.supportsBigInt`.
#[must_use]
pub fn supports_big_int() -> bool {
    true
}

/// Port of `FeatureDetection.supportsWebWorkers` (native builds use OS
/// threads instead of Web Workers → false).
#[must_use]
pub fn supports_web_workers() -> bool {
    // DEVIATION: `typeof Worker !== "undefined"` is false in native builds.
    false
}

/// Port of `FeatureDetection.supportsWebAssembly`.
#[must_use]
pub fn supports_web_assembly() -> bool {
    cfg!(target_family = "wasm")
}

// ---------------------------------------------------------------------------
// supportsWebP state machine
// ---------------------------------------------------------------------------

static WEBP_PROMISE_DONE: LazyLock<std::sync::Mutex<Option<bool>>> =
    LazyLock::new(|| std::sync::Mutex::new(None));

/// Port of `FeatureDetection.supportsWebP.initialized` (getter).
#[must_use]
pub fn supports_web_p_initialized() -> bool {
    WEBP_PROMISE_DONE.lock().expect("webp state poisoned").is_some()
}

/// Port of `FeatureDetection.supportsWebP()`.
///
/// # Panics
/// Panics with `DeveloperError` when [`supports_web_p_initialize`] has not
/// completed yet (mirrors the JS debug check).
#[must_use]
pub fn supports_web_p() -> bool {
    // >>includeStart('debug', pragmas.debug)
    if cfg!(debug_assertions) && !supports_web_p_initialized() {
        throw_developer_error(
            "You must call FeatureDetection.supportsWebP.initialize and wait for the promise to resolve before calling FeatureDetection.supportsWebP",
        );
    }
    // >>includeEnd('debug')
    WEBP_PROMISE_DONE
        .lock()
        .expect("webp state poisoned")
        .unwrap_or(false)
}

/// Port of `FeatureDetection.supportsWebP.initialize()`.
///
/// DEVIATION: the JS probe decodes a 1×1 WebP through an `Image` element;
/// the native port resolves `false` (no DOM image pipeline). See
/// docs/deviations.md.
pub async fn supports_web_p_initialize() -> bool {
    let mut state = WEBP_PROMISE_DONE.lock().expect("webp state poisoned");
    if let Some(result) = *state {
        return result;
    }
    // DEVIATION: new Image() decode probe replaced by a constant false.
    *state = Some(false);
    false
}

/// Resets the WebP detection state (spec helper; the JS specs mutate
/// `supportsWebP._promise` / `_result` directly).
pub fn reset_web_p_state_for_specs() {
    *WEBP_PROMISE_DONE.lock().expect("webp state poisoned") = None;
}

/// Port of `FeatureDetection.supportsEsmWebWorkers`.
#[must_use]
pub fn supports_esm_web_workers() -> bool {
    NATIVE.supports_esm_web_workers()
}
