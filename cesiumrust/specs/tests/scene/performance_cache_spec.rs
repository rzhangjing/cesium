//! Performance + Cache specs
//! Ported from CesiumJS Scene/FrameRateControllerSpec.js + ResourceCacheSpec.js

use cesium_performance::{
    CacheStatistics, FrameRateConfig, FrameRateController, LruCache, MemoryBudget, MemoryTracker,
    RequestPriority, RequestScheduler,
};

// ==================== FrameRateConfig ====================

#[test]
fn frame_rate_config_defaults() {
    let config = FrameRateConfig::default();
    assert!((config.target_fps - 60.0).abs() < 1e-10);
    assert!(config.vsync);
    assert!(!config.render_on_demand);
    assert!((config.min_frame_time - 1.0 / 240.0).abs() < 1e-10);
    assert!((config.max_frame_time - 1.0 / 10.0).abs() < 1e-10);
}

#[test]
fn frame_rate_controller_target_frame_time() {
    let controller = FrameRateController::new(FrameRateConfig {
        target_fps: 30.0,
        ..Default::default()
    });
    assert!((controller.target_frame_time() - 1.0 / 30.0).abs() < 1e-10);
}

#[test]
fn frame_rate_controller_render_always() {
    let mut controller = FrameRateController::new(FrameRateConfig {
        render_on_demand: false,
        ..Default::default()
    });
    // Always renders regardless of request state
    assert!(controller.should_render());
    assert!(controller.should_render());
    assert!(controller.should_render());
}

#[test]
fn frame_rate_controller_render_on_demand() {
    let mut controller = FrameRateController::new(FrameRateConfig {
        render_on_demand: true,
        ..Default::default()
    });
    // Initially requested
    assert!(controller.should_render());
    // After consumed, no more
    assert!(!controller.should_render());
    assert!(!controller.should_render());
    // Request again
    controller.request_render();
    assert!(controller.should_render());
    assert!(!controller.should_render());
}

#[test]
fn frame_rate_controller_average_without_history() {
    let controller = FrameRateController::new(FrameRateConfig::default());
    // Without frames, average = 1/target_fps
    assert!((controller.average_frame_time() - 1.0 / 60.0).abs() < 1e-10);
    assert!((controller.current_fps() - 60.0).abs() < 1e-10);
}

// ==================== RequestPriority ====================

#[test]
fn request_priority_ordering() {
    assert!(RequestPriority::Critical > RequestPriority::High);
    assert!(RequestPriority::High > RequestPriority::Normal);
    assert!(RequestPriority::Normal > RequestPriority::Low);
    assert_eq!(RequestPriority::default(), RequestPriority::Normal);
}

// ==================== RequestScheduler ====================

#[test]
fn request_scheduler_basic_scheduling() {
    let mut scheduler = RequestScheduler::new(6);
    assert!(scheduler.has_capacity());
    assert_eq!(scheduler.pending_count(), 0);

    let id = scheduler.schedule(RequestPriority::Normal, 1);
    assert_eq!(scheduler.pending_count(), 1);

    let req = scheduler.next_request().unwrap();
    assert_eq!(req.id, id);
    assert_eq!(req.priority, RequestPriority::Normal);
    assert_eq!(scheduler.pending_count(), 0);
}

#[test]
fn request_scheduler_priority_ordering() {
    let mut scheduler = RequestScheduler::new(6);
    let _low = scheduler.schedule(RequestPriority::Low, 1);
    let high = scheduler.schedule(RequestPriority::High, 1);
    let _normal = scheduler.schedule(RequestPriority::Normal, 1);
    let critical = scheduler.schedule(RequestPriority::Critical, 1);

    // Critical first, then High
    let r1 = scheduler.next_request().unwrap();
    assert_eq!(r1.id, critical);
    let r2 = scheduler.next_request().unwrap();
    assert_eq!(r2.id, high);
}

#[test]
fn request_scheduler_capacity_limit() {
    let mut scheduler = RequestScheduler::new(2);
    scheduler.schedule(RequestPriority::Normal, 1);
    scheduler.schedule(RequestPriority::Normal, 1);
    scheduler.schedule(RequestPriority::Normal, 1);

    // Can only take 2
    assert!(scheduler.next_request().is_some());
    assert!(scheduler.next_request().is_some());
    assert!(!scheduler.has_capacity());
    assert!(scheduler.next_request().is_none());

    // Complete one → capacity restored
    scheduler.complete_request();
    assert!(scheduler.has_capacity());
    assert!(scheduler.next_request().is_some());
}

#[test]
fn request_scheduler_cancel() {
    let mut scheduler = RequestScheduler::new(6);
    let id1 = scheduler.schedule(RequestPriority::Normal, 1);
    let _id2 = scheduler.schedule(RequestPriority::High, 1);

    scheduler.cancel(id1);
    assert_eq!(scheduler.pending_count(), 1);

    // Cancelled request not returned
    let req = scheduler.next_request().unwrap();
    assert_eq!(req.priority, RequestPriority::High);
}

#[test]
fn request_scheduler_total_processed() {
    let mut scheduler = RequestScheduler::new(6);
    scheduler.schedule(RequestPriority::Normal, 1);
    scheduler.schedule(RequestPriority::Normal, 1);

    scheduler.next_request();
    scheduler.complete_request();
    scheduler.next_request();
    scheduler.complete_request();

    assert_eq!(scheduler.total_processed, 2);
}

// ==================== MemoryBudget + MemoryTracker ====================

