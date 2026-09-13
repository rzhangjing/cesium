//! Particle system: emitters, particles, bursts, and system lifecycle.
//!
//! Maps to CesiumJS:
//! - `Scene/ParticleSystem.js`
//! - `Scene/Particle.js`
//! - `Scene/ParticleBurst.js`
//! - `Scene/BoxEmitter.js`
//! - `Scene/CircleEmitter.js`
//! - `Scene/SphereEmitter.js`
//! - `Scene/ConeEmitter.js`

use glam::DVec3;
use std::f64::consts::PI;

const TWO_PI: f64 = 2.0 * PI;

// ============================================================================
// Emitters
// ============================================================================

/// Particle emitter types.
#[derive(Debug, Clone, PartialEq)]
pub enum ParticleEmitter {
    /// Emits within a box. Velocity emanates from center.
    Box {
        /// Width, height, depth dimensions in meters.
        dimensions: DVec3,
    },
    /// Emits from a circle. Velocity along +Z.
    Circle {
        /// Radius in meters.
        radius: f64,
    },
    /// Emits within a sphere. Velocity emanates from center.
    Sphere {
        /// Radius in meters.
        radius: f64,
    },
    /// Emits from cone tip. Velocity towards base.
    Cone {
        /// Half-angle of the cone in radians.
        angle: f64,
    },
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self::Circle { radius: 0.5 }
    }
}

impl ParticleEmitter {
    /// Compute the initial position and velocity for a new particle.
    ///
    /// Uses a simple deterministic pseudo-random based on seed for reproducibility.
    pub fn emit(&self, seed: f64) -> (DVec3, DVec3) {
        match self {
            Self::Box { dimensions } => {
                let half = *dimensions * 0.5;
                let x = lerp_signed(-half.x, half.x, frac(seed * 7.13));
                let y = lerp_signed(-half.y, half.y, frac(seed * 3.77));
                let z = lerp_signed(-half.z, half.z, frac(seed * 5.91));
                let pos = DVec3::new(x, y, z);
                let vel = if pos.length() > 1e-10 {
                    pos.normalize()
                } else {
                    DVec3::Z
                };
                (pos, vel)
            }
            Self::Circle { radius } => {
                let theta = frac(seed * 6.28) * TWO_PI;
                let rad = frac(seed * 2.17) * radius;
                let x = rad * theta.cos();
                let y = rad * theta.sin();
                (DVec3::new(x, y, 0.0), DVec3::Z)
            }
            Self::Sphere { radius } => {
                let theta = frac(seed * 4.31) * TWO_PI;
                let phi = frac(seed * 2.79) * PI;
                let rad = frac(seed * 1.53) * radius;
                let x = rad * theta.cos() * phi.sin();
                let y = rad * theta.sin() * phi.sin();
                let z = rad * phi.cos();
                let pos = DVec3::new(x, y, z);
                let vel = if pos.length() > 1e-10 {
                    pos.normalize()
                } else {
                    DVec3::Z
                };
                (pos, vel)
            }
            Self::Cone { angle } => {
                let cone_radius = angle.tan();
                let theta = frac(seed * 5.47) * TWO_PI;
                let rad = frac(seed * 3.23) * cone_radius;
                let x = rad * theta.cos();
                let y = rad * theta.sin();
                let vel = DVec3::new(x, y, 1.0).normalize();
                (DVec3::ZERO, vel)
            }
        }
    }
}

// ============================================================================
// Particle
// ============================================================================

/// A single particle in the system.
#[derive(Debug, Clone)]
pub struct Particle {
    /// Mass in kilograms.
    pub mass: f64,
    /// Position in world coordinates.
    pub position: DVec3,
    /// Velocity in world coordinates (m/s).
    pub velocity: DVec3,
    /// Total life in seconds.
    pub life: f64,
    /// Color at birth [R, G, B, A].
    pub start_color: [f64; 4],
    /// Color at death [R, G, B, A].
    pub end_color: [f64; 4],
    /// Scale at birth.
    pub start_scale: f64,
    /// Scale at death.
    pub end_scale: f64,
    /// Image size [width, height] in pixels.
    pub image_size: [f64; 2],
    /// Current age in seconds.
    pub age: f64,
}

