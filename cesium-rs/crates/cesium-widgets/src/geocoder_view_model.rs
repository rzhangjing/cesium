//! Ported from `packages/widgets/Source/Geocoder/GeocoderViewModel.js`.
//!
//! The view model for the `Geocoder` widget: location search with camera
//! flights.
//!
//! DEVIATIONS (all documented per-item below):
//! - the JS view model operates on a real `Scene` with a credit display,
//!   ellipsoid, terrain provider and camera tweens; the widgets layer is
//!   GPU-free, so the scene is injected through the [`GeocoderScene`]
//!   trait and geocoder services through [`GeocoderServiceLike`]; the
//!   engine wiring for a real scene is provided by
//!   [`SceneGeocoderAdapter`] (credits flow into the scene
//!   [`CreditDisplay`](cesium_scene::credit_display::CreditDisplay) and
//!   the camera flight goes through
//!   [`cesium_scene::scene::Scene::fly_to`]; the terrain-sampler surface
//!   stays injected through [`FlyToRectangleScene`] because the Scene
//!   port has no terrain provider / map projection surface yet);
//! - JS geocoding is promise-based; the Rust port is synchronous
//!   (`geocode` returns the results directly and never rejects);
//! - rectangle destinations go through the engine
//!   `compute_fly_to_location_for_rectangle` and the cartographic paths
//!   mirror the JS `computeFlyToLocationForCartographic`, both through the
//!   [`FlyToRectangleScene`] terrain-sampler seam (the JS promise chain
//!   over `sampleTerrainMostDetailed` is synchronous here);
//! - the knockout `rateLimit`ed `searchText` subscription that triggers
//!   suggestion updates is modeled as the explicit
//!   [`GeocoderViewModel::update_search_suggestions`] method;
//! - `_adjustSuggestionsScroll` is a DOM scroll handler and is a no-op.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::credit::Credit;
use cesium_core::developer_error::throw_developer_error;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::event::Event;
use cesium_core::geocode_type::GeocodeType;
use cesium_core::geocoder_service::{
    GeocodeDestination as CoreGeocodeDestination, GeocoderResult as CoreGeocoderResult,
    GeocoderService as CoreGeocoderService,
};
use cesium_core::ion_geocoder_service::IonGeocoderService;
use cesium_core::math::CesiumMath;
use cesium_core::rectangle::Rectangle;
use cesium_scene::compute_fly_to_location_for_rectangle::{
    compute_fly_to_location_for_rectangle, FlyToRectangleScene,
};
use cesium_scene::scene::Scene;
use cesium_scene::scene_mode::SceneMode;

/// The height we use if geocoding to a specific point instead of a
/// rectangle, mirroring the JS `DEFAULT_HEIGHT` constant.
pub const DEFAULT_HEIGHT: f64 = 1000.0;

/// A destination returned by a geocoder service, mirroring the JS
/// `Cartesian3 | Rectangle` result destination.
#[derive(Clone, Debug)]
pub enum GeocodeDestination {
    /// A point destination.
    Cartesian(Cartesian3),
    /// A rectangular region destination.
    Rectangle(Rectangle),
}

impl PartialEq for GeocodeDestination {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (GeocodeDestination::Cartesian(a), GeocodeDestination::Cartesian(b)) => a == b,
            (GeocodeDestination::Rectangle(a), GeocodeDestination::Rectangle(b)) => {
                a.west == b.west && a.south == b.south && a.east == b.east && a.north == b.north
            }
            _ => false,
        }
    }
}

/// A single geocoder result, mirroring the JS
/// `{ displayName, destination, attributions }` shape.
#[derive(Clone, Debug, PartialEq)]
pub struct GeocoderResult {
    /// The display name of the result.
    pub display_name: String,
    /// The destination of the result.
    pub destination: GeocodeDestination,
    /// Credit attribution html strings
    /// (`GeocoderService.getCreditsFromResult` turns each into a
    /// `Credit(html, false)`).
    pub attributions: Vec<String>,
}

/// The geocoder service abstraction, mirroring the parts of CesiumJS
/// geocoder services (e.g. `IonGeocoderService` or plain option objects)
/// used by the view model.
pub trait GeocoderServiceLike {
    /// Performs the geocode query. DEVIATION: synchronous instead of
    /// promise-based, and never rejects (the JS `attemptGeocode` reject
    /// path has no analogue).
    fn geocode(&self, query: &str, geocode_type: GeocodeType) -> Vec<GeocoderResult>;
    /// The service credit (`service.credit`), if any.
    fn credit(&self) -> Option<Credit>;
}

