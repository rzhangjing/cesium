//! Button widget view models.
//!
//! Maps to CesiumJS:
//! - `HomeButton/HomeButtonViewModel.js`
//! - `FullscreenButton/FullscreenButtonViewModel.js`
//! - `NavigationHelpButton/NavigationHelpButtonViewModel.js`
//! - `VRButton/VRButtonViewModel.js`

/// A generic toggle button view model.
#[derive(Debug, Clone)]
pub struct ToggleButtonViewModel {
    /// Whether the button is toggled on.
    pub is_toggled: bool,
    /// Button tooltip text.
    pub tooltip: String,
    /// Whether the button is visible.
    pub show: bool,
    /// Whether the button is enabled.
    pub is_enabled: bool,
}

impl ToggleButtonViewModel {
    /// Create a new toggle button.
    pub fn new(tooltip: impl Into<String>) -> Self {
        Self {
            is_toggled: false,
            tooltip: tooltip.into(),
            show: true,
            is_enabled: true,
        }
    }

    /// Toggle the button state.
    pub fn toggle(&mut self) {
        if self.is_enabled {
            self.is_toggled = !self.is_toggled;
        }
    }

    /// Set the toggled state.
    pub fn set_toggled(&mut self, toggled: bool) {
        if self.is_enabled {
            self.is_toggled = toggled;
        }
    }
}

/// Home button view model.
///
/// Resets the camera to the default home view.
#[derive(Debug, Clone)]
pub struct HomeButtonViewModel {
    /// Button tooltip.
    pub tooltip: String,
    /// Whether the button is visible.
    pub show: bool,
    /// Duration of the home flight in seconds.
    pub duration: f64,
    /// Home view longitude in radians.
    pub home_longitude: f64,
    /// Home view latitude in radians.
    pub home_latitude: f64,
    /// Home view height in meters.
    pub home_height: f64,
}

impl Default for HomeButtonViewModel {
    fn default() -> Self {
        Self {
            tooltip: "View Home".to_string(),
            show: true,
            duration: 1.5,
            // Default home: looking at Earth from a distance
            home_longitude: 0.0,
            home_latitude: 0.0,
            home_height: 15_000_000.0,
        }
    }
}

impl HomeButtonViewModel {
    /// Create a new home button.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the home view position.
    pub fn set_home(&mut self, longitude: f64, latitude: f64, height: f64) {
        self.home_longitude = longitude;
        self.home_latitude = latitude;
        self.home_height = height;
    }

    /// Get the home position as (longitude, latitude, height).
    pub fn home_position(&self) -> (f64, f64, f64) {
        (self.home_longitude, self.home_latitude, self.home_height)
    }
}

/// Fullscreen button view model.
///
/// Toggles browser fullscreen mode.
#[derive(Debug, Clone)]
pub struct FullscreenButtonViewModel {
    /// Whether fullscreen is currently active.
    pub is_fullscreen: bool,
    /// Tooltip when not fullscreen.
    pub enter_tooltip: String,
    /// Tooltip when fullscreen.
    pub exit_tooltip: String,
    /// Whether the button is visible.
    pub show: bool,
    /// Whether fullscreen is supported by the environment.
    pub is_supported: bool,
}

impl Default for FullscreenButtonViewModel {
    fn default() -> Self {
        Self {
            is_fullscreen: false,
            enter_tooltip: "Full screen".to_string(),
            exit_tooltip: "Exit full screen".to_string(),
            show: true,
            is_supported: true,
        }
    }
}

impl FullscreenButtonViewModel {
    /// Create a new fullscreen button.
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle fullscreen state.
    pub fn toggle_fullscreen(&mut self) {
        if self.is_supported {
            self.is_fullscreen = !self.is_fullscreen;
        }
    }

    /// Get the current tooltip.
    pub fn current_tooltip(&self) -> &str {
        if self.is_fullscreen {
            &self.exit_tooltip
        } else {
            &self.enter_tooltip
        }
    }
}

/// Navigation help button view model.
///
/// Shows/hides navigation help overlay.
#[derive(Debug, Clone)]
pub struct NavigationHelpButtonViewModel {
    /// Whether the help panel is visible.
    pub is_help_visible: bool,
    /// Button tooltip.
    pub tooltip: String,
    /// Whether the button is visible.
    pub show: bool,
    /// Whether to show touch navigation help (vs mouse).
    pub show_touch: bool,
}

impl Default for NavigationHelpButtonViewModel {
    fn default() -> Self {
        Self {
            is_help_visible: false,
            tooltip: "Navigation Instructions".to_string(),
            show: true,
            show_touch: false,
        }
    }
}

