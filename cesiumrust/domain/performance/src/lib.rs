//! cesium-performance: Performance optimization utilities.
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping:
//! - Frame rate control
//! - Request scheduling
//! - Memory management

pub mod performance;

pub use performance::{
    FrameRateConfig, FrameRateController, MemoryBudget, MemoryTracker, RequestPriority,
    RequestScheduler, ScheduledRequest,
};
