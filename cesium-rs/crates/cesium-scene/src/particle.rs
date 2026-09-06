//! Ported from `packages/engine/Source/Scene/Particle.js`.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;

/// A particle emitted by a `ParticleSystem`.
pub struct Particle {
    /// The mass of the particle in kilograms.
    pub mass: f64,
    /// The position of the particle in world coordinates.
    pub position: Cartesian3,
    /// The velocity of the particle in world coordinates.
    pub velocity: Cartesian3,
    /// The life of the particle in seconds.
    pub life: f64,
    /// The color of a particle when it is born.
    pub start_color: Color,
    /// The color of a particle when it dies.
    pub end_color: Color,
    /// The scale of the particle when it is born.
    pub start_scale: f64,
    /// The scale of the particle when it dies.
    pub end_scale: f64,
    /// The dimensions, width by height, to scale the particle image in pixels.
    pub image_size: Cartesian2,
    age: f64,
    normalized_age: f64,
}

impl Particle {
    /// Creates a new particle with default values.
    pub fn new() -> Self {
        Self {
            mass: 1.0,
            position: Cartesian3::ZERO,
            velocity: Cartesian3::ZERO,
            life: f64::MAX,
            start_color: Color::WHITE,
            end_color: Color::WHITE,
            start_scale: 1.0,
            end_scale: 1.0,
            image_size: Cartesian2::new(1.0, 1.0),
            age: 0.0,
            normalized_age: 0.0,
        }
    }

    /// Gets the age of the particle in seconds.
    pub fn age(&self) -> f64 {
        self.age
    }

    /// Gets the age normalized to a value in the range `[0.0, 1.0]`.
    pub fn normalized_age(&self) -> f64 {
        self.normalized_age
    }

    /// Advances the particle by `dt` seconds.
    ///
    /// Returns `true` if the particle is still alive, `false` if it has
    /// exceeded its lifespan.
    pub fn update(&mut self, dt: f64) -> bool {
        // Apply velocity: position += velocity * dt
        let delta = Cartesian3::multiply_by_scalar_new(&self.velocity, dt);
        self.position = Cartesian3::add_new(&self.position, &delta);

        // Age the particle
        self.age += dt;

        // Compute normalized age
        if self.life == f64::MAX {
            self.normalized_age = 0.0;
        } else {
            self.normalized_age = self.age / self.life;
        }

        self.age <= self.life
    }
}

impl Default for Particle {
    fn default() -> Self {
        Self::new()
    }
}
