pub mod components;
pub mod controller_system;
pub mod flight_system;
pub mod scene_mode_system;
pub mod update_system;

pub use components::{ActiveFlight, ActiveMorph, CameraInputState, CesiumCamera, FlightComplete, FlyToRequest};
pub use controller_system::camera_controller_system;
pub use flight_system::camera_flight_system;
pub use scene_mode_system::scene_mode_system;
pub use update_system::camera_update_system;

use bevy::prelude::*;

/// Plugin that registers CesiumRust camera systems and resources.
pub struct CesiumCameraPlugin;

impl Plugin for CesiumCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraInputState>()
            .init_resource::<ActiveFlight>()
            .init_resource::<ActiveMorph>()
            .add_event::<FlyToRequest>()
            .add_event::<FlightComplete>()
            .add_systems(PreUpdate, camera_controller_system)
            .add_systems(
                PostUpdate,
                (camera_update_system, camera_flight_system, scene_mode_system),
            );
    }
}
