//! Ported from `packages/widgets/Source/PerformanceWatchdog/PerformanceWatchdogViewModel.js`.
//!
//! The view model for the `PerformanceWatchdog` widget: monitors scene
//! performance and shows a dismissible message when the frame rate drops
//! below a threshold.
//!
//! DEVIATION: the JS view model obtains its `lowFrameRate` /
//! `nominalFrameRate` events via `FrameRateMonitor.fromScene(scene)`; the
//! frame-rate sampling itself lives in the render loop (engine side). The
//! widgets layer is GPU-free, so the events are injected through the
//! [`WatchdogScene`] trait (the same dependency-injection style used by
//! the other widget view models); the view model behavior in response to
//! the events is mirrored one-to-one.

use std::cell::Cell;
use std::rc::Rc;

use cesium_core::event::{Event, RemoveCallback};

use crate::command::Command;
use crate::observables::ObservableCell;

/// The default message displayed when a low frame rate is detected
/// (`options.lowFrameRateMessage` default in the JS constructor).
pub const DEFAULT_LOW_FRAME_RATE_MESSAGE: &str = "This application appears to be performing poorly on your system.  Please try using a different web browser or updating your video drivers.";

/// The scene abstraction required by [`PerformanceWatchdogViewModel`],
/// mirroring the `FrameRateMonitor.fromScene(scene)` events the JS view
/// model subscribes to.
///
/// DEVIATION: JS obtains these events from `FrameRateMonitor`; the
/// monitor's timing machinery (quiet/warmup/sampling periods) belongs to
/// the render loop and is injected here as plain events.
pub trait WatchdogScene {
    /// Raised when a low frame rate is detected
    /// (`monitor.lowFrameRate`).
    fn low_frame_rate(&self) -> &Event<()>;
    /// Raised when the frame rate returns to nominal
    /// (`monitor.nominalFrameRate`).
    fn nominal_frame_rate(&self) -> &Event<()>;
}

/// Options for [`PerformanceWatchdogViewModel::new`], mirroring the JS
/// `options` object (minus `scene`, which is a required positional
/// argument in the Rust port).
pub struct PerformanceWatchdogViewModelOptions {
    /// The message to display when a low frame rate is detected (HTML).
    pub low_frame_rate_message: Option<String>,
}

/// The view model for the `PerformanceWatchdog` widget.
pub struct PerformanceWatchdogViewModel {
    scene: Rc<dyn WatchdogScene>,
    /// Gets or sets the message to display when a low frame rate is
    /// detected (knockout observable in JS).
    low_frame_rate_message: ObservableCell<String>,
    /// Gets or sets whether the low frame rate message has previously
    /// been dismissed by the user (knockout observable in JS).
    low_frame_rate_message_dismissed: ObservableCell<bool>,
    /// Gets or sets whether the low frame rate message is currently
    /// being displayed (knockout observable in JS).
    showing_low_frame_rate_message: ObservableCell<bool>,
    /// The command that dismisses the low frame rate message.
    dismiss_message: Command,
    /// Removal callback for the `lowFrameRate` subscription.
    unsubscribe_low_frame_rate: Option<RemoveCallback<()>>,
    /// Removal callback for the `nominalFrameRate` subscription.
    unsubscribe_nominal_frame_rate: Option<RemoveCallback<()>>,
    /// Whether [`PerformanceWatchdogViewModel::destroy`] has been called.
    destroyed: Cell<bool>,
}

