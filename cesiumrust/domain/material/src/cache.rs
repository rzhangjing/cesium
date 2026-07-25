//! The material cache and the 25 built-in Fabric materials.
//!
//! Maps to CesiumJS `Material._materialCache` and the
//! `Material._materialCache.addMaterial(...)` calls at the bottom of
//! `Scene/Material.js`. Instead of a mutable global, the domain layer uses an
//! explicit [`MaterialSystem`] value so caches are testable and can be scoped
//! per application.

use crate::error::MaterialError;
use crate::fabric::{FabricTemplate, MaterialComponents};
use crate::glsl;
use crate::material::{build_material, Material, MaterialOptions};
use crate::translucent::TranslucentSpec;
use crate::uniform::{UniformValue, DEFAULT_IMAGE_ID};
use std::collections::{BTreeMap, HashMap};

/// A cached material definition: the original Fabric template plus its
/// translucency rule.
///
/// Maps to a single entry in `Material._materialCache._materials`, i.e. the
/// `{ fabric, translucent }` object passed to `addMaterial`.
#[derive(Debug, Clone)]
pub struct CachedMaterial {
    /// The original Fabric template (uniforms + source/components).
    pub fabric: FabricTemplate,
    /// The translucency rule (`translucent` member of the cache entry).
    pub translucent: Option<TranslucentSpec>,
}

impl CachedMaterial {
    fn from_components(
        type_name: &str,
        uniforms: BTreeMap<String, UniformValue>,
        components: MaterialComponents,
        translucent: Option<TranslucentSpec>,
    ) -> Self {
        CachedMaterial {
            fabric: FabricTemplate {
                type_name: Some(type_name.to_string()),
                uniforms,
                materials: BTreeMap::new(),
                components: Some(components),
                source: None,
            },
            translucent,
        }
    }

    fn from_source(
        type_name: &str,
        uniforms: BTreeMap<String, UniformValue>,
        source: &str,
        translucent: Option<TranslucentSpec>,
    ) -> Self {
        CachedMaterial {
            fabric: FabricTemplate {
                type_name: Some(type_name.to_string()),
                uniforms,
                materials: BTreeMap::new(),
                components: None,
                source: Some(source.to_string()),
            },
            translucent,
        }
    }
}

/// Shorthand builders for uniform maps used by the built-in materials.
fn color(r: f64, g: f64, b: f64, a: f64) -> UniformValue {
    UniformValue::Vec4([r, g, b, a])
}
fn vec2(x: f64, y: f64) -> UniformValue {
    UniformValue::Vec2([x, y])
}
fn float(v: f64) -> UniformValue {
    UniformValue::Float(v)
}
fn bool_u(v: bool) -> UniformValue {
    UniformValue::Bool(v)
}
fn default_image() -> UniformValue {
    UniformValue::Sampler2D(DEFAULT_IMAGE_ID.to_string())
}
fn channels(s: &str) -> UniformValue {
    UniformValue::Channels(s.to_string())
}

/// A uniform-map builder to keep the built-in definitions readable.
struct U(BTreeMap<String, UniformValue>);
impl U {
    fn new() -> Self {
        U(BTreeMap::new())
    }
    fn set(mut self, key: &str, value: UniformValue) -> Self {
        self.0.insert(key.to_string(), value);
        self
    }
    fn build(self) -> BTreeMap<String, UniformValue> {
        self.0
    }
}

/// The material cache + factory.
///
/// Maps to `Material._materialCache` plus the `new Material(...)` /
/// `Material.fromType(...)` construction entry points.
#[derive(Debug, Clone, Default)]
pub struct MaterialSystem {
    cache: HashMap<String, CachedMaterial>,
}

impl MaterialSystem {
    /// An empty cache with no built-in materials.
    pub fn new() -> Self {
        MaterialSystem {
            cache: HashMap::new(),
        }
    }

    /// A cache pre-populated with the 25 built-in CesiumJS materials.
    pub fn with_builtin_materials() -> Self {
        let mut system = MaterialSystem::new();
        for (name, material) in builtin_materials() {
            system.cache.insert(name, material);
        }
        system
    }

