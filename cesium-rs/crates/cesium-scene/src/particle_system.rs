//! Ported from `packages/engine/Source/Scene/ParticleSystem.js`.
//!
//! A particle system for visual effects.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;
use cesium_core::matrix4::Matrix4;

use crate::frame_state::FrameState;

/// A particle system for creating visual effects like fire, smoke, or sparks.
///
/// Mirrors CesiumJS `ParticleSystem` (725 lines).
pub struct ParticleSystem {
    /// Whether this system is shown.
    pub show: bool,
    /// The model matrix for the emitter.
    pub model_matrix: Matrix4,
    /// Whether the emitter is looping.
    pub loop_: bool,
    /// The emitter type (box, circle, cone, sphere).
    pub emitter_type: ParticleEmitterType,
    /// The emission rate (particles per second).
    pub emission_rate: f64,
    /// The emitter size/radius.
    pub emitter_size: f64,
    /// The minimum and maximum particle lifetime (seconds).
    pub minimum_particle_lifetime: f64,
    pub maximum_particle_lifetime: f64,
    /// The minimum and maximum particle speed.
    pub minimum_speed: f64,
    pub maximum_speed: f64,
    /// The minimum and maximum particle size (pixels).
    pub start_scale: f64,
    pub end_scale: f64,
    /// The particle color.
    pub start_color: Color,
    pub end_color: Color,
    /// The particle image URI.
    pub image: Option<String>,
    /// Whether the system is currently active.
    is_complete: bool,
    /// Whether this system has been destroyed.
    is_destroyed: bool,
    /// Current number of live particles.
    particle_count: i32,
}

/// The type of particle emitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleEmitterType {
    /// A box-shaped emitter.
    Box,
    /// A circle-shaped emitter.
    Circle,
    /// A cone-shaped emitter.
    Cone,
    /// A sphere-shaped emitter.
    Sphere,
}

impl ParticleSystem {
    /// Creates a new ParticleSystem.
    pub fn new() -> Self {
        Self {
            show: true,
            model_matrix: Matrix4::IDENTITY,
            loop_: true,
            emitter_type: ParticleEmitterType::Cone,
            emission_rate: 5.0,
            emitter_size: 1.0,
            minimum_particle_lifetime: 3.0,
            maximum_particle_lifetime: 5.0,
            minimum_speed: 1.0,
            maximum_speed: 3.0,
            start_scale: 1.0,
            end_scale: 3.0,
            start_color: Color::new(1.0, 1.0, 1.0, 1.0),
            end_color: Color::new(1.0, 1.0, 1.0, 0.0),
            image: None,
            is_complete: false,
            is_destroyed: false,
            particle_count: 0,
        }
    }

    /// Updates the particle system for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        // DEVIATION: Requires particle simulation and GPU buffer management
    }

    /// Returns whether the system is complete (non-looping and all particles dead).
    pub fn is_complete(&self) -> bool {
        self.is_complete
    }

    /// Returns the current number of live particles.
    pub fn particle_count(&self) -> i32 {
        self.particle_count
    }

    /// Returns whether this system has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this system.
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}

impl Default for ParticleSystem {
    fn default() -> Self { Self::new() }
}
