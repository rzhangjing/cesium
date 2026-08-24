//! Ported from `packages/widgets/Source/NavigationHelpButton/NavigationHelpButtonViewModel.js`.
//!
//! The view model for the NavigationHelpButton widget.
//!
//! DEVIATION: knockout-tracked properties are modeled with
//! [`ObservableCell`] shared state so the command closures can toggle
//! `showInstructions` with the same reference semantics as the JS
//! `that = this` capture.

use crate::command::Command;
use crate::create_command::create_command;
use crate::observables::ObservableCell;

/// The view model for the NavigationHelpButton widget.
pub struct NavigationHelpButtonViewModel {
    /// `showInstructions` knockout-tracked property.
    show_instructions: ObservableCell<bool>,
    /// `_touch` knockout-tracked property.
    touch: ObservableCell<bool>,
    command: Command,
    show_click: Command,
    show_touch: Command,
    tooltip: String,
}

impl NavigationHelpButtonViewModel {
    /// Creates a new navigation help button view model.
    ///
    /// Mirrors `new NavigationHelpButtonViewModel()`.
    pub fn new() -> Self {
        let show_instructions = ObservableCell::new(false);
        let touch = ObservableCell::new(false);

        let command_show_instructions = show_instructions.clone();
        let command = create_command(
            move |_| {
                let current = command_show_instructions.get();
                command_show_instructions.set(!current);
                None
            },
            None,
        );

        let show_click_touch = touch.clone();
        let show_click = create_command(
            move |_| {
                show_click_touch.set(false);
                None
            },
            None,
        );

        let show_touch_touch = touch.clone();
        let show_touch = create_command(
            move |_| {
                show_touch_touch.set(true);
                None
            },
            None,
        );

        Self {
            show_instructions,
            touch,
            command,
            show_click,
            show_touch,
            tooltip: "Navigation Instructions".to_string(),
        }
    }

    /// Gets whether the instructions are currently shown.
    pub fn show_instructions(&self) -> bool {
        self.show_instructions.get()
    }

    /// Sets whether the instructions are currently shown.
    pub fn set_show_instructions(&self, value: bool) {
        self.show_instructions.set(value);
    }

    /// Gets the tooltip.
    pub fn tooltip(&self) -> &str {
        &self.tooltip
    }

    /// Sets the tooltip.
    pub fn set_tooltip(&mut self, tooltip: &str) {
        self.tooltip = tooltip.to_string();
    }

    /// Gets the Command that is executed when the button is clicked.
    pub fn command(&self) -> &Command {
        &self.command
    }

    /// Gets the Command that is executed when the mouse instructions
    /// should be shown.
    pub fn show_click(&self) -> &Command {
        &self.show_click
    }

    /// Gets the Command that is executed when the touch instructions
    /// should be shown.
    pub fn show_touch(&self) -> &Command {
        &self.show_touch
    }
}

impl Default for NavigationHelpButtonViewModel {
    fn default() -> Self {
        Self::new()
    }
}
