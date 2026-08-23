//! Ported from `packages/engine/Source/Scene/BlendFunction.js`.

/// A blend function for compositing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BlendFunction {
    /// Zero.
    Zero = 0,
    /// One.
    One = 1,
    /// Source color.
    SourceColor = 2,
    /// One minus source color.
    OneMinusSourceColor = 3,
    /// Destination color.
    DestinationColor = 4,
    /// One minus destination color.
    OneMinusDestinationColor = 5,
    /// Source alpha.
    SourceAlpha = 6,
    /// One minus source alpha.
    OneMinusSourceAlpha = 7,
    /// Destination alpha.
    DestinationAlpha = 8,
    /// One minus destination alpha.
    OneMinusDestinationAlpha = 9,
}
