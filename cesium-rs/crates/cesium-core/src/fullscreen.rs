//! Ported from `packages/engine/Source/Core/Fullscreen.js` (250 lines).
//!
//! Browser-independent functions for working with the standard fullscreen
//! API.
//!
//! DEVIATION: the JS module probes the live browser `document`/`document.body`
//! for capability functions (`requestFullscreen`, prefixed variants, ...).
//! The Rust port has no DOM, so the probe surface is injected through the
//! [`FullscreenDocument`] trait; the default [`HeadlessDocument`] reports no
//! capabilities, mirroring a browser without fullscreen support. The
//! detection algorithm, the cached `_supportsFullscreen` / `_names` state,
//! and the property semantics (`enabled` / `fullscreen` / `element` /
//! `changeEventName` / `errorEventName` returning "undefined" when
//! unsupported) are mirrored one-to-one.

use std::sync::{Mutex, MutexGuard};

/// The vendor prefix name mappings discovered by [`supports_fullscreen`].
///
/// Mirrors the module-level `_names` object.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FullscreenNames {
    /// The element method name (`requestFullscreen` / prefixed variant).
    pub request_fullscreen: Option<String>,
    /// The document method name (`exitFullscreen` / prefixed variant).
    pub exit_fullscreen: Option<String>,
    /// The document property name (`fullscreenEnabled` / prefixed variant).
    pub fullscreen_enabled: Option<String>,
    /// The document property name (`fullscreenElement` / prefixed variant).
    pub fullscreen_element: Option<String>,
    /// The fullscreen-change event name.
    pub fullscreenchange: Option<String>,
    /// The fullscreen-error event name.
    pub fullscreenerror: Option<String>,
}

/// The DOM surface probed and driven by the [`Fullscreen`](Fullscreen)
/// facade.
///
/// DEVIATION: replaces the JS `document` / `document.body` global objects.
/// `has_*` mirrors the JS `typeof body[name] === "function"` /
/// `document[name] !== undefined` / `document['on' + name] !== undefined`
/// probes; `invoke_*` / `read_*` mirror the dynamic
/// `element[name]()` / `document[name]` accesses.
pub trait FullscreenDocument: Send {
    /// Whether the element has a `requestFullscreen` (or prefixed /
    /// case-variant) function — mirrors `typeof body[name] === "function"`.
    fn has_element_function(&self, name: &str) -> bool;
    /// Whether the document has a function property — mirrors
    /// `typeof document[name] === "function"`.
    fn has_document_function(&self, name: &str) -> bool;
    /// Whether the document has a (non-function) property — mirrors
    /// `document[name] !== undefined`.
    fn has_document_property(&self, name: &str) -> bool;
    /// Whether the document has an `on<name>` handler property — mirrors
    /// `document['on' + name] !== undefined`.
    fn has_on_property(&self, name: &str) -> bool;
    /// Reads the document `fullscreenEnabled` property — mirrors
    /// `document[_names.fullscreenEnabled]`.
    fn read_enabled(&self, name: &str) -> bool;
    /// Reads the document `fullscreenElement` property — mirrors
    /// `document[_names.fullscreenElement]`; `None` mirrors JS `null`.
    fn read_element(&self, name: &str) -> Option<String>;
    /// Calls the element fullscreen-request function — mirrors
    /// `element[_names.requestFullscreen]({ vrDisplay: vrDevice })`.
    fn invoke_request(&self, name: &str);
    /// Calls the document fullscreen-exit function — mirrors
    /// `document[_names.exitFullscreen]()`.
    fn invoke_exit(&self, name: &str);
}

/// A document with no fullscreen capabilities.
///
/// DEVIATION: the default backend; mirrors a browser that exposes none of
/// the fullscreen API surface (so [`supports_fullscreen`] returns `false`).
#[derive(Debug, Clone, Copy, Default)]
pub struct HeadlessDocument;

impl FullscreenDocument for HeadlessDocument {
    fn has_element_function(&self, _name: &str) -> bool {
        false
    }
    fn has_document_function(&self, _name: &str) -> bool {
        false
    }
    fn has_document_property(&self, _name: &str) -> bool {
        false
    }
    fn has_on_property(&self, _name: &str) -> bool {
        false
    }
    fn read_enabled(&self, _name: &str) -> bool {
        false
    }
    fn read_element(&self, _name: &str) -> Option<String> {
        None
    }
    fn invoke_request(&self, _name: &str) {}
    fn invoke_exit(&self, _name: &str) {}
}

/// The module-level `_supportsFullscreen` / `_names` / document binding.
struct FullscreenState {
    supports: Option<bool>,
    names: FullscreenNames,
    document: Box<dyn FullscreenDocument>,
}

