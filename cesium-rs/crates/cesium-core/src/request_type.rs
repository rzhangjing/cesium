//! Ported from `packages/engine/Source/Core/RequestType.js`.

/// An enum identifying the type of request. Used for finer grained logging
/// and priority sorting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum RequestType {
    /// Terrain request.
    Terrain = 0,
    /// Imagery request.
    Imagery = 1,
    /// 3D Tiles request.
    Tiles3D = 2,
    /// Other request.
    Other = 3,
}
