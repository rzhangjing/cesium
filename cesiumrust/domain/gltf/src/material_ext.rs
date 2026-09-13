//! PBR material extensions for glTF 2.0.
//!
//! Maps to CesiumJS:
//! - `Scene/ModelComponents.js` (MetallicRoughness, SpecularGlossiness, Specular, Clearcoat, Anisotropy)
//! - `Scene/Model/GltfLoaderUtility.js` (extension parsing)
//!
//! Supported KHR extensions:
//! - KHR_materials_pbrSpecularGlossiness
//! - KHR_materials_specular
//! - KHR_materials_clearcoat
//! - KHR_materials_anisotropy
//! - KHR_materials_transmission
//! - KHR_materials_ior
//! - KHR_materials_emissive_strength
//! - KHR_materials_unlit
//! - KHR_materials_sheen
//! - KHR_materials_volume
//! - KHR_texture_transform

use crate::gltf_model::TextureInfo;
use serde::{Deserialize, Serialize};

/// Extended material properties combining base PBR with all KHR extensions.
///
/// Maps to CesiumJS `ModelComponents.Material`
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtendedMaterial {
    /// Base PBR metallic-roughness (always present in glTF 2.0).
    #[serde(default)]
    pub metallic_roughness: MetallicRoughness,

    /// KHR_materials_pbrSpecularGlossiness extension.
    #[serde(default)]
    pub specular_glossiness: Option<SpecularGlossiness>,

    /// KHR_materials_specular extension.
    #[serde(default)]
    pub specular: Option<Specular>,

    /// KHR_materials_clearcoat extension.
    #[serde(default)]
    pub clearcoat: Option<Clearcoat>,

    /// KHR_materials_anisotropy extension.
    #[serde(default)]
    pub anisotropy: Option<Anisotropy>,

    /// KHR_materials_transmission extension.
    #[serde(default)]
    pub transmission: Option<Transmission>,

    /// KHR_materials_ior extension.
    #[serde(default)]
    pub ior: Option<Ior>,

    /// KHR_materials_emissive_strength extension.
    #[serde(default)]
    pub emissive_strength: Option<EmissiveStrength>,

    /// KHR_materials_sheen extension.
    #[serde(default)]
    pub sheen: Option<Sheen>,

    /// KHR_materials_volume extension.
    #[serde(default)]
    pub volume: Option<Volume>,

    /// KHR_materials_unlit extension present.
    #[serde(default)]
    pub unlit: bool,
}

/// PBR metallic-roughness shading model.
///
/// Maps to CesiumJS `ModelComponents.MetallicRoughness`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetallicRoughness {
    /// Base color factor [r, g, b, a]. Default [1,1,1,1].
    #[serde(default = "default_base_color_factor")]
    pub base_color_factor: [f64; 4],

    /// Base color texture.
    #[serde(default)]
    pub base_color_texture: Option<TextureTransformInfo>,

    /// Metallic factor. Default 1.0.
    #[serde(default = "default_one")]
    pub metallic_factor: f64,

    /// Roughness factor. Default 1.0.
    #[serde(default = "default_one")]
    pub roughness_factor: f64,

    /// Metallic-roughness texture (G=roughness, B=metallic).
    #[serde(default)]
    pub metallic_roughness_texture: Option<TextureTransformInfo>,
}

impl Default for MetallicRoughness {
    fn default() -> Self {
        Self {
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            base_color_texture: None,
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            metallic_roughness_texture: None,
        }
    }
}

/// KHR_materials_pbrSpecularGlossiness extension.
///
/// Maps to CesiumJS `ModelComponents.SpecularGlossiness`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecularGlossiness {
    /// Diffuse factor [r, g, b, a]. Default [1,1,1,1].
    #[serde(default = "default_base_color_factor")]
    pub diffuse_factor: [f64; 4],

    /// Diffuse texture.
    #[serde(default)]
    pub diffuse_texture: Option<TextureTransformInfo>,

    /// Specular factor [r, g, b]. Default [1,1,1].
    #[serde(default = "default_specular_factor")]
    pub specular_factor: [f64; 3],

    /// Glossiness factor. Default 1.0.
    #[serde(default = "default_one")]
    pub glossiness_factor: f64,

    /// Specular-glossiness texture.
    #[serde(default)]
    pub specular_glossiness_texture: Option<TextureTransformInfo>,
}

