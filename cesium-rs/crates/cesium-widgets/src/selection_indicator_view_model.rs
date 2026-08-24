//! Ported from `packages/widgets/Source/SelectionIndicator/SelectionIndicatorViewModel.js`.
//!
//! The view model for the SelectionIndicator widget.
//!
//! DEVIATION: the JS view model operates on a concrete `Scene`; the
//! widgets layer is GPU-free, so the scene is injected through the
//! [`SelectionScene`] trait. The engine wiring is provided by
//! `impl SelectionScene for cesium_scene::scene::Scene` below:
//! `worldToWindowCoordinates` delegates to
//! [`cesium_scene::scene::Scene::world_to_window_coordinates`] and the
//! `animateAppear`/`animateDepart` `_scale` tweens are added through
//! `scene.tweens` ([`cesium_scene::scene::Scene::tweens_mut`]).
//! See `docs/deviations.md`.

use std::cell::Cell;
use std::rc::Rc;

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::developer_error::throw_developer_error;
use cesium_core::easing_function::exponential_out;
use cesium_scene::scene::Scene;
use cesium_scene::tween_collection::TweenOptions;

use crate::knockout::MockDomElement;

/// Off-screen position used when the indicator is hidden or the position
/// cannot be projected (mirrors the module-level `offScreen` constant).
const OFF_SCREEN: &str = "-1000px";

/// The Rust analogue of the scene handle held by the view model
/// (`scene.tweens` for the `_scale` animations and the default
/// screen-space projection).
pub trait SelectionScene {
    /// Mirrors `SceneTransforms.worldToWindowCoordinates(scene, position)`
    /// (the default `computeScreenSpacePosition`). Returns `None` when
    /// the position cannot be projected (the JS `result` left undefined).
    fn world_to_window_coordinates(&self, position: &Cartesian3) -> Option<Cartesian2>;
    /// Mirrors `scene.tweens.add(...)` (the JS `animateAppear`/
    /// `animateDepart` go through `scene.tweens.addProperty`).
    fn add_tween(&self, options: TweenOptions) -> u64;
}

/// The engine wiring of [`SelectionScene`] on the real scene.
impl SelectionScene for Scene {
    fn world_to_window_coordinates(&self, position: &Cartesian3) -> Option<Cartesian2> {
        Scene::world_to_window_coordinates(self, position)
    }

    fn add_tween(&self, options: TweenOptions) -> u64 {
        self.tweens_mut().add(options)
    }
}

/// A function that converts the world position of an object to a screen
/// space position (`SelectionIndicatorViewModel.ComputeScreenSpacePosition`).
/// Returns `None` when the position cannot be projected (the JS `result`
/// parameter left undefined).
pub type ComputeScreenSpacePosition = Rc<dyn Fn(&Cartesian3) -> Option<Cartesian2>>;

/// The view model for the SelectionIndicator widget.
///
/// Shows a visual indicator at the position of the selected entity.
pub struct SelectionIndicatorViewModel {
    scene: Rc<dyn SelectionScene>,
    selection_indicator_element: MockDomElement,
    container: MockDomElement,
    screen_position_x: String,
    screen_position_y: String,
    /// `_scale`; shared with the tween update callbacks (the JS tweens
    /// write `this._scale` through the captured view model reference).
    scale: Rc<Cell<f64>>,
    /// The world position of the object for which to display the
    /// selection indicator (`position` observable).
    position: Option<Cartesian3>,
    /// The visibility of the selection indicator (`showSelection`
    /// observable).
    show_selection: bool,
    compute_screen_space_position: ComputeScreenSpacePosition,
}

impl SelectionIndicatorViewModel {
    /// Creates a new selection indicator view model.
    ///
    /// Mirrors `new SelectionIndicatorViewModel(scene,
    /// selectionIndicatorElement, container)`; the JS DeveloperErrors for
    /// undefined arguments are mirrored by
    /// [`SelectionIndicatorViewModel::try_new`] with `None`.
    pub fn new(
        scene: Rc<dyn SelectionScene>,
        selection_indicator_element: MockDomElement,
        container: MockDomElement,
    ) -> Self {
        Self::try_new(
            Some(scene),
            Some(selection_indicator_element),
            Some(container),
        )
    }

    /// Creates a new selection indicator view model from optional
    /// arguments, mirroring the JS undefined-argument DeveloperError
    /// checks.
    ///
    /// # Panics
    /// Panics with a `DeveloperError` when `scene`,
    /// `selection_indicator_element` or `container` is `None`.
    pub fn try_new(
        scene: Option<Rc<dyn SelectionScene>>,
        selection_indicator_element: Option<MockDomElement>,
        container: Option<MockDomElement>,
    ) -> Self {
        #[cfg(debug_assertions)]
        {
            if scene.is_none() {
                throw_developer_error("scene is required.");
            }
            if selection_indicator_element.is_none() {
                throw_developer_error("selectionIndicatorElement is required.");
            }
            if container.is_none() {
                throw_developer_error("container is required.");
            }
        }
        let scene = scene.expect("scene is required.");
        let selection_indicator_element =
            selection_indicator_element.expect("selectionIndicatorElement is required.");
        let container = container.expect("container is required.");

        // this.computeScreenSpacePosition = function (position, result) {
        //   return SceneTransforms.worldToWindowCoordinates(scene, position, result);
        // };
        let scene_for_default = Rc::clone(&scene);

        Self {
            scene,
            selection_indicator_element,
            container,
            screen_position_x: OFF_SCREEN.to_string(),
            screen_position_y: OFF_SCREEN.to_string(),
            scale: Rc::new(Cell::new(1.0)),
            position: None,
            show_selection: false,
            compute_screen_space_position: Rc::new(move |position: &Cartesian3| {
                scene_for_default.world_to_window_coordinates(position)
            }),
        }
    }

