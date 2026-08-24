//! Ported from `packages/widgets/Source/I3SBuildingSceneLayerExplorer/I3SBuildingSceneLayerExplorerViewModel.js`.
//!
//! The view model for the `I3SBuildingSceneLayerExplorer` widget:
//! presents the sublayer tree of an I3S building scene layer and lets
//! the user switch between the "Full Model" and "Overview" top layers
//! and filter by building level.
//!
//! DEVIATION: the JS view model looks up the `#bsl-wrapper` DOM element
//! (`document.getElementById`) to show/hide the layer tree; the widgets
//! layer is DOM-free, so the element is injected through the
//! [`BslWrapperElement`] trait. The JS `expandClickHandler` /
//! `setOptionDisable` knockout binding helpers are DOM-only and have no
//! Rust counterpart. Knockout `undefined` initial observable values are
//! modeled as `BuildingLevel::All` / `None`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::observables::ObservableCell;

/// A building level filter value: `"All"` or a numeric level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingLevel {
    /// The `"All"` pseudo-level.
    All,
    /// A numeric building level (`BldgLevel` attribute value).
    Level(i64),
}

/// An attribute filter passed to
/// `I3SDataProvider.filterByAttributes`, mirroring the JS
/// `{ name, values }` objects.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeFilter {
    /// The attribute name (e.g. `"BldgLevel"`).
    pub name: String,
    /// The attribute values to filter by.
    pub values: Vec<i64>,
}

/// An input sublayer of the I3S provider, mirroring the JS
/// `i3sProvider.sublayers` tree entries.
#[derive(Debug, Clone)]
pub struct I3sSublayerInput {
    /// The sublayer name.
    pub name: String,
    /// The sublayer model name (`"FullModel"` / `"Overview"` for top
    /// layers); may be absent for nested categories.
    pub model_name: Option<String>,
    /// Whether the sublayer is visible.
    pub visibility: bool,
    /// The nested sublayers.
    pub sublayers: Vec<I3sSublayerInput>,
}

/// The I3S provider abstraction required by the view model, mirroring
/// the parts of `I3SDataProvider` it touches.
pub trait I3sProviderLike {
    /// The sublayer tree (`i3sProvider.sublayers`).
    fn sublayers(&self) -> Vec<I3sSublayerInput>;
    /// `i3sProvider.getAttributeNames()`.
    fn attribute_names(&self) -> Vec<String>;
    /// `i3sProvider.getAttributeValues(attribute)`.
    fn attribute_values(&self, attribute: &str) -> Vec<i64>;
    /// `i3sProvider.filterByAttributes(filters)`; an empty slice mirrors
    /// the argument-less `filterByAttributes()` call that clears the
    /// filter.
    fn filter_by_attributes(&self, filters: &[AttributeFilter]);
    /// Writing `i3sProvider.show` (used by the synthesized "Full Model"
    /// layer when the provider has no top layers).
    fn set_show(&self, show: bool);
}

/// The `#bsl-wrapper` element abstraction, mirroring the
/// `style.display` writes the JS view model performs on
/// `document.getElementById("bsl-wrapper")`.
pub trait BslWrapperElement {
    /// Sets `element.style.display`.
    fn set_style_display(&self, display: &str);
}

/// A node of the view model's sublayer tree (the JS objects mutated in
/// place by `trackSublayer` / `addTopLayer` / `handleTopLayerSelector`).
#[derive(Debug)]
pub struct SublayerNode {
    /// The sublayer name.
    pub name: String,
    /// The sublayer model name, if any.
    pub model_name: Option<String>,
    /// Whether the sublayer is visible.
    pub visibility: bool,
    /// The nested sublayers.
    pub sublayers: Vec<Rc<RefCell<SublayerNode>>>,
}