#[test]
fn memory_budget_defaults() {
    let budget = MemoryBudget::default();
    assert_eq!(budget.max_texture_bytes, 512 * 1024 * 1024);
    assert_eq!(budget.max_geometry_bytes, 256 * 1024 * 1024);
    assert_eq!(budget.max_tile_cache_entries, 1000);
    assert!(budget.auto_evict);
}

#[test]
fn memory_tracker_allocate_and_free() {
    let mut tracker = MemoryTracker::new();
    assert_eq!(tracker.total_bytes(), 0);

    tracker.allocate_texture(1000);
    tracker.allocate_geometry(500);
    assert_eq!(tracker.texture_bytes, 1000);
    assert_eq!(tracker.geometry_bytes, 500);
    assert_eq!(tracker.total_bytes(), 1500);

    tracker.free_texture(400);
    assert_eq!(tracker.texture_bytes, 600);
    // Saturating sub
    tracker.free_geometry(9999);
    assert_eq!(tracker.geometry_bytes, 0);
}

#[test]
fn memory_tracker_peak_tracking() {
    let mut tracker = MemoryTracker::new();
    tracker.allocate_texture(1000);
    tracker.allocate_texture(500);
    tracker.free_texture(800);
    assert_eq!(tracker.texture_bytes, 700);
    assert_eq!(tracker.peak_texture_bytes, 1500);
}

#[test]
fn memory_tracker_over_budget() {
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

// ==================== CacheStatistics ====================

#[test]
fn cache_statistics_hit_rate() {
    let mut stats = CacheStatistics::new();
    assert!((stats.hit_rate()).abs() < 1e-10); // 0/0 → 0

    stats.record_hit();
    stats.record_hit();
    stats.record_miss();
    assert!((stats.hit_rate() - 2.0 / 3.0).abs() < 1e-10);
}

#[test]
fn cache_statistics_byte_tracking() {
    let mut stats = CacheStatistics::new();
    stats.add_geometry(1000);
    stats.add_texture(2000);
    assert_eq!(stats.total_bytes, 3000);
    assert_eq!(stats.peak_bytes, 3000);
    assert_eq!(stats.geometry_byte_length, 1000);
    assert_eq!(stats.textures_byte_length, 2000);

    stats.remove_geometry(500);
    assert_eq!(stats.total_bytes, 2500);
    assert_eq!(stats.peak_bytes, 3000); // peak unchanged
}

#[test]
fn cache_statistics_clear() {
    let mut stats = CacheStatistics::new();
    stats.record_hit();
    stats.record_miss();
    stats.add_geometry(100);
    stats.clear();
    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
    assert_eq!(stats.total_bytes, 0);
}

// ==================== LruCache ====================

#[test]
fn lru_cache_basic_put_get() {
    let mut cache = LruCache::new(3);
    assert!(cache.is_empty());
    assert_eq!(cache.capacity(), 3);

    cache.put("a", 1);
    cache.put("b", 2);
    assert_eq!(cache.len(), 2);
    assert_eq!(cache.get(&"a"), Some(&1));
    assert_eq!(cache.get(&"b"), Some(&2));
    assert_eq!(cache.get(&"c"), None);
}

#[test]
fn lru_cache_eviction_order() {
    let mut cache = LruCache::new(2);
    cache.put("a", 1);
    cache.put("b", 2);
    // Access "a" to make it recently used
    cache.get(&"a");
    // Insert "c" → should evict "b" (LRU)
    cache.put("c", 3);
    assert_eq!(cache.len(), 2);
    assert!(cache.contains(&"a"));
    assert!(!cache.contains(&"b"));
    assert!(cache.contains(&"c"));
}

#[test]
fn lru_cache_update_existing() {
    let mut cache = LruCache::new(3);
    cache.put("x", 10);
    let old = cache.put("x", 20);
    assert_eq!(old, Some(10));
    assert_eq!(cache.len(), 1);
    assert_eq!(cache.get(&"x"), Some(&20));
}

#[test]
fn lru_cache_remove() {
    let mut cache = LruCache::new(3);
    cache.put("a", 1);
    cache.put("b", 2);
    let removed = cache.remove(&"a");
    assert_eq!(removed, Some(1));
    assert_eq!(cache.len(), 1);
    assert!(!cache.contains(&"a"));
}

#[test]
fn lru_cache_peek_does_not_update_order() {
    let mut cache = LruCache::new(2);
    cache.put("a", 1);
    cache.put("b", 2);
    // Peek "a" (does NOT make it recently used)
    assert_eq!(cache.peek(&"a"), Some(&1));
    // Insert "c" → "a" is still LRU → evicted
    cache.put("c", 3);
    assert!(!cache.contains(&"a"));
    assert!(cache.contains(&"b"));
    assert!(cache.contains(&"c"));
}

#[test]
fn lru_cache_clear() {
    let mut cache = LruCache::new(5);
    cache.put("a", 1);
    cache.put("b", 2);
    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

#[test]
fn lru_cache_stats_tracking() {
    let mut cache = LruCache::new(2);
    cache.put("a", 1);
    cache.get(&"a"); // hit
    cache.get(&"z"); // miss
    cache.put("b", 2);
    cache.put("c", 3); // eviction

    assert_eq!(cache.stats.hits, 1);
    assert_eq!(cache.stats.misses, 1);
    assert_eq!(cache.stats.evictions, 1);
    assert_eq!(cache.stats.insertions, 3);
}
