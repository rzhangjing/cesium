//! Batch 2 spec mirrors: service-backed widget ViewModels.
//!
//! Mirrors (one `#[test]` per JS `it`):
//! - `packages/widgets/Specs/BaseLayerPicker/ProviderViewModelSpec.js` (10 its)
//! - `packages/widgets/Specs/BaseLayerPicker/BaseLayerPickerViewModelSpec.js` (14 its)
//! - `packages/widgets/Specs/SceneModePicker/SceneModePickerViewModelSpec.js` (6 its)
//! - `packages/widgets/Specs/Geocoder/GeocoderViewModelSpec.js` (16 its)
//!
//! GPU-dependent behaviour (real Globe imagery layers, Scene morphing,
//! camera tweens, credit display) is exercised through the widget-local
//! injection traits (`PickerGlobe`, `MorphableScene`, `GeocoderScene`)
//! with mock implementations, following the batch 1 pattern.

use std::cell::Cell;
use std::rc::Rc;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::credit::Credit;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::event::Event;
use cesium_core::geocode_type::GeocodeType;
use cesium_core::julian_date::JulianDate;
use cesium_core::rectangle::Rectangle;
use cesium_scene::compute_fly_to_location_for_rectangle::FlyToRectangleScene;
use cesium_scene::scene::Scene;
use cesium_scene::scene_mode::SceneMode;
use cesium_test_utils::expect_to_throw_dev_error;

use cesium_widgets::base_layer_picker_view_model::{
    BaseLayerPickerViewModel, BaseLayerPickerViewModelOptions, ImageryLayerToken, PickerGlobe,
};
use cesium_widgets::observables::ObservableCell;
use cesium_widgets::provider_view_model::{
    ProviderCreationOutput, ProviderHandle, ProviderViewModel, ProviderViewModelOptions,
    SharedPromise, StringProp,
};

// ===========================================================================
// Widgets/BaseLayerPicker/ProviderViewModel
// — packages/widgets/Specs/BaseLayerPicker/ProviderViewModelSpec.js
// ===========================================================================

/// Spy creation function analogue (`jasmine.createSpy("creationFunction")`).
fn spy_creation_function() -> (Rc<dyn Fn() -> ProviderCreationOutput>, Rc<Cell<bool>>) {
    let called = Rc::new(Cell::new(false));
    let called_clone = Rc::clone(&called);
    let func: Rc<dyn Fn() -> ProviderCreationOutput> = Rc::new(move || {
        called_clone.set(true);
        ProviderCreationOutput::Providers(Vec::new())
    });
    (func, called)
}

/// Mirrors `it("constructor sets expected parameters")` (with observables).
#[test]
fn provider_view_model_observable_constructor_sets_expected_parameters() {
    let (creation_function, spy) = spy_creation_function();
    let view_model = ProviderViewModel::new(ProviderViewModelOptions {
        name: Some(StringProp::Observable(ObservableCell::new("name".to_string()))),
        tooltip: Some(StringProp::Observable(ObservableCell::new(
            "tooltip".to_string(),
        ))),
        icon_url: Some(StringProp::Observable(ObservableCell::new(
            "iconUrl".to_string(),
        ))),
        category: Some("mycategory".to_string()),
        creation_function: Some(creation_function),
    });

    assert_eq!(view_model.name(), "name");
    assert_eq!(view_model.tooltip(), "tooltip");
    assert_eq!(view_model.icon_url(), "iconUrl");
    assert_eq!(view_model.category(), "mycategory");

    view_model.creation_command().execute();
    assert!(spy.get());
}

/// Mirrors `it("constructor throws with no name")` (with observables).
#[test]
fn provider_view_model_observable_constructor_throws_with_no_name() {
    let (creation_function, _) = spy_creation_function();
    expect_to_throw_dev_error(|| {
        let _ = ProviderViewModel::new(ProviderViewModelOptions {
            name: None,
            tooltip: Some(StringProp::Observable(ObservableCell::new(
                "tooltip".to_string(),
            ))),
            icon_url: Some(StringProp::Observable(ObservableCell::new(
                "iconUrl".to_string(),
            ))),
            category: None,
            creation_function: Some(creation_function),
        });
    });
}

/// Mirrors `it("constructor throws with no tooltip")` (with observables).
#[test]
fn provider_view_model_observable_constructor_throws_with_no_tooltip() {
    let (creation_function, _) = spy_creation_function();
    expect_to_throw_dev_error(|| {
        let _ = ProviderViewModel::new(ProviderViewModelOptions {
            name: Some(StringProp::Observable(ObservableCell::new("name".to_string()))),
            tooltip: None,
            icon_url: Some(StringProp::Observable(ObservableCell::new(
                "iconUrl".to_string(),
            ))),
            category: None,
            creation_function: Some(creation_function),
        });
    });
}

/// Mirrors `it("constructor throws with no iconUrl")` (with observables).
#[test]
fn provider_view_model_observable_constructor_throws_with_no_icon_url() {
    let (creation_function, _) = spy_creation_function();
    expect_to_throw_dev_error(|| {
        let _ = ProviderViewModel::new(ProviderViewModelOptions {
            name: Some(StringProp::Observable(ObservableCell::new("name".to_string()))),
            tooltip: Some(StringProp::Observable(ObservableCell::new(
                "tooltip".to_string(),
            ))),
            icon_url: None,
            category: None,
            creation_function: Some(creation_function),
        });
    });
}

/// Mirrors `it("constructor throws with no creationFunction")`
/// (with observables).
#[test]
fn provider_view_model_observable_constructor_throws_with_no_creation_function() {
    expect_to_throw_dev_error(|| {
        let _ = ProviderViewModel::new(ProviderViewModelOptions {
            name: Some(StringProp::Observable(ObservableCell::new("name".to_string()))),
            tooltip: Some(StringProp::Observable(ObservableCell::new(
                "tooltip".to_string(),
            ))),
            icon_url: Some(StringProp::Observable(ObservableCell::new(
                "iconUrl".to_string(),
            ))),
            category: None,
            creation_function: None,
        });
    });
}

/// Mirrors `it("constructor sets expected parameters")` (with values).
#[test]
fn provider_view_model_value_constructor_sets_expected_parameters() {
    let (creation_function, spy) = spy_creation_function();
    let view_model = ProviderViewModel::new(ProviderViewModelOptions {
        name: Some(StringProp::Value("name".to_string())),
        tooltip: Some(StringProp::Value("tooltip".to_string())),
        icon_url: Some(StringProp::Value("iconUrl".to_string())),
        category: None,
        creation_function: Some(creation_function),
    });

    assert_eq!(view_model.name(), "name");
    assert_eq!(view_model.tooltip(), "tooltip");
    assert_eq!(view_model.icon_url(), "iconUrl");

    view_model.creation_command().execute();
    assert!(spy.get());
}

/// Mirrors `it("constructor throws with no name")` (with values).
#[test]
fn provider_view_model_value_constructor_throws_with_no_name() {
    let (creation_function, _) = spy_creation_function();
    expect_to_throw_dev_error(|| {
        let _ = ProviderViewModel::new(ProviderViewModelOptions {
            name: None,
            tooltip: Some(StringProp::Value("tooltip".to_string())),
            icon_url: Some(StringProp::Value("iconUrl".to_string())),
            category: None,
            creation_function: Some(creation_function),
        });
    });
}