    /// Updates the view of the selection indicator to match the position
    /// and content properties of the view model. This function should be
    /// called as part of the render loop
    /// (`SelectionIndicatorViewModel.prototype.update`).
    pub fn update(&mut self) {
        if self.show_selection && self.position.is_some() {
            let position = self.position.as_ref().expect("position checked above");
            let screen_position = (self.compute_screen_space_position)(position);
            match screen_position {
                None => {
                    self.screen_position_x = OFF_SCREEN.to_string();
                    self.screen_position_y = OFF_SCREEN.to_string();
                }
                Some(mut screen_position) => {
                    let container_width = self.container.parent_client_width;
                    let container_height = self.container.parent_client_height;
                    let indicator_size = self.selection_indicator_element.client_width;
                    let half_size = indicator_size as f64 * 0.5;

                    screen_position.x = screen_position
                        .x
                        .clamp(-(indicator_size as f64), container_width as f64 + indicator_size as f64)
                        - half_size;
                    screen_position.y = screen_position
                        .y
                        .clamp(-(indicator_size as f64), container_height as f64 + indicator_size as f64)
                        - half_size;

                    self.screen_position_x =
                        format!("{}px", (screen_position.x + 0.25).floor());
                    self.screen_position_y =
                        format!("{}px", (screen_position.y + 0.25).floor());
                }
            }
        }
    }

    /// Animate the indicator to draw attention to the selection.
    ///
    /// Mirrors `animateAppear`: adds a `_scale` tween (2 → 1, 0.8s,
    /// `EasingFunction.EXPONENTIAL_OUT`) through `scene.tweens`
    /// ([`SelectionScene::add_tween`]).
    pub fn animate_appear(&self) {
        self.add_scale_tween(2.0, 1.0);
    }

    /// Animate the indicator to release the selection.
    ///
    /// Mirrors `animateDepart`: adds a `_scale` tween (current → 1.5,
    /// 0.8s, `EasingFunction.EXPONENTIAL_OUT`) through `scene.tweens`.
    pub fn animate_depart(&self) {
        let start = self.scale.get();
        self.add_scale_tween(start, 1.5);
    }

    /// The shared `_scale` tween path (the Rust analogue of
    /// `this._tweens.addProperty({ object: this, property: "_scale", ... })`).
    fn add_scale_tween(&self, start_value: f64, stop_value: f64) {
        let scale = Rc::clone(&self.scale);
        let mut options = TweenOptions::new(
            vec![("_scale".to_string(), start_value)],
            vec![("_scale".to_string(), stop_value)],
            0.8,
        );
        options.easing_function = exponential_out;
        options.update = Some(Box::new(move |values| {
            scale.set(values[0].1);
        }));
        self.scene.add_tween(options);
    }

    /// Gets the world position of the object for which to display the
    /// selection indicator.
    pub fn position(&self) -> Option<&Cartesian3> {
        self.position.as_ref()
    }

    /// Sets the world position of the object for which to display the
    /// selection indicator.
    pub fn set_position(&mut self, position: Option<Cartesian3>) {
        self.position = position;
    }

    /// Gets the visibility of the selection indicator.
    pub fn show_selection(&self) -> bool {
        self.show_selection
    }

    /// Sets the visibility of the selection indicator.
    pub fn set_show_selection(&mut self, value: bool) {
        self.show_selection = value;
    }

    /// Gets the visibility of the position indicator (`isVisible`
    /// computed). This can be false even if an object is selected, when
    /// the selected object has no position.
    pub fn is_visible(&self) -> bool {
        self.show_selection && self.position.is_some()
    }

    /// Gets the CSS transform applied to the indicator (`_transform`
    /// computed).
    pub fn transform(&self) -> String {
        format!("scale({})", self.scale.get())
    }

    /// Gets the current indicator scale (`_scale`).
    pub fn scale(&self) -> f64 {
        self.scale.get()
    }

    /// Sets the current indicator scale (`_scale`).
    pub fn set_scale(&mut self, value: f64) {
        self.scale.set(value);
    }

    /// Gets the computed horizontal screen position of the indicator
    /// (`_screenPositionX`).
    pub fn screen_position_x(&self) -> &str {
        &self.screen_position_x
    }

    /// Gets the computed vertical screen position of the indicator
    /// (`_screenPositionY`).
    pub fn screen_position_y(&self) -> &str {
        &self.screen_position_y
    }

    /// Gets the function for converting the world position of the object
    /// to the screen space position.
    pub fn compute_screen_space_position(&self) -> &ComputeScreenSpacePosition {
        &self.compute_screen_space_position
    }

    /// Sets the function for converting the world position of the object
    /// to the screen space position.
    ///
    /// DEVIATION: the JS `computeScreenSpacePosition must be a function`
    /// check is enforced by the type system.
    pub fn set_compute_screen_space_position(&mut self, value: ComputeScreenSpacePosition) {
        self.compute_screen_space_position = value;
    }

    /// Gets the HTML element that contains the widget (mirrors the
    /// read-only `container` property).
    pub fn container(&self) -> &MockDomElement {
        &self.container
    }

    /// Gets the HTML element that holds the selection indicator (mirrors
    /// the read-only `selectionIndicatorElement` property).
    pub fn selection_indicator_element(&self) -> &MockDomElement {
        &self.selection_indicator_element
    }

    /// Gets the scene being used (mirrors the read-only `scene`
    /// property).
    pub fn scene(&self) -> &Rc<dyn SelectionScene> {
        &self.scene
    }
}
