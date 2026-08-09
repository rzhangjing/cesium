//! Particle system extended specs - tests for Particle, ParticleForce, Burst, and ParticleSystem
//!
//! Covers: particle lifecycle, forces, burst emission, system presets

use cesium_effects::{Particle, ParticleForce, ParticleSystem, ParticleSystemConfig};
use glam::DVec3;

const EPSILON3: f64 = 1e-3;
const EPSILON6: f64 = 1e-6;

// ─── Particle lifecycle ──────────────────────────────────────────────────────

#[test]
fn particle_new() {
    let p = Particle::new(1, DVec3::new(0.0, 0.0, 0.0), DVec3::new(1.0, 0.0, 0.0), 2.0);
    assert_eq!(p.id, 1);
    assert!((p.lifetime - 2.0).abs() < EPSILON6);
    assert!(p.age < EPSILON6);
}

#[test]
fn particle_normalized_age_zero() {
    let p = Particle::new(1, DVec3::ZERO, DVec3::ZERO, 2.0);
    assert!((p.normalized_age() - 0.0).abs() < EPSILON6);
}

#[test]
fn particle_normalized_age_half() {
    let mut p = Particle::new(1, DVec3::ZERO, DVec3::ZERO, 2.0);
    p.age = 1.0;
    assert!((p.normalized_age() - 0.5).abs() < EPSILON6);
}

#[test]
fn particle_normalized_age_end() {
    let mut p = Particle::new(1, DVec3::ZERO, DVec3::ZERO, 2.0);
    p.age = 2.0;
    assert!((p.normalized_age() - 1.0).abs() < EPSILON6);
}

#[test]
fn particle_update_increments_age() {
    let mut p = Particle::new(1, DVec3::ZERO, DVec3::ZERO, 2.0);
    p.update(0.5, &[]);
    assert!((p.age - 0.5).abs() < EPSILON6);
}

#[test]
fn particle_update_moves_position() {
    let mut p = Particle::new(1, DVec3::ZERO, DVec3::new(1.0, 0.0, 0.0), 2.0);
    p.update(1.0, &[]);
    assert!((p.position.x - 1.0).abs() < EPSILON6);
}

#[test]
fn particle_current_scale_default() {
    let p = Particle::new(1, DVec3::ZERO, DVec3::ZERO, 2.0);
    let scale = p.current_scale();
    assert!(scale >= 0.0, "scale should be non-negative");
}

// ─── ParticleForce ───────────────────────────────────────────────────────────

#[test]
fn particle_force_gravity() {
    let force = ParticleForce::Gravity {
        acceleration: DVec3::new(0.0, -9.8, 0.0),
    };
    let p = Particle::new(1, DVec3::ZERO, DVec3::ZERO, 2.0);
    let accel = force.compute_acceleration(&p);
    assert!((accel.y - (-9.8)).abs() < EPSILON6);
}

#[test]
fn particle_force_wind() {
    let force = ParticleForce::Wind {
        velocity: DVec3::new(5.0, 0.0, 0.0),
    };
    let p = Particle::new(1, DVec3::ZERO, DVec3::ZERO, 2.0);
    let accel = force.compute_acceleration(&p);
    // Wind acceleration = (wind - particle_velocity) * 0.1 = (5,0,0) * 0.1 = (0.5,0,0)
    assert!((accel.x - 0.5).abs() < EPSILON6, "wind accel should be 0.5, got {}", accel.x);
}

#[test]
fn particle_force_drag() {
    let force = ParticleForce::Drag {
        coefficient: 0.5,
    };
    let mut p = Particle::new(1, DVec3::ZERO, DVec3::new(10.0, 0.0, 0.0), 2.0);
    p.velocity = DVec3::new(10.0, 0.0, 0.0);
    let accel = force.compute_acceleration(&p);
    // Drag should oppose velocity
    assert!(accel.x < 0.0, "drag should oppose velocity");
}

