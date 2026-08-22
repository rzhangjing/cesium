//! Ported from packages/engine/Source/Core/isBitSet.js

/// @private
///
/// Port of CesiumJS `isBitSet(bits, mask)`.
#[inline]
#[must_use]
pub fn is_bit_set(bits: u32, mask: u32) -> bool {
    (bits & mask) != 0
}
