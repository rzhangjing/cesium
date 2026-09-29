//! 时间轴（Timeline）widget 模型。
//!
//! 映射到 CesiumJS `Timeline/Timeline.js`。

/// 以秒表示的时间轴刻度（tic scale）。
pub const TIMELINE_TIC_SCALES: &[f64] = &[
    0.001, 0.002, 0.005, 0.01, 0.02, 0.05, 0.1, 0.25, 0.5,
    1.0, 2.0, 5.0, 10.0, 15.0, 30.0,
    60.0,      // 1 分钟
    120.0,     // 2 分钟
    300.0,     // 5 分钟
    600.0,     // 10 分钟
    900.0,     // 15 分钟
    1800.0,    // 30 分钟
    3600.0,    // 1 小时
    7200.0,    // 2 小时
    14400.0,   // 4 小时
    21600.0,   // 6 小时
    43200.0,   // 12 小时
    86400.0,   // 24 小时
    172800.0,  // 2 天
    345600.0,  // 4 天
    604800.0,  // 7 天
    1296000.0, // 15 天
    2592000.0, // 30 天
    5184000.0, // 60 天
    7776000.0, // 90 天
    15552000.0,  // 180 天
    31536000.0,  // 365 天
    63072000.0,  // 2 年
    126144000.0, // 4 年
    157680000.0, // 5 年
    315360000.0, // 10 年
    630720000.0, // 20 年
    1261440000.0, // 40 年
    1576800000.0, // 50 年
    3153600000.0, // 100 年
    6307200000.0, // 200 年
    12614400000.0, // 400 年
    15768000000.0, // 500 年
    31536000000.0, // 1000 年
];

/// 一个带标签格式化的时间轴刻度。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineTicScale {
    /// 以秒表示的刻度值。
    pub seconds: f64,
    /// 是否为主刻度（major tic）。
    pub is_major: bool,
}

impl TimelineTicScale {
    /// 为给定的时间跳度与像素宽度获取合适的刻度。
    pub fn for_span_and_width(span_seconds: f64, width_pixels: f64, min_tic_spacing: f64) -> Self {
        let ideal_tic_seconds = span_seconds * min_tic_spacing / width_pixels;

        for &scale in TIMELINE_TIC_SCALES {
            if scale >= ideal_tic_seconds {
                let is_major = scale >= 3600.0; // 1 小时及以上为主刻度
                return Self { seconds: scale, is_major };
            }
        }

        // 回退到最大刻度
        Self {
            seconds: *TIMELINE_TIC_SCALES.last().unwrap(),
            is_major: true,
        }
    }

    /// 为该刻度格式化一个时间值。
    pub fn format_label(&self, time_seconds: f64) -> String {
        if self.seconds >= 86400.0 {
            // 天或更长 - 显示日期
            let days = (time_seconds / 86400.0).floor() as i64;
            format!("Day {}", days)
        } else if self.seconds >= 3600.0 {
            // 小时
            let hours = (time_seconds / 3600.0).floor() as i64;
            let minutes = ((time_seconds % 3600.0) / 60.0).floor() as i64;
            format!("{:02}:{:02}", hours, minutes)
        } else if self.seconds >= 60.0 {
            // 分钟
            let minutes = (time_seconds / 60.0).floor() as i64;
            let seconds = (time_seconds % 60.0).floor() as i64;
            format!("{:02}:{:02}", minutes, seconds)
        } else {
            // 秒
            format!("{:.1}s", time_seconds)
        }
    }
}

/// 时间轴上的一条轨道。
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineTrack {
    /// 轨道名称/标识符。
    pub name: String,
    /// 开始时间（自纪元起算的秒数）。
    pub start_time: f64,
    /// 结束时间（自纪元起算的秒数）。
    pub end_time: f64,
    /// 轨道颜色，RGBA [0, 1]。
    pub color: [f64; 4],
    /// 轨道高度（像素）。
    pub height: f64,
}

impl TimelineTrack {
    /// 创建一条新的时间轴轨道。
    pub fn new(name: impl Into<String>, start_time: f64, end_time: f64) -> Self {
        Self {
            name: name.into(),
            start_time,
            end_time,
            color: [0.5, 0.5, 1.0, 1.0],
            height: 10.0,
        }
    }

    /// 设置轨道颜色。
    pub fn with_color(mut self, color: [f64; 4]) -> Self {
        self.color = color;
        self
    }

    /// 设置轨道高度。
    pub fn with_height(mut self, height: f64) -> Self {
        self.height = height;
        self
    }

    /// 检查某个时间是否在此轨道内。
    pub fn contains_time(&self, time: f64) -> bool {
        time >= self.start_time && time <= self.end_time
    }

