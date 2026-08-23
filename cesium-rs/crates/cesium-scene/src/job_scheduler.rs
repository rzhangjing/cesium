//! Ported from `packages/engine/Source/Scene/JobScheduler.js`.

/// Schedules jobs.
pub struct JobScheduler {
    _private: (),
}

impl JobScheduler {
    /// Creates a new JobScheduler.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for JobScheduler {
    fn default() -> Self { Self::new() }
}