impl Particle {
    /// Create a new particle.
    pub fn new(position: DVec3, velocity: DVec3, life: f64) -> Self {
        Self {
            mass: 1.0,
            position,
            velocity,
            life,
            start_color: [1.0, 1.0, 1.0, 1.0],
            end_color: [1.0, 1.0, 1.0, 1.0],
            start_scale: 1.0,
            end_scale: 1.0,
            image_size: [1.0, 1.0],
            age: 0.0,
        }
    }

    /// Get normalized age [0, 1].
    pub fn normalized_age(&self) -> f64 {
        if self.life <= 0.0 {
            return 1.0;
        }
        (self.age / self.life).clamp(0.0, 1.0)
    }

    /// Whether the particle is still alive.
    pub fn is_alive(&self) -> bool {
        self.age < self.life
    }

    /// Update the particle by dt seconds. Returns true if still alive.
    pub fn update(&mut self, dt: f64) -> bool {
        // Apply velocity
        self.position += self.velocity * dt;
        // Age
        self.age += dt;
        self.age < self.life
    }

    /// Get interpolated color at current age.
    pub fn current_color(&self) -> [f64; 4] {
        let t = self.normalized_age();
        [
            lerp_signed(self.start_color[0], self.end_color[0], t),
            lerp_signed(self.start_color[1], self.end_color[1], t),
            lerp_signed(self.start_color[2], self.end_color[2], t),
            lerp_signed(self.start_color[3], self.end_color[3], t),
        ]
    }

    /// Get interpolated scale at current age.
    pub fn current_scale(&self) -> f64 {
        let t = self.normalized_age();
        lerp_signed(self.start_scale, self.end_scale, t)
    }
}

// ============================================================================
// ParticleBurst
// ============================================================================

/// A burst of particles at a specific time.
#[derive(Debug, Clone, PartialEq)]
pub struct ParticleBurst {
    /// Time in seconds after system start.
    pub time: f64,
    /// Minimum number of particles.
    pub minimum: u32,
    /// Maximum number of particles.
    pub maximum: u32,
    /// Whether this burst has fired.
    pub complete: bool,
}

impl ParticleBurst {
    /// Create a new burst.
    pub fn new(time: f64, minimum: u32, maximum: u32) -> Self {
        Self {
            time,
            minimum,
            maximum,
            complete: false,
        }
    }

    /// Reset the burst for looping.
    pub fn reset(&mut self) {
        self.complete = false;
    }
}

// ============================================================================
// ParticleSystem
// ============================================================================

/// Particle system configuration and state.
#[derive(Debug, Clone)]
pub struct ParticleSystem {
    /// Whether the system is visible.
    pub show: bool,
    /// Whether to loop bursts.
    pub loop: bool,
    /// The emitter type.
    pub emitter: ParticleEmitter,
    /// Emission rate (particles per second).
    pub emission_rate: f64,
    /// Bursts configuration.
    pub bursts: Vec<ParticleBurst>,
    /// Start color [R, G, B, A].
    pub start_color: [f64; 4],
    /// End color [R, G, B, A].
    pub end_color: [f64; 4],
    /// Start scale.
    pub start_scale: f64,
    /// End scale.
    pub end_scale: f64,
    /// Minimum speed (m/s).
    pub minimum_speed: f64,
    /// Maximum speed (m/s).
    pub maximum_speed: f64,
    /// Minimum particle life (seconds).
    pub minimum_particle_life: f64,
    /// Maximum particle life (seconds).
    pub maximum_particle_life: f64,
    /// Minimum mass (kg).
    pub minimum_mass: f64,
    /// Maximum mass (kg).
    pub maximum_mass: f64,
    /// Minimum image size [w, h].
    pub minimum_image_size: [f64; 2],
    /// Maximum image size [w, h].
    pub maximum_image_size: [f64; 2],
    /// Whether size is in meters (vs pixels).
    pub size_in_meters: bool,
    /// System lifetime in seconds.
    pub lifetime: f64,
    /// Model matrix (4x4 column-major).
    pub model_matrix: [f64; 16],
    /// Emitter model matrix.
    pub emitter_model_matrix: [f64; 16],
    /// Image URI.
    pub image: Option<String>,