/// Mirrors `it("constructor throws with no tooltip")` (with values).
#[test]
fn provider_view_model_value_constructor_throws_with_no_tooltip() {
    let (creation_function, _) = spy_creation_function();
    expect_to_throw_dev_error(|| {
        let _ = ProviderViewModel::new(ProviderViewModelOptions {
            name: Some(StringProp::Value("name".to_string())),
            tooltip: None,
            icon_url: Some(StringProp::Value("iconUrl".to_string())),
            category: None,
            creation_function: Some(creation_function),
        });
    });
}

/// Mirrors `it("constructor throws with no iconUrl")` (with values).
#[test]
fn provider_view_model_value_constructor_throws_with_no_icon_url() {
    let (creation_function, _) = spy_creation_function();
    expect_to_throw_dev_error(|| {
        let _ = ProviderViewModel::new(ProviderViewModelOptions {
            name: Some(StringProp::Value("name".to_string())),
            tooltip: Some(StringProp::Value("tooltip".to_string())),
            icon_url: None,
            category: None,
            creation_function: Some(creation_function),
        });
    });
}

/// Mirrors `it("constructor throws with no creationFunction")`
/// (with values).
#[test]
fn provider_view_model_value_constructor_throws_with_no_creation_function() {
    expect_to_throw_dev_error(|| {
        let _ = ProviderViewModel::new(ProviderViewModelOptions {
            name: Some(StringProp::Value("name".to_string())),
            tooltip: Some(StringProp::Value("tooltip".to_string())),
            icon_url: Some(StringProp::Value("iconUrl".to_string())),
            category: None,
            creation_function: None,
        });
    });
}

// ===========================================================================
// Widgets/BaseLayerPicker/BaseLayerPickerViewModel
// — packages/widgets/Specs/BaseLayerPicker/BaseLayerPickerViewModelSpec.js
// ===========================================================================

/// MockGlobe analogue of the JS spec fixture.
struct MockGlobe {
    layers: std::cell::RefCell<Vec<(ImageryLayerToken, ProviderHandle, Option<String>)>>,
    next_token: Cell<usize>,
    terrain_provider: std::cell::RefCell<Option<ProviderHandle>>,
    depth_test_against_terrain: Cell<bool>,
    terrain_provider_changed: Event<()>,
    /// The handle reported as an `EllipsoidTerrainProvider` instance.
    ellipsoid_handle: ProviderHandle,
}

impl MockGlobe {
    fn new(ellipsoid_handle: ProviderHandle) -> Self {
        Self {
            layers: std::cell::RefCell::new(Vec::new()),
            next_token: Cell::new(1),
            terrain_provider: std::cell::RefCell::new(None),
            depth_test_against_terrain: Cell::new(false),
            terrain_provider_changed: Event::new(),
            ellipsoid_handle,
        }
    }

    /// Mirrors `imageryLayers.addImageryProvider(provider, index)` used by
    /// the "only removes layers added by view model" spec.
    fn add_imagery_provider_at(&self, provider: ProviderHandle, index: usize) {
        let token = self.next_token.get();
        self.next_token.set(token + 1);
        self.layers.borrow_mut().insert(index, (token, provider, None));
    }

    /// Mirrors `imageryLayers.remove(imageryLayers.get(index))` used by
    /// the "only removes layers added by view model" spec.
    fn remove_layer_at(&self, index: usize) {
        self.layers.borrow_mut().remove(index);
    }
}

impl PickerGlobe for MockGlobe {
    fn imagery_layers_len(&self) -> usize {
        self.layers.borrow().len()
    }

    fn add_imagery_layer_at(
        &self,
        index: usize,
        provider: ProviderHandle,
        name: Option<String>,
    ) -> ImageryLayerToken {
        let token = self.next_token.get();
        self.next_token.set(token + 1);
        self.layers.borrow_mut().insert(index, (token, provider, name));
        token
    }

    fn remove_imagery_layer(&self, token: ImageryLayerToken) {
        self.layers
            .borrow_mut()
            .retain(|(layer_token, _, _)| *layer_token != token);
    }

    fn has_imagery_layer(&self, token: ImageryLayerToken) -> bool {
        self.layers
            .borrow()
            .iter()
            .any(|(layer_token, _, _)| *layer_token == token)
    }

    fn remove_imagery_layer_at(&self, index: usize) {
        self.layers.borrow_mut().remove(index);
    }

    fn imagery_layer_provider(&self, index: usize) -> Option<ProviderHandle> {
        self.layers
            .borrow()
            .get(index)
            .map(|(_, provider, _)| Rc::clone(provider))
    }

    fn terrain_provider(&self) -> Option<ProviderHandle> {
        self.terrain_provider.borrow().clone()
    }

    fn set_terrain_provider(&self, provider: ProviderHandle) {
        *self.terrain_provider.borrow_mut() = Some(provider);
    }

    fn is_ellipsoid_terrain_provider(&self, provider: &ProviderHandle) -> bool {
        Rc::ptr_eq(provider, &self.ellipsoid_handle)
    }

    fn depth_test_against_terrain(&self) -> bool {
        self.depth_test_against_terrain.get()
    }

    fn set_depth_test_against_terrain(&self, value: bool) {
        self.depth_test_against_terrain.set(value);
    }

    fn terrain_provider_changed(&self) -> &Event<()> {
        &self.terrain_provider_changed
    }
}

/// Opaque test provider handles mirroring the JS spec's `testProvider`,
/// `testProvider2`, `testProvider3` and the `EllipsoidTerrainProvider`.
struct ProviderHandles {
    test_provider: ProviderHandle,
    test_provider2: ProviderHandle,
    test_provider3: ProviderHandle,
    ellipsoid_provider: ProviderHandle,
}

fn provider_handles() -> ProviderHandles {
    ProviderHandles {
        test_provider: Rc::new("testProvider"),
        test_provider2: Rc::new("testProvider2"),
        test_provider3: Rc::new("testProvider3"),
        ellipsoid_provider: Rc::new("ellipsoidTerrainProvider"),
    }
}

fn provider_view_model(
    name: &str,
    tooltip: &str,
    icon_url: &str,
    creation_function: Rc<dyn Fn() -> ProviderCreationOutput>,
) -> Rc<ProviderViewModel> {
    Rc::new(ProviderViewModel::new(ProviderViewModelOptions {
        name: Some(StringProp::Value(name.to_string())),
        tooltip: Some(StringProp::Value(tooltip.to_string())),
        icon_url: Some(StringProp::Value(icon_url.to_string())),
        category: None,
        creation_function: Some(creation_function),
    }))
}

fn categorized_provider_view_model(
    category: &str,
    creation_function: Rc<dyn Fn() -> ProviderCreationOutput>,
) -> Rc<ProviderViewModel> {
    Rc::new(ProviderViewModel::new(ProviderViewModelOptions {
        name: Some(StringProp::Value("name".to_string())),
        tooltip: Some(StringProp::Value("tooltip".to_string())),
        icon_url: Some(StringProp::Value("url".to_string())),
        category: Some(category.to_string()),
        creation_function: Some(creation_function),
    }))
}

fn sync_creation(provider: ProviderHandle) -> Rc<dyn Fn() -> ProviderCreationOutput> {
    Rc::new(move || ProviderCreationOutput::Providers(vec![Rc::clone(&provider)]))
}

fn multi_sync_creation(
    providers: Vec<ProviderHandle>,
) -> Rc<dyn Fn() -> ProviderCreationOutput> {
    Rc::new(move || ProviderCreationOutput::Providers(providers.clone()))
}

fn same_handle(actual: Option<ProviderHandle>, expected: &ProviderHandle) -> bool {
    match actual {
        Some(actual) => Rc::ptr_eq(&actual, expected),
        None => false,
    }
}

