//! Particle system for visual effects (fire, smoke, snow, etc.).
//!
//! Maps to CesiumJS `Scene/ParticleSystem.js`:
//! - Particle emitters (point, cone, box, sphere)
//! - Particle lifecycle (birth, update, death)
//! - Particle forces (gravity, drag, wind)

use glam::DVec3;

/// Particle emitter shape.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum EmitterShape {
    /// Emits from a single point.
    #[default]
    Point,
    /// Emits from a cone shape.
    Cone {
        /// Cone angle in radians.
        angle: f64,
    },
    /// Emits from a box volume.
    Box {
        /// Half-extents of the box.
        half_extents: DVec3,
    },
    /// Emits from a sphere surface.
    Sphere {
        /// Sphere radius.
        radius: f64,
    },
    /// Emits from a circle (disc).
    Circle {
        /// Circle radius.
        radius: f64,
    },
}


/// A single particle in the system.
#[derive(Debug, Clone, PartialEq)]
pub struct Particle {
    /// Unique particle ID.
    pub id: u64,
    /// Current position (world space).
    pub position: DVec3,
    /// Current velocity.
    pub velocity: DVec3,
    /// Current color (RGBA, 0-1).
    pub color: [f64; 4],
    /// Current size (pixels or world units depending on config).
    pub size: f64,
    /// Age in seconds.
    pub age: f64,
    /// Maximum lifetime in seconds.
    pub lifetime: f64,
    /// Mass in kilograms.
    pub mass: f64,
    /// Start scale.
    pub start_scale: f64,
    /// End scale.
    pub end_scale: f64,
    /// Image size [width, height] in pixels.
    pub image_size: [f64; 2],
    /// Whether the particle is alive.
    pub alive: bool,
}

impl Particle {
    /// Creates a new particle.
    pub fn new(id: u64, position: DVec3, velocity: DVec3, lifetime: f64) -> Self {
        Self {
            id,
            position,
            velocity,
            color: [1.0, 1.0, 1.0, 1.0],
            size: 1.0,
            age: 0.0,
            lifetime,
            mass: 1.0,
            start_scale: 1.0,
            end_scale: 1.0,
            image_size: [1.0, 1.0],
            alive: true,
        }
    }

    /// Returns the normalized age (0.0 to 1.0).
    pub fn normalized_age(&self) -> f64 {
        (self.age / self.lifetime).clamp(0.0, 1.0)
    }

    /// Returns the interpolated scale at current age.
    pub fn current_scale(&self) -> f64 {
        let t = self.normalized_age();
        self.start_scale + (self.end_scale - self.start_scale) * t
    }

    /// Updates the particle by a time delta.
    pub fn update(&mut self, dt: f64, forces: &[ParticleForce]) {
        if !self.alive {
            return;
        }

        self.age += dt;

        if self.age >= self.lifetime {
            self.alive = false;
            return;
        }

        // Apply forces
        let mut acceleration = DVec3::ZERO;
        for force in forces {
            acceleration += force.compute_acceleration(self);
        }

        // Integrate velocity and position
        self.velocity += acceleration * dt;
        self.position += self.velocity * dt;
    }
}

/// A force that affects particles.
#[derive(Debug, Clone, PartialEq)]
pub enum ParticleForce {
    /// Constant gravity acceleration.
    Gravity {
        /// Gravity vector (e.g., (0, -9.81, 0)).
        acceleration: DVec3,
    },
    /// Linear drag (air resistance).
    Drag {
        /// Drag coefficient.
        coefficient: f64,
    },
    /// Wind force.
    Wind {
        /// Wind direction and speed.
        velocity: DVec3,
    },
    /// Attractor point (pulls particles towards a point).
    Attractor {
        /// Attractor position.
        position: DVec3,
        /// Attraction strength.
        strength: f64,
    },
    /// Vortex force (spiral motion).
    Vortex {
        /// Vortex axis.
        axis: DVec3,
        /// Vortex strength.
        strength: f64,
    },
}

