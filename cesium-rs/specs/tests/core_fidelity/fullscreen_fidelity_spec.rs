//! Mirror of `packages/engine/Specs/Core/FullscreenSpec.js` (71 lines).
//!
//! Conventions:
//! - Jasmine `it(...)` titles map to assertion blocks inside a single
//!   sequential `#[test]` (the JS module state — cached detection result /
//!   `_names` — is mirrored as Rust global state, so parallel test threads
//!   would race; JS Jasmine runs serially).
//! - JS `undefined` results map to `None`; JS `null` (fullscreen element)
//!   maps to inner `None` via `element_raw`.
//!
//! DEVIATION (mirroring note): the JS spec's supported branch spies on the
//! real `document`; the Rust port binds a mock `FullscreenDocument` via
//! `set_document_for_test` and observes the same name resolution and
//! request/exit call semantics.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use cesium_core::fullscreen::{
    set_document_for_test, reset_document, Fullscreen, FullscreenDocument,
};

/// Mock document exposing the unprefixed, standard fullscreen API surface.
struct MockDocument {
    enabled: bool,
    element: Mutex<Option<String>>,
    requested: Arc<AtomicUsize>,
    exited: Arc<AtomicUsize>,
}

impl FullscreenDocument for MockDocument {
    fn has_element_function(&self, name: &str) -> bool {
        name == "requestFullscreen"
    }
    fn has_document_function(&self, name: &str) -> bool {
        name == "exitFullscreen"
    }
    fn has_document_property(&self, name: &str) -> bool {
        name == "fullscreenEnabled" || name == "fullscreenElement"
    }
    fn has_on_property(&self, name: &str) -> bool {
        name == "fullscreenchange" || name == "fullscreenerror"
    }
    fn read_enabled(&self, _name: &str) -> bool {
        self.enabled
    }
    fn read_element(&self, _name: &str) -> Option<String> {
        self.element.lock().unwrap().clone()
    }
    fn invoke_request(&self, name: &str) {
        assert_eq!(name, "requestFullscreen");
        self.requested.fetch_add(1, Ordering::SeqCst);
    }
    fn invoke_exit(&self, name: &str) {
        assert_eq!(name, "exitFullscreen");
        self.exited.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn fullscreen_spec() {
    // ---- Headless (unsupported) branch: mirrors the JS spec's `else` ----
    // arms (Jasmine runs in a browser, but the assertions are the same
    // whenever supportsFullscreen() is false).
    reset_document();

    // "can tell if fullscreen is supported"
    let supports = Fullscreen::supports_fullscreen();
    assert!(!supports);

    // "can tell if fullscreen is enabled" — undefined when unsupported.
    assert_eq!(Fullscreen::enabled(), None);

    // "can get fullscreen element" — undefined when unsupported.
    assert_eq!(Fullscreen::element_raw(), None);

    // "can tell if the browser is in fullscreen" — undefined.
    assert_eq!(Fullscreen::fullscreen(), None);

    // "can request fullscreen" — no-ops when unsupported.
    Fullscreen::request_fullscreen();
    Fullscreen::exit_fullscreen();

    // "can get the fullscreen change event name" — undefined.
    assert_eq!(Fullscreen::change_event_name(), None);

    // "can get the fullscreen error event name" — undefined.
    assert_eq!(Fullscreen::error_event_name(), None);

    // ---- Supported branch: bind a mock standard-API document ----
    let requested = Arc::new(AtomicUsize::new(0));
    let exited = Arc::new(AtomicUsize::new(0));
    set_document_for_test(Box::new(MockDocument {
        enabled: true,
        element: Mutex::new(None),
        requested: Arc::clone(&requested),
        exited: Arc::clone(&exited),
    }));

    assert!(Fullscreen::supports_fullscreen());

    // Detection resolves the unprefixed, standard set of names.
    let names = Fullscreen::names();
    assert_eq!(names.request_fullscreen.as_deref(), Some("requestFullscreen"));
    assert_eq!(names.exit_fullscreen.as_deref(), Some("exitFullscreen"));
    assert_eq!(names.fullscreen_enabled.as_deref(), Some("fullscreenEnabled"));
    assert_eq!(names.fullscreen_element.as_deref(), Some("fullscreenElement"));
    assert_eq!(names.fullscreenchange.as_deref(), Some("fullscreenchange"));
    assert_eq!(names.fullscreenerror.as_deref(), Some("fullscreenerror"));

    // "enabled" mirrors `document.fullscreenEnabled`.
    assert_eq!(Fullscreen::enabled(), Some(true));

    // "element" is null (inner None) and "fullscreen" is false while no
    // element is fullscreen.
    assert_eq!(Fullscreen::element_raw(), Some(None));
    assert_eq!(Fullscreen::fullscreen(), Some(false));

    // "can request fullscreen" — the resolved names are invoked.
    Fullscreen::request_fullscreen();
    assert_eq!(requested.load(Ordering::SeqCst), 1);
    Fullscreen::exit_fullscreen();
    assert_eq!(exited.load(Ordering::SeqCst), 1);

    // Restore the default headless binding for other tests.
    reset_document();
    assert!(!Fullscreen::supports_fullscreen());
}