/// Mirrors `it("constructor sets expected values")`.
#[test]
fn base_layer_picker_constructor_sets_expected_values() {
    let handles = provider_handles();
    let globe = Rc::new(MockGlobe::new(Rc::clone(&handles.ellipsoid_provider)));
    let globe_dyn = Rc::clone(&globe) as Rc<dyn PickerGlobe>;

    let view_model = BaseLayerPickerViewModel::new(BaseLayerPickerViewModelOptions {
        globe: Some(Rc::clone(&globe_dyn)),
        imagery_provider_view_models: Vec::new(),
        selected_imagery_provider_view_model: None,
        terrain_provider_view_models: Vec::new(),
        selected_terrain_provider_view_model: None,
    });

    assert!(std::ptr::addr_eq(
        Rc::as_ptr(view_model.globe()),
        Rc::as_ptr(&globe_dyn)
    ));
    assert!(view_model.imagery_provider_view_models().is_empty());
    assert!(view_model.terrain_provider_view_models().is_empty());
}

/// Mirrors `it("separates providers into categories")`.
#[test]
fn base_layer_picker_separates_providers_into_categories() {
    let handles = provider_handles();
    let creation = sync_creation(Rc::clone(&handles.test_provider));

    let imagery_providers = vec![
        categorized_provider_view_model("cat1", Rc::clone(&creation)),
        categorized_provider_view_model("cat1", Rc::clone(&creation)),
        categorized_provider_view_model("cat2", Rc::clone(&creation)),
    ];
    let terrain_providers = vec![
        categorized_provider_view_model("cat1", Rc::clone(&creation)),
        categorized_provider_view_model("cat2", Rc::clone(&creation)),
        categorized_provider_view_model("cat2", Rc::clone(&creation)),
    ];

    let globe = Rc::new(MockGlobe::new(Rc::clone(&handles.ellipsoid_provider)));
    let view_model = BaseLayerPickerViewModel::new(BaseLayerPickerViewModelOptions {
        globe: Some(globe as Rc<dyn PickerGlobe>),
        imagery_provider_view_models: imagery_providers,
        selected_imagery_provider_view_model: None,
        terrain_provider_view_models: terrain_providers,
        selected_terrain_provider_view_model: None,
    });

    let imagery = view_model.imagery_providers();
    assert_eq!(imagery.len(), 2);
    assert_eq!(imagery[0].providers.len(), 2);
    assert_eq!(imagery[0].name, "cat1");
    assert_eq!(imagery[1].providers.len(), 1);
    assert_eq!(imagery[1].name, "cat2");

    let terrain = view_model.terrain_providers();
    assert_eq!(terrain.len(), 2);
    assert_eq!(terrain[0].providers.len(), 1);
    assert_eq!(terrain[0].name, "cat1");
    assert_eq!(terrain[1].providers.len(), 2);
    assert_eq!(terrain[1].name, "cat2");
}

/// Shared fixture for the selection specs: a globe, the spec's provider
/// view models and a view model constructed with them.
struct BaseLayerPickerFixture {
    globe: Rc<MockGlobe>,
    handles: ProviderHandles,
    test_provider_view_model: Rc<ProviderViewModel>,
    test_provider_view_model2: Rc<ProviderViewModel>,
    test_provider_view_model3: Rc<ProviderViewModel>,
    ellipsoid_provider_view_model: Rc<ProviderViewModel>,
    async_provider_view_model: Rc<ProviderViewModel>,
    async_promise: SharedPromise,
}

fn base_layer_picker_fixture() -> BaseLayerPickerFixture {
    let handles = provider_handles();
    let globe = Rc::new(MockGlobe::new(Rc::clone(&handles.ellipsoid_provider)));

    let test_provider_view_model = provider_view_model(
        "name",
        "tooltip",
        "url",
        sync_creation(Rc::clone(&handles.test_provider)),
    );
    let test_provider_view_model2 = provider_view_model(
        "name2",
        "tooltip2",
        "url2",
        multi_sync_creation(vec![
            Rc::clone(&handles.test_provider),
            Rc::clone(&handles.test_provider2),
        ]),
    );
    let test_provider_view_model3 = provider_view_model(
        "name3",
        "tooltip3",
        "url3",
        sync_creation(Rc::clone(&handles.test_provider3)),
    );
    let ellipsoid_provider_view_model = provider_view_model(
        "name",
        "tooltip",
        "url",
        sync_creation(Rc::clone(&handles.ellipsoid_provider)),
    );

    // The async creation function returns clones of the same shared
    // promise, mirroring the JS async creationFunction whose promise the
    // spec awaits (resolving the test's clone resolves the view model's
    // clone too).
    let async_promise = SharedPromise::new();
    let async_promise_clone = async_promise.clone();
    let async_provider_view_model = provider_view_model(
        "name3",
        "tooltip3",
        "url3",
        Rc::new(move || ProviderCreationOutput::Promise(async_promise_clone.clone())),
    );

    BaseLayerPickerFixture {
        globe,
        handles,
        test_provider_view_model,
        test_provider_view_model2,
        test_provider_view_model3,
        ellipsoid_provider_view_model,
        async_provider_view_model,
        async_promise,
    }
}

/// Mirrors `it("selecting imagery closes the dropDown")`.
#[test]
fn base_layer_picker_selecting_imagery_closes_the_drop_down() {
    let fixture = base_layer_picker_fixture();
    let view_model = BaseLayerPickerViewModel::new(BaseLayerPickerViewModelOptions {
        globe: Some(Rc::clone(&fixture.globe) as Rc<dyn PickerGlobe>),
        imagery_provider_view_models: vec![Rc::clone(&fixture.test_provider_view_model)],
        selected_imagery_provider_view_model: None,
        terrain_provider_view_models: Vec::new(),
        selected_terrain_provider_view_model: None,
    });

    view_model.set_drop_down_visible(true);
    view_model.set_selected_imagery(Some(Rc::clone(&fixture.test_provider_view_model)));
    assert!(!view_model.drop_down_visible());
}

/// Mirrors `it("selecting terrain closes the dropDown")`.
#[test]
fn base_layer_picker_selecting_terrain_closes_the_drop_down() {
    let fixture = base_layer_picker_fixture();
    let view_model = BaseLayerPickerViewModel::new(BaseLayerPickerViewModelOptions {
        globe: Some(Rc::clone(&fixture.globe) as Rc<dyn PickerGlobe>),
        imagery_provider_view_models: vec![Rc::clone(&fixture.test_provider_view_model)],
        selected_imagery_provider_view_model: None,
        terrain_provider_view_models: Vec::new(),
        selected_terrain_provider_view_model: None,
    });

    view_model.set_drop_down_visible(true);
    view_model.set_selected_terrain(Some(Rc::clone(&fixture.test_provider_view_model)));
    // JS: `await testProviderViewModel.creationCommand()` — the creation is
    // synchronous here, nothing extra to await.
    assert!(!view_model.drop_down_visible());
}

/// Mirrors `it("tooltip, buttonImageUrl, and selectedImagery all return
/// expected values")`.
#[test]
fn base_layer_picker_tooltip_button_image_url_and_selected_imagery() {
    let fixture = base_layer_picker_fixture();
    let view_model = BaseLayerPickerViewModel::new(BaseLayerPickerViewModelOptions {
        globe: Some(Rc::clone(&fixture.globe) as Rc<dyn PickerGlobe>),
        imagery_provider_view_models: vec![Rc::clone(&fixture.test_provider_view_model)],
        selected_imagery_provider_view_model: None,
        terrain_provider_view_models: vec![Rc::clone(&fixture.test_provider_view_model3)],
        selected_terrain_provider_view_model: None,
    });

    view_model.set_selected_imagery(Some(Rc::clone(&fixture.test_provider_view_model)));
    view_model.set_selected_terrain(Some(Rc::clone(&fixture.test_provider_view_model3)));
    assert_eq!(view_model.button_tooltip(), Some("name\nname3".to_string()));

    view_model.set_selected_imagery(None);
    assert_eq!(view_model.button_tooltip(), Some("name3".to_string()));

    view_model.set_selected_imagery(Some(Rc::clone(&fixture.test_provider_view_model)));
    view_model.set_selected_terrain(None);
    assert_eq!(view_model.button_tooltip(), Some("name".to_string()));

    assert_eq!(view_model.button_image_url(), Some("url".to_string()));
}