impl ParticleForce {
    /// Computes the acceleration this force applies to a particle.
    pub fn compute_acceleration(&self, particle: &Particle) -> DVec3 {
        match self {
            Self::Gravity { acceleration } => *acceleration,
            Self::Drag { coefficient } => -particle.velocity * *coefficient,
            Self::Wind { velocity } => (*velocity - particle.velocity) * 0.1,
            Self::Attractor { position, strength } => {
                let to_attractor = *position - particle.position;
                let distance = to_attractor.length().max(0.001);
                to_attractor.normalize() * (*strength / (distance * distance))
            }
            Self::Vortex { axis, strength } => {
                let radial = particle.position.cross(*axis);
                radial.normalize() * *strength
            }
        }
    }
}

/// A burst of particles at a specific time in the system's lifetime.
///
/// Maps to CesiumJS `Scene/ParticleBurst.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct ParticleBurst {
    /// Time in seconds after system start when the burst occurs.
    pub time: f64,
    /// Minimum number of particles emitted in the burst.
    pub minimum: u32,
    /// Maximum number of particles emitted in the burst.
    pub maximum: u32,
    /// Whether this burst has already fired.
    pub complete: bool,
}

impl ParticleBurst {
    /// Creates a new particle burst.
    pub fn new(time: f64, minimum: u32, maximum: u32) -> Self {
        Self {
            time,
            minimum,
            maximum,
            complete: false,
        }
    }

    /// Resets the burst for looping.
    pub fn reset(&mut self) {
        self.complete = false;
    }
}

/// Particle system configuration.
#[derive(Debug, Clone)]
pub struct ParticleSystemConfig {
    /// Emitter shape.
    pub emitter_shape: EmitterShape,
    /// Emission rate (particles per second).
    pub emission_rate: f64,
    /// Minimum particle lifetime (seconds).
    pub min_lifetime: f64,
    /// Maximum particle lifetime (seconds).
    pub max_lifetime: f64,
    /// Minimum initial speed.
    pub min_speed: f64,
    /// Maximum initial speed.
    pub max_speed: f64,
    /// Minimum particle size.
    pub min_size: f64,
    /// Maximum particle size.
    pub max_size: f64,
    /// Start color (RGBA).
    pub start_color: [f64; 4],
    /// End color (RGBA).
    pub end_color: [f64; 4],
    /// Maximum number of particles.
    pub max_particles: usize,
    /// Whether the system loops.
    pub looping: bool,
    /// Forces applied to particles.
    pub forces: Vec<ParticleForce>,
    /// Bursts of particles at specific times.
    pub bursts: Vec<ParticleBurst>,
    /// System lifetime in seconds (f64::MAX = infinite).
    pub system_lifetime: f64,
    /// Minimum particle mass in kilograms.
    pub min_mass: f64,
    /// Maximum particle mass in kilograms.
    pub max_mass: f64,
    /// Start scale for particles.
    pub start_scale: f64,
    /// End scale for particles.
    pub end_scale: f64,
    /// Minimum image size [width, height] in pixels.
    pub min_image_size: [f64; 2],
    /// Maximum image size [width, height] in pixels.
    pub max_image_size: [f64; 2],
    /// Whether particle size is in meters (true) or pixels (false).
    pub size_in_meters: bool,
    /// Image URI for particle billboard.
    pub image: Option<String>,
}

impl Default for ParticleSystemConfig {
    fn default() -> Self {
        Self {
            emitter_shape: EmitterShape::Point,
            emission_rate: 10.0,
            min_lifetime: 1.0,
            max_lifetime: 3.0,
            min_speed: 1.0,
            max_speed: 5.0,
            min_size: 0.5,
            max_size: 2.0,
            start_color: [1.0, 1.0, 1.0, 1.0],
            end_color: [1.0, 1.0, 1.0, 0.0],
            max_particles: 1000,
            looping: true,
            forces: vec![ParticleForce::Gravity {
                acceleration: DVec3::new(0.0, -9.81, 0.0),
            }],
            bursts: Vec::new(),
            system_lifetime: f64::MAX,
            min_mass: 1.0,
            max_mass: 1.0,
            start_scale: 1.0,
            end_scale: 1.0,
            min_image_size: [1.0, 1.0],
            max_image_size: [1.0, 1.0],
            size_in_meters: false,
            image: None,
        }
    }
}

