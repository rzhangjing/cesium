//! Ported from `packages/engine/Source/Scene/BingMapsStyle.js`.

/// The style of Bing Maps imagery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BingMapsStyle {
    /// Aerial photography.
    Aerial = 0,
    /// Aerial photography with labels.
    AerialWithLabels = 1,
    /// Aerial with labels on demand.
    AerialWithLabelsOnDemand = 2,
    /// Road map.
    Road = 3,
    /// Road map on demand.
    RoadOnDemand = 4,
    /// Dark canvas style.
    CanvasDark = 5,
    /// Light canvas style.
    CanvasLight = 6,
    /// Gray canvas style.
    CanvasGray = 7,
    /// Ordnance Survey.
    OrdnanceSurvey = 8,
    /// Collins Bart.
    CollinsBart = 9,
}
