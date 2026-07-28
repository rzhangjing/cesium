//! Scene/ModelComponentsSpec.js + GltfLoaderUtilitySpec.js → Rust integration tests.
//!
//! Maps to CesiumJS:
//! - Scene/ModelComponents.js (Material, MetallicRoughness, all KHR extensions)
//! - Scene/Model/GltfLoaderUtility.js (extension parsing, texture transform)
//!
//! A-class tests: default values for all 10 KHR extensions, parse_material_extensions,
//! TextureTransform.compute_matrix/transform_uv, TextureTransformInfo.effective_tex_coord,
//! serde roundtrip.
//! C-class omitted: WebGL texture creation, shader generation.

use cesium_gltf::{
    Anisotropy, Clearcoat, EmissiveStrength, ExtendedMaterial, Ior,
    MetallicRoughness, Sheen, Specular, SpecularGlossiness, TextureTransform,
    TextureTransformExtensions, TextureTransformInfo, Transmission, Volume,
    parse_material_extensions,
};

// === Default values ===

#[test]
fn metallic_roughness_defaults() {
    let mr = MetallicRoughness::default();
    assert_eq!(mr.base_color_factor, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(mr.metallic_factor, 1.0);
    assert_eq!(mr.roughness_factor, 1.0);
    assert!(mr.base_color_texture.is_none());
    assert!(mr.metallic_roughness_texture.is_none());
}

#[test]
fn specular_glossiness_defaults() {
    let sg = SpecularGlossiness::default();
    assert_eq!(sg.diffuse_factor, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(sg.specular_factor, [1.0, 1.0, 1.0]);
    assert_eq!(sg.glossiness_factor, 1.0);
    assert!(sg.diffuse_texture.is_none());
}

#[test]
fn specular_defaults() {
    let sp = Specular::default();
    assert_eq!(sp.specular_factor, 1.0);
    assert_eq!(sp.specular_color_factor, [1.0, 1.0, 1.0]);
    assert!(sp.specular_texture.is_none());
    assert!(sp.specular_color_texture.is_none());
}

#[test]
fn clearcoat_defaults() {
    let cc = Clearcoat::default();
    assert_eq!(cc.clearcoat_factor, 0.0);
    assert_eq!(cc.clearcoat_roughness_factor, 0.0);
    assert!(cc.clearcoat_texture.is_none());
    assert!(cc.clearcoat_normal_texture.is_none());
}

#[test]
fn anisotropy_defaults() {
    let an = Anisotropy::default();
    assert_eq!(an.anisotropy_strength, 0.0);
    assert_eq!(an.anisotropy_rotation, 0.0);
    assert!(an.anisotropy_texture.is_none());
}

#[test]
fn transmission_defaults() {
    let tr = Transmission::default();
    assert_eq!(tr.transmission_factor, 0.0);
    assert!(tr.transmission_texture.is_none());
}

#[test]
fn ior_defaults() {
    let ior = Ior::default();
    assert!((ior.ior - 1.5).abs() < 1e-10);
}

#[test]
fn emissive_strength_defaults() {
    let es = EmissiveStrength::default();
    assert!((es.emissive_strength - 1.0).abs() < 1e-10);
}

#[test]
fn sheen_defaults() {
    let sh = Sheen::default();
    assert_eq!(sh.sheen_color_factor, [0.0, 0.0, 0.0]);
    assert_eq!(sh.sheen_roughness_factor, 0.0);
    assert!(sh.sheen_color_texture.is_none());
}

#[test]
fn volume_defaults() {
    let vol = Volume::default();
    assert_eq!(vol.thickness_factor, 0.0);
    assert_eq!(vol.attenuation_distance, f64::INFINITY);
    assert_eq!(vol.attenuation_color, [1.0, 1.0, 1.0]);
    assert!(vol.thickness_texture.is_none());
}

// === parse_material_extensions ===

#[test]
fn parse_empty_extensions() {
    let json = serde_json::json!({});
    let mat = parse_material_extensions(&json);
    assert!(mat.specular_glossiness.is_none());
    assert!(mat.clearcoat.is_none());
    assert!(!mat.unlit);
}

#[test]
fn parse_specular_glossiness() {
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
fn parse_clearcoat() {
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
fn parse_anisotropy() {
    let json = serde_json::json!({
        "KHR_materials_anisotropy": {
            "anisotropyStrength": 0.7,
            "anisotropyRotation": 1.57
        }
    });
    let mat = parse_material_extensions(&json);
    let an = mat.anisotropy.unwrap();
    assert!((an.anisotropy_strength - 0.7).abs() < 1e-10);
    assert!((an.anisotropy_rotation - 1.57).abs() < 1e-10);
}

#[test]
fn parse_transmission() {
    let json = serde_json::json!({
        "KHR_materials_transmission": {
            "transmissionFactor": 0.95
        }
    });
    let mat = parse_material_extensions(&json);
    let tr = mat.transmission.unwrap();
    assert!((tr.transmission_factor - 0.95).abs() < 1e-10);
}

#[test]
fn parse_ior() {
    let json = serde_json::json!({
        "KHR_materials_ior": { "ior": 1.45 }
    });
    let mat = parse_material_extensions(&json);
    let ior = mat.ior.unwrap();
    assert!((ior.ior - 1.45).abs() < 1e-10);
}

#[test]
fn parse_emissive_strength() {
    let json = serde_json::json!({
        "KHR_materials_emissive_strength": { "emissiveStrength": 5.0 }
    });
    let mat = parse_material_extensions(&json);
    let es = mat.emissive_strength.unwrap();
    assert!((es.emissive_strength - 5.0).abs() < 1e-10);
}

#[test]
fn parse_sheen() {
    let json = serde_json::json!({
        "KHR_materials_sheen": {
            "sheenColorFactor": [0.5, 0.3, 0.1],
            "sheenRoughnessFactor": 0.8
        }
    });
    let mat = parse_material_extensions(&json);
    let sh = mat.sheen.unwrap();
    assert_eq!(sh.sheen_color_factor, [0.5, 0.3, 0.1]);
    assert!((sh.sheen_roughness_factor - 0.8).abs() < 1e-10);
}

#[test]
fn parse_volume() {
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

#[test]
fn parse_unlit() {
    let json = serde_json::json!({
        "KHR_materials_unlit": {}
    });
    let mat = parse_material_extensions(&json);
    assert!(mat.unlit);
}

#[test]
fn parse_multiple_extensions_combined() {
    let json = serde_json::json!({
        "KHR_materials_clearcoat": { "clearcoatFactor": 1.0 },
        "KHR_materials_ior": { "ior": 1.45 },
        "KHR_materials_transmission": { "transmissionFactor": 0.5 },
        "KHR_materials_unlit": {}
    });
    let mat = parse_material_extensions(&json);
    assert!(mat.clearcoat.is_some());
    assert!(mat.ior.is_some());
    assert!(mat.transmission.is_some());
    assert!(mat.unlit);
    assert!(mat.specular_glossiness.is_none());
    assert!(mat.sheen.is_none());
}

// === TextureTransform ===

#[test]
fn texture_transform_default_is_identity() {
    let tt = TextureTransform::default();
    assert_eq!(tt.offset, [0.0, 0.0]);
    assert_eq!(tt.rotation, 0.0);
    assert_eq!(tt.scale, [1.0, 1.0]);
    assert!(tt.tex_coord.is_none());

    let uv = tt.transform_uv(0.3, 0.7);
    assert!((uv[0] - 0.3).abs() < 1e-10);
    assert!((uv[1] - 0.7).abs() < 1e-10);
}

#[test]
fn texture_transform_offset() {
    let tt = TextureTransform {
        offset: [0.1, 0.2],
        ..Default::default()
    };
    let uv = tt.transform_uv(0.5, 0.5);
    assert!((uv[0] - 0.6).abs() < 1e-10);
    assert!((uv[1] - 0.7).abs() < 1e-10);
}

#[test]
fn texture_transform_scale() {
    let tt = TextureTransform {
        scale: [2.0, 3.0],
        ..Default::default()
    };
    let uv = tt.transform_uv(0.5, 0.5);
    assert!((uv[0] - 1.0).abs() < 1e-10);
    assert!((uv[1] - 1.5).abs() < 1e-10);
}

#[test]
fn texture_transform_rotation_90() {
    let tt = TextureTransform {
        rotation: std::f64::consts::FRAC_PI_2,
        ..Default::default()
    };
    // (1, 0) rotated 90° CCW → (0, 1)
    let uv = tt.transform_uv(1.0, 0.0);
    assert!(uv[0].abs() < 1e-10);
    assert!((uv[1] - 1.0).abs() < 1e-10);
}

#[test]
fn texture_transform_combined_srt() {
    // T(0.5, 0.5) * R(90°) * S(2, 2) applied to (1, 0)
    // Scale: (2, 0), Rotate 90°: (0, 2), Offset: (0.5, 2.5)
    let tt = TextureTransform {
        offset: [0.5, 0.5],
        rotation: std::f64::consts::FRAC_PI_2,
        scale: [2.0, 2.0],
        tex_coord: None,
    };
    let uv = tt.transform_uv(1.0, 0.0);
    assert!((uv[0] - 0.5).abs() < 1e-10);
    assert!((uv[1] - 2.5).abs() < 1e-10);
}

#[test]
fn texture_transform_compute_matrix_identity() {
    let tt = TextureTransform::default();
    let m = tt.compute_matrix();
    // Column-major 3x3 identity
    assert!((m[0] - 1.0).abs() < 1e-10);
    assert!((m[4] - 1.0).abs() < 1e-10);
    assert!((m[8] - 1.0).abs() < 1e-10);
    assert!(m[1].abs() < 1e-10);
    assert!(m[3].abs() < 1e-10);
    assert!(m[6].abs() < 1e-10);
    assert!(m[7].abs() < 1e-10);
}

#[test]
fn texture_transform_compute_matrix_scale_offset() {
    let tt = TextureTransform {
        offset: [0.5, 0.25],
        rotation: 0.0,
        scale: [2.0, 4.0],
        tex_coord: None,
    };
    let m = tt.compute_matrix();
    // cos=1, sin=0 → col0=[sx, 0, 0], col1=[0, sy, 0], col2=[ox, oy, 1]
    assert!((m[0] - 2.0).abs() < 1e-10); // cos*sx
    assert!((m[4] - 4.0).abs() < 1e-10); // cos*sy
    assert!((m[6] - 0.5).abs() < 1e-10); // ox
    assert!((m[7] - 0.25).abs() < 1e-10); // oy
}

// === TextureTransformInfo ===

#[test]
fn texture_transform_info_effective_tex_coord_override() {
    let info = TextureTransformInfo {
        index: 0,
        tex_coord: 0,
        extensions: Some(TextureTransformExtensions {
            texture_transform: Some(TextureTransform {
                tex_coord: Some(2),
                ..Default::default()
            }),
        }),
        scale: None,
    };
    assert_eq!(info.effective_tex_coord(), 2);
}

#[test]
fn texture_transform_info_effective_tex_coord_no_override() {
    let info = TextureTransformInfo {
        index: 0,
        tex_coord: 3,
        extensions: None,
        scale: None,
    };
    assert_eq!(info.effective_tex_coord(), 3);
}

#[test]
fn texture_transform_info_get_transform() {
    let info = TextureTransformInfo {
        index: 1,
        tex_coord: 0,
        extensions: Some(TextureTransformExtensions {
            texture_transform: Some(TextureTransform {
                offset: [0.5, 0.5],
                ..Default::default()
            }),
        }),
        scale: None,
    };
    let tt = info.get_transform().unwrap();
    assert_eq!(tt.offset, [0.5, 0.5]);
}

#[test]
fn texture_transform_info_get_transform_none() {
    let info = TextureTransformInfo {
        index: 0,
        tex_coord: 0,
        extensions: None,
        scale: None,
    };
    assert!(info.get_transform().is_none());
}

// === ExtendedMaterial serde roundtrip ===

#[test]
fn extended_material_serde_roundtrip() {
    let mat = ExtendedMaterial {
        metallic_roughness: MetallicRoughness {
            base_color_factor: [0.5, 0.6, 0.7, 1.0],
            metallic_factor: 0.3,
            roughness_factor: 0.8,
            ..Default::default()
        },
        clearcoat: Some(Clearcoat {
            clearcoat_factor: 0.5,
            ..Default::default()
        }),
        ior: Some(Ior { ior: 1.45 }),
        unlit: true,
        ..Default::default()
    };

    let json = serde_json::to_string(&mat).unwrap();
    let parsed: ExtendedMaterial = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.metallic_roughness.base_color_factor, [0.5, 0.6, 0.7, 1.0]);
    assert!((parsed.metallic_roughness.metallic_factor - 0.3).abs() < 1e-10);
    assert!((parsed.clearcoat.unwrap().clearcoat_factor - 0.5).abs() < 1e-10);
    assert!((parsed.ior.unwrap().ior - 1.45).abs() < 1e-10);
    assert!(parsed.unlit);
    assert!(parsed.specular_glossiness.is_none());
}

#[test]
fn extended_material_default_all_none() {
    let mat = ExtendedMaterial::default();
    assert!(mat.specular_glossiness.is_none());
    assert!(mat.specular.is_none());
    assert!(mat.clearcoat.is_none());
    assert!(mat.anisotropy.is_none());
    assert!(mat.transmission.is_none());
    assert!(mat.ior.is_none());
    assert!(mat.emissive_strength.is_none());
    assert!(mat.sheen.is_none());
    assert!(mat.volume.is_none());
    assert!(!mat.unlit);
    // Base metallic_roughness should have defaults
    assert_eq!(mat.metallic_roughness.metallic_factor, 1.0);
}
