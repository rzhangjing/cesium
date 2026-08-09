use bevy::prelude::*;

use crate::entity::time_system::AnimationClock;

#[derive(Resource, Debug, Clone)]
pub struct AnimationWidget {
    pub show_ui_text: bool,
    last_time_text: String,
}

impl Default for AnimationWidget {
    fn default() -> Self {
        Self {
            show_ui_text: true,
            last_time_text: String::new(),
        }
    }
}

pub fn setup_animation_widget(mut _commands: Commands) {}

pub fn animation_widget_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut clock: ResMut<AnimationClock>,
    time: Res<Time>,
    mut widget: ResMut<AnimationWidget>,
    mut gizmos: Gizmos,
) {
    if keyboard.just_pressed(KeyCode::Space) || keyboard.just_pressed(KeyCode::KeyP) {
        if clock.is_playing() {
            clock.pause();
        } else {
            clock.play();
        }
    }

    if keyboard.just_pressed(KeyCode::Escape) {
        clock.stop();
    }

    if keyboard.just_pressed(KeyCode::ArrowRight) {
        let current = clock.current_time();
        let step = current.add_seconds(time.delta_secs_f64() * 3600.0);
        clock.seek(step);
    }

    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        let current = clock.current_time();
        let step = current.add_seconds(-time.delta_secs_f64() * 3600.0);
        clock.seek(step);
    }

    if keyboard.just_pressed(KeyCode::Equal) || keyboard.just_pressed(KeyCode::NumpadAdd) {
        let current_speed = clock.controller.clock.multiplier;
        clock.set_speed(current_speed * 2.0);
    }

    if keyboard.just_pressed(KeyCode::Minus) || keyboard.just_pressed(KeyCode::NumpadSubtract) {
        let current_speed = clock.controller.clock.multiplier;
        clock.set_speed(current_speed * 0.5);
    }

    if widget.show_ui_text {
        let state_str = if clock.is_playing() { "PLAY" } else { "PAUSE" };
        let jd = clock.current_time();
        let total_days = jd.total_days();
        let speed = clock.controller.clock.multiplier;

        widget.last_time_text = format!(
            "[{}] JD: {:.6}  Speed: {:.2}x  [Space=Pause/Play  Arrows=Seek  +/-=Speed  Esc=Stop]",
            state_str, total_days, speed
        );

        let color = if clock.is_playing() {
            Color::srgb(0.2, 0.8, 0.2)
        } else {
            Color::srgb(0.8, 0.6, 0.2)
        };

        gizmos.grid_2d(
            Vec2::new(20.0, -20.0),
            UVec2::new(20, 2),
            Vec2::new(1.0, 1.0),
            color,
        );

        info!("{}", widget.last_time_text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_widget_default() {
        let widget = AnimationWidget::default();
        assert!(widget.show_ui_text);
        assert!(widget.last_time_text.is_empty());
    }

    #[test]
    fn test_animation_widget_plays_pauses() {
        let start = cesium_time::julian_date::JulianDate::from_date_components(2024, 6, 1, 0, 0, 0, 0.0);
        let stop = start.add_seconds(86400.0);
        let mut clock = AnimationClock::new(start, stop);

        assert!(!clock.is_playing());
        clock.play();
        assert!(clock.is_playing());
        clock.pause();
        assert!(!clock.is_playing());
    }

    #[test]
    fn test_animation_widget_seek() {
        let start = cesium_time::julian_date::JulianDate::from_date_components(2024, 6, 1, 0, 0, 0, 0.0);
        let stop = start.add_seconds(86400.0);
        let mut clock = AnimationClock::new(start, stop);

        let mid = start.add_seconds(43200.0);
        clock.seek(mid);
        let progress = clock.progress();
        assert!((progress - 0.5).abs() < 0.01);
    }
}