/// Mirrors `it("selectedImagery actually sets base layer")`.
#[test]
fn base_layer_picker_selected_imagery_actually_sets_base_layer() {
    let fixture = base_layer_picker_fixture();
    let view_model = BaseLayerPickerViewModel::new(BaseLayerPickerViewModelOptions {
        globe: Some(Rc::clone(&fixture.globe) as Rc<dyn PickerGlobe>),
        imagery_provider_view_models: vec![Rc::clone(&fixture.test_provider_view_model)],
        selected_imagery_provider_view_model: None,
        terrain_provider_view_models: Vec::new(),
        selected_terrain_provider_view_model: None,
    });

    assert_eq!(fixture.globe.imagery_layers_len(), 1);

    view_model.set_selected_imagery(Some(Rc::clone(&fixture.test_provider_view_model)));
    assert_eq!(fixture.globe.imagery_layers_len(), 1);
    assert!(same_handle(
        fixture.globe.imagery_layer_provider(0),
        &fixture.handles.test_provider
    ));

    view_model.set_selected_imagery(Some(Rc::clone(&fixture.test_provider_view_model2)));
    assert_eq!(fixture.globe.imagery_layers_len(), 2);
    assert!(same_handle(
        fixture.globe.imagery_layer_provider(0),
        &fixture.handles.test_provider
    ));
    assert!(same_handle(
        fixture.globe.imagery_layer_provider(1),
        &fixture.handles.test_provider2
    ));
}

/// Mirrors `it("selectedTerrain actually sets terrainProvider")`.
#[test]
fn base_layer_picker_selected_terrain_actually_sets_terrain_provider() {
    let fixture = base_layer_picker_fixture();
    let view_model = BaseLayerPickerViewModel::new(BaseLayerPickerViewModelOptions {
        globe: Some(Rc::clone(&fixture.globe) as Rc<dyn PickerGlobe>),
        imagery_provider_view_models: Vec::new(),
        selected_imagery_provider_view_model: None,
        terrain_provider_view_models: vec![
            Rc::clone(&fixture.test_provider_view_model),
            Rc::clone(&fixture.test_provider_view_model3),
        ],
        selected_terrain_provider_view_model: None,
    });

    view_model.set_selected_terrain(Some(Rc::clone(&fixture.test_provider_view_model3)));
    assert!(same_handle(
        fixture.globe.terrain_provider(),
        &fixture.handles.test_provider3
    ));
}

/// Mirrors `it("selectedTerrain actually sets async terrainProvider")`.
#[test]
fn base_layer_picker_selected_terrain_actually_sets_async_terrain_provider() {
    let fixture = base_layer_picker_fixture();
    let view_model = BaseLayerPickerViewModel::new(BaseLayerPickerViewModelOptions {
        globe: Some(Rc::clone(&fixture.globe) as Rc<dyn PickerGlobe>),
        imagery_provider_view_models: Vec::new(),
        selected_imagery_provider_view_model: None,
        terrain_provider_view_models: vec![
            Rc::clone(&fixture.test_provider_view_model),
            Rc::clone(&fixture.async_provider_view_model),
        ],
        selected_terrain_provider_view_model: None,
    });

    view_model.set_selected_terrain(Some(Rc::clone(&fixture.async_provider_view_model)));
    // JS: `await testProviderViewModelAsync.creationCommand()`.
    fixture
        .async_promise
        .resolve(vec![Rc::clone(&fixture.handles.test_provider)]);
    assert!(same_handle(
        fixture.globe.terrain_provider(),
        &fixture.handles.test_provider
    ));
    assert!(fixture.globe.depth_test_against_terrain());
}

/// Mirrors `it("selectedTerrain sets ellipsoid terrain provider")`.
#[test]
fn base_layer_picker_selected_terrain_sets_ellipsoid_terrain_provider() {
    let fixture = base_layer_picker_fixture();
    let view_model = BaseLayerPickerViewModel::new(BaseLayerPickerViewModelOptions {
        globe: Some(Rc::clone(&fixture.globe) as Rc<dyn PickerGlobe>),
        imagery_provider_view_models: Vec::new(),
        selected_imagery_provider_view_model: None,
        terrain_provider_view_models: vec![Rc::clone(&fixture.ellipsoid_provider_view_model)],
        selected_terrain_provider_view_model: None,
    });

    view_model.set_selected_terrain(Some(Rc::clone(&fixture.ellipsoid_provider_view_model)));
    assert!(same_handle(
        fixture.globe.terrain_provider(),
        &fixture.handles.ellipsoid_provider
    ));
    assert!(!fixture.globe.depth_test_against_terrain());
}

/// Mirrors `it("default does not override default value of
/// depthTestAgainstTerrain")`.
#[test]
fn base_layer_picker_default_does_not_override_depth_test_against_terrain() {
    let fixture = base_layer_picker_fixture();
    let _view_model = BaseLayerPickerViewModel::new(BaseLayerPickerViewModelOptions {
        globe: Some(Rc::clone(&fixture.globe) as Rc<dyn PickerGlobe>),
        imagery_provider_view_models: Vec::new(),
        selected_imagery_provider_view_model: None,
        terrain_provider_view_models: vec![Rc::clone(&fixture.ellipsoid_provider_view_model)],
        selected_terrain_provider_view_model: None,
    });

    fixture.globe.set_depth_test_against_terrain(true);

    // JS: `await testEllipsoidProviderViewModel.creationCommand()` — the
    // creation runs but nothing consumes it (no terrain selection).
    fixture
        .ellipsoid_provider_view_model
        .creation_command()
        .execute();
    assert!(same_handle(
        fixture.globe.terrain_provider(),
        &fixture.handles.ellipsoid_provider
    ) || fixture.globe.terrain_provider().is_none());
    assert!(fixture.globe.depth_test_against_terrain());
}

/// Mirrors `it("selectedTerrain cancels update if terrainProvider is set
/// externally")`.
#[test]
fn base_layer_picker_selected_terrain_cancels_update_on_external_change() {
    let fixture = base_layer_picker_fixture();
    let view_model = BaseLayerPickerViewModel::new(BaseLayerPickerViewModelOptions {
        globe: Some(Rc::clone(&fixture.globe) as Rc<dyn PickerGlobe>),
        imagery_provider_view_models: Vec::new(),
        selected_imagery_provider_view_model: None,
        terrain_provider_view_models: vec![Rc::clone(&fixture.test_provider_view_model3)],
        selected_terrain_provider_view_model: None,
    });

    view_model.set_selected_terrain(Some(Rc::clone(&fixture.async_provider_view_model)));
    fixture.globe.terrain_provider_changed.raise_event(&());
    // JS: `await testProviderViewModelAsync.creationCommand()`.
    fixture
        .async_promise
        .resolve(vec![Rc::clone(&fixture.handles.test_provider)]);
    assert!(!same_handle(
        fixture.globe.terrain_provider(),
        &fixture.handles.test_provider
    ));
}

