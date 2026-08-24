//! Ported from `packages/widgets/Source/ProjectionPicker/ProjectionPickerViewModel.js`.
//!
//! The view model for the `ProjectionPicker` widget: allows switching
//! between perspective and orthographic projections.
//!
//! DEVIATION: the JS view model operates on a real `Scene` / `Camera`
//! (frustum type checks, `switchTo*Frustum`, `camera._currentFlight`);
//! the widgets layer is GPU-free, so those capabilities are injected
//! through the [`ProjectionScene`] trait. The JS `morphComplete`
//! listener receives `(transitioner, oldMode, newMode, isMorphing)`;
//! the Rust event payload is only the new mode (the other arguments are
//! unused by the view model).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use cesium_core::event::{Event, RemoveCallback};
use cesium_core::event_helper::EventHelper;
use cesium_scene::scene_mode::SceneMode;

use crate::command::Command;
use crate::observables::ObservableCell;

/// The scene abstraction required by [`ProjectionPickerViewModel`],
/// mirroring the parts of CesiumJS `Scene` / `Camera` the view model
/// touches.
pub trait ProjectionScene {
    /// The current scene mode (`scene.mode`).
    fn mode(&self) -> SceneMode;
    /// The `morphComplete` event. DEVIATION: the JS payload is
    /// `(transitioner, oldMode, newMode, isMorphing)`; the Rust payload
    /// is the new mode only.
    fn morph_complete(&self) -> &Event<SceneMode>;
    /// The `preRender` event.
    fn pre_render(&self) -> &Event<()>;
    /// Whether the camera frustum is an `OrthographicFrustum`
    /// (`scene.camera.frustum instanceof OrthographicFrustum`).
    fn is_orthographic_frustum(&self) -> bool;
    /// Switches the camera to a perspective frustum
    /// (`camera.switchToPerspectiveFrustum()`).
    fn switch_to_perspective_frustum(&self);
    /// Switches the camera to an orthographic frustum
    /// (`camera.switchToOrthographicFrustum()`).
    fn switch_to_orthographic_frustum(&self);
    /// Whether a camera flight is in progress
    /// (`defined(scene.camera._currentFlight)`).
    fn flight_in_progress(&self) -> bool;
}

/// The view model for the `ProjectionPicker` widget.
pub struct ProjectionPickerViewModel {
    scene: Rc<dyn ProjectionScene>,
    /// Whether the scene is currently using an orthographic projection
    /// (JS private knockout-tracked `_orthographic`).
    orthographic: Rc<Cell<bool>>,
    /// Whether a camera flight is in progress (JS private
    /// knockout-tracked `_flightInProgress`).
    flight_in_progress: Rc<Cell<bool>>,
    /// Gets or sets whether the button drop-down is currently visible.
    drop_down_visible: ObservableCell<bool>,
    /// Gets or sets the perspective projection tooltip.
    tooltip_perspective: RefCell<String>,
    /// Gets or sets the orthographic projection tooltip.
    tooltip_orthographic: RefCell<String>,
    /// Gets or sets the current scene mode.
    scene_mode: ObservableCell<SceneMode>,
    /// The command to toggle the drop down box.
    toggle_drop_down: Command,
    /// The command to switch to a perspective projection.
    switch_to_perspective: Command,
    /// The command to switch to orthographic projection.
    switch_to_orthographic: Command,
    event_helper: EventHelper,
    /// Whether [`ProjectionPickerViewModel::destroy`] has been called
    /// (JS `destroyObject`).
    destroyed: Cell<bool>,
}

