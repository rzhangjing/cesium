//! Ported from `packages/widgets/Source/VRButton/VRButtonViewModel.js`.
//!
//! The view model for the VRButton widget.
//!
//! DEVIATION: the JS view model drives `scene.useWebVR`, uses
//! `Fullscreen` statics, `NoSleep` and screen-orientation locking, and
//! registers a `fullscreenchange` document listener. Scene behavior is
//! injected through the [`VrScene`] trait (the engine wiring is provided
//! by `impl VrScene for cesium_scene::scene::Scene` below: `useWebVR`
//! writes, the `preRender` subscription that tracks orthographic mode,
//! and the camera frustum kind query); headless Rust has no fullscreen /
//! screen-lock APIs, so: fullscreen state comes from the widget-local
//! capability surface (`fullscreen_enabled`/`fullscreen_active`, always
//! `false` headless); `NoSleep`/screen-locking are no-ops; the
//! `fullscreenchange` listener is replaced by
//! [`VrButtonViewModel::sync_fullscreen_state`]. See
//! `docs/deviations.md`.

use std::cell::RefCell;
use std::rc::Rc;

use cesium_core::developer_error::throw_developer_error;
use cesium_core::event::Event;
use cesium_core::event_helper::EventHelper;
use cesium_core::julian_date::JulianDate;
use cesium_scene::camera::CameraProjection;
use cesium_scene::scene::Scene;

use crate::command::Command;
use crate::create_command::create_command_with_can_execute_provider;
use crate::knockout::{
    fullscreen_active, fullscreen_enabled, get_element, ElementOrId, MockDocument,
    MockDomElement,
};
use crate::observables::ObservableCell;

/// The Rust analogue of the scene behavior used by the VR button
/// (`scene.useWebVR` writes, the `scene.preRender` subscription and the
/// `scene.camera.frustum instanceof OrthographicFrustum` query).
pub trait VrScene {
    /// Mirrors assigning `scene.useWebVR`.
    fn set_use_web_vr(&self, value: bool);
    /// Mirrors `scene.preRender` (the view model subscribes to it to
    /// refresh the orthographic flag every frame).
    fn pre_render(&self) -> &Event<JulianDate>;
    /// Mirrors `scene.camera.frustum instanceof OrthographicFrustum`.
    fn camera_is_orthographic(&self) -> bool;
}

/// The engine wiring of [`VrScene`] on the real scene. DEVIATION:
/// `useWebVR` is a flag only (the stereo/VR frustum handling has no
/// headless analogue, see [`Scene::set_use_web_vr`]).
impl VrScene for Scene {
    fn set_use_web_vr(&self, value: bool) {
        Scene::set_use_web_vr(self, value);
    }

    fn pre_render(&self) -> &Event<JulianDate> {
        Scene::pre_render(self)
    }

    fn camera_is_orthographic(&self) -> bool {
        self.camera().projection_type() == CameraProjection::Orthographic
    }
}

/// The view model for the VRButton widget.
pub struct VrButtonViewModel {
    scene: Rc<dyn VrScene>,
    /// `isEnabled` knockout observable (initialized from `Fullscreen.enabled`).
    is_enabled: ObservableCell<bool>,
    /// `isVRMode` knockout observable.
    is_vr_mode: ObservableCell<bool>,
    /// `isOrthographic` knockout observable.
    is_orthographic: ObservableCell<bool>,
    command: Command,
    vr_element: Rc<RefCell<MockDomElement>>,
    /// The JS `_eventHelper` managing the `preRender` subscription.
    event_helper: EventHelper,
    destroyed: bool,
}

