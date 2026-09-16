pub mod components;
pub mod controller_system;
pub mod control_impl;
pub mod flight_system;
pub mod scene_mode_system;
pub mod touch_system;
pub mod update_system;

pub use components::{ActiveFlight, ActiveMorph, CameraInputState, CesiumCamera, FlightComplete, FlyToRequest};
pub use controller_system::camera_controller_system;
pub use control_impl::{
    camera_control_port_system, CameraControlImpl, CameraControlPort, CameraState,
};
pub use flight_system::camera_flight_system;
pub use scene_mode_system::scene_mode_system;
pub use touch_system::{camera_touch_system, TouchCameraState};
pub use update_system::camera_update_system;

use bevy::prelude::*;

/// Plugin that registers CesiumRust camera systems and resources.
pub struct CesiumCameraPlugin;

impl Plugin for CesiumCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraInputState>()
            .init_resource::<ActiveFlight>()
            .init_resource::<ActiveMorph>()
            .init_resource::<TouchCameraState>()
            .init_resource::<CameraControlPort>()
            .add_event::<FlyToRequest>()
            .add_event::<FlightComplete>()
            .add_systems(PreUpdate, camera_controller_system)
            // Two-finger touch → camera (inert without touch input; runs after
            // the mouse controller and before the PostUpdate Transform write).
            .add_systems(Update, camera_touch_system)
            .add_systems(
                PostUpdate,
                (
                    // M2.5: CameraControl driving-port bridge, before the
                    // Transform writer so the port-driven pose is what gets
                    // converted to render units.
                    camera_control_port_system.before(camera_update_system),
                    camera_update_system,
                    camera_flight_system,
                    scene_mode_system,
                ),
            );
    }
}