static STATE: Mutex<Option<FullscreenState>> = Mutex::new(None);

fn state_slot() -> MutexGuard<'static, Option<FullscreenState>> {
    STATE.lock().unwrap()
}

/// Resets the fullscreen state to the default (headless) document binding.
///
/// DEVIATION: the JS module state is bound to the browser `document` for
/// the page lifetime; the Rust port allows rebinding (see
/// [`set_document_for_test`]) and this restores the default headless
/// binding with a cleared detection cache.
pub fn reset_document() {
    let mut slot = state_slot();
    *slot = Some(FullscreenState {
        supports: None,
        names: FullscreenNames::default(),
        document: Box::new(HeadlessDocument),
    });
}

fn ensure_state<'a>(slot: &'a mut Option<FullscreenState>) -> &'a mut FullscreenState {
    if slot.is_none() {
        *slot = Some(FullscreenState {
            supports: None,
            names: FullscreenNames::default(),
            document: Box::new(HeadlessDocument),
        });
    }
    slot.as_mut().unwrap()
}

/// Rebinds the document surface probed by the fullscreen facade and clears
/// the cached detection result.
///
/// DEVIATION: no JS analogue (the JS module is bound to the page
/// `document`); provided so a host environment (or tests) can supply the
/// real capability surface.
pub fn set_document_for_test(document: Box<dyn FullscreenDocument>) {
    let mut slot = state_slot();
    let state = ensure_state(&mut slot);
    state.document = document;
    state.supports = None;
    state.names = FullscreenNames::default();
}

/// Browser-independent functions for working with the standard fullscreen
/// API.
///
/// Mirrors the CesiumJS `Fullscreen` namespace; the JS property getters are
/// mirrored as associated functions.
pub struct Fullscreen;

impl Fullscreen {
    /// The element that is currently fullscreen, if any. To simply check if
    /// the browser is in fullscreen mode or not, use
    /// [`Fullscreen::fullscreen`].
    ///
    /// Mirrors the `element` property: `None` covers both the JS
    /// `undefined` (unsupported) result and the JS `null` (supported, no
    /// element) result; use [`Fullscreen::element_raw`] to distinguish.
    pub fn element() -> Option<String> {
        if !Self::supports_fullscreen() {
            return None;
        }
        let slot = state_slot();
        let state = slot.as_ref().unwrap();
        let name = state.names.fullscreen_element.clone().unwrap_or_default();
        state.document.read_element(&name)
    }

    /// Same as [`Fullscreen::element`] but distinguishes the JS
    /// `undefined` (unsupported → outer `None`) from the JS `null`
    /// (supported, no element → `Some(None)`).
    pub fn element_raw() -> Option<Option<String>> {
        if !Self::supports_fullscreen() {
            return None;
        }
        Some(Self::element())
    }

    /// The name of the event on the document that is fired when fullscreen
    /// is entered or exited.
    pub fn change_event_name() -> Option<String> {
        if !Self::supports_fullscreen() {
            return None;
        }
        let slot = state_slot();
        slot.as_ref().unwrap().names.fullscreenchange.clone()
    }

    /// The name of the event that is fired when a fullscreen error occurs.
    pub fn error_event_name() -> Option<String> {
        if !Self::supports_fullscreen() {
            return None;
        }
        let slot = state_slot();
        slot.as_ref().unwrap().names.fullscreenerror.clone()
    }

    /// Determines whether the browser will allow an element to be made
    /// fullscreen, or not.
    ///
    /// Mirrors the `enabled` property: `None` is the JS `undefined`
    /// (unsupported) result.
    pub fn enabled() -> Option<bool> {
        if !Self::supports_fullscreen() {
            return None;
        }
        let slot = state_slot();
        let state = slot.as_ref().unwrap();
        let name = state.names.fullscreen_enabled.clone().unwrap_or_default();
        Some(state.document.read_enabled(&name))
    }

    /// Determines if the browser is currently in fullscreen mode.
    ///
    /// Mirrors the `fullscreen` property: `None` is the JS `undefined`
    /// (unsupported) result; otherwise `element !== null`.
    pub fn fullscreen() -> Option<bool> {
        if !Self::supports_fullscreen() {
            return None;
        }
        Some(Self::element().is_some())
    }