    /// Registers a material type. Maps to `addMaterial`.
    pub fn add_material(&mut self, type_name: &str, material: CachedMaterial) {
        self.cache.insert(type_name.to_string(), material);
    }

    /// Looks up a cached material type. Maps to `getMaterial`.
    pub fn get_material(&self, type_name: &str) -> Option<&CachedMaterial> {
        self.cache.get(type_name)
    }

    /// The number of cached material types.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Builds a material from a Fabric template (low-level entry used by the
    /// construction pipeline and tests).
    pub(crate) fn build(
        &self,
        fabric: FabricTemplate,
        strict: bool,
        translucent: Option<bool>,
    ) -> Result<Material, MaterialError> {
        let mut count = 0usize;
        let (material, _collected) =
            build_material(fabric, strict, translucent, &self.cache, &mut count)?;
        Ok(material)
    }

    /// Creates a material from options.
    ///
    /// Maps to `new Material(options)`. When the resulting type is new (not in
    /// the cache) it is added afterwards, mirroring `initializeMaterial`.
    pub fn create_material(&mut self, options: MaterialOptions) -> Result<Material, MaterialError> {
        let type_name = options
            .fabric
            .type_name
            .clone()
            .unwrap_or_else(|| options.fabric.type_name.clone().unwrap_or_default());

        let already_cached = !type_name.is_empty() && self.cache.contains_key(&type_name);

        let material = self.build(options.fabric.clone(), options.strict, options.translucent)?;

        // Add new types to the cache (with no translucency rule of their own;
        // CesiumJS stores the Material whose `translucent` property is
        // undefined, i.e. `None` here).
        if !already_cached {
            self.cache.insert(
                material.type_name().to_string(),
                CachedMaterial {
                    fabric: options.fabric,
                    translucent: None,
                },
            );
        }

        Ok(material)
    }

    /// Creates a new material from an existing cached type.
    ///
    /// Maps to `Material.fromType(type, uniforms)`.
    pub fn from_type(
        &self,
        type_name: &str,
        overrides: BTreeMap<String, UniformValue>,
    ) -> Result<Material, MaterialError> {
        if !self.cache.contains_key(type_name) {
            return Err(MaterialError::UnknownMaterialType {
                type_name: type_name.to_string(),
            });
        }

        let mut fabric = FabricTemplate {
            type_name: Some(type_name.to_string()),
            ..Default::default()
        };
        fabric.uniforms = overrides;

        self.build(fabric, false, None)
    }
}

