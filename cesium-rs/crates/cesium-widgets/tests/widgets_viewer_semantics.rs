//! Track A10 batch 3 spec mirrors (cesium-widgets):
//!
//! - `packages/widgets/Specs/PerformanceWatchdog/PerformanceWatchdogViewModelSpec.js`
//! - `packages/widgets/Specs/ProjectionPicker/ProjectionPickerViewModelSpec.js`
//! - `packages/widgets/Specs/I3SBSLExplorer/I3SBSLExplorerViewModelSpec.js`
//! - `packages/widgets/Specs/Viewer/ViewerSpec.js` (engine-side semantics
//!   subset: clock propagation, event subscription, selectedEntity /
//!   trackedEntity, isDestroyed)
//! - `packages/widgets/Specs/CesiumInspector/CesiumInspectorViewModelSpec.js`
//!   and `packages/widgets/Specs/Cesium3DTilesInspector/Cesium3DTilesInspectorViewModelSpec.js`
//!   as `#[ignore]` mirrors (scene/render dependencies, Track B).
//!
//! Each JS `it` maps 1:1 to a `#[test]` named after the spec title.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use cesium_core::clock::Clock;
use cesium_core::event::Event;
use cesium_scene::scene_mode::SceneMode;

use cesium_widgets::cesium3_d_tiles_inspector_view_model::Cesium3DTilesInspectorViewModel;
use cesium_widgets::cesium_inspector_view_model::CesiumInspectorViewModel;
use cesium_widgets::cesium_widget::{CesiumWidget, CesiumWidgetOptions};
use cesium_widgets::i3_s_building_scene_layer_explorer_view_model::{
    AttributeFilter, BslWrapperElement, BuildingLevel, I3sProviderLike, I3sSublayerInput,
    I3sBuildingSceneLayerExplorerViewModel, TopLayerSelection,
};
use cesium_widgets::performance_watchdog_view_model::{
    PerformanceWatchdogViewModel, PerformanceWatchdogViewModelOptions, WatchdogScene,
    DEFAULT_LOW_FRAME_RATE_MESSAGE,
};
use cesium_widgets::projection_picker_view_model::{ProjectionPickerViewModel, ProjectionScene};
use cesium_widgets::viewer::{Viewer, ViewerOptions};

// ===========================================================================
// Widgets/PerformanceWatchdog/PerformanceWatchdogViewModel
// ===========================================================================

/// Mock scene exposing the `FrameRateMonitor` events the view model
/// subscribes to.
struct MockWatchdogScene {
    low_frame_rate: Event<()>,
    nominal_frame_rate: Event<()>,
}

impl MockWatchdogScene {
    fn new() -> Self {
        Self {
            low_frame_rate: Event::new(),
            nominal_frame_rate: Event::new(),
        }
    }
}

impl WatchdogScene for MockWatchdogScene {
    fn low_frame_rate(&self) -> &Event<()> {
        &self.low_frame_rate
    }

    fn nominal_frame_rate(&self) -> &Event<()> {
        &self.nominal_frame_rate
    }
}

#[test]
#[ignore = "DEVIATION: options.scene is required — enforced by the Rust type system (mandatory constructor argument), so no runtime DeveloperError exists to assert"]
fn performance_watchdog_throws_when_constructed_without_a_scene() {}

#[test]
fn performance_watchdog_can_be_constructed_with_just_a_scene() {
    let scene: Rc<dyn WatchdogScene> = Rc::new(MockWatchdogScene::new());
    let view_model = PerformanceWatchdogViewModel::new(Rc::clone(&scene), None);

    // expect(viewModel.lowFrameRateMessage).toBeDefined();
    assert_eq!(
        view_model.low_frame_rate_message(),
        DEFAULT_LOW_FRAME_RATE_MESSAGE
    );
    assert!(!view_model.low_frame_rate_message_dismissed());
    assert!(!view_model.showing_low_frame_rate_message());
    assert!(Rc::ptr_eq(view_model.scene(), &scene));
}

