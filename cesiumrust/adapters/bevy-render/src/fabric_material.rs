//! Fabric procedural material adapter (bevy-render).
//!
//! Bridges the domain [`cesium_material::Material`] (a Fabric material:
//! assembled GLSL source + uniform values) to a native Bevy/WGSL procedural
//! material so it can be rendered without a runtime GLSL→WGSL transpiler.
//!
//! Maps to CesiumJS `Scene/Material.js` rendering path: the domain layer does
//! the exact textual assembly CesiumJS performs, and this adapter provides the
//! GPU-side evaluation of the same built-in procedural patterns (see
//! `shaders/fabric_material.wgsl`, a faithful port of
//! `Source/Shaders/Materials/*.glsl`).
//!
//! Covering all 21 CesiumJS built-in procedural material types:
//! Color(0)..Fade(6) and PolylineArrow(7)..WaterMask(20).

use bevy::asset::load_internal_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Shader, ShaderRef, ShaderType};
use cesium_material::{Material as DomainMaterial, UniformValue};
use std::collections::BTreeMap;

/// Strong handle to the embedded Fabric material WGSL shader.
///
/// The shader is compiled into the crate via [`load_internal_asset!`] so the
/// adapter works without an external `assets/` directory (the application
/// crate does not need to copy the `.wgsl` file).
pub const FABRIC_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0x4641_4252_4943_4D41_5445_5249_414C);

/// The procedural pattern selector. Values match the `kind` switch in
/// `shaders/fabric_material.wgsl`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum FabricKind {
    /// Solid colour (`Color` material).
    #[default]
    Color = 0,
    /// Tiled image (`Image` material).
    Image = 1,
    /// Checkerboard (`Checkerboard` material).
    Checkerboard = 2,
    /// Stripes (`Stripe` material).
    Stripe = 3,
    /// Grid lines (`Grid` material).
    Grid = 4,
    /// Dots (`Dot` material).
    Dot = 5,
    /// Distance fade (`Fade` material).
    Fade = 6,
    /// Arrow head on polyline (`PolylineArrow` material).
    PolylineArrow = 7,
    /// Dashed polyline (`PolylineDash` material).
    PolylineDash = 8,
    /// Glowing polyline (`PolylineGlow` material).
    PolylineGlow = 9,
    /// Outlined polyline (`PolylineOutline` material).
    PolylineOutline = 10,
    /// Contour lines by elevation (`ElevationContour` material).
    ElevationContour = 11,
    /// Color ramp by elevation (`ElevationRamp` material).
    ElevationRamp = 12,
    /// Color ramp by slope aspect (`AspectRamp` material).
    AspectRamp = 13,
    /// Color ramp by slope steepness (`SlopeRamp` material).
    SlopeRamp = 14,
    /// Normal mapping (`NormalMap` material).
    NormalMap = 15,
    /// Bump mapping (`BumpMap` material).
    BumpMap = 16,
    /// Animated water surface (`Water` material).
    Water = 17,
    /// Rim lighting effect (`RimLighting` material).
    RimLighting = 18,
    /// Discrete elevation bands (`ElevationBand` material).
    ElevationBand = 19,
    /// Water/land mask colouring (`WaterMask` material).
    WaterMask = 20,
}

impl FabricKind {
    /// Maps a CesiumJS built-in material type name to a [`FabricKind`].
    /// Unknown / custom types fall back to [`FabricKind::Color`].
    pub fn from_type_name(type_name: &str) -> Self {
        match type_name {
            "Color" => FabricKind::Color,
            "Image" => FabricKind::Image,
            "Checkerboard" => FabricKind::Checkerboard,
            "Stripe" => FabricKind::Stripe,
            "Grid" => FabricKind::Grid,
            "Dot" => FabricKind::Dot,
            "Fade" => FabricKind::Fade,
            "PolylineArrow" => FabricKind::PolylineArrow,
            "PolylineDash" => FabricKind::PolylineDash,
            "PolylineGlow" => FabricKind::PolylineGlow,
            "PolylineOutline" => FabricKind::PolylineOutline,
            "ElevationContour" => FabricKind::ElevationContour,
            "ElevationRamp" => FabricKind::ElevationRamp,
            "AspectRamp" => FabricKind::AspectRamp,
            "SlopeRamp" => FabricKind::SlopeRamp,
            "NormalMap" => FabricKind::NormalMap,
            "BumpMap" => FabricKind::BumpMap,
            "Water" => FabricKind::Water,
            "RimLighting" => FabricKind::RimLighting,
            "ElevationBand" => FabricKind::ElevationBand,
            "WaterMask" => FabricKind::WaterMask,
            _ => FabricKind::Color,
        }
    }
}

