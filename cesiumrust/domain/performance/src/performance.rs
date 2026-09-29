//! 性能优化：帧率控制、请求调度、内存管理。
//!
//! 映射到 CesiumJS 性能特性：
//! - `Scene/FrameRateController.js`（目标 FPS）
//! - 请求调度与限流
//! - 内存预算管理

use std::collections::VecDeque;
use std::time::Instant;

/// 帧率控制器配置。
#[derive(Debug, Clone)]
pub struct FrameRateConfig {
    /// 目标每秒帧数。
    pub target_fps: f64,
    /// 最小帧时间（秒）。
    pub min_frame_time: f64,
    /// 最大帧时间（秒）。
    pub max_frame_time: f64,
    /// 是否使用 vsync。
    pub vsync: bool,
    /// 是否仅在需要时渲染。
    pub render_on_demand: bool,
}

impl Default for FrameRateConfig {
    fn default() -> Self {
        Self {
            target_fps: 60.0,
            min_frame_time: 1.0 / 240.0,
            max_frame_time: 1.0 / 10.0,
            vsync: true,
            render_on_demand: false,
        }
    }
}

/// 帧率控制器。
#[derive(Debug)]
pub struct FrameRateController {
    /// 配置。
    pub config: FrameRateConfig,
    /// 上一帧时间。
    last_frame_time: Option<Instant>,
    /// 用于求平均的帧时间历史。
    frame_history: VecDeque<f64>,
    /// 最大历史长度。
    history_size: usize,
    /// 是否已请求渲染。
    render_requested: bool,
}

impl FrameRateController {
    /// 创建一个新的帧率控制器。
    pub fn new(config: FrameRateConfig) -> Self {
        Self {
            config,
            last_frame_time: None,
            frame_history: VecDeque::new(),
            history_size: 60,
            render_requested: true,
        }
    }

    /// 在每帧开始时调用。
    /// 返回以秒计的间隔时间（delta time）。
    pub fn begin_frame(&mut self) -> f64 {
        let now = Instant::now();
        let delta = match self.last_frame_time {
            Some(last) => now.duration_since(last).as_secs_f64(),
            None => 1.0 / self.config.target_fps,
        };
        self.last_frame_time = Some(now);

        // 将 delta time 夹取到范围内
        let delta = delta.clamp(self.config.min_frame_time, self.config.max_frame_time);

        // 记录历史
        self.frame_history.push_back(delta);
        if self.frame_history.len() > self.history_size {
            self.frame_history.pop_front();
        }

        delta
    }

    /// 返回平均帧时间。
    pub fn average_frame_time(&self) -> f64 {
        if self.frame_history.is_empty() {
            return 1.0 / self.config.target_fps;
        }
        let sum: f64 = self.frame_history.iter().sum();
        sum / self.frame_history.len() as f64
    }

    /// 返回当前 FPS。
    pub fn current_fps(&self) -> f64 {
        1.0 / self.average_frame_time()
    }

    /// 请求在下一帧渲染。
    pub fn request_render(&mut self) {
        self.render_requested = true;
    }

    /// 如果本帧应当渲染则返回 true。
    pub fn should_render(&mut self) -> bool {
        if !self.config.render_on_demand {
            return true;
        }
        let should = self.render_requested;
        self.render_requested = false;
        should
    }

    /// 返回以秒计的目标帧时间。
    pub fn target_frame_time(&self) -> f64 {
        1.0 / self.config.target_fps
    }
}

/// 请求优先级级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum RequestPriority {
    /// 低优先级（预加载）。
    Low = 0,
    /// 普通优先级。
    #[default]
    Normal = 1,
    /// 高优先级（可见瓦片）。
    High = 2,
    /// 关键优先级（立即）。
    Critical = 3,
}

/// 一个已调度的请求。
#[derive(Debug, Clone)]
pub struct ScheduledRequest {
    /// 请求 ID。
    pub id: u64,
    /// 优先级。
    pub priority: RequestPriority,
    /// 调度时的帧编号。
    pub frame_number: u64,
    /// 请求是否已被取消。
    pub cancelled: bool,
}

/// 带限流的请求调度器。
#[derive(Debug)]
pub struct RequestScheduler {
    /// 待处理请求。
    pending: VecDeque<ScheduledRequest>,
    /// 最大并发请求数。
    pub max_concurrent: usize,
    /// 当前活动请求数。
    active_count: usize,
    /// 已处理请求总数。
    pub total_processed: u64,
    /// 下一个请求 ID。
    next_id: u64,
}

impl Default for RequestScheduler {
    fn default() -> Self {
        Self::new(6) // 默认：6 个并发（浏览器每域名限制）
    }
}