#[test]
fn performance_watchdog_honors_parameters_to_the_constructor() {
    let scene: Rc<dyn WatchdogScene> = Rc::new(MockWatchdogScene::new());
    let options = PerformanceWatchdogViewModelOptions {
        low_frame_rate_message: Some(String::from("why so slow?")),
    };

    let view_model = PerformanceWatchdogViewModel::new(Rc::clone(&scene), Some(options));

    assert_eq!(view_model.low_frame_rate_message(), "why so slow?");
    assert!(Rc::ptr_eq(view_model.scene(), &scene));
}

#[test]
fn performance_watchdog_shows_a_message_on_low_frame_rate() {
    // DEVIATION: the JS test drives the FrameRateMonitor timing
    // machinery (quiet/warmup/sampling periods) through scene.render();
    // that machinery belongs to the render loop (engine side). The view
    // model behavior in response to the monitor's lowFrameRate event is
    // mirrored 1:1 by raising the event directly.
    let scene = Rc::new(MockWatchdogScene::new());
    let view_model =
        PerformanceWatchdogViewModel::new(Rc::clone(&scene) as Rc<dyn WatchdogScene>, None);

    assert!(!view_model.showing_low_frame_rate_message());

    // The watchdog notices that our frame rate is too low.
    scene.low_frame_rate.raise_event(&());
    assert!(view_model.showing_low_frame_rate_message());
}

#[test]
fn performance_watchdog_does_not_report_a_low_frame_rate_during_the_quiet_period() {
    // DEVIATION: the quiet period lives inside FrameRateMonitor (render
    // loop side); with the monitor not raising lowFrameRate the view
    // model must not show the message — mirrored by raising nothing.
    let scene = Rc::new(MockWatchdogScene::new());
    let view_model =
        PerformanceWatchdogViewModel::new(Rc::clone(&scene) as Rc<dyn WatchdogScene>, None);

    // Even though our frame rate is too low, the watchdog shouldn't
    // bark because we're in the quiet period (no event raised).
    assert!(!view_model.showing_low_frame_rate_message());
}

#[test]
fn performance_watchdog_message_goes_away_after_the_warmup_period_if_the_frame_rate_returns_to_nominal(
) {
    // DEVIATION: as above, the FrameRateMonitor timing is driven via
    // direct event raises instead of scene.render() timing.
    let scene = Rc::new(MockWatchdogScene::new());
    let view_model =
        PerformanceWatchdogViewModel::new(Rc::clone(&scene) as Rc<dyn WatchdogScene>, None);

    assert!(!view_model.showing_low_frame_rate_message());

    // The watchdog notices that our frame rate is too low.
    scene.low_frame_rate.raise_event(&());
    assert!(view_model.showing_low_frame_rate_message());

    // The frame rate returns to nominal; the message should go away.
    scene.nominal_frame_rate.raise_event(&());
    assert!(!view_model.showing_low_frame_rate_message());
}

#[test]
fn performance_watchdog_does_not_show_the_low_frame_rate_message_again_once_it_is_dismissed() {
    let scene = Rc::new(MockWatchdogScene::new());
    let view_model =
        PerformanceWatchdogViewModel::new(Rc::clone(&scene) as Rc<dyn WatchdogScene>, None);

    assert!(!view_model.showing_low_frame_rate_message());

    // The watchdog notices that our frame rate is too low.
    scene.low_frame_rate.raise_event(&());
    assert!(view_model.showing_low_frame_rate_message());

    view_model.dismiss_message().execute();

    // Render several slow frames. The message should not re-appear.
    scene.low_frame_rate.raise_event(&());
    assert!(!view_model.showing_low_frame_rate_message());
    assert!(view_model.low_frame_rate_message_dismissed());

    let mut view_model = view_model;
    view_model.destroy();
    assert!(view_model.is_destroyed());
}

// ===========================================================================
// Widgets/ProjectionPicker/ProjectionPickerViewModel
// ===========================================================================

/// Mock scene/camera exposing the projection-related capabilities the
/// view model touches.
struct MockProjectionScene {
    mode: Cell<SceneMode>,
    morph_complete: Event<SceneMode>,
    pre_render: Event<()>,
    frustum_orthographic: Cell<bool>,
    flight: Cell<bool>,
}

