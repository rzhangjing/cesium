//! Ported from `packages/widgets/Source/NavigationHelpButton/NavigationHelpButtonViewModel.js`.

/// The view model for the NavigationHelpButton widget.
pub struct NavigationHelpButtonViewModel {
    /// Whether the help panel is visible.
    pub show_instructions: bool,
}

impl NavigationHelpButtonViewModel {
    /// Creates a new navigation help button view model.
    pub fn new() -> Self {
        Self { show_instructions: false }
    }

    /// Toggles the instructions panel.
    pub fn toggle_instructions(&mut self) {
        self.show_instructions = !self.show_instructions;
    }
}

impl Default for NavigationHelpButtonViewModel {
    fn default() -> Self { Self::new() }
}