/// Mirrors `it("settings selectedImagery only removes layers added by view
/// model")`.
#[test]
fn base_layer_picker_selected_imagery_only_removes_layers_added_by_view_model() {
    let fixture = base_layer_picker_fixture();
    let view_model = BaseLayerPickerViewModel::new(BaseLayerPickerViewModelOptions {
        globe: Some(Rc::clone(&fixture.globe) as Rc<dyn PickerGlobe>),
        imagery_provider_view_models: vec![Rc::clone(&fixture.test_provider_view_model)],
        selected_imagery_provider_view_model: None,
        terrain_provider_view_models: Vec::new(),
        selected_terrain_provider_view_model: None,
    });

    assert_eq!(fixture.globe.imagery_layers_len(), 1);

    view_model.set_selected_imagery(Some(Rc::clone(&fixture.test_provider_view_model2)));
    assert_eq!(fixture.globe.imagery_layers_len(), 2);
    assert!(same_handle(
        fixture.globe.imagery_layer_provider(0),
        &fixture.handles.test_provider
    ));
    assert!(same_handle(
        fixture.globe.imagery_layer_provider(1),
        &fixture.handles.test_provider2
    ));

    // imageryLayers.addImageryProvider(testProvider3, 1);
    fixture.globe.add_imagery_provider_at(
        Rc::clone(&fixture.handles.test_provider3),
        1,
    );
    // imageryLayers.remove(imageryLayers.get(0));
    fixture.globe.remove_layer_at(0);

    view_model.set_selected_imagery(None);

    assert_eq!(fixture.globe.imagery_layers_len(), 1);
    assert!(same_handle(
        fixture.globe.imagery_layer_provider(0),
        &fixture.handles.test_provider3
    ));
}

/// Mirrors `it("dropDownVisible and toggleDropDown work")`.
#[test]
fn base_layer_picker_drop_down_visible_and_toggle_drop_down_work() {
    let handles = provider_handles();
    let globe = Rc::new(MockGlobe::new(Rc::clone(&handles.ellipsoid_provider)));
    let view_model = BaseLayerPickerViewModel::new(BaseLayerPickerViewModelOptions {
        globe: Some(globe as Rc<dyn PickerGlobe>),
        imagery_provider_view_models: Vec::new(),
        selected_imagery_provider_view_model: None,
        terrain_provider_view_models: Vec::new(),
        selected_terrain_provider_view_model: None,
    });

    assert!(!view_model.drop_down_visible());
    view_model.toggle_drop_down().execute();
    assert!(view_model.drop_down_visible());
    view_model.set_drop_down_visible(false);
    assert!(!view_model.drop_down_visible());
}

/// Mirrors `it("constructor throws with no globe")`.
#[test]
fn base_layer_picker_constructor_throws_with_no_globe() {
    expect_to_throw_dev_error(|| {
        let _ = BaseLayerPickerViewModel::new(BaseLayerPickerViewModelOptions {
            globe: None,
            imagery_provider_view_models: Vec::new(),
            selected_imagery_provider_view_model: None,
            terrain_provider_view_models: Vec::new(),
            selected_terrain_provider_view_model: None,
        });
    });
}

// ===========================================================================
// Widgets/SceneModePicker/SceneModePickerViewModel
// — packages/widgets/Specs/SceneModePicker/SceneModePickerViewModelSpec.js
// ===========================================================================

use cesium_widgets::scene_mode_picker_view_model::{
    MorphableScene, SceneModePickerViewModel,
};

/// Mock scene analogue of the JS spec's `createScene()` fixture. Morph
/// calls record the request, raise `morphStart` with the new mode
/// (mirroring the real scene raising `morphStart` when the morph begins)
/// and hold the mode until `complete_morph` is invoked.
struct MockMorphScene {
    mode: Cell<SceneMode>,
    pending: Cell<Option<SceneMode>>,
    morph_start: Event<SceneMode>,
    morph_calls: std::cell::RefCell<Vec<(SceneMode, f64)>>,
}

impl MockMorphScene {
    fn new() -> Self {
        Self {
            mode: Cell::new(SceneMode::Scene3D),
            pending: Cell::new(None),
            morph_start: Event::new(),
            morph_calls: std::cell::RefCell::new(Vec::new()),
        }
    }

    fn start_morph(&self, target: SceneMode) {
        self.pending.set(Some(target));
        self.morph_start.raise_event(&target);
    }

    /// Mirrors `scene.completeMorph()`.
    fn complete_morph(&self) {
        if let Some(target) = self.pending.take() {
            self.mode.set(target);
        }
    }
}

impl MorphableScene for MockMorphScene {
    fn mode(&self) -> SceneMode {
        self.mode.get()
    }

    fn morph_start(&self) -> &Event<SceneMode> {
        &self.morph_start
    }

    fn morph_to_2d(&self, duration: f64) {
        self.morph_calls
            .borrow_mut()
            .push((SceneMode::Scene2D, duration));
        self.start_morph(SceneMode::Scene2D);
    }

    fn morph_to_3d(&self, duration: f64) {
        self.morph_calls
            .borrow_mut()
            .push((SceneMode::Scene3D, duration));
        self.start_morph(SceneMode::Scene3D);
    }

    fn morph_to_columbus_view(&self, duration: f64) {
        self.morph_calls
            .borrow_mut()
            .push((SceneMode::ColumbusView, duration));
        self.start_morph(SceneMode::ColumbusView);
    }
}

/// Mirrors `it("Can construct and destroy")`.
#[test]
fn scene_mode_picker_can_construct_and_destroy() {
    let scene = Rc::new(MockMorphScene::new());
    let scene_dyn = Rc::clone(&scene) as Rc<dyn MorphableScene>;

    let mut view_model = SceneModePickerViewModel::new(Rc::clone(&scene_dyn), Some(1.0));
    assert!(std::ptr::addr_eq(
        Rc::as_ptr(view_model.scene()),
        Rc::as_ptr(&scene_dyn)
    ));
    assert_eq!(view_model.duration(), 1.0);
    assert_eq!(scene.morph_start.number_of_listeners(), 1);
    assert!(!view_model.is_destroyed());
    view_model.destroy();
    assert!(view_model.is_destroyed());
    assert_eq!(scene.morph_start.number_of_listeners(), 0);
}

/// Mirrors `it("dropDownVisible and toggleDropDown work")`.
#[test]
fn scene_mode_picker_drop_down_visible_and_toggle_drop_down_work() {
    let scene = Rc::new(MockMorphScene::new());
    let view_model = SceneModePickerViewModel::new(scene as Rc<dyn MorphableScene>, None);

    assert!(!view_model.drop_down_visible());
    view_model.toggle_drop_down().execute();
    assert!(view_model.drop_down_visible());
    view_model.set_drop_down_visible(false);
    assert!(!view_model.drop_down_visible());
}

/// Mirrors `it("morphing closes the dropDown")`.
#[test]
fn scene_mode_picker_morphing_closes_the_drop_down() {
    let scene = Rc::new(MockMorphScene::new());
    let view_model = SceneModePickerViewModel::new(scene as Rc<dyn MorphableScene>, None);

    view_model.set_drop_down_visible(true);
    view_model.morph_to_columbus_view().execute();
    assert!(!view_model.drop_down_visible());

    view_model.set_drop_down_visible(true);
    view_model.morph_to_3d().execute();
    assert!(!view_model.drop_down_visible());

    view_model.set_drop_down_visible(true);
    view_model.morph_to_2d().execute();
    assert!(!view_model.drop_down_visible());
}

