//! Performance optimization: frame rate control, request scheduling, memory management.
//!
//! Maps to CesiumJS performance features:
//! - `Scene/FrameRateController.js` (target FPS)
//! - Request scheduling and throttling
//! - Memory budget management

use std::collections::VecDeque;
use std::time::Instant;

/// Frame rate controller configuration.
#[derive(Debug, Clone)]
pub struct FrameRateConfig {
    /// Target frames per second.
    pub target_fps: f64,
    /// Minimum frame time (seconds).
    pub min_frame_time: f64,
    /// Maximum frame time (seconds).
    pub max_frame_time: f64,
    /// Whether to use vsync.
    pub vsync: bool,
    /// Whether to render on demand only.
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

/// Frame rate controller.
#[derive(Debug)]
pub struct FrameRateController {
    /// Configuration.
    pub config: FrameRateConfig,
    /// Last frame time.
    last_frame_time: Option<Instant>,
    /// Frame time history for averaging.
    frame_history: VecDeque<f64>,
    /// Maximum history size.
    history_size: usize,
    /// Whether a render is requested.
    render_requested: bool,
}

impl FrameRateController {
    /// Creates a new frame rate controller.
    pub fn new(config: FrameRateConfig) -> Self {
        Self {
            config,
            last_frame_time: None,
            frame_history: VecDeque::new(),
            history_size: 60,
            render_requested: true,
        }
    }

    /// Called at the start of each frame.
    /// Returns the delta time in seconds.
    pub fn begin_frame(&mut self) -> f64 {
        let now = Instant::now();
        let delta = match self.last_frame_time {
            Some(last) => now.duration_since(last).as_secs_f64(),
            None => 1.0 / self.config.target_fps,
        };
        self.last_frame_time = Some(now);

        // Clamp delta time
        let delta = delta.clamp(self.config.min_frame_time, self.config.max_frame_time);

        // Record history
        self.frame_history.push_back(delta);
        if self.frame_history.len() > self.history_size {
            self.frame_history.pop_front();
        }

        delta
    }

    /// Returns the average frame time.
    pub fn average_frame_time(&self) -> f64 {
        if self.frame_history.is_empty() {
            return 1.0 / self.config.target_fps;
        }
        let sum: f64 = self.frame_history.iter().sum();
        sum / self.frame_history.len() as f64
    }

    /// Returns the current FPS.
    pub fn current_fps(&self) -> f64 {
        1.0 / self.average_frame_time()
    }

    /// Requests a render on the next frame.
    pub fn request_render(&mut self) {
        self.render_requested = true;
    }

    /// Returns true if a render should occur this frame.
    pub fn should_render(&mut self) -> bool {
        if !self.config.render_on_demand {
            return true;
        }
        let should = self.render_requested;
        self.render_requested = false;
        should
    }

    /// Returns the target frame time in seconds.
    pub fn target_frame_time(&self) -> f64 {
        1.0 / self.config.target_fps
    }
}

/// Request priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum RequestPriority {
    /// Low priority (preload).
    Low = 0,
    /// Normal priority.
    #[default]
    Normal = 1,
    /// High priority (visible tiles).
    High = 2,
    /// Critical priority (immediate).
    Critical = 3,
}

/// A scheduled request.
#[derive(Debug, Clone)]
pub struct ScheduledRequest {
    /// Request ID.
    pub id: u64,
    /// Priority.
    pub priority: RequestPriority,
    /// Frame number when scheduled.
    pub frame_number: u64,
    /// Whether the request has been cancelled.
    pub cancelled: bool,
}

/// Request scheduler with throttling.
#[derive(Debug)]
pub struct RequestScheduler {
    /// Pending requests.
    pending: VecDeque<ScheduledRequest>,
    /// Maximum concurrent requests.
    pub max_concurrent: usize,
    /// Currently active request count.
    active_count: usize,
    /// Total requests processed.
    pub total_processed: u64,
    /// Next request ID.
    next_id: u64,
}

impl Default for RequestScheduler {
    fn default() -> Self {
        Self::new(6) // Default: 6 concurrent (browser limit per domain)
    }
}

