pub mod celestial_system;
pub mod sky_system;

pub use celestial_system::celestial_system;
pub use celestial_system::LightingParams;
pub use sky_system::{sky_system, SkyAtmosphere};

use bevy::prelude::*;

pub struct CesiumAtmospherePlugin;

impl Plugin for CesiumAtmospherePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SkyAtmosphere>()
            .init_resource::<LightingParams>()
            .add_systems(Update, (celestial_system, sky_system));
    }
}