// The `ShaderType` derive (encase 0.10) emits, for every field, a
// `const _: fn() = || { fn check() { .. } }` compile-time trait-bound
// assertion. The inner `fn check` is intentionally never *called* (it only
// forces the field type's bounds to be checked), so Rust 1.95+'s `dead_code`
// lint reports it — a false positive in third-party generated code. Isolating
// the derive in a submodule with a tightly-scoped `#![allow(dead_code)]`
// silences it without disabling the lint for the rest of this file.
mod fabric_params {
    #![allow(dead_code)]
    use super::*;

    /// GPU uniform block for [`FabricMaterial`](super::FabricMaterial).
    ///
    /// Fields are packed from the domain material's uniform map. The layout
    /// must match the `FabricParams` struct in `shaders/fabric_material.wgsl`.
    #[derive(ShaderType, Debug, Clone)]
    pub struct FabricParams {
        /// [`FabricKind`] discriminant.
        pub kind: u32,
        /// Stripe `horizontal` flag (0/1).
        pub horizontal: u32,
        /// Fade `repeat` flag (0/1).
        pub repeat_flag: u32,
        /// Grid `czm_pixelRatio` (integer, typically 1).
        pub pixel_ratio: u32,
        /// Primary colour (light/even/color/fadeIn/waterColor/baseColor).
        pub color_a: Vec4,
        /// Secondary colour (dark/odd/fadeOut/outlineColor/rimColor/landColor/gapColor).
        pub color_b: Vec4,
        /// Image tint colour.
        pub color_c: Vec4,
        /// x=repeat.x, y=repeat.y, z=stripe offset, w=fade maximumDistance.
        pub repeat_offset: Vec4,
        /// x=lineCount.x, y=lineCount.y, z=lineThickness.x, w=lineThickness.y.
        pub line_params: Vec4,
        /// x=lineOffset.x, y=lineOffset.y, z=cellAlpha, w=(spare).
        pub line_off_cell: Vec4,
        /// x=fadeDirection.x, y=fadeDirection.y, z=time.x, w=time.y.
        pub fade_dir_time: Vec4,
        /// x=glowPower, y=taperPower, z=outlineWidth/rimWidth, w=dashLength.
        pub extra_a: Vec4,
        /// x=spacing(contour), y=contourWidth, z=strength(normal/bump), w=dashPattern.
        pub extra_b: Vec4,
        /// x=minHeight(ramp/band), y=maxHeight(ramp/band), z=time(water), w=animationSpeed.
        pub extra_c: Vec4,
    }

    impl Default for FabricParams {
        fn default() -> Self {
            Self {
                kind: 0,
                horizontal: 0,
                repeat_flag: 0,
                pixel_ratio: 1,
                color_a: Vec4::ONE,
                color_b: Vec4::ZERO,
                color_c: Vec4::ONE,
                repeat_offset: Vec4::new(1.0, 1.0, 0.0, 0.5),
                line_params: Vec4::new(8.0, 8.0, 1.0, 1.0),
                line_off_cell: Vec4::new(0.0, 0.0, 0.1, 0.0),
                fade_dir_time: Vec4::new(1.0, 1.0, 0.5, 0.5),
                extra_a: Vec4::new(1.0, 0.0, 0.3, 16.0),
                extra_b: Vec4::new(1000.0, 2.0, 0.5, 255.0),
                extra_c: Vec4::new(0.0, 1000.0, 0.0, 0.5),
            }
        }
    }
}
pub use fabric_params::FabricParams;

/// A Bevy material that renders a CesiumJS Fabric procedural pattern.
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct FabricMaterial {
    /// Packed uniform block.
    #[uniform(0)]
    pub params: FabricParams,
    /// Texture used by `Sampler2D` uniforms (e.g. the `Image` material).
    #[texture(1)]
    #[sampler(2)]
    pub image: Handle<Image>,
    /// Whether the material is translucent (drives [`AlphaMode`]).
    /// Mirrors `Material.isTranslucent()` from the domain layer.
    pub translucent: bool,
}

