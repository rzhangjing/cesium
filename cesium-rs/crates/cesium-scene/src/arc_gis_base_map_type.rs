//! Ported from `packages/engine/Source/Scene/ArcGisBaseMapType.js`.

/// The base map type for ArcGIS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ArcGisBaseMapType {
    /// Streets.
    Streets = 0,
    /// Satellite imagery.
    Satellite = 1,
    /// Hybrid (streets + satellite).
    Hybrid = 2,
    /// Topographic.
    Topographic = 3,
    /// Dark gray canvas.
    DarkGray = 4,
    /// Light gray canvas.
    LightGray = 5,
    /// National Geographic.
    NationalGeographic = 6,
    /// Oceans.
    Oceans = 7,
    /// Terrain with labels.
    Terrain = 8,
}
