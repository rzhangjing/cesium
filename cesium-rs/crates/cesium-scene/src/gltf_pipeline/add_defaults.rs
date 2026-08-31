//! Ported from `packages/engine/Source/Scene/GltfPipeline/addDefaults.js`.

use std::collections::HashSet;

use cesium_core::webgl_constants::WebGLConstants;
use serde_json::{json, Value};

use crate::gltf_pipeline::add_to_array::add_to_array_value;
use crate::gltf_pipeline::defined;
use crate::gltf_pipeline::for_each;
use crate::gltf_pipeline::get_accessor_byte_stride::get_accessor_byte_stride;

/// Adds default glTF values if they don't exist.
pub fn add_defaults(gltf: &mut Value) {
    for_each::accessor(gltf, |accessor, _id| {
        if defined(accessor.get("bufferView")) && accessor.get("byteOffset").map_or(true, |v| v.is_null()) {
            accessor["byteOffset"] = json!(0);
        }
        None::<()>
    });

    for_each::buffer_view(gltf, |buffer_view, _id| {
        if defined(buffer_view.get("buffer"))
            && buffer_view.get("byteOffset").map_or(true, |v| v.is_null())
        {
            buffer_view["byteOffset"] = json!(0);
        }
        None::<()>
    });

    // JS iterates meshes and pushes default materials into gltf.materials
    // inside the handler; the Rust port takes the meshes array out for the
    // duration of the loop to satisfy the borrow checker.
    let meshes = gltf.get_mut("meshes").map(|meshes| meshes.take());
    if let Some(Value::Array(mut meshes)) = meshes {
        for mesh in meshes.iter_mut() {
            for_each::mesh_primitive(mesh, |primitive, _index| {
                if primitive.get("mode").map_or(true, |mode| mode.is_null()) {
                    primitive["mode"] = json!(WebGLConstants::TRIANGLES);
                }
                if !defined(primitive.get("material")) {
                    // JS: gltf.materials = gltf.materials ?? []
                    let materials = gltf.get_mut("materials");
                    let materials = match materials {
                        Some(materials) if !materials.is_null() => materials,
                        Some(materials) => {
                            *materials = json!([]);
                            materials
                        }
                        None => {
                            gltf.as_object_mut().expect("gltf is an object")
                                .insert("materials".to_string(), json!([]));
                            gltf.get_mut("materials").expect("inserted above")
                        }
                    };
                    let default_material = json!({ "name": "default" });
                    let material_index = add_to_array_value(materials, default_material, false);
                    primitive["material"] = json!(material_index);
                }
                None::<()>
            });
        }
        gltf["meshes"] = Value::Array(meshes);
    }

    // Collect the accessor ids up front so the loop body may index back into
    // the glTF (the JS interleaves traversal and mutation).
    let vertex_attribute_ids = for_each::vertex_attribute_accessor_ids(gltf);
    for accessor_id in vertex_attribute_ids {
        let Some(accessor) = gltf["accessors"].get(accessor_id) else {
            continue;
        };
        if accessor.is_null() {
            continue;
        }
        let accessor = accessor.clone();
        gltf["accessors"][accessor_id]["normalized"] = accessor
            .get("normalized")
            .cloned()
            .filter(|value| !value.is_null())
            .unwrap_or(json!(false));
        if let Some(buffer_view_id) = accessor.get("bufferView").and_then(|v| v.as_u64()) {
            let byte_stride = get_accessor_byte_stride(gltf, &accessor);
            gltf["bufferViews"][buffer_view_id as usize]["byteStride"] = json!(byte_stride);
            gltf["bufferViews"][buffer_view_id as usize]["target"] =
                json!(WebGLConstants::ARRAY_BUFFER);
        }
    }

    let index_data_ids = for_each::index_data_accessor_ids(gltf);
    for accessor_id in index_data_ids {
        if let Some(buffer_view_id) = gltf["accessors"]
            .get(accessor_id)
            .and_then(|accessor| accessor.get("bufferView"))
            .and_then(|v| v.as_u64())
        {
            gltf["bufferViews"][buffer_view_id as usize]["target"] =
                json!(WebGLConstants::ELEMENT_ARRAY_BUFFER);
        }
    }

    for_each::material(gltf, |material, _id| {
        let has_materials_common = material
            .get("extensions")
            .and_then(|extensions| extensions.get("KHR_materials_common"))
            .map(|value| !value.is_null())
            .unwrap_or(false);
        if has_materials_common {
            let materials_common = material
                .pointer_mut("/extensions/KHR_materials_common")
                .expect("checked above");
            let technique = materials_common.get("technique").cloned();

            // materialsCommon.values = materialsCommon.values ?? {}
            if materials_common
                .get("values")
                .map_or(true, |values| values.is_null())
            {
                materials_common["values"] = json!({});
            }

            // values.ambient / values.emission default [0, 0, 0, 1]
            for name in ["ambient", "emission"] {
                if materials_common
                    .get("values")
                    .and_then(|values| values.get(name))
                    .map_or(true, |value| value.is_null())
                {
                    materials_common["values"][name] = json!([0.0, 0.0, 0.0, 1.0]);
                }
            }

            if materials_common
                .get("values")
                .and_then(|values| values.get("transparency"))
                .map_or(true, |value| value.is_null())
            {
                materials_common["values"]["transparency"] = json!(1.0);
            }

            if technique.as_ref().and_then(|t| t.as_str()) != Some("CONSTANT") {
                if materials_common
                    .get("values")
                    .and_then(|values| values.get("diffuse"))
                    .map_or(true, |value| value.is_null())
                {
                    materials_common["values"]["diffuse"] = json!([0.0, 0.0, 0.0, 1.0]);
                }
                if technique.as_ref().and_then(|t| t.as_str()) != Some("LAMBERT") {
                    if materials_common
                        .get("values")
                        .and_then(|values| values.get("specular"))
                        .map_or(true, |value| value.is_null())
                    {
                        materials_common["values"]["specular"] = json!([0.0, 0.0, 0.0, 1.0]);
                    }
                    if materials_common
                        .get("values")
                        .and_then(|values| values.get("shininess"))
                        .map_or(true, |value| value.is_null())
                    {
                        materials_common["values"]["shininess"] = json!(0.0);
                    }
                }
            }

            // These actually exist on the extension object, not the values
            // object despite what's shown in the spec
            if materials_common
                .get("transparent")
                .map_or(true, |value| value.is_null())
            {
                materials_common["transparent"] = json!(false);
            }
            if materials_common
                .get("doubleSided")
                .map_or(true, |value| value.is_null())
            {
                materials_common["doubleSided"] = json!(false);
            }

            return None;
        }

        if material.get("emissiveFactor").map_or(true, |v| v.is_null()) {
            material["emissiveFactor"] = json!([0.0, 0.0, 0.0]);
        }
        if material.get("alphaMode").map_or(true, |v| v.is_null()) {
            material["alphaMode"] = json!("OPAQUE");
        }
        if material.get("doubleSided").map_or(true, |v| v.is_null()) {
            material["doubleSided"] = json!(false);
        }

        if material.get("alphaMode").and_then(|v| v.as_str()) == Some("MASK")
            && material.get("alphaCutoff").map_or(true, |v| v.is_null())
        {
            material["alphaCutoff"] = json!(0.5);
        }

        let has_techniques_extension = material
            .get("extensions")
            .and_then(|extensions| extensions.get("KHR_techniques_webgl"))
            .map(|value| !value.is_null())
            .unwrap_or(false);
        if has_techniques_extension {
            for_each::material_value(material, |material_value, _name| {
                // Check if material value is a TextureInfo object
                if defined(material_value.get("index")) {
                    add_texture_defaults(Some(material_value));
                }
                None::<()>
            });
        }

        add_texture_defaults(material.get_mut("emissiveTexture"));
        add_texture_defaults(material.get_mut("normalTexture"));
        add_texture_defaults(material.get_mut("occlusionTexture"));

        let has_pbr = material
            .get("pbrMetallicRoughness")
            .map(|value| !value.is_null())
            .unwrap_or(false);
        if has_pbr {
            let pbr = material
                .get_mut("pbrMetallicRoughness")
                .expect("checked above");
            if pbr.get("baseColorFactor").map_or(true, |v| v.is_null()) {
                pbr["baseColorFactor"] = json!([1.0, 1.0, 1.0, 1.0]);
            }
            if pbr.get("metallicFactor").map_or(true, |v| v.is_null()) {
                pbr["metallicFactor"] = json!(1.0);
            }
            if pbr.get("roughnessFactor").map_or(true, |v| v.is_null()) {
                pbr["roughnessFactor"] = json!(1.0);
            }
            add_texture_defaults(pbr.get_mut("baseColorTexture"));
            add_texture_defaults(pbr.get_mut("metallicRoughnessTexture"));
        }

        let has_spec_gloss = material
            .get("extensions")
            .and_then(|extensions| extensions.get("KHR_materials_pbrSpecularGlossiness"))
            .map(|value| !value.is_null())
            .unwrap_or(false);
        if has_spec_gloss {
            let spec_gloss = material
                .pointer_mut("/extensions/KHR_materials_pbrSpecularGlossiness")
                .expect("checked above");
            if spec_gloss.get("diffuseFactor").map_or(true, |v| v.is_null()) {
                spec_gloss["diffuseFactor"] = json!([1.0, 1.0, 1.0, 1.0]);
            }
            if spec_gloss.get("specularFactor").map_or(true, |v| v.is_null()) {
                spec_gloss["specularFactor"] = json!([1.0, 1.0, 1.0]);
            }
            if spec_gloss.get("glossinessFactor").map_or(true, |v| v.is_null()) {
                spec_gloss["glossinessFactor"] = json!(1.0);
            }
            add_texture_defaults(spec_gloss.get_mut("specularGlossinessTexture"));
        }
        None::<()>
    });

    for_each::animation(gltf, |animation, _id| {
        for_each::animation_sampler(animation, |sampler, _index| {
            if sampler.get("interpolation").map_or(true, |v| v.is_null()) {
                sampler["interpolation"] = json!("LINEAR");
            }
            None::<()>
        });
        None::<()>
    });

    let animated_nodes = get_animated_nodes(gltf);
    for_each::node(gltf, |node, id| {
        let animated = id.parse::<u64>().map(|id| animated_nodes.contains(&id)).unwrap_or(false);
        if animated
            || defined(node.get("translation"))
            || defined(node.get("rotation"))
            || defined(node.get("scale"))
        {
            if node.get("translation").map_or(true, |v| v.is_null()) {
                node["translation"] = json!([0.0, 0.0, 0.0]);
            }
            if node.get("rotation").map_or(true, |v| v.is_null()) {
                node["rotation"] = json!([0.0, 0.0, 0.0, 1.0]);
            }
            if node.get("scale").map_or(true, |v| v.is_null()) {
                node["scale"] = json!([1.0, 1.0, 1.0]);
            }
        } else if node.get("matrix").map_or(true, |v| v.is_null()) {
            node["matrix"] = json!([
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0
            ]);
        }
        None::<()>
    });

    for_each::sampler(gltf, |sampler, _id| {
        if sampler.get("wrapS").map_or(true, |v| v.is_null()) {
            sampler["wrapS"] = json!(WebGLConstants::REPEAT);
        }
        if sampler.get("wrapT").map_or(true, |v| v.is_null()) {
            sampler["wrapT"] = json!(WebGLConstants::REPEAT);
        }
        None::<()>
    });

    if defined(gltf.get("scenes")) && !defined(gltf.get("scene")) {
        gltf["scene"] = json!(0);
    }
}