/// Bridges an engine [`CoreGeocoderService`] (e.g. the real
/// [`IonGeocoderService`]) to the widget-local [`GeocoderServiceLike`]
/// surface, mapping the engine result types to the view model types.
///
/// DEVIATION: the engine `GeocoderAttribution.collapsible` flag is
/// dropped by the mapping (the widget-local result mirrors the
/// attribution html only, and the widget-local
/// `get_credits_from_result` creates `Credit(html, false)` credits).
pub struct CoreGeocoderServiceAdapter {
    inner: Box<dyn CoreGeocoderService>,
}

impl CoreGeocoderServiceAdapter {
    /// Wraps an engine geocoder service implementation.
    pub fn new(inner: Box<dyn CoreGeocoderService>) -> Self {
        Self { inner }
    }
}

impl GeocoderServiceLike for CoreGeocoderServiceAdapter {
    fn geocode(&self, query: &str, geocode_type: GeocodeType) -> Vec<GeocoderResult> {
        self.inner
            .geocode(query, geocode_type)
            .into_iter()
            .map(map_core_geocoder_result)
            .collect()
    }

    fn credit(&self) -> Option<Credit> {
        self.inner.credit()
    }
}

/// Maps an engine [`CoreGeocoderResult`] to the widget-local
/// [`GeocoderResult`].
fn map_core_geocoder_result(result: CoreGeocoderResult) -> GeocoderResult {
    GeocoderResult {
        display_name: result.display_name,
        destination: match result.destination {
            CoreGeocodeDestination::Rectangle(rectangle) => {
                GeocodeDestination::Rectangle(rectangle)
            }
            CoreGeocodeDestination::Cartesian3(cartesian) => {
                GeocodeDestination::Cartesian(cartesian)
            }
        },
        attributions: result.attributions.map_or_else(Vec::new, |attributions| {
            attributions
                .into_iter()
                .map(|attribution| attribution.html)
                .collect()
        }),
    }
}

/// The scene abstraction required by [`GeocoderViewModel`], mirroring the
/// parts of CesiumJS `Scene` the view model touches (credit display,
/// ellipsoid, camera flights).
///
/// Extends [`FlyToRectangleScene`] so the rectangle flight path can call
/// the engine `compute_fly_to_location_for_rectangle` (the JS passes the
/// concrete `Scene` to it).
pub trait GeocoderScene: FlyToRectangleScene {
    /// Mirrors `scene.isDestroyed()`.
    fn is_destroyed(&self) -> bool;
    /// Mirrors `scene.frameState.creditDisplay.isDestroyed()`.
    fn credit_display_is_destroyed(&self) -> bool;
    /// Mirrors `scene.frameState.creditDisplay.addStaticCredit(credit)`.
    fn add_static_credit(&self, credit: Credit);
    /// Mirrors `scene.frameState.creditDisplay.removeStaticCredit(credit)`.
    fn remove_static_credit(&self, credit: &Credit);
    /// Mirrors `camera.flyTo({ destination, duration, complete })`
    /// (DEVIATION: `endTransform: Matrix4.IDENTITY` is implicit).
    fn fly_to(
        &self,
        destination: Cartesian3,
        duration: Option<f64>,
        complete: Box<dyn FnOnce()>,
    );
}

/// Bridges a real [`Scene`] to the widget-local [`GeocoderScene`]
/// surface, mirroring the JS `new GeocoderViewModel({ scene })` wiring:
/// the credits flow into the scene
/// [`CreditDisplay`](cesium_scene::credit_display::CreditDisplay) and
/// [`GeocoderViewModel::fly_to_destination`] flies through
/// [`Scene::fly_to`].
///
/// DEVIATION: the terrain-sampler surface required by the rectangle
/// flight path ([`FlyToRectangleScene`]) stays injected because the
/// `Scene` port has no terrain provider / map projection /
/// `getRectangleCameraCoordinates` surface yet.
pub struct SceneGeocoderAdapter {
    scene: Rc<Scene>,
    surface: Rc<dyn FlyToRectangleScene>,
}

