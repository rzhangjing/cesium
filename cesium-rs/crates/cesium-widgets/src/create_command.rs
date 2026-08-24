//! Ported from `packages/widgets/Source/createCommand.js`.
//!
//! Create a Command from a given function, for use with ViewModels.
//!
//! A Command is a function with an extra `canExecute` observable property
//! to determine whether the command can be executed. When executed, a
//! Command function will check the value of `canExecute` and throw if
//! false. It also provides events for when a command has been or is about
//! to be executed.

use crate::command::Command;

/// Create a Command from a given function, for use with ViewModels.
///
/// Mirrors `createCommand(func, canExecute)`; `can_execute` defaults to
/// `true` when `None`, mirroring `canExecute = canExecute ?? true`.
///
/// DEVIATION: the JS `func is required.` DeveloperError is enforced by the
/// type system (`func` is a required parameter).
pub fn create_command<F>(func: F, can_execute: Option<bool>) -> Command
where
    F: Fn(&[serde_json::Value]) -> Option<serde_json::Value> + 'static,
{
    Command::new(func, can_execute.unwrap_or(true))
}

/// Create a Command whose `canExecute` state is computed dynamically.
///
/// Mirrors `createCommand(func, knockoutObservable)` — the CesiumJS call
/// sites that pass a knockout observable/computed instead of a plain
/// boolean (e.g. `FullscreenButtonViewModel`, the realtime button of
/// `AnimationViewModel`).
pub fn create_command_with_can_execute_provider<F, P>(func: F, can_execute_provider: P) -> Command
where
    F: Fn(&[serde_json::Value]) -> Option<serde_json::Value> + 'static,
    P: Fn() -> bool + 'static,
{
    Command::new_with_can_execute_provider(func, can_execute_provider)
}