fn get_animated_nodes(gltf: &Value) -> HashSet<u64> {
    let mut nodes = HashSet::new();
    if let Some(animations) = gltf.get("animations").and_then(|a| a.as_array()) {
        for animation in animations {
            if let Some(channels) = animation.get("channels").and_then(|c| c.as_array()) {
                for channel in channels {
                    let Some(target) = channel.get("target") else {
                        continue;
                    };
                    let Some(node_id) = target.get("node").and_then(|node| node.as_u64()) else {
                        continue;
                    };
                    let path = target.get("path").and_then(|path| path.as_str());
                    // Ignore animations that target 'weights'
                    if matches!(path, Some("translation") | Some("rotation") | Some("scale")) {
                        nodes.insert(node_id);
                    }
                }
            }
        }
    }
    nodes
}

fn add_texture_defaults(texture: Option<&mut Value>) {
    let Some(texture) = texture.filter(|texture| !texture.is_null()) else {
        return;
    };
    if texture.get("texCoord").map_or(true, |v| v.is_null()) {
        texture["texCoord"] = json!(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessor_and_buffer_view_byte_offsets_defaulted() {
        let mut gltf = json!({
            "accessors": [{ "bufferView": 0 }],
            "bufferViews": [{ "buffer": 0 }]
        });
        add_defaults(&mut gltf);
        assert_eq!(gltf["accessors"][0]["byteOffset"], json!(0));
        assert_eq!(gltf["bufferViews"][0]["byteOffset"], json!(0));
    }

    #[test]
    fn primitives_get_mode_and_default_material() {
        let mut gltf = json!({
            "meshes": [
                { "primitives": [{ "attributes": {} }, { "attributes": {} }] }
            ]
        });
        add_defaults(&mut gltf);
        assert_eq!(gltf["meshes"][0]["primitives"][0]["mode"], json!(4));
        assert_eq!(gltf["meshes"][0]["primitives"][0]["material"], json!(0));
        assert_eq!(gltf["meshes"][0]["primitives"][1]["material"], json!(1));
        assert_eq!(gltf["materials"][0]["name"], json!("default"));
        assert_eq!(gltf["materials"][1]["name"], json!("default"));
    }

    #[test]
    fn vertex_and_index_accessor_buffer_view_targets() {
        let mut gltf = json!({
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "type": "VEC3" },
                { "bufferView": 1, "componentType": 5123, "type": "SCALAR" }
            ],
            "bufferViews": [{ "buffer": 0 }, { "buffer": 0 }],
            "meshes": [
                {
                    "primitives": [{ "attributes": { "POSITION": 0 }, "indices": 1 }]
                }
            ]
        });
        add_defaults(&mut gltf);
        assert_eq!(gltf["accessors"][0]["normalized"], json!(false));
        assert_eq!(gltf["bufferViews"][0]["byteStride"], json!(12));
        assert_eq!(gltf["bufferViews"][0]["target"], json!(0x8892));
        assert_eq!(gltf["bufferViews"][1]["target"], json!(0x8893));
    }

    #[test]
    fn material_pbr_defaults() {
        let mut gltf = json!({
            "materials": [
                {
                    "alphaMode": "MASK",
                    "pbrMetallicRoughness": {
                        "baseColorTexture": { "index": 0 }
                    }
                }
            ]
        });
        add_defaults(&mut gltf);
        let material = &gltf["materials"][0];
        assert_eq!(material["emissiveFactor"], json!([0.0, 0.0, 0.0]));
        assert_eq!(material["alphaCutoff"], json!(0.5));
        assert_eq!(material["doubleSided"], json!(false));
        let pbr = &material["pbrMetallicRoughness"];
        assert_eq!(pbr["baseColorFactor"], json!([1.0, 1.0, 1.0, 1.0]));
        assert_eq!(pbr["metallicFactor"], json!(1.0));
        assert_eq!(pbr["roughnessFactor"], json!(1.0));
        assert_eq!(pbr["baseColorTexture"]["texCoord"], json!(0));
    }

    #[test]
    fn materials_common_defaults() {
        let mut gltf = json!({
            "materials": [
                { "extensions": { "KHR_materials_common": { "technique": "PHONG" } } }
            ]
        });
        add_defaults(&mut gltf);
        let common = &gltf["materials"][0]["extensions"]["KHR_materials_common"];
        assert_eq!(common["values"]["ambient"], json!([0.0, 0.0, 0.0, 1.0]));
        assert_eq!(common["values"]["diffuse"], json!([0.0, 0.0, 0.0, 1.0]));
        assert_eq!(common["values"]["specular"], json!([0.0, 0.0, 0.0, 1.0]));
        assert_eq!(common["values"]["shininess"], json!(0.0));
        assert_eq!(common["values"]["transparency"], json!(1.0));
        assert_eq!(common["transparent"], json!(false));
        assert_eq!(common["doubleSided"], json!(false));
    }

    #[test]
    fn animated_nodes_get_trs_and_others_get_matrix() {
        let mut gltf = json!({
            "nodes": [{}, { "translation": [1.0, 2.0, 3.0] }, {}],
            "animations": [
                {
                    "channels": [{ "target": { "node": 0, "path": "rotation" } }]
                }
            ]
        });
        add_defaults(&mut gltf);
        assert_eq!(gltf["nodes"][0]["rotation"], json!([0.0, 0.0, 0.0, 1.0]));
        assert!(gltf["nodes"][0].get("matrix").is_none());
        assert_eq!(gltf["nodes"][1]["rotation"], json!([0.0, 0.0, 0.0, 1.0]));
        assert_eq!(gltf["nodes"][2]["matrix"][0], json!(1.0));
        assert_eq!(gltf["nodes"][2]["matrix"][15], json!(1.0));
        assert!(gltf["nodes"][2].get("translation").is_none());
    }

    #[test]
    fn sampler_wrap_and_default_scene() {
        let mut gltf = json!({
            "samplers": [{}],
            "scenes": [{ "nodes": [] }]
        });
        add_defaults(&mut gltf);
        assert_eq!(gltf["samplers"][0]["wrapS"], json!(0x2901));
        assert_eq!(gltf["samplers"][0]["wrapT"], json!(0x2901));
        assert_eq!(gltf["scene"], json!(0));
    }
}
