use bevy::prelude::*;
use cesium_atmosphere::celestial::compute_sun_direction_eci;

use crate::entity::time_system::AnimationClock;

#[derive(Resource, Debug, Clone)]
pub struct LightingParams {
    pub sun_direction: Vec3,
    pub sun_color: [f32; 3],
    pub ambient_color: [f32; 3],
    pub ambient_intensity: f32,
}

impl Default for LightingParams {
    fn default() -> Self {
        Self {
            sun_direction: Vec3::new(1.0, 0.0, 0.0),
            sun_color: [1.0, 1.0, 0.9],
            ambient_color: [0.1, 0.1, 0.15],
            ambient_intensity: 0.3,
        }
    }
}

pub fn celestial_system(
    clock: Option<Res<AnimationClock>>,
    mut params: ResMut<LightingParams>,
    mut light_query: Query<&mut Transform, With<DirectionalLight>>,
) {
    let clock = match clock {
        Some(c) => c,
        None => return,
    };
    let jd = clock.current_time();
    let julian_date = jd.total_days();

    let sun_dir_eci = compute_sun_direction_eci(julian_date);

    let sun_dir_f32 = Vec3::new(
        sun_dir_eci.x as f32,
        sun_dir_eci.y as f32,
        sun_dir_eci.z as f32,
    );

    let normalized = sun_dir_f32.normalize_or_zero();
    if normalized != Vec3::ZERO {
        params.sun_direction = normalized;

        let sun_elevation = sun_dir_eci.z;
        if sun_elevation > 0.0 {
            let t = (sun_elevation * 2.0).clamp(0.0, 1.0) as f32;
            params.sun_color = [1.0, 0.95 + t * 0.05, 0.8 + t * 0.2];
            params.ambient_color = [0.2 + t * 0.1, 0.2 + t * 0.1, 0.3 + t * 0.1];
            params.ambient_intensity = 0.3 + t * 0.3;
        } else {
            let t = (-sun_elevation * 2.0).clamp(0.0, 1.0) as f32;
            params.sun_color = [1.0, 0.5 + t * 0.3, 0.2 + t * 0.3];
            params.ambient_color = [0.05 + t * 0.05, 0.05 + t * 0.05, 0.1 + t * 0.1];
            params.ambient_intensity = 0.1 + t * 0.1;
        }

        for mut light_transform in light_query.iter_mut() {
            light_transform.look_to(-normalized, Vec3::Y);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lighting_params_default() {
        let params = LightingParams::default();
        assert!((params.sun_direction.length() - 1.0).abs() < 1e-10);
        assert_eq!(params.sun_color, [1.0, 1.0, 0.9]);
        assert!((params.ambient_intensity - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_sun_direction_at_j2000() {
        let dir = compute_sun_direction_eci(2451545.0);
        assert!((dir.length() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_sun_position_changes_with_time() {
        let dir_a = compute_sun_direction_eci(2451545.0);
        let dir_b = compute_sun_direction_eci(2451545.0 + 0.5);
        let delta = (dir_a - dir_b).length();
        assert!(delta > 0.001, "Sun direction should change after 12 hours");
    }
}
