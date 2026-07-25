//! Scene mode picker view model.
//!
//! Maps to CesiumJS `SceneModePicker/SceneModePickerViewModel.js`.

use cesium_scene_mode::SceneMode;

/// Scene mode picker view model.
///
/// Controls switching between 3D, 2D, and Columbus View modes.
#[derive(Debug, Clone)]
pub struct SceneModePickerViewModel {
    /// The currently selected scene mode.
    pub selected_mode: SceneMode,
    /// Whether the dropdown is expanded.
    pub is_dropdown_open: bool,
    /// Whether the widget is visible.
    pub show: bool,
    /// Duration of mode morph transitions in seconds.
    pub morph_duration: f64,
}

impl Default for SceneModePickerViewModel {
    fn default() -> Self {
        Self {
            selected_mode: SceneMode::Scene3D,
            is_dropdown_open: false,
            show: true,
            morph_duration: 2.0,
        }
    }
}

impl SceneModePickerViewModel {
    /// Create a new scene mode picker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Select a scene mode and trigger morphing.
    pub fn select_mode(&mut self, mode: SceneMode) {
        if mode != SceneMode::Morphing {
            self.selected_mode = mode;
            self.is_dropdown_open = false;
        }
    }

    /// Select 3D mode.
    pub fn select_3d(&mut self) {
        self.select_mode(SceneMode::Scene3D);
    }

    /// Select 2D mode.
    pub fn select_2d(&mut self) {
        self.select_mode(SceneMode::Scene2D);
    }

    /// Select Columbus View mode.
    pub fn select_columbus_view(&mut self) {
        self.select_mode(SceneMode::ColumbusView);
    }

    /// Toggle the dropdown.
    pub fn toggle_dropdown(&mut self) {
        self.is_dropdown_open = !self.is_dropdown_open;
    }

    /// Close the dropdown.
    pub fn close_dropdown(&mut self) {
        self.is_dropdown_open = false;
    }

    /// Get the display label for the current mode.
    pub fn current_label(&self) -> &'static str {
        match self.selected_mode {
            SceneMode::Scene3D => "3D",
            SceneMode::Scene2D => "2D",
            SceneMode::ColumbusView => "Columbus View",
            SceneMode::Morphing => "Morphing",
        }
    }

    /// Get the tooltip for a given mode.
    pub fn tooltip_for_mode(mode: SceneMode) -> &'static str {
        match mode {
            SceneMode::Scene3D => "3D globe view",
            SceneMode::Scene2D => "2D flat map view",
            SceneMode::ColumbusView => "Columbus View (2.5D)",
            SceneMode::Morphing => "Morphing between modes",
        }
    }

    /// Get all selectable modes (excludes Morphing).
    pub fn available_modes() -> &'static [SceneMode] {
        &[SceneMode::Scene3D, SceneMode::Scene2D, SceneMode::ColumbusView]
    }

    /// Check if a mode is currently selected.
    pub fn is_mode_selected(&self, mode: SceneMode) -> bool {
        self.selected_mode == mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let vm = SceneModePickerViewModel::default();
        assert_eq!(vm.selected_mode, SceneMode::Scene3D);
        assert!(!vm.is_dropdown_open);
        assert!(vm.show);
        assert_eq!(vm.morph_duration, 2.0);
    }

    #[test]
    fn test_select_mode() {
        let mut vm = SceneModePickerViewModel::new();
        vm.is_dropdown_open = true;
        vm.select_mode(SceneMode::Scene2D);
        assert_eq!(vm.selected_mode, SceneMode::Scene2D);
        assert!(!vm.is_dropdown_open);
    }

    #[test]
    fn test_select_morphing_ignored() {
        let mut vm = SceneModePickerViewModel::new();
        vm.select_mode(SceneMode::Morphing);
        // Should not change to Morphing
        assert_eq!(vm.selected_mode, SceneMode::Scene3D);
    }

    #[test]
    fn test_convenience_selectors() {
        let mut vm = SceneModePickerViewModel::new();
        vm.select_2d();
        assert_eq!(vm.selected_mode, SceneMode::Scene2D);
        vm.select_columbus_view();
        assert_eq!(vm.selected_mode, SceneMode::ColumbusView);
        vm.select_3d();
        assert_eq!(vm.selected_mode, SceneMode::Scene3D);
    }

    #[test]
    fn test_toggle_dropdown() {
        let mut vm = SceneModePickerViewModel::new();
        assert!(!vm.is_dropdown_open);
        vm.toggle_dropdown();
        assert!(vm.is_dropdown_open);
        vm.toggle_dropdown();
        assert!(!vm.is_dropdown_open);
    }

    #[test]
    fn test_current_label() {
        let mut vm = SceneModePickerViewModel::new();
        assert_eq!(vm.current_label(), "3D");
        vm.select_2d();
        assert_eq!(vm.current_label(), "2D");
        vm.select_columbus_view();
        assert_eq!(vm.current_label(), "Columbus View");
    }

    #[test]
    fn test_available_modes() {
        let modes = SceneModePickerViewModel::available_modes();
        assert_eq!(modes.len(), 3);
        assert!(!modes.contains(&SceneMode::Morphing));
    }

    #[test]
    fn test_is_mode_selected() {
        let mut vm = SceneModePickerViewModel::new();
        assert!(vm.is_mode_selected(SceneMode::Scene3D));
        assert!(!vm.is_mode_selected(SceneMode::Scene2D));
        vm.select_2d();
        assert!(vm.is_mode_selected(SceneMode::Scene2D));
        assert!(!vm.is_mode_selected(SceneMode::Scene3D));
    }
}