/// The 25 built-in materials, in the order they are registered in
/// `Scene/Material.js`.
fn builtin_materials() -> Vec<(String, CachedMaterial)> {
    let mut out: Vec<(String, CachedMaterial)> = Vec::with_capacity(25);
    let mut add = |name: &str, m: CachedMaterial| out.push((name.to_string(), m));

    // Color
    add(
        "Color",
        CachedMaterial::from_components(
            "Color",
            U::new().set("color", color(1.0, 0.0, 0.0, 0.5)).build(),
            MaterialComponents {
                diffuse: Some("color.rgb".to_string()),
                alpha: Some("color.a".to_string()),
                ..Default::default()
            },
            Some(TranslucentSpec::AnyAlphaLt1(vec!["color"])),
        ),
    );

    // Image
    add(
        "Image",
        CachedMaterial::from_components(
            "Image",
            U::new()
                .set("image", default_image())
                .set("repeat", vec2(1.0, 1.0))
                .set("color", color(1.0, 1.0, 1.0, 1.0))
                .build(),
            MaterialComponents {
                diffuse: Some(
                    "texture(image, fract(repeat * materialInput.st)).rgb * color.rgb".to_string(),
                ),
                alpha: Some(
                    "texture(image, fract(repeat * materialInput.st)).a * color.a".to_string(),
                ),
                ..Default::default()
            },
            Some(TranslucentSpec::AnyAlphaLt1(vec!["color"])),
        ),
    );

    // DiffuseMap
    add(
        "DiffuseMap",
        CachedMaterial::from_components(
            "DiffuseMap",
            U::new()
                .set("image", default_image())
                .set("channels", channels("rgb"))
                .set("repeat", vec2(1.0, 1.0))
                .build(),
            MaterialComponents {
                diffuse: Some("texture(image, fract(repeat * materialInput.st)).channels".to_string()),
                ..Default::default()
            },
            Some(TranslucentSpec::Never),
        ),
    );

    // AlphaMap
    add(
        "AlphaMap",
        CachedMaterial::from_components(
            "AlphaMap",
            U::new()
                .set("image", default_image())
                .set("channel", channels("a"))
                .set("repeat", vec2(1.0, 1.0))
                .build(),
            MaterialComponents {
                alpha: Some("texture(image, fract(repeat * materialInput.st)).channel".to_string()),
                ..Default::default()
            },
            Some(TranslucentSpec::Always),
        ),
    );

    // SpecularMap
    add(
        "SpecularMap",
        CachedMaterial::from_components(
            "SpecularMap",
            U::new()
                .set("image", default_image())
                .set("channel", channels("r"))
                .set("repeat", vec2(1.0, 1.0))
                .build(),
            MaterialComponents {
                specular: Some(
                    "texture(image, fract(repeat * materialInput.st)).channel".to_string(),
                ),
                ..Default::default()
            },
            Some(TranslucentSpec::Never),
        ),
    );

    // EmissionMap
    add(
        "EmissionMap",
        CachedMaterial::from_components(
            "EmissionMap",
            U::new()
                .set("image", default_image())
                .set("channels", channels("rgb"))
                .set("repeat", vec2(1.0, 1.0))
                .build(),
            MaterialComponents {
                emission: Some(
                    "texture(image, fract(repeat * materialInput.st)).channels".to_string(),
                ),
                ..Default::default()
            },
            Some(TranslucentSpec::Never),
        ),
    );

    // BumpMap
    add(
        "BumpMap",
        CachedMaterial::from_source(
            "BumpMap",
            U::new()
                .set("image", default_image())
                .set("channel", channels("r"))
                .set("strength", float(0.8))
                .set("repeat", vec2(1.0, 1.0))
                .build(),
            glsl::BUMP_MAP_MATERIAL,
            Some(TranslucentSpec::Never),
        ),
    );

    // NormalMap
    add(
        "NormalMap",
        CachedMaterial::from_source(
            "NormalMap",
            U::new()
                .set("image", default_image())
                .set("channels", channels("rgb"))
                .set("strength", float(0.8))
                .set("repeat", vec2(1.0, 1.0))
                .build(),
            glsl::NORMAL_MAP_MATERIAL,
            Some(TranslucentSpec::Never),
        ),
    );

    // Grid
    add(
        "Grid",
        CachedMaterial::from_source(
            "Grid",
            U::new()
                .set("color", color(0.0, 1.0, 0.0, 1.0))
                .set("cellAlpha", float(0.1))
                .set("lineCount", vec2(8.0, 8.0))
                .set("lineThickness", vec2(1.0, 1.0))
                .set("lineOffset", vec2(0.0, 0.0))
                .build(),
            glsl::GRID_MATERIAL,
            Some(TranslucentSpec::AnyAlphaLt1(vec!["color", "cellAlpha"])),
        ),
    );

    // Stripe
    add(
        "Stripe",
        CachedMaterial::from_source(
            "Stripe",
            U::new()
                .set("horizontal", bool_u(true))
                .set("evenColor", color(1.0, 1.0, 1.0, 0.5))
                .set("oddColor", color(0.0, 0.0, 1.0, 0.5))
                .set("offset", float(0.0))
                .set("repeat", float(5.0))
                .build(),
            glsl::STRIPE_MATERIAL,
            Some(TranslucentSpec::AnyAlphaLt1(vec!["evenColor", "oddColor"])),
        ),
    );

    // Checkerboard
    add(
        "Checkerboard",
        CachedMaterial::from_source(
            "Checkerboard",
            U::new()
                .set("lightColor", color(1.0, 1.0, 1.0, 0.5))
                .set("darkColor", color(0.0, 0.0, 0.0, 0.5))
                .set("repeat", vec2(5.0, 5.0))
                .build(),
            glsl::CHECKERBOARD_MATERIAL,
            Some(TranslucentSpec::AnyAlphaLt1(vec!["lightColor", "darkColor"])),
        ),
    );

    // Dot
    add(
        "Dot",
        CachedMaterial::from_source(
            "Dot",
            U::new()
                .set("lightColor", color(1.0, 1.0, 0.0, 0.75))
                .set("darkColor", color(0.0, 1.0, 1.0, 0.75))
                .set("repeat", vec2(5.0, 5.0))
                .build(),
            glsl::DOT_MATERIAL,
            Some(TranslucentSpec::AnyAlphaLt1(vec!["lightColor", "darkColor"])),
        ),
    );

    // Water
    add(
        "Water",
        CachedMaterial::from_source(
            "Water",
            U::new()
                .set("baseWaterColor", color(0.2, 0.3, 0.6, 1.0))
                .set("blendColor", color(0.0, 1.0, 0.699, 1.0))
                .set("specularMap", default_image())
                .set("normalMap", default_image())
                .set("frequency", float(10.0))
                .set("animationSpeed", float(0.01))
                .set("amplitude", float(1.0))
                .set("specularIntensity", float(0.5))
                .set("fadeFactor", float(1.0))
                .build(),
            glsl::WATER_MATERIAL,
            Some(TranslucentSpec::AnyAlphaLt1(vec![
                "baseWaterColor",
                "blendColor",
            ])),
        ),
    );

    // RimLighting
    add(
        "RimLighting",
        CachedMaterial::from_source(
            "RimLighting",
            U::new()
                .set("color", color(1.0, 0.0, 0.0, 0.7))
                .set("rimColor", color(1.0, 1.0, 1.0, 0.4))
                .set("width", float(0.3))
                .build(),
            glsl::RIM_LIGHTING_MATERIAL,
            Some(TranslucentSpec::AnyAlphaLt1(vec!["color", "rimColor"])),
        ),
    );

    // Fade
    add(
        "Fade",
        CachedMaterial::from_source(
            "Fade",
            U::new()
                .set("fadeInColor", color(1.0, 0.0, 0.0, 1.0))
                .set("fadeOutColor", color(0.0, 0.0, 0.0, 0.0))
                .set("maximumDistance", float(0.5))
                .set("repeat", bool_u(true))
                .set("fadeDirection", vec2(1.0, 1.0))
                .set("time", vec2(0.5, 0.5))
                .build(),
            glsl::FADE_MATERIAL,
            Some(TranslucentSpec::AnyAlphaLt1(vec![
                "fadeInColor",
                "fadeOutColor",
            ])),
        ),
    );

    // PolylineArrow
    add(
        "PolylineArrow",
        CachedMaterial::from_source(
            "PolylineArrow",
            U::new().set("color", color(1.0, 1.0, 1.0, 1.0)).build(),
            glsl::POLYLINE_ARROW_MATERIAL,
            Some(TranslucentSpec::Always),
        ),
    );

    // PolylineDash
    add(
        "PolylineDash",
        CachedMaterial::from_source(
            "PolylineDash",
            U::new()
                .set("color", color(1.0, 0.0, 1.0, 1.0))
                .set("gapColor", color(0.0, 0.0, 0.0, 0.0))
                .set("dashLength", float(16.0))
                .set("dashPattern", float(255.0))
                .build(),
            glsl::POLYLINE_DASH_MATERIAL,
            Some(TranslucentSpec::Always),
        ),
    );

    // PolylineGlow
    add(
        "PolylineGlow",
        CachedMaterial::from_source(
            "PolylineGlow",
            U::new()
                .set("color", color(0.0, 0.5, 1.0, 1.0))
                .set("glowPower", float(0.25))
                .set("taperPower", float(1.0))
                .build(),
            glsl::POLYLINE_GLOW_MATERIAL,
            Some(TranslucentSpec::Always),
        ),
    );

    // PolylineOutline
    add(
        "PolylineOutline",
        CachedMaterial::from_source(
            "PolylineOutline",
            U::new()
                .set("color", color(1.0, 1.0, 1.0, 1.0))
                .set("outlineColor", color(1.0, 0.0, 0.0, 1.0))
                .set("outlineWidth", float(1.0))
                .build(),
            glsl::POLYLINE_OUTLINE_MATERIAL,
            Some(TranslucentSpec::AnyAlphaLt1(vec!["color", "outlineColor"])),
        ),
    );

    // ElevationContour
    add(
        "ElevationContour",
        CachedMaterial::from_source(
            "ElevationContour",
            U::new()
                .set("spacing", float(100.0))
                .set("color", color(1.0, 0.0, 0.0, 1.0))
                .set("width", float(1.0))
                .build(),
            glsl::ELEVATION_CONTOUR_MATERIAL,
            Some(TranslucentSpec::Never),
        ),
    );

    // ElevationRamp
    add(
        "ElevationRamp",
        CachedMaterial::from_source(
            "ElevationRamp",
            U::new()
                .set("image", default_image())
                .set("minimumHeight", float(0.0))
                .set("maximumHeight", float(10000.0))
                .build(),
            glsl::ELEVATION_RAMP_MATERIAL,
            Some(TranslucentSpec::Never),
        ),
    );

    // SlopeRamp
    add(
        "SlopeRamp",
        CachedMaterial::from_source(
            "SlopeRamp",
            U::new().set("image", default_image()).build(),
            glsl::SLOPE_RAMP_MATERIAL,
            Some(TranslucentSpec::Never),
        ),
    );

    // AspectRamp
    add(
        "AspectRamp",
        CachedMaterial::from_source(
            "AspectRamp",
            U::new().set("image", default_image()).build(),
            glsl::ASPECT_RAMP_MATERIAL,
            Some(TranslucentSpec::Never),
        ),
    );

    // ElevationBand
    add(
        "ElevationBand",
        CachedMaterial::from_source(
            "ElevationBand",
            U::new()
                .set("heights", default_image())
                .set("colors", default_image())
                .build(),
            glsl::ELEVATION_BAND_MATERIAL,
            Some(TranslucentSpec::Always),
        ),
    );

    // WaterMask
    add(
        "WaterMask",
        CachedMaterial::from_source(
            "WaterMask",
            U::new()
                .set("waterColor", color(1.0, 1.0, 1.0, 1.0))
                .set("landColor", color(0.0, 0.0, 0.0, 0.0))
                .build(),
            glsl::WATER_MASK_MATERIAL,
            Some(TranslucentSpec::Never),
        ),
    );

    out
}

