//! Built-in material GLSL shader sources.
//!
//! Maps to CesiumJS `Source/Shaders/Materials/*.glsl`. These are the exact
//! shader sources referenced by the built-in material definitions in
//! `Scene/Material.js` (imported as `Shaders/Materials/*.js` there).
//!
//! The domain layer stores and composes these sources verbatim (pure text
//! processing, exactly like CesiumJS's `Material` does). The bevy-render
//! adapter is responsible for translating them to WGSL at render time.

/// `Shaders/Materials/AspectRampMaterial.glsl`
pub const ASPECT_RAMP_MATERIAL: &str = include_str!("../shaders/AspectRampMaterial.glsl");
/// `Shaders/Materials/BumpMapMaterial.glsl`
pub const BUMP_MAP_MATERIAL: &str = include_str!("../shaders/BumpMapMaterial.glsl");
/// `Shaders/Materials/CheckerboardMaterial.glsl`
pub const CHECKERBOARD_MATERIAL: &str = include_str!("../shaders/CheckerboardMaterial.glsl");
/// `Shaders/Materials/DotMaterial.glsl`
pub const DOT_MATERIAL: &str = include_str!("../shaders/DotMaterial.glsl");
/// `Shaders/Materials/ElevationBandMaterial.glsl`
pub const ELEVATION_BAND_MATERIAL: &str = include_str!("../shaders/ElevationBandMaterial.glsl");
/// `Shaders/Materials/ElevationContourMaterial.glsl`
pub const ELEVATION_CONTOUR_MATERIAL: &str =
    include_str!("../shaders/ElevationContourMaterial.glsl");
/// `Shaders/Materials/ElevationRampMaterial.glsl`
pub const ELEVATION_RAMP_MATERIAL: &str = include_str!("../shaders/ElevationRampMaterial.glsl");
/// `Shaders/Materials/FadeMaterial.glsl`
pub const FADE_MATERIAL: &str = include_str!("../shaders/FadeMaterial.glsl");
/// `Shaders/Materials/GridMaterial.glsl`
pub const GRID_MATERIAL: &str = include_str!("../shaders/GridMaterial.glsl");
/// `Shaders/Materials/NormalMapMaterial.glsl`
pub const NORMAL_MAP_MATERIAL: &str = include_str!("../shaders/NormalMapMaterial.glsl");
/// `Shaders/Materials/PolylineArrowMaterial.glsl`
pub const POLYLINE_ARROW_MATERIAL: &str = include_str!("../shaders/PolylineArrowMaterial.glsl");
/// `Shaders/Materials/PolylineDashMaterial.glsl`
pub const POLYLINE_DASH_MATERIAL: &str = include_str!("../shaders/PolylineDashMaterial.glsl");
/// `Shaders/Materials/PolylineGlowMaterial.glsl`
pub const POLYLINE_GLOW_MATERIAL: &str = include_str!("../shaders/PolylineGlowMaterial.glsl");
/// `Shaders/Materials/PolylineOutlineMaterial.glsl`
pub const POLYLINE_OUTLINE_MATERIAL: &str =
    include_str!("../shaders/PolylineOutlineMaterial.glsl");
/// `Shaders/Materials/RimLightingMaterial.glsl`
pub const RIM_LIGHTING_MATERIAL: &str = include_str!("../shaders/RimLightingMaterial.glsl");
/// `Shaders/Materials/SlopeRampMaterial.glsl`
pub const SLOPE_RAMP_MATERIAL: &str = include_str!("../shaders/SlopeRampMaterial.glsl");
/// `Shaders/Materials/StripeMaterial.glsl`
pub const STRIPE_MATERIAL: &str = include_str!("../shaders/StripeMaterial.glsl");
/// `Shaders/Materials/Water.glsl`
pub const WATER_MATERIAL: &str = include_str!("../shaders/Water.glsl");
/// `Shaders/Materials/WaterMaskMaterial.glsl`
pub const WATER_MASK_MATERIAL: &str = include_str!("../shaders/WaterMaskMaterial.glsl");
