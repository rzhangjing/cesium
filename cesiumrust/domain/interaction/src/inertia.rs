//! 惯性相机运动（手势释放后的滑行）。
//!
//! 纯 f64 领域逻辑，**不**依赖 Bevy / render-unit。像素空间的运动保持原样；
//! 像素→弧度/米 的缩放由适配层边界施加（参见
//! [`crate::camera_controller::CameraController::coast_inertia`]）。
//!
//! 映射到 CesiumJS `Scene/ScreenSpaceCameraController.js` 的惯性辅助函数，以及
//! Rust 蓝图 `cesium-rs/crates/cesium-scene/src/screen_space_camera_controller.rs`：
//! - `InertiaState` — 蓝图 L164-173（`Spin`/`Zoom`/`Translate`/`Tilt`）。
//! - `decay(time, coefficient)` — 蓝图 L107-113（`exp(-tau*time)`、
//!   `tau = (1 - coefficient) * 25`）。
//! - `activateInertia` — 蓝图 L766-786（重新启用某个状态，并禁用来自 CesiumJS
//!   `_inertiaDisablers` 的冲突状态）。
//! - `maintainInertia` — 蓝图 L796-875（在按钮抬起期间用衰减指数函数收窄最后一次
//!   移动，使相机滑行直至停下）。
//!
//! CesiumJS 的 `inertiaMaxClickTimeThreshold` 保护（蓝图 L98-102）在此以
//! [`INERTIA_MAX_CLICK_TIME_THRESHOLD`] 重现。

use glam::DVec2;

/// 若鼠标按下与抬起之间的时间不低于该阈值
/// （秒），则将该手势视为有意的按住，相机将
/// **不**会带惯性滑行。
///
/// CesiumJS `inertiaMaxClickTimeThreshold`（蓝图 L102）。
pub const INERTIA_MAX_CLICK_TIME_THRESHOLD: f64 = 0.4;

/// 低于该运动值即视为滑行停止（像素）。
///
/// 一旦 `Cartesian2.distance(start, end) < 0.5`，CesiumJS 就会退出
/// `maintainInertia`（蓝图 L865）；否则接近零的指数函数可能产生 NaN 或
/// 无尽的亚像素更新流。
pub const INERTIA_STOP_DISTANCE: f64 = 0.5;

/// `decay(time, coefficient)` — 用于收窄惯性运动的递减指数函数。
///
/// 返回 `exp(-tau * time)`，其中 `tau = (1 - coefficient) * 25`。更大的
/// `coefficient`（更接近 `1.0`）会得到更小的 `tau`，从而衰减更慢
/// （运动滑行更久）。负的 `time` 会被钳制为 `0.0`。
///
/// 忠实于蓝图 L107-113。
///
/// # 参数
/// * `time` - 自手势释放以来的经过时间（秒）。
/// * `coefficient` - `[0, 1]` 范围内的惯性系数（例如 CesiumJS 的
///   `inertiaSpin`/`inertiaZoom`/`inertiaTranslate`/`inertiaTilt`）。
#[inline]
pub fn decay(time: f64, coefficient: f64) -> f64 {
    if time < 0.0 {
        return 0.0;
    }
    let tau = (1.0 - coefficient) * 25.0;
    (-tau * time).exp()
}

/// 四个惯性运动状态，替代 CesiumJS 的字符串字段名。
///
/// 映射到蓝图 `InertiaState`（L164-173）：
/// - [`InertiaState::Spin`] — `_lastInertiaSpinMovement`。
/// - [`InertiaState::Zoom`] — `_lastInertiaZoomMovement`。
/// - [`InertiaState::Translate`] — `_lastInertiaTranslateMovement`。
/// - [`InertiaState::Tilt`] — `_lastInertiaTiltMovement`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InertiaState {
    /// 旋转自转（rotate3D / spin3D 滑行）。
    Spin,
    /// 缩放滑行。
    Zoom,
    /// 平移（pan）滑行。
    Translate,
    /// 俯仰倾斜滑行。
    Tilt,
}

impl InertiaState {
    /// 按声明顺序排列的所有状态。
    pub const ALL: [InertiaState; 4] = [
        InertiaState::Spin,
        InertiaState::Zoom,
        InertiaState::Translate,
        InertiaState::Tilt,
    ];