/// The particle system that manages particle lifecycle.
#[derive(Debug)]
pub struct ParticleSystem {
    /// Configuration.
    pub config: ParticleSystemConfig,
    /// Active particles.
    pub particles: Vec<Particle>,
    /// Emitter position (world space).
    pub emitter_position: DVec3,
    /// Emitter direction (for cone emitters).
    pub emitter_direction: DVec3,
    /// Accumulated time for emission.
    emission_accumulator: f64,
    /// Next particle ID.
    next_id: u64,
    /// Total elapsed time.
    pub elapsed_time: f64,
    /// Whether the system is running.
    pub running: bool,
}

impl ParticleSystem {
    /// Creates a new particle system.
    pub fn new(config: ParticleSystemConfig, emitter_position: DVec3) -> Self {
        Self {
            config,
            particles: Vec::new(),
            emitter_position,
            emitter_direction: DVec3::Y, // Default up
            emission_accumulator: 0.0,
            next_id: 0,
            elapsed_time: 0.0,
            running: true,
        }
    }

    /// Creates a fire particle system preset.
    pub fn fire(emitter_position: DVec3) -> Self {
        let config = ParticleSystemConfig {
            emitter_shape: EmitterShape::Cone { angle: 0.3 },
            emission_rate: 50.0,
            min_lifetime: 0.5,
            max_lifetime: 1.5,
            min_speed: 2.0,
            max_speed: 5.0,
            min_size: 0.5,
            max_size: 1.5,
            start_color: [1.0, 0.8, 0.2, 1.0], // Orange-yellow
            end_color: [0.8, 0.2, 0.0, 0.0],   // Dark red, transparent
            max_particles: 500,
            looping: true,
            forces: vec![
                ParticleForce::Gravity {
                    acceleration: DVec3::new(0.0, 2.0, 0.0), // Upward (buoyancy)
                },
                ParticleForce::Drag { coefficient: 0.5 },
            ],
            ..Default::default()
        };
        Self::new(config, emitter_position)
    }

    /// Creates a smoke particle system preset.
    pub fn smoke(emitter_position: DVec3) -> Self {
        let config = ParticleSystemConfig {
            emitter_shape: EmitterShape::Sphere { radius: 0.5 },
            emission_rate: 20.0,
            min_lifetime: 2.0,
            max_lifetime: 5.0,
            min_speed: 0.5,
            max_speed: 2.0,
            min_size: 1.0,
            max_size: 3.0,
            start_color: [0.4, 0.4, 0.4, 0.8], // Gray
            end_color: [0.6, 0.6, 0.6, 0.0],   // Light gray, transparent
            max_particles: 300,
            looping: true,
            forces: vec![
                ParticleForce::Gravity {
                    acceleration: DVec3::new(0.0, 0.5, 0.0), // Slight upward
                },
                ParticleForce::Wind {
                    velocity: DVec3::new(1.0, 0.0, 0.0),
                },
            ],
            ..Default::default()
        };
        Self::new(config, emitter_position)
    }

    /// Creates a snow particle system preset.
    pub fn snow(emitter_position: DVec3) -> Self {
        let config = ParticleSystemConfig {
            emitter_shape: EmitterShape::Box {
                half_extents: DVec3::new(50.0, 0.1, 50.0),
            },
            emission_rate: 100.0,
            min_lifetime: 5.0,
            max_lifetime: 10.0,
            min_speed: 1.0,
            max_speed: 3.0,
            min_size: 0.1,
            max_size: 0.3,
            start_color: [1.0, 1.0, 1.0, 1.0], // White
            end_color: [1.0, 1.0, 1.0, 0.5],   // Semi-transparent
            max_particles: 2000,
            looping: true,
            forces: vec![
                ParticleForce::Gravity {
                    acceleration: DVec3::new(0.0, -1.0, 0.0), // Gentle fall
                },
                ParticleForce::Wind {
                    velocity: DVec3::new(0.5, 0.0, 0.3),
                },
            ],
            ..Default::default()
        };
        Self::new(config, emitter_position)
    }