/// An entry of the top-layer selector (`viewModel.topLayers`),
/// mirroring the JS `{ name, modelName, disable, index }` objects.
#[derive(Debug, Clone, PartialEq)]
pub struct TopLayer {
    /// The display name.
    pub name: String,
    /// The model name.
    pub model_name: String,
    /// Whether the option is disabled (JS knockout observable; modeled
    /// as a plain flag since it only feeds knockout DOM bindings).
    pub disable: bool,
    /// Index into [`I3sBuildingSceneLayerExplorerViewModel::sublayers`]
    /// (-1 for the placeholder entry).
    pub index: isize,
}

/// The value written to the `currentLayer` observable, mirroring the JS
/// `{ name, modelName, index }` objects produced by the dropdown.
#[derive(Debug, Clone, PartialEq)]
pub struct TopLayerSelection {
    /// The display name.
    pub name: String,
    /// The model name.
    pub model_name: String,
    /// Index into [`I3sBuildingSceneLayerExplorerViewModel::sublayers`].
    pub index: usize,
}

fn is_full_model(model_name: &str) -> bool {
    model_name == "FullModel"
}

fn is_overview(model_name: &str) -> bool {
    model_name == "Overview"
}

fn is_top_layer(model_name: &str) -> bool {
    is_overview(model_name) || is_full_model(model_name)
}

fn convert_input(input: I3sSublayerInput) -> SublayerNode {
    SublayerNode {
        name: input.name,
        model_name: input.model_name,
        visibility: input.visibility,
        sublayers: input
            .sublayers
            .into_iter()
            .map(|child| Rc::new(RefCell::new(convert_input(child))))
            .collect(),
    }
}

/// The view model for the `I3SBuildingSceneLayerExplorer` widget.
///
/// DEVIATION: the JS constructor returns the inner `this.viewModel`
/// object; the Rust port exposes those members directly on the view
/// model.
pub struct I3sBuildingSceneLayerExplorerViewModel {
    /// The sublayer tree (`viewModel.sublayers`).
    sublayers: Vec<Rc<RefCell<SublayerNode>>>,
    /// The top-layer selector entries (`viewModel.topLayers`).
    top_layers: Vec<TopLayer>,
    /// The available levels (`viewModel.levels`).
    levels: Vec<BuildingLevel>,
    /// The level selected before switching to the Overview
    /// (`viewModel.selectedLevel`).
    selected_level: Rc<RefCell<BuildingLevel>>,
    /// The current level filter (`viewModel.currentLevel` observable).
    current_level: ObservableCell<BuildingLevel>,
    /// The currently selected top layer (`viewModel.currentLayer`
    /// observable).
    current_layer: ObservableCell<Option<TopLayerSelection>>,
    /// The default top layer (`viewModel.defaultLayer`).
    default_layer: Option<TopLayer>,
}