impl Material for FabricMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(FABRIC_MATERIAL_SHADER_HANDLE)
    }

    fn alpha_mode(&self) -> AlphaMode {
        if self.translucent {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        }
    }
}

// ---------------------------------------------------------------------------
// Uniform packing helpers
// ---------------------------------------------------------------------------

fn vec4_of(v: &UniformValue) -> Option<[f32; 4]> {
    match v {
        UniformValue::Vec4(a) => Some([a[0] as f32, a[1] as f32, a[2] as f32, a[3] as f32]),
        UniformValue::Vec3(a) => Some([a[0] as f32, a[1] as f32, a[2] as f32, 1.0]),
        _ => None,
    }
}

fn vec2_of(v: &UniformValue) -> Option<[f32; 2]> {
    match v {
        UniformValue::Vec2(a) => Some([a[0] as f32, a[1] as f32]),
        _ => None,
    }
}

fn float_of(v: &UniformValue) -> Option<f32> {
    match v {
        UniformValue::Float(f) => Some(*f as f32),
        _ => None,
    }
}

fn bool_of(v: &UniformValue) -> Option<bool> {
    match v {
        UniformValue::Bool(b) => Some(*b),
        _ => None,
    }
}

fn get_vec4(u: &BTreeMap<String, UniformValue>, name: &str, default: [f32; 4]) -> Vec4 {
    Vec4::from_slice(&u.get(name).and_then(vec4_of).unwrap_or(default))
}

fn get_vec2(u: &BTreeMap<String, UniformValue>, name: &str, default: [f32; 2]) -> [f32; 2] {
    u.get(name).and_then(vec2_of).unwrap_or(default)
}

fn get_float(u: &BTreeMap<String, UniformValue>, name: &str, default: f32) -> f32 {
    u.get(name).and_then(float_of).unwrap_or(default)
}

fn get_bool(u: &BTreeMap<String, UniformValue>, name: &str, default: bool) -> bool {
    u.get(name).and_then(bool_of).unwrap_or(default)
}

