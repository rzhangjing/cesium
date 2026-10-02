//! 动画时钟控件：用键盘驱动时间轴播放/暂停/跳转/变速。
//!
//! `Space`/`P` 切换播放与暂停，`Esc` 停止，左/右方向键按帧步长回退/前进，
//! `+`/`-` 倍速加倍/减半。无真实 UI 时以 `info!` 回显时钟文本。
use bevy::prelude::*;

use crate::entity::time_system::AnimationClock;

/// 动画控件状态资源。
#[derive(Resource, Debug, Clone)]
pub struct AnimationWidget {
    /// 是否显示时钟文本与网格。
    pub show_ui_text: bool,
    /// 上一帧渲染的时钟文本（缓存）。
    last_time_text: String,
}

impl Default for AnimationWidget {
    /// 默认：显示文本、无历史内容。
    fn default() -> Self {
        Self {
            show_ui_text: true,
            last_time_text: String::new(),
        }
    }
}

/// 控件初始化占位系统（当前无实体需生成，保留接口）。
pub fn setup_animation_widget(mut _commands: Commands) {}

/// 主系统：根据键盘事件驱动 [`AnimationClock`]，并回显时钟状态。
///
/// # 参数
/// - `keyboard`：按键输入状态
/// - `clock`：动画时钟（可写）
/// - `time`：帧时钟（提供 delta 用于跳转步长）
/// - `widget`：控件状态（可写）
/// - `gizmos`：调试网格绘制器
pub fn animation_widget_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut clock: ResMut<AnimationClock>,
    time: Res<Time>,
    mut widget: ResMut<AnimationWidget>,
    mut gizmos: Gizmos,
) {
    // Space/P：在播放与暂停之间切换。
    if keyboard.just_pressed(KeyCode::Space) || keyboard.just_pressed(KeyCode::KeyP) {
        if clock.is_playing() {
            clock.pause();
        } else {
            clock.play();
        }
    }

    // Esc：停止时钟。
    if keyboard.just_pressed(KeyCode::Escape) {
        clock.stop();
    }

    // 右方向键：按当前帧时长的 3600 倍向前跳转。
    if keyboard.just_pressed(KeyCode::ArrowRight) {
        let current = clock.current_time();
        let step = current.add_seconds(time.delta_secs_f64() * 3600.0);
        clock.seek(step);
    }

    // 左方向键：同样步长向后跳转（取负）。
    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        let current = clock.current_time();
        let step = current.add_seconds(-time.delta_secs_f64() * 3600.0);
        clock.seek(step);
    }

    // +：倍速加倍。
    if keyboard.just_pressed(KeyCode::Equal) || keyboard.just_pressed(KeyCode::NumpadAdd) {
        let current_speed = clock.controller.clock.multiplier;
        clock.set_speed(current_speed * 2.0);
    }

    // -：倍速减半。
    if keyboard.just_pressed(KeyCode::Minus) || keyboard.just_pressed(KeyCode::NumpadSubtract) {
        let current_speed = clock.controller.clock.multiplier;
        clock.set_speed(current_speed * 0.5);
    }

    // 文本回显：拼接状态/儒略日/倍速，并根据播放态选色。
    if widget.show_ui_text {
        let state_str = if clock.is_playing() { "PLAY" } else { "PAUSE" };
        let jd = clock.current_time();
        let total_days = jd.total_days();
        let speed = clock.controller.clock.multiplier;

        widget.last_time_text = format!(
            "[{}] JD: {:.6}  Speed: {:.2}x  [Space=Pause/Play  Arrows=Seek  +/-=Speed  Esc=Stop]",
            state_str, total_days, speed
        );

        // 播放中用绿色，暂停用橙色。
        let color = if clock.is_playing() {
            Color::srgb(0.2, 0.8, 0.2)
        } else {
            Color::srgb(0.8, 0.6, 0.2)
        };

        // 以 2D 网格作为占位可视化（无真实文本渲染）。
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
    /// 验证默认显示文本且无历史内容。
    fn test_animation_widget_default() {
        let widget = AnimationWidget::default();
        assert!(widget.show_ui_text);
        assert!(widget.last_time_text.is_empty());
    }

    #[test]
    /// 验证 play/pause 切换播放标志。
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
    /// 验证跳到中点时进度约为 0.5。
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
