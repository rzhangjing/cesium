//! Ported from `packages/engine/Source/Core/GeocodeType.js`.

/// The type of geocoding to be performed by a `GeocoderService`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum GeocodeType {
    /// Perform a search where the input is considered complete.
    Search = 0,
    /// Perform an auto-complete using partial input.
    Autocomplete = 1,
}
