//! Ported from `packages/widgets/Source/ToggleButtonViewModel.js`.
//!
//! A view model for a toggle button.

use crate::command::Command;

/// A view model that tracks a toggled state and exposes a command.
///
/// In CesiumJS, ToggleButtonViewModel wraps a Command and adds a `toggled`
/// observable property. Many widget buttons use this:
/// - FullscreenButton
/// - NavigationHelpButton
/// - BaseLayerPicker (drop-down toggle)
pub struct ToggleButtonViewModel {
    /// The underlying command.
    command: Command,
    /// Whether the button is currently toggled.
    toggled: bool,
    /// The tooltip text.
    tooltip: String,
}

impl ToggleButtonViewModel {
    /// Creates a new toggle button view model.
    pub fn new(command: Command, tooltip: &str) -> Self {
        Self {
            command,
            toggled: false,
            tooltip: tooltip.to_string(),
        }
    }

    /// Returns whether the button is toggled.
    pub fn is_toggled(&self) -> bool {
        self.toggled
    }

    /// Sets the toggled state.
    pub fn set_toggled(&mut self, toggled: bool) {
        self.toggled = toggled;
    }

    /// Toggles the state.
    pub fn toggle(&mut self) {
        self.toggled = !self.toggled;
    }

    /// Returns the tooltip.
    pub fn tooltip(&self) -> &str {
        &self.tooltip
    }

    /// Sets the tooltip.
    pub fn set_tooltip(&mut self, tooltip: &str) {
        self.tooltip = tooltip.to_string();
    }

    /// Returns the underlying command.
    pub fn command(&self) -> &Command {
        &self.command
    }

    /// Executes the underlying command if enabled.
    pub fn execute(&self) {
        self.command.execute();
    }
}

impl Default for ToggleButtonViewModel {
    fn default() -> Self {
        Self::new(Command::empty(), "")
    }
}
