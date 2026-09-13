//! Info box widget view model.
//!
//! Maps to CesiumJS `InfoBox/InfoBoxViewModel.js`.

/// Info box widget view model.
///
/// Displays information about a selected entity in a panel.
#[derive(Debug, Clone)]
pub struct InfoBoxViewModel {
    /// Whether the info box is visible.
    pub show: bool,
    /// Whether the info box frame/panel is shown (expanded).
    pub is_frame_visible: bool,
    /// Title text (usually entity name).
    pub title: String,
    /// Description content (HTML or plain text).
    pub description: String,
    /// Whether the close button is visible.
    pub show_close: bool,
    /// Whether the info box has content to display.
    pub has_content: bool,
    /// Camera view offset when tracking the entity.
    pub camera_view_offset: Option<[f64; 3]>,
    /// Whether the info box is in "tracking" mode.
    pub is_tracking: bool,
}

impl Default for InfoBoxViewModel {
    fn default() -> Self {
        Self {
            show: true,
            is_frame_visible: false,
            title: String::new(),
            description: String::new(),
            show_close: true,
            has_content: false,
            camera_view_offset: None,
            is_tracking: false,
        }
    }
}

impl InfoBoxViewModel {
    /// Create a new info box view model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Show entity information.
    pub fn show_entity(&mut self, title: impl Into<String>, description: impl Into<String>) {
        self.title = title.into();
        self.description = description.into();
        self.has_content = true;
        self.is_frame_visible = true;
    }

    /// Clear the info box content.
    pub fn clear(&mut self) {
        self.title.clear();
        self.description.clear();
        self.has_content = false;
        self.is_frame_visible = false;
        self.is_tracking = false;
        self.camera_view_offset = None;
    }

    /// Close the info box panel (hide frame but keep widget).
    pub fn close(&mut self) {
        self.is_frame_visible = false;
    }

    /// Toggle the frame visibility.
    pub fn toggle_frame(&mut self) {
        if self.has_content {
            self.is_frame_visible = !self.is_frame_visible;
        }
    }

    /// Set tracking mode.
    pub fn set_tracking(&mut self, tracking: bool) {
        self.is_tracking = tracking;
    }

    /// Set the camera view offset for tracking.
    pub fn set_camera_offset(&mut self, offset: [f64; 3]) {
        self.camera_view_offset = Some(offset);
    }

    /// Get a summary line for the info box.
    pub fn summary(&self) -> String {
        if !self.has_content {
            return String::new();
        }
        if self.description.len() > 100 {
            format!("{}...", &self.description[..97])
        } else {
            self.description.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let vm = InfoBoxViewModel::default();
        assert!(vm.show);
        assert!(!vm.is_frame_visible);
        assert!(!vm.has_content);
        assert!(vm.title.is_empty());
    }

    #[test]
    fn test_show_entity() {
        let mut vm = InfoBoxViewModel::new();
        vm.show_entity("Test Entity", "A description");
        assert_eq!(vm.title, "Test Entity");
        assert_eq!(vm.description, "A description");
        assert!(vm.has_content);
        assert!(vm.is_frame_visible);
    }

    #[test]
    fn test_clear() {
        let mut vm = InfoBoxViewModel::new();
        vm.show_entity("Test", "Desc");
        vm.set_tracking(true);
        vm.set_camera_offset([1.0, 2.0, 3.0]);
        vm.clear();

        assert!(vm.title.is_empty());
        assert!(vm.description.is_empty());
        assert!(!vm.has_content);
        assert!(!vm.is_frame_visible);
        assert!(!vm.is_tracking);
        assert!(vm.camera_view_offset.is_none());
    }

    #[test]
    fn test_close() {
        let mut vm = InfoBoxViewModel::new();
        vm.show_entity("Test", "Desc");
        assert!(vm.is_frame_visible);
        vm.close();
        assert!(!vm.is_frame_visible);
        assert!(vm.has_content); // Content preserved
    }

    #[test]
    fn test_toggle_frame() {
        let mut vm = InfoBoxViewModel::new();
        // No content - toggle should not work
        vm.toggle_frame();
        assert!(!vm.is_frame_visible);

        vm.show_entity("Test", "Desc");
        vm.toggle_frame();
        assert!(!vm.is_frame_visible);
        vm.toggle_frame();
        assert!(vm.is_frame_visible);
    }

    #[test]
    fn test_summary_short() {
        let mut vm = InfoBoxViewModel::new();
        vm.show_entity("Test", "Short desc");
        assert_eq!(vm.summary(), "Short desc");
    }

    #[test]
    fn test_summary_long() {
        let mut vm = InfoBoxViewModel::new();
        let long_desc = "A".repeat(200);
        vm.show_entity("Test", long_desc);
        let summary = vm.summary();
        assert_eq!(summary.len(), 100); // 97 chars + "..."
        assert!(summary.ends_with("..."));
    }

    #[test]
    fn test_summary_no_content() {
        let vm = InfoBoxViewModel::new();
        assert!(vm.summary().is_empty());
    }

    #[test]
    fn test_tracking() {
        let mut vm = InfoBoxViewModel::new();
        vm.set_tracking(true);
        assert!(vm.is_tracking);
        vm.set_camera_offset([100.0, 200.0, 300.0]);
        assert_eq!(vm.camera_view_offset, Some([100.0, 200.0, 300.0]));
    }
}