    /// 用于内部存储的稳定索引。
    #[inline]
    const fn index(self) -> usize {
        match self {
            InertiaState::Spin => 0,
            InertiaState::Zoom => 1,
            InertiaState::Translate => 2,
            InertiaState::Tilt => 3,
        }
    }

    /// 当 `self` 被激活时，CesiumJS 的 `_inertiaDisablers` 映射会关闭其惯性的
    /// 那些状态（蓝图 L776-780）。
    ///
    /// - `Zoom` 禁用 `[Spin, Translate, Tilt]`。
    /// - `Tilt` 禁用 `[Spin, Translate]`。
    /// - `Spin` / `Translate` 不禁用任何状态。
    #[inline]
    const fn disablers(self) -> &'static [InertiaState] {
        match self {
            InertiaState::Zoom => &[
                InertiaState::Spin,
                InertiaState::Translate,
                InertiaState::Tilt,
            ],
            InertiaState::Tilt => &[InertiaState::Spin, InertiaState::Translate],
            InertiaState::Spin | InertiaState::Translate => &[],
        }
    }
}

/// CesiumJS 在每个 `_lastInertia*Movement` 字段下存储的
/// `{ startPosition, endPosition, motion, inertiaEnabled }` 对象。
///
/// 映射到蓝图 `InertiaMovementState`（L178-188）。位置为像素
/// 坐标；`motion` 为最后一次移动增量的一半（蓝图 L852-853）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InertiaMovementState {
    /// `startPosition` — 滑行运动的锚定像素。
    pub start_position: DVec2,
    /// `endPosition` — 本帧的 `start_position + motion * decay(...)`。
    pub end_position: DVec2,
    /// `motion` — 最后一次移动增量的一半（像素）。
    pub motion: DVec2,
    /// `inertiaEnabled` — 该状态是否允许滑行。
    pub inertia_enabled: bool,
}

impl Default for InertiaMovementState {
    fn default() -> Self {
        Self {
            start_position: DVec2::ZERO,
            end_position: DVec2::ZERO,
            motion: DVec2::ZERO,
            inertia_enabled: true,
        }
    }
}

/// 逐帧的惯性样本：求值 [`InertiaController::maintain`] 所需的时序与系数。
///
/// 将这些打包在一起，可使公共 API 保持在 clippy 的参数预算之内，同时
/// 镜像蓝图的 `(decayCoef, pressTime, releaseTime, now)` 输入。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InertiaSample {
    /// 传入 [`decay`] 的惯性系数，范围 `[0, 1]`。
    pub decay_coef: f64,
    /// 按钮按下时间戳（毫秒）。
    pub press_time: f64,
    /// 按钮释放时间戳（毫秒）。
    pub release_time: f64,
    /// 当前时间戳（毫秒）。
    pub now: f64,
}

impl InertiaSample {
    /// 创建一个新样本。
    pub fn new(decay_coef: f64, press_time: f64, release_time: f64, now: f64) -> Self {
        Self {
            decay_coef,
            press_time,
            release_time,
            now,
        }
    }

    /// 按下→释放的时长（秒）（`(release - press) / 1000`）。
    ///
    /// 蓝图 L824。
    #[inline]
    pub fn click_threshold(&self) -> f64 {
        (self.release_time - self.press_time) / 1000.0
    }

    /// 自释放以来的经过时间（秒）（`(now - release) / 1000`）。
    ///
    /// 蓝图 L831。
    #[inline]
    pub fn from_now(&self) -> f64 {
        (self.now - self.release_time) / 1000.0
    }
}

/// 持有四个 [`InertiaMovementState`] 槽位并驱动滑行衰减。
///
/// 映射到 CesiumJS `ScreenSpaceCameraController` 的惯性字段及其
/// `activateInertia` / `maintainInertia` 辅助函数。这是一个纯领域对象：
/// 它既不读取活动的事件聚合器，也不触碰相机 —— 调用方在释放时捕获
/// 最后一次移动，然后喂入逐帧的 [`InertiaSample`] 并
/// 应用返回的增量（参见
/// [`crate::camera_controller::CameraController::coast_inertia`]）。
#[derive(Debug, Clone, Default)]
pub struct InertiaController {
    states: [Option<InertiaMovementState>; 4],
}