impl ProjectionPickerViewModel {
    /// Creates a new view model for the given scene.
    ///
    /// Mirrors `new ProjectionPickerViewModel(scene)`; the JS
    /// `scene is required.` DeveloperError is enforced by the Rust type
    /// system (the scene argument is mandatory).
    pub fn new(scene: Rc<dyn ProjectionScene>) -> Self {
        // this._orthographic = scene.camera.frustum instanceof OrthographicFrustum;
        let orthographic = Rc::new(Cell::new(scene.is_orthographic_frustum()));
        // this._flightInProgress = false;
        let flight_in_progress = Rc::new(Cell::new(false));

        let drop_down_visible = ObservableCell::new(false);
        let tooltip_perspective = RefCell::new(String::from("Perspective Projection"));
        let tooltip_orthographic = RefCell::new(String::from("Orthographic Projection"));
        let scene_mode = ObservableCell::new(scene.mode());

        // this._toggleDropDown = createCommand(function () { ... });
        let scene_mode_for_toggle = scene_mode.clone();
        let flight_for_toggle = Rc::clone(&flight_in_progress);
        let drop_down_for_toggle = drop_down_visible.clone();
        let toggle_drop_down = Command::new(
            move |_| {
                if scene_mode_for_toggle.get() == SceneMode::Scene2D || flight_for_toggle.get() {
                    return None;
                }
                drop_down_for_toggle.set(!drop_down_for_toggle.get());
                None
            },
            true,
        );

        // this._eventHelper = new EventHelper();
        let mut event_helper = EventHelper::new();

        // this._eventHelper.add(scene.morphComplete, function (...) { ... });
        let scene_mode_for_morph = scene_mode.clone();
        let orthographic_for_morph = Rc::clone(&orthographic);
        let scene_for_morph = Rc::clone(&scene);
        let morph_removal: RemoveCallback<SceneMode> =
            scene.morph_complete().add_listener(move |new_mode: &SceneMode| {
                scene_mode_for_morph.set(*new_mode);
                orthographic_for_morph.set(
                    *new_mode == SceneMode::Scene2D || scene_for_morph.is_orthographic_frustum(),
                );
            });
        let mut morph_removal_slot = Some(morph_removal);
        let scene_for_morph_removal = Rc::clone(&scene);
        event_helper.add_removal(Box::new(move || {
            if let Some(removal) = morph_removal_slot.take() {
                removal.call(scene_for_morph_removal.morph_complete());
            }
        }));

        // this._eventHelper.add(scene.preRender, function () { ... });
        let flight_for_pre_render = Rc::clone(&flight_in_progress);
        let scene_for_pre_render = Rc::clone(&scene);
        let pre_render_removal: RemoveCallback<()> = scene.pre_render().add_listener(move |_| {
            flight_for_pre_render.set(scene_for_pre_render.flight_in_progress());
        });
        let mut pre_render_removal_slot = Some(pre_render_removal);
        let scene_for_pre_render_removal = Rc::clone(&scene);
        event_helper.add_removal(Box::new(move || {
            if let Some(removal) = pre_render_removal_slot.take() {
                removal.call(scene_for_pre_render_removal.pre_render());
            }
        }));

        // this._switchToPerspective = createCommand(function () { ... });
        let scene_mode_for_perspective = scene_mode.clone();
        let scene_for_perspective = Rc::clone(&scene);
        let orthographic_for_perspective = Rc::clone(&orthographic);
        let drop_down_for_perspective = drop_down_visible.clone();
        let switch_to_perspective = Command::new(
            move |_| {
                if scene_mode_for_perspective.get() == SceneMode::Scene2D {
                    return None;
                }
                scene_for_perspective.switch_to_perspective_frustum();
                orthographic_for_perspective.set(false);
                drop_down_for_perspective.set(false);
                None
            },
            true,
        );

        // this._switchToOrthographic = createCommand(function () { ... });
        let scene_mode_for_orthographic = scene_mode.clone();
        let scene_for_orthographic = Rc::clone(&scene);
        let orthographic_for_orthographic = Rc::clone(&orthographic);
        let drop_down_for_orthographic = drop_down_visible.clone();
        let switch_to_orthographic = Command::new(
            move |_| {
                if scene_mode_for_orthographic.get() == SceneMode::Scene2D {
                    return None;
                }
                scene_for_orthographic.switch_to_orthographic_frustum();
                orthographic_for_orthographic.set(true);
                drop_down_for_orthographic.set(false);
                None
            },
            true,
        );

        Self {
            scene,
            orthographic,
            flight_in_progress,
            drop_down_visible,
            tooltip_perspective,
            tooltip_orthographic,
            scene_mode,
            toggle_drop_down,
            switch_to_perspective,
            switch_to_orthographic,
            event_helper,
            destroyed: Cell::new(false),
        }
    }

    /// Gets the scene.
    pub fn scene(&self) -> &Rc<dyn ProjectionScene> {
        &self.scene
    }

    /// Gets or sets whether the button drop-down is currently visible.
    pub fn drop_down_visible(&self) -> bool {
        self.drop_down_visible.get()
    }

    /// Sets whether the button drop-down is currently visible.
    pub fn set_drop_down_visible(&self, value: bool) {
        self.drop_down_visible.set(value);
    }

    /// Gets the perspective projection tooltip.
    pub fn tooltip_perspective(&self) -> String {
        self.tooltip_perspective.borrow().clone()
    }

    /// Sets the perspective projection tooltip.
    pub fn set_tooltip_perspective(&self, value: String) {
        *self.tooltip_perspective.borrow_mut() = value;
    }

    /// Gets the orthographic projection tooltip.
    pub fn tooltip_orthographic(&self) -> String {
        self.tooltip_orthographic.borrow().clone()
    }

    /// Sets the orthographic projection tooltip.
    pub fn set_tooltip_orthographic(&self, value: String) {
        *self.tooltip_orthographic.borrow_mut() = value;
    }

    /// Gets the currently active tooltip (JS knockout computed
    /// `selectedTooltip`).
    pub fn selected_tooltip(&self) -> String {
        if self.orthographic.get() {
            self.tooltip_orthographic()
        } else {
            self.tooltip_perspective()
        }
    }

    /// Gets the current scene mode.
    pub fn scene_mode(&self) -> SceneMode {
        self.scene_mode.get()
    }

    /// Gets the command to toggle the drop down box.
    pub fn toggle_drop_down(&self) -> &Command {
        &self.toggle_drop_down
    }

    /// Gets the command to switch to a perspective projection.
    pub fn switch_to_perspective(&self) -> &Command {
        &self.switch_to_perspective
    }

    /// Gets the command to switch to orthographic projection.
    pub fn switch_to_orthographic(&self) -> &Command {
        &self.switch_to_orthographic
    }

    /// Gets whether the scene is currently using an orthographic
    /// projection.
    pub fn is_orthographic_projection(&self) -> bool {
        self.orthographic.get()
    }

    /// Gets whether a camera flight is currently in progress (JS private
    /// `_flightInProgress`, knockout-tracked).
    pub fn flight_in_progress(&self) -> bool {
        self.flight_in_progress.get()
    }

    /// Returns whether this view model has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.destroyed.get()
    }

    /// Destroys the view model, removing all event subscriptions.
    pub fn destroy(&mut self) {
        self.event_helper.remove_all();
        self.destroyed.set(true);
    }
}
