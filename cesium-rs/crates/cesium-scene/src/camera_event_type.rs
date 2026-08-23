//! Ported from `packages/engine/Source/Scene/CameraEventType.js`.

/// The type of camera event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CameraEventType {
    /// Left mouse button drag.
    LeftDrag = 0,
    /// Middle mouse button drag.
    MiddleDrag = 1,
    /// Right mouse button drag.
    RightDrag = 2,
    /// Mouse wheel scroll.
    Wheel = 3,
    /// Touch pinch.
    Pinch = 4,
}