impl MockProjectionScene {
    fn new() -> Self {
        Self {
            mode: Cell::new(SceneMode::Scene3D),
            morph_complete: Event::new(),
            pre_render: Event::new(),
            frustum_orthographic: Cell::new(false),
            flight: Cell::new(false),
        }
    }

    /// Mirrors `scene.morphTo2D(0)` completing immediately (duration 0):
    /// the mode changes and `morphComplete` is raised.
    fn morph_to_2d(&self) {
        self.mode.set(SceneMode::Scene2D);
        self.morph_complete.raise_event(&SceneMode::Scene2D);
    }
}

impl ProjectionScene for MockProjectionScene {
    fn mode(&self) -> SceneMode {
        self.mode.get()
    }

    fn morph_complete(&self) -> &Event<SceneMode> {
        &self.morph_complete
    }

    fn pre_render(&self) -> &Event<()> {
        &self.pre_render
    }

    fn is_orthographic_frustum(&self) -> bool {
        self.frustum_orthographic.get()
    }

    fn switch_to_perspective_frustum(&self) {
        self.frustum_orthographic.set(false);
    }

    fn switch_to_orthographic_frustum(&self) {
        self.frustum_orthographic.set(true);
    }

    fn flight_in_progress(&self) -> bool {
        self.flight.get()
    }
}

#[test]
fn projection_picker_can_construct_and_destroy() {
    let scene = Rc::new(MockProjectionScene::new());
    let mut view_model =
        ProjectionPickerViewModel::new(Rc::clone(&scene) as Rc<dyn ProjectionScene>);

    assert!(Rc::ptr_eq(view_model.scene(), &(Rc::clone(&scene) as Rc<dyn ProjectionScene>)));
    assert_eq!(scene.morph_complete.number_of_listeners(), 1);
    assert_eq!(scene.pre_render.number_of_listeners(), 1);
    assert!(!view_model.is_destroyed());
    view_model.destroy();
    assert!(view_model.is_destroyed());
    assert_eq!(scene.morph_complete.number_of_listeners(), 0);
    assert_eq!(scene.pre_render.number_of_listeners(), 0);
}

#[test]
fn projection_picker_drop_down_visible_and_toggle_drop_down_work() {
    let scene = Rc::new(MockProjectionScene::new());
    let view_model =
        ProjectionPickerViewModel::new(Rc::clone(&scene) as Rc<dyn ProjectionScene>);

    assert!(!view_model.drop_down_visible());
    view_model.toggle_drop_down().execute();
    assert!(view_model.drop_down_visible());
    view_model.set_drop_down_visible(false);
    assert!(!view_model.drop_down_visible());
}

#[test]
fn projection_picker_morphing_to_2d_calls_correct_transition() {
    let scene = Rc::new(MockProjectionScene::new());
    let view_model =
        ProjectionPickerViewModel::new(Rc::clone(&scene) as Rc<dyn ProjectionScene>);

    assert_eq!(scene.mode(), SceneMode::Scene3D);
    assert!(!view_model.is_orthographic_projection());

    scene.morph_to_2d();
    assert_eq!(scene.mode(), SceneMode::Scene2D);
    assert_eq!(view_model.scene_mode(), SceneMode::Scene2D);
    assert!(view_model.is_orthographic_projection());
}

#[test]
fn projection_picker_switching_projection_calls_correct_transition() {
    let scene = Rc::new(MockProjectionScene::new());
    let view_model =
        ProjectionPickerViewModel::new(Rc::clone(&scene) as Rc<dyn ProjectionScene>);

    assert_eq!(scene.mode(), SceneMode::Scene3D);
    assert!(!view_model.is_orthographic_projection());
    // expect(scene.camera.frustum).toBeInstanceOf(PerspectiveFrustum);
    assert!(!scene.is_orthographic_frustum());

    view_model.switch_to_orthographic().execute();
    assert!(view_model.is_orthographic_projection());
    // expect(scene.camera.frustum).toBeInstanceOf(OrthographicFrustum);
    assert!(scene.is_orthographic_frustum());

    view_model.switch_to_perspective().execute();
    assert!(!view_model.is_orthographic_projection());
    assert!(!scene.is_orthographic_frustum());
}