impl Default for SpecularGlossiness {
    fn default() -> Self {
        Self {
            diffuse_factor: [1.0, 1.0, 1.0, 1.0],
            diffuse_texture: None,
            specular_factor: [1.0, 1.0, 1.0],
            glossiness_factor: 1.0,
            specular_glossiness_texture: None,
        }
    }
}

/// KHR_materials_specular extension.
///
/// Maps to CesiumJS `ModelComponents.Specular`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Specular {
    /// Specular factor. Default 1.0.
    #[serde(default = "default_one")]
    pub specular_factor: f64,

    /// Specular texture.
    #[serde(default)]
    pub specular_texture: Option<TextureTransformInfo>,

    /// Specular color factor [r, g, b]. Default [1,1,1].
    #[serde(default = "default_specular_factor")]
    pub specular_color_factor: [f64; 3],

    /// Specular color texture.
    #[serde(default)]
    pub specular_color_texture: Option<TextureTransformInfo>,
}

impl Default for Specular {
    fn default() -> Self {
        Self {
            specular_factor: 1.0,
            specular_texture: None,
            specular_color_factor: [1.0, 1.0, 1.0],
            specular_color_texture: None,
        }
    }
}

/// KHR_materials_clearcoat extension.
///
/// Maps to CesiumJS `ModelComponents.Clearcoat`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clearcoat {
    /// Clearcoat layer intensity. Default 0.0.
    #[serde(default)]
    pub clearcoat_factor: f64,

    /// Clearcoat intensity texture.
    #[serde(default)]
    pub clearcoat_texture: Option<TextureTransformInfo>,

    /// Clearcoat roughness. Default 0.0.
    #[serde(default)]
    pub clearcoat_roughness_factor: f64,

    /// Clearcoat roughness texture.
    #[serde(default)]
    pub clearcoat_roughness_texture: Option<TextureTransformInfo>,

    /// Clearcoat normal map texture.
    #[serde(default)]
    pub clearcoat_normal_texture: Option<NormalTextureInfo>,
}

impl Default for Clearcoat {
    fn default() -> Self {
        Self {
            clearcoat_factor: 0.0,
            clearcoat_texture: None,
            clearcoat_roughness_factor: 0.0,
            clearcoat_roughness_texture: None,
            clearcoat_normal_texture: None,
        }
    }
}

/// KHR_materials_anisotropy extension.
///
/// Maps to CesiumJS `ModelComponents.Anisotropy`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Anisotropy {
    /// Anisotropy strength. Default 0.0.
    #[serde(default)]
    pub anisotropy_strength: f64,

    /// Anisotropy rotation in radians. Default 0.0.
    #[serde(default)]
    pub anisotropy_rotation: f64,

    /// Anisotropy texture.
    #[serde(default)]
    pub anisotropy_texture: Option<TextureTransformInfo>,
}

impl Default for Anisotropy {
    fn default() -> Self {
        Self {
            anisotropy_strength: 0.0,
            anisotropy_rotation: 0.0,
            anisotropy_texture: None,
        }
    }
}

/// KHR_materials_transmission extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transmission {
    /// Transmission factor. Default 0.0.
    #[serde(default)]
    pub transmission_factor: f64,

    /// Transmission texture.
    #[serde(default)]
    pub transmission_texture: Option<TextureTransformInfo>,
}

impl Default for Transmission {
    fn default() -> Self {
        Self {
            transmission_factor: 0.0,
            transmission_texture: None,
        }
    }
}

/// KHR_materials_ior extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ior {
    /// Index of refraction. Default 1.5.
    #[serde(default = "default_ior")]
    pub ior: f64,
}

impl Default for Ior {
    fn default() -> Self {
        Self { ior: 1.5 }
    }
}

/// KHR_materials_emissive_strength extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmissiveStrength {
    /// Emissive strength multiplier. Default 1.0.
    #[serde(default = "default_one")]
    pub emissive_strength: f64,
}