impl SceneGeocoderAdapter {
    /// Wraps a real scene and an injected rectangle-flight surface.
    pub fn new(scene: Rc<Scene>, surface: Rc<dyn FlyToRectangleScene>) -> Self {
        Self { scene, surface }
    }
}

impl FlyToRectangleScene for SceneGeocoderAdapter {
    fn mode(&self) -> SceneMode {
        self.surface.mode()
    }

    fn ellipsoid(&self) -> Ellipsoid {
        self.surface.ellipsoid()
    }

    fn unproject(&self, cartesian: &Cartesian3) -> Cartographic {
        self.surface.unproject(cartesian)
    }

    fn get_rectangle_camera_coordinates(&self, rectangle: &Rectangle) -> Cartesian3 {
        self.surface.get_rectangle_camera_coordinates(rectangle)
    }

    fn terrain_provider_defined(&self) -> bool {
        self.surface.terrain_provider_defined()
    }

    fn terrain_availability_defined(&self) -> bool {
        self.surface.terrain_availability_defined()
    }

    fn sample_terrain_most_detailed(&self, positions: &[Cartographic]) -> Vec<Option<f64>> {
        self.surface.sample_terrain_most_detailed(positions)
    }
}

impl GeocoderScene for SceneGeocoderAdapter {
    fn is_destroyed(&self) -> bool {
        self.scene.is_destroyed()
    }

    fn credit_display_is_destroyed(&self) -> bool {
        self.scene.credit_display_is_destroyed()
    }

    fn add_static_credit(&self, credit: Credit) {
        self.scene.add_static_credit(credit);
    }

    fn remove_static_credit(&self, credit: &Credit) {
        self.scene.remove_static_credit(credit);
    }

    fn fly_to(
        &self,
        destination: Cartesian3,
        duration: Option<f64>,
        complete: Box<dyn FnOnce()>,
    ) {
        // DEVIATION: the widget-local `GeocoderScene::fly_to` takes a
        // required `complete` callback; [`Scene::fly_to`] takes an
        // `Option`, so the callback is wrapped in `Some`.
        self.scene.fly_to(destination, duration, Some(complete));
    }
}

/// The callback invoked after a successful geocode, mirroring
/// `Geocoder.DestinationFoundFunction`.
pub type DestinationFound = dyn Fn(&GeocoderViewModel, GeocodeDestination);

/// Options for constructing a [`GeocoderViewModel`], mirroring the JS
/// `options` object.
pub struct GeocoderViewModelOptions {
    /// The Scene instance to use.
    pub scene: Rc<dyn GeocoderScene>,
    /// Geocoder services to use for geocoding queries. Defaults to the
    /// JS `[new IonGeocoderService({ scene: options.scene })]` (the JS
    /// `options.scene` argument is dropped by the engine constructor,
    /// which only used it for the default-token static credit).
    pub geocoder_services: Option<Vec<Rc<dyn GeocoderServiceLike>>>,
    /// The duration of the camera flight to an entered location, in
    /// seconds.
    pub flight_duration: Option<f64>,
    /// A callback invoked after a successful geocode; defaults to
    /// [`GeocoderViewModel::fly_to_destination`].
    pub destination_found: Option<Box<DestinationFound>>,
    /// True if the geocoder should query as the user types to
    /// autocomplete (default true).
    pub autocomplete: Option<bool>,
}

struct GeocoderInner {
    search_text: String,
    is_search_in_progress: bool,
    was_geocode_cancelled: bool,
    suggestions: Vec<GeocoderResult>,
    selected_suggestion: Option<GeocoderResult>,
    show_suggestions: bool,
    previous_credits: Vec<Credit>,
    focus_textbox: bool,
}

/// The view model for the `Geocoder` widget.
pub struct GeocoderViewModel {
    inner: RefCell<GeocoderInner>,
    scene: Rc<dyn GeocoderScene>,
    geocoder_services: Vec<Rc<dyn GeocoderServiceLike>>,
    flight_duration: Cell<Option<f64>>,
    /// Gets or sets a value indicating if this instance should always show
    /// its text input field (observable).
    pub keep_expanded: Cell<bool>,
    /// True if the geocoder should query as the user types to
    /// autocomplete.
    pub auto_complete: Cell<bool>,
    complete: Rc<Event<()>>,
    destination_found: Box<DestinationFound>,
    destroyed: Cell<bool>,
}

