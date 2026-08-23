//! Ported from `packages/widgets/Source/FullscreenButton/FullscreenButtonViewModel.js`.

/// The view model for the FullscreenButton widget.
pub struct FullscreenButtonViewModel {
    /// Whether fullscreen is currently active.
    pub is_fullscreen: bool,
    /// Whether fullscreen is supported.
    pub is_fullscreen_supported: bool,
}

impl FullscreenButtonViewModel {
    /// Creates a new fullscreen button view model.
    pub fn new() -> Self {
        Self {
            is_fullscreen: false,
            is_fullscreen_supported: true,
        }
    }

    /// Toggles fullscreen mode.
    pub fn toggle_fullscreen(&mut self) {
        if self.is_fullscreen_supported {
            self.is_fullscreen = !self.is_fullscreen;
        }
    }
}

impl Default for FullscreenButtonViewModel {
    fn default() -> Self { Self::new() }
}