impl Default for EmissiveStrength {
    fn default() -> Self {
        Self {
            emissive_strength: 1.0,
        }
    }
}

/// KHR_materials_sheen extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sheen {
    /// Sheen color factor [r, g, b]. Default [0,0,0].
    #[serde(default)]
    pub sheen_color_factor: [f64; 3],

    /// Sheen color texture.
    #[serde(default)]
    pub sheen_color_texture: Option<TextureTransformInfo>,

    /// Sheen roughness factor. Default 0.0.
    #[serde(default)]
    pub sheen_roughness_factor: f64,

    /// Sheen roughness texture.
    #[serde(default)]
    pub sheen_roughness_texture: Option<TextureTransformInfo>,
}

impl Default for Sheen {
    fn default() -> Self {
        Self {
            sheen_color_factor: [0.0, 0.0, 0.0],
            sheen_color_texture: None,
            sheen_roughness_factor: 0.0,
            sheen_roughness_texture: None,
        }
    }
}

/// KHR_materials_volume extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    /// Thickness factor. Default 0.0.
    #[serde(default)]
    pub thickness_factor: f64,

    /// Thickness texture.
    #[serde(default)]
    pub thickness_texture: Option<TextureTransformInfo>,

    /// Attenuation distance. Default +infinity.
    #[serde(default = "default_attenuation_distance")]
    pub attenuation_distance: f64,

    /// Attenuation color [r, g, b]. Default [1,1,1].
    #[serde(default = "default_specular_factor")]
    pub attenuation_color: [f64; 3],
}

impl Default for Volume {
    fn default() -> Self {
        Self {
            thickness_factor: 0.0,
            thickness_texture: None,
            attenuation_distance: f64::INFINITY,
            attenuation_color: [1.0, 1.0, 1.0],
        }
    }
}

/// Texture info with KHR_texture_transform extension support.
///
/// Maps to CesiumJS `ModelComponents.TextureReader`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureTransformInfo {
    /// Index of the texture.
    pub index: usize,

    /// Texture coordinate set.
    #[serde(default)]
    pub tex_coord: usize,

    /// KHR_texture_transform extension.
    #[serde(default)]
    pub extensions: Option<TextureTransformExtensions>,

    /// Normal map scale (only for normal textures).
    #[serde(default)]
    pub scale: Option<f64>,
}

/// Container for texture transform extension.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureTransformExtensions {
    /// KHR_texture_transform data.
    #[serde(default, rename = "KHR_texture_transform")]
    pub texture_transform: Option<TextureTransform>,
}

/// KHR_texture_transform extension data.
///
/// Provides UV transformation: offset, rotation, scale, and texCoord override.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureTransform {
    /// UV offset [u, v]. Default [0, 0].
    #[serde(default)]
    pub offset: [f64; 2],

    /// Rotation in radians (counter-clockwise). Default 0.
    #[serde(default)]
    pub rotation: f64,

    /// UV scale [u, v]. Default [1, 1].
    #[serde(default = "default_uv_scale")]
    pub scale: [f64; 2],

    /// Override texCoord set index.
    #[serde(default)]
    pub tex_coord: Option<usize>,
}

impl Default for TextureTransform {
    fn default() -> Self {
        Self {
            offset: [0.0, 0.0],
            rotation: 0.0,
            scale: [1.0, 1.0],
            tex_coord: None,
        }
    }
}

impl TextureTransform {
    /// Computes the 3x3 UV transformation matrix.
    ///
    /// The transformation order is: T(offset) * R(rotation) * S(scale).
    /// Maps to CesiumJS `GltfLoaderUtility.getTextureTransformMatrix`
    pub fn compute_matrix(&self) -> [f64; 9] {
        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();

        // Column-major 3x3: T * R * S
        // T = [1 0 ox; 0 1 oy; 0 0 1]
        // R = [cos -sin 0; sin cos 0; 0 0 1]
        // S = [sx 0 0; 0 sy 0; 0 0 1]
        let sx = self.scale[0];
        let sy = self.scale[1];
        let ox = self.offset[0];
        let oy = self.offset[1];

        // Combined: T * R * S (row-major for clarity, stored column-major)
        // Row 0: [cos*sx, -sin*sy, ox]
        // Row 1: [sin*sx,  cos*sy, oy]
        // Row 2: [0,       0,      1 ]
        [
            cos_r * sx,
            sin_r * sx,
            0.0, // column 0
            -sin_r * sy,
            cos_r * sy,
            0.0, // column 1
            ox,
            oy,
            1.0, // column 2
        ]
    }