impl GeocoderViewModel {
    /// Creates a new geocoder view model, mirroring
    /// `new GeocoderViewModel(options)`.
    pub fn new(options: GeocoderViewModelOptions) -> Self {
        Self {
            inner: RefCell::new(GeocoderInner {
                search_text: String::new(),
                is_search_in_progress: false,
                was_geocode_cancelled: false,
                suggestions: Vec::new(),
                selected_suggestion: None,
                show_suggestions: true,
                previous_credits: Vec::new(),
                focus_textbox: false,
            }),
            // JS default: `[new IonGeocoderService({ scene: options.scene })]`
            // (the `scene` option is dropped by the engine constructor).
            geocoder_services: options.geocoder_services.unwrap_or_else(|| {
                vec![Rc::new(CoreGeocoderServiceAdapter::new(Box::new(
                    IonGeocoderService::new(None),
                ))) as Rc<dyn GeocoderServiceLike>]
            }),
            scene: options.scene,
            flight_duration: Cell::new(options.flight_duration),
            keep_expanded: Cell::new(false),
            auto_complete: Cell::new(options.autocomplete.unwrap_or(true)),
            complete: Rc::new(Event::new()),
            destination_found: options.destination_found.unwrap_or(Box::new(
                |view_model: &GeocoderViewModel, destination: GeocodeDestination| {
                    view_model.fly_to_destination(destination);
                },
            )),
            destroyed: Cell::new(false),
        }
    }

    /// Gets the scene to control, mirroring the readonly `scene` property.
    pub fn scene(&self) -> &Rc<dyn GeocoderScene> {
        &self.scene
    }

    /// Gets the event triggered on flight completion, mirroring the
    /// readonly `complete` property.
    pub fn complete(&self) -> &Event<()> {
        &self.complete
    }

    /// Gets or sets the duration of the camera flight in seconds,
    /// mirroring the `flightDuration` property.
    pub fn flight_duration(&self) -> Option<f64> {
        self.flight_duration.get()
    }

    /// Sets the flight duration.
    ///
    /// # Panics
    /// Panics with a `DeveloperError` when `value` is negative.
    pub fn set_flight_duration(&self, value: Option<f64>) {
        //>>includeStart('debug', pragmas.debug);
        if let Some(value) = value {
            if value < 0.0 {
                throw_developer_error("value must be positive.");
            }
        }
        //>>includeEnd('debug');
        self.flight_duration.set(value);
    }

    /// Gets a value indicating whether a search is currently in progress,
    /// mirroring the `isSearchInProgress` property.
    pub fn is_search_in_progress(&self) -> bool {
        self.inner.borrow().is_search_in_progress
    }

    /// Gets the text to search for, mirroring the `searchText` getter
    /// (returns `"Searching..."` while a search is in progress).
    pub fn search_text(&self) -> String {
        if self.is_search_in_progress() {
            return "Searching...".to_string();
        }
        self.inner.borrow().search_text.clone()
    }

    /// Sets the text to search for, mirroring the `searchText` setter.
    /// DEVIATION: the JS "value must be a valid string" DeveloperError is
    /// guaranteed by the type system.
    pub fn set_search_text(&self, value: &str) {
        self.inner.borrow_mut().search_text = value.to_string();
    }

    /// Direct access to the `_searchText` backing field, mirroring specs
    /// that poke `viewModel._searchText` directly.
    pub fn raw_search_text(&self) -> String {
        self.inner.borrow().search_text.clone()
    }

    /// Sets the `_searchText` backing field directly.
    pub fn set_raw_search_text(&self, value: &str) {
        self.inner.borrow_mut().search_text = value.to_string();
    }

    /// Whether the suggestion panel should be shown, mirroring the
    /// `_suggestionsVisible` knockout computed.
    pub fn suggestions_visible(&self) -> bool {
        let inner = self.inner.borrow();
        !inner.suggestions.is_empty() && inner.show_suggestions
    }

    /// Gets the currently selected geocoder search suggestion, mirroring
    /// the readonly `selectedSuggestion` property.
    pub fn selected_suggestion(&self) -> Option<GeocoderResult> {
        self.inner.borrow().selected_suggestion.clone()
    }

