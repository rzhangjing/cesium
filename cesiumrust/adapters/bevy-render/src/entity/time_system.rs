//! Time-dynamic entity update system.
//!
//! Manages the AnimationClock resource and per-frame updates:
//! - Advances time on the AnimationController
//! - Updates entities with time-dynamic properties (position, color, orientation)
//! - Handles availability intervals (show/hide entities based on time)

use bevy::prelude::*;
use cesium_animation::timeline::AnimationController;
use cesium_geospatial::cartographic::Cartographic;
use cesium_time::clock::Clock;
use cesium_time::julian_date::JulianDate;

use super::components::{CesiumEntity, EntityWrapper, GlobeEllipsoid, TimeDynamicProperties};

/// Resource for controlling animation playback.
#[derive(Resource)]
pub struct AnimationClock {
    pub controller: AnimationController,
}

impl AnimationClock {
    pub fn new(start: JulianDate, stop: JulianDate) -> Self {
        let clock = Clock::new(start, stop, start);
        Self {
            controller: AnimationController::new(clock),
        }
    }

    pub fn play(&mut self) {
        self.controller.play();
    }

    pub fn pause(&mut self) {
        self.controller.pause();
    }

    pub fn stop(&mut self) {
        self.controller.stop();
    }

    pub fn seek(&mut self, time: JulianDate) {
        self.controller.seek(time);
    }

    pub fn seek_fraction(&mut self, fraction: f64) {
        self.controller.seek_fraction(fraction);
    }

    pub fn set_speed(&mut self, multiplier: f64) {
        self.controller.set_speed(multiplier);
    }

    pub fn current_time(&self) -> JulianDate {
        self.controller.clock.current_time
    }

    pub fn progress(&self) -> f64 {
        self.controller.progress()
    }

    pub fn is_playing(&self) -> bool {
        !self.controller.paused
    }
}

/// Default animation clock (epoch to epoch+24h).
impl Default for AnimationClock {
    fn default() -> Self {
        let start = JulianDate::from_date_components(2024, 1, 1, 0, 0, 0, 0.0);
        let stop = start.add_seconds(86400.0);
        Self::new(start, stop)
    }
}

/// System that advances the animation clock and updates dynamic entities.
pub fn time_dynamic_update_system(
    time: Res<Time>,
    mut animation_clock: ResMut<AnimationClock>,
    ellipsoid: Res<GlobeEllipsoid>,
    mut query: Query<(
        &EntityWrapper,
        &mut Transform,
        &mut CesiumEntity,
        &TimeDynamicProperties,
    )>,
) {
    if !animation_clock.is_playing() {
        return;
    }

    let current_jd = animation_clock.controller.tick(time.delta_secs_f64());

    let start = animation_clock.controller.clock.start_time;
    let elapsed_seconds = current_jd.seconds_difference(&start);

    for (entity_wrapper, mut transform, mut cesium_entity, time_dyn) in query.iter_mut() {
        let domain_entity = &entity_wrapper.0;

        if time_dyn.has_availability {
            if let Some(ref avail) = cesium_entity.availability {
                cesium_entity.show = avail.contains(&current_jd);
            }
        }

        if !cesium_entity.show {
            continue;
        }

        if time_dyn.has_interpolated_position {
            if let Some(pos) = domain_entity.position.get_value(elapsed_seconds) {
                let carto = Cartographic::from_radians(pos[0], pos[1], pos[2]);
                let ecef = ellipsoid.0.cartographic_to_cartesian(&carto);
                transform.translation = bevy::math::Vec3::new(
                    ecef.x as f32,
                    ecef.y as f32,
                    ecef.z as f32,
                );
            }
        }
    }
}

/// System that applies entity visibility based on the `show` field.
pub fn entity_visibility_system(
    mut query: Query<(&CesiumEntity, &mut Visibility)>,
) {
    for (entity, mut visibility) in query.iter_mut() {
        *visibility = if entity.show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_clock_creation() {
        let start = JulianDate::from_date_components(2024, 6, 1, 12, 0, 0, 0.0);
        let stop = start.add_seconds(7200.0);
        let clock = AnimationClock::new(start, stop);

        assert!(!clock.is_playing());
        assert!((clock.progress() - 0.0).abs() < 1e-10);
        assert_eq!(clock.current_time(), start);
    }

    #[test]
    fn test_animation_clock_play_pause() {
        let mut clock = AnimationClock::default();
        clock.play();
        assert!(clock.is_playing());
        clock.pause();
        assert!(!clock.is_playing());
    }

    #[test]
    fn test_animation_clock_seek() {
        let start = JulianDate::from_date_components(2024, 1, 1, 0, 0, 0, 0.0);
        let stop = start.add_seconds(3600.0);
        let mut clock = AnimationClock::new(start, stop);

        clock.seek_fraction(0.5);
        assert!((clock.progress() - 0.5).abs() < 1e-4);
    }

    #[test]
    fn test_animation_clock_default() {
        let clock = AnimationClock::default();
        let start = JulianDate::from_date_components(2024, 1, 1, 0, 0, 0, 0.0);
        assert_eq!(clock.current_time(), start);
    }
}