    /// Transforms a UV coordinate using this transform.
    pub fn transform_uv(&self, u: f64, v: f64) -> [f64; 2] {
        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();

        let su = u * self.scale[0];
        let sv = v * self.scale[1];

        let ru = cos_r * su - sin_r * sv;
        let rv = sin_r * su + cos_r * sv;

        [ru + self.offset[0], rv + self.offset[1]]
    }
}

/// Normal texture info with scale.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalTextureInfo {
    /// Index of the texture.
    pub index: usize,

    /// Texture coordinate set.
    #[serde(default)]
    pub tex_coord: usize,

    /// Normal map scale. Default 1.0.
    #[serde(default = "default_one")]
    pub scale: f64,

    /// KHR_texture_transform extension.
    #[serde(default)]
    pub extensions: Option<TextureTransformExtensions>,
}

impl TextureTransformInfo {
    /// Creates from a basic TextureInfo.
    pub fn from_texture_info(info: &TextureInfo) -> Self {
        Self {
            index: info.index,
            tex_coord: info.tex_coord,
            extensions: None,
            scale: None,
        }
    }

    /// Gets the effective texCoord (considering KHR_texture_transform override).
    pub fn effective_tex_coord(&self) -> usize {
        self.extensions
            .as_ref()
            .and_then(|e| e.texture_transform.as_ref())
            .and_then(|t| t.tex_coord)
            .unwrap_or(self.tex_coord)
    }

    /// Gets the texture transform if present.
    pub fn get_transform(&self) -> Option<&TextureTransform> {
        self.extensions
            .as_ref()
            .and_then(|e| e.texture_transform.as_ref())
    }
}

/// Parses extended material from a glTF material's extensions JSON.
///
/// Maps to CesiumJS `GltfLoaderUtility` material extension parsing.
pub fn parse_material_extensions(
    extensions: &serde_json::Value,
) -> ExtendedMaterial {
    let mut mat = ExtendedMaterial::default();

    if let Some(obj) = extensions.as_object() {
        // KHR_materials_pbrSpecularGlossiness
        if let Some(sg) = obj.get("KHR_materials_pbrSpecularGlossiness") {
            mat.specular_glossiness =
                serde_json::from_value(sg.clone()).ok();
        }

        // KHR_materials_specular
        if let Some(sp) = obj.get("KHR_materials_specular") {
            mat.specular = serde_json::from_value(sp.clone()).ok();
        }

        // KHR_materials_clearcoat
        if let Some(cc) = obj.get("KHR_materials_clearcoat") {
            mat.clearcoat = serde_json::from_value(cc.clone()).ok();
        }

        // KHR_materials_anisotropy
        if let Some(an) = obj.get("KHR_materials_anisotropy") {
            mat.anisotropy = serde_json::from_value(an.clone()).ok();
        }

        // KHR_materials_transmission
        if let Some(tr) = obj.get("KHR_materials_transmission") {
            mat.transmission = serde_json::from_value(tr.clone()).ok();
        }

        // KHR_materials_ior
        if let Some(ior) = obj.get("KHR_materials_ior") {
            mat.ior = serde_json::from_value(ior.clone()).ok();
        }

        // KHR_materials_emissive_strength
        if let Some(es) = obj.get("KHR_materials_emissive_strength") {
            mat.emissive_strength = serde_json::from_value(es.clone()).ok();
        }

        // KHR_materials_sheen
        if let Some(sh) = obj.get("KHR_materials_sheen") {
            mat.sheen = serde_json::from_value(sh.clone()).ok();
        }

        // KHR_materials_volume
        if let Some(vol) = obj.get("KHR_materials_volume") {
            mat.volume = serde_json::from_value(vol.clone()).ok();
        }

        // KHR_materials_unlit
        if obj.contains_key("KHR_materials_unlit") {
            mat.unlit = true;
        }
    }

    mat
}

