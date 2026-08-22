//! Ported from `packages/engine/Source/Core/KeyboardEventModifier.js`.

/// This enumerated type is for representing keyboard modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum KeyboardEventModifier {
    /// Represents the shift key being held down.
    Shift = 0,
    /// Represents the control key being held down.
    Ctrl = 1,
    /// Represents the alt key being held down.
    Alt = 2,
}
