//! 时间轴与动画控制器。
//!
//! 映射到 CesiumJS：
//! - `Widgets/Timeline/Timeline.js`
//! - `Widgets/Animation/AnimationViewModel.js`
//! - `Scene/Clock.js`（扩展）

use cesium_time::clock::Clock;
use cesium_time::julian_date::JulianDate;

/// 时间轴配置。
#[derive(Debug, Clone)]
pub struct TimelineConfig {
    /// 时间轴起始时间。
    pub start_time: JulianDate,
    /// 时间轴结束时间。
    pub end_time: JulianDate,
    /// 时间轴是否可见。
    pub visible: bool,
    /// 缩放级别（每像素秒数）。
    pub seconds_per_pixel: f64,
}

impl TimelineConfig {
    /// 创建一个新的时间轴配置。
    pub fn new(start_time: JulianDate, end_time: JulianDate) -> Self {
        let duration = end_time.seconds_difference(&start_time);
        Self {
            start_time,
            end_time,
            visible: true,
            seconds_per_pixel: duration / 1000.0, // 默认：1000px 宽
        }
    }

    /// 返回总时长（秒）。
    pub fn duration_seconds(&self) -> f64 {
        self.end_time.seconds_difference(&self.start_time)
    }
}

/// 用于 UI 渲染的时间轴状态。
#[derive(Debug, Clone)]
pub struct TimelineState {
    /// 当前时间位置。
    pub current_time: JulianDate,
    /// 可见起始时间。
    pub visible_start: JulianDate,
    /// 可见结束时间。
    pub visible_end: JulianDate,
    /// 播放头是否位于起点。
    pub at_start: bool,
    /// 播放头是否位于终点。
    pub at_end: bool,
}

/// 用于播放的动画控制器。
///
/// 映射到 CesiumJS `Widgets/Animation/AnimationViewModel.js`
#[derive(Debug, Clone)]
pub struct AnimationController {
    /// 底层时钟。
    pub clock: Clock,
    /// 播放速度乘数。
    pub speed_multiplier: f64,
    /// 播放是否已暂停。
    pub paused: bool,
    /// 是否循环播放。
    pub looping: bool,
    /// 拨盘环角度（-1.0 到 1.0）。
    pub shuttle_ring_angle: f64,
}

impl AnimationController {
    /// 创建一个新的动画控制器。
    pub fn new(clock: Clock) -> Self {
        Self {
            clock,
            speed_multiplier: 1.0,
            paused: true,
            looping: true,
            shuttle_ring_angle: 0.0,
        }
    }

    /// 向前播放动画。
    pub fn play(&mut self) {
        self.paused = false;
        self.clock.multiplier = self.speed_multiplier.abs();
    }

    /// 反向播放动画。
    pub fn play_reverse(&mut self) {
        self.paused = false;
        self.speed_multiplier = -self.speed_multiplier.abs();
        self.clock.multiplier = self.speed_multiplier;
    }

    /// 暂停动画。
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// 停止并重置到起点。
    pub fn stop(&mut self) {
        self.paused = true;
        self.clock.current_time = self.clock.start_time;
    }

    /// 设置速度乘数。
    pub fn set_speed(&mut self, multiplier: f64) {
        self.speed_multiplier = multiplier;
        if !self.paused {
            self.clock.multiplier = if self.clock.multiplier >= 0.0 {
                multiplier.abs()
            } else {
                -multiplier.abs()
            };
        }
    }

    /// 设置拨盘环角度（-1.0 到 1.0）。
    pub fn set_shuttle_ring(&mut self, angle: f64) {
        self.shuttle_ring_angle = angle.clamp(-1.0, 1.0);
        // 将角度映射为速度：0 = 暂停，±1 = 最大速度
        let max_speed = 1000.0; // 满偏时 1000 倍实时
        self.speed_multiplier = self.shuttle_ring_angle * max_speed;
        if self.shuttle_ring_angle.abs() < 0.01 {
            self.paused = true;
        } else {
            self.paused = false;
            self.clock.multiplier = self.speed_multiplier;
        }
    }

