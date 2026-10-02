//! 动画（animation）widget 视图模型。
//!
//! 提供播放/暂停、正反向、速度倍率与动感环控制的纯领域视图模型，
//! 并含动感环角度与速度倍率间基于对数尺度的双向转换。

/// 实时（倍率 1.0）对应的动感环角度（度）。
pub const REALTIME_SHUTTLE_RING_ANGLE: f64 = 15.0;
/// 动感环可偏转的最大角度（度），对应最快/最慢倍率。
pub const MAX_SHUTTLE_RING_ANGLE: f64 = 105.0;

/// 默认动感环刻度（速度倍率）。
pub const DEFAULT_SHUTTLE_RING_TICKS: &[f64] = &[
    -1000.0, -100.0, -50.0, -25.0, -10.0, -5.0, -2.0, -1.0,
    1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0, 1000.0,
];

/// 用于日期显示的月份名称。
pub const MONTH_NAMES: &[&str] = &[
    "Jan", "Feb", "Mar", "Apr", "May", "Jun",
    "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// 动感环角度 ↔ 倍率转换。
///
/// 将环上角度映射为时间流速倍率：15° 内线性对应 [-1, 1]，
/// 超出部分按对数尺度平滑过渡到最快/最慢档。
#[derive(Debug, Clone)]
pub struct ShuttleRing {
    /// 动感环的刻度值。
    pub ticks: Vec<f64>,
}

impl Default for ShuttleRing {
    /// 默认采用内置的 16 档典型速度刻度。
    fn default() -> Self {
        Self {
            ticks: DEFAULT_SHUTTLE_RING_TICKS.to_vec(),
        }
    }
}

impl ShuttleRing {
    /// 使用自定义刻度创建。
    pub fn with_ticks(ticks: Vec<f64>) -> Self {
        // 拷入后按升序排序，保证二分查找与刻度读取依赖有序前提
        let mut sorted = ticks;
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Self { ticks: sorted }
    }

    /// 将动感环角度转换为速度倍率。
    ///
    /// 角度范围：[-MAX_SHUTTLE_RING_ANGLE, MAX_SHUTTLE_RING_ANGLE]
    /// - [-15, 15] 内的角度线性映射到 [-1, 1]
    /// - 范围外的角度使用对数尺度
    pub fn angle_to_multiplier(&self, angle: f64) -> f64 {
        // 实时区：角度在 ±15° 内直接线性归一到 [-1, 1]
        if angle.abs() <= REALTIME_SHUTTLE_RING_ANGLE {
            return angle / REALTIME_SHUTTLE_RING_ANGLE;
        }

        // 超出实时区：以角度区间 [15°, 105°] 对数映射到倍率区间
        let minp = REALTIME_SHUTTLE_RING_ANGLE;
        let maxp = MAX_SHUTTLE_RING_ANGLE;
        let minv = 0.0_f64;

        if angle > 0.0 {
            let maxv = self.ticks.last().copied().unwrap_or(1000.0).ln();
            let scale = (maxv - minv) / (maxp - minp);
            (minv + scale * (angle - minp)).exp()
        } else {
            let maxv = (-self.ticks.first().copied().unwrap_or(-1000.0)).ln();
            let scale = (maxv - minv) / (maxp - minp);
            -((minv + scale * (angle.abs() - minp)).exp())
        }
    }

    /// 将速度倍率转换为动感环角度。
    pub fn multiplier_to_angle(&self, multiplier: f64, is_system_clock: bool) -> f64 {
        // 系统时钟恒为实时，角度固定在 ±15° 处
        if is_system_clock {
            return REALTIME_SHUTTLE_RING_ANGLE;
        }

        // 倍率在 [-1, 1] 内时反向线性映射回角度
        if multiplier.abs() <= 1.0 {
            return multiplier * REALTIME_SHUTTLE_RING_ANGLE;
        }

        // 超出实时倍率：先把倍率钳制到最快档，再按对数尺度反推角度
        let fastest = self.ticks.last().copied().unwrap_or(1000.0);
        let clamped = multiplier.clamp(-fastest, fastest);

        let minp = REALTIME_SHUTTLE_RING_ANGLE;
        let maxp = MAX_SHUTTLE_RING_ANGLE;
        let minv = 0.0_f64;

        if clamped > 0.0 {
            let maxv = fastest.ln();
            let scale = (maxv - minv) / (maxp - minp);
            (clamped.ln() - minv) / scale + minp
        } else {
            let maxv = (-self.ticks.first().copied().unwrap_or(-1000.0)).ln();
            let scale = (maxv - minv) / (maxp - minp);
            -((clamped.abs().ln() - minv) / scale + minp)
        }
    }

    /// 获取给定倍率对应的典型倍率索引。
    pub fn get_typical_multiplier_index(&self, multiplier: f64) -> usize {
        // 二分查找；未命中时返回插入点作为相邻典型档位索引
        match self.ticks.binary_search_by(|t| {
            t.partial_cmp(&multiplier).unwrap_or(std::cmp::Ordering::Equal)
        }) {
            Ok(idx) => idx,
            Err(idx) => idx,
        }
    }
}

/// 动画 widget 视图模型。
///
/// 通过播放/暂停、速度倍率与动感环控制时间 playback。
#[derive(Debug, Clone)]
pub struct AnimationViewModel {
    /// 动画是否正在播放。
    pub is_playing: bool,
    /// 当前速度倍率（1.0 = 实时）。
    pub multiplier: f64,
    /// 当前动感环角度（度）。
    pub shuttle_ring_angle: f64,
    /// 时钟是否处于系统时钟模式。
    pub is_system_clock: bool,
    /// 当前时间，以自 J2000 纪元起算的秒数表示。
    pub current_time: f64,
    /// 动感环转换器。
    pub shuttle_ring: ShuttleRing,
}

impl Default for AnimationViewModel {
    /// 默认暂停、实时倍率 1.0、动感环居中于实时角度、非系统时钟。
    fn default() -> Self {
        Self {
            is_playing: false,
            multiplier: 1.0,
            shuttle_ring_angle: REALTIME_SHUTTLE_RING_ANGLE,
            is_system_clock: false,
            current_time: 0.0,
            shuttle_ring: ShuttleRing::default(),
        }
    }
}

impl AnimationViewModel {
    /// 创建一个新的动画视图模型。
    pub fn new() -> Self {
        Self::default()
    }

    /// 切换播放/暂停。
    pub fn toggle_play(&mut self) {
        // 直接翻转播放标志，不改动倍率与动感环状态
        self.is_playing = !self.is_playing;
    }

    /// 播放动画。
    pub fn play(&mut self) {
        self.is_playing = true;
    }

    /// 暂停动画。
    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    /// 反向播放。
    pub fn play_reverse(&mut self) {
        // 反向播放需保持播放且倍率为负；已是负值则维持不变
        self.is_playing = true;
        if self.multiplier > 0.0 {
            self.multiplier = -self.multiplier;
        }
    }

    /// 正向播放。
    pub fn play_forward(&mut self) {
        // 正向播放保持播放且倍率为正；原为倒放则取反回正
        self.is_playing = true;
        if self.multiplier < 0.0 {
            self.multiplier = -self.multiplier;
        }
    }

    /// 设置速度倍率。
    pub fn set_multiplier(&mut self, multiplier: f64) {
        // 存下倍率并反向同步动感环角度，保持两者一致
        self.multiplier = multiplier;
        self.shuttle_ring_angle = self.shuttle_ring.multiplier_to_angle(multiplier, self.is_system_clock);
    }

    /// 设置动感环角度。
    pub fn set_shuttle_ring_angle(&mut self, angle: f64) {
        // 先把角度钳制到可动范围，再由它推导出对应倍率
        let clamped = angle.clamp(-MAX_SHUTTLE_RING_ANGLE, MAX_SHUTTLE_RING_ANGLE);
        self.shuttle_ring_angle = clamped;
        self.multiplier = self.shuttle_ring.angle_to_multiplier(clamped);
    }

    /// 设置系统时钟模式。
    pub fn set_system_clock(&mut self, enabled: bool) {
        // 开启系统时钟时把动感环归回实时角度，避免手动倍率干扰
        self.is_system_clock = enabled;
        if enabled {
            self.shuttle_ring_angle = REALTIME_SHUTTLE_RING_ANGLE;
        }
    }

    /// 更新当前时间。
    pub fn update_time(&mut self, time: f64) {
        self.current_time = time;
    }

    /// 将当前时间格式化为日期字符串。
    pub fn format_date(&self) -> String {
        // 简化处理：将自 J2000 起算的秒数转换为日期字符串
        // J2000 纪元为 2000-01-01 12:00:00 TT
        let j2000_unix = 946728000.0; // J2000 的 Unix 时间戳
        let unix_time = self.current_time + j2000_unix;
        let days = (unix_time / 86400.0).floor() as i64;

        // 简单的日期计算（近似）
        // 以 365 天为年、 30 天为月做粗粒度拆解，不追求日历精确性
        let years_since_1970 = days / 365;
        let year = 1970 + years_since_1970;
        let day_of_year = days % 365;
        let month = (day_of_year / 30).clamp(0, 11) as usize;
        let day = (day_of_year % 30) + 1;

        format!("{} {}, {}", MONTH_NAMES[month], day, year)
    }

    /// 将当前时间格式化为时间字符串。
    pub fn format_time(&self) -> String {
        // 把纪元秒平移到 Unix 基准，再取当日内的秒数拆分时分秒
        let j2000_unix = 946728000.0;
        let unix_time = self.current_time + j2000_unix;
        let seconds_in_day = unix_time % 86400.0;
        let hours = (seconds_in_day / 3600.0).floor() as i32;
        let minutes = ((seconds_in_day % 3600.0) / 60.0).floor() as i32;
        let seconds = (seconds_in_day % 60.0).floor() as i32;

        format!("{:02}:{:02}:{:02} UTC", hours, minutes, seconds)
    }

    /// 获取倍率的显示字符串。
    pub fn multiplier_string(&self) -> String {
        // ±1 显为整数倍，小于 1 保留两位小数，其余取整，均附 x 后缀
        if self.multiplier == 1.0 {
            "1x".to_string()
        } else if self.multiplier == -1.0 {
            "-1x".to_string()
        } else if self.multiplier.abs() < 1.0 {
            format!("{:.2}x", self.multiplier)
        } else {
            format!("{:.0}x", self.multiplier)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shuttle_ring_default() {
        let ring = ShuttleRing::default();
        // 默认 16 档刻度，首尾分别为负（倒放）与正（快进）
        assert_eq!(ring.ticks.len(), 16);
        assert!(ring.ticks[0] < 0.0);
        assert!(ring.ticks[15] > 0.0);
    }

    #[test]
    fn test_shuttle_ring_angle_to_multiplier_linear() {
        let ring = ShuttleRing::default();
        // 在范围 [-15, 15] 内
        assert!((ring.angle_to_multiplier(0.0)).abs() < 1e-10);
        assert!((ring.angle_to_multiplier(15.0) - 1.0).abs() < 1e-10);
        assert!((ring.angle_to_multiplier(-15.0) - (-1.0)).abs() < 1e-10);
        assert!((ring.angle_to_multiplier(7.5) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_shuttle_ring_angle_to_multiplier_log() {
        let ring = ShuttleRing::default();
        // 在最大角度时，应接近最大刻度
        // 105° 超出 ±15° 实时区，走对数尺度，倍率应远大于 100
        let max_mult = ring.angle_to_multiplier(MAX_SHUTTLE_RING_ANGLE);
        assert!(max_mult > 100.0);

        // 对称的负角度对应负向大倍率（倒放快进）
        let min_mult = ring.angle_to_multiplier(-MAX_SHUTTLE_RING_ANGLE);
        assert!(min_mult < -100.0);
    }

    #[test]
    fn test_shuttle_ring_multiplier_to_angle() {
        let ring = ShuttleRing::default();
        // 倍率 1.0 应得到角度 15
        assert!((ring.multiplier_to_angle(1.0, false) - 15.0).abs() < 1e-10);
        assert!((ring.multiplier_to_angle(-1.0, false) - (-15.0)).abs() < 1e-10);
        assert!((ring.multiplier_to_angle(0.5, false) - 7.5).abs() < 1e-10);
    }

    #[test]
    fn test_shuttle_ring_system_clock() {
        let ring = ShuttleRing::default();
        // 系统时钟总返回实时角度
        assert!((ring.multiplier_to_angle(100.0, true) - REALTIME_SHUTTLE_RING_ANGLE).abs() < 1e-10);
    }

    #[test]
    fn test_shuttle_ring_roundtrip() {
        let ring = ShuttleRing::default();
        // 角度→倍率→角度应近似回环（对数尺度存在微小舍入误差）
        for angle in [-100.0, -50.0, -15.0, 0.0, 15.0, 50.0, 100.0] {
            let mult = ring.angle_to_multiplier(angle);
            let angle_back = ring.multiplier_to_angle(mult, false);
            assert!((angle - angle_back).abs() < 0.1, "angle {} -> mult {} -> angle {}", angle, mult, angle_back);
        }
    }

    #[test]
    fn test_animation_view_model_default() {
        let vm = AnimationViewModel::new();
        assert!(!vm.is_playing);
        assert_eq!(vm.multiplier, 1.0);
        assert!(!vm.is_system_clock);
    }

    #[test]
    fn test_animation_toggle_play() {
        let mut vm = AnimationViewModel::new();
        assert!(!vm.is_playing);
        vm.toggle_play();
        assert!(vm.is_playing);
        vm.toggle_play();
        assert!(!vm.is_playing);
    }

    #[test]
    fn test_animation_play_reverse() {
        let mut vm = AnimationViewModel::new();
        // 正倍率下反向播放应翻为负值且进入播放
        vm.multiplier = 5.0;
        vm.play_reverse();
        assert!(vm.is_playing);
        assert!(vm.multiplier < 0.0);
    }

    #[test]
    fn test_animation_play_forward() {
        let mut vm = AnimationViewModel::new();
        vm.multiplier = -5.0;
        vm.play_forward();
        assert!(vm.is_playing);
        assert!(vm.multiplier > 0.0);
    }

    #[test]
    fn test_animation_set_multiplier() {
        let mut vm = AnimationViewModel::new();
        // 设定 10x 后，动感环角度应随之超过实时基准 15°
        vm.set_multiplier(10.0);
        assert_eq!(vm.multiplier, 10.0);
        assert!(vm.shuttle_ring_angle > REALTIME_SHUTTLE_RING_ANGLE);
    }

    #[test]
    fn test_animation_set_shuttle_ring_angle() {
        let mut vm = AnimationViewModel::new();
        vm.set_shuttle_ring_angle(50.0);
        assert_eq!(vm.shuttle_ring_angle, 50.0);
        assert!(vm.multiplier > 1.0);
    }

    #[test]
    fn test_animation_multiplier_string() {
        let mut vm = AnimationViewModel::new();
        assert_eq!(vm.multiplier_string(), "1x");
        vm.multiplier = -1.0;
        assert_eq!(vm.multiplier_string(), "-1x");
        vm.multiplier = 10.0;
        assert_eq!(vm.multiplier_string(), "10x");
        vm.multiplier = 0.5;
        assert_eq!(vm.multiplier_string(), "0.50x");
    }

    #[test]
    fn test_animation_format_time() {
        let mut vm = AnimationViewModel::new();
        vm.current_time = 0.0; // J2000 纪元 = 2000-01-01 12:00:00
        let time_str = vm.format_time();
        assert!(time_str.contains("UTC"));
    }

    #[test]
    fn test_typical_multiplier_index() {
        let ring = ShuttleRing::default();
        let idx = ring.get_typical_multiplier_index(1.0);
        assert!(idx < ring.ticks.len());
    }
}