impl RequestScheduler {
    /// 创建一个新的请求调度器。
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            pending: VecDeque::new(),
            max_concurrent,
            active_count: 0,
            total_processed: 0,
            next_id: 0,
        }
    }

    /// 调度一个新请求。
    pub fn schedule(&mut self, priority: RequestPriority, frame_number: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.pending.push_back(ScheduledRequest {
            id,
            priority,
            frame_number,
            cancelled: false,
        });
        id
    }

    /// 取消一个请求。
    pub fn cancel(&mut self, id: u64) {
        if let Some(req) = self.pending.iter_mut().find(|r| r.id == id) {
            req.cancelled = true;
        }
    }

    /// 获取下一个要处理的请求。
    pub fn next_request(&mut self) -> Option<ScheduledRequest> {
        if self.active_count >= self.max_concurrent {
            return None;
        }

        // 移除已取消的请求
        self.pending.retain(|r| !r.cancelled);

        // 找出优先级最高、最旧的请求
        let best_idx = self
            .pending
            .iter()
            .enumerate()
            .max_by_key(|(_, r)| (r.priority, std::cmp::Reverse(r.frame_number)))
            .map(|(i, _)| i)?;

        let request = self.pending.remove(best_idx)?;
        self.active_count += 1;
        Some(request)
    }

    /// 将一个请求标记为完成。
    pub fn complete_request(&mut self) {
        if self.active_count > 0 {
            self.active_count -= 1;
        }
        self.total_processed += 1;
    }

    /// 返回待处理请求的数量。
    pub fn pending_count(&self) -> usize {
        self.pending.iter().filter(|r| !r.cancelled).count()
    }

    /// 如果还有可用槽位则返回 true。
    pub fn has_capacity(&self) -> bool {
        self.active_count < self.max_concurrent
    }
}

/// 内存预算配置。
#[derive(Debug, Clone)]
pub struct MemoryBudget {
    /// 最大纹理内存（字节）。
    pub max_texture_bytes: u64,
    /// 最大几何内存（字节）。
    pub max_geometry_bytes: u64,
    /// 最大瓦片缓存大小。
    pub max_tile_cache_entries: usize,
    /// 超出预算时是否自动逐出。
    pub auto_evict: bool,
}

impl Default for MemoryBudget {
    fn default() -> Self {
        Self {
            max_texture_bytes: 512 * 1024 * 1024,  // 512 MB
            max_geometry_bytes: 256 * 1024 * 1024, // 256 MB
            max_tile_cache_entries: 1000,
            auto_evict: true,
        }
    }
}

/// 内存使用跟踪器。
#[derive(Debug, Default)]
pub struct MemoryTracker {
    /// 当前纹理内存使用量。
    pub texture_bytes: u64,
    /// 当前几何内存使用量。
    pub geometry_bytes: u64,
    /// 已缓存瓦片数。
    pub tile_cache_count: usize,
    /// 纹理使用峰值。
    pub peak_texture_bytes: u64,
    /// 几何使用峰值。
    pub peak_geometry_bytes: u64,
}

impl MemoryTracker {
    /// 创建一个新的内存跟踪器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 分配纹理内存。
    pub fn allocate_texture(&mut self, bytes: u64) {
        self.texture_bytes += bytes;
        self.peak_texture_bytes = self.peak_texture_bytes.max(self.texture_bytes);
    }

    /// 释放纹理内存。
    pub fn free_texture(&mut self, bytes: u64) {
        self.texture_bytes = self.texture_bytes.saturating_sub(bytes);
    }

    /// 分配几何内存。
    pub fn allocate_geometry(&mut self, bytes: u64) {
        self.geometry_bytes += bytes;
        self.peak_geometry_bytes = self.peak_geometry_bytes.max(self.geometry_bytes);
    }

    /// 释放几何内存。
    pub fn free_geometry(&mut self, bytes: u64) {
        self.geometry_bytes = self.geometry_bytes.saturating_sub(bytes);
    }

    /// 返回总内存使用量。
    pub fn total_bytes(&self) -> u64 {
        self.texture_bytes + self.geometry_bytes
    }

    /// 检查是否超出预算。
    pub fn is_over_budget(&self, budget: &MemoryBudget) -> bool {
        self.texture_bytes > budget.max_texture_bytes
            || self.geometry_bytes > budget.max_geometry_bytes
            || self.tile_cache_count > budget.max_tile_cache_entries
    }

