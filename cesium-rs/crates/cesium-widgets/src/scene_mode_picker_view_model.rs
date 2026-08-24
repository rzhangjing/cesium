//! Ported from `packages/widgets/Source/SceneModePicker/SceneModePickerViewModel.js`.
//!
//! The view model for the `SceneModePicker` widget: allows the user to
//! switch between 3D, 2D, and Columbus View scene modes.
//!
//! DEVIATION: the JS view model operates on a real `Scene`; the widgets
//! layer is GPU-free, so the scene is injected through the
//! [`MorphableScene`] trait (the same dependency-injection style used by
//! the other widget view models); the engine wiring is provided by
//! `impl MorphableScene for cesium_scene::scene::Scene` below. The JS
//! `morphStart` listener receives `(transitioner, oldMode, newMode,
//! isMorphing)`; the Rust event payload is only the new mode (the other
//! arguments are unused by the view model).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use cesium_core::developer_error::throw_developer_error;
use cesium_core::event::Event;
use cesium_core::event_helper::EventHelper;
use cesium_scene::scene::Scene;
use cesium_scene::scene_mode::SceneMode;

use crate::command::Command;
use crate::observables::ObservableCell;

/// The scene abstraction required by [`SceneModePickerViewModel`],
/// mirroring the parts of CesiumJS `Scene` the view model touches.
pub trait MorphableScene {
    /// The current scene mode (`scene.mode`).
    fn mode(&self) -> SceneMode;
    /// The `morphStart` event. DEVIATION: the JS payload is
    /// `(transitioner, oldMode, newMode, isMorphing)`; the Rust payload is
    /// the new mode only.
    fn morph_start(&self) -> &Event<SceneMode>;
    /// Starts morphing to 2D (`scene.morphTo2D(duration)`).
    fn morph_to_2d(&self, duration: f64);
    /// Starts morphing to 3D (`scene.morphTo3D(duration)`).
    fn morph_to_3d(&self, duration: f64);
    /// Starts morphing to Columbus View
    /// (`scene.morphToColumbusView(duration)`).
    fn morph_to_columbus_view(&self, duration: f64);
}

/// The engine wiring of [`MorphableScene`] on the real scene.
/// DEVIATION: the scene morph completes synchronously (mode set +
/// `morphStart` raised, see [`Scene::morph_to_2d`]) where the JS spreads
/// the transition across frames through the `SceneTransitioner`.
impl MorphableScene for Scene {
    fn mode(&self) -> SceneMode {
        Scene::mode(self)
    }

    fn morph_start(&self) -> &Event<SceneMode> {
        Scene::morph_start(self)
    }

    fn morph_to_2d(&self, duration: f64) {
        Scene::morph_to_2d(self, duration);
    }

    fn morph_to_3d(&self, duration: f64) {
        Scene::morph_to_3d(self, duration);
    }

    fn morph_to_columbus_view(&self, duration: f64) {
        Scene::morph_to_columbus_view(self, duration);
    }
}

/// The view model for the `SceneModePicker` widget.
pub struct SceneModePickerViewModel {
    scene: Rc<dyn MorphableScene>,
    event_helper: EventHelper,
    duration: Rc<Cell<f64>>,
    scene_mode: ObservableCell<SceneMode>,
    drop_down_visible: ObservableCell<bool>,
    tooltip_2d: RefCell<String>,
    tooltip_3d: RefCell<String>,
    tooltip_columbus_view: RefCell<String>,
    toggle_drop_down: Command,
    morph_to_2d: Command,
    morph_to_3d: Command,
    morph_to_columbus_view: Command,
    destroyed: Cell<bool>,
}