/// Builds a renderable [`FabricMaterial`] from a domain [`DomainMaterial`].
///
/// The `image` handle supplies any `Sampler2D` uniform (CesiumJS's
/// `czm_defaultImage`). Translucency is taken from the domain material's
/// `is_translucent()` so the alpha mode matches CesiumJS behaviour.
pub fn fabric_material_from_domain(
    domain_material: &DomainMaterial,
    image: Handle<Image>,
) -> FabricMaterial {
    let u = domain_material.uniforms();
    let kind = FabricKind::from_type_name(domain_material.type_name());

    let mut params = FabricParams {
        kind: kind as u32,
        ..Default::default()
    };

    match kind {
        FabricKind::Color => {
            params.color_a = get_vec4(u, "color", [1.0, 0.0, 0.0, 0.5]);
        }
        FabricKind::Image => {
            let repeat = get_vec2(u, "repeat", [1.0, 1.0]);
            params.repeat_offset = Vec4::new(repeat[0], repeat[1], 0.0, 0.5);
            params.color_c = get_vec4(u, "color", [1.0, 1.0, 1.0, 1.0]);
        }
        FabricKind::Checkerboard | FabricKind::Dot => {
            let repeat = get_vec2(u, "repeat", [5.0, 5.0]);
            params.repeat_offset = Vec4::new(repeat[0], repeat[1], 0.0, 0.5);
            params.color_a = get_vec4(u, "lightColor", [1.0, 1.0, 1.0, 0.5]);
            params.color_b = get_vec4(u, "darkColor", [0.0, 0.0, 0.0, 0.5]);
        }
        FabricKind::Stripe => {
            let repeat = get_float(u, "repeat", 5.0);
            let offset = get_float(u, "offset", 0.0);
            params.repeat_offset = Vec4::new(repeat, repeat, offset, 0.5);
            params.horizontal = u32::from(get_bool(u, "horizontal", true));
            params.color_a = get_vec4(u, "evenColor", [1.0, 1.0, 1.0, 0.5]);
            params.color_b = get_vec4(u, "oddColor", [0.0, 0.0, 1.0, 0.5]);
        }
        FabricKind::Grid => {
            let line_count = get_vec2(u, "lineCount", [8.0, 8.0]);
            let line_thickness = get_vec2(u, "lineThickness", [1.0, 1.0]);
            let line_offset = get_vec2(u, "lineOffset", [0.0, 0.0]);
            let cell_alpha = get_float(u, "cellAlpha", 0.1);
            params.line_params = Vec4::new(
                line_count[0],
                line_count[1],
                line_thickness[0],
                line_thickness[1],
            );
            params.line_off_cell = Vec4::new(line_offset[0], line_offset[1], cell_alpha, 0.0);
            params.color_a = get_vec4(u, "color", [0.0, 1.0, 0.0, 1.0]);
        }
        FabricKind::Fade => {
            let max_dist = get_float(u, "maximumDistance", 0.5);
            params.repeat_offset = Vec4::new(1.0, 1.0, 0.0, max_dist);
            params.repeat_flag = u32::from(get_bool(u, "repeat", true));
            let fade_dir = get_vec2(u, "fadeDirection", [1.0, 1.0]);
            let time = get_vec2(u, "time", [0.5, 0.5]);
            params.fade_dir_time = Vec4::new(fade_dir[0], fade_dir[1], time[0], time[1]);
            params.color_a = get_vec4(u, "fadeInColor", [1.0, 0.0, 0.0, 1.0]);
            params.color_b = get_vec4(u, "fadeOutColor", [0.0, 0.0, 0.0, 0.0]);
        }
        // --- New material types ---
        FabricKind::PolylineArrow => {
            params.color_a = get_vec4(u, "color", [1.0, 1.0, 1.0, 1.0]);
        }
        FabricKind::PolylineDash => {
            params.color_a = get_vec4(u, "color", [1.0, 1.0, 1.0, 1.0]);
            params.color_b = get_vec4(u, "gapColor", [0.0, 0.0, 0.0, 0.0]);
            params.extra_a.w = get_float(u, "dashLength", 16.0);
            params.extra_b.w = get_float(u, "dashPattern", 255.0);
        }
        FabricKind::PolylineGlow => {
            params.color_a = get_vec4(u, "color", [0.0, 1.0, 1.0, 1.0]);
            params.extra_a.x = get_float(u, "glowPower", 0.25);
            params.extra_a.y = get_float(u, "taperPower", 1.0);
        }
        FabricKind::PolylineOutline => {
            params.color_a = get_vec4(u, "color", [1.0, 1.0, 1.0, 1.0]);
            params.color_b = get_vec4(u, "outlineColor", [0.0, 0.0, 0.0, 1.0]);
            params.extra_a.z = get_float(u, "outlineWidth", 0.3);
        }
        FabricKind::ElevationContour => {
            params.color_a = get_vec4(u, "color", [1.0, 1.0, 1.0, 1.0]);
            params.extra_b.x = get_float(u, "spacing", 1000.0);
            params.extra_b.y = get_float(u, "width", 2.0);
        }
        FabricKind::ElevationRamp => {
            params.extra_c.x = get_float(u, "minimumHeight", 0.0);
            params.extra_c.y = get_float(u, "maximumHeight", 1000.0);
        }
        FabricKind::AspectRamp => {
            // Uses image texture for the ramp
        }
        FabricKind::SlopeRamp => {
            // Uses image texture for the ramp
        }
        FabricKind::NormalMap => {
            let repeat = get_vec2(u, "repeat", [1.0, 1.0]);
            params.repeat_offset = Vec4::new(repeat[0], repeat[1], 0.0, 0.0);
            params.extra_b.z = get_float(u, "strength", 0.5);
        }
        FabricKind::BumpMap => {
            let repeat = get_vec2(u, "repeat", [1.0, 1.0]);
            params.repeat_offset = Vec4::new(repeat[0], repeat[1], 0.0, 0.0);
            params.extra_b.z = get_float(u, "strength", 0.5);
        }
        FabricKind::Water => {
            params.color_a = get_vec4(u, "baseWaterColor", [0.2, 0.3, 0.6, 0.8]);
            params.color_b = get_vec4(u, "blendColor", [0.5, 0.5, 0.5, 0.5]);
            params.extra_c.w = get_float(u, "animationSpeed", 0.5);
            params.extra_c.z = 0.0; // time will be updated per-frame
        }
        FabricKind::RimLighting => {
            params.color_a = get_vec4(u, "color", [1.0, 1.0, 1.0, 1.0]);
            params.color_b = get_vec4(u, "rimColor", [0.3, 0.3, 1.0, 1.0]);
            params.extra_a.z = get_float(u, "width", 0.3);
        }
        FabricKind::ElevationBand => {
            params.extra_c.x = get_float(u, "minimumHeight", 0.0);
            params.extra_c.y = get_float(u, "maximumHeight", 1000.0);
        }
        FabricKind::WaterMask => {
            params.color_a = get_vec4(u, "waterColor", [0.1, 0.3, 0.7, 1.0]);
            params.color_b = get_vec4(u, "landColor", [0.3, 0.6, 0.2, 1.0]);
            params.extra_c.x = 0.0; // water level
        }
    }

    FabricMaterial {
        params,
        image,
        translucent: domain_material.is_translucent(),
    }
}