    /// 返回为降到预算内需逐出的字节数。
    pub fn bytes_to_evict(&self, budget: &MemoryBudget) -> u64 {
        let texture_over = self.texture_bytes.saturating_sub(budget.max_texture_bytes);
        let geometry_over = self.geometry_bytes.saturating_sub(budget.max_geometry_bytes);
        texture_over + geometry_over
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_rate_config_default() {
        let config = FrameRateConfig::default();
        assert_eq!(config.target_fps, 60.0);
        assert!(config.vsync);
        assert!(!config.render_on_demand);
    }

    #[test]
    fn test_frame_rate_controller() {
        let controller = FrameRateController::new(FrameRateConfig::default());
        assert!((controller.target_frame_time() - 1.0 / 60.0).abs() < 1e-10);
    }

    #[test]
    fn test_should_render_always() {
        let mut controller = FrameRateController::new(FrameRateConfig {
            render_on_demand: false,
            ..Default::default()
        });
        assert!(controller.should_render());
        assert!(controller.should_render());
    }

    #[test]
    fn test_should_render_on_demand() {
        let mut controller = FrameRateController::new(FrameRateConfig {
            render_on_demand: true,
            ..Default::default()
        });
        // 初始时已请求
        assert!(controller.should_render());
        // 渲染后，未请求
        assert!(!controller.should_render());
        // 再次请求
        controller.request_render();
        assert!(controller.should_render());
    }

    #[test]
    fn test_request_priority_ordering() {
        assert!(RequestPriority::Critical > RequestPriority::High);
        assert!(RequestPriority::High > RequestPriority::Normal);
        assert!(RequestPriority::Normal > RequestPriority::Low);
    }

    #[test]
    fn test_request_scheduler() {
        let mut scheduler = RequestScheduler::new(2);
        assert!(scheduler.has_capacity());

        let _id1 = scheduler.schedule(RequestPriority::Normal, 1);
        let id2 = scheduler.schedule(RequestPriority::High, 1);

        assert_eq!(scheduler.pending_count(), 2);

        // 应先获得高优先级请求
        let req = scheduler.next_request().unwrap();
        assert_eq!(req.id, id2);
        assert_eq!(req.priority, RequestPriority::High);
    }

    #[test]
    fn test_request_scheduler_capacity() {
        let mut scheduler = RequestScheduler::new(1);

        scheduler.schedule(RequestPriority::Normal, 1);
        scheduler.schedule(RequestPriority::Normal, 1);

        // 第一个请求
        let req = scheduler.next_request();
        assert!(req.is_some());
        assert!(!scheduler.has_capacity());

        // 没有更多容量
        let req = scheduler.next_request();
        assert!(req.is_none());

        // 完成一个后重试
        scheduler.complete_request();
        assert!(scheduler.has_capacity());
    }

    #[test]
    fn test_request_cancel() {
        let mut scheduler = RequestScheduler::new(2);
        let id = scheduler.schedule(RequestPriority::Normal, 1);
        scheduler.cancel(id);

        assert_eq!(scheduler.pending_count(), 0);
        assert!(scheduler.next_request().is_none());
    }

    #[test]
    fn test_memory_budget_default() {
        let budget = MemoryBudget::default();
        assert_eq!(budget.max_texture_bytes, 512 * 1024 * 1024);
        assert!(budget.auto_evict);
    }

    #[test]
    fn test_memory_tracker() {
        let mut tracker = MemoryTracker::new();
        assert_eq!(tracker.total_bytes(), 0);

        tracker.allocate_texture(1000);
        tracker.allocate_geometry(500);

        assert_eq!(tracker.texture_bytes, 1000);
        assert_eq!(tracker.geometry_bytes, 500);
        assert_eq!(tracker.total_bytes(), 1500);
        assert_eq!(tracker.peak_texture_bytes, 1000);
    }

    #[test]
    fn test_memory_free() {
        let mut tracker = MemoryTracker::new();
        tracker.allocate_texture(1000);
        tracker.free_texture(400);
        assert_eq!(tracker.texture_bytes, 600);

        // 不能低于零
        tracker.free_texture(1000);
        assert_eq!(tracker.texture_bytes, 0);
    }

    #[test]
    fn test_memory_over_budget() {
        let budget = MemoryBudget {
            max_texture_bytes: 1000,
            max_geometry_bytes: 500,
            ..Default::default()
        };

        let mut tracker = MemoryTracker::new();
        assert!(!tracker.is_over_budget(&budget));

        tracker.allocate_texture(1500);
        assert!(tracker.is_over_budget(&budget));
        assert_eq!(tracker.bytes_to_evict(&budget), 500);
    }

    #[test]
    fn test_peak_tracking() {
        let mut tracker = MemoryTracker::new();
        tracker.allocate_texture(1000);
        tracker.allocate_texture(500);
        tracker.free_texture(800);

        assert_eq!(tracker.texture_bytes, 700);
        assert_eq!(tracker.peak_texture_bytes, 1500);
    }
}