#[test]
fn projection_picker_selected_tooltip_changes_on_transition() {
    let scene = Rc::new(MockProjectionScene::new());
    let view_model =
        ProjectionPickerViewModel::new(Rc::clone(&scene) as Rc<dyn ProjectionScene>);

    view_model.switch_to_orthographic().execute();
    assert_eq!(view_model.selected_tooltip(), view_model.tooltip_orthographic());

    view_model.switch_to_perspective().execute();
    assert_eq!(view_model.selected_tooltip(), view_model.tooltip_perspective());
}

#[test]
#[ignore = "DEVIATION: scene is required — enforced by the Rust type system (mandatory constructor argument), so no runtime DeveloperError exists to assert"]
fn projection_picker_create_throws_with_undefined_scene() {}

// ===========================================================================
// Widgets/I3SBuildingSceneLayerExplorer/I3SBuildingSceneLayerExplorerViewModel
// ===========================================================================

/// Mock I3S provider mirroring the spec's `i3sProvider` fixture.
struct MockI3sProvider {
    sublayers: Vec<I3sSublayerInput>,
    filter_calls: RefCell<Vec<Vec<AttributeFilter>>>,
    show: Cell<bool>,
}

impl MockI3sProvider {
    fn new(sublayers: Vec<I3sSublayerInput>) -> Self {
        Self {
            sublayers,
            filter_calls: RefCell::new(Vec::new()),
            show: Cell::new(true),
        }
    }

    fn filter_calls(&self) -> Vec<Vec<AttributeFilter>> {
        self.filter_calls.borrow().clone()
    }
}

impl I3sProviderLike for MockI3sProvider {
    fn sublayers(&self) -> Vec<I3sSublayerInput> {
        self.sublayers.clone()
    }

    fn attribute_names(&self) -> Vec<String> {
        vec![String::from("BldgLevel"), String::from("testAttr")]
    }

    fn attribute_values(&self, _attribute: &str) -> Vec<i64> {
        vec![1, 0]
    }

    fn filter_by_attributes(&self, filters: &[AttributeFilter]) {
        self.filter_calls.borrow_mut().push(filters.to_vec());
    }

    fn set_show(&self, show: bool) {
        self.show.set(show);
    }
}

/// Mock `#bsl-wrapper` element recording `style.display` writes.
struct MockBslWrapper {
    display: RefCell<String>,
}

impl MockBslWrapper {
    fn new() -> Self {
        Self {
            display: RefCell::new(String::new()),
        }
    }

    fn display(&self) -> String {
        self.display.borrow().clone()
    }
}

impl BslWrapperElement for MockBslWrapper {
    fn set_style_display(&self, display: &str) {
        *self.display.borrow_mut() = display.to_owned();
    }
}

/// The spec's `i3sProvider` fixture (Full Model + Overview).
fn i3s_provider_fixture() -> Rc<MockI3sProvider> {
    Rc::new(MockI3sProvider::new(vec![
        I3sSublayerInput {
            name: String::from("Full Model"),
            model_name: Some(String::from("FullModel")),
            visibility: true,
            sublayers: vec![I3sSublayerInput {
                name: String::from("Cat1"),
                model_name: None,
                visibility: false,
                sublayers: vec![
                    I3sSublayerInput {
                        name: String::from("SubCat1"),
                        model_name: None,
                        visibility: true,
                        sublayers: Vec::new(),
                    },
                    I3sSublayerInput {
                        name: String::from("SubCat2"),
                        model_name: None,
                        visibility: false,
                        sublayers: Vec::new(),
                    },
                ],
            }],
        },
        I3sSublayerInput {
            name: String::from("Overview"),
            model_name: Some(String::from("Overview")),
            visibility: true,
            sublayers: Vec::new(),
        },
    ]))
}

/// The spec's `i3sProviderWithoutOverview` fixture.
fn i3s_provider_without_overview_fixture() -> Rc<MockI3sProvider> {
    Rc::new(MockI3sProvider::new(vec![I3sSublayerInput {
        name: String::from("Cat1"),
        model_name: None,
        visibility: false,
        sublayers: vec![
            I3sSublayerInput {
                name: String::from("SubCat1"),
                model_name: None,
                visibility: true,
                sublayers: Vec::new(),
            },
            I3sSublayerInput {
                name: String::from("SubCat2"),
                model_name: None,
                visibility: false,
                sublayers: Vec::new(),
            },
        ],
    }]))
}

