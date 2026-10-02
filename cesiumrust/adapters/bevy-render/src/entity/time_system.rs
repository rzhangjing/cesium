//! 时间动态实体更新系统。
//!
//! 管理 AnimationClock 资源与逐帧更新：
//! - 推进 AnimationController 的时间
//! - 更新带有时间动态属性的实体（位置、颜色、朝向）
//! - 处理可用性区间（基于时间显示/隐藏实体）

use bevy::prelude::*;
use cesium_animation::timeline::AnimationController;
use cesium_geospatial::cartographic::Cartographic;
use cesium_time::clock::Clock;
use cesium_time::julian_date::JulianDate;

use super::components::{CesiumEntity, EntityWrapper, GlobeEllipsoid, TimeDynamicProperties};

/// 用于控制动画播放的资源。
#[derive(Resource)]
pub struct AnimationClock {
    /// 底层动画控制器（持有 Clock 与播放状态）。
    pub controller: AnimationController,
}

impl AnimationClock {
    /// 用起止时间新建时钟（当前时间从 start 开始，初始暂停）。
    ///
    /// # 参数
    /// - `start`：起始儒略日
    /// - `stop`：结束儒略日
    pub fn new(start: JulianDate, stop: JulianDate) -> Self {
        let clock = Clock::new(start, stop, start);
        Self {
            controller: AnimationController::new(clock),
        }
    }

    /// 开始播放。
    pub fn play(&mut self) {
        self.controller.play();
    }

    /// 暂停（保留当前时间）。
    pub fn pause(&mut self) {
        self.controller.pause();
    }

    /// 停止并回到起点。
    pub fn stop(&mut self) {
        self.controller.stop();
    }

    /// 跳转到指定的绝对时间。
    ///
    /// # 参数
    /// - `time`：目标儒略日
    pub fn seek(&mut self, time: JulianDate) {
        self.controller.seek(time);
    }

    /// 按 [0,1] 比例跳转到时间轴上的对应位置。
    ///
    /// # 参数
    /// - `fraction`：进度比例
    pub fn seek_fraction(&mut self, fraction: f64) {
        self.controller.seek_fraction(fraction);
    }

    /// 设置时间倍速乘子。
    ///
    /// # 参数
    /// - `multiplier`：倍速（1=实时）
    pub fn set_speed(&mut self, multiplier: f64) {
        self.controller.set_speed(multiplier);
    }

    /// 取当前时间（儒略日）。
    pub fn current_time(&self) -> JulianDate {
        self.controller.clock.current_time
    }

    /// 取当前进度比例 [0,1]。
    pub fn progress(&self) -> f64 {
        self.controller.progress()
    }

    /// 是否处于播放（非暂停）状态。
    pub fn is_playing(&self) -> bool {
        !self.controller.paused
    }
}

/// 默认动画时钟（从 epoch 到 epoch+24h）。
impl Default for AnimationClock {
    /// 默认：2024-01-01 起、历 24 小时。
    fn default() -> Self {
        let start = JulianDate::from_date_components(2024, 1, 1, 0, 0, 0, 0.0);
        let stop = start.add_seconds(86400.0);
        Self::new(start, stop)
    }
}

/// 推进动画时钟并更新动态实体的系统。
///
/// # 参数
/// - `time`：帧时钟（提供 delta 秒）
/// - `animation_clock`：动画时钟（可写）
/// - `ellipsoid`：地球椭球（坐标转换）
/// - `query`：实体包装、变换、实体属性与时动态标记
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
    // 未播放时不推进也不重算。
    if !animation_clock.is_playing() {
        return;
    }

    // 按帧时长 tick 时钟，得到当前儒略日。
    let current_jd = animation_clock.controller.tick(time.delta_secs_f64());

    // 换算为从起点算起的秒数，供属性插值取值。
    let start = animation_clock.controller.clock.start_time;
    let elapsed_seconds = current_jd.seconds_difference(&start);

    for (entity_wrapper, mut transform, mut cesium_entity, time_dyn) in query.iter_mut() {
        let domain_entity = &entity_wrapper.0;

        // 可用性：当前时刻不在区间内则隐藏。
        if time_dyn.has_availability {
            if let Some(ref avail) = cesium_entity.availability {
                cesium_entity.show = avail.contains(&current_jd);
            }
        }

        // 隐藏的实体不再计算位置。
        if !cesium_entity.show {
            continue;
        }

        // 位置插值：取当前时刻的测绘学坐标→ECEF→变换平移。
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

/// 基于 `show` 字段应用实体可见性的系统。
///
/// # 参数
/// - `query`：实体属性与可见性
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
    /// 验证新建时钟初始暂停、进度 0、当前时间为起点。
    fn test_animation_clock_creation() {
        let start = JulianDate::from_date_components(2024, 6, 1, 12, 0, 0, 0.0);
        let stop = start.add_seconds(7200.0);
        let clock = AnimationClock::new(start, stop);

        assert!(!clock.is_playing());
        assert!((clock.progress() - 0.0).abs() < 1e-10);
        assert_eq!(clock.current_time(), start);
    }

    #[test]
    /// 验证 play/pause 切换播放标志。
    fn test_animation_clock_play_pause() {
        let mut clock = AnimationClock::default();
        clock.play();
        assert!(clock.is_playing());
        clock.pause();
        assert!(!clock.is_playing());
    }

    #[test]
    /// 验证按比例跳转后进度约为 0.5。
    fn test_animation_clock_seek() {
        let start = JulianDate::from_date_components(2024, 1, 1, 0, 0, 0, 0.0);
        let stop = start.add_seconds(3600.0);
        let mut clock = AnimationClock::new(start, stop);

        clock.seek_fraction(0.5);
        assert!((clock.progress() - 0.5).abs() < 1e-4);
    }

    #[test]
    /// 验证默认时钟当前时间为 2024-01-01。
    fn test_animation_clock_default() {
        let clock = AnimationClock::default();
        let start = JulianDate::from_date_components(2024, 1, 1, 0, 0, 0, 0.0);
        assert_eq!(clock.current_time(), start);
    }
}
