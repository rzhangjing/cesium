//! Ported from `packages/widgets/Source/ToggleButtonViewModel.js`.
//!
//! A view model which exposes the properties of a toggle button.

use crate::command::Command;

/// Options for [`ToggleButtonViewModel::new`], mirroring the JS `options`
/// object (`{ toggled, tooltip }`, both optional). The JS constructor also
/// accepts knockout computeds for both properties (as `AnimationViewModel`
/// passes); those are modeled by the `*_computed` fields, which win over
/// the plain values when present.
#[derive(Default)]
pub struct ToggleButtonViewModelOptions {
    /// A boolean indicating whether the button should be initially toggled
    /// (`options.toggled`, default `false`).
    pub toggled: Option<bool>,
    /// A string containing the button's tooltip (`options.tooltip`,
    /// default `""`).
    pub tooltip: Option<String>,
    /// A knockout computed analogue for `options.toggled`
    /// (evaluated on read; takes precedence over `toggled`).
    pub toggled_computed: Option<Box<dyn Fn() -> bool>>,
    /// A knockout computed analogue for `options.tooltip`
    /// (evaluated on read; takes precedence over `tooltip`).
    pub tooltip_computed: Option<Box<dyn Fn() -> String>>,
}

/// A view model which exposes the properties of a toggle button.
///
/// In CesiumJS the `toggled`/`tooltip` properties are knockout observables
/// and the constructor also accepts knockout computeds for them (as
/// `AnimationViewModel` does). The Rust port models both shapes with
/// `Box<dyn Fn>` providers; static values are wrapped in constant closures.
pub struct ToggleButtonViewModel {
    command: Command,
    toggled: Box<dyn Fn() -> bool>,
    tooltip: Box<dyn Fn() -> String>,
}

impl ToggleButtonViewModel {
    /// Port of `new ToggleButtonViewModel(command, options)` with plain
    /// boolean/string option values.
    ///
    /// DEVIATION: the JS `command is required.` DeveloperError is enforced
    /// by the type system (`command` is a required parameter).
    pub fn new(command: Command, options: ToggleButtonViewModelOptions) -> Self {
        let toggled_value = options.toggled.unwrap_or(false);
        let tooltip_value = options.tooltip.unwrap_or_default();
        let toggled: Box<dyn Fn() -> bool> = match options.toggled_computed {
            Some(computed) => computed,
            None => Box::new(move || toggled_value),
        };
        let tooltip: Box<dyn Fn() -> String> = match options.tooltip_computed {
            Some(computed) => computed,
            None => Box::new(move || tooltip_value.clone()),
        };
        Self {
            command,
            toggled,
            tooltip,
        }
    }

    /// Variant of [`Self::new`] accepting computed providers, mirroring the
    /// CesiumJS call sites that pass `knockout.computed(...)` for
    /// `options.toggled`/`options.tooltip` (e.g. `AnimationViewModel`).
    pub fn new_with_computed<T, P>(command: Command, toggled: T, tooltip: P) -> Self
    where
        T: Fn() -> bool + 'static,
        P: Fn() -> String + 'static,
    {
        Self {
            command,
            toggled: Box::new(toggled),
            tooltip: Box::new(tooltip),
        }
    }

    /// Gets whether the button is currently toggled (mirrors reading the
    /// `toggled` observable).
    pub fn toggled(&self) -> bool {
        (self.toggled)()
    }

    /// Sets the toggled state, replacing any computed provider (mirrors
    /// writing the `toggled` observable).
    pub fn set_toggled(&mut self, value: bool) {
        self.toggled = Box::new(move || value);
    }

    /// Gets the button's tooltip (mirrors reading the `tooltip`
    /// observable).
    pub fn tooltip(&self) -> String {
        (self.tooltip)()
    }

    /// Sets the tooltip, replacing any computed provider (mirrors writing
    /// the `tooltip` observable).
    pub fn set_tooltip(&mut self, value: &str) {
        let value = value.to_string();
        self.tooltip = Box::new(move || value.clone());
    }

    /// Gets the command which will be executed when the button is toggled
    /// (mirrors the read-only `command` property).
    pub fn command(&self) -> &Command {
        &self.command
    }
}