    /// Updates the particle system by a time delta.
    ///
    /// # Arguments
    /// * `dt` - Time delta in seconds
    /// * `rng_seed` - Simple seed for deterministic randomness
    pub fn update(&mut self, dt: f64, rng_seed: u64) {
        if !self.running {
            return;
        }

        self.elapsed_time += dt;

        // Update existing particles
        for particle in &mut self.particles {
            particle.update(dt, &self.config.forces);
        }

        // Remove dead particles
        self.particles.retain(|p| p.alive);

        // Emit new particles
        if self.config.looping || self.elapsed_time < self.config.max_lifetime {
            self.emission_accumulator += dt * self.config.emission_rate;

            while self.emission_accumulator >= 1.0
                && self.particles.len() < self.config.max_particles
            {
                self.emission_accumulator -= 1.0;
                self.emit_particle(rng_seed + self.next_id);
            }
        }

        // Process bursts
        let bursts_to_fire: Vec<(usize, u32)> = self
            .config
            .bursts
            .iter()
            .enumerate()
            .filter(|(_, b)| !b.complete && self.elapsed_time >= b.time)
            .map(|(i, b)| (i, b.minimum.max(1)))
            .collect();

        for (idx, count) in bursts_to_fire {
            for i in 0..count {
                if self.particles.len() < self.config.max_particles {
                    self.emit_particle(rng_seed + self.next_id + i as u64);
                }
            }
            self.config.bursts[idx].complete = true;
        }

        // Check system lifetime
        if self.elapsed_time >= self.config.system_lifetime {
            if self.config.looping {
                self.elapsed_time = 0.0;
                for burst in &mut self.config.bursts {
                    burst.reset();
                }
            } else {
                self.running = false;
            }
        }
    }

    /// Emits a single particle.
    fn emit_particle(&mut self, seed: u64) {
        let rng = SimpleRng::new(seed);

        // Random lifetime
        let lifetime = rng.range(self.config.min_lifetime, self.config.max_lifetime);

        // Random speed
        let speed = rng.range(self.config.min_speed, self.config.max_speed);

        // Random size
        let size = rng.range(self.config.min_size, self.config.max_size);

        // Compute emission position and direction based on emitter shape
        let (position, direction) = self.compute_emission(&rng);

        let velocity = direction * speed;

        let mut particle = Particle::new(self.next_id, position, velocity, lifetime);
        particle.size = size;
        particle.color = self.config.start_color;

        self.particles.push(particle);
        self.next_id += 1;
    }

    /// Computes emission position and direction based on emitter shape.
    fn compute_emission(&self, rng: &SimpleRng) -> (DVec3, DVec3) {
        match &self.config.emitter_shape {
            EmitterShape::Point => (self.emitter_position, self.emitter_direction),
            EmitterShape::Cone { angle } => {
                // Random direction within cone
                let theta = rng.range(0.0, std::f64::consts::TAU);
                let phi = rng.range(0.0, *angle);

                let sin_phi = phi.sin();
                let dir = DVec3::new(
                    sin_phi * theta.cos(),
                    phi.cos(),
                    sin_phi * theta.sin(),
                );

                (self.emitter_position, dir.normalize())
            }
            EmitterShape::Box { half_extents } => {
                let offset = DVec3::new(
                    rng.range(-half_extents.x, half_extents.x),
                    rng.range(-half_extents.y, half_extents.y),
                    rng.range(-half_extents.z, half_extents.z),
                );
                (self.emitter_position + offset, self.emitter_direction)
            }
            EmitterShape::Sphere { radius } => {
                // Random point on sphere surface
                let theta = rng.range(0.0, std::f64::consts::TAU);
                let phi = rng.range(0.0, std::f64::consts::PI);

                let dir = DVec3::new(
                    phi.sin() * theta.cos(),
                    phi.cos(),
                    phi.sin() * theta.sin(),
                );

                (self.emitter_position + dir * *radius, dir)
            }
            EmitterShape::Circle { radius } => {
                let theta = rng.range(0.0, std::f64::consts::TAU);
                let r = rng.range(0.0, *radius);

                let offset = DVec3::new(r * theta.cos(), 0.0, r * theta.sin());
                (self.emitter_position + offset, self.emitter_direction)
            }
        }
    }

