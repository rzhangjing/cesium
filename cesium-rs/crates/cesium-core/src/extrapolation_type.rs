//! Ported from `packages/engine/Source/Core/ExtrapolationType.js`.

/// Constants to determine how an interpolated value is extrapolated
/// when querying outside the bounds of available data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ExtrapolationType {
    /// No extrapolation occurs.
    None = 0,
    /// The first or last value is used when outside the range of sample data.
    Hold = 1,
    /// The value is extrapolated.
    Extrapolate = 2,
}