    /// Gets the list of geocoder search suggestions, mirroring the
    /// readonly `suggestions` property.
    pub fn suggestions(&self) -> Vec<GeocoderResult> {
        self.inner.borrow().suggestions.clone()
    }

    /// Performs a search, mirroring invoking the `search` command with the
    /// default `GeocodeType.SEARCH`. DEVIATION: the JS `search` is a
    /// `Command`; the Rust port models it as a method since the command
    /// body needs `&self`.
    pub fn search(&self) {
        self.search_with_type(GeocodeType::Search);
    }

    /// Performs a search with an explicit geocode type, mirroring
    /// `this._searchCommand(geocodeType)`.
    pub fn search_with_type(&self, geocode_type: GeocodeType) {
        self.inner.borrow_mut().focus_textbox = false;
        if let Some(selected) = self.inner.borrow().selected_suggestion.clone() {
            self.activate_suggestion(selected);
            return;
        }
        self.hide_suggestions();
        if self.is_search_in_progress() {
            self.cancel_geocode();
        } else {
            self.geocode(geocode_type);
        }
    }

    /// Sets `_selectedSuggestion` to `None`, mirroring `deselectSuggestion`.
    pub fn deselect_suggestion(&self) {
        self.inner.borrow_mut().selected_suggestion = None;
    }

    /// Activates a suggestion, mirroring `activateSuggestion(data)`.
    pub fn activate_suggestion(&self, data: GeocoderResult) {
        self.hide_suggestions();
        self.inner.borrow_mut().search_text = data.display_name.clone();
        let destination = data.destination.clone();
        self.clear_suggestions();
        (self.destination_found)(self, destination);
    }

    /// Hides the suggestion panel, mirroring `hideSuggestions()`.
    pub fn hide_suggestions(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.show_suggestions = false;
        inner.selected_suggestion = None;
    }

    /// Shows the suggestion panel, mirroring `showSuggestions()`.
    pub fn show_suggestions(&self) {
        self.inner.borrow_mut().show_suggestions = true;
    }

    /// Updates the suggestion list from the geocoder services, mirroring
    /// the (test-exposed) `GeocoderViewModel._updateSearchSuggestions`.
    /// DEVIATION: synchronous instead of promise-based.
    pub fn update_search_suggestions(&self) {
        if !self.auto_complete.get() {
            return;
        }

        let query = self.inner.borrow().search_text.clone();

        self.clear_suggestions();
        self.clear_credits();

        if has_only_whitespace(&query) {
            return;
        }

        for service in &self.geocoder_services {
            let new_results = service.geocode(&query, GeocodeType::Autocomplete);
            {
                let mut inner = self.inner.borrow_mut();
                inner.suggestions.extend(new_results.iter().cloned());
            }
            if !new_results.is_empty() {
                let mut use_default_credit = true;
                for result in &new_results {
                    let credits = get_credits_from_result(result);
                    use_default_credit = use_default_credit && credits.is_none();
                    self.update_credits(credits);
                }

                // Use the service credit if there were no attributions in
                // the results.
                if use_default_credit {
                    self.update_credit(service.credit());
                }
            }

            if self.inner.borrow().suggestions.len() >= 5 {
                return;
            }
        }
    }

    /// Adjusts the suggestions scroll position. DEVIATION: DOM scrolling
    /// has no headless analogue; this is a no-op (mirrors the JS spy in
    /// the arrow-key spec).
    pub fn adjust_suggestions_scroll(&self, _focused_item_index: usize) {}

    /// Handles the ArrowDown key over the suggestion list, mirroring the
    /// JS `handleArrowDown` (module-level, test-exposed through `_handleArrowDown`).
    pub fn handle_arrow_down(&self) {
        let (suggestions, selected) = {
            let inner = self.inner.borrow();
            (inner.suggestions.clone(), inner.selected_suggestion.clone())
        };
        if suggestions.is_empty() {
            return;
        }
        let number_of_suggestions = suggestions.len();
        let current_index = index_of(&suggestions, &selected);
        let next = (current_index.wrapping_add(1)) % number_of_suggestions;
        self.inner.borrow_mut().selected_suggestion = Some(suggestions[next].clone());
        self.adjust_suggestions_scroll(next);
    }