impl I3sBuildingSceneLayerExplorerViewModel {
    /// Creates a new view model for the given I3S provider.
    ///
    /// DEVIATION: the JS constructor looks up the `#bsl-wrapper` DOM
    /// element itself; the Rust port takes it as an injected argument
    /// (may be `None`, in which case the show/hide writes are skipped).
    pub fn new(
        provider: Rc<dyn I3sProviderLike>,
        bsl_wrapper: Option<Rc<dyn BslWrapperElement>>,
    ) -> Self {
        // this.viewModel = { ... topLayers: [{ placeholder }] ... };
        let mut top_layers = vec![TopLayer {
            name: String::from("Select a layer to explore..."),
            model_name: String::new(),
            disable: true,
            index: -1,
        }];
        let mut sublayers: Vec<Rc<RefCell<SublayerNode>>> = Vec::new();
        let mut default_layer: Option<TopLayer> = None;

        // Setting a sublayers tree to the viewModel
        let input_sublayers = provider.sublayers();
        let mut all_nodes: Vec<Rc<RefCell<SublayerNode>>> = Vec::new();
        for input in input_sublayers {
            // trackSublayer — DEVIATION: knockout tracking is modeled by
            // the ObservableCell-based members of the view model.
            let node = Rc::new(RefCell::new(convert_input(input)));
            all_nodes.push(Rc::clone(&node));
            if let Some(top_layer) = Self::add_top_layer(&node, &mut top_layers, &mut sublayers) {
                if is_overview(&top_layer.model_name)
                    || (default_layer.is_none() && is_full_model(&top_layer.model_name))
                {
                    default_layer = Some(top_layer);
                }
            }
        }

        // There is no Full Model and/or Overview
        let sync_provider_show;
        if top_layers.len() == 1 && !all_nodes.is_empty() {
            provider.set_show(false);
            // fullModel.sublayers = i3sProvider.sublayers (all input
            // nodes, not just registered top layers)
            let full_model = SublayerNode {
                name: String::from("Full Model"),
                model_name: Some(String::from("FullModel")),
                visibility: false, // visibility: i3sProvider.show
                sublayers: all_nodes,
            };
            let node = Rc::new(RefCell::new(full_model));
            default_layer = Self::add_top_layer(&node, &mut top_layers, &mut sublayers);
            sync_provider_show = true;
        } else if top_layers.len() == 1 {
            top_layers[0].name = String::from("Building layers not found");
            sync_provider_show = false;
        } else {
            sync_provider_show = false;
        }

        // Setting levels to the viewModel
        let levels = Self::set_levels(provider.as_ref());

        let selected_level = Rc::new(RefCell::new(BuildingLevel::All));
        let current_level = ObservableCell::new(BuildingLevel::All);
        let current_layer = ObservableCell::new(None);

        // Filtering by levels
        let provider_for_level = Rc::clone(&provider);
        current_level.subscribe(move |value: &BuildingLevel| {
            match value {
                BuildingLevel::Level(level) => {
                    provider_for_level.filter_by_attributes(&[AttributeFilter {
                        name: String::from("BldgLevel"),
                        values: vec![*level],
                    }]);
                }
                BuildingLevel::All => provider_for_level.filter_by_attributes(&[]),
            }
        });

        // Handling change of a layer for exploring
        let sublayers_for_layer = sublayers.clone();
        let selected_level_for_layer = Rc::clone(&selected_level);
        let current_level_for_layer = current_level.clone();
        let bsl_for_layer = bsl_wrapper.clone();
        let provider_for_show = Rc::clone(&provider);
        current_layer.subscribe(move |selection: &Option<TopLayerSelection>| {
            if let Some(layer) = selection {
                if is_top_layer(&layer.model_name) {
                    for sublayer in &sublayers_for_layer {
                        sublayer.borrow_mut().visibility = false;
                    }
                    if let Some(sublayer) = sublayers_for_layer.get(layer.index) {
                        sublayer.borrow_mut().visibility = true;
                    }
                    if let Some(element) = &bsl_for_layer {
                        if is_full_model(&layer.model_name) {
                            // viewModel.currentLevel = viewModel.selectedLevel;
                            let selected = *selected_level_for_layer.borrow();
                            current_level_for_layer
                                .set_with_comparer(selected, |left, right| left == right);
                            element.set_style_display("block");
                        } else {
                            // viewModel.selectedLevel = viewModel.currentLevel;
                            *selected_level_for_layer.borrow_mut() =
                                current_level_for_layer.get();
                            current_level_for_layer
                                .set_with_comparer(BuildingLevel::All, |left, right| {
                                    left == right
                                });
                            element.set_style_display("none");
                        }
                    } else if is_full_model(&layer.model_name) {
                        let selected = *selected_level_for_layer.borrow();
                        current_level_for_layer
                            .set_with_comparer(selected, |left, right| left == right);
                    } else {
                        *selected_level_for_layer.borrow_mut() = current_level_for_layer.get();
                        current_level_for_layer
                            .set_with_comparer(BuildingLevel::All, |left, right| left == right);
                    }
                }
                if sync_provider_show {
                    provider_for_show.set_show(is_full_model(&layer.model_name));
                }
            }
        });

        Self {
            sublayers,
            top_layers,
            levels,
            selected_level,
            current_level,
            current_layer,
            default_layer,
        }
    }

