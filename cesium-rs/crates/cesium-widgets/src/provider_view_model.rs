//! Ported from `packages/widgets/Source/BaseLayerPicker/ProviderViewModel.js`.
//!
//! A view model that represents each item in the `BaseLayerPicker`.
//!
//! DEVIATION: the JS `name`/`tooltip`/`iconUrl` properties are knockout
//! observables or plain values; the Rust port models both cases with
//! [`StringProp`]. The JS `creationFunction` may be a plain function or a
//! `Command`; the Rust port models both with a plain closure since the
//! crate's [`crate::command::Command`] carries `serde_json::Value` results
//! and cannot transport typed provider handles. The JS asynchronous
//! creation (promise-returning `creationFunction`) is modeled with
//! [`SharedPromise`], a manually-resolvable shared promise analogue.

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use cesium_core::developer_error::throw_developer_error;

use crate::observables::ObservableCell;

/// An opaque provider instance created by a [`ProviderViewModel`]'s
/// creation function (imagery or terrain provider). The widgets layer is
/// agnostic of the concrete provider type, mirroring how CesiumJS treats
/// the creation result opaquely (`ImageryProvider | TerrainProvider`).
pub type ProviderHandle = Rc<dyn Any>;

/// A string property that may be a plain value or a knockout observable,
/// mirroring the dual `with observables` / `with values` constructor forms
/// of the JS `ProviderViewModel`.
#[derive(Clone)]
pub enum StringProp {
    /// A plain string value.
    Value(String),
    /// A knockout observable string.
    Observable(ObservableCell<String>),
}

impl StringProp {
    /// Reads the current value, mirroring `options.name()` for observables
    /// and `options.name` for values.
    pub fn get(&self) -> String {
        match self {
            StringProp::Value(value) => value.clone(),
            StringProp::Observable(observable) => observable.get(),
        }
    }
}

/// The output of a [`ProviderViewModel`] creation function, mirroring the
/// JS return type `provider | provider[] | Promise<provider | provider[]>`.
pub enum ProviderCreationOutput {
    /// One or more providers created synchronously.
    Providers(Vec<ProviderHandle>),
    /// A promise that resolves with one or more providers.
    Promise(SharedPromise),
}

struct PromiseInner {
    resolved: Option<Vec<ProviderHandle>>,
    callbacks: Vec<Box<dyn FnOnce(&[ProviderHandle])>>,
}

/// A manually-resolvable promise analogue shared by clone, mirroring the
/// promise returned by an async JS `creationFunction`. All clones observe
/// the same resolution, so a creation function returning clones of one
/// [`SharedPromise`] lets both the caller and the view model react to the
/// same resolution event.
#[derive(Clone)]
pub struct SharedPromise {
    inner: Rc<RefCell<PromiseInner>>,
}

impl Default for SharedPromise {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedPromise {
    /// Creates a new pending promise.
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(PromiseInner {
                resolved: None,
                callbacks: Vec::new(),
            })),
        }
    }

    /// Whether the promise has been resolved.
    pub fn is_resolved(&self) -> bool {
        self.inner.borrow().resolved.is_some()
    }

    /// Resolves the promise with `providers`, invoking all registered
    /// `then` callbacks, mirroring `resolve(...)` on a JS promise.
    pub fn resolve(&self, providers: Vec<ProviderHandle>) {
        let callbacks = {
            let mut inner = self.inner.borrow_mut();
            if inner.resolved.is_some() {
                return;
            }
            inner.resolved = Some(providers);
            std::mem::take(&mut inner.callbacks)
        };
        let resolved = self.inner.borrow().resolved.clone();
        if let Some(resolved) = resolved {
            for callback in callbacks {
                callback(&resolved);
            }
        }
    }

    /// Registers a callback invoked with the resolved providers, firing
    /// immediately when the promise is already resolved, mirroring
    /// `promise.then(...)`.
    pub fn then(&self, callback: impl FnOnce(&[ProviderHandle]) + 'static) {
        let resolved = self.inner.borrow().resolved.clone();
        match resolved {
            Some(providers) => callback(&providers),
            None => self.inner.borrow_mut().callbacks.push(Box::new(callback)),
        }
    }
}

/// The creation command of a [`ProviderViewModel`], mirroring the JS
/// `creationCommand` (a `Command` wrapping the `creationFunction`).
pub struct CreationCommand {
    func: Rc<dyn Fn() -> ProviderCreationOutput>,
}

impl CreationCommand {
    /// Creates a creation command wrapping `func`.
    pub fn new(func: Rc<dyn Fn() -> ProviderCreationOutput>) -> Self {
        Self { func }
    }

    /// Executes the creation function, mirroring invoking the JS
    /// `creationCommand()`.
    pub fn execute(&self) -> ProviderCreationOutput {
        (self.func)()
    }
}

/// Options for constructing a [`ProviderViewModel`], mirroring the JS
/// `options` object. Missing required fields trigger the same
/// `DeveloperError`s as CesiumJS at [`ProviderViewModel::new`].
pub struct ProviderViewModelOptions {
    /// The name of the layer (observable or value).
    pub name: Option<StringProp>,
    /// The tooltip to show when the item is moused over.
    pub tooltip: Option<StringProp>,
    /// An icon representing the layer.
    pub icon_url: Option<StringProp>,
    /// A category for the layer.
    pub category: Option<String>,
    /// A function that creates one or more providers which will be added
    /// to the globe when this item is selected.
    pub creation_function: Option<Rc<dyn Fn() -> ProviderCreationOutput>>,
}

/// A view model that represents each item in the `BaseLayerPicker`.
pub struct ProviderViewModel {
    name: StringProp,
    tooltip: StringProp,
    icon_url: StringProp,
    category: String,
    creation_command: CreationCommand,
}

impl ProviderViewModel {
    /// Creates a new provider view model, mirroring
    /// `new ProviderViewModel(options)`.
    ///
    /// # Panics
    /// Panics with a `DeveloperError` when `name`, `tooltip`, `icon_url`,
    /// or `creation_function` is missing.
    pub fn new(options: ProviderViewModelOptions) -> Self {
        //>>includeStart('debug', pragmas.debug);
        if options.name.is_none() {
            throw_developer_error("options.name is required.");
        }
        if options.tooltip.is_none() {
            throw_developer_error("options.tooltip is required.");
        }
        if options.icon_url.is_none() {
            throw_developer_error("options.iconUrl is required.");
        }
        if options.creation_function.is_none() {
            throw_developer_error("options.creationFunction is required.");
        }
        //>>includeEnd('debug');

        Self {
            name: options.name.unwrap(),
            tooltip: options.tooltip.unwrap(),
            icon_url: options.icon_url.unwrap(),
            category: options.category.unwrap_or_default(),
            creation_command: CreationCommand::new(options.creation_function.unwrap()),
        }
    }

    /// Gets the display name, mirroring the observable `name` property.
    pub fn name(&self) -> String {
        self.name.get()
    }

    /// Gets the tooltip, mirroring the observable `tooltip` property.
    pub fn tooltip(&self) -> String {
        self.tooltip.get()
    }

    /// Gets the icon url, mirroring the observable `iconUrl` property.
    pub fn icon_url(&self) -> String {
        self.icon_url.get()
    }

    /// Gets the category, mirroring the readonly `category` property.
    pub fn category(&self) -> &str {
        &self.category
    }

    /// Gets the command that creates one or more providers, mirroring the
    /// readonly `creationCommand` property.
    pub fn creation_command(&self) -> &CreationCommand {
        &self.creation_command
    }
}