    /// Handles the ArrowUp key over the suggestion list, mirroring the
    /// JS `handleArrowUp`.
    pub fn handle_arrow_up(&self) {
        let (suggestions, selected) = {
            let inner = self.inner.borrow();
            (inner.suggestions.clone(), inner.selected_suggestion.clone())
        };
        if suggestions.is_empty() {
            return;
        }
        let current_index = index_of(&suggestions, &selected);
        if current_index == usize::MAX || current_index == 0 {
            self.inner.borrow_mut().selected_suggestion = None;
            return;
        }
        let next = current_index - 1;
        self.inner.borrow_mut().selected_suggestion = Some(suggestions[next].clone());
        self.adjust_suggestions_scroll(next);
    }

    /// Flies the camera to the destination found by a successful geocode,
    /// mirroring the static `GeocoderViewModel.flyToDestination`.
    pub fn fly_to_destination(&self, destination: GeocodeDestination) {
        let ellipsoid = self.scene.ellipsoid();

        let cartographic = match destination {
            GeocodeDestination::Rectangle(rectangle) => {
                // Some geocoders return a Rectangle of zero width/height,
                // treat it like a point instead.
                let zero_area = CesiumMath::equals_epsilon(
                    rectangle.south,
                    rectangle.north,
                    Some(CesiumMath::EPSILON7),
                    None,
                ) && CesiumMath::equals_epsilon(
                    rectangle.east,
                    rectangle.west,
                    Some(CesiumMath::EPSILON7),
                    None,
                );
                if zero_area {
                    compute_fly_to_location_for_cartographic(
                        &Rectangle::center(&rectangle),
                        &*self.scene,
                    )
                } else {
                    compute_fly_to_location_for_rectangle(&rectangle, &*self.scene)
                }
            }
            GeocodeDestination::Cartesian(cartesian) => {
                let mut cartographic = Cartographic::default();
                ellipsoid.cartesian_to_cartographic(&cartesian, &mut cartographic);
                compute_fly_to_location_for_cartographic(&cartographic, &*self.scene)
            }
        };

        // DEVIATION: the JS promise chain over `sampleTerrainMostDetailed`
        // is synchronous through the [`FlyToRectangleScene`] sampler seam.
        let final_cartographic = cartographic;

        let mut final_destination = Cartesian3::default();
        ellipsoid.cartographic_to_cartesian(&final_cartographic, &mut final_destination);

        let complete = Rc::clone(&self.complete);
        let flight_duration = self.flight_duration.get();
        self.scene.fly_to(
            final_destination,
            flight_duration,
            Box::new(move || {
                complete.raise_event(&());
            }),
        );
    }

    /// Returns whether this view model has been destroyed, mirroring
    /// `isDestroyed()`.
    pub fn is_destroyed(&self) -> bool {
        self.destroyed.get()
    }

    /// Destroys the view model, mirroring `destroy()`: clears the
    /// previously registered credits and marks the object destroyed.
    /// DEVIATION: the knockout suggestion subscription dispose has no
    /// analogue (see module docs).
    pub fn destroy(&self) {
        self.clear_credits();
        self.destroyed.set(true);
    }

    // ------------------------------------------------------------------
    // private helpers
    // ------------------------------------------------------------------

    /// Mirrors the module-level `geocode(viewModel, geocoderServices,
    /// geocodeType)` function.
    fn geocode(&self, geocode_type: GeocodeType) {
        let query = self.inner.borrow().search_text.clone();

        if has_only_whitespace(&query) {
            self.show_suggestions();
            return;
        }

        self.inner.borrow_mut().is_search_in_progress = true;
        self.inner.borrow_mut().was_geocode_cancelled = false;

        let mut result: Option<(usize, Vec<GeocoderResult>)> = None;
        for (i, service) in self.geocoder_services.iter().enumerate() {
            if self.inner.borrow().was_geocode_cancelled {
                return;
            }

            // DEVIATION: synchronous geocode; the JS `attemptGeocode`
            // reject path has no analogue.
            let results = service.geocode(&query, geocode_type);
            if !results.is_empty() {
                result = Some((i, results));
                break;
            }
        }

        if self.inner.borrow().was_geocode_cancelled {
            return;
        }

        self.inner.borrow_mut().is_search_in_progress = false;
        self.clear_credits();

        if let Some((service_index, geocoder_results)) = result {
            self.inner.borrow_mut().search_text =
                geocoder_results[0].display_name.clone();
            let destination = geocoder_results[0].destination.clone();
            (self.destination_found)(self, destination);
            let credits = get_credits_from_result(&geocoder_results[0]);
            let had_credits = credits.is_some();
            self.update_credits(credits);
            // If the result does not contain any credits, default to the
            // service credit.
            if !had_credits {
                let service_credit = self.geocoder_services[service_index].credit();
                self.update_credit(service_credit);
            }
            return;
        }

        self.inner.borrow_mut().search_text = format!("{query} (not found)");
    }

