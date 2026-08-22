//! Ported from `packages/engine/Source/Core/InterpolationType.js`.

/// An enum describing the type of interpolation used in a glTF animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InterpolationType {
    Step = 0,
    Linear = 1,
    CubicSpline = 2,
}