/// Mirrors `it("morphing calls correct transition")`.
#[test]
fn scene_mode_picker_morphing_calls_correct_transition() {
    let scene = Rc::new(MockMorphScene::new());
    let scene_dyn = Rc::clone(&scene) as Rc<dyn MorphableScene>;
    let view_model = SceneModePickerViewModel::new(Rc::clone(&scene_dyn), None);

    assert_eq!(scene.mode(), SceneMode::Scene3D);

    view_model.morph_to_columbus_view().execute();
    scene.complete_morph();
    assert_eq!(scene.mode(), SceneMode::ColumbusView);

    view_model.morph_to_3d().execute();
    scene.complete_morph();
    assert_eq!(scene.mode(), SceneMode::Scene3D);

    view_model.morph_to_2d().execute();
    scene.complete_morph();
    assert_eq!(scene.mode(), SceneMode::Scene2D);
}

/// Mirrors `it("selectedTooltip changes on transition")`.
#[test]
fn scene_mode_picker_selected_tooltip_changes_on_transition() {
    let scene = Rc::new(MockMorphScene::new());
    let view_model = SceneModePickerViewModel::new(scene as Rc<dyn MorphableScene>, None);

    view_model.morph_to_columbus_view().execute();
    assert_eq!(view_model.selected_tooltip(), view_model.tooltip_columbus_view());

    view_model.morph_to_3d().execute();
    assert_eq!(view_model.selected_tooltip(), view_model.tooltip_3d());

    view_model.morph_to_2d().execute();
    assert_eq!(view_model.selected_tooltip(), view_model.tooltip_2d());
}

/// Mirrors `it("create throws with undefined scene")`.
#[ignore = "DEVIATION: the Rust port takes the scene as a required Rc<dyn MorphableScene> parameter, so the JS 'scene is required.' DeveloperError is guaranteed by the type system"]
#[test]
fn scene_mode_picker_create_throws_with_undefined_scene() {
    // Type-system guaranteed; see the #[ignore] note.
}

/// Wiring test (no JS mirror; task #18): the engine `Scene` plugs into
/// the picker through `impl MorphableScene for Scene` — the morph
/// commands drive `Scene::morph_to_*` and the scene's synchronous morph
/// (DEVIATION) raises `morphStart`, updating `sceneMode` and closing the
/// drop-down.
#[test]
fn scene_mode_picker_drives_the_engine_scene() {
    let scene = Rc::new(Scene::new());
    let scene_dyn = Rc::clone(&scene) as Rc<dyn MorphableScene>;
    let view_model = SceneModePickerViewModel::new(Rc::clone(&scene_dyn), Some(0.0));

    assert_eq!(scene.mode(), SceneMode::Scene3D);
    assert_eq!(scene.morph_start().number_of_listeners(), 1);

    view_model.morph_to_2d().execute();
    assert_eq!(scene.mode(), SceneMode::Scene2D);
    assert_eq!(view_model.scene_mode(), SceneMode::Scene2D);
    assert_eq!(view_model.selected_tooltip(), view_model.tooltip_2d());

    view_model.set_drop_down_visible(true);
    view_model.morph_to_columbus_view().execute();
    assert_eq!(scene.mode(), SceneMode::ColumbusView);
    // morphStart fired: the drop-down closed.
    assert!(!view_model.drop_down_visible());

    view_model.morph_to_3d().execute();
    assert_eq!(scene.mode(), SceneMode::Scene3D);
    assert_eq!(view_model.selected_tooltip(), view_model.tooltip_3d());
}

// ===========================================================================
// Widgets/Geocoder/GeocoderViewModel
// — packages/widgets/Specs/Geocoder/GeocoderViewModelSpec.js
// ===========================================================================

use cesium_widgets::geocoder_view_model::{
    DestinationFound, GeocodeDestination, GeocoderResult, GeocoderScene, GeocoderServiceLike,
    GeocoderViewModel, GeocoderViewModelOptions, SceneGeocoderAdapter,
};

/// Mock scene analogue of the JS spec's `createScene()` fixture: records
/// static credits and camera flights (completing flights immediately).
struct MockGeocoderScene {
    static_credits: std::cell::RefCell<Vec<Credit>>,
    fly_to_destinations: std::cell::RefCell<Vec<Cartesian3>>,
}

impl MockGeocoderScene {
    fn new() -> Self {
        Self {
            static_credits: std::cell::RefCell::new(Vec::new()),
            fly_to_destinations: std::cell::RefCell::new(Vec::new()),
        }
    }
}

impl GeocoderScene for MockGeocoderScene {
    fn is_destroyed(&self) -> bool {
        false
    }

    fn credit_display_is_destroyed(&self) -> bool {
        false
    }

    fn add_static_credit(&self, credit: Credit) {
        self.static_credits.borrow_mut().push(credit);
    }

    fn remove_static_credit(&self, credit: &Credit) {
        self.static_credits
            .borrow_mut()
            .retain(|existing| existing.html() != credit.html());
    }

    fn fly_to(
        &self,
        destination: Cartesian3,
        _duration: Option<f64>,
        complete: Box<dyn FnOnce()>,
    ) {
        self.fly_to_destinations.borrow_mut().push(destination);
        complete();
    }
}

/// The [`FlyToRectangleScene`] seam backing the engine
/// `compute_fly_to_location_for_rectangle` call: a 3D scene with a
/// geographic map projection and no terrain provider (mirrors the JS
/// spec's `createScene()` fixture, whose terrain provider has no
/// availability — so flights gain `DEFAULT_HEIGHT` like the JS).
impl FlyToRectangleScene for MockGeocoderScene {
    fn mode(&self) -> SceneMode {
        SceneMode::Scene3D
    }

    fn ellipsoid(&self) -> Ellipsoid {
        Ellipsoid::WGS84
    }

    fn unproject(&self, cartesian: &Cartesian3) -> Cartographic {
        let mut cartographic = Cartographic::default();
        Ellipsoid::WGS84.cartesian_to_cartographic(cartesian, &mut cartographic);
        cartographic
    }

    fn get_rectangle_camera_coordinates(&self, rectangle: &Rectangle) -> Cartesian3 {
        let mut cartesian = Cartesian3::default();
        Ellipsoid::WGS84
            .cartographic_to_cartesian(&Rectangle::center(rectangle), &mut cartesian);
        cartesian
    }

    fn terrain_provider_defined(&self) -> bool {
        false
    }

    fn terrain_availability_defined(&self) -> bool {
        false
    }

    fn sample_terrain_most_detailed(&self, positions: &[Cartographic]) -> Vec<Option<f64>> {
        positions.iter().map(|_| None).collect()
    }
}

/// Mock geocoder service analogue of the JS spec's `customGeocoderOptions`
/// objects.
struct MockGeocoderService {
    results: Vec<GeocoderResult>,
    credit: Option<Credit>,
}

impl GeocoderServiceLike for MockGeocoderService {
    fn geocode(&self, _query: &str, _geocode_type: GeocodeType) -> Vec<GeocoderResult> {
        self.results.clone()
    }

    fn credit(&self) -> Option<Credit> {
        self.credit.clone()
    }
}

fn mock_destination() -> Cartesian3 {
    Cartesian3::new(1.0, 2.0, 3.0)
}

fn geocoder_results_1() -> Vec<GeocoderResult> {
    ["a", "b", "c"]
        .iter()
        .map(|name| GeocoderResult {
            display_name: name.to_string(),
            destination: GeocodeDestination::Cartesian(mock_destination()),
            attributions: Vec::new(),
        })
        .collect()
}