    // Runtime state
    /// Active particles.
    particles: Vec<Particle>,
    /// Current system time.
    current_time: f64,
    /// Carry-over fractional particles.
    carry_over: f64,
    /// Whether the system is complete.
    is_complete: bool,
    /// Seed counter for deterministic emission.
    seed_counter: f64,
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self {
            show: true,
            loop: true,
            emitter: ParticleEmitter::default(),
            emission_rate: 5.0,
            bursts: Vec::new(),
            start_color: [1.0, 1.0, 1.0, 1.0],
            end_color: [1.0, 1.0, 1.0, 1.0],
            start_scale: 1.0,
            end_scale: 1.0,
            minimum_speed: 1.0,
            maximum_speed: 1.0,
            minimum_particle_life: 5.0,
            maximum_particle_life: 5.0,
            minimum_mass: 1.0,
            maximum_mass: 1.0,
            minimum_image_size: [1.0, 1.0],
            maximum_image_size: [1.0, 1.0],
            size_in_meters: false,
            lifetime: f64::MAX,
            model_matrix: identity_matrix(),
            emitter_model_matrix: identity_matrix(),
            image: None,
            particles: Vec::new(),
            current_time: 0.0,
            carry_over: 0.0,
            is_complete: false,
            seed_counter: 0.0,
        }
    }
}

impl ParticleSystem {
    /// Create a new particle system.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the active particles.
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    /// Get the number of active particles.
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    /// Whether the system has completed its lifetime.
    pub fn is_complete(&self) -> bool {
        self.is_complete
    }

    /// Get the current system time.
    pub fn current_time(&self) -> f64 {
        self.current_time
    }

    /// Update the particle system by dt seconds.
    pub fn update(&mut self, dt: f64) {
        if !self.show || self.is_complete {
            return;
        }

        self.current_time += dt;

        // Check lifetime
        if self.current_time >= self.lifetime {
            if self.loop {
                self.current_time = 0.0;
                for burst in &mut self.bursts {
                    burst.reset();
                }
            } else {
                self.is_complete = true;
                return;
            }
        }

        // Emit new particles based on rate
        let to_emit = self.emission_rate * dt + self.carry_over;
        let count = to_emit.floor() as usize;
        self.carry_over = to_emit - count as f64;

        for _ in 0..count {
            self.emit_particle();
        }

        // Process bursts
        for burst in &mut self.bursts {
            if !burst.complete && self.current_time >= burst.time {
                let burst_count = if burst.maximum > burst.minimum {
                    burst.minimum
                        + (frac(self.seed_counter * 1.37) * (burst.maximum - burst.minimum) as f64)
                            .floor() as u32
                } else {
                    burst.minimum
                };
                for _ in 0..burst_count {
                    self.emit_particle();
                }
                burst.complete = true;
            }
        }

        // Update existing particles
        self.particles.retain_mut(|p| p.update(dt));
    }

