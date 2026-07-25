//! Scene/ParticleSystemSpec.js, ParticleSpec.js, EmitterSpec.js → Rust integration tests

use cesium_effects::{ParticleSystem, ParticleSystemConfig, EmitterShape};
use glam::DVec3;

// === EmitterShape ===

#[test]
fn test_emitter_shape_default() {
    let shape = EmitterShape::default();
    assert!(matches!(shape, EmitterShape::Point));
}

#[test]
fn test_emitter_shape_variants() {
    let _point = EmitterShape::Point;
    let _sphere = EmitterShape::Sphere { radius: 1.0 };
    let _box = EmitterShape::Box { half_extents: DVec3::new(1.0, 1.0, 1.0) };
    let _cone = EmitterShape::Cone { angle: 0.5 };
    let _circle = EmitterShape::Circle { radius: 1.0 };
}

// === ParticleSystemConfig ===

#[test]
fn test_particle_system_config_default() {
    let config = ParticleSystemConfig::default();
    assert!(config.emission_rate > 0.0);
}

// === ParticleSystem ===

#[test]
fn test_particle_system_new() {
    let config = ParticleSystemConfig::default();
    let system = ParticleSystem::new(config, DVec3::ZERO);
    assert_eq!(system.particle_count(), 0);
}

#[test]
fn test_particle_system_update() {
    let config = ParticleSystemConfig {
        emission_rate: 100.0,
        ..Default::default()
    };
    let mut system = ParticleSystem::new(config, DVec3::ZERO);
    system.update(0.1, 42); // 100ms with seed
    assert!(system.particle_count() > 0);
}