#[test]
fn i3s_bsl_explorer_can_create_bsl_explorer_view_model() {
    let provider = i3s_provider_fixture();
    let view_model = I3sBuildingSceneLayerExplorerViewModel::new(
        Rc::clone(&provider) as Rc<dyn I3sProviderLike>,
        None,
    );

    assert_eq!(
        view_model.levels(),
        &[BuildingLevel::All, BuildingLevel::Level(0), BuildingLevel::Level(1)]
    );
    assert_eq!(view_model.selected_level(), BuildingLevel::All);

    assert_eq!(view_model.sublayers().len(), 2);
    {
        let overview = view_model.sublayers()[1].borrow();
        assert_eq!(overview.name, "Overview");
        assert_eq!(overview.model_name.as_deref(), Some("Overview"));
        assert!(!overview.visibility);
        assert_eq!(overview.sublayers.len(), 0);
    }
    {
        let full_model = view_model.sublayers()[0].borrow();
        assert_eq!(full_model.name, "Full Model");
        assert_eq!(full_model.model_name.as_deref(), Some("FullModel"));
        assert!(!full_model.visibility);
        assert_eq!(full_model.sublayers.len(), 1);

        let cat1 = full_model.sublayers[0].borrow();
        assert_eq!(cat1.name, "Cat1");
        assert!(cat1.visibility);
        assert_eq!(cat1.sublayers.len(), 2);

        let sub_cat1 = cat1.sublayers[0].borrow();
        assert_eq!(sub_cat1.name, "SubCat1");
        assert!(sub_cat1.visibility);
        assert_eq!(sub_cat1.sublayers.len(), 0);

        let sub_cat2 = cat1.sublayers[1].borrow();
        assert_eq!(sub_cat2.name, "SubCat2");
        assert!(!sub_cat2.visibility);
        assert_eq!(sub_cat2.sublayers.len(), 0);
    }

    assert_eq!(view_model.top_layers().len(), 3);
    assert_eq!(view_model.default_layer().unwrap().model_name, "Overview");
}

#[test]
fn i3s_bsl_explorer_can_create_bsl_explorer_view_model_if_no_overview() {
    let provider = i3s_provider_without_overview_fixture();
    let view_model = I3sBuildingSceneLayerExplorerViewModel::new(
        Rc::clone(&provider) as Rc<dyn I3sProviderLike>,
        None,
    );

    assert_eq!(view_model.sublayers().len(), 1);
    {
        let full_model = view_model.sublayers()[0].borrow();
        assert_eq!(full_model.name, "Full Model");
        assert_eq!(full_model.model_name.as_deref(), Some("FullModel"));
        assert!(!full_model.visibility);
        assert_eq!(full_model.sublayers.len(), 1);

        let cat1 = full_model.sublayers[0].borrow();
        assert_eq!(cat1.name, "Cat1");
        assert!(cat1.visibility);
        assert_eq!(cat1.sublayers.len(), 2);

        let sub_cat1 = cat1.sublayers[0].borrow();
        assert_eq!(sub_cat1.name, "SubCat1");
        assert!(sub_cat1.visibility);
        assert_eq!(sub_cat1.sublayers.len(), 0);

        let sub_cat2 = cat1.sublayers[1].borrow();
        assert_eq!(sub_cat2.name, "SubCat2");
        assert!(!sub_cat2.visibility);
        assert_eq!(sub_cat2.sublayers.len(), 0);
    }

    assert_eq!(view_model.top_layers().len(), 2);
    assert_eq!(view_model.default_layer().unwrap().model_name, "FullModel");
    // i3sProvider.show = false during construction
    assert!(!provider.show.get());
}