    /// Returns the number of alive particles.
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    /// Interpolates particle color based on age.
    pub fn particle_color(&self, particle: &Particle) -> [f64; 4] {
        let t = particle.normalized_age();
        let start = &self.config.start_color;
        let end = &self.config.end_color;

        [
            start[0] + (end[0] - start[0]) * t,
            start[1] + (end[1] - start[1]) * t,
            start[2] + (end[2] - start[2]) * t,
            start[3] + (end[3] - start[3]) * t,
        ]
    }

    /// Stops the particle system (no new emissions, existing particles continue).
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Starts/resumes the particle system.
    pub fn start(&mut self) {
        self.running = true;
    }

    /// Resets the particle system.
    pub fn reset(&mut self) {
        self.particles.clear();
        self.emission_accumulator = 0.0;
        self.elapsed_time = 0.0;
        self.next_id = 0;
        self.running = true;
    }
}

/// Simple deterministic RNG for particle emission.
#[derive(Debug, Clone)]
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407),
        }
    }

    fn next(&mut self) -> f64 {
        // xorshift64
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;

        // Convert to [0, 1)
        (self.state >> 11) as f64 / ((1u64 << 53) as f64)
    }

    fn range(&self, min: f64, max: f64) -> f64 {
        let mut rng = self.clone();
        min + rng.next() * (max - min)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_creation() {
        let particle = Particle::new(
            1,
            DVec3::ZERO,
            DVec3::new(1.0, 2.0, 3.0),
            5.0,
        );

        assert_eq!(particle.id, 1);
        assert!(particle.alive);
        assert_eq!(particle.age, 0.0);
        assert_eq!(particle.lifetime, 5.0);
    }

    #[test]
    fn test_particle_normalized_age() {
        let mut particle = Particle::new(1, DVec3::ZERO, DVec3::Y, 4.0);
        particle.age = 2.0;

        assert!((particle.normalized_age() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_particle_update_gravity() {
        let mut particle = Particle::new(1, DVec3::ZERO, DVec3::ZERO, 10.0);
        let forces = vec![ParticleForce::Gravity {
            acceleration: DVec3::new(0.0, -10.0, 0.0),
        }];

        particle.update(1.0, &forces);

        // After 1 second with -10 m/s² gravity
        assert!((particle.velocity.y - (-10.0)).abs() < 1e-10);
        assert!((particle.position.y - (-10.0)).abs() < 1e-10);
    }

    #[test]
    fn test_particle_death() {
        let mut particle = Particle::new(1, DVec3::ZERO, DVec3::Y, 1.0);
        let forces: Vec<ParticleForce> = vec![];

        particle.update(0.5, &forces);
        assert!(particle.alive);

        particle.update(0.6, &forces); // Total age = 1.1 > lifetime
        assert!(!particle.alive);
    }

    #[test]
    fn test_particle_drag() {
        let mut particle = Particle::new(1, DVec3::ZERO, DVec3::new(10.0, 0.0, 0.0), 10.0);
        let forces = vec![ParticleForce::Drag { coefficient: 0.5 }];

        particle.update(1.0, &forces);

        // Velocity should decrease due to drag
        assert!(particle.velocity.x < 10.0);
        assert!(particle.velocity.x > 0.0);
    }

    #[test]
    fn test_particle_system_creation() {
        let system = ParticleSystem::new(
            ParticleSystemConfig::default(),
            DVec3::ZERO,
        );

        assert_eq!(system.particle_count(), 0);
        assert!(system.running);
    }

    #[test]
    fn test_particle_system_emission() {
        let config = ParticleSystemConfig {
            emission_rate: 100.0, // 100 particles per second
            ..Default::default()
        };
        let mut system = ParticleSystem::new(config, DVec3::ZERO);

        system.update(1.0, 42); // 1 second

        // Should have emitted ~100 particles
        assert!(system.particle_count() > 50);
        assert!(system.particle_count() <= 100);
    }

    #[test]
    fn test_particle_system_max_particles() {
        let config = ParticleSystemConfig {
            emission_rate: 1000.0,
            max_particles: 50,
            ..Default::default()
        };
        let mut system = ParticleSystem::new(config, DVec3::ZERO);

        system.update(1.0, 42);

        assert!(system.particle_count() <= 50);
    }

    #[test]
    fn test_particle_system_stop() {
        let config = ParticleSystemConfig {
            emission_rate: 100.0,
            ..Default::default()
        };
        let mut system = ParticleSystem::new(config, DVec3::ZERO);

        system.update(0.5, 42);
        let count_before = system.particle_count();

        system.stop();
        system.update(0.5, 42);

        // No new particles emitted after stop
        // (some may have died, so count could be less)
        assert!(system.particle_count() <= count_before);
    }

    #[test]
    fn test_particle_system_reset() {
        let config = ParticleSystemConfig {
            emission_rate: 100.0,
            ..Default::default()
        };
        let mut system = ParticleSystem::new(config, DVec3::ZERO);

        system.update(1.0, 42);
        assert!(system.particle_count() > 0);

        system.reset();
        assert_eq!(system.particle_count(), 0);
        assert_eq!(system.elapsed_time, 0.0);
    }

    #[test]
    fn test_fire_preset() {
        let system = ParticleSystem::fire(DVec3::ZERO);

        assert_eq!(system.config.emission_rate, 50.0);
        assert!(matches!(system.config.emitter_shape, EmitterShape::Cone { .. }));
        assert_eq!(system.config.start_color, [1.0, 0.8, 0.2, 1.0]);
    }

    #[test]
    fn test_smoke_preset() {
        let system = ParticleSystem::smoke(DVec3::ZERO);

        assert_eq!(system.config.emission_rate, 20.0);
        assert!(matches!(system.config.emitter_shape, EmitterShape::Sphere { .. }));
    }

    #[test]
    fn test_snow_preset() {
        let system = ParticleSystem::snow(DVec3::ZERO);

        assert_eq!(system.config.emission_rate, 100.0);
        assert!(matches!(system.config.emitter_shape, EmitterShape::Box { .. }));
    }

    #[test]
    fn test_particle_color_interpolation() {
        let config = ParticleSystemConfig {
            start_color: [1.0, 0.0, 0.0, 1.0], // Red
            end_color: [0.0, 0.0, 1.0, 0.0],   // Blue, transparent
            ..Default::default()
        };
        let system = ParticleSystem::new(config, DVec3::ZERO);

        let mut particle = Particle::new(1, DVec3::ZERO, DVec3::Y, 2.0);
        particle.age = 1.0; // 50% through lifetime

        let color = system.particle_color(&particle);

        // Should be halfway between red and blue
        assert!((color[0] - 0.5).abs() < 1e-10);
        assert!((color[2] - 0.5).abs() < 1e-10);
        assert!((color[3] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_force_attractor() {
        let force = ParticleForce::Attractor {
            position: DVec3::new(10.0, 0.0, 0.0),
            strength: 100.0,
        };

        let particle = Particle::new(1, DVec3::ZERO, DVec3::ZERO, 10.0);
        let accel = force.compute_acceleration(&particle);

        // Should accelerate towards the attractor (+X direction)
        assert!(accel.x > 0.0);
    }

    #[test]
    fn test_force_vortex() {
        let force = ParticleForce::Vortex {
            axis: DVec3::Y,
            strength: 5.0,
        };

        let particle = Particle::new(1, DVec3::new(1.0, 0.0, 0.0), DVec3::ZERO, 10.0);
        let accel = force.compute_acceleration(&particle);

        // Should create circular motion (perpendicular to position and axis)
        assert!(accel.length() > 0.0);
    }

    #[test]
    fn test_simple_rng_deterministic() {
        let rng1 = SimpleRng::new(42);
        let rng2 = SimpleRng::new(42);

        assert!((rng1.range(0.0, 1.0) - rng2.range(0.0, 1.0)).abs() < 1e-10);
    }
}