    /// 获取轨道的时长。
    pub fn duration(&self) -> f64 {
        self.end_time - self.start_time
    }
}

/// 时间轴上的一个高亮区间。
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineHighlightRange {
    /// 开始时间（秒）。
    pub start_time: f64,
    /// 结束时间（秒）。
    pub end_time: f64,
    /// 高亮颜色，RGBA [0, 1]。
    pub color: [f64; 4],
}

impl TimelineHighlightRange {
    /// 创建一个新的高亮区间。
    pub fn new(start_time: f64, end_time: f64) -> Self {
        Self {
            start_time,
            end_time,
            color: [1.0, 1.0, 0.0, 0.3],
        }
    }

    /// 设置高亮颜色。
    pub fn with_color(mut self, color: [f64; 4]) -> Self {
        self.color = color;
        self
    }

    /// 检查某个时间是否在此区间内。
    pub fn contains_time(&self, time: f64) -> bool {
        time >= self.start_time && time <= self.end_time
    }

    /// 获取时长。
    pub fn duration(&self) -> f64 {
        self.end_time - self.start_time
    }
}

/// 时间轴 widget 模型。
///
/// 显示并控制当前场景时间，带轨道与高亮。
#[derive(Debug, Clone)]
pub struct Timeline {
    /// 可见范围的开始时间（自纪元起算的秒数）。
    pub start_time: f64,
    /// 可见范围的结束时间（自纪元起算的秒数）。
    pub end_time: f64,
    /// 当前时间（自纪元起算的秒数）。
    pub current_time: f64,
    /// 时间轴上的轨道。
    pub tracks: Vec<TimelineTrack>,
    /// 高亮区间。
    pub highlight_ranges: Vec<TimelineHighlightRange>,
    /// 时间轴是否可见。
    pub show: bool,
}

impl Default for Timeline {
    fn default() -> Self {
        Self {
            start_time: 0.0,
            end_time: 86400.0, // 1 天
            current_time: 0.0,
            tracks: Vec::new(),
            highlight_ranges: Vec::new(),
            show: true,
        }
    }
}

impl Timeline {
    /// 创建一个新的、带时间范围的时间轴。
    pub fn new(start_time: f64, end_time: f64) -> Self {
        Self {
            start_time,
            end_time,
            current_time: start_time,
            ..Default::default()
        }
    }

    /// 获取可见时间跳度（秒）。
    pub fn span(&self) -> f64 {
        self.end_time - self.start_time
    }

    /// 设置可见时间范围。
    pub fn set_range(&mut self, start_time: f64, end_time: f64) {
        self.start_time = start_time;
        self.end_time = end_time;
        self.current_time = self.current_time.clamp(start_time, end_time);
    }

    /// 设置当前时间。
    pub fn set_current_time(&mut self, time: f64) {
        self.current_time = time.clamp(self.start_time, self.end_time);
    }

    /// 将一个时间转换为时间轴上的归一化位置 [0, 1]。
    pub fn time_to_position(&self, time: f64) -> f64 {
        let span = self.span();
        if span <= 0.0 {
            return 0.0;
        }
        ((time - self.start_time) / span).clamp(0.0, 1.0)
    }

    /// 将归一化位置 [0, 1] 转换为时间。
    pub fn position_to_time(&self, position: f64) -> f64 {
        let clamped = position.clamp(0.0, 1.0);
        self.start_time + clamped * self.span()
    }

    /// 添加一条轨道。
    pub fn add_track(&mut self, track: TimelineTrack) {
        self.tracks.push(track);
    }

    /// 按名称移除一条轨道。
    pub fn remove_track(&mut self, name: &str) -> bool {
        let len_before = self.tracks.len();
        self.tracks.retain(|t| t.name != name);
        self.tracks.len() < len_before
    }

    /// 添加一个高亮区间。
    pub fn add_highlight(&mut self, highlight: TimelineHighlightRange) {
        self.highlight_ranges.push(highlight);
    }

    /// 清除所有高亮。
    pub fn clear_highlights(&mut self) {
        self.highlight_ranges.clear();
    }

    /// 按一个倍数放大（以当前时间为中心）。
    pub fn zoom_in(&mut self, factor: f64) {
        let new_span = self.span() / factor.max(1.01);
        let center = self.current_time;
        self.start_time = center - new_span / 2.0;
        self.end_time = center + new_span / 2.0;
    }

    /// 按一个倍数缩小（以当前时间为中心）。
    pub fn zoom_out(&mut self, factor: f64) {
        let new_span = self.span() * factor.max(1.01);
        let center = self.current_time;
        self.start_time = center - new_span / 2.0;
        self.end_time = center + new_span / 2.0;
    }