impl InertiaController {
    /// 创建一个未捕获任何惯性的空控制器。
    pub fn new() -> Self {
        Self {
            states: [None, None, None, None],
        }
    }

    /// `slot` 已存储的状态，若有。
    #[inline]
    pub fn state(&self, slot: InertiaState) -> Option<&InertiaMovementState> {
        self.states[slot.index()].as_ref()
    }

    /// 对 `slot` 已存储状态的可变访问，若有。
    #[inline]
    pub fn state_mut(&mut self, slot: InertiaState) -> Option<&mut InertiaMovementState> {
        self.states[slot.index()].as_mut()
    }

    /// 清除所有已存储的状态（例如在模式改变或相机重置时）。
    pub fn clear(&mut self) {
        self.states = [None, None, None, None];
    }

    /// 记录一次手势的最后移动，以便在释放时能够滑行。
    ///
    /// `motion` 存储为 `(last_end - last_start)` 的一半（蓝图
    /// L852-853），并启用该状态。由适配层在拖拽
    /// 手势结束时调用。
    pub fn capture(&mut self, slot: InertiaState, last_start: DVec2, last_end: DVec2) {
        let state = self.states[slot.index()].get_or_insert_with(Default::default);
        state.start_position = last_start;
        state.end_position = last_end;
        state.motion = (last_end - last_start) * 0.5;
        state.inertia_enabled = true;
    }

    /// `activateInertia(controller, inertiaStateName)`（蓝图 L766-786）。
    ///
    /// 在 `slot` 上重新启用惯性，并禁用列在 CesiumJS
    /// `_inertiaDisablers` 映射中的那些状态。`None`（CesiumJS 的 `undefined`，例如
    /// `look3D`）为空操作。仅修改已存在的槽位，正如蓝图用 `if let Some(...)`
    /// 保护每一次写入。
    pub fn activate(&mut self, slot: Option<InertiaState>) {
        let slot = match slot {
            Some(slot) => slot,
            None => return,
        };

        if let Some(state) = self.states[slot.index()].as_mut() {
            state.inertia_enabled = true;
        }
        for &other in slot.disablers() {
            if let Some(state) = self.states[other.index()].as_mut() {
                state.inertia_enabled = false;
            }
        }
    }

    /// 禁用 `slot` 的滑行，使其在下一次
    /// [`Self::maintain`] 调用时立即停止（即"释放 ⇒ 停止"路径）。
    pub fn deactivate(&mut self, slot: InertiaState) {
        if let Some(state) = self.states[slot.index()].as_mut() {
            state.inertia_enabled = false;
        }
    }

