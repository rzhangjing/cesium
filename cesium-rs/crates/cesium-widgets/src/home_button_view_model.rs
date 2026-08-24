//! Ported from `packages/widgets/Source/HomeButton/HomeButtonViewModel.js`.
//!
//! The view model for the HomeButton widget.
//!
//! DEVIATION: the JS view model calls `scene.camera.flyHome(duration)` on
//! a concrete `Scene`. The cesium-widgets port is GPU-free, so the home
//! camera behavior is injected through the [`HomeCamera`] trait; the
//! engine wiring is provided by `impl HomeCamera for
//! cesium_scene::scene::Scene` below (delegating to
//! [`cesium_scene::scene::Scene::fly_home`]). See `docs/deviations.md`.

use std::cell::RefCell;
use std::rc::Rc;

use cesium_core::developer_error::throw_developer_error;
use cesium_scene::scene::Scene;

use crate::command::Command;
use crate::create_command::create_command;

/// The camera behavior invoked when the home button command executes.
///
/// DEVIATION: the Rust analogue of `scene.camera.flyHome(duration)`.
pub trait HomeCamera {
    /// Flies the camera to the home position, mirroring
    /// `Camera.flyHome(duration)`.
    fn fly_home(&self, duration: Option<f64>);
}

/// The view model for the HomeButton widget.
pub struct HomeButtonViewModel {
    camera: Rc<dyn HomeCamera>,
    command: Command,
    tooltip: String,
    /// Shared with the command closure so execution always observes the
    /// latest duration (the Rust analogue of `that._duration`).
    duration: Rc<RefCell<Option<f64>>>,
}

impl HomeButtonViewModel {
    /// Creates a new home button view model.
    ///
    /// Mirrors `new HomeButtonViewModel(scene, duration)`; the JS
    /// `scene is required.` DeveloperError is mirrored by
    /// [`HomeButtonViewModel::try_new`] with `None`.
    pub fn new(camera: Rc<dyn HomeCamera>, duration: Option<f64>) -> Self {
        Self::try_new(Some(camera), duration)
    }

    /// Creates a new home button view model from an optional camera,
    /// mirroring the JS undefined-scene DeveloperError check.
    ///
    /// # Panics
    /// Panics with a `DeveloperError` when `camera` is `None`.
    pub fn try_new(camera: Option<Rc<dyn HomeCamera>>, duration: Option<f64>) -> Self {
        #[cfg(debug_assertions)]
        if camera.is_none() {
            throw_developer_error("scene is required.");
        }
        let camera = camera.expect("scene is required.");

        // DEVIATION: the JS command body is `scene.camera.flyHome(this._duration)`;
        // the Rust port invokes the injected [`HomeCamera`]. The current
        // duration is captured in shared interior-mutable state so the
        // command always observes the latest value, matching the JS
        // `that._duration` read-at-call semantics.
        let current_duration = Rc::new(RefCell::new(duration));
        let command_duration = Rc::clone(&current_duration);
        let command_camera = Rc::clone(&camera);
        let command = create_command(
            move |_| {
                command_camera.fly_home(*command_duration.borrow());
                None
            },
            None,
        );

        Self {
            camera,
            command,
            tooltip: "View Home".to_string(),
            duration: current_duration,
        }
    }

    /// Gets the camera to control.
    ///
    /// DEVIATION: mirrors the JS `scene` getter; returns the injected
    /// [`HomeCamera`] handle instead of a `Scene`.
    pub fn camera(&self) -> &Rc<dyn HomeCamera> {
        &self.camera
    }

    /// Gets the Command that is executed when the button is clicked.
    pub fn command(&self) -> &Command {
        &self.command
    }

    /// Gets the tooltip.
    pub fn tooltip(&self) -> &str {
        &self.tooltip
    }

    /// Sets the tooltip.
    pub fn set_tooltip(&mut self, tooltip: &str) {
        self.tooltip = tooltip.to_string();
    }

    /// Gets the duration of the camera flight in seconds. A value of zero
    /// causes the camera to instantly switch to home view. The duration
    /// will be computed based on the distance when `None`.
    pub fn duration(&self) -> Option<f64> {
        *self.duration.borrow()
    }

    /// Sets the duration of the camera flight in seconds.
    ///
    /// # Panics
    /// Panics with a `DeveloperError` when the value is negative,
    /// mirroring the JS `value must be positive.` check.
    pub fn set_duration(&mut self, value: Option<f64>) {
        #[cfg(debug_assertions)]
        if let Some(value) = value {
            if value < 0.0 {
                throw_developer_error("value must be positive.");
            }
        }
        *self.duration.borrow_mut() = value;
    }
}

/// The engine wiring of [`HomeCamera`]: the home command drives the real
/// scene camera through [`Scene::fly_home`] (the JS
/// `scene.camera.flyHome(duration)` goes through the scene because the
/// widget holds a shared scene handle).
impl HomeCamera for Scene {
    fn fly_home(&self, duration: Option<f64>) {
        Scene::fly_home(self, duration);
    }
}