impl NavigationHelpButtonViewModel {
    /// Create a new navigation help button.
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle the help panel.
    pub fn toggle_help(&mut self) {
        self.is_help_visible = !self.is_help_visible;
    }

    /// Show the help panel.
    pub fn show_help(&mut self) {
        self.is_help_visible = true;
    }

    /// Hide the help panel.
    pub fn hide_help(&mut self) {
        self.is_help_visible = false;
    }

    /// Switch to mouse navigation instructions.
    pub fn show_mouse_help(&mut self) {
        self.show_touch = false;
    }

    /// Switch to touch navigation instructions.
    pub fn show_touch_help(&mut self) {
        self.show_touch = true;
    }
}

/// VR button view model.
///
/// Toggles VR mode.
#[derive(Debug, Clone)]
pub struct VRButtonViewModel {
    /// Whether VR mode is active.
    pub is_vr_active: bool,
    /// Tooltip when not in VR.
    pub enter_tooltip: String,
    /// Tooltip when in VR.
    pub exit_tooltip: String,
    /// Whether the button is visible.
    pub show: bool,
    /// Whether VR is supported.
    pub is_supported: bool,
}

impl Default for VRButtonViewModel {
    fn default() -> Self {
        Self {
            is_vr_active: false,
            enter_tooltip: "Enter VR".to_string(),
            exit_tooltip: "Exit VR".to_string(),
            show: true,
            is_supported: false,
        }
    }
}

impl VRButtonViewModel {
    /// Create a new VR button.
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle VR mode.
    pub fn toggle_vr(&mut self) {
        if self.is_supported {
            self.is_vr_active = !self.is_vr_active;
        }
    }

    /// Get the current tooltip.
    pub fn current_tooltip(&self) -> &str {
        if self.is_vr_active {
            &self.exit_tooltip
        } else {
            &self.enter_tooltip
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_button() {
        let mut btn = ToggleButtonViewModel::new("Test");
        assert!(!btn.is_toggled);
        btn.toggle();
        assert!(btn.is_toggled);
        btn.toggle();
        assert!(!btn.is_toggled);
    }

    #[test]
    fn test_toggle_button_disabled() {
        let mut btn = ToggleButtonViewModel::new("Test");
        btn.is_enabled = false;
        btn.toggle();
        assert!(!btn.is_toggled);
    }

    #[test]
    fn test_home_button_default() {
        let btn = HomeButtonViewModel::default();
        assert_eq!(btn.tooltip, "View Home");
        assert_eq!(btn.duration, 1.5);
        assert_eq!(btn.home_height, 15_000_000.0);
    }

    #[test]
    fn test_home_button_set_home() {
        let mut btn = HomeButtonViewModel::new();
        btn.set_home(1.0, 0.5, 1000.0);
        assert_eq!(btn.home_position(), (1.0, 0.5, 1000.0));
    }

    #[test]
    fn test_fullscreen_button() {
        let mut btn = FullscreenButtonViewModel::default();
        assert!(!btn.is_fullscreen);
        assert_eq!(btn.current_tooltip(), "Full screen");
        btn.toggle_fullscreen();
        assert!(btn.is_fullscreen);
        assert_eq!(btn.current_tooltip(), "Exit full screen");
    }

    #[test]
    fn test_fullscreen_unsupported() {
        let mut btn = FullscreenButtonViewModel::default();
        btn.is_supported = false;
        btn.toggle_fullscreen();
        assert!(!btn.is_fullscreen);
    }

    #[test]
    fn test_navigation_help_button() {
        let mut btn = NavigationHelpButtonViewModel::default();
        assert!(!btn.is_help_visible);
        btn.toggle_help();
        assert!(btn.is_help_visible);
        btn.hide_help();
        assert!(!btn.is_help_visible);
        btn.show_help();
        assert!(btn.is_help_visible);
    }

    #[test]
    fn test_navigation_help_touch() {
        let mut btn = NavigationHelpButtonViewModel::default();
        assert!(!btn.show_touch);
        btn.show_touch_help();
        assert!(btn.show_touch);
        btn.show_mouse_help();
        assert!(!btn.show_touch);
    }

    #[test]
    fn test_vr_button() {
        let mut btn = VRButtonViewModel::default();
        assert!(!btn.is_vr_active);
        assert!(!btn.is_supported);
        // VR not supported, toggle should not work
        btn.toggle_vr();
        assert!(!btn.is_vr_active);

        btn.is_supported = true;
        btn.toggle_vr();
        assert!(btn.is_vr_active);
        assert_eq!(btn.current_tooltip(), "Exit VR");
    }
}
