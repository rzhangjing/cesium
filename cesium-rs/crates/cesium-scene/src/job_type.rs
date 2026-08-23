//! Ported from `packages/engine/Source/Scene/JobType.js`.

/// Type of job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum JobType {
    /// Default.
    Default = 0,
    /// Terrain.
    Terrain = 1,
    /// Imagery.
    Imagery = 2,
}