/// Mirrors the JS `toggleVR(viewModel, scene, isVRMode, isOrthographic)`
/// helper.
fn toggle_vr(
    scene: &Rc<dyn VrScene>,
    is_vr_mode: &ObservableCell<bool>,
    is_orthographic: &ObservableCell<bool>,
) {
    if is_orthographic.get() {
        return;
    }

    if is_vr_mode.get() {
        scene.set_use_web_vr(false);
        // DEVIATION: the JS unlocks the screen orientation, disables
        // NoSleep and calls `Fullscreen.exitFullscreen()`; all three are
        // no-ops headless (no screen/DOM/fullscreen API).
        is_vr_mode.set(false);
    } else {
        // DEVIATION: the JS requests fullscreen on `viewModel._vrElement`
        // when not already fullscreen, enables NoSleep and locks the
        // screen orientation to landscape; all no-ops headless.
        scene.set_use_web_vr(true);
        is_vr_mode.set(true);
    }
}

impl VrButtonViewModel {
    /// Creates a new VR button view model.
    ///
    /// Mirrors `new VRButtonViewModel(scene, vrElement)`; `vr_element`
    /// defaults to the document body (`getElement(vrElement) ??
    /// document.body`). The JS `scene is required.` DeveloperError is
    /// mirrored by [`VrButtonViewModel::try_new`] with `None`.
    pub fn new(
        scene: Rc<dyn VrScene>,
        vr_element: Option<ElementOrId>,
        document: &MockDocument,
    ) -> Self {
        Self::try_new(Some(scene), vr_element, document)
    }

    /// Creates a new VR button view model from an optional scene,
    /// mirroring the JS undefined-scene DeveloperError check.
    ///
    /// # Panics
    /// Panics with a `DeveloperError` when `scene` is `None`.
    pub fn try_new(
        scene: Option<Rc<dyn VrScene>>,
        vr_element: Option<ElementOrId>,
        document: &MockDocument,
    ) -> Self {
        #[cfg(debug_assertions)]
        if scene.is_none() {
            throw_developer_error("scene is required.");
        }
        let scene = scene.expect("scene is required.");

        let is_enabled = ObservableCell::new(fullscreen_enabled());
        let is_vr_mode = ObservableCell::new(false);
        let is_orthographic = ObservableCell::new(false);

        // this._eventHelper = new EventHelper();
        // this._eventHelper.add(scene.preRender, function () {
        //   isOrthographic(scene.camera.frustum instanceof OrthographicFrustum);
        // });
        let orthographic_for_listener = is_orthographic.clone();
        let scene_for_listener = Rc::clone(&scene);
        let removal = scene.pre_render().add_listener(move |_time: &JulianDate| {
            orthographic_for_listener.set(scene_for_listener.camera_is_orthographic());
        });
        let mut event_helper = EventHelper::new();
        let scene_for_removal = Rc::clone(&scene);
        let mut removal_slot = Some(removal);
        event_helper.add_removal(Box::new(move || {
            if let Some(removal) = removal_slot.take() {
                removal.call(scene_for_removal.pre_render());
            }
        }));

        // DEVIATION: JS passes `knockout.getObservable(this, "isVREnabled")`
        // as a live canExecute observable; the Rust port uses a computed
        // canExecute provider over the same shared observable with
        // identical read-time semantics.
        let command_scene = Rc::clone(&scene);
        let command_is_vr_mode = is_vr_mode.clone();
        let command_is_orthographic = is_orthographic.clone();
        let command_can_execute = is_enabled.clone();
        let command = create_command_with_can_execute_provider(
            move |_| {
                toggle_vr(
                    &command_scene,
                    &command_is_vr_mode,
                    &command_is_orthographic,
                );
                None
            },
            move || command_can_execute.get(),
        );

        Self {
            scene,
            is_enabled,
            is_vr_mode,
            is_orthographic,
            command,
            vr_element: Rc::new(RefCell::new(
                get_element(document, vr_element.as_ref())
                    .unwrap_or_else(|| document.body().clone()),
            )),
            event_helper,
            destroyed: false,
        }
    }

    /// Gets whether or not VR mode is active (mirrors the `isVRMode`
    /// defineProperty getter).
    pub fn is_vr_mode(&self) -> bool {
        self.is_vr_mode.get()
    }

    /// Gets whether or not VR functionality should be enabled (mirrors
    /// the `isVREnabled` defineProperty getter).
    pub fn is_vr_enabled(&self) -> bool {
        self.is_enabled.get()
    }

