//! Ported from `packages/engine/Source/Scene/GltfPipeline/forEachTextureInMaterial.js`.

use serde_json::Value;

use crate::gltf_pipeline::defined;
use crate::gltf_pipeline::for_each;

/// Calls the handler on each texture used by the material, in the same order
/// as the JavaScript original:
/// metallic-roughness textures, then the `KHR_materials_pbrSpecularGlossiness`,
/// `KHR_materials_specular`, `KHR_materials_transmission` and
/// `KHR_materials_common` extension textures, then technique material values,
/// then the top level emissive/normal/occlusion textures.
///
/// If the handler returns `Some(value)`, iteration stops and that value is
/// returned (mirroring the JS `if (defined(value)) return value;` pattern).
///
/// # Panics
/// Debug builds panic when `material` is not an object (the JS
/// `Check.typeOf.object` assertion).
pub fn for_each_texture_in_material<T>(
    material: &mut Value,
    mut handler: impl FnMut(&Value, &mut Value) -> Option<T>,
) -> Option<T> {
    // Check.typeOf.object("material", material)
    debug_assert!(material.is_object(), "material must be an object");

    // Metallic roughness
    let pbr = material
        .get("pbrMetallicRoughness")
        .filter(|value| !value.is_null())
        .map(|_| ());
    if pbr.is_some() {
        let result = for_each_texture_info(
            material.pointer_mut("/pbrMetallicRoughness/baseColorTexture"),
            &mut handler,
        );
        if result.is_some() {
            return result;
        }
        let result = for_each_texture_info(
            material.pointer_mut("/pbrMetallicRoughness/metallicRoughnessTexture"),
            &mut handler,
        );
        if result.is_some() {
            return result;
        }
    }

    let has_extensions = material
        .get("extensions")
        .map(|value| !value.is_null())
        .unwrap_or(false);
    if has_extensions {
        // Spec gloss extension
        let has_spec_gloss = defined(
            material
                .get("extensions")
                .and_then(|extensions| extensions.get("KHR_materials_pbrSpecularGlossiness")),
        );
        if has_spec_gloss {
            let result = for_each_texture_info(
                material.pointer_mut(
                    "/extensions/KHR_materials_pbrSpecularGlossiness/diffuseTexture",
                ),
                &mut handler,
            );
            if result.is_some() {
                return result;
            }
            let result = for_each_texture_info(
                material.pointer_mut(
                    "/extensions/KHR_materials_pbrSpecularGlossiness/specularGlossinessTexture",
                ),
                &mut handler,
            );
            if result.is_some() {
                return result;
            }
        }

        // Specular extension
        let has_specular = defined(
            material
                .get("extensions")
                .and_then(|extensions| extensions.get("KHR_materials_specular")),
        );
        if has_specular {
            let result = for_each_texture_info(
                material.pointer_mut("/extensions/KHR_materials_specular/specularTexture"),
                &mut handler,
            );
            if result.is_some() {
                return result;
            }
            let result = for_each_texture_info(
                material.pointer_mut("/extensions/KHR_materials_specular/specularColorTexture"),
                &mut handler,
            );
            if result.is_some() {
                return result;
            }
        }

        // Transmission extension
        let has_transmission = defined(
            material
                .get("extensions")
                .and_then(|extensions| extensions.get("KHR_materials_transmission")),
        );
        if has_transmission {
            let result = for_each_texture_info(
                material
                    .pointer_mut("/extensions/KHR_materials_transmission/transmissionTexture"),
                &mut handler,
            );
            if result.is_some() {
                return result;
            }
        }

        // Materials common extension (may be present in models converted from glTF 1.0)
        let has_materials_common = defined(
            material
                .get("extensions")
                .and_then(|extensions| extensions.get("KHR_materials_common"))
                .and_then(|materials_common| materials_common.get("values")),
        );
        if has_materials_common {
            for value_name in ["diffuse", "ambient", "emission", "specular"] {
                let present = material
                    .get("extensions")
                    .and_then(|extensions| extensions.get("KHR_materials_common"))
                    .and_then(|materials_common| materials_common.get("values"))
                    .and_then(|values| values.get(value_name))
                    .and_then(|value| value.get("index"))
                    .map(|value| !value.is_null())
                    .unwrap_or(false);
                if present {
                    let pointer = format!("/extensions/KHR_materials_common/values/{value_name}");
                    let result = for_each_texture_info(material.pointer_mut(&pointer), &mut handler);
                    if result.is_some() {
                        return result;
                    }
                }
            }
        }
    }

    // KHR_techniques_webgl extension
    let result = for_each::material_value(material, |material_value, _name| {
        let has_index = defined(material_value.get("index"));
        if has_index {
            let index = material_value.get("index").expect("checked above").clone();
            let value = handler(&index, material_value);
            if value.is_some() {
                return value;
            }
        }
        None
    });
    if result.is_some() {
        return result;
    }

    // Top level textures
    for name in ["emissiveTexture", "normalTexture", "occlusionTexture"] {
        let pointer = format!("/{name}");
        let result = for_each_texture_info(material.pointer_mut(&pointer), &mut handler);
        if result.is_some() {
            return result;
        }
    }

    None
}