impl PerformanceWatchdogViewModel {
    /// Creates a new view model monitoring the given scene.
    ///
    /// Mirrors `new PerformanceWatchdogViewModel(options)`; the JS
    /// `options.scene is required.` DeveloperError is enforced by the
    /// Rust type system (the scene argument is mandatory).
    pub fn new(
        scene: Rc<dyn WatchdogScene>,
        options: Option<PerformanceWatchdogViewModelOptions>,
    ) -> Self {
        let low_frame_rate_message = options
            .and_then(|o| o.low_frame_rate_message)
            .unwrap_or_else(|| DEFAULT_LOW_FRAME_RATE_MESSAGE.to_owned());

        let low_frame_rate_message = ObservableCell::new(low_frame_rate_message);
        let low_frame_rate_message_dismissed = ObservableCell::new(false);
        let showing_low_frame_rate_message = ObservableCell::new(false);

        // this._dismissMessage = createCommand(function () { ... });
        let showing_for_dismiss = showing_low_frame_rate_message.clone();
        let dismissed_for_dismiss = low_frame_rate_message_dismissed.clone();
        let dismiss_message = Command::new(
            move |_| {
                showing_for_dismiss.set(false);
                dismissed_for_dismiss.set(true);
                None
            },
            true,
        );

        // this._unsubscribeLowFrameRate = monitor.lowFrameRate.addEventListener(...)
        let showing_for_low = showing_low_frame_rate_message.clone();
        let dismissed_for_low = low_frame_rate_message_dismissed.clone();
        let unsubscribe_low_frame_rate = scene.low_frame_rate().add_listener(move |_| {
            if !dismissed_for_low.get() {
                showing_for_low.set(true);
            }
        });

        // this._unsubscribeNominalFrameRate = monitor.nominalFrameRate.addEventListener(...)
        let showing_for_nominal = showing_low_frame_rate_message.clone();
        let unsubscribe_nominal_frame_rate = scene.nominal_frame_rate().add_listener(move |_| {
            showing_for_nominal.set(false);
        });

        Self {
            scene,
            low_frame_rate_message,
            low_frame_rate_message_dismissed,
            showing_low_frame_rate_message,
            dismiss_message,
            unsubscribe_low_frame_rate: Some(unsubscribe_low_frame_rate),
            unsubscribe_nominal_frame_rate: Some(unsubscribe_nominal_frame_rate),
            destroyed: Cell::new(false),
        }
    }

    /// Gets the scene instance for which to monitor performance.
    pub fn scene(&self) -> &Rc<dyn WatchdogScene> {
        &self.scene
    }

    /// Gets the low frame rate message.
    pub fn low_frame_rate_message(&self) -> String {
        self.low_frame_rate_message.get()
    }

    /// Sets the low frame rate message.
    pub fn set_low_frame_rate_message(&self, value: String) {
        self.low_frame_rate_message.set(value);
    }

    /// Gets whether the low frame rate message has been dismissed.
    pub fn low_frame_rate_message_dismissed(&self) -> bool {
        self.low_frame_rate_message_dismissed.get()
    }

    /// Sets whether the low frame rate message has been dismissed.
    pub fn set_low_frame_rate_message_dismissed(&self, value: bool) {
        self.low_frame_rate_message_dismissed.set(value);
    }

    /// Gets whether the low frame rate message is currently displayed.
    pub fn showing_low_frame_rate_message(&self) -> bool {
        self.showing_low_frame_rate_message.get()
    }

    /// Sets whether the low frame rate message is currently displayed.
    pub fn set_showing_low_frame_rate_message(&self, value: bool) {
        self.showing_low_frame_rate_message.set(value);
    }

    /// Gets a command that dismisses the low frame rate message. Once it
    /// is dismissed, the message will not be redisplayed.
    pub fn dismiss_message(&self) -> &Command {
        &self.dismiss_message
    }

    /// Returns whether this view model has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.destroyed.get()
    }

    /// Destroys the view model, unsubscribing from the frame rate
    /// events.
    pub fn destroy(&mut self) {
        if let Some(removal) = self.unsubscribe_low_frame_rate.take() {
            removal.call(self.scene.low_frame_rate());
        }
        if let Some(removal) = self.unsubscribe_nominal_frame_rate.take() {
            removal.call(self.scene.nominal_frame_rate());
        }
        self.destroyed.set(true);
    }
}
