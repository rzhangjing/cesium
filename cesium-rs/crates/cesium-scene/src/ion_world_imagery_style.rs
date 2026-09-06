//! Ported from `packages/engine/Source/Scene/IonWorldImageryStyle.js`.

/// The types of imagery provided by `createWorldImagery`.
///
/// Note: These values map directly to ion asset ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum IonWorldImageryStyle {
    /// Aerial imagery (ion asset 2).
    Aerial = 2,
    /// Aerial imagery with a road overlay (ion asset 3).
    AerialWithLabels = 3,
    /// Roads without additional imagery (ion asset 4).
    Road = 4,
}

impl IonWorldImageryStyle {
    /// Returns the integer value (ion asset id).
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// Creates from an integer value (ion asset id).
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            2 => Some(Self::Aerial),
            3 => Some(Self::AerialWithLabels),
            4 => Some(Self::Road),
            _ => None,
        }
    }
}
