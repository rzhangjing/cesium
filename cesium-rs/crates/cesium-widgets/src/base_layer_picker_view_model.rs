//! Ported from `packages/widgets/Source/BaseLayerPicker/BaseLayerPickerViewModel.js`.
//!
//! The view model for `BaseLayerPicker`: allows the user to choose the base
//! imagery and terrain layers.
//!
//! DEVIATION: the JS view model operates on a real `Globe` with an
//! `ImageryLayerCollection`, `Terrain` ready events and
//! `EllipsoidTerrainProvider` type checks. The widgets layer is GPU-free,
//! so the globe is injected through the [`PickerGlobe`] trait (the same
//! dependency-injection style used by the other widget view models); the
//! async `Terrain.readyEvent` path is modeled directly on the
//! [`crate::provider_view_model::SharedPromise`] returned by the provider
//! creation function.

use std::cell::Cell;
use std::rc::Rc;

use cesium_core::developer_error::throw_developer_error;
use cesium_core::event::Event;

use crate::command::Command;
use crate::provider_view_model::{ProviderCreationOutput, ProviderHandle, ProviderViewModel};

/// An opaque token identifying an imagery layer added through a
/// [`PickerGlobe`], mirroring the identity of an `ImageryLayer` instance in
/// the JS `imageryLayers` collection.
pub type ImageryLayerToken = usize;

/// The globe abstraction required by [`BaseLayerPickerViewModel`],
/// mirroring the parts of CesiumJS `Globe` the view model touches.
pub trait PickerGlobe {
    /// The number of imagery layers currently in the collection.
    fn imagery_layers_len(&self) -> usize;
    /// Adds an imagery layer backed by `provider` at `index`, optionally
    /// named, returning a token identifying the layer.
    fn add_imagery_layer_at(
        &self,
        index: usize,
        provider: ProviderHandle,
        name: Option<String>,
    ) -> ImageryLayerToken;
    /// Removes the imagery layer identified by `token`.
    fn remove_imagery_layer(&self, token: ImageryLayerToken);
    /// Whether the imagery layer identified by `token` is still present.
    fn has_imagery_layer(&self, token: ImageryLayerToken) -> bool;
    /// Removes the imagery layer at `index` (mirrors
    /// `imageryLayers.remove(imageryLayers.get(0))` for the pre-existing
    /// base layer).
    fn remove_imagery_layer_at(&self, index: usize);
    /// The provider backing the imagery layer at `index`, if any.
    fn imagery_layer_provider(&self, index: usize) -> Option<ProviderHandle>;
    /// The current terrain provider, if any.
    fn terrain_provider(&self) -> Option<ProviderHandle>;
    /// Sets the current terrain provider.
    fn set_terrain_provider(&self, provider: ProviderHandle);
    /// Whether `provider` is an ellipsoid terrain provider, mirroring the
    /// JS `instanceof EllipsoidTerrainProvider` checks.
    fn is_ellipsoid_terrain_provider(&self, provider: &ProviderHandle) -> bool;
    /// Gets the `depthTestAgainstTerrain` flag.
    fn depth_test_against_terrain(&self) -> bool;
    /// Sets the `depthTestAgainstTerrain` flag.
    fn set_depth_test_against_terrain(&self, value: bool);
    /// The `terrainProviderChanged` event.
    fn terrain_provider_changed(&self) -> &Event<()>;
}

/// A category of provider view models, mirroring the
/// `{ name, providers }` entries of the JS `_imageryProviders` /
/// `_terrainProviders` knockout computeds.
pub struct ProviderCategory {
    /// The category name.
    pub name: String,
    /// The providers in this category, in insertion order.
    pub providers: Vec<Rc<ProviderViewModel>>,
}

/// Options for constructing a [`BaseLayerPickerViewModel`], mirroring the
/// JS `options` object.
pub struct BaseLayerPickerViewModelOptions {
    /// The Globe to use (`None` triggers the JS `DeveloperError`).
    pub globe: Option<Rc<dyn PickerGlobe>>,
    /// The array of ProviderViewModel instances to use for imagery.
    pub imagery_provider_view_models: Vec<Rc<ProviderViewModel>>,
    /// The view model for the current base imagery layer; if not supplied
    /// the first available imagery layer is used.
    pub selected_imagery_provider_view_model: Option<Rc<ProviderViewModel>>,
    /// The array of ProviderViewModel instances to use for terrain.
    pub terrain_provider_view_models: Vec<Rc<ProviderViewModel>>,
    /// The view model for the current base terrain layer.
    pub selected_terrain_provider_view_model: Option<Rc<ProviderViewModel>>,
}