    /// 将动画推进 delta 秒。
    pub fn tick(&mut self, delta_secs: f64) -> JulianDate {
        if self.paused {
            return self.clock.current_time;
        }

        let effective_delta = delta_secs * self.speed_multiplier;
        let new_time = self.clock.current_time.add_seconds(effective_delta);

        // 处理循环
        if self.looping {
            let duration = self.clock.stop_time.seconds_difference(&self.clock.start_time);
            if duration > 0.0 {
                let elapsed = new_time.seconds_difference(&self.clock.start_time);
                let wrapped = elapsed.rem_euclid(duration);
                self.clock.current_time = self.clock.start_time.add_seconds(wrapped);
            } else {
                self.clock.current_time = new_time;
            }
        } else {
            // 夹取到范围内
            if new_time.less_than(&self.clock.start_time) {
                self.clock.current_time = self.clock.start_time;
                self.paused = true;
            } else if self.clock.stop_time.less_than(&new_time) {
                self.clock.current_time = self.clock.stop_time;
                self.paused = true;
            } else {
                self.clock.current_time = new_time;
            }
        }

        self.clock.current_time
    }

    /// 定位到特定时间。
    pub fn seek(&mut self, time: JulianDate) {
        self.clock.current_time = time;
    }

    /// 定位到时间轴的一个分数位置（0.0 到 1.0）。
    pub fn seek_fraction(&mut self, fraction: f64) {
        let duration = self.clock.stop_time.seconds_difference(&self.clock.start_time);
        let offset = duration * fraction.clamp(0.0, 1.0);
        self.clock.current_time = self.clock.start_time.add_seconds(offset);
    }

    /// 以分数（0.0 到 1.0）返回当前进度。
    pub fn progress(&self) -> f64 {
        let duration = self.clock.stop_time.seconds_difference(&self.clock.start_time);
        if duration <= 0.0 {
            return 0.0;
        }
        let elapsed = self.clock.current_time.seconds_difference(&self.clock.start_time);
        (elapsed / duration).clamp(0.0, 1.0)
    }

    /// 若正在向前播放则返回 true。
    pub fn is_playing_forward(&self) -> bool {
        !self.paused && self.speed_multiplier > 0.0
    }

    /// 若正在反向播放则返回 true。
    pub fn is_playing_reverse(&self) -> bool {
        !self.paused && self.speed_multiplier < 0.0
    }
}

/// 动画控制器的速度预设。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpeedPreset {
    /// 1 倍实时。
    RealTime,
    /// 2 倍速。
    Fast2x,
    /// 5 倍速。
    Fast5x,
    /// 10 倍速。
    Fast10x,
    /// 60 倍速（每秒 1 分钟）。
    Fast60x,
    /// 3600 倍速（每秒 1 小时）。
    Fast3600x,
    /// 86400 倍速（每秒 1 天）。
    Fast86400x,
}

