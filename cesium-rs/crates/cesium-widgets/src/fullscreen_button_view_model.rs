//! Ported from `packages/widgets/Source/FullscreenButton/FullscreenButtonViewModel.js`.
//!
//! The view model for the FullscreenButton widget.
//!
//! The engine [`Fullscreen`] facade is wired as the default
//! [`FullscreenSource`] (see [`FullscreenButtonViewModel::with_engine_fullscreen`]).
//!
//! DEVIATION: the JS view model reads the `Fullscreen` statics directly
//! and operates on DOM elements (`getElement`, `document.body`,
//! `addEventListener` for `fullscreenchange`). The widget layer has no
//! DOM, so the fullscreen source is injected through the
//! [`FullscreenSource`] trait (bound to the engine [`Fullscreen`] by
//! default) and elements are modeled as [`FullscreenElement`]; the
//! `fullscreenchange` listener is replaced by
//! [`FullscreenButtonViewModel::sync_fullscreen_state`] (the widget layer
//! invokes it when the fullscreen state changes). See
//! `docs/deviations.md`.

use std::cell::RefCell;
use std::rc::Rc;

use cesium_core::fullscreen::Fullscreen;

use crate::command::Command;
use crate::observables::ObservableCell;

/// The Rust analogue of the engine's static `Fullscreen` object.
pub trait FullscreenSource {
    /// Mirrors `Fullscreen.enabled`: whether the browser/device supports
    /// fullscreen.
    fn enabled(&self) -> bool;
    /// Mirrors `Fullscreen.fullscreen`: whether fullscreen is currently
    /// active.
    fn fullscreen(&self) -> bool;
    /// Mirrors `Fullscreen.requestFullscreen(element)`.
    fn request_fullscreen(&self);
    /// Mirrors `Fullscreen.exitFullscreen()`.
    fn exit_fullscreen(&self);
}

/// Binds the engine [`Fullscreen`] facade to [`FullscreenSource`].
///
/// DEVIATION: the JS view model reads the `Fullscreen` statics directly;
/// the port keeps the source injected through [`FullscreenSource`] (the
/// widget layer stays DOM-free and testable) and exposes the engine
/// facade through this impl. The JS `undefined` results of the
/// `enabled` / `fullscreen` properties on unsupported browsers are
/// mirrored as `false` (both are falsy in the JS knockout observables).
impl FullscreenSource for Fullscreen {
    fn enabled(&self) -> bool {
        Fullscreen::enabled().unwrap_or(false)
    }

    fn fullscreen(&self) -> bool {
        Fullscreen::fullscreen().unwrap_or(false)
    }

    fn request_fullscreen(&self) {
        Fullscreen::request_fullscreen();
    }

    fn exit_fullscreen(&self) {
        Fullscreen::exit_fullscreen();
    }
}

/// The Rust analogue of the HTML element placed into fullscreen mode.
///
/// DEVIATION: elements are identified by their DOM role (body) or id
/// (`getElement` resolution) instead of a live `Element` reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FullscreenElement {
    /// The document body (`document.body`, the default).
    Body,
    /// An element resolved by id (`getElement(id)`).
    Id(String),
}

/// The view model for the FullscreenButton widget.
pub struct FullscreenButtonViewModel {
    source: Rc<dyn FullscreenSource>,
    /// `tmpIsFullscreen` knockout observable.
    is_fullscreen: ObservableCell<bool>,
    /// `tmpIsEnabled` knockout observable.
    is_fullscreen_enabled: ObservableCell<bool>,
    command: Command,
    fullscreen_element: Rc<RefCell<FullscreenElement>>,
    destroyed: bool,
}