    /// Emit a single particle.
    fn emit_particle(&mut self) {
        self.seed_counter += 1.0;
        let seed = self.seed_counter;

        let (mut pos, mut vel) = self.emitter.emit(seed);

        // Apply speed
        let speed = lerp_signed(
            self.minimum_speed,
            self.maximum_speed,
            frac(seed * 1.71),
        );
        vel *= speed;

        // Apply life
        let life = lerp_signed(
            self.minimum_particle_life,
            self.maximum_particle_life,
            frac(seed * 2.31),
        );

        let mut particle = Particle::new(pos, vel, life);
        particle.mass = lerp_signed(self.minimum_mass, self.maximum_mass, frac(seed * 3.11));
        particle.start_color = self.start_color;
        particle.end_color = self.end_color;
        particle.start_scale = self.start_scale;
        particle.end_scale = self.end_scale;
        particle.image_size = [
            lerp_signed(self.minimum_image_size[0], self.maximum_image_size[0], frac(seed * 4.13)),
            lerp_signed(self.minimum_image_size[1], self.maximum_image_size[1], frac(seed * 5.17)),
        ];

        // Apply emitter model matrix translation to position
        let em = &self.emitter_model_matrix;
        pos = DVec3::new(
            pos.x + em[12],
            pos.y + em[13],
            pos.z + em[14],
        );
        particle.position = pos;

        self.particles.push(particle);
    }

    /// Remove all particles.
    pub fn clear(&mut self) {
        self.particles.clear();
    }