    /// `maintainInertia(...)`（蓝图 L796-875），简化为其纯数学部分。
    ///
    /// 用 [`decay`] 指数函数收窄捕获的运动，并返回本帧要应用的
    /// 增量（像素），当滑行应停止时返回 `None`。
    /// 在以下情况下滑行停止：
    /// - `slot` 未捕获任何内容；
    /// - 该状态被禁用（`inertia_enabled == false`）；
    /// - 按下→释放的时长达到 [`INERTIA_MAX_CLICK_TIME_THRESHOLD`]
    ///   （有意的按住，而非轻扫）；
    /// - 衰减后的增量为 NaN 或短于 [`INERTIA_STOP_DISTANCE`]。
    ///
    /// 所存储状态的 `end_position` 会被原地更新（蓝图 L858-860），
    /// 以便重复调用能观察到收窄后的运动。
    pub fn maintain(&mut self, slot: InertiaState, sample: &InertiaSample) -> Option<DVec2> {
        let state = self.states[slot.index()].as_mut()?;
        if !state.inertia_enabled {
            return None;
        }
        if sample.click_threshold() >= INERTIA_MAX_CLICK_TIME_THRESHOLD {
            return None;
        }

        let d = decay(sample.from_now(), sample.decay_coef);
        let delta = state.motion * d;
        state.end_position = state.start_position + delta;

        if delta.x.is_nan() || delta.y.is_nan() || delta.length() < INERTIA_STOP_DISTANCE {
            return None;
        }
        Some(delta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME_MS: f64 = 1000.0 / 60.0;

    #[test]
    fn decay_zero_time_is_one() {
        assert!((decay(0.0, 0.9) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn decay_negative_time_clamps_to_zero() {
        assert!((decay(-1.0, 0.9)).abs() < 1e-12);
    }

    #[test]
    fn decay_matches_exponential_formula() {
        // tau = (1 - 0.8) * 25 = 5 → decay(1.0) = exp(-5).
        let expected = (-5.0_f64).exp();
        assert!((decay(1.0, 0.8) - expected).abs() < 1e-12);
    }

    #[test]
    fn decay_higher_coefficient_decays_slower() {
        // 系数 0.95（tau = 1.25）比 0.5（tau = 12.5）衰减更慢。
        assert!(decay(1.0, 0.95) > decay(1.0, 0.5));
    }

    /// 任务要求：初速度 `v0` 在 60 帧后衰减到 `0.01 * v0` 以下
    /// （在 60 fps 下约 1 秒）。
    #[test]
    fn inertia_decays_below_one_percent_after_60_frames() {
        let mut controller = InertiaController::new();
        // motion = (last_end - last_start) * 0.5 = (1000, 0) → |v0| = 1000 px.
        controller.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(2000.0, 0.0));
        let v0 = 1000.0;

        // 一次快速轻扫：在同一瞬间按下并释放（阈值为 0）。
        let sample = InertiaSample::new(0.8, 0.0, 0.0, 60.0 * FRAME_MS);
        let delta = controller.maintain(InertiaState::Spin, &sample).expect("still coasting");

        let speed = delta.length();
        assert!(
            speed < 0.01 * v0,
            "after 60 frames speed {speed} should be < {} (1% of v0)",
            0.01 * v0
        );
        // 并且它尚未被钳制到完全停止。
        assert!(speed >= INERTIA_STOP_DISTANCE);
    }

    /// 任务要求：禁用惯性时运动立即停止。
    #[test]
    fn maintain_stops_immediately_when_disabled() {
        let mut controller = InertiaController::new();
        controller.capture(InertiaState::Translate, DVec2::ZERO, DVec2::new(400.0, 0.0));
        controller.deactivate(InertiaState::Translate);

        let sample = InertiaSample::new(0.9, 0.0, 0.0, FRAME_MS);
        assert!(controller.maintain(InertiaState::Translate, &sample).is_none());
    }

    #[test]
    fn maintain_returns_none_without_capture() {
        let mut controller = InertiaController::new();
        let sample = InertiaSample::new(0.9, 0.0, 0.0, FRAME_MS);
        assert!(controller.maintain(InertiaState::Zoom, &sample).is_none());
    }

    #[test]
    fn maintain_suppressed_for_deliberate_hold() {
        let mut controller = InertiaController::new();
        controller.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(2000.0, 0.0));
        // 按住 0.5s ≥ INERTIA_MAX_CLICK_TIME_THRESHOLD → 无滑行。
        let sample = InertiaSample::new(0.9, 0.0, 500.0, 516.0);
        assert!(controller.maintain(InertiaState::Spin, &sample).is_none());
    }

    #[test]
    fn maintain_stops_when_delta_below_stop_distance() {
        let mut controller = InertiaController::new();
        // 小运动 → 迅速衰减到 INERTIA_STOP_DISTANCE 以下。
        controller.capture(InertiaState::Tilt, DVec2::ZERO, DVec2::new(2.0, 0.0));
        let sample = InertiaSample::new(0.5, 0.0, 0.0, 5000.0);
        assert!(controller.maintain(InertiaState::Tilt, &sample).is_none());
    }

    #[test]
    fn maintain_monotonically_decays() {
        let mut controller = InertiaController::new();
        controller.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(4000.0, 0.0));

        let mut previous = f64::INFINITY;
        for frame in 1..=30 {
            let sample = InertiaSample::new(0.85, 0.0, 0.0, frame as f64 * FRAME_MS);
            match controller.maintain(InertiaState::Spin, &sample) {
                Some(delta) => {
                    let speed = delta.length();
                    assert!(speed < previous, "speed must monotonically decrease");
                    previous = speed;
                }
                None => break,
            }
        }
        assert!(previous < f64::INFINITY);
    }

    #[test]
    fn activate_zoom_disables_conflicting_states() {
        let mut controller = InertiaController::new();
        controller.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(100.0, 0.0));
        controller.capture(InertiaState::Translate, DVec2::ZERO, DVec2::new(100.0, 0.0));
        controller.capture(InertiaState::Tilt, DVec2::ZERO, DVec2::new(100.0, 0.0));
        controller.capture(InertiaState::Zoom, DVec2::ZERO, DVec2::new(100.0, 0.0));

        controller.activate(Some(InertiaState::Zoom));

        assert!(controller.state(InertiaState::Zoom).unwrap().inertia_enabled);
        assert!(!controller.state(InertiaState::Spin).unwrap().inertia_enabled);
        assert!(!controller.state(InertiaState::Translate).unwrap().inertia_enabled);
        assert!(!controller.state(InertiaState::Tilt).unwrap().inertia_enabled);
    }

    #[test]
    fn activate_tilt_disables_spin_and_translate() {
        let mut controller = InertiaController::new();
        controller.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(100.0, 0.0));
        controller.capture(InertiaState::Translate, DVec2::ZERO, DVec2::new(100.0, 0.0));
        controller.capture(InertiaState::Tilt, DVec2::ZERO, DVec2::new(100.0, 0.0));

        controller.activate(Some(InertiaState::Tilt));

        assert!(controller.state(InertiaState::Tilt).unwrap().inertia_enabled);
        assert!(!controller.state(InertiaState::Spin).unwrap().inertia_enabled);
        assert!(!controller.state(InertiaState::Translate).unwrap().inertia_enabled);
    }

