//! Ported from `packages/engine/Source/Scene/Model/ModelAnimationCollection.js`.
//!
//! A collection of model animations.

use crate::frame_state::FrameState;
use crate::model_animation_state::ModelAnimationState;

/// A collection of model animations that can be played, paused, and stopped.
///
/// Mirrors CesiumJS `ModelAnimationCollection` (445 lines).
pub struct ModelAnimationCollection {
    /// The animations in this collection.
    animations: Vec<ModelAnimationInfo>,
}

/// Information about a model animation.
pub struct ModelAnimationInfo {
    /// The name of the animation.
    pub name: String,
    /// The duration of the animation in seconds.
    pub duration: f64,
    /// The current state of the animation.
    pub state: ModelAnimationState,
    /// The current time within the animation.
    pub current_time: f64,
    /// The speed multiplier.
    pub speed: f64,
    /// Whether the animation loops.
    pub loops: bool,
    /// Whether the animation is paused.
    pub paused: bool,
    /// Whether to remove the animation when it stops.
    pub remove_on_stop: bool,
}

impl ModelAnimationCollection {
    /// Creates a new empty animation collection.
    pub fn new() -> Self {
        Self { animations: Vec::new() }
    }

    /// Returns the number of animations.
    pub fn length(&self) -> usize {
        self.animations.len()
    }

    /// Gets an animation by index.
    pub fn get(&self, index: usize) -> Option<&ModelAnimationInfo> {
        self.animations.get(index)
    }

    /// Adds an animation to the collection.
    pub fn add(&mut self, animation: ModelAnimationInfo) -> &ModelAnimationInfo {
        self.animations.push(animation);
        self.animations.last().unwrap()
    }

    /// Removes all animations from the collection.
    pub fn remove_all(&mut self) {
        self.animations.clear();
    }

    /// Updates all animations for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        for animation in &mut self.animations {
            if animation.state == ModelAnimationState::Animating && !animation.paused {
                animation.current_time += 1.0 / 60.0 * animation.speed; // DEVIATION: use real delta time
                if animation.current_time >= animation.duration {
                    if animation.loops {
                        animation.current_time %= animation.duration;
                    } else {
                        animation.current_time = animation.duration;
                        animation.state = ModelAnimationState::Stopped;
                    }
                }
            }
        }
    }
}

impl Default for ModelAnimationCollection {
    fn default() -> Self { Self::new() }
}
