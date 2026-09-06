//! Ported from `packages/engine/Source/Scene/JobScheduler.js`.

/// Job scheduler.
///
/// Manages frame-budgeted job execution for loading and processing.
pub struct JobScheduler {
    /// The maximum time per frame in milliseconds.
    pub maximum_time_per_frame: f64,
    /// Whether the scheduler is active.
    pub active: bool,
}

impl JobScheduler {
    /// Creates a new JobScheduler.
    pub fn new() -> Self { Self { maximum_time_per_frame: 5.0, active: true } }
}

impl Default for JobScheduler {
    fn default() -> Self { Self::new() }
}
