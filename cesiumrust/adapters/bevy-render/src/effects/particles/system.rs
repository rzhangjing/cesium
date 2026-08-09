use bevy::prelude::*;
use cesium_effects::{
    EmitterShape, ParticleBurst, ParticleSystem, ParticleSystemConfig,
};
use glam::{DVec3, Vec3};

#[derive(Component, Debug)]
pub struct ParticleSystemComponent {
    pub system: ParticleSystem,
    pub visible: bool,
}

impl ParticleSystemComponent {
    pub fn new(config: ParticleSystemConfig, emitter_position: DVec3) -> Self {
        Self {
            system: ParticleSystem::new(config, emitter_position),
            visible: true,
        }
    }

    pub fn fire(emitter_position: DVec3) -> Self {
        Self {
            system: ParticleSystem::fire(emitter_position),
            visible: true,
        }
    }

    pub fn smoke(emitter_position: DVec3) -> Self {
        Self {
            system: ParticleSystem::smoke(emitter_position),
            visible: true,
        }
    }

    pub fn snow(emitter_position: DVec3) -> Self {
        Self {
            system: ParticleSystem::snow(emitter_position),
            visible: true,
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct ParticleEmitterComponent {
    pub position: DVec3,
    pub direction: DVec3,
    pub shape: EmitterShape,
}

impl Default for ParticleEmitterComponent {
    fn default() -> Self {
        Self {
            position: DVec3::ZERO,
            direction: DVec3::Y,
            shape: EmitterShape::Point,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct ParticleBurstResource {
    pub bursts: Vec<ParticleBurst>,
    pub time: f64,
}

impl Default for ParticleBurstResource {
    fn default() -> Self {
        Self {
            bursts: Vec::new(),
            time: 0.0,
        }
    }
}

impl ParticleBurstResource {
    pub fn add_burst(&mut self, burst: ParticleBurst) {
        self.bursts.push(burst);
    }
}

pub fn particle_spawn_system(
    time: Res<Time>,
    mut query: Query<(&mut ParticleSystemComponent, &GlobalTransform)>,
) {
    let dt = time.delta_secs_f64();
    let rng_seed = (time.elapsed_secs_f64() * 1000.0) as u64;

    for (mut comp, transform) in query.iter_mut() {
        if !comp.visible {
            continue;
        }

        let pos = DVec3::new(
            transform.translation().x as f64,
            transform.translation().y as f64,
            transform.translation().z as f64,
        );
        comp.system.emitter_position = pos;

        comp.system.update(dt, rng_seed);
    }
}

pub fn particle_update_system(
    mut gizmos: Gizmos,
    query: Query<(&ParticleSystemComponent, &GlobalTransform)>,
) {
    for (comp, transform) in query.iter() {
        let parent_pos = DVec3::new(
            transform.translation().x as f64,
            transform.translation().y as f64,
            transform.translation().z as f64,
        );

        for particle in &comp.system.particles {
            if !particle.alive {
                continue;
            }
            let color = comp.system.particle_color(particle);
            let pos = particle.position;
            let size = (particle.size * particle.current_scale()) as f32;

            let world_pos = Vec3::new(
                (pos.x) as f32,
                (pos.y) as f32,
                (pos.z) as f32,
            );

            let _parent = Vec3::new(
                parent_pos.x as f32,
                parent_pos.y as f32,
                parent_pos.z as f32,
            );

            gizmos.circle(
                Isometry3d::new(world_pos, Quat::IDENTITY),
                size.max(0.05),
                Color::srgba(
                    color[0] as f32,
                    color[1] as f32,
                    color[2] as f32,
                    color[3] as f32,
                ),
            );
        }
    }
}

pub fn particle_render_system(
    query: Query<(&ParticleSystemComponent, &GlobalTransform)>,
) {
    for (_comp, _transform) in query.iter() {
        // GPU rendering deferred — particles are currently drawn
        // via gizmos in particle_update_system.
        // A future compute-shader pass will replace this.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_effects::ParticleForce;

    #[test]
    fn test_particle_system_component_fire() {
        let comp = ParticleSystemComponent::fire(DVec3::new(0.0, 0.0, 0.0));
        assert!(comp.visible);
        assert!(
            matches!(comp.system.config.emitter_shape, EmitterShape::Cone { .. })
        );
        assert_eq!(comp.system.config.start_color, [1.0, 0.8, 0.2, 1.0]);
    }

    #[test]
    fn test_particle_system_component_smoke() {
        let comp = ParticleSystemComponent::smoke(DVec3::new(1.0, 2.0, 3.0));
        assert!(comp.visible);
        assert!(
            matches!(comp.system.config.emitter_shape, EmitterShape::Sphere { .. })
        );
        assert_eq!(comp.system.config.start_color, [0.4, 0.4, 0.4, 0.8]);
    }

    #[test]
    fn test_particle_system_component_snow() {
        let comp = ParticleSystemComponent::snow(DVec3::new(10.0, 100.0, 10.0));
        assert!(comp.visible);
        assert!(
            matches!(comp.system.config.emitter_shape, EmitterShape::Box { .. })
        );
        assert_eq!(comp.system.config.start_color, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_particle_lifecycle() {
        let mut comp = ParticleSystemComponent::fire(DVec3::ZERO);
        comp.system.config.emission_rate = 50.0;
        comp.system.config.min_lifetime = 0.5;
        comp.system.config.max_lifetime = 0.5;

        comp.system.update(0.1, 42);
        assert!(comp.system.particle_count() > 0);

        for _ in 0..20 {
            comp.system.update(0.1, 42);
        }

        // Particles should still be alive because system loops
        // (some may have died but new ones emitted)
        assert!(comp.system.particle_count() > 0);
    }

    #[test]
    fn test_preset_creation() {
        let fire = ParticleSystemComponent::fire(DVec3::ZERO);
        assert_eq!(fire.system.config.emission_rate, 50.0);

        let smoke = ParticleSystemComponent::smoke(DVec3::ZERO);
        assert_eq!(smoke.system.config.emission_rate, 20.0);

        let snow = ParticleSystemComponent::snow(DVec3::ZERO);
        assert_eq!(snow.system.config.emission_rate, 100.0);
    }
}
