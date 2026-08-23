//! Ported from `packages/widgets/Source/command.js`.
//!
//! A command is a function with an enabled state.

use std::sync::Arc;

/// A command is a callable object with an enabled state.
///
/// In CesiumJS, a Command is created by `createCommand(callback, [container])`.
/// It wraps a function and adds an `enabled` observable property.
/// When the command is executed, it calls the wrapped function only if enabled.
///
/// Commands are used throughout the Viewer to bind UI actions to logic:
/// - `playCommand` in ClockViewModel
/// - `goHome` in HomeButtonViewModel
/// - `search` in GeocoderViewModel
/// - etc.
pub struct Command {
    /// The callback function to execute.
    callback: Arc<dyn Fn() + Send + Sync>,
    /// Whether this command is currently enabled.
    pub enabled: bool,
}

impl Command {
    /// Creates a new command with the given callback.
    pub fn new<F: Fn() + Send + Sync + 'static>(callback: F) -> Self {
        Self {
            callback: Arc::new(callback),
            enabled: true,
        }
    }

    /// Creates a no-op command.
    pub fn empty() -> Self {
        Self {
            callback: Arc::new(|| {}),
            enabled: false,
        }
    }

    /// Executes the command if enabled.
    pub fn execute(&self) {
        if self.enabled {
            (self.callback)();
        }
    }

    /// Returns whether this command is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Sets whether this command is enabled.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

impl Default for Command {
    fn default() -> Self {
        Self::empty()
    }
}
