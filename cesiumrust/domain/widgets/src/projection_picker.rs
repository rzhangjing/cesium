//! Projection picker view model.
//!
//! Maps to CesiumJS `ProjectionPicker/ProjectionPickerViewModel.js`.

/// Projection type for the camera.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectionType {
    /// Perspective projection.
    #[default]
    Perspective,
    /// Orthographic projection.
    Orthographic,
}

impl ProjectionType {
    /// Get the display label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Perspective => "Perspective",
            Self::Orthographic => "Orthographic",
        }
    }

    /// Get the tooltip text.
    pub fn tooltip(&self) -> &'static str {
        match self {
            Self::Perspective => "Perspective projection",
            Self::Orthographic => "Orthographic projection",
        }
    }
}

/// Projection picker view model.
///
/// Controls switching between perspective and orthographic projections.
#[derive(Debug, Clone)]
pub struct ProjectionPickerViewModel {
    /// The currently selected projection type.
    pub selected_projection: ProjectionType,
    /// Whether the dropdown is expanded.
    pub is_dropdown_open: bool,
    /// Whether the widget is visible.
    pub show: bool,
    /// Whether the transition is animated.
    pub is_transitioning: bool,
    /// Transition progress [0, 1].
    pub transition_progress: f64,
}

impl Default for ProjectionPickerViewModel {
    fn default() -> Self {
        Self {
            selected_projection: ProjectionType::Perspective,
            is_dropdown_open: false,
            show: true,
            is_transitioning: false,
            transition_progress: 0.0,
        }
    }
}

impl ProjectionPickerViewModel {
    /// Create a new projection picker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Select a projection type.
    pub fn select_projection(&mut self, projection: ProjectionType) {
        if self.selected_projection != projection {
            self.selected_projection = projection;
            self.is_transitioning = true;
            self.transition_progress = 0.0;
        }
        self.is_dropdown_open = false;
    }

    /// Switch to perspective projection.
    pub fn select_perspective(&mut self) {
        self.select_projection(ProjectionType::Perspective);
    }

    /// Switch to orthographic projection.
    pub fn select_orthographic(&mut self) {
        self.select_projection(ProjectionType::Orthographic);
    }

    /// Toggle the dropdown.
    pub fn toggle_dropdown(&mut self) {
        self.is_dropdown_open = !self.is_dropdown_open;
    }

    /// Close the dropdown.
    pub fn close_dropdown(&mut self) {
        self.is_dropdown_open = false;
    }

    /// Update the transition animation.
    /// Returns true if transition is complete.
    pub fn update_transition(&mut self, delta_seconds: f64) -> bool {
        if !self.is_transitioning {
            return true;
        }

        let duration = 0.5; // 0.5 second transition
        self.transition_progress += delta_seconds / duration;

        if self.transition_progress >= 1.0 {
            self.transition_progress = 1.0;
            self.is_transitioning = false;
            true
        } else {
            false
        }
    }

    /// Get the current label.
    pub fn current_label(&self) -> &'static str {
        self.selected_projection.label()
    }

    /// Check if a projection is selected.
    pub fn is_selected(&self, projection: ProjectionType) -> bool {
        self.selected_projection == projection
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let vm = ProjectionPickerViewModel::default();
        assert_eq!(vm.selected_projection, ProjectionType::Perspective);
        assert!(!vm.is_dropdown_open);
        assert!(vm.show);
        assert!(!vm.is_transitioning);
    }

    #[test]
    fn test_select_projection() {
        let mut vm = ProjectionPickerViewModel::new();
        vm.select_orthographic();
        assert_eq!(vm.selected_projection, ProjectionType::Orthographic);
        assert!(vm.is_transitioning);
        assert_eq!(vm.transition_progress, 0.0);
    }

    #[test]
    fn test_select_same_projection() {
        let mut vm = ProjectionPickerViewModel::new();
        vm.select_perspective();
        // Already perspective, no transition
        assert!(!vm.is_transitioning);
    }

    #[test]
    fn test_transition_update() {
        let mut vm = ProjectionPickerViewModel::new();
        vm.select_orthographic();
        assert!(vm.is_transitioning);

        // Partial update
        let done = vm.update_transition(0.25);
        assert!(!done);
        assert!((vm.transition_progress - 0.5).abs() < 1e-10);

        // Complete
        let done = vm.update_transition(0.3);
        assert!(done);
        assert!(!vm.is_transitioning);
        assert_eq!(vm.transition_progress, 1.0);
    }

    #[test]
    fn test_toggle_dropdown() {
        let mut vm = ProjectionPickerViewModel::new();
        vm.toggle_dropdown();
        assert!(vm.is_dropdown_open);
        vm.toggle_dropdown();
        assert!(!vm.is_dropdown_open);
    }

    #[test]
    fn test_projection_type_labels() {
        assert_eq!(ProjectionType::Perspective.label(), "Perspective");
        assert_eq!(ProjectionType::Orthographic.label(), "Orthographic");
    }

    #[test]
    fn test_current_label() {
        let mut vm = ProjectionPickerViewModel::new();
        assert_eq!(vm.current_label(), "Perspective");
        vm.select_orthographic();
        assert_eq!(vm.current_label(), "Orthographic");
    }
}