    /// 按可见跳度的一个比例平移。
    pub fn pan(&mut self, fraction: f64) {
        let delta = self.span() * fraction;
        self.start_time += delta;
        self.end_time += delta;
    }

    /// 为当前视图获取合适的刻度。
    pub fn tic_scale(&self, width_pixels: f64) -> TimelineTicScale {
        TimelineTicScale::for_span_and_width(self.span(), width_pixels, 50.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_default() {
        let timeline = Timeline::default();
        assert_eq!(timeline.start_time, 0.0);
        assert_eq!(timeline.end_time, 86400.0);
        assert!(timeline.show);
    }

    #[test]
    fn test_timeline_new() {
        let timeline = Timeline::new(1000.0, 2000.0);
        assert_eq!(timeline.start_time, 1000.0);
        assert_eq!(timeline.end_time, 2000.0);
        assert_eq!(timeline.current_time, 1000.0);
    }

    #[test]
    fn test_timeline_span() {
        let timeline = Timeline::new(0.0, 3600.0);
        assert_eq!(timeline.span(), 3600.0);
    }

    #[test]
    fn test_timeline_time_to_position() {
        let timeline = Timeline::new(0.0, 100.0);
        assert!((timeline.time_to_position(0.0)).abs() < 1e-10);
        assert!((timeline.time_to_position(50.0) - 0.5).abs() < 1e-10);
        assert!((timeline.time_to_position(100.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_timeline_position_to_time() {
        let timeline = Timeline::new(0.0, 100.0);
        assert!((timeline.position_to_time(0.0)).abs() < 1e-10);
        assert!((timeline.position_to_time(0.5) - 50.0).abs() < 1e-10);
        assert!((timeline.position_to_time(1.0) - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_timeline_set_current_time_clamped() {
        let mut timeline = Timeline::new(0.0, 100.0);
        timeline.set_current_time(150.0);
        assert_eq!(timeline.current_time, 100.0);

        timeline.set_current_time(-50.0);
        assert_eq!(timeline.current_time, 0.0);
    }

    #[test]
    fn test_timeline_tracks() {
        let mut timeline = Timeline::default();
        timeline.add_track(TimelineTrack::new("track1", 0.0, 100.0));
        timeline.add_track(TimelineTrack::new("track2", 50.0, 150.0));

        assert_eq!(timeline.tracks.len(), 2);
        assert!(timeline.remove_track("track1"));
        assert_eq!(timeline.tracks.len(), 1);
        assert!(!timeline.remove_track("nonexistent"));
    }

    #[test]
    fn test_timeline_track_contains() {
        let track = TimelineTrack::new("test", 10.0, 20.0);
        assert!(track.contains_time(15.0));
        assert!(!track.contains_time(5.0));
        assert!(!track.contains_time(25.0));
        assert_eq!(track.duration(), 10.0);
    }

    #[test]
    fn test_timeline_highlights() {
        let mut timeline = Timeline::default();
        timeline.add_highlight(TimelineHighlightRange::new(0.0, 100.0));
        assert_eq!(timeline.highlight_ranges.len(), 1);

        timeline.clear_highlights();
        assert!(timeline.highlight_ranges.is_empty());
    }

    #[test]
    fn test_timeline_zoom() {
        let mut timeline = Timeline::new(0.0, 100.0);
        timeline.current_time = 50.0;

        timeline.zoom_in(2.0);
        assert!(timeline.span() < 100.0);
        assert!((timeline.current_time - 50.0).abs() < 1e-10);

        timeline.zoom_out(2.0);
        assert!(timeline.span() > 50.0);
    }

    #[test]
    fn test_timeline_pan() {
        let mut timeline = Timeline::new(0.0, 100.0);
        timeline.pan(0.1);
        assert!((timeline.start_time - 10.0).abs() < 1e-10);
        assert!((timeline.end_time - 110.0).abs() < 1e-10);
    }

    #[test]
    fn test_tic_scale() {
        let scale = TimelineTicScale::for_span_and_width(3600.0, 1000.0, 50.0);
        assert!(scale.seconds >= 1.0);

        let scale2 = TimelineTicScale::for_span_and_width(86400.0, 1000.0, 50.0);
        assert!(scale2.seconds >= 60.0);
    }

    #[test]
    fn test_tic_scale_format() {
        let scale = TimelineTicScale { seconds: 3600.0, is_major: true };
        let label = scale.format_label(7200.0);
        assert!(label.contains("02"));

        let scale2 = TimelineTicScale { seconds: 60.0, is_major: false };
        let label2 = scale2.format_label(125.0);
        assert!(label2.contains("02"));
    }
}