    /// Mirrors `cancelGeocode(viewModel)`.
    fn cancel_geocode(&self) {
        let mut inner = self.inner.borrow_mut();
        if inner.is_search_in_progress {
            inner.is_search_in_progress = false;
            inner.was_geocode_cancelled = true;
        }
    }

    /// Mirrors `clearSuggestions(viewModel)`.
    fn clear_suggestions(&self) {
        self.inner.borrow_mut().suggestions.clear();
    }

    /// Mirrors `updateCredit(viewModel, credit)`.
    fn update_credit(&self, credit: Option<Credit>) {
        if let Some(credit) = credit {
            if !self.scene.is_destroyed() && !self.scene.credit_display_is_destroyed() {
                self.scene.add_static_credit(credit.clone());
                self.inner.borrow_mut().previous_credits.push(credit);
            }
        }
    }

    /// Mirrors `updateCredits(viewModel, credits)`.
    fn update_credits(&self, credits: Option<Vec<Credit>>) {
        if let Some(credits) = credits {
            for credit in credits {
                self.update_credit(Some(credit));
            }
        }
    }

    /// Mirrors `clearCredits(viewModel)`.
    fn clear_credits(&self) {
        if !self.scene.is_destroyed() && !self.scene.credit_display_is_destroyed() {
            let previous = std::mem::take(&mut self.inner.borrow_mut().previous_credits);
            for credit in &previous {
                self.scene.remove_static_credit(credit);
            }
        } else {
            self.inner.borrow_mut().previous_credits.clear();
        }
    }
}

/// Mirrors the module-level `computeFlyToLocationForCartographic`: the
/// cartographic path used for point destinations and zero-area rectangles
/// (adds [`DEFAULT_HEIGHT`], sampling the terrain first when the terrain
/// provider has availability).
///
/// DEVIATION: the JS promise over `sampleTerrainMostDetailed` is
/// synchronous through the [`FlyToRectangleScene::sample_terrain_most_detailed`]
/// seam; a failed sample (`None` height) keeps the input height where the
/// JS would compute with an `undefined` height.
fn compute_fly_to_location_for_cartographic(
    cartographic: &Cartographic,
    scene: &dyn FlyToRectangleScene,
) -> Cartographic {
    if !scene.terrain_availability_defined() {
        let mut result = *cartographic;
        result.height += DEFAULT_HEIGHT;
        return result;
    }

    let mut result = scene
        .sample_terrain_most_detailed(std::slice::from_ref(cartographic))
        .into_iter()
        .next()
        .flatten()
        .map_or(*cartographic, |height| Cartographic {
            height,
            ..*cartographic
        });
    result.height += DEFAULT_HEIGHT;
    result
}

/// Mirrors `Array.prototype.indexOf` over the suggestions, with `None`
/// matching the JS `indexOf(undefined) === -1` behaviour.
fn index_of(suggestions: &[GeocoderResult], selected: &Option<GeocoderResult>) -> usize {
    match selected {
        None => usize::MAX,
        Some(selected) => suggestions
            .iter()
            .position(|suggestion| suggestion == selected)
            .unwrap_or(usize::MAX),
    }
}

/// Mirrors the JS `hasOnlyWhitespace` regex (`/^\s*$/`).
fn has_only_whitespace(value: &str) -> bool {
    value.chars().all(char::is_whitespace)
}

/// Mirrors `GeocoderService.getCreditsFromResult(result)`: maps the
/// result attributions to `Credit(html, false)`, returning `None` when the
/// result carries no attributions.
fn get_credits_from_result(result: &GeocoderResult) -> Option<Vec<Credit>> {
    if result.attributions.is_empty() {
        return None;
    }
    Some(
        result
            .attributions
            .iter()
            .map(|html| Credit::new(html, false))
            .collect(),
    )
}