fn for_each_texture_info<T>(
    texture_info: Option<&mut Value>,
    handler: &mut impl FnMut(&Value, &mut Value) -> Option<T>,
) -> Option<T> {
    let texture_info = texture_info.filter(|value| !value.is_null())?;
    let index = texture_info.get("index")?.clone();
    handler(&index, texture_info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn visits_pbr_and_top_level_textures_in_order() {
        let mut material = json!({
            "pbrMetallicRoughness": {
                "baseColorTexture": { "index": 0, "texCoord": 0 },
                "metallicRoughnessTexture": { "index": 1 }
            },
            "emissiveTexture": { "index": 2 },
            "normalTexture": { "index": 3 },
            "occlusionTexture": { "index": 4 }
        });
        let mut indices = Vec::new();
        for_each_texture_in_material(&mut material, |index, _info| {
            indices.push(index.as_u64().unwrap());
            None::<()>
        });
        assert_eq!(indices, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn visits_extension_textures() {
        let mut material = json!({
            "extensions": {
                "KHR_materials_pbrSpecularGlossiness": {
                    "diffuseTexture": { "index": 5 },
                    "specularGlossinessTexture": { "index": 6 }
                },
                "KHR_materials_specular": {
                    "specularTexture": { "index": 7 },
                    "specularColorTexture": { "index": 8 }
                },
                "KHR_materials_transmission": {
                    "transmissionTexture": { "index": 9 }
                },
                "KHR_materials_common": {
                    "values": {
                        "diffuse": { "index": 10 },
                        "emission": { "index": 11 }
                    }
                }
            }
        });
        let mut indices = Vec::new();
        for_each_texture_in_material(&mut material, |index, _info| {
            indices.push(index.as_u64().unwrap());
            None::<()>
        });
        assert_eq!(indices, vec![5, 6, 7, 8, 9, 10, 11]);
    }

    #[test]
    fn handler_short_circuits_on_defined_value() {
        let mut material = json!({
            "pbrMetallicRoughness": {
                "baseColorTexture": { "index": 0 },
                "metallicRoughnessTexture": { "index": 1 }
            }
        });
        let found = for_each_texture_in_material(&mut material, |index, _info| {
            if index.as_u64() == Some(1) {
                Some(42u32)
            } else {
                None
            }
        });
        assert_eq!(found, Some(42));
    }

    #[test]
    fn handler_can_mutate_texture_info() {
        let mut material = json!({
            "normalTexture": { "index": 2 }
        });
        for_each_texture_in_material(&mut material, |_index, info| {
            info["index"] = json!(10);
            None::<()>
        });
        assert_eq!(material["normalTexture"]["index"], json!(10));
    }
}