#[test]
fn i3s_bsl_explorer_can_handle_filtering_by_level() {
    let provider = i3s_provider_fixture();
    let view_model = I3sBuildingSceneLayerExplorerViewModel::new(
        Rc::clone(&provider) as Rc<dyn I3sProviderLike>,
        None,
    );

    view_model.set_current_level(BuildingLevel::Level(1));
    let calls = provider.filter_calls();
    assert_eq!(
        calls.last().unwrap(),
        &[AttributeFilter {
            name: String::from("BldgLevel"),
            values: vec![1],
        }]
    );

    view_model.set_current_level(BuildingLevel::All);
    let calls = provider.filter_calls();
    assert!(calls.last().unwrap().is_empty());
}

#[test]
fn i3s_bsl_explorer_can_handle_top_layer_selection() {
    let bsl_wrapper = Rc::new(MockBslWrapper::new());
    let provider = i3s_provider_fixture();
    let view_model = I3sBuildingSceneLayerExplorerViewModel::new(
        Rc::clone(&provider) as Rc<dyn I3sProviderLike>,
        Some(Rc::clone(&bsl_wrapper) as Rc<dyn BslWrapperElement>),
    );

    view_model.set_current_layer(Some(TopLayerSelection {
        name: String::from("Full Model"),
        model_name: String::from("FullModel"),
        index: 1,
    }));
    view_model.set_current_level(BuildingLevel::Level(1));
    view_model.set_current_layer(Some(TopLayerSelection {
        name: String::from("Overview"),
        model_name: String::from("Overview"),
        index: 0,
    }));
    assert!(view_model.sublayers()[0].borrow().visibility);
    assert!(!view_model.sublayers()[1].borrow().visibility);
    assert_eq!(view_model.selected_level(), BuildingLevel::Level(1));
    assert_eq!(view_model.current_level(), BuildingLevel::All);
    assert_eq!(bsl_wrapper.display(), "none");

    view_model.set_current_layer(Some(TopLayerSelection {
        name: String::from("Full Model"),
        model_name: String::from("FullModel"),
        index: 1,
    }));
    assert!(!view_model.sublayers()[0].borrow().visibility);
    assert!(view_model.sublayers()[1].borrow().visibility);
    assert_eq!(view_model.current_level(), BuildingLevel::Level(1));
    assert_eq!(bsl_wrapper.display(), "block");
}

#[test]
fn i3s_bsl_explorer_can_handle_top_layer_selection_if_no_overview() {
    let bsl_wrapper = Rc::new(MockBslWrapper::new());
    let provider = i3s_provider_without_overview_fixture();
    let view_model = I3sBuildingSceneLayerExplorerViewModel::new(
        Rc::clone(&provider) as Rc<dyn I3sProviderLike>,
        Some(Rc::clone(&bsl_wrapper) as Rc<dyn BslWrapperElement>),
    );

    view_model.set_current_layer(Some(TopLayerSelection {
        name: String::from("Full Model"),
        model_name: String::from("FullModel"),
        index: 0,
    }));
    assert!(view_model.sublayers()[0].borrow().visibility);
    assert_eq!(bsl_wrapper.display(), "block");
    // i3sProvider.show = isFullModel(layer)
    assert!(provider.show.get());
}

// ===========================================================================
// Widgets/Viewer/Viewer — engine-side semantics subset
// (clock propagation, event subscription, selectedEntity/trackedEntity,
// isDestroyed)
// ===========================================================================

#[test]
fn viewer_constructor_sets_default_values() {
    // Subset of the JS "constructor sets default values": the widget /
    // clock / data source plumbing and the destroy lifecycle. DOM-only
    // widget members are out of scope for the GPU-free widgets crate.
    let viewer = Viewer::new(None);

    assert!(!viewer.is_destroyed());
    // viewer.cesiumWidget is a CesiumWidget
    let _ = viewer.cesium_widget();
    // viewer.clock is the cesiumWidget clock
    let _ = viewer.clock();
    // viewer.dataSources is a DataSourceCollection
    assert_eq!(viewer.data_sources().length(), 0);
}

#[test]
fn viewer_can_get_and_set_selected_entity() {
    let mut viewer = Viewer::new(None);

    viewer.set_selected_entity_id(Some(String::from("entity-1")));
    assert_eq!(viewer.selected_entity_id(), Some("entity-1"));

    viewer.set_selected_entity_id(None);
    assert_eq!(viewer.selected_entity_id(), None);
}