    /// Detects whether the browser supports the standard fullscreen API.
    ///
    /// Port of `Fullscreen.supportsFullscreen`.
    pub fn supports_fullscreen() -> bool {
        let mut slot = state_slot();
        let state = ensure_state(&mut slot);
        if let Some(supports) = state.supports {
            return supports;
        }

        state.supports = Some(false);

        if state.document.has_element_function("requestFullscreen") {
            // go with the unprefixed, standard set of names
            state.names.request_fullscreen = Some("requestFullscreen".to_string());
            state.names.exit_fullscreen = Some("exitFullscreen".to_string());
            state.names.fullscreen_enabled = Some("fullscreenEnabled".to_string());
            state.names.fullscreen_element = Some("fullscreenElement".to_string());
            state.names.fullscreenchange = Some("fullscreenchange".to_string());
            state.names.fullscreenerror = Some("fullscreenerror".to_string());
            state.supports = Some(true);
            return true;
        }

        // check for the correct combination of prefix plus the various
        // names that browsers use
        let prefixes = ["webkit", "moz", "o", "ms", "khtml"];
        let mut supports = false;
        for prefix in prefixes {
            // casing of Fullscreen differs across browsers
            let name = format!("{prefix}RequestFullscreen");
            if state.document.has_element_function(&name) {
                state.names.request_fullscreen = Some(name);
                supports = true;
            } else {
                let name = format!("{prefix}RequestFullScreen");
                if state.document.has_element_function(&name) {
                    state.names.request_fullscreen = Some(name);
                    supports = true;
                }
            }

            // disagreement about whether it's "exit" as per spec, or "cancel"
            let name = format!("{prefix}ExitFullscreen");
            if state.document.has_document_function(&name) {
                state.names.exit_fullscreen = Some(name);
            } else {
                let name = format!("{prefix}CancelFullScreen");
                if state.document.has_document_function(&name) {
                    state.names.exit_fullscreen = Some(name);
                }
            }

            // casing of Fullscreen differs across browsers
            let name = format!("{prefix}FullscreenEnabled");
            if state.document.has_document_property(&name) {
                state.names.fullscreen_enabled = Some(name);
            } else {
                let name = format!("{prefix}FullScreenEnabled");
                if state.document.has_document_property(&name) {
                    state.names.fullscreen_enabled = Some(name);
                }
            }

            // casing of Fullscreen differs across browsers
            let name = format!("{prefix}FullscreenElement");
            if state.document.has_document_property(&name) {
                state.names.fullscreen_element = Some(name);
            } else {
                let name = format!("{prefix}FullScreenElement");
                if state.document.has_document_property(&name) {
                    state.names.fullscreen_element = Some(name);
                }
            }

            // thankfully, event names are all lowercase per spec
            let mut name = format!("{prefix}fullscreenchange");
            // event names do not have 'on' in the front, but the property
            // on the document does
            if state.document.has_on_property(&name) {
                // except on IE
                if prefix == "ms" {
                    name = "MSFullscreenChange".to_string();
                }
                state.names.fullscreenchange = Some(name);
            }

            let mut name = format!("{prefix}fullscreenerror");
            if state.document.has_on_property(&name) {
                // except on IE
                if prefix == "ms" {
                    name = "MSFullscreenError".to_string();
                }
                state.names.fullscreenerror = Some(name);
            }
        }

        state.supports = Some(supports);
        supports
    }

    /// Asynchronously requests the browser to enter fullscreen mode on the
    /// given element. If fullscreen mode is not supported by the browser,
    /// does nothing.
    ///
    /// Port of `Fullscreen.requestFullscreen`.
    ///
    /// DEVIATION: the JS `element` / `vrDevice` arguments have no analogue
    /// in the headless port; the bound document surface is driven directly.
    pub fn request_fullscreen() {
        if !Self::supports_fullscreen() {
            return;
        }
        let slot = state_slot();
        let state = slot.as_ref().unwrap();
        let name = state.names.request_fullscreen.clone().unwrap_or_default();
        state.document.invoke_request(&name);
    }

    /// Asynchronously exits fullscreen mode. If the browser is not
    /// currently in fullscreen, or if fullscreen mode is not supported by
    /// the browser, does nothing.
    ///
    /// Port of `Fullscreen.exitFullscreen`.
    pub fn exit_fullscreen() {
        if !Self::supports_fullscreen() {
            return;
        }
        let slot = state_slot();
        let state = slot.as_ref().unwrap();
        let name = state.names.exit_fullscreen.clone().unwrap_or_default();
        state.document.invoke_exit(&name);
    }

    /// The discovered vendor name mappings.
    ///
    /// Mirrors `Fullscreen._names` (exposed by the JS module for unit
    /// tests).
    pub fn names() -> FullscreenNames {
        let _ = Self::supports_fullscreen();
        let slot = state_slot();
        slot.as_ref().unwrap().names.clone()
    }
}