impl SceneModePickerViewModel {
    /// Creates a new scene mode picker view model, mirroring
    /// `new SceneModePickerViewModel(scene, duration)`. `duration`
    /// defaults to `2.0` when not supplied.
    pub fn new(scene: Rc<dyn MorphableScene>, duration: Option<f64>) -> Self {
        let scene_mode = ObservableCell::new(scene.mode());
        let drop_down_visible = ObservableCell::new(false);

        // const morphStart = function (transitioner, oldMode, newMode, isMorphing) {
        //   that.sceneMode = newMode;
        //   that.dropDownVisible = false;
        // };
        // this._eventHelper = new EventHelper();
        // this._eventHelper.add(scene.morphStart, morphStart);
        let scene_mode_clone = scene_mode.clone();
        let drop_down_clone = drop_down_visible.clone();
        let removal = scene.morph_start().add_listener(move |new_mode: &SceneMode| {
            scene_mode_clone.set(*new_mode);
            drop_down_clone.set(false);
        });
        let mut event_helper = EventHelper::new();
        let scene_for_removal = Rc::clone(&scene);
        let mut removal_slot = Some(removal);
        event_helper.add_removal(Box::new(move || {
            if let Some(removal) = removal_slot.take() {
                removal.call(scene_for_removal.morph_start());
            }
        }));

        let duration_cell = Rc::new(Cell::new(duration.unwrap_or(2.0)));

        // this._toggleDropDown = createCommand(function () {
        //   that.dropDownVisible = !that.dropDownVisible;
        // });
        let toggle_cell = drop_down_visible.clone();
        let toggle_drop_down = Command::new(
            move |_| {
                toggle_cell.set(!toggle_cell.get());
                None
            },
            true,
        );

        // this._morphTo2D = createCommand(function () { scene.morphTo2D(that._duration); });
        let scene_for_2d = Rc::clone(&scene) as Rc<dyn MorphableScene>;
        let duration_for_2d = Rc::clone(&duration_cell);
        let morph_to_2d = Command::new(
            move |_| {
                scene_for_2d.morph_to_2d(duration_for_2d.get());
                None
            },
            true,
        );

        let scene_for_3d = Rc::clone(&scene) as Rc<dyn MorphableScene>;
        let duration_for_3d = Rc::clone(&duration_cell);
        let morph_to_3d = Command::new(
            move |_| {
                scene_for_3d.morph_to_3d(duration_for_3d.get());
                None
            },
            true,
        );

        let scene_for_cv = Rc::clone(&scene) as Rc<dyn MorphableScene>;
        let duration_for_cv = Rc::clone(&duration_cell);
        let morph_to_columbus_view = Command::new(
            move |_| {
                scene_for_cv.morph_to_columbus_view(duration_for_cv.get());
                None
            },
            true,
        );

        Self {
            scene,
            event_helper,
            duration: duration_cell,
            scene_mode,
            drop_down_visible,
            tooltip_2d: RefCell::new("2D".to_string()),
            tooltip_3d: RefCell::new("3D".to_string()),
            tooltip_columbus_view: RefCell::new("Columbus View".to_string()),
            toggle_drop_down,
            morph_to_2d,
            morph_to_3d,
            morph_to_columbus_view,
            destroyed: Cell::new(false),
        }
    }

    /// Gets the scene, mirroring the readonly `scene` property.
    pub fn scene(&self) -> &Rc<dyn MorphableScene> {
        &self.scene
    }

    /// Gets or sets the duration of scene mode transition animations in
    /// seconds, mirroring the `duration` property. A value of zero causes
    /// the scene to instantly change modes.
    pub fn duration(&self) -> f64 {
        self.duration.get()
    }

    /// Sets the morph duration.
    ///
    /// # Panics
    /// Panics with a `DeveloperError` when `value` is negative.
    pub fn set_duration(&self, value: f64) {
        //>>includeStart('debug', pragmas.debug);
        if value < 0.0 {
            throw_developer_error("duration value must be positive.");
        }
        //>>includeEnd('debug');
        self.duration.set(value);
    }

    /// Gets the current scene mode, mirroring the observable `sceneMode`
    /// property.
    pub fn scene_mode(&self) -> SceneMode {
        self.scene_mode.get()
    }

    /// Gets or sets whether the button drop-down is currently visible,
    /// mirroring the observable `dropDownVisible` property.
    pub fn drop_down_visible(&self) -> bool {
        self.drop_down_visible.get()
    }

    /// Sets the drop-down visibility.
    pub fn set_drop_down_visible(&self, value: bool) {
        self.drop_down_visible.set(value);
    }

    /// Gets the 2D tooltip, mirroring the observable `tooltip2D`.
    pub fn tooltip_2d(&self) -> String {
        self.tooltip_2d.borrow().clone()
    }

    /// Gets the 3D tooltip, mirroring the observable `tooltip3D`.
    pub fn tooltip_3d(&self) -> String {
        self.tooltip_3d.borrow().clone()
    }

    /// Gets the Columbus View tooltip, mirroring the observable
    /// `tooltipColumbusView`.
    pub fn tooltip_columbus_view(&self) -> String {
        self.tooltip_columbus_view.borrow().clone()
    }

    /// Gets the currently active tooltip, mirroring the `selectedTooltip`
    /// computed.
    pub fn selected_tooltip(&self) -> String {
        match self.scene_mode.get() {
            SceneMode::Scene2D => self.tooltip_2d(),
            SceneMode::Scene3D => self.tooltip_3d(),
            _ => self.tooltip_columbus_view(),
        }
    }

    /// Gets the command to toggle the drop down box, mirroring the
    /// readonly `toggleDropDown` property.
    pub fn toggle_drop_down(&self) -> &Command {
        &self.toggle_drop_down
    }

    /// Gets the command to morph to 2D, mirroring the readonly `morphTo2D`
    /// property.
    pub fn morph_to_2d(&self) -> &Command {
        &self.morph_to_2d
    }

    /// Gets the command to morph to 3D, mirroring the readonly `morphTo3D`
    /// property.
    pub fn morph_to_3d(&self) -> &Command {
        &self.morph_to_3d
    }

    /// Gets the command to morph to Columbus View, mirroring the readonly
    /// `morphToColumbusView` property.
    pub fn morph_to_columbus_view(&self) -> &Command {
        &self.morph_to_columbus_view
    }

    /// Returns whether this view model has been destroyed, mirroring
    /// `isDestroyed()`.
    pub fn is_destroyed(&self) -> bool {
        self.destroyed.get()
    }

    /// Destroys the view model, mirroring `destroy()`: removes all event
    /// listeners registered through the internal `EventHelper`.
    pub fn destroy(&mut self) {
        self.event_helper.remove_all();
        self.destroyed.set(true);
    }
}