// Default value functions for serde
fn default_one() -> f64 {
    1.0
}

fn default_base_color_factor() -> [f64; 4] {
    [1.0, 1.0, 1.0, 1.0]
}

fn default_specular_factor() -> [f64; 3] {
    [1.0, 1.0, 1.0]
}

fn default_ior() -> f64 {
    1.5
}

fn default_attenuation_distance() -> f64 {
    f64::INFINITY
}

fn default_uv_scale() -> [f64; 2] {
    [1.0, 1.0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_metallic_roughness() {
        let mr = MetallicRoughness::default();
        assert_eq!(mr.base_color_factor, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(mr.metallic_factor, 1.0);
        assert_eq!(mr.roughness_factor, 1.0);
        assert!(mr.base_color_texture.is_none());
    }

    #[test]
    fn test_specular_glossiness_defaults() {
        let sg = SpecularGlossiness::default();
        assert_eq!(sg.diffuse_factor, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(sg.specular_factor, [1.0, 1.0, 1.0]);
        assert_eq!(sg.glossiness_factor, 1.0);
    }

    #[test]
    fn test_clearcoat_defaults() {
        let cc = Clearcoat::default();
        assert_eq!(cc.clearcoat_factor, 0.0);
        assert_eq!(cc.clearcoat_roughness_factor, 0.0);
    }

    #[test]
    fn test_anisotropy_defaults() {
        let an = Anisotropy::default();
        assert_eq!(an.anisotropy_strength, 0.0);
        assert_eq!(an.anisotropy_rotation, 0.0);
    }

    #[test]
    fn test_ior_default() {
        let ior = Ior::default();
        assert!((ior.ior - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_parse_specular_glossiness_extension() {
        let json = serde_json::json!({
            "KHR_materials_pbrSpecularGlossiness": {
                "diffuseFactor": [0.8, 0.2, 0.1, 1.0],
                "specularFactor": [0.5, 0.5, 0.5],
                "glossinessFactor": 0.9
            }
        });

        let mat = parse_material_extensions(&json);
        let sg = mat.specular_glossiness.unwrap();
        assert_eq!(sg.diffuse_factor, [0.8, 0.2, 0.1, 1.0]);
        assert_eq!(sg.specular_factor, [0.5, 0.5, 0.5]);
        assert!((sg.glossiness_factor - 0.9).abs() < 1e-10);
    }

    #[test]
    fn test_parse_clearcoat_extension() {
        let json = serde_json::json!({
            "KHR_materials_clearcoat": {
                "clearcoatFactor": 0.8,
                "clearcoatRoughnessFactor": 0.2
            }
        });

        let mat = parse_material_extensions(&json);
        let cc = mat.clearcoat.unwrap();
        assert!((cc.clearcoat_factor - 0.8).abs() < 1e-10);
        assert!((cc.clearcoat_roughness_factor - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_parse_unlit_extension() {
        let json = serde_json::json!({
            "KHR_materials_unlit": {}
        });

        let mat = parse_material_extensions(&json);
        assert!(mat.unlit);
    }

    #[test]
    fn test_parse_transmission_extension() {
        let json = serde_json::json!({
            "KHR_materials_transmission": {
                "transmissionFactor": 0.7
            }
        });

        let mat = parse_material_extensions(&json);
        let tr = mat.transmission.unwrap();
        assert!((tr.transmission_factor - 0.7).abs() < 1e-10);
    }

    #[test]
    fn test_parse_multiple_extensions() {
        let json = serde_json::json!({
            "KHR_materials_clearcoat": { "clearcoatFactor": 1.0 },
            "KHR_materials_ior": { "ior": 1.45 },
            "KHR_materials_unlit": {}
        });

        let mat = parse_material_extensions(&json);
        assert!(mat.clearcoat.is_some());
        assert!(mat.ior.is_some());
        assert!(mat.unlit);
        assert!(mat.specular_glossiness.is_none());
    }

    #[test]
    fn test_texture_transform_identity() {
        let transform = TextureTransform::default();
        let uv = transform.transform_uv(0.5, 0.5);
        assert!((uv[0] - 0.5).abs() < 1e-10);
        assert!((uv[1] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_texture_transform_offset() {
        let transform = TextureTransform {
            offset: [0.1, 0.2],
            ..Default::default()
        };
        let uv = transform.transform_uv(0.5, 0.5);
        assert!((uv[0] - 0.6).abs() < 1e-10);
        assert!((uv[1] - 0.7).abs() < 1e-10);
    }

    #[test]
    fn test_texture_transform_scale() {
        let transform = TextureTransform {
            scale: [2.0, 3.0],
            ..Default::default()
        };
        let uv = transform.transform_uv(0.5, 0.5);
        assert!((uv[0] - 1.0).abs() < 1e-10);
        assert!((uv[1] - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_texture_transform_rotation_90() {
        let transform = TextureTransform {
            rotation: std::f64::consts::FRAC_PI_2,
            ..Default::default()
        };
        let uv = transform.transform_uv(1.0, 0.0);
        // cos(90°)=0, sin(90°)=1 → (0*1 - 1*0, 1*1 + 0*0) = (0, 1)
        assert!(uv[0].abs() < 1e-10);
        assert!((uv[1] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_texture_transform_matrix_identity() {
        let transform = TextureTransform::default();
        let m = transform.compute_matrix();
        // Column-major identity
        assert!((m[0] - 1.0).abs() < 1e-10);
        assert!((m[4] - 1.0).abs() < 1e-10);
        assert!((m[8] - 1.0).abs() < 1e-10);
        assert!(m[1].abs() < 1e-10);
        assert!(m[3].abs() < 1e-10);
    }

    #[test]
    fn test_texture_transform_info_effective_tex_coord() {
        let info = TextureTransformInfo {
            index: 0,
            tex_coord: 0,
            extensions: Some(TextureTransformExtensions {
                texture_transform: Some(TextureTransform {
                    tex_coord: Some(1),
                    ..Default::default()
                }),
            }),
            scale: None,
        };
        assert_eq!(info.effective_tex_coord(), 1);
    }

    #[test]
    fn test_texture_transform_info_no_override() {
        let info = TextureTransformInfo {
            index: 0,
            tex_coord: 2,
            extensions: None,
            scale: None,
        };
        assert_eq!(info.effective_tex_coord(), 2);
    }

    #[test]
    fn test_extended_material_serde_roundtrip() {
        let mat = ExtendedMaterial {
            clearcoat: Some(Clearcoat {
                clearcoat_factor: 0.5,
                ..Default::default()
            }),
            unlit: true,
            ..Default::default()
        };

        let json = serde_json::to_string(&mat).unwrap();
        let parsed: ExtendedMaterial = serde_json::from_str(&json).unwrap();
        assert!(parsed.unlit);
        assert!((parsed.clearcoat.unwrap().clearcoat_factor - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_parse_sheen_extension() {
        let json = serde_json::json!({
            "KHR_materials_sheen": {
                "sheenColorFactor": [0.5, 0.3, 0.1],
                "sheenRoughnessFactor": 0.8
            }
        });

        let mat = parse_material_extensions(&json);
        let sheen = mat.sheen.unwrap();
        assert_eq!(sheen.sheen_color_factor, [0.5, 0.3, 0.1]);
        assert!((sheen.sheen_roughness_factor - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_parse_volume_extension() {
        let json = serde_json::json!({
            "KHR_materials_volume": {
                "thicknessFactor": 2.0,
                "attenuationDistance": 5.0,
                "attenuationColor": [0.9, 0.8, 0.7]
            }
        });

        let mat = parse_material_extensions(&json);
        let vol = mat.volume.unwrap();
        assert!((vol.thickness_factor - 2.0).abs() < 1e-10);
        assert!((vol.attenuation_distance - 5.0).abs() < 1e-10);
        assert_eq!(vol.attenuation_color, [0.9, 0.8, 0.7]);
    }
}
