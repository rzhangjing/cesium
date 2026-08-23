//! Ported from `packages/widgets/Source/HomeButton/HomeButtonViewModel.js`.

/// The view model for the HomeButton widget.
///
/// Controls the "home" camera position.
pub struct HomeButtonViewModel {
    /// The tooltip text.
    pub tooltip: String,
}

impl HomeButtonViewModel {
    /// Creates a new home button view model.
    pub fn new() -> Self {
        Self {
            tooltip: String::from("Home"),
        }
    }

    /// Executes the home command (fly to default view).
    pub fn go_home(&self) {
        // DEVIATION: Requires scene.camera.flyHome() integration
    }
}

impl Default for HomeButtonViewModel {
    fn default() -> Self { Self::new() }
}