fn geocoder_results_2() -> Vec<GeocoderResult> {
    ["1", "2"]
        .iter()
        .map(|name| GeocoderResult {
            display_name: name.to_string(),
            destination: GeocodeDestination::Cartesian(mock_destination()),
            attributions: Vec::new(),
        })
        .collect()
}

fn custom_geocoder() -> Rc<dyn GeocoderServiceLike> {
    Rc::new(MockGeocoderService {
        results: geocoder_results_1(),
        credit: None,
    })
}

fn custom_geocoder_2() -> Rc<dyn GeocoderServiceLike> {
    Rc::new(MockGeocoderService {
        results: geocoder_results_2(),
        credit: None,
    })
}

fn no_results_geocoder() -> Rc<dyn GeocoderServiceLike> {
    Rc::new(MockGeocoderService {
        results: Vec::new(),
        credit: None,
    })
}

/// `destinationFound` spy analogue recording the destinations it received.
type DestinationSpy = Rc<std::cell::RefCell<Vec<GeocodeDestination>>>;

fn destination_spy() -> (Box<DestinationFound>, DestinationSpy) {
    let calls: DestinationSpy = Rc::new(std::cell::RefCell::new(Vec::new()));
    let calls_clone = Rc::clone(&calls);
    let callback: Box<DestinationFound> = Box::new(move |_view_model, destination| {
        calls_clone.borrow_mut().push(destination);
    });
    (callback, calls)
}

fn geocoder_view_model(
    services: Vec<Rc<dyn GeocoderServiceLike>>,
    flight_duration: Option<f64>,
    destination_found: Option<Box<DestinationFound>>,
) -> (GeocoderViewModel, Rc<MockGeocoderScene>) {
    let scene = Rc::new(MockGeocoderScene::new());
    let scene_dyn = Rc::clone(&scene) as Rc<dyn GeocoderScene>;
    let view_model = GeocoderViewModel::new(GeocoderViewModelOptions {
        scene: scene_dyn,
        geocoder_services: Some(services),
        flight_duration,
        destination_found,
        autocomplete: None,
    });
    (view_model, scene)
}

/// Mirrors `it("constructor sets expected properties")`.
#[test]
fn geocoder_constructor_sets_expected_properties() {
    let flight_duration = 1234.0;
    let (view_model, scene) =
        geocoder_view_model(vec![custom_geocoder()], Some(flight_duration), None);

    assert!(std::ptr::addr_eq(
        Rc::as_ptr(view_model.scene()),
        Rc::as_ptr(&scene) as *const dyn GeocoderScene
    ));
    assert_eq!(view_model.flight_duration(), Some(flight_duration));
    assert!(!view_model.keep_expanded.get());
}

/// Mirrors `it("can get and set flight duration")`.
#[test]
fn geocoder_can_get_and_set_flight_duration() {
    let (view_model, _scene) = geocoder_view_model(vec![custom_geocoder()], None, None);
    view_model.set_flight_duration(Some(324.0));
    assert_eq!(view_model.flight_duration(), Some(324.0));

    expect_to_throw_dev_error(|| {
        view_model.set_flight_duration(Some(-123.0));
    });
}

/// Mirrors `it("throws if searchText is not a string")`.
#[ignore = "DEVIATION: the Rust searchText setter takes &str, so the JS 'value must be a valid string.' DeveloperError is guaranteed by the type system"]
#[test]
fn geocoder_throws_if_search_text_is_not_a_string() {
    // Type-system guaranteed; see the #[ignore] note.
}

/// Mirrors `it("moves camera when search command invoked")`.
/// DEVIATION: the JS spec polls `scene.tweens` until the camera position
/// changes; the mock scene records the flyTo destination and completes the
/// flight immediately.
#[test]
fn geocoder_moves_camera_when_search_command_invoked() {
    let found = Rc::new(Cell::new(false));
    let found_clone = Rc::clone(&found);
    let destination_found: Box<DestinationFound> =
        Box::new(move |view_model: &GeocoderViewModel, destination| {
            // await GeocoderViewModel.flyToDestination(viewModel, destination);
            view_model.fly_to_destination(destination);
            found_clone.set(true);
        });

    let (view_model, scene) = geocoder_view_model(
        vec![custom_geocoder()],
        None,
        Some(destination_found),
    );

    view_model.set_search_text("220 Valley Creek Blvd, Exton, PA");
    view_model.search();
    assert!(!scene.fly_to_destinations.borrow().is_empty());
    assert!(found.get());
}

/// Mirrors `it("constructor throws without scene")`.
#[ignore = "DEVIATION: the Rust port takes the scene as a required field of GeocoderViewModelOptions, so the JS 'options.scene is required.' DeveloperError is guaranteed by the type system"]
#[test]
fn geocoder_constructor_throws_without_scene() {
    // Type-system guaranteed; see the #[ignore] note.
}

/// Mirrors `it("raises the complete event camera finished")`.
#[test]
fn geocoder_raises_the_complete_event_when_camera_finished() {
    let destination_found: Box<DestinationFound> =
        Box::new(|view_model: &GeocoderViewModel, destination| {
            view_model.fly_to_destination(destination);
        });
    let (view_model, _scene) = geocoder_view_model(
        vec![custom_geocoder()],
        Some(0.0),
        Some(destination_found),
    );

    let listener_calls = Rc::new(Cell::new(0));
    let listener_clone = Rc::clone(&listener_calls);
    view_model
        .complete()
        .add_listener(move |_: &()| listener_clone.set(listener_clone.get() + 1));

    view_model.set_search_text("-1.0, -2.0");
    view_model.search();
    assert_eq!(listener_calls.get(), 1);
}

/// Mirrors `it("can be created with a custom geocoder")`.
#[test]
fn geocoder_can_be_created_with_a_custom_geocoder() {
    let _view_model = geocoder_view_model(vec![custom_geocoder()], None, None);
}

/// Mirrors `it("automatic suggestions can be retrieved")`.
#[test]
fn geocoder_automatic_suggestions_can_be_retrieved() {
    let (callback, spy) = destination_spy();
    let (view_model, _scene) =
        geocoder_view_model(vec![custom_geocoder()], None, Some(callback));

    view_model.set_raw_search_text("some_text");
    view_model.update_search_suggestions();
    assert_eq!(view_model.suggestions().len(), 3);
    assert!(spy.borrow().is_empty());
}

/// Mirrors `it("update search suggestions results in empty list if the query
/// is empty")`.
#[test]
fn geocoder_update_search_suggestions_empty_list_if_query_empty() {
    let (callback, spy) = destination_spy();
    let (view_model, _scene) =
        geocoder_view_model(vec![custom_geocoder()], None, Some(callback));

    view_model.set_raw_search_text("");
    view_model.update_search_suggestions();
    assert_eq!(view_model.suggestions().len(), 0);
    assert!(spy.borrow().is_empty());
}

/// Mirrors `it("can activate selected search suggestion")`.
#[test]
fn geocoder_can_activate_selected_search_suggestion() {
    let (callback, spy) = destination_spy();
    let (view_model, _scene) =
        geocoder_view_model(vec![custom_geocoder()], None, Some(callback));

    let destination = GeocodeDestination::Rectangle(Rectangle::new(0.0, -0.1, 0.1, 0.1));
    let suggestion = GeocoderResult {
        display_name: "a".to_string(),
        destination: destination.clone(),
        attributions: Vec::new(),
    };
    view_model.activate_suggestion(suggestion);
    assert_eq!(view_model.raw_search_text(), "a");
    assert_eq!(spy.borrow().len(), 1);
    assert_eq!(spy.borrow()[0], destination);
}