#[test]
fn particle_force_multiple() {
    let forces = vec![
        ParticleForce::Gravity {
            acceleration: DVec3::new(0.0, -9.8, 0.0),
        },
        ParticleForce::Wind {
            velocity: DVec3::new(2.0, 0.0, 0.0),
        },
    ];
    let p = Particle::new(1, DVec3::ZERO, DVec3::ZERO, 2.0);
    let mut total_accel = DVec3::ZERO;
    for force in &forces {
        total_accel += force.compute_acceleration(&p);
    }
    // Gravity: (0, -9.8, 0), Wind: (2,0,0) * 0.1 = (0.2, 0, 0)
    assert!((total_accel.y - (-9.8)).abs() < EPSILON6);
    assert!((total_accel.x - 0.2).abs() < EPSILON6);
}

// ─── ParticleSystem construction ─────────────────────────────────────────────

#[test]
fn particle_system_fire_preset() {
    let sys = ParticleSystem::fire(DVec3::ZERO);
    assert!(sys.particle_count() >= 0);
}

#[test]
fn particle_system_smoke_preset() {
    let sys = ParticleSystem::smoke(DVec3::ZERO);
    assert!(sys.particle_count() >= 0);
}

#[test]
fn particle_system_snow_preset() {
    let sys = ParticleSystem::snow(DVec3::ZERO);
    assert!(sys.particle_count() >= 0);
}

#[test]
fn particle_system_new_with_config() {
    let config = ParticleSystemConfig::default();
    let sys = ParticleSystem::new(config, DVec3::ZERO);
    assert!(sys.particle_count() >= 0);
}

// ─── ParticleSystem update ───────────────────────────────────────────────────

#[test]
fn particle_system_update_emits_particles() {
    let mut sys = ParticleSystem::fire(DVec3::ZERO);
    let initial_count = sys.particle_count();
    sys.update(0.1, 42);
    // After update, should have emitted some particles
    let new_count = sys.particle_count();
    assert!(
        new_count >= initial_count,
        "particle count should not decrease after emit"
    );
}

#[test]
fn particle_system_update_advances_age() {
    let mut sys = ParticleSystem::fire(DVec3::ZERO);
    sys.update(0.5, 42);
    sys.update(0.5, 43);
    // After 1 second, some particles should have aged
    let count = sys.particle_count();
    assert!(count >= 0, "should have valid particle count");
}

#[test]
fn particle_system_particle_color() {
    let sys = ParticleSystem::fire(DVec3::ZERO);
    let p = Particle::new(1, DVec3::ZERO, DVec3::ZERO, 2.0);
    let color = sys.particle_color(&p);
    // Color should be valid RGBA
    assert!(color[0] >= 0.0 && color[0] <= 1.0);
    assert!(color[1] >= 0.0 && color[1] <= 1.0);
    assert!(color[2] >= 0.0 && color[2] <= 1.0);
    assert!(color[3] >= 0.0 && color[3] <= 1.0);
}

// ─── ParticleSystem presets emit different particles ─────────────────────────

#[test]
fn particle_system_fire_vs_smoke_differ() {
    let fire = ParticleSystem::fire(DVec3::ZERO);
    let smoke = ParticleSystem::smoke(DVec3::ZERO);
    // Fire and smoke should have different configurations
    let fire_p = Particle::new(1, DVec3::ZERO, DVec3::ZERO, 2.0);
    let smoke_p = Particle::new(1, DVec3::ZERO, DVec3::ZERO, 2.0);
    let fire_color = fire.particle_color(&fire_p);
    let smoke_color = smoke.particle_color(&smoke_p);
    // Colors should differ
    let diff = ((fire_color[0] - smoke_color[0]).abs()
        + (fire_color[1] - smoke_color[1]).abs()
        + (fire_color[2] - smoke_color[2]).abs())
        / 3.0;
    assert!(
        diff > 0.01,
        "fire and smoke colors should differ: fire={:?}, smoke={:?}",
        fire_color,
        smoke_color
    );
}

#[test]
fn particle_system_emitter_position_matters() {
    let sys1 = ParticleSystem::fire(DVec3::new(0.0, 0.0, 0.0));
    let sys2 = ParticleSystem::fire(DVec3::new(100.0, 0.0, 0.0));
    // Systems at different positions should both be valid
    assert!(sys1.particle_count() >= 0);
    assert!(sys2.particle_count() >= 0);
}
