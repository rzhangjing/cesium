//! Ported from `packages/widgets/Source/createCommand.js`.

use crate::command::Command;

/// Creates a new command with the given callback and enabled state.
///
/// In CesiumJS, `createCommand(callback, [container])` creates a Command
/// object that wraps the callback function with an `enabled` observable.
pub fn create_command<F: Fn() + Send + Sync + 'static>(callback: F, enabled: bool) -> Command {
    let mut cmd = Command::new(callback);
    cmd.set_enabled(enabled);
    cmd
}
