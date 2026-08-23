//! Ported from `packages/engine/Source/Scene/ModelAnimationState.js`.

/// The state of a model animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ModelAnimationState {
    /// Animation is stopped.
    Stopped = 0,
    /// Animation is starting.
    Starting = 1,
    /// Animation is playing.
    Animating = 2,
    /// Animation is stopping.
    Stopping = 3,
}