/// Plugin registering the [`FabricMaterial`] with Bevy's asset/pipeline system.
pub struct FabricMaterialPlugin;

impl Plugin for FabricMaterialPlugin {
    fn build(&self, app: &mut App) {
        // Embed the WGSL shader into the binary so no external asset path is
        // required by the host application.
        load_internal_asset!(
            app,
            FABRIC_MATERIAL_SHADER_HANDLE,
            "../shaders/fabric_material.wgsl",
            Shader::from_wgsl
        );
        app.add_plugins(MaterialPlugin::<FabricMaterial>::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_material::MaterialSystem;

    fn build(type_name: &str) -> DomainMaterial {
        let system = MaterialSystem::with_builtin_materials();
        system
            .from_type(type_name, BTreeMap::new())
            .unwrap_or_else(|e| panic!("failed to build {}: {}", type_name, e))
    }

    #[test]
    fn test_kind_mapping() {
        assert_eq!(FabricKind::from_type_name("Checkerboard"), FabricKind::Checkerboard);
        assert_eq!(FabricKind::from_type_name("Stripe"), FabricKind::Stripe);
        assert_eq!(FabricKind::from_type_name("Grid"), FabricKind::Grid);
        assert_eq!(FabricKind::from_type_name("Color"), FabricKind::Color);
        assert_eq!(FabricKind::from_type_name("SomeCustom"), FabricKind::Color);
    }

    #[test]
    fn test_kind_mapping_extended() {
        assert_eq!(FabricKind::from_type_name("PolylineArrow"), FabricKind::PolylineArrow);
        assert_eq!(FabricKind::from_type_name("PolylineGlow"), FabricKind::PolylineGlow);
        assert_eq!(FabricKind::from_type_name("ElevationContour"), FabricKind::ElevationContour);
        assert_eq!(FabricKind::from_type_name("Water"), FabricKind::Water);
        assert_eq!(FabricKind::from_type_name("RimLighting"), FabricKind::RimLighting);
        assert_eq!(FabricKind::from_type_name("BumpMap"), FabricKind::BumpMap);
        assert_eq!(FabricKind::from_type_name("WaterMask"), FabricKind::WaterMask);
        assert_eq!(FabricKind::from_type_name("UnknownType"), FabricKind::Color);
    }

    #[test]
    fn test_from_domain_checkerboard() {
        let m = build("Checkerboard");
        let handle = Handle::<Image>::default();
        let fm = fabric_material_from_domain(&m, handle);
        assert_eq!(fm.params.kind, FabricKind::Checkerboard as u32);
        // Default repeat is (5, 5).
        assert!((fm.params.repeat_offset.x - 5.0).abs() < 1e-6);
        assert!((fm.params.repeat_offset.y - 5.0).abs() < 1e-6);
        // Default lightColor is white (alpha 0.5) -> translucent.
        assert!(fm.translucent);
    }

    #[test]
    fn test_from_domain_grid() {
        let m = build("Grid");
        let fm = fabric_material_from_domain(&m, Handle::<Image>::default());
        assert_eq!(fm.params.kind, FabricKind::Grid as u32);
        assert!((fm.params.line_params.x - 8.0).abs() < 1e-6); // lineCount.x
        assert!((fm.params.line_off_cell.z - 0.1).abs() < 1e-6); // cellAlpha
        // Default Grid is translucent (cellAlpha 0.1).
        assert!(fm.translucent);
    }

    #[test]
    fn test_from_domain_stripe() {
        let m = build("Stripe");
        let fm = fabric_material_from_domain(&m, Handle::<Image>::default());
        assert_eq!(fm.params.kind, FabricKind::Stripe as u32);
        assert!((fm.params.repeat_offset.x - 5.0).abs() < 1e-6); // repeat
        assert_eq!(fm.params.horizontal, 1); // default horizontal = true
    }

    #[test]
    fn test_from_domain_color_opaque_override() {
        let system = MaterialSystem::with_builtin_materials();
        let mut overrides = BTreeMap::new();
        overrides.insert(
            "color".to_string(),
            UniformValue::Vec4([0.0, 1.0, 0.0, 1.0]),
        );
        let m = system.from_type("Color", overrides).unwrap();
        let fm = fabric_material_from_domain(&m, Handle::<Image>::default());
        assert_eq!(fm.params.kind, FabricKind::Color as u32);
        assert!(!fm.translucent); // alpha 1.0 -> opaque
        assert!((fm.params.color_a.y - 1.0).abs() < 1e-6); // green
    }

    #[test]
    fn test_alpha_mode_follows_translucency() {
        let m = build("Grid");
        let fm = fabric_material_from_domain(&m, Handle::<Image>::default());
        assert!(matches!(fm.alpha_mode(), AlphaMode::Blend));

        let system = MaterialSystem::with_builtin_materials();
        let mut overrides = BTreeMap::new();
        overrides.insert("cellAlpha".to_string(), UniformValue::Float(1.0));
        overrides.insert(
            "color".to_string(),
            UniformValue::Vec4([0.0, 1.0, 0.0, 1.0]),
        );
        let opaque = system.from_type("Grid", overrides).unwrap();
        let fm2 = fabric_material_from_domain(&opaque, Handle::<Image>::default());
        assert!(matches!(fm2.alpha_mode(), AlphaMode::Opaque));
    }

    #[test]
    fn test_from_domain_polyline_arrow() {
        let system = MaterialSystem::with_builtin_materials();
        let m = system.from_type("PolylineArrow", BTreeMap::new()).unwrap();
        let fm = fabric_material_from_domain(&m, Handle::<Image>::default());
        assert_eq!(fm.params.kind, FabricKind::PolylineArrow as u32);
    }

    #[test]
    fn test_from_domain_elevation_contour() {
        let system = MaterialSystem::with_builtin_materials();
        let mut overrides = BTreeMap::new();
        overrides.insert("spacing".to_string(), UniformValue::Float(500.0));
        overrides.insert("width".to_string(), UniformValue::Float(3.0));
        let m = system.from_type("ElevationContour", overrides).unwrap();
        let fm = fabric_material_from_domain(&m, Handle::<Image>::default());
        assert_eq!(fm.params.kind, FabricKind::ElevationContour as u32);
        assert!((fm.params.extra_b.x - 500.0).abs() < 1e-6);
        assert!((fm.params.extra_b.y - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_from_domain_rim_lighting() {
        let system = MaterialSystem::with_builtin_materials();
        let mut overrides = BTreeMap::new();
        overrides.insert("width".to_string(), UniformValue::Float(0.5));
        let m = system.from_type("RimLighting", overrides).unwrap();
        let fm = fabric_material_from_domain(&m, Handle::<Image>::default());
        assert_eq!(fm.params.kind, FabricKind::RimLighting as u32);
        assert!((fm.params.extra_a.z - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_from_domain_water() {
        let system = MaterialSystem::with_builtin_materials();
        let mut overrides = BTreeMap::new();
        overrides.insert("animationSpeed".to_string(), UniformValue::Float(0.3));
        let m = system.from_type("Water", overrides).unwrap();
        let fm = fabric_material_from_domain(&m, Handle::<Image>::default());
        assert_eq!(fm.params.kind, FabricKind::Water as u32);
        assert!((fm.params.extra_c.w - 0.3).abs() < 1e-6);
    }

    #[test]
    fn test_from_domain_water_mask() {
        let system = MaterialSystem::with_builtin_materials();
        let m = system.from_type("WaterMask", BTreeMap::new()).unwrap();
        let fm = fabric_material_from_domain(&m, Handle::<Image>::default());
        assert_eq!(fm.params.kind, FabricKind::WaterMask as u32);
    }
}