#[test]
fn viewer_raises_an_event_when_the_selected_entity_changes() {
    let mut viewer = Viewer::new(None);

    let received = Rc::new(RefCell::new(Vec::<Option<String>>::new()));
    let received_for_listener = Rc::clone(&received);
    viewer
        .selected_entity_changed()
        .add_listener(move |new_selection: &Option<String>| {
            received_for_listener.borrow_mut().push(new_selection.clone());
        });

    viewer.set_selected_entity_id(Some(String::from("entity-1")));
    assert_eq!(
        received.borrow().last().unwrap(),
        &Some(String::from("entity-1"))
    );

    viewer.set_selected_entity_id(None);
    assert_eq!(received.borrow().last().unwrap(), &None);
}

#[test]
fn cesium_widget_propagates_options_clock() {
    // Clock propagation: the options clock becomes the widget clock.
    let clock = Clock::new(None, None, None, Some(5.0), None, None, None, None);
    let options = CesiumWidgetOptions {
        clock: Some(clock),
        ..Default::default()
    };
    let widget = CesiumWidget::new(Some(options));

    assert_eq!(widget.clock().get_multiplier(), 5.0);
}

#[test]
fn cesium_widget_propagates_options_scene_mode() {
    let options = CesiumWidgetOptions {
        scene_mode: SceneMode::Scene2D,
        ..Default::default()
    };
    let widget = CesiumWidget::new(Some(options));

    assert_eq!(widget.scene().mode(), SceneMode::Scene2D);
}

#[test]
fn viewer_tracked_entity_delegates_to_cesium_widget_and_raises_event() {
    let mut viewer = Viewer::new(None);

    let received = Rc::new(RefCell::new(Vec::<Option<String>>::new()));
    let received_for_listener = Rc::clone(&received);
    viewer
        .tracked_entity_changed()
        .add_listener(move |new_tracked: &Option<String>| {
            received_for_listener.borrow_mut().push(new_tracked.clone());
        });

    viewer.set_tracked_entity_id(Some(String::from("entity-1")));
    assert_eq!(viewer.tracked_entity_id(), Some("entity-1"));
    assert_eq!(
        viewer.cesium_widget().tracked_entity_id(),
        Some("entity-1")
    );
    assert_eq!(
        received.borrow().last().unwrap(),
        &Some(String::from("entity-1"))
    );

    viewer.set_tracked_entity_id(None);
    assert_eq!(viewer.tracked_entity_id(), None);
    assert_eq!(received.borrow().last().unwrap(), &None);
}

#[test]
fn viewer_destroy_destroys_cesium_widget() {
    let mut viewer = Viewer::new(None);
    assert!(!viewer.is_destroyed());
    assert!(!viewer.cesium_widget().is_destroyed());

    viewer.destroy();
    assert!(viewer.is_destroyed());
    assert!(viewer.cesium_widget().is_destroyed());
}

#[test]
fn viewer_options_default_matches_js_defaults() {
    // Subset of JS constructor option defaults relevant to the engine
    // side (widgets default to enabled, vrButton defaults to false).
    let options = ViewerOptions::default();
    assert!(options.animation);
    assert!(options.base_layer_picker);
    assert!(options.fullscreen_button);
    assert!(options.geocoder);
    assert!(options.home_button);
    assert!(options.info_box);
    assert!(options.navigation_help_button);
    assert!(options.projection_picker);
    assert!(options.scene_mode_picker);
    assert!(options.selection_indicator);
    assert!(options.timeline);
    assert!(!options.vr_button);
}

