use bevy::prelude::*;
use cesium_interaction::CameraFlight;

use crate::camera::components::{ActiveFlight, CesiumCamera, FlightComplete, FlyToRequest};

/// 处理 FlyToRequest 事件并推进活飞行动画。
pub fn camera_flight_system(
    mut cameras: Query<&mut CesiumCamera>,
    mut active_flight: ResMut<ActiveFlight>,
    mut fly_requests: EventReader<FlyToRequest>,
    mut flight_complete: EventWriter<FlightComplete>,
    time: Res<Time>,
) {
    let dt = time.delta_secs() as f64;

    // --- 处理新的 fly-to 请求 ---
    for request in fly_requests.read() {
        for cesium_cam in cameras.iter() {
            let flight = CameraFlight::fly_to_cartographic(
                &cesium_cam.camera,
                &request.destination,
                &cesium_geospatial::Ellipsoid::WGS84,
                request.duration_secs.max(0.001),
            );
            active_flight.flight = Some(flight);
        }
    }

    // --- 推进活飞行 ---
    let mut is_done = false;
    if let Some(flight) = active_flight.flight.as_mut() {
        if flight.complete {
            is_done = true;
        } else {
            for mut cesium_cam in cameras.iter_mut() {
                let still_flying = flight.apply_to_camera(&mut cesium_cam.camera, dt);
                if !still_flying {
                    is_done = true;
                }
            }
        }
    }
    if is_done {
        active_flight.flight = None;
        flight_complete.send(FlightComplete);
    }
}