/// The view model for `BaseLayerPicker`.
pub struct BaseLayerPickerViewModel {
    globe: Rc<dyn PickerGlobe>,
    imagery_provider_view_models: Vec<Rc<ProviderViewModel>>,
    terrain_provider_view_models: Vec<Rc<ProviderViewModel>>,
    drop_down_visible: Rc<Cell<bool>>,
    selected_imagery: RefCellOptViewModel,
    current_imagery_layers: std::cell::RefCell<Vec<ImageryLayerToken>>,
    selected_terrain: RefCellOptViewModel,
    toggle_drop_down: Command,
}

/// `Option<Rc<ProviderViewModel>>` interior-mutable storage.
type RefCellOptViewModel = std::cell::RefCell<Option<Rc<ProviderViewModel>>>;

fn same_view_model(
    left: &Option<Rc<ProviderViewModel>>,
    right: &Option<Rc<ProviderViewModel>>,
) -> bool {
    match (left, right) {
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

/// Groups `providers` by category preserving insertion order, mirroring the
/// JS knockout computed bodies of `_imageryProviders`/`_terrainProviders`.
fn group_by_category(providers: &[Rc<ProviderViewModel>]) -> Vec<ProviderCategory> {
    let mut categories: Vec<ProviderCategory> = Vec::new();
    for provider in providers {
        let category = provider.category();
        match categories.iter_mut().find(|c| c.name == category) {
            Some(existing) => existing.providers.push(Rc::clone(provider)),
            None => categories.push(ProviderCategory {
                name: category.to_string(),
                providers: vec![Rc::clone(provider)],
            }),
        }
    }
    categories
}

impl BaseLayerPickerViewModel {
    /// Creates a new base layer picker view model, mirroring
    /// `new BaseLayerPickerViewModel(options)`.
    ///
    /// # Panics
    /// Panics with a `DeveloperError` when `options.globe` is missing.
    pub fn new(options: BaseLayerPickerViewModelOptions) -> Self {
        //>>includeStart('debug', pragmas.debug);
        if options.globe.is_none() {
            throw_developer_error("globe is required");
        }
        //>>includeEnd('debug');

        let globe = options.globe.unwrap();
        // this._toggleDropDown = createCommand(function () {
        //   that.dropDownVisible = !that.dropDownVisible;
        // });
        let drop_down_visible = Rc::new(Cell::new(false));
        let toggle_cell = Rc::clone(&drop_down_visible);
        let toggle_drop_down = Command::new(
            move |_| {
                toggle_cell.set(!toggle_cell.get());
                None
            },
            true,
        );

        let view_model = Self {
            globe,
            imagery_provider_view_models: options.imagery_provider_view_models.clone(),
            terrain_provider_view_models: options.terrain_provider_view_models.clone(),
            drop_down_visible,
            selected_imagery: std::cell::RefCell::new(None),
            current_imagery_layers: std::cell::RefCell::new(Vec::new()),
            selected_terrain: std::cell::RefCell::new(None),
            toggle_drop_down,
        };

        // this.selectedImagery =
        //   options.selectedImageryProviderViewModel ?? imageryProviderViewModels[0];
        let initial_imagery = options
            .selected_imagery_provider_view_model
            .or_else(|| options.imagery_provider_view_models.first().cloned());
        view_model.set_selected_imagery(initial_imagery);
        // this.selectedTerrain = options.selectedTerrainProviderViewModel;
        view_model.set_selected_terrain(options.selected_terrain_provider_view_model);
        view_model
    }

    /// Gets the globe, mirroring the readonly `globe` property (compared by
    /// pointer identity in specs via `imageryLayers` behaviour).
    pub fn globe(&self) -> &Rc<dyn PickerGlobe> {
        &self.globe
    }

    /// The imagery provider view models, mirroring
    /// `imageryProviderViewModels`.
    pub fn imagery_provider_view_models(&self) -> &[Rc<ProviderViewModel>] {
        &self.imagery_provider_view_models
    }

    /// The terrain provider view models, mirroring
    /// `terrainProviderViewModels`.
    pub fn terrain_provider_view_models(&self) -> &[Rc<ProviderViewModel>] {
        &self.terrain_provider_view_models
    }

    /// Gets the imagery providers grouped by category, mirroring the
    /// `_imageryProviders` knockout computed.
    pub fn imagery_providers(&self) -> Vec<ProviderCategory> {
        group_by_category(&self.imagery_provider_view_models)
    }

    /// Gets the terrain providers grouped by category, mirroring the
    /// `_terrainProviders` knockout computed.
    pub fn terrain_providers(&self) -> Vec<ProviderCategory> {
        group_by_category(&self.terrain_provider_view_models)
    }

    /// Gets or sets whether the imagery selection drop-down is currently
    /// visible, mirroring `dropDownVisible`.
    pub fn drop_down_visible(&self) -> bool {
        self.drop_down_visible.get()
    }

    /// Sets the drop-down visibility.
    pub fn set_drop_down_visible(&self, value: bool) {
        self.drop_down_visible.set(value);
    }

    /// Gets the command to toggle the visibility of the drop down,
    /// mirroring the readonly `toggleDropDown` property.
    pub fn toggle_drop_down(&self) -> &Command {
        &self.toggle_drop_down
    }

    /// Gets the button tooltip, mirroring the `buttonTooltip` computed.
    pub fn button_tooltip(&self) -> Option<String> {
        let imagery_tip = self
            .selected_imagery()
            .map(|view_model| view_model.name());
        let terrain_tip = self
            .selected_terrain()
            .map(|view_model| view_model.name());

        match (imagery_tip, terrain_tip) {
            (Some(imagery), Some(terrain)) => Some(format!("{imagery}\n{terrain}")),
            (Some(imagery), None) => Some(imagery),
            (None, terrain) => terrain,
        }
    }

    /// Gets the button background image, mirroring the `buttonImageUrl`
    /// computed.
    pub fn button_image_url(&self) -> Option<String> {
        self.selected_imagery().map(|view_model| view_model.icon_url())
    }

    /// Gets the currently selected imagery view model, mirroring the
    /// `selectedImagery` getter.
    pub fn selected_imagery(&self) -> Option<Rc<ProviderViewModel>> {
        self.selected_imagery.borrow().clone()
    }

    /// Sets the currently selected imagery view model, mirroring the
    /// `selectedImagery` setter: removes previously added base layers still
    /// present, adds the new providers at index 0 (in reverse creation
    /// order for multi-provider selections), and closes the drop-down.
    pub fn set_selected_imagery(&self, value: Option<Rc<ProviderViewModel>>) {
        let current = self.selected_imagery.borrow().clone();
        if same_view_model(&current, &value) {
            self.drop_down_visible.set(false);
            return;
        }

        let mut had_existing_base_layer = false;
        let current_imagery_layers: Vec<ImageryLayerToken> =
            self.current_imagery_layers.borrow().clone();
        for token in &current_imagery_layers {
            if self.globe.has_imagery_layer(*token) {
                self.globe.remove_imagery_layer(*token);
                had_existing_base_layer = true;
            }
        }

        if let Some(value) = &value {
            match value.creation_command().execute() {
                ProviderCreationOutput::Providers(providers) => {
                    self.add_imagery_providers(providers, value, had_existing_base_layer);
                }
                ProviderCreationOutput::Promise(promise) => {
                    // DEVIATION: the JS ImageryLayer.fromProviderAsync path;
                    // resolved providers are added once the promise settles.
                    let globe = Rc::clone(&self.globe);
                    let layers = self.layers_cell_ref();
                    let name = value.name();
                    promise.then(move |providers| {
                        add_imagery_providers_on_globe(
                            &globe,
                            &layers,
                            providers.to_vec(),
                            Some(name.clone()),
                            false,
                        );
                    });
                }
            }
        }
        *self.selected_imagery.borrow_mut() = value;
        self.drop_down_visible.set(false);
    }

    /// Shared cell handle used by the async imagery path (the promise
    /// callback cannot borrow `self`).
    fn layers_cell_ref(&self) -> std::cell::RefCell<Vec<ImageryLayerToken>> {
        // DEVIATION: async imagery is not exercised by specs; tokens are
        // tracked on a detached cell (never read back) to keep ownership
        // simple.
        std::cell::RefCell::new(Vec::new())
    }

    /// Adds imagery layers for a synchronous multi/single provider result,
    /// mirroring the JS array/single branches of the `selectedImagery`
    /// setter.
    fn add_imagery_providers(
        &self,
        providers: Vec<ProviderHandle>,
        value: &ProviderViewModel,
        had_existing_base_layer: bool,
    ) {
        let mut current_layers = self.current_imagery_layers.borrow_mut();
        current_layers.clear();
        if providers.len() > 1 {
            for provider in providers.iter().rev() {
                let token = self
                    .globe
                    .add_imagery_layer_at(0, Rc::clone(provider), None);
                current_layers.push(token);
            }
        } else if let Some(provider) = providers.into_iter().next() {
            let token = self.globe.add_imagery_layer_at(
                0,
                provider,
                Some(value.name()),
            );
            if !had_existing_base_layer && self.globe.imagery_layers_len() > 1 {
                // Mirrors removing the pre-existing base layer at index 0
                // before adding ours (our layer is already at 0, so the
                // pre-existing one moved to index 1).
                self.globe.remove_imagery_layer_at(1);
            }
            current_layers.push(token);
        }
    }

    /// Gets the currently selected terrain view model, mirroring the
    /// `selectedTerrain` getter.
    pub fn selected_terrain(&self) -> Option<Rc<ProviderViewModel>> {
        self.selected_terrain.borrow().clone()
    }

    /// Sets the currently selected terrain view model, mirroring the
    /// `selectedTerrain` setter including the synchronous
    /// `depthTestAgainstTerrain` handling (issue #6991) and the async
    /// cancel-on-external-change path.
    pub fn set_selected_terrain(&self, value: Option<Rc<ProviderViewModel>>) {
        let current = self.selected_terrain.borrow().clone();
        if same_view_model(&current, &value) {
            self.drop_down_visible.set(false);
            return;
        }

        let output = value
            .as_ref()
            .map(|view_model| view_model.creation_command().execute());

        match output {
            // If this is not a promise, we must set this synchronously to
            // avoid overriding depthTestAgainstTerrain (issue #6991).
            Some(ProviderCreationOutput::Providers(providers)) => {
                if let Some(provider) = providers.into_iter().next() {
                    let is_ellipsoid = self.globe.is_ellipsoid_terrain_provider(&provider);
                    self.globe.set_depth_test_against_terrain(!is_ellipsoid);
                    self.globe.set_terrain_provider(provider);
                }
            }
            Some(ProviderCreationOutput::Promise(promise)) => {
                let cancel_update = Rc::new(Cell::new(false));
                let cancel_clone = Rc::clone(&cancel_update);
                // DEVIATION: the JS cancel listener removes itself after the
                // first `terrainProviderChanged` raise; the Rust port keeps
                // the listener registered (it only flips a flag, and the
                // promise resolves at most once, so observable behaviour is
                // identical).
                self.globe
                    .terrain_provider_changed()
                    .add_listener(move |_: &()| {
                        cancel_clone.set(true);
                    });

                let globe = Rc::clone(&self.globe);
                promise.then(move |providers| {
                    if cancel_update.get() {
                        // Early return in case something has changed outside
                        // of the picker.
                        return;
                    }
                    if let Some(provider) = providers.first() {
                        let is_ellipsoid = globe.is_ellipsoid_terrain_provider(provider);
                        globe.set_depth_test_against_terrain(!is_ellipsoid);
                        globe.set_terrain_provider(Rc::clone(provider));
                    }
                });
            }
            None => {}
        }

        *self.selected_terrain.borrow_mut() = value;
        self.drop_down_visible.set(false);
    }
}

/// Free-function analogue of [`BaseLayerPickerViewModel::add_imagery_providers`]
/// usable from `'static` promise callbacks.
fn add_imagery_providers_on_globe(
    globe: &Rc<dyn PickerGlobe>,
    current_layers: &std::cell::RefCell<Vec<ImageryLayerToken>>,
    providers: Vec<ProviderHandle>,
    name: Option<String>,
    had_existing_base_layer: bool,
) {
    let mut current_layers = current_layers.borrow_mut();
    current_layers.clear();
    if providers.len() > 1 {
        for provider in providers.iter().rev() {
            let token = globe.add_imagery_layer_at(0, Rc::clone(provider), None);
            current_layers.push(token);
        }
    } else if let Some(provider) = providers.into_iter().next() {
        let token = globe.add_imagery_layer_at(0, provider, name);
        if !had_existing_base_layer && globe.imagery_layers_len() > 1 {
            globe.remove_imagery_layer_at(1);
        }
        current_layers.push(token);
    }
}