impl FullscreenButtonViewModel {
    /// Creates a new fullscreen button view model.
    ///
    /// Mirrors `new FullscreenButtonViewModel(fullscreenElement,
    /// container)`; `fullscreen_element` defaults to the document body
    /// (`getElement(fullscreenElement) ?? ownerDocument.body`), and the
    /// DOM `container` argument has no observable semantics and is dropped
    /// (DEVIATION).
    pub fn new(
        source: Rc<dyn FullscreenSource>,
        fullscreen_element: Option<FullscreenElement>,
    ) -> Self {
        let tmp_is_fullscreen = ObservableCell::new(source.fullscreen());
        let tmp_is_enabled = ObservableCell::new(source.enabled());

        // DEVIATION: the JS command canExecute is
        // `knockout.getObservable(this, "isFullscreenEnabled")`; the Rust
        // port uses a computed canExecute provider over the same shared
        // observable with identical read-time semantics.
        let command_source = Rc::clone(&source);
        let command_is_fullscreen = tmp_is_fullscreen.clone();
        let command_can_execute = tmp_is_enabled.clone();
        let command = Command::new_with_can_execute_provider(
            move |_| {
                if command_is_fullscreen.get() {
                    command_source.exit_fullscreen();
                } else {
                    command_source.request_fullscreen();
                }
                None
            },
            move || command_can_execute.get(),
        );

        Self {
            source,
            is_fullscreen: tmp_is_fullscreen,
            is_fullscreen_enabled: tmp_is_enabled,
            command,
            fullscreen_element: Rc::new(RefCell::new(
                fullscreen_element.unwrap_or(FullscreenElement::Body),
            )),
            destroyed: false,
        }
    }

    /// Mirrors `new FullscreenButtonViewModel(fullscreenElement, container)`
    /// bound to the engine [`Fullscreen`] facade (the JS reads the
    /// `Fullscreen` statics directly); the DOM `container` argument has no
    /// observable semantics and is dropped (DEVIATION).
    pub fn with_engine_fullscreen(
        fullscreen_element: Option<FullscreenElement>,
    ) -> Self {
        Self::new(
            Rc::new(Fullscreen) as Rc<dyn FullscreenSource>,
            fullscreen_element,
        )
    }

    /// Gets whether or not fullscreen mode is active.
    pub fn is_fullscreen(&self) -> bool {
        self.is_fullscreen.get()
    }

    /// Gets whether or not fullscreen functionality should be enabled.
    pub fn is_fullscreen_enabled(&self) -> bool {
        self.is_fullscreen_enabled.get()
    }

    /// Sets whether or not fullscreen functionality should be enabled,
    /// mirroring `isFullscreenEnabled = value && Fullscreen.enabled`.
    pub fn set_is_fullscreen_enabled(&self, value: bool) {
        self.is_fullscreen_enabled
            .set(value && self.source.enabled());
    }

    /// Gets the tooltip (`tooltip` computed).
    pub fn tooltip(&self) -> String {
        if !self.is_fullscreen_enabled() {
            return "Full screen unavailable".to_string();
        }
        if self.is_fullscreen.get() {
            "Exit full screen".to_string()
        } else {
            "Full screen".to_string()
        }
    }

    /// Gets the Command to toggle fullscreen mode.
    pub fn command(&self) -> &Command {
        &self.command
    }

    /// Gets the element to place into fullscreen mode when the
    /// corresponding button is pressed.
    pub fn fullscreen_element(&self) -> FullscreenElement {
        self.fullscreen_element.borrow().clone()
    }

    /// Sets the element to place into fullscreen mode.
    ///
    /// DEVIATION: the JS `value must be a valid Element.` DeveloperError
    /// is mostly enforced by the [`FullscreenElement`] type; the `None`
    /// (non-Element) case is mirrored by
    /// [`FullscreenButtonViewModel::try_set_fullscreen_element`].
    pub fn set_fullscreen_element(&self, value: FullscreenElement) {
        *self.fullscreen_element.borrow_mut() = value;
    }

    /// Sets the fullscreen element from an optional element, mirroring
    /// the JS non-Element DeveloperError check.
    ///
    /// # Panics
    /// Panics with a `DeveloperError` when `value` is `None`.
    pub fn try_set_fullscreen_element(&self, value: Option<FullscreenElement>) {
        #[cfg(debug_assertions)]
        if value.is_none() {
            cesium_core::developer_error::throw_developer_error(
                "value must be a valid Element.",
            );
        }
        *self.fullscreen_element.borrow_mut() =
            value.expect("value must be a valid Element.");
    }

    /// Refreshes the tracked fullscreen state from the source.
    ///
    /// DEVIATION: replaces the JS `fullscreenchange` document listener
    /// (`tmpIsFullscreen(Fullscreen.fullscreen)`).
    pub fn sync_fullscreen_state(&self) {
        self.is_fullscreen.set(self.source.fullscreen());
    }

    /// Returns `true` if the object has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.destroyed
    }

    /// Destroys the view model. Should be called to properly clean up the
    /// view model when it is no longer needed.
    ///
    /// DEVIATION: the JS `document.removeEventListener` has no analogue
    /// (no DOM listener is registered); see module docs.
    pub fn destroy(&mut self) {
        self.destroyed = true;
    }
}