    /// Sets whether or not VR functionality should be enabled, mirroring
    /// `isEnabled(value && Fullscreen.enabled)`.
    pub fn set_is_vr_enabled(&self, value: bool) {
        self.is_enabled.set(value && fullscreen_enabled());
    }

    /// Gets the tooltip (`tooltip` computed).
    pub fn tooltip(&self) -> String {
        if !self.is_enabled.get() {
            return "VR mode is unavailable".to_string();
        }
        if self.is_vr_mode.get() {
            "Exit VR mode".to_string()
        } else {
            "Enter VR mode".to_string()
        }
    }

    /// Gets whether the camera is currently in orthographic mode
    /// (mirrors the `_isOrthographic` defineProperty getter). The value
    /// is refreshed by the `preRender` subscription installed in the
    /// constructor, mirroring the JS.
    pub fn is_orthographic(&self) -> bool {
        self.is_orthographic.get()
    }

    /// Gets the Command to toggle VR mode.
    pub fn command(&self) -> &Command {
        &self.command
    }

    /// Gets the scene being used.
    ///
    /// DEVIATION: mirrors the JS `scene` capture; returns the injected
    /// [`VrScene`] handle instead of a `Scene`.
    pub fn scene(&self) -> &Rc<dyn VrScene> {
        &self.scene
    }

    /// Gets the element to place into VR mode when the corresponding
    /// button is pressed (mirrors the `vrElement` getter).
    pub fn vr_element(&self) -> MockDomElement {
        self.vr_element.borrow().clone()
    }

    /// Sets the element to place into VR mode (mirrors the `vrElement`
    /// setter).
    ///
    /// DEVIATION: the JS `value must be a valid Element.` DeveloperError
    /// is enforced by the [`MockDomElement`] type; the `None` case is
    /// mirrored by [`VrButtonViewModel::try_set_vr_element`].
    pub fn set_vr_element(&self, value: MockDomElement) {
        *self.vr_element.borrow_mut() = value;
    }

    /// Sets the VR element from an optional element, mirroring the JS
    /// non-Element DeveloperError check.
    ///
    /// # Panics
    /// Panics with a `DeveloperError` when `value` is `None`.
    pub fn try_set_vr_element(&self, value: Option<MockDomElement>) {
        #[cfg(debug_assertions)]
        if value.is_none() {
            throw_developer_error("value must be a valid Element.");
        }
        *self.vr_element.borrow_mut() = value.expect("value must be a valid Element.");
    }

    /// Refreshes the VR state after a fullscreen change.
    ///
    /// DEVIATION: replaces the JS `fullscreenchange` document listener
    /// (`this._callback`): when fullscreen is no longer active while VR
    /// mode is on, VR mode is switched off.
    pub fn sync_fullscreen_state(&self) {
        if !fullscreen_active() && self.is_vr_mode.get() {
            self.scene.set_use_web_vr(false);
            // DEVIATION: the JS additionally unlocks the screen
            // orientation and disables NoSleep; both are no-ops headless.
            self.is_vr_mode.set(false);
        }
    }

    /// Returns `true` if the object has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.destroyed
    }

    /// Destroys the view model. Should be called to properly clean up the
    /// view model when it is no longer needed.
    ///
    /// Mirrors `destroy()`: `this._eventHelper.removeAll()` removes the
    /// `preRender` subscription. DEVIATION: the JS
    /// `document.removeEventListener(Fullscreen.changeEventName, ...)`
    /// has no analogue (the `fullscreenchange` listener is modeled by
    /// [`VrButtonViewModel::sync_fullscreen_state`]).
    ///
    /// # Panics
    /// Panics with a `DeveloperError` when destroyed twice, mirroring
    /// `destroyObject`.
    pub fn destroy(&mut self) {
        if self.destroyed {
            throw_developer_error("This object has been destroyed.");
        }
        self.event_helper.remove_all();
        self.destroyed = true;
    }
}
