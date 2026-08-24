//! Ported from `packages/widgets/Source/createCommand.js`.
//!
//! A Command is a function with an extra `canExecute` observable property
//! to determine whether the command can be executed. When executed, a
//! Command checks the value of `canExecute` and throws if false. It also
//! provides events for when a command has been or is about to be executed.
//!
//! DEVIATION: the JS command is a callable function object; the Rust port
//! is a struct invoked through [`Command::call`]/[`Command::execute`]. JS
//! `arguments` are modeled as [`serde_json::Value`]s; the JS knockout
//! observable `canExecute` is modeled as a settable flag plus an optional
//! computed provider (see [`Command::new_with_can_execute_provider`]).

use std::cell::Cell;

use cesium_core::developer_error::throw_developer_error;
use cesium_core::event::Event;

/// The info object passed to `beforeExecute` listeners, mirroring the JS
/// `{ args: arguments, cancel: false }` object. Listeners may set `cancel`
/// to prevent execution.
#[derive(Debug)]
pub struct CommandInfo {
    /// The arguments the command was invoked with.
    pub args: Vec<serde_json::Value>,
    /// Whether execution should be cancelled. Listeners may flip this via
    /// [`Cell::set`] (Rust analogue of `info.cancel = true`, since
    /// [`Event`] listeners receive a shared reference).
    pub cancel: Cell<bool>,
}

/// A command: a callable object with an enabled state and pre/post
/// execution events. The Rust analogue of the function object returned by
/// CesiumJS `createCommand`.
pub struct Command {
    func: Box<dyn Fn(&[serde_json::Value]) -> Option<serde_json::Value>>,
    /// Settable `canExecute` flag (JS knockout-tracked observable analogue).
    can_execute_flag: Cell<bool>,
    /// Optional computed `canExecute` provider, mirroring the JS pattern of
    /// passing a knockout observable/computed as `canExecute` (e.g.
    /// `knockout.getObservable(this, "isFullscreenEnabled")` in
    /// `FullscreenButtonViewModel`). When present it wins over the flag.
    can_execute_provider: Option<Box<dyn Fn() -> bool>>,
    /// Raised before the wrapped function executes, with a [`CommandInfo`].
    pub before_execute: Event<CommandInfo>,
    /// Raised after the wrapped function executes, with its result
    /// (`serde_json::Value::Null` when the function returns `None`).
    pub after_execute: Event<serde_json::Value>,
}

impl Command {
    /// Creates a command wrapping `func` with the given initial
    /// `can_execute` state. Mirrors `createCommand(func, canExecute)`.
    pub fn new<F>(func: F, can_execute: bool) -> Self
    where
        F: Fn(&[serde_json::Value]) -> Option<serde_json::Value> + 'static,
    {
        Self {
            func: Box::new(func),
            can_execute_flag: Cell::new(can_execute),
            can_execute_provider: None,
            before_execute: Event::new(),
            after_execute: Event::new(),
        }
    }

    /// Creates a command whose `canExecute` state is computed dynamically,
    /// mirroring `createCommand(func, knockoutObservable)` where the
    /// observable is a knockout computed/observable reference.
    pub fn new_with_can_execute_provider<F, P>(func: F, can_execute_provider: P) -> Self
    where
        F: Fn(&[serde_json::Value]) -> Option<serde_json::Value> + 'static,
        P: Fn() -> bool + 'static,
    {
        Self {
            func: Box::new(func),
            can_execute_flag: Cell::new(true),
            can_execute_provider: Some(Box::new(can_execute_provider)),
            before_execute: Event::new(),
            after_execute: Event::new(),
        }
    }

    /// Creates a no-op command that is enabled by default.
    ///
    /// Convenience constructor used by view models that replace the command
    /// body during construction; has no direct CesiumJS counterpart.
    pub fn empty() -> Self {
        Self::new(|_| None, true)
    }

    /// Reads the current `canExecute` state (provider first, then flag),
    /// mirroring reading the `command.canExecute` observable.
    pub fn can_execute(&self) -> bool {
        match &self.can_execute_provider {
            Some(provider) => provider(),
            None => self.can_execute_flag.get(),
        }
    }

    /// Sets the `canExecute` flag, mirroring writing the `canExecute`
    /// observable. Has no effect when a computed provider is installed
    /// (the JS observable bound to the computed would behave the same).
    pub fn set_can_execute(&self, value: bool) {
        self.can_execute_flag.set(value);
    }

    /// Executes the command, mirroring invoking the JS command function
    /// with the given `arguments`.
    ///
    /// # Panics
    /// Panics with a `DeveloperError` when `can_execute` is false (the JS
    /// check lives in a debug pragma block, hence the debug-only guard).
    pub fn call(&self, args: &[serde_json::Value]) -> Option<serde_json::Value> {
        #[cfg(debug_assertions)]
        if !self.can_execute() {
            throw_developer_error("Cannot execute command, canExecute is false.");
        }
        #[cfg(not(debug_assertions))]
        let _ = &self.can_execute_flag;

        let command_info = CommandInfo {
            args: args.to_vec(),
            cancel: Cell::new(false),
        };
        self.before_execute.raise_event(&command_info);

        if !command_info.cancel.get() {
            let result = (self.func)(args);
            self.after_execute
                .raise_event(&result.clone().unwrap_or(serde_json::Value::Null));
            return result;
        }
        None
    }

    /// Executes the command with no arguments.
    ///
    /// # Panics
    /// Panics with a `DeveloperError` when `can_execute` is false.
    pub fn execute(&self) -> Option<serde_json::Value> {
        self.call(&[])
    }
}
