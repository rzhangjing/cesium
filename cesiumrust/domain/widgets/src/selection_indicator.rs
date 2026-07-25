//! Selection indicator widget view model.
//!
//! Maps to CesiumJS `SelectionIndicator/SelectionIndicatorViewModel.js`.

/// Selection indicator widget view model.
///
/// Shows a visual indicator at the screen position of a selected entity.
#[derive(Debug, Clone)]
pub struct SelectionIndicatorViewModel {
    /// Whether the indicator is visible.
    pub show: bool,
    /// Screen X position in pixels.
    pub screen_x: f64,
    /// Screen Y position in pixels.
    pub screen_y: f64,
    /// Scale of the indicator.
    pub scale: f64,
    /// Rotation angle in radians.
    pub rotation: f64,
    /// Whether the indicator is currently animating (appearing/disappearing).
    pub is_animating: bool,
    /// Animation progress [0, 1].
    pub animation_progress: f64,
    /// Whether the selected entity is on screen.
    pub is_on_screen: bool,
}

impl Default for SelectionIndicatorViewModel {
    fn default() -> Self {
        Self {
            show: false,
            screen_x: 0.0,
            screen_y: 0.0,
            scale: 1.0,
            rotation: 0.0,
            is_animating: false,
            animation_progress: 0.0,
            is_on_screen: false,
        }
    }
}

impl SelectionIndicatorViewModel {
    /// Create a new selection indicator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the indicator at a screen position.
    pub fn show_at(&mut self, x: f64, y: f64) {
        self.show = true;
        self.screen_x = x;
        self.screen_y = y;
        self.is_on_screen = true;
        if !self.is_animating {
            self.is_animating = true;
            self.animation_progress = 0.0;
        }
    }

    /// Hide the indicator.
    pub fn hide(&mut self) {
        self.show = false;
        self.is_on_screen = false;
        self.is_animating = false;
        self.animation_progress = 0.0;
    }

    /// Update the screen position.
    pub fn update_position(&mut self, x: f64, y: f64, on_screen: bool) {
        self.screen_x = x;
        self.screen_y = y;
        self.is_on_screen = on_screen;
    }

    /// Update the appear/disappear animation.
    /// Returns true if animation is complete.
    pub fn update_animation(&mut self, delta_seconds: f64) -> bool {
        if !self.is_animating {
            return true;
        }

        let duration = 0.3; // 300ms animation
        if self.show {
            // Appearing
            self.animation_progress += delta_seconds / duration;
            if self.animation_progress >= 1.0 {
                self.animation_progress = 1.0;
                self.is_animating = false;
                self.scale = 1.0;
                return true;
            }
            // Scale from 2.0 to 1.0 (bounce in)
            self.scale = 2.0 - self.animation_progress;
        } else {
            // Disappearing
            self.animation_progress += delta_seconds / duration;
            if self.animation_progress >= 1.0 {
                self.animation_progress = 1.0;
                self.is_animating = false;
                return true;
            }
            self.scale = 1.0 - self.animation_progress;
        }

        false
    }

    /// Set the rotation angle.
    pub fn set_rotation(&mut self, radians: f64) {
        self.rotation = radians;
    }

    /// Check if the indicator should be rendered.
    pub fn should_render(&self) -> bool {
        self.show && self.is_on_screen
    }

    /// Get the CSS-like transform string for the indicator.
    pub fn transform_description(&self) -> String {
        format!(
            "translate({:.1}px, {:.1}px) scale({:.3}) rotate({:.2}rad)",
            self.screen_x, self.screen_y, self.scale, self.rotation
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let vm = SelectionIndicatorViewModel::default();
        assert!(!vm.show);
        assert!(!vm.is_on_screen);
        assert_eq!(vm.scale, 1.0);
    }

    #[test]
    fn test_show_at() {
        let mut vm = SelectionIndicatorViewModel::new();
        vm.show_at(100.0, 200.0);
        assert!(vm.show);
        assert_eq!(vm.screen_x, 100.0);
        assert_eq!(vm.screen_y, 200.0);
        assert!(vm.is_on_screen);
        assert!(vm.is_animating);
    }

    #[test]
    fn test_hide() {
        let mut vm = SelectionIndicatorViewModel::new();
        vm.show_at(100.0, 200.0);
        vm.hide();
        assert!(!vm.show);
        assert!(!vm.is_on_screen);
        assert!(!vm.is_animating);
    }

    #[test]
    fn test_update_position() {
        let mut vm = SelectionIndicatorViewModel::new();
        vm.show_at(100.0, 200.0);
        vm.update_position(150.0, 250.0, true);
        assert_eq!(vm.screen_x, 150.0);
        assert_eq!(vm.screen_y, 250.0);

        vm.update_position(300.0, 400.0, false);
        assert!(!vm.is_on_screen);
    }

    #[test]
    fn test_appear_animation() {
        let mut vm = SelectionIndicatorViewModel::new();
        vm.show_at(100.0, 200.0);
        assert!(vm.is_animating);
        assert_eq!(vm.animation_progress, 0.0);

        // Partial update
        let done = vm.update_animation(0.15);
        assert!(!done);
        assert!((vm.animation_progress - 0.5).abs() < 1e-10);
        assert!(vm.scale > 1.0 && vm.scale < 2.0);

        // Complete
        let done = vm.update_animation(0.2);
        assert!(done);
        assert!(!vm.is_animating);
        assert_eq!(vm.scale, 1.0);
    }

    #[test]
    fn test_should_render() {
        let mut vm = SelectionIndicatorViewModel::new();
        assert!(!vm.should_render());
        vm.show_at(100.0, 200.0);
        assert!(vm.should_render());
        vm.update_position(100.0, 200.0, false);
        assert!(!vm.should_render());
    }

    #[test]
    fn test_rotation() {
        let mut vm = SelectionIndicatorViewModel::new();
        vm.set_rotation(std::f64::consts::FRAC_PI_4);
        assert!((vm.rotation - std::f64::consts::FRAC_PI_4).abs() < 1e-10);
    }

    #[test]
    fn test_transform_description() {
        let mut vm = SelectionIndicatorViewModel::new();
        vm.show_at(100.0, 200.0);
        vm.update_animation(1.0); // Complete animation
        let desc = vm.transform_description();
        assert!(desc.contains("translate(100.0px, 200.0px)"));
        assert!(desc.contains("scale(1.000)"));
    }
}