    /// Mirrors `addTopLayer(layer, viewModel)`: registers the node as a
    /// top layer when its model name is `"FullModel"` / `"Overview"`,
    /// hiding the layer itself and showing its direct children.
    fn add_top_layer(
        node: &Rc<RefCell<SublayerNode>>,
        top_layers: &mut Vec<TopLayer>,
        sublayers: &mut Vec<Rc<RefCell<SublayerNode>>>,
    ) -> Option<TopLayer> {
        let (name, model_name) = {
            let borrowed = node.borrow();
            (
                borrowed.name.clone(),
                borrowed.model_name.clone().unwrap_or_default(),
            )
        };
        if !is_top_layer(&model_name) {
            return None;
        }

        // layer.visibility = false;
        node.borrow_mut().visibility = false;
        // layer.sublayers[i].visibility = true;
        for child in &node.borrow().sublayers {
            child.borrow_mut().visibility = true;
        }

        let top_layer = TopLayer {
            name,
            model_name,
            disable: false,
            index: sublayers.len() as isize,
        };
        top_layers.push(top_layer.clone());
        sublayers.push(Rc::clone(node));
        Some(top_layer)
    }

    /// Mirrors `setLevels(i3sProvider, levels)`: collects the numeric
    /// `BldgLevel` attribute values (sorted) with `"All"` prepended.
    fn set_levels(provider: &dyn I3sProviderLike) -> Vec<BuildingLevel> {
        let mut levels = Vec::new();
        // DEVIATION: the JS wraps the attribute lookups in try/catch;
        // the Rust trait methods are infallible.
        let attributes = provider.attribute_names();
        for attribute in &attributes {
            if attribute == "BldgLevel" {
                let values = provider.attribute_values(attribute);
                for value in values {
                    levels.push(BuildingLevel::Level(value));
                }
            }
        }
        levels.sort_by_key(|level| match level {
            BuildingLevel::Level(value) => *value,
            BuildingLevel::All => i64::MIN,
        });
        levels.insert(0, BuildingLevel::All);
        levels
    }

    /// Gets the sublayer tree (`viewModel.sublayers`).
    pub fn sublayers(&self) -> &[Rc<RefCell<SublayerNode>>] {
        &self.sublayers
    }

    /// Gets the top-layer selector entries (`viewModel.topLayers`).
    pub fn top_layers(&self) -> &[TopLayer] {
        &self.top_layers
    }

    /// Gets the available levels (`viewModel.levels`).
    pub fn levels(&self) -> &[BuildingLevel] {
        &self.levels
    }

    /// Gets the level selected before switching to the Overview
    /// (`viewModel.selectedLevel`).
    pub fn selected_level(&self) -> BuildingLevel {
        *self.selected_level.borrow()
    }

    /// Gets the current level filter (`viewModel.currentLevel`).
    pub fn current_level(&self) -> BuildingLevel {
        self.current_level.get()
    }

    /// Sets the current level filter, triggering
    /// `filterByAttributes` on the provider.
    pub fn set_current_level(&self, level: BuildingLevel) {
        self.current_level
            .set_with_comparer(level, |left, right| left == right);
    }

    /// Gets the currently selected top layer (`viewModel.currentLayer`).
    pub fn current_layer(&self) -> Option<TopLayerSelection> {
        self.current_layer.get()
    }

    /// Sets the currently selected top layer, triggering the
    /// `handleTopLayerSelector` behavior.
    pub fn set_current_layer(&self, selection: Option<TopLayerSelection>) {
        self.current_layer
            .set_with_comparer(selection, |left, right| left == right);
    }

    /// Gets the default top layer (`viewModel.defaultLayer`).
    pub fn default_layer(&self) -> Option<&TopLayer> {
        self.default_layer.as_ref()
    }
}