// ===========================================================================
// Widgets/CesiumInspector/CesiumInspectorViewModel
//
// DEVIATION: every case in CesiumInspectorViewModelSpec.js depends on a
// real WebGL Scene (createScene, scene.render, globe internals,
// primitives, QuadtreeTile rendering); the widgets crate is GPU-free,
// so the cases are mirrored 1:1 as `#[ignore]` anchors until the Scene
// render pipeline (Track B) is available.
// ===========================================================================

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene (createScene/scene.render); mirrored from CesiumInspectorViewModelSpec.js"]
fn cesium_inspector_view_model_can_create_and_destroy() {
    let _ = CesiumInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene; mirrored from CesiumInspectorViewModelSpec.js"]
fn cesium_inspector_view_model_throws_if_scene_is_undefined() {
    let _ = CesiumInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene; mirrored from CesiumInspectorViewModelSpec.js"]
fn cesium_inspector_view_model_throws_if_performance_container_is_undefined() {
    let _ = CesiumInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene; mirrored from CesiumInspectorViewModelSpec.js"]
fn cesium_inspector_view_model_show_frustums() {
    let _ = CesiumInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene; mirrored from CesiumInspectorViewModelSpec.js"]
fn cesium_inspector_view_model_show_performance() {
    let _ = CesiumInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene; mirrored from CesiumInspectorViewModelSpec.js"]
fn cesium_inspector_view_model_primitive_bounding_sphere() {
    let _ = CesiumInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene; mirrored from CesiumInspectorViewModelSpec.js"]
fn cesium_inspector_view_model_primitive_filter() {
    let _ = CesiumInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene; mirrored from CesiumInspectorViewModelSpec.js"]
fn cesium_inspector_view_model_primitive_reference_frame() {
    let _ = CesiumInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene; mirrored from CesiumInspectorViewModelSpec.js"]
fn cesium_inspector_view_model_show_wireframe() {
    let _ = CesiumInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene; mirrored from CesiumInspectorViewModelSpec.js"]
fn cesium_inspector_view_model_suspend_updates() {
    let _ = CesiumInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene; mirrored from CesiumInspectorViewModelSpec.js"]
fn cesium_inspector_view_model_show_tile_coords() {
    let _ = CesiumInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene; mirrored from CesiumInspectorViewModelSpec.js"]
fn cesium_inspector_view_model_show_tile_bounding_sphere() {
    let _ = CesiumInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene; mirrored from CesiumInspectorViewModelSpec.js"]
fn cesium_inspector_view_model_filter_tile() {
    let _ = CesiumInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene; mirrored from CesiumInspectorViewModelSpec.js"]
fn cesium_inspector_view_model_does_not_try_to_render_a_non_renderable_tile() {
    let _ = CesiumInspectorViewModel::default();
}

// ===========================================================================
// Widgets/Cesium3DTilesInspector/Cesium3DTilesInspectorViewModel
//
// DEVIATION: every case in Cesium3DTilesInspectorViewModelSpec.js
// depends on a real WebGL Scene plus a loaded Cesium3DTileset
// (createScene, Cesium3DTileset.fromUrl, tileset debug flags); the
// widgets crate is GPU-free, so the cases are mirrored 1:1 as
// `#[ignore]` anchors until the Scene render pipeline (Track B) and the
// GPU-side tileset integration are available.
// ===========================================================================

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_can_create_and_destroy() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_throws_if_scene_is_undefined() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_throws_if_performance_container_is_undefined() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_tileset_options_show_properties() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_display_options_colorize() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_display_options_wireframe() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_display_options_show_bounding_volumes() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_display_options_show_content_volumes() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_display_options_show_request_volumes() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_display_options_show_only_picked_tile_debug_label() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_display_options_show_geometric_error() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_display_options_show_rendering_statistics() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_display_options_show_memory_usage() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_display_options_show_url() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_display_options_point_cloud_shading() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_display_options_geometric_error_scale() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_display_options_maximum_attenuation() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_display_options_base_resolution() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_display_options_eye_dome_lighting() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_display_options_eye_dome_lighting_strength() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_display_options_eye_dome_lighting_radius() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_update_options_freeze_frame() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_update_options_maximum_screen_space_error() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_update_options_dynamic_screen_space_error() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_update_options_density_slider_exponential_scale() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_style_options_loads_tileset_style() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_style_options_does_not_throw_on_invalid_syntax() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_style_options_recompiles_style() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}

#[test]
#[ignore = "DEVIATION: requires real WebGL Scene + tileset; mirrored from Cesium3DTilesInspectorViewModelSpec.js"]
fn cesium3_d_tiles_inspector_view_model_style_options_does_not_throw_on_invalid_value() {
    let _ = Cesium3DTilesInspectorViewModel::default();
}