/// Mirrors `it("if more than one geocoder service is provided, use first
/// result from first geocode in array order")`.
#[test]
fn geocoder_uses_first_result_from_first_geocoder_in_array_order() {
    let (callback, spy) = destination_spy();
    let (view_model, _scene) = geocoder_view_model(
        vec![no_results_geocoder(), custom_geocoder_2()],
        None,
        Some(callback),
    );

    view_model.set_raw_search_text("sthsnth"); // an empty query would prevent geocoding
    view_model.search();
    assert_eq!(view_model.raw_search_text(), "1");
    assert_eq!(spy.borrow().len(), 1);
    assert_eq!(
        spy.borrow()[0],
        GeocodeDestination::Cartesian(mock_destination())
    );
}

/// Mirrors `it("can update autoComplete suggestions list using multiple
/// geocoders")`.
#[test]
fn geocoder_can_update_suggestions_using_multiple_geocoders() {
    let (callback, spy) = destination_spy();
    let (view_model, _scene) = geocoder_view_model(
        vec![custom_geocoder(), custom_geocoder_2()],
        None,
        Some(callback),
    );

    view_model.set_raw_search_text("sthsnth"); // an empty query would prevent geocoding
    view_model.update_search_suggestions();
    assert_eq!(
        view_model.suggestions().len(),
        geocoder_results_1().len() + geocoder_results_2().len()
    );
    assert!(spy.borrow().is_empty());
}

/// Mirrors `it("uses custom destination found callback")`.
#[test]
fn geocoder_uses_custom_destination_found_callback() {
    let (callback, spy) = destination_spy();
    let (view_model, scene) = geocoder_view_model(
        vec![no_results_geocoder(), custom_geocoder_2()],
        None,
        Some(callback),
    );

    view_model.set_raw_search_text("sthsnth"); // an empty query would prevent geocoding
    view_model.search();
    assert_eq!(view_model.raw_search_text(), "1");
    // GeocoderViewModel.flyToDestination was not called.
    assert!(scene.fly_to_destinations.borrow().is_empty());
    assert_eq!(spy.borrow().len(), 1);
    assert_eq!(
        spy.borrow()[0],
        GeocodeDestination::Cartesian(mock_destination())
    );
}

/// Mirrors `it("automatic suggestions can be navigated by arrow up/down
/// keys")`.
#[test]
fn geocoder_suggestions_can_be_navigated_by_arrow_up_down_keys() {
    let (callback, spy) = destination_spy();
    let (view_model, _scene) =
        geocoder_view_model(vec![custom_geocoder()], None, Some(callback));

    view_model.set_raw_search_text("some_text");
    view_model.update_search_suggestions();

    assert_eq!(view_model.selected_suggestion(), None);
    view_model.handle_arrow_down();
    assert_eq!(
        view_model.selected_suggestion().unwrap().display_name,
        "a"
    );
    view_model.handle_arrow_down();
    view_model.handle_arrow_down();
    assert_eq!(
        view_model.selected_suggestion().unwrap().display_name,
        "c"
    );
    view_model.handle_arrow_down();
    assert_eq!(
        view_model.selected_suggestion().unwrap().display_name,
        "a"
    );
    view_model.handle_arrow_down();
    view_model.handle_arrow_up();
    assert_eq!(
        view_model.selected_suggestion().unwrap().display_name,
        "a"
    );
    view_model.handle_arrow_up();
    assert_eq!(view_model.selected_suggestion(), None);
    assert!(spy.borrow().is_empty());
}

/// Mirrors `it("updates credits based on returned results")`.
/// DEVIATION: the JS spec counts the default ion credit seeded into the
/// real scene's credit display; the mock scene seeds none, so the count
/// assertion expects 1 instead of the JS 2.
#[test]
fn geocoder_updates_credits_based_on_returned_results() {
    let mut results = geocoder_results_1();
    results[0].attributions = vec!["attribution".to_string()];
    let service: Rc<dyn GeocoderServiceLike> = Rc::new(MockGeocoderService {
        results,
        credit: Some(Credit::new("custom credit", false)),
    });

    let (callback, spy) = destination_spy();
    let (view_model, scene) =
        geocoder_view_model(vec![service], None, Some(callback));

    view_model.set_raw_search_text("sthsnth"); // an empty query would prevent geocoding
    view_model.search();

    let credits = scene.static_credits.borrow();
    assert_eq!(credits.len(), 1);
    assert_eq!(credits[0].html(), "attribution");
    assert!(!credits[0].show_on_screen());
    assert!(!spy.borrow().is_empty());
}

/// Mirrors `it("uses default geocoder service credit if not present in
/// results")`.
/// DEVIATION: as above, the mock scene seeds no default ion credit, so
/// the count assertion expects 1 instead of the JS 2.
#[test]
fn geocoder_uses_default_geocoder_service_credit_if_not_present() {
    let service: Rc<dyn GeocoderServiceLike> = Rc::new(MockGeocoderService {
        results: geocoder_results_1(),
        credit: Some(Credit::new("custom credit", false)),
    });

    let (callback, spy) = destination_spy();
    let (view_model, scene) =
        geocoder_view_model(vec![service], None, Some(callback));

    view_model.set_raw_search_text("sthsnth"); // an empty query would prevent geocoding
    view_model.search();

    let credits = scene.static_credits.borrow();
    assert_eq!(credits.len(), 1);
    assert_eq!(credits[0].html(), "custom credit");
    assert!(!credits[0].show_on_screen());
    assert!(!spy.borrow().is_empty());
}

/// Wiring test (no JS mirror; task #18): [`SceneGeocoderAdapter`]
/// mirrors the JS `new GeocoderViewModel({ scene })` wiring on the
/// engine `Scene` — the geocoder credit lands in the scene
/// `CreditDisplay` (count assertion) and the camera flight goes through
/// `Scene::fly_to` (a flight tween whose completion raises the view
/// model `complete` event).
#[test]
fn geocoder_scene_adapter_routes_credits_and_flight_through_the_engine_scene() {
    let scene = Rc::new(Scene::new());
    let surface = Rc::new(MockGeocoderScene::new()) as Rc<dyn FlyToRectangleScene>;
    let adapter = Rc::new(SceneGeocoderAdapter::new(Rc::clone(&scene), surface));

    let service: Rc<dyn GeocoderServiceLike> = Rc::new(MockGeocoderService {
        results: geocoder_results_1(),
        credit: Some(Credit::new("custom credit", false)),
    });

    let view_model = GeocoderViewModel::new(GeocoderViewModelOptions {
        scene: adapter as Rc<dyn GeocoderScene>,
        geocoder_services: Some(vec![service]),
        flight_duration: Some(0.0),
        destination_found: None,
        autocomplete: None,
    });

    // `complete` fires when the `Scene::fly_to` tween finishes.
    let completed = Rc::new(Cell::new(false));
    let completed_clone = Rc::clone(&completed);
    view_model
        .complete()
        .add_listener(move |_: &()| completed_clone.set(true));

    view_model.set_raw_search_text("sthsnth"); // an empty query would prevent geocoding
    view_model.search();

    // Credit count in the engine CreditDisplay: exactly the service
    // credit (the headless scene seeds no default ion credit).
    let credits = scene.credit_display();
    assert_eq!(credits.static_credits().len(), 1);
    assert_eq!(credits.static_credits()[0].html(), "custom credit");
    drop(credits);

    // The flight went through `Scene::fly_to`: one tween is pending and
    // completing it (zero duration) raises `complete`.
    assert_eq!(scene.tweens().len(), 1);
    assert!(!completed.get());
    let time = JulianDate::now();
    scene.tweens_mut().update(&time);
    assert!(completed.get());
    assert!(scene.tweens().is_empty());
}

