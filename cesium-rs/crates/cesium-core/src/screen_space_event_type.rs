//! Ported from `packages/engine/Source/Core/ScreenSpaceEventType.js`.

/// This enumerated type is for classifying mouse events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ScreenSpaceEventType {
    LeftDown = 0,
    LeftUp = 1,
    LeftClick = 2,
    LeftDoubleClick = 3,
    RightDown = 5,
    RightUp = 6,
    RightClick = 7,
    MiddleDown = 10,
    MiddleUp = 11,
    MiddleClick = 12,
    MouseMove = 15,
    Wheel = 16,
    PinchStart = 17,
    PinchEnd = 18,
    PinchMove = 19,
}
