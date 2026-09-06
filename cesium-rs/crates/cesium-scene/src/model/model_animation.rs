//! Ported from `packages/engine/Source/Scene/Model/ModelAnimation.js`.
//!
//! An active animation derived from a glTF asset.

use cesium_core::event::Event;
use cesium_core::julian_date::JulianDate;

use super::model_animation_channel::ModelAnimationChannel;
use crate::model_animation_loop::ModelAnimationLoop;
use crate::model_animation_state::ModelAnimationState;

/// An active animation from a glTF asset.
///
/// An active animation is an instance of an animation; for example, there can
/// be multiple active animations for the same glTF animation, each with a
/// different start time.
///
/// Mirrors CesiumJS `ModelAnimation` (422 lines).
pub struct ModelAnimation {
    /// The name that identifies this animation in the model, if it exists.
    pub name: String,
    /// When `true`, the animation is removed after it stops playing.
    pub remove_on_stop: bool,
    /// Speed multiplier relative to scene clock (1.0 = normal speed).
    pub multiplier: f64,
    /// When `true`, the animation plays in reverse.
    pub reverse: bool,
    /// Determines if and how the animation is looped.
    pub loop_mode: ModelAnimationLoop,

    // -- Timing --
    /// The scene time to start playing this animation.
    pub start_time: Option<JulianDate>,
    /// The delay in seconds from `start_time` to start playing.
    pub delay: f64,
    /// The scene time to stop playing this animation.
    pub stop_time: Option<JulianDate>,

    // -- Local animation time range --
    /// The minimum time value across all keyframes of this animation.
    pub local_start_time: f64,
    /// The maximum time value across all keyframes of this animation.
    pub local_stop_time: f64,

    // -- State --
    /// The current state of this animation.
    pub state: ModelAnimationState,

    // -- Runtime channels --
    /// The runtime animation channels for this animation.
    pub runtime_channels: Vec<ModelAnimationChannel>,

    // -- Events --
    /// Event fired when this animation starts.
    pub start_event: Event<()>,
    /// Event fired on each frame when this animation is updated.
    pub update_event: Event<()>,
    /// Event fired when this animation stops.
    pub stop_event: Event<()>,
}

impl ModelAnimation {
    /// Creates a new `ModelAnimation`.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            remove_on_stop: false,
            multiplier: 1.0,
            reverse: false,
            loop_mode: ModelAnimationLoop::None,
            start_time: None,
            delay: 0.0,
            stop_time: None,
            local_start_time: f64::MAX,
            local_stop_time: f64::MIN,
            state: ModelAnimationState::Stopped,
            runtime_channels: Vec::new(),
            start_event: Event::new(),
            update_event: Event::new(),
            stop_event: Event::new(),
        }
    }

    /// Returns the duration of this animation in local time.
    pub fn duration(&self) -> f64 {
        if self.local_stop_time > self.local_start_time {
            self.local_stop_time - self.local_start_time
        } else {
            0.0
        }
    }

    /// Returns whether this animation is currently playing.
    pub fn is_playing(&self) -> bool {
        self.state == ModelAnimationState::Animating
            || self.state == ModelAnimationState::Starting
    }

    /// Updates the local time range from a channel's sampler times.
    ///
    /// Call this for each channel during initialization to compute
    /// `local_start_time` and `local_stop_time`.
    pub fn update_time_range(&mut self, times: &[f64]) {
        if let Some(&first) = times.first() {
            if first < self.local_start_time {
                self.local_start_time = first;
            }
        }
        if let Some(&last) = times.last() {
            if last > self.local_stop_time {
                self.local_stop_time = last;
            }
        }
    }
}

impl Default for ModelAnimation {
    fn default() -> Self {
        Self::new("")
    }
}