    #[test]
    fn activate_spin_disables_nothing() {
        let mut controller = InertiaController::new();
        controller.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(100.0, 0.0));
        controller.capture(InertiaState::Zoom, DVec2::ZERO, DVec2::new(100.0, 0.0));

        controller.activate(Some(InertiaState::Spin));

        assert!(controller.state(InertiaState::Spin).unwrap().inertia_enabled);
        assert!(controller.state(InertiaState::Zoom).unwrap().inertia_enabled);
    }

    #[test]
    fn activate_none_is_noop() {
        let mut controller = InertiaController::new();
        controller.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(100.0, 0.0));
        controller.activate(None);
        assert!(controller.state(InertiaState::Spin).unwrap().inertia_enabled);
    }

    #[test]
    fn disabled_state_stops_coasting_after_activation_conflict() {
        let mut controller = InertiaController::new();
        controller.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(2000.0, 0.0));
        controller.capture(InertiaState::Zoom, DVec2::ZERO, DVec2::new(2000.0, 0.0));
        // 缩放手势胜出 → 自转惯性被禁用。
        controller.activate(Some(InertiaState::Zoom));

        let sample = InertiaSample::new(0.9, 0.0, 0.0, FRAME_MS);
        assert!(controller.maintain(InertiaState::Spin, &sample).is_none());
        assert!(controller.maintain(InertiaState::Zoom, &sample).is_some());
    }

    #[test]
    fn clear_removes_all_states() {
        let mut controller = InertiaController::new();
        controller.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(100.0, 0.0));
        controller.clear();
        assert!(controller.state(InertiaState::Spin).is_none());
    }

    #[test]
    fn capture_stores_half_delta_motion() {
        let mut controller = InertiaController::new();
        controller.capture(InertiaState::Translate, DVec2::new(10.0, 20.0), DVec2::new(50.0, 80.0));
        let state = controller.state(InertiaState::Translate).unwrap();
        assert!((state.motion.x - 20.0).abs() < 1e-12);
        assert!((state.motion.y - 30.0).abs() < 1e-12);
        assert_eq!(state.start_position, DVec2::new(10.0, 20.0));
    }

    #[test]
    fn inertia_state_all_covers_four_slots() {
        assert_eq!(InertiaState::ALL.len(), 4);
        for slot in InertiaState::ALL {
            assert!(slot.index() < 4);
        }
    }

    #[test]
    fn sample_thresholds_are_in_seconds() {
        let sample = InertiaSample::new(0.9, 100.0, 300.0, 800.0);
        assert!((sample.click_threshold() - 0.2).abs() < 1e-12);
        assert!((sample.from_now() - 0.5).abs() < 1e-12);
    }
}
