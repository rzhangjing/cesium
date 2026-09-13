use cesium_effects::{
    EmitterShape, ParticleForce, ParticleSystem, ParticleSystemConfig,
};
use glam::DVec3;

pub fn fire_preset(emitter_position: DVec3) -> ParticleSystem {
    ParticleSystem::fire(emitter_position)
}

pub fn smoke_preset(emitter_position: DVec3) -> ParticleSystem {
    ParticleSystem::smoke(emitter_position)
}

pub fn snow_preset(emitter_position: DVec3) -> ParticleSystem {
    ParticleSystem::snow(emitter_position)
}

pub fn spark_preset(emitter_position: DVec3) -> ParticleSystem {
    let config = ParticleSystemConfig {
        emitter_shape: EmitterShape::Point,
        emission_rate: 30.0,
        min_lifetime: 0.1,
        max_lifetime: 1.0,
        min_speed: 5.0,
        max_speed: 15.0,
        min_size: 0.05,
        max_size: 0.2,
        start_color: [1.0, 0.9, 0.3, 1.0],
        end_color: [0.0, 0.0, 0.0, 0.0],
        max_particles: 200,
        looping: false,
        forces: vec![ParticleForce::Gravity {
            acceleration: DVec3::new(0.0, -9.81, 0.0),
        }],
        ..Default::default()
    };
    ParticleSystem::new(config, emitter_position)
}

pub fn rain_preset(emitter_position: DVec3) -> ParticleSystem {
    let config = ParticleSystemConfig {
        emitter_shape: EmitterShape::Box {
            half_extents: DVec3::new(20.0, 0.1, 20.0),
        },
        emission_rate: 200.0,
        min_lifetime: 1.0,
        max_lifetime: 3.0,
        min_speed: 15.0,
        max_speed: 30.0,
        min_size: 0.05,
        max_size: 0.15,
        start_color: [0.3, 0.5, 0.9, 0.6],
        end_color: [0.3, 0.5, 0.9, 0.1],
        max_particles: 3000,
        looping: true,
        forces: vec![
            ParticleForce::Gravity {
                acceleration: DVec3::new(0.0, -20.0, 0.0),
            },
            ParticleForce::Wind {
                velocity: DVec3::new(0.0, 0.0, 0.0),
            },
        ],
        ..Default::default()
    };
    let mut sys = ParticleSystem::new(config, emitter_position);
    sys.emitter_direction = DVec3::NEG_Y;
    sys
}
