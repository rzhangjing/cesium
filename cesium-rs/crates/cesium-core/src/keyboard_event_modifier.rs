//! Ported from `packages/engine/Source/Core/KeyboardEventModifier.js`.

/// This enumerated type is for representing keyboard modifiers. These are keys
/// that are held down in addition to other event types.
///
/// `PartialOrd`/`Ord` are derived so that a modifier list can be sorted to
/// reproduce the CesiumJS `modifiers.toSorted()` key ordering (the derived
/// order matches the numeric discriminants `SHIFT=0 < CTRL=1 < ALT=2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum KeyboardEventModifier {
    /// Represents the shift key being held down.
    Shift = 0,
    /// Represents the control key being held down.
    Ctrl = 1,
    /// Represents the alt key being held down.
    Alt = 2,
}
