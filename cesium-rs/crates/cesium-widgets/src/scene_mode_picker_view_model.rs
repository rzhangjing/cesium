//! Ported from `packages/widgets/Source/SceneModePicker/SceneModePickerViewModel.js`.
//!
//! The view model for the SceneModePicker widget.

use cesium_scene::scene_mode::SceneMode;

/// The view model for the SceneModePicker widget.
///
/// In CesiumJS, SceneModePickerViewModel.js allows the user to switch
/// between 3D, 2D, and Columbus View scene modes. It provides:
/// - Current mode display
/// - Morph transition triggering
/// - Tooltip text for each mode
pub struct SceneModePickerViewModel {
    /// The current scene mode.
    scene_mode: SceneMode,
    /// The morph time for transitions.
    morph_time: f64,
    /// The tooltip text.
    tooltip_3d: String,
    tooltip_2d: String,
    tooltip_columbus: String,
    is_destroyed: bool,
}

impl SceneModePickerViewModel {
    /// Creates a new scene mode picker view model.
    pub fn new() -> Self {
        Self {
            scene_mode: SceneMode::Scene3D,
            morph_time: 1.0,
            tooltip_3d: String::from("3D"),
            tooltip_2d: String::from("2D"),
            tooltip_columbus: String::from("Columbus View"),
            is_destroyed: false,
        }
    }

    /// Returns the current scene mode.
    pub fn scene_mode(&self) -> SceneMode {
        self.scene_mode
    }

    /// Sets the scene mode.
    pub fn set_scene_mode(&mut self, mode: SceneMode) {
        self.scene_mode = mode;
    }

    /// Switches to 3D mode.
    pub fn morph_to2_d(&mut self) {
        self.scene_mode = SceneMode::Scene2D;
    }

    /// Switches to 2D mode.
    pub fn morph_to3_d(&mut self) {
        self.scene_mode = SceneMode::Scene3D;
    }

    /// Switches to Columbus View mode.
    pub fn morph_to_columbus_view(&mut self) {
        self.scene_mode = SceneMode::ColumbusView;
    }

    /// Returns the morph time.
    pub fn morph_time(&self) -> f64 {
        self.morph_time
    }

    /// Sets the morph time.
    pub fn set_morph_time(&mut self, time: f64) {
        self.morph_time = time;
    }

    /// Returns whether this view model has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this view model.
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}

impl Default for SceneModePickerViewModel {
    fn default() -> Self {
        Self::new()
    }
}
