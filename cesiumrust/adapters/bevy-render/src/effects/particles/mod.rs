pub mod presets;
pub mod system;

use bevy::prelude::*;
pub use system::{
    particle_render_system, particle_spawn_system, particle_update_system,
    ParticleBurstResource, ParticleEmitterComponent, ParticleSystemComponent,
};

pub struct CesiumParticlePlugin;

impl Plugin for CesiumParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            particle_spawn_system,
            particle_update_system,
            particle_render_system,
        ).chain());
    }
}
