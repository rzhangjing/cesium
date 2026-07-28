//! Scene/ParticleSystemSpec.js, ParticleSpec.js, EmitterSpec.js → Rust integration tests
//!
//! Maps to CesiumJS:
//! - Scene/ParticleSystem.js (emission, lifecycle, bursts, forces)
//! - Scene/Particle.js (update, normalized_age, scale interpolation)
//! - Scene/BoxEmitter.js, CircleEmitter.js, SphereEmitter.js, ConeEmitter.js
//!
//! A-class tests: Particle lifecycle, forces (gravity/drag/wind/attractor/vortex),
//! ParticleSystem emission/max_particles/stop/reset/bursts, presets (fire/smoke/snow),
//! color interpolation, emitter shapes.
//! C-class omitted: WebGL rendering, billboard textures, Scene integration.

use cesium_effects::particles::{
    EmitterShape, Particle, ParticleBurst, ParticleForce, ParticleSystem, ParticleSystemConfig,
};
use glam::DVec3;

// === EmitterShape ===

#[test]
fn emitter_shape_default_is_point() {
    let shape = EmitterShape::default();
    assert!(matches!(shape, EmitterShape::Point));
}

#[test]
fn emitter_shape_variants() {
    let _point = EmitterShape::Point;
    let _sphere = EmitterShape::Sphere { radius: 1.0 };
    let _box = EmitterShape::Box { half_extents: DVec3::new(1.0, 1.0, 1.0) };
    let _cone = EmitterShape::Cone { angle: 0.5 };
    let _circle = EmitterShape::Circle { radius: 1.0 };
}

// === ParticleSystemConfig ===

#[test]
fn particle_system_config_defaults() {
    let config = ParticleSystemConfig::default();
    assert!((config.emission_rate - 10.0).abs() < 1e-10);
    assert!((config.min_lifetime - 1.0).abs() < 1e-10);
    assert!((config.max_lifetime - 3.0).abs() < 1e-10);
    assert!((config.min_speed - 1.0).abs() < 1e-10);
    assert!((config.max_speed - 5.0).abs() < 1e-10);
    assert_eq!(config.max_particles, 1000);
    assert!(config.looping);
    assert!(matches!(config.emitter_shape, EmitterShape::Point));
}

// === Particle ===

#[test]
fn particle_creation() {
    let p = Particle::new(1, DVec3::ZERO, DVec3::new(1.0, 2.0, 3.0), 5.0);
    assert_eq!(p.id, 1);
    assert!(p.alive);
    assert_eq!(p.age, 0.0);
    assert!((p.lifetime - 5.0).abs() < 1e-10);
    assert!((p.mass - 1.0).abs() < 1e-10);
}

#[test]
fn particle_normalized_age() {
    let mut p = Particle::new(1, DVec3::ZERO, DVec3::Y, 4.0);
    p.age = 2.0;
    assert!((p.normalized_age() - 0.5).abs() < 1e-10);
}

#[test]
fn particle_normalized_age_clamped() {
    let mut p = Particle::new(1, DVec3::ZERO, DVec3::Y, 2.0);
    p.age = 5.0;
    assert!((p.normalized_age() - 1.0).abs() < 1e-10);
}

#[test]
fn particle_current_scale_interpolation() {
    let mut p = Particle::new(1, DVec3::ZERO, DVec3::Y, 2.0);
    p.start_scale = 1.0;
    p.end_scale = 3.0;
    p.age = 1.0; // 50%
    assert!((p.current_scale() - 2.0).abs() < 1e-10);
}

#[test]
fn particle_update_gravity() {
    let mut p = Particle::new(1, DVec3::ZERO, DVec3::ZERO, 10.0);
    let forces = vec![ParticleForce::Gravity {
        acceleration: DVec3::new(0.0, -10.0, 0.0),
    }];
    p.update(1.0, &forces);
    assert!((p.velocity.y - (-10.0)).abs() < 1e-10);
    assert!((p.position.y - (-10.0)).abs() < 1e-10);
}

#[test]
fn particle_update_drag() {
    let mut p = Particle::new(1, DVec3::ZERO, DVec3::new(10.0, 0.0, 0.0), 10.0);
    let forces = vec![ParticleForce::Drag { coefficient: 0.5 }];
    p.update(1.0, &forces);
    // Drag: accel = -velocity * coeff = -10*0.5 = -5
    // new velocity = 10 + (-5)*1 = 5
    assert!((p.velocity.x - 5.0).abs() < 1e-10);
}

#[test]
fn particle_death_at_lifetime() {
    let mut p = Particle::new(1, DVec3::ZERO, DVec3::Y, 1.0);
    let forces: Vec<ParticleForce> = vec![];
    p.update(0.5, &forces);
    assert!(p.alive);
    p.update(0.6, &forces); // total 1.1 > 1.0
    assert!(!p.alive);
}

#[test]
fn particle_dead_no_update() {
    let mut p = Particle::new(1, DVec3::ZERO, DVec3::Y, 1.0);
    p.alive = false;
    let forces = vec![ParticleForce::Gravity { acceleration: DVec3::new(0.0, -10.0, 0.0) }];
    p.update(1.0, &forces);
    assert_eq!(p.position, DVec3::ZERO); // No movement
}

// === ParticleForce ===

#[test]
fn force_gravity_constant() {
    let force = ParticleForce::Gravity { acceleration: DVec3::new(0.0, -9.81, 0.0) };
    let p = Particle::new(1, DVec3::new(100.0, 200.0, 300.0), DVec3::ZERO, 10.0);
    let accel = force.compute_acceleration(&p);
    assert!((accel.y - (-9.81)).abs() < 1e-10);
}