impl RequestScheduler {
    /// Creates a new request scheduler.
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            pending: VecDeque::new(),
            max_concurrent,
            active_count: 0,
            total_processed: 0,
            next_id: 0,
        }
    }

    /// Schedules a new request.
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

    /// Cancels a request.
    pub fn cancel(&mut self, id: u64) {
        if let Some(req) = self.pending.iter_mut().find(|r| r.id == id) {
            req.cancelled = true;
        }
    }

    /// Gets the next request to process.
    pub fn next_request(&mut self) -> Option<ScheduledRequest> {
        if self.active_count >= self.max_concurrent {
            return None;
        }

        // Remove cancelled requests
        self.pending.retain(|r| !r.cancelled);

        // Find highest priority, oldest request
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

    /// Marks a request as complete.
    pub fn complete_request(&mut self) {
        if self.active_count > 0 {
            self.active_count -= 1;
        }
        self.total_processed += 1;
    }

    /// Returns the number of pending requests.
    pub fn pending_count(&self) -> usize {
        self.pending.iter().filter(|r| !r.cancelled).count()
    }

    /// Returns true if there are available slots.
    pub fn has_capacity(&self) -> bool {
        self.active_count < self.max_concurrent
    }
}

/// Memory budget configuration.
#[derive(Debug, Clone)]
pub struct MemoryBudget {
    /// Maximum texture memory in bytes.
    pub max_texture_bytes: u64,
    /// Maximum geometry memory in bytes.
    pub max_geometry_bytes: u64,
    /// Maximum tile cache size.
    pub max_tile_cache_entries: usize,
    /// Whether to automatically evict when over budget.
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

/// Memory usage tracker.
#[derive(Debug, Default)]
pub struct MemoryTracker {
    /// Current texture memory usage.
    pub texture_bytes: u64,
    /// Current geometry memory usage.
    pub geometry_bytes: u64,
    /// Number of cached tiles.
    pub tile_cache_count: usize,
    /// Peak texture usage.
    pub peak_texture_bytes: u64,
    /// Peak geometry usage.
    pub peak_geometry_bytes: u64,
}

impl MemoryTracker {
    /// Creates a new memory tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocates texture memory.
    pub fn allocate_texture(&mut self, bytes: u64) {
        self.texture_bytes += bytes;
        self.peak_texture_bytes = self.peak_texture_bytes.max(self.texture_bytes);
    }

    /// Frees texture memory.
    pub fn free_texture(&mut self, bytes: u64) {
        self.texture_bytes = self.texture_bytes.saturating_sub(bytes);
    }

    /// Allocates geometry memory.
    pub fn allocate_geometry(&mut self, bytes: u64) {
        self.geometry_bytes += bytes;
        self.peak_geometry_bytes = self.peak_geometry_bytes.max(self.geometry_bytes);
    }

    /// Frees geometry memory.
    pub fn free_geometry(&mut self, bytes: u64) {
        self.geometry_bytes = self.geometry_bytes.saturating_sub(bytes);
    }

    /// Returns total memory usage.
    pub fn total_bytes(&self) -> u64 {
        self.texture_bytes + self.geometry_bytes
    }

    /// Checks if over budget.
    pub fn is_over_budget(&self, budget: &MemoryBudget) -> bool {
        self.texture_bytes > budget.max_texture_bytes
            || self.geometry_bytes > budget.max_geometry_bytes
            || self.tile_cache_count > budget.max_tile_cache_entries
    }

    /// Returns bytes to evict to get under budget.
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
        // Initially requested
        assert!(controller.should_render());
        // After render, not requested
        assert!(!controller.should_render());
        // Request again
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

        let id1 = scheduler.schedule(RequestPriority::Normal, 1);
        let id2 = scheduler.schedule(RequestPriority::High, 1);

        assert_eq!(scheduler.pending_count(), 2);

        // Should get high priority first
        let req = scheduler.next_request().unwrap();
        assert_eq!(req.id, id2);
        assert_eq!(req.priority, RequestPriority::High);
    }

    #[test]
    fn test_request_scheduler_capacity() {
        let mut scheduler = RequestScheduler::new(1);

        scheduler.schedule(RequestPriority::Normal, 1);
        scheduler.schedule(RequestPriority::Normal, 1);

        // First request
        let req = scheduler.next_request();
        assert!(req.is_some());
        assert!(!scheduler.has_capacity());

        // No more capacity
        let req = scheduler.next_request();
        assert!(req.is_none());

        // Complete and try again
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

        // Can't go below zero
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