impl SpeedPreset {
    /// 返回此预设的乘数。
    pub fn multiplier(&self) -> f64 {
        match self {
            Self::RealTime => 1.0,
            Self::Fast2x => 2.0,
            Self::Fast5x => 5.0,
            Self::Fast10x => 10.0,
            Self::Fast60x => 60.0,
            Self::Fast3600x => 3600.0,
            Self::Fast86400x => 86400.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_clock() -> Clock {
        let start = JulianDate::from_date_components(2024, 1, 1, 0, 0, 0, 0.0);
        let stop = start.add_seconds(3600.0); // 1 小时
        Clock::new(start, stop, start)
    }

    #[test]
    fn test_timeline_config() {
        let start = JulianDate::from_date_components(2024, 1, 1, 0, 0, 0, 0.0);
        let end = start.add_seconds(7200.0);
        let config = TimelineConfig::new(start, end);

        assert!((config.duration_seconds() - 7200.0).abs() < 1e-10);
        assert!(config.visible);
    }

    #[test]
    fn test_animation_controller_creation() {
        let clock = create_test_clock();
        let controller = AnimationController::new(clock);

        assert!(controller.paused);
        assert_eq!(controller.speed_multiplier, 1.0);
        assert!(controller.looping);
    }

    #[test]
    fn test_play_pause() {
        let clock = create_test_clock();
        let mut controller = AnimationController::new(clock);

        controller.play();
        assert!(!controller.paused);
        assert!(controller.is_playing_forward());

        controller.pause();
        assert!(controller.paused);
    }

    #[test]
    fn test_play_reverse() {
        let clock = create_test_clock();
        let mut controller = AnimationController::new(clock);

        controller.play_reverse();
        assert!(!controller.paused);
        assert!(controller.is_playing_reverse());
    }

    #[test]
    fn test_tick_advances_time() {
        let clock = create_test_clock();
        let start = clock.current_time;
        let mut controller = AnimationController::new(clock);
        controller.play();

        let new_time = controller.tick(1.0); // 1 倍速下 1 秒
        let elapsed = new_time.seconds_difference(&start);
        assert!((elapsed - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_tick_with_speed() {
        let clock = create_test_clock();
        let start = clock.current_time;
        let mut controller = AnimationController::new(clock);
        controller.set_speed(60.0);
        controller.play();

        let new_time = controller.tick(1.0); // 60 倍速下 1 秒
        let elapsed = new_time.seconds_difference(&start);
        assert!((elapsed - 60.0).abs() < 1e-10);
    }

    #[test]
    fn test_tick_paused() {
        let clock = create_test_clock();
        let start = clock.current_time;
        let mut controller = AnimationController::new(clock);
        // 默认处于暂停

        let new_time = controller.tick(1.0);
        assert_eq!(new_time, start);
    }

    #[test]
    fn test_loop_wrapping() {
        let clock = create_test_clock();
        let start = clock.start_time;
        let mut controller = AnimationController::new(clock);
        controller.looping = true;
        controller.play();

        // 推进超过终点（3600 + 100 秒）
        let new_time = controller.tick(3700.0);
        let elapsed = new_time.seconds_difference(&start);
        assert!((elapsed - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_no_loop_clamp() {
        let clock = create_test_clock();
        let stop = clock.stop_time;
        let mut controller = AnimationController::new(clock);
        controller.looping = false;
        controller.play();

        // 推进超过终点
        let new_time = controller.tick(3700.0);
        assert_eq!(new_time, stop);
        assert!(controller.paused); // 到达终点自动暂停
    }

    #[test]
    fn test_seek() {
        let clock = create_test_clock();
        let start = clock.start_time;
        let mut controller = AnimationController::new(clock);

        let target = start.add_seconds(1800.0);
        controller.seek(target);
        assert_eq!(controller.clock.current_time, target);
    }

    #[test]
    fn test_seek_fraction() {
        let clock = create_test_clock();
        let start = clock.start_time;
        let mut controller = AnimationController::new(clock);

        controller.seek_fraction(0.5);
        let elapsed = controller.clock.current_time.seconds_difference(&start);
        assert!((elapsed - 1800.0).abs() < 1e-10);
    }

    #[test]
    fn test_progress() {
        let clock = create_test_clock();
        let mut controller = AnimationController::new(clock);

        controller.seek_fraction(0.25);
        assert!((controller.progress() - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_shuttle_ring() {
        let clock = create_test_clock();
        let mut controller = AnimationController::new(clock);

        controller.set_shuttle_ring(0.5);
        assert!(!controller.paused);
        assert!(controller.speed_multiplier > 0.0);

        controller.set_shuttle_ring(0.0);
        assert!(controller.paused);
    }

    #[test]
    fn test_stop() {
        let clock = create_test_clock();
        let start = clock.start_time;
        let mut controller = AnimationController::new(clock);
        controller.play();
        controller.tick(100.0);

        controller.stop();
        assert!(controller.paused);
        assert_eq!(controller.clock.current_time, start);
    }

    #[test]
    fn test_speed_presets() {
        assert_eq!(SpeedPreset::RealTime.multiplier(), 1.0);
        assert_eq!(SpeedPreset::Fast60x.multiplier(), 60.0);
        assert_eq!(SpeedPreset::Fast86400x.multiplier(), 86400.0);
    }
}