    /// Reset the system to initial state.
    pub fn reset(&mut self) {
        self.particles.clear();
        self.current_time = 0.0;
        self.carry_over = 0.0;
        self.is_complete = false;
        self.seed_counter = 0.0;
        for burst in &mut self.bursts {
            burst.reset();
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn lerp_signed(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn frac(x: f64) -> f64 {
    x - x.floor()
}

fn identity_matrix() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_lifecycle() {
        let mut p = Particle::new(DVec3::ZERO, DVec3::new(1.0, 0.0, 0.0), 2.0);
        assert!(p.is_alive());
        assert_eq!(p.age, 0.0);

        assert!(p.update(1.0));
        assert!((p.age - 1.0).abs() < 1e-10);
        assert!((p.position.x - 1.0).abs() < 1e-10);
        assert!((p.normalized_age() - 0.5).abs() < 1e-10);

        assert!(!p.update(1.5));
        assert!(!p.is_alive());
    }

    #[test]
    fn test_particle_color_interpolation() {
        let mut p = Particle::new(DVec3::ZERO, DVec3::ZERO, 4.0);
        p.start_color = [1.0, 0.0, 0.0, 1.0];
        p.end_color = [0.0, 0.0, 1.0, 0.0];
        p.age = 2.0; // 50%

        let color = p.current_color();
        assert!((color[0] - 0.5).abs() < 1e-10);
        assert!((color[2] - 0.5).abs() < 1e-10);
        assert!((color[3] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_particle_scale_interpolation() {
        let mut p = Particle::new(DVec3::ZERO, DVec3::ZERO, 10.0);
        p.start_scale = 2.0;
        p.end_scale = 4.0;
        p.age = 5.0;

        assert!((p.current_scale() - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_box_emitter() {
        let emitter = ParticleEmitter::Box {
            dimensions: DVec3::new(2.0, 2.0, 2.0),
        };
        let (pos, vel) = emitter.emit(0.5);
        // Position should be within [-1, 1]^3
        assert!(pos.x.abs() <= 1.0);
        assert!(pos.y.abs() <= 1.0);
        assert!(pos.z.abs() <= 1.0);
        // Velocity should be normalized
        assert!((vel.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_circle_emitter() {
        let emitter = ParticleEmitter::Circle { radius: 2.0 };
        let (pos, vel) = emitter.emit(0.3);
        // Position in XY plane
        assert!((pos.z).abs() < 1e-10);
        // Within radius
        assert!((pos.x * pos.x + pos.y * pos.y).sqrt() <= 2.0 + 1e-10);
        // Velocity along Z
        assert!((vel - DVec3::Z).length() < 1e-10);
    }

    #[test]
    fn test_sphere_emitter() {
        let emitter = ParticleEmitter::Sphere { radius: 3.0 };
        let (pos, vel) = emitter.emit(0.7);
        assert!(pos.length() <= 3.0 + 1e-10);
        assert!((vel.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cone_emitter() {
        let emitter = ParticleEmitter::Cone {
            angle: std::f64::consts::FRAC_PI_4,
        };
        let (pos, vel) = emitter.emit(0.9);
        // Position at origin
        assert!(pos.length() < 1e-10);
        // Velocity has positive Z
        assert!(vel.z > 0.0);
        assert!((vel.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_particle_burst() {
        let mut burst = ParticleBurst::new(2.0, 10, 20);
        assert!(!burst.complete);
        burst.complete = true;
        burst.reset();
        assert!(!burst.complete);
    }

    #[test]
    fn test_particle_system_emission() {
        let mut sys = ParticleSystem::new();
        sys.emission_rate = 100.0;
        sys.minimum_particle_life = 1.0;
        sys.maximum_particle_life = 1.0;

        sys.update(0.1); // Should emit ~10 particles
        assert!(sys.particle_count() >= 9);
        assert!(sys.particle_count() <= 11);
    }

    #[test]
    fn test_particle_system_lifetime() {
        let mut sys = ParticleSystem::new();
        sys.lifetime = 1.0;
        sys.loop = false;
        sys.emission_rate = 10.0;

        sys.update(0.5);
        assert!(!sys.is_complete());

        sys.update(0.6);
        assert!(sys.is_complete());
    }

    #[test]
    fn test_particle_system_loop() {
        let mut sys = ParticleSystem::new();
        sys.lifetime = 1.0;
        sys.loop = true;
        sys.emission_rate = 10.0;

        sys.update(1.5); // Past lifetime, should loop
        assert!(!sys.is_complete());
        assert!(sys.current_time() < 1.0);
    }

    #[test]
    fn test_particle_system_burst() {
        let mut sys = ParticleSystem::new();
        sys.emission_rate = 0.0; // No continuous emission
        sys.bursts.push(ParticleBurst::new(0.5, 20, 20));
        sys.minimum_particle_life = 10.0;
        sys.maximum_particle_life = 10.0;

        sys.update(0.3);
        assert_eq!(sys.particle_count(), 0);

        sys.update(0.3); // Now at 0.6, burst at 0.5 should fire
        assert_eq!(sys.particle_count(), 20);
    }

    #[test]
    fn test_particle_system_reset() {
        let mut sys = ParticleSystem::new();
        sys.emission_rate = 50.0;
        sys.update(1.0);
        assert!(sys.particle_count() > 0);

        sys.reset();
        assert_eq!(sys.particle_count(), 0);
        assert_eq!(sys.current_time(), 0.0);
        assert!(!sys.is_complete());
    }

    #[test]
    fn test_particle_system_clear() {
        let mut sys = ParticleSystem::new();
        sys.emission_rate = 50.0;
        sys.update(1.0);
        sys.clear();
        assert_eq!(sys.particle_count(), 0);
    }

    #[test]
    fn test_particle_system_hidden() {
        let mut sys = ParticleSystem::new();
        sys.show = false;
        sys.emission_rate = 100.0;
        sys.update(1.0);
        assert_eq!(sys.particle_count(), 0);
    }

    #[test]
    fn test_particles_die_over_time() {
        let mut sys = ParticleSystem::new();
        sys.emission_rate = 10.0;
        sys.minimum_particle_life = 0.5;
        sys.maximum_particle_life = 0.5;

        sys.update(0.1); // Emit ~1
        let count_after_emit = sys.particle_count();
        assert!(count_after_emit > 0);

        // Wait for particles to die
        for _ in 0..10 {
            sys.update(0.1);
        }
        // Old particles should be dead, new ones emitted
        // With life=0.5 and 10 updates of 0.1, particles from first frame are dead
        assert!(sys.particle_count() < count_after_emit + 10);
    }
}