#[test]
fn force_wind() {
    let force = ParticleForce::Wind { velocity: DVec3::new(5.0, 0.0, 0.0) };
    let p = Particle::new(1, DVec3::ZERO, DVec3::ZERO, 10.0);
    let accel = force.compute_acceleration(&p);
    // Wind: (wind_vel - particle_vel) * 0.1 = (5-0)*0.1 = 0.5
    assert!((accel.x - 0.5).abs() < 1e-10);
}

#[test]
fn force_attractor() {
    let force = ParticleForce::Attractor {
        position: DVec3::new(10.0, 0.0, 0.0),
        strength: 100.0,
    };
    let p = Particle::new(1, DVec3::ZERO, DVec3::ZERO, 10.0);
    let accel = force.compute_acceleration(&p);
    assert!(accel.x > 0.0); // Towards attractor
}

#[test]
fn force_vortex() {
    let force = ParticleForce::Vortex { axis: DVec3::Y, strength: 5.0 };
    let p = Particle::new(1, DVec3::new(1.0, 0.0, 0.0), DVec3::ZERO, 10.0);
    let accel = force.compute_acceleration(&p);
    assert!(accel.length() > 0.0);
}

// === ParticleBurst ===

#[test]
fn burst_creation() {
    let burst = ParticleBurst::new(2.0, 10, 50);
    assert!((burst.time - 2.0).abs() < 1e-10);
    assert_eq!(burst.minimum, 10);
    assert_eq!(burst.maximum, 50);
    assert!(!burst.complete);
}

#[test]
fn burst_reset() {
    let mut burst = ParticleBurst::new(1.0, 5, 20);
    burst.complete = true;
    burst.reset();
    assert!(!burst.complete);
}

// === ParticleSystem ===

#[test]
fn particle_system_new() {
    let system = ParticleSystem::new(ParticleSystemConfig::default(), DVec3::ZERO);
    assert_eq!(system.particle_count(), 0);
    assert!(system.running);
    assert!((system.elapsed_time - 0.0).abs() < 1e-10);
}

#[test]
fn particle_system_emission() {
    let config = ParticleSystemConfig {
        emission_rate: 100.0,
        ..Default::default()
    };
    let mut system = ParticleSystem::new(config, DVec3::ZERO);
    system.update(1.0, 42);
    assert!(system.particle_count() > 50);
    assert!(system.particle_count() <= 100);
}

#[test]
fn particle_system_max_particles_cap() {
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
fn particle_system_stop() {
    let config = ParticleSystemConfig {
        emission_rate: 100.0,
        ..Default::default()
    };
    let mut system = ParticleSystem::new(config, DVec3::ZERO);
    system.update(0.5, 42);
    let count = system.particle_count();
    system.stop();
    system.update(0.5, 42);
    assert!(system.particle_count() <= count);
}

#[test]
fn particle_system_reset() {
    let config = ParticleSystemConfig {
        emission_rate: 100.0,
        ..Default::default()
    };
    let mut system = ParticleSystem::new(config, DVec3::ZERO);
    system.update(1.0, 42);
    assert!(system.particle_count() > 0);
    system.reset();
    assert_eq!(system.particle_count(), 0);
    assert!((system.elapsed_time - 0.0).abs() < 1e-10);
}

#[test]
fn particle_system_burst_fires() {
    let config = ParticleSystemConfig {
        emission_rate: 0.0, // No continuous emission
        bursts: vec![ParticleBurst::new(0.5, 20, 20)],
        ..Default::default()
    };
    let mut system = ParticleSystem::new(config, DVec3::ZERO);
    system.update(0.3, 42); // Before burst time
    assert_eq!(system.particle_count(), 0);
    system.update(0.3, 42); // Total 0.6 > 0.5, burst fires
    assert!(system.particle_count() >= 20);
}

#[test]
fn particle_system_color_interpolation() {
    let config = ParticleSystemConfig {
        start_color: [1.0, 0.0, 0.0, 1.0],
        end_color: [0.0, 0.0, 1.0, 0.0],
        ..Default::default()
    };
    let system = ParticleSystem::new(config, DVec3::ZERO);
    let mut p = Particle::new(1, DVec3::ZERO, DVec3::Y, 2.0);
    p.age = 1.0; // 50%
    let color = system.particle_color(&p);
    assert!((color[0] - 0.5).abs() < 1e-10);
    assert!((color[2] - 0.5).abs() < 1e-10);
    assert!((color[3] - 0.5).abs() < 1e-10);
}

// === Presets ===

#[test]
fn preset_fire() {
    let system = ParticleSystem::fire(DVec3::ZERO);
    assert!((system.config.emission_rate - 50.0).abs() < 1e-10);
    assert!(matches!(system.config.emitter_shape, EmitterShape::Cone { .. }));
    assert_eq!(system.config.start_color, [1.0, 0.8, 0.2, 1.0]);
}

#[test]
fn preset_smoke() {
    let system = ParticleSystem::smoke(DVec3::ZERO);
    assert!((system.config.emission_rate - 20.0).abs() < 1e-10);
    assert!(matches!(system.config.emitter_shape, EmitterShape::Sphere { .. }));
}

#[test]
fn preset_snow() {
    let system = ParticleSystem::snow(DVec3::ZERO);
    assert!((system.config.emission_rate - 100.0).abs() < 1e-10);
    assert!(matches!(system.config.emitter_shape, EmitterShape::Box { .. }));
}