/// The built-in material type names. Maps to the `Material.*Type` constants.
pub const BUILTIN_MATERIAL_TYPES: [&str; 25] = [
    "Color",
    "Image",
    "DiffuseMap",
    "AlphaMap",
    "SpecularMap",
    "EmissionMap",
    "BumpMap",
    "NormalMap",
    "Grid",
    "Stripe",
    "Checkerboard",
    "Dot",
    "Water",
    "RimLighting",
    "Fade",
    "PolylineArrow",
    "PolylineDash",
    "PolylineGlow",
    "PolylineOutline",
    "ElevationContour",
    "ElevationRamp",
    "SlopeRamp",
    "AspectRamp",
    "ElevationBand",
    "WaterMask",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_cache_has_25_materials() {
        let system = MaterialSystem::with_builtin_materials();
        assert_eq!(system.len(), 25);
        for name in BUILTIN_MATERIAL_TYPES {
            assert!(
                system.get_material(name).is_some(),
                "missing built-in material: {}",
                name
            );
        }
    }

    #[test]
    fn test_from_type_color() {
        let system = MaterialSystem::with_builtin_materials();
        let m = system.from_type("Color", BTreeMap::new()).unwrap();
        assert_eq!(m.type_name(), "Color");
        assert!(m.shader_source().contains("czm_getMaterial"));
        assert!(m.is_translucent()); // default alpha 0.5
    }

    #[test]
    fn test_from_type_with_override() {
        let system = MaterialSystem::with_builtin_materials();
        let mut overrides = BTreeMap::new();
        overrides.insert("color".to_string(), color(0.0, 1.0, 0.0, 1.0));
        let m = system.from_type("Color", overrides).unwrap();
        assert_eq!(
            m.uniforms().get("color"),
            Some(&UniformValue::Vec4([0.0, 1.0, 0.0, 1.0]))
        );
        assert!(!m.is_translucent()); // alpha 1.0
    }

    #[test]
    fn test_from_type_unknown_errors() {
        let system = MaterialSystem::with_builtin_materials();
        let err = system.from_type("DoesNotExist", BTreeMap::new()).unwrap_err();
        assert!(matches!(
            err,
            MaterialError::UnknownMaterialType { ref type_name } if type_name == "DoesNotExist"
        ));
    }

    #[test]
    fn test_all_builtins_build_successfully() {
        let system = MaterialSystem::with_builtin_materials();
        for name in BUILTIN_MATERIAL_TYPES {
            let m = system
                .from_type(name, BTreeMap::new())
                .unwrap_or_else(|e| panic!("built-in material {} failed to build: {}", name, e));
            assert_eq!(m.type_name(), name);
            assert!(!m.shader_source().is_empty());
        }
    }

    #[test]
    fn test_builtin_translucency_expectations() {
        let system = MaterialSystem::with_builtin_materials();
        // Always-translucent built-ins.
        for name in ["AlphaMap", "PolylineArrow", "PolylineDash", "PolylineGlow", "ElevationBand"] {
            assert!(
                system.from_type(name, BTreeMap::new()).unwrap().is_translucent(),
                "{} should be translucent",
                name
            );
        }
        // Never-translucent built-ins.
        for name in ["DiffuseMap", "SpecularMap", "EmissionMap", "BumpMap", "NormalMap", "ElevationContour", "WaterMask"] {
            assert!(
                !system.from_type(name, BTreeMap::new()).unwrap().is_translucent(),
                "{} should be opaque",
                name
            );
        }
    }

    #[test]
    fn test_create_material_caches_new_type() {
        let mut system = MaterialSystem::with_builtin_materials();
        let fabric = FabricTemplate::from_json_str(
            r#"{"type": "MyCustom", "components": {"diffuse": "vec3(1.0)"}}"#,
        )
        .unwrap();
        let m = system
            .create_material(MaterialOptions {
                strict: false,
                translucent: None,
                fabric,
            })
            .unwrap();
        assert_eq!(m.type_name(), "MyCustom");
        assert!(system.get_material("MyCustom").is_some());

        // A second material of the same type reuses the cached template.
        let m2 = system.from_type("MyCustom", BTreeMap::new()).unwrap();
        assert_eq!(m2.type_name(), "MyCustom");
        assert!(m2.shader_source().contains("material.diffuse"));
    }

    #[test]
    fn test_grid_translucency_via_cell_alpha() {
        let system = MaterialSystem::with_builtin_materials();
        // Default Grid: color alpha 1.0 but cellAlpha 0.1 -> translucent.
        assert!(system.from_type("Grid", BTreeMap::new()).unwrap().is_translucent());
        // cellAlpha 1.0 and color alpha 1.0 -> opaque.
        let mut overrides = BTreeMap::new();
        overrides.insert("cellAlpha".to_string(), float(1.0));
        overrides.insert("color".to_string(), color(0.0, 1.0, 0.0, 1.0));
        assert!(!system.from_type("Grid", overrides).unwrap().is_translucent());
    }
}
