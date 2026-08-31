//! Ported from
//! `packages/engine/Source/Scene/GltfPipeline/moveTechniquesToExtension.js`.

use std::collections::HashMap;

use serde_json::{json, Map, Value};

use crate::gltf_pipeline::add_extensions_required::add_extensions_required;
use crate::gltf_pipeline::add_extensions_used::add_extensions_used;
use crate::gltf_pipeline::add_to_array::add_to_array;
use crate::gltf_pipeline::defined;
use crate::gltf_pipeline::for_each;
use crate::gltf_pipeline::key_string;

/// Moves glTF 1.0 material techniques to the glTF 2.0 `KHR_techniques_webgl`
/// extension.
///
/// DEVIATION: JavaScript keeps `undefined` properties on intermediate objects
/// (dropped by `JSON.stringify`); the Rust port omits such fields outright,
/// which is observable-equivalent for JSON consumers.
pub fn move_techniques_to_extension(gltf: &mut Value) {
    let mut mapped_uniforms: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut updated_technique_indices: HashMap<String, u64> = HashMap::new();
    let mut seen_programs: HashMap<String, u64> = HashMap::new();

    let techniques_legacy_defined = defined(gltf.get("techniques"));
    if techniques_legacy_defined {
        let mut extension = json!({
            "programs": [],
            "shaders": [],
            "techniques": []
        });

        // Some 1.1 models have a glExtensionsUsed property that can be
        // transferred to program.glExtensions
        let gl_extensions = gltf
            .get("glExtensionsUsed")
            .filter(|value| !value.is_null())
            .cloned();
        if let Some(root) = gltf.as_object_mut() {
            root.remove("glExtensionsUsed");
        }

        // Take the techniques array/object out of the glTF so the loop may
        // read gltf.programs / gltf.shaders; gltf.techniques is deleted at
        // the end of the function anyway.
        let techniques_value = gltf
            .get_mut("techniques")
            .map(|techniques| techniques.take())
            .unwrap_or(Value::Null);
        let technique_pairs: Vec<(String, Value)> = match techniques_value {
            Value::Array(list) => list
                .into_iter()
                .enumerate()
                .map(|(index, value)| (index.to_string(), value))
                .collect(),
            Value::Object(map) => map.into_iter().collect(),
            _ => Vec::new(),
        };

        for (technique_id, technique_legacy) in technique_pairs {
            let mut technique = json!({
                "attributes": {},
                "uniforms": {}
            });
            if let Some(name) = technique_legacy.get("name").filter(|v| !v.is_null()) {
                technique["name"] = name.clone();
            }

            // technique.attributes[attributeName] = { semantic: parameter.semantic }
            let attribute_entries: Vec<(String, Value)> = technique_legacy
                .get("attributes")
                .and_then(|attributes| attributes.as_object())
                .map(|attributes| attributes.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                .unwrap_or_default();
            for (attribute_name, parameter_name) in attribute_entries {
                let parameter_legacy = technique_legacy
                    .get("parameters")
                    .and_then(|parameters| parameters.get(key_string(&parameter_name)));
                let mut attribute = Map::new();
                if let Some(semantic) =
                    parameter_legacy.and_then(|p| p.get("semantic")).filter(|v| !v.is_null())
                {
                    attribute.insert("semantic".to_string(), semantic.clone());
                }
                technique["attributes"][attribute_name] = Value::Object(attribute);
            }

            // technique.uniforms[uniformName] = { count, node, type, semantic, value }
            let uniform_entries: Vec<(String, Value)> = technique_legacy
                .get("uniforms")
                .and_then(|uniforms| uniforms.as_object())
                .map(|uniforms| uniforms.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                .unwrap_or_default();
            for (uniform_name, parameter_name) in uniform_entries {
                let parameter_legacy = technique_legacy
                    .get("parameters")
                    .and_then(|parameters| parameters.get(key_string(&parameter_name)));
                let mut uniform = Map::new();
                if let Some(parameter) = parameter_legacy {
                    for field in ["count", "node", "type", "semantic", "value"] {
                        if let Some(value) = parameter.get(field).filter(|v| !v.is_null()) {
                            uniform.insert(field.to_string(), value.clone());
                        }
                    }
                }
                technique["uniforms"][uniform_name.clone()] = Value::Object(uniform);

                // Store the name of the uniform to update material values.
                mapped_uniforms
                    .entry(technique_id.clone())
                    .or_default()
                    .insert(key_string(&parameter_name), uniform_name);
            }

            let program_id = key_string(technique_legacy.get("program").unwrap_or(&Value::Null));
            if let Some(&program_index) = seen_programs.get(&program_id) {
                technique["program"] = json!(program_index);
            } else {
                let program_legacy = gltf
                    .get("programs")
                    .and_then(|programs| programs.get(program_id.parse::<usize>().unwrap_or(usize::MAX)))
                    .cloned()
                    .or_else(|| {
                        gltf.get("programs")
                            .and_then(|programs| programs.get(&program_id))
                            .cloned()
                    })
                    .unwrap_or(Value::Null);

                let mut program = Map::new();
                if let Some(name) = program_legacy.get("name").filter(|v| !v.is_null()) {
                    program.insert("name".to_string(), name.clone());
                }
                if let Some(gl_extensions) = gl_extensions.as_ref().filter(|v| !v.is_null()) {
                    program.insert("glExtensions".to_string(), gl_extensions.clone());
                }

                let fs = shader_by_id(gltf, program_legacy.get("fragmentShader"));
                let fragment_index = {
                    let shaders = extension["shaders"].as_array_mut().expect("extension shape");
                    add_to_array(shaders, fs, true)
                };
                program.insert("fragmentShader".to_string(), json!(fragment_index));

                let vs = shader_by_id(gltf, program_legacy.get("vertexShader"));
                let vertex_index = {
                    let shaders = extension["shaders"].as_array_mut().expect("extension shape");
                    add_to_array(shaders, vs, true)
                };
                program.insert("vertexShader".to_string(), json!(vertex_index));

                let program_index = add_to_array(
                    extension["programs"].as_array_mut().expect("extension shape"),
                    Value::Object(program),
                    false,
                );
                technique["program"] = json!(program_index);
                seen_programs.insert(program_id, program_index as u64);
            }

            // Store the index of the new technique to reference instead.
            let technique_index = add_to_array(
                extension["techniques"].as_array_mut().expect("extension shape"),
                technique,
                false,
            );
            updated_technique_indices.insert(technique_id, technique_index as u64);
        }

        if extension["techniques"].as_array().map(|t| !t.is_empty()).unwrap_or(false) {
            if !defined(gltf.get("extensions")) {
                gltf["extensions"] = json!({});
            }
            gltf["extensions"]["KHR_techniques_webgl"] = extension;
            add_extensions_used(gltf, "KHR_techniques_webgl");
            add_extensions_required(gltf, "KHR_techniques_webgl");
        }
    }

    for_each::material(gltf, |material, _id| {
        let technique_id = material
            .get("technique")
            .filter(|value| !value.is_null())
            .map(|value| key_string(value));
        if let Some(technique_id) = technique_id {
            let mut material_extension = json!({
                "technique": updated_technique_indices.get(&technique_id).copied()
            });

            let values = material.get("values").cloned().unwrap_or(Value::Null);
            if let Some(values_map) = values.as_object() {
                for (parameter_name, value) in values_map {
                    if material_extension.get("values").map_or(true, |v| v.is_null()) {
                        material_extension["values"] = json!({});
                    }
                    let uniform_name = mapped_uniforms
                        .get(&technique_id)
                        .and_then(|uniforms| uniforms.get(parameter_name));
                    if let Some(uniform_name) = uniform_name {
                        material_extension["values"][uniform_name.clone()] = value.clone();
                    }
                }
            }

            if !defined(material.get("extensions")) {
                material["extensions"] = json!({});
            }
            material["extensions"]["KHR_techniques_webgl"] = material_extension;
        }

        if let Some(material_object) = material.as_object_mut() {
            material_object.remove("technique");
            material_object.remove("values");
        }
        None::<()>
    });

    if let Some(root) = gltf.as_object_mut() {
        root.remove("techniques");
        root.remove("programs");
        root.remove("shaders");
    }
}

fn shader_by_id(gltf: &Value, shader_id: Option<&Value>) -> Value {
    let Some(shader_id) = shader_id.filter(|value| !value.is_null()) else {
        return Value::Null;
    };
    if let Some(index) = shader_id.as_u64() {
        return gltf
            .get("shaders")
            .and_then(|shaders| shaders.get(index as usize))
            .cloned()
            .unwrap_or(Value::Null);
    }
    gltf.get("shaders")
        .and_then(|shaders| shaders.get(key_string(shader_id)))
        .cloned()
        .unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_gltf() -> Value {
        json!({
            "techniques": [
                {
                    "name": "technique0",
                    "parameters": {
                        "diffuse": { "type": 35678, "semantic": "TEXTURE" },
                        "position": { "type": 35665, "semantic": "POSITION" }
                    },
                    "attributes": { "a_position": "position" },
                    "uniforms": { "u_diffuse": "diffuse" },
                    "program": 0
                }
            ],
            "programs": [
                { "name": "program0", "fragmentShader": 1, "vertexShader": 0 }
            ],
            "shaders": [
                { "type": 35633, "uri": "vs.glsl" },
                { "type": 35632, "uri": "fs.glsl" }
            ],
            "materials": [
                {
                    "technique": 0,
                    "values": { "diffuse": { "index": 0 } }
                }
            ]
        })
    }

    #[test]
    fn moves_techniques_into_khr_extension() {
        let mut gltf = legacy_gltf();
        move_techniques_to_extension(&mut gltf);

        let extension = &gltf["extensions"]["KHR_techniques_webgl"];
        assert_eq!(extension["techniques"][0]["name"], json!("technique0"));
        assert_eq!(
            extension["techniques"][0]["attributes"]["a_position"]["semantic"],
            json!("POSITION")
        );
        assert_eq!(
            extension["techniques"][0]["uniforms"]["u_diffuse"]["semantic"],
            json!("TEXTURE")
        );
        // JS adds the fragment shader to `extension.shaders` before the
        // vertex shader, so fs.glsl lands at index 0 and vs.glsl at 1.
        assert_eq!(extension["programs"][0]["vertexShader"], json!(1));
        assert_eq!(extension["programs"][0]["fragmentShader"], json!(0));
        assert_eq!(extension["shaders"][0]["uri"], json!("fs.glsl"));
        assert_eq!(extension["shaders"][1]["uri"], json!("vs.glsl"));

        assert_eq!(gltf["extensionsUsed"], json!(["KHR_techniques_webgl"]));
        assert_eq!(gltf["extensionsRequired"], json!(["KHR_techniques_webgl"]));

        let material = &gltf["materials"][0];
        assert_eq!(
            material["extensions"]["KHR_techniques_webgl"]["technique"],
            json!(0)
        );
        assert_eq!(
            material["extensions"]["KHR_techniques_webgl"]["values"]["u_diffuse"],
            json!({ "index": 0 })
        );
        assert!(material.get("technique").is_none());
        assert!(material.get("values").is_none());
        assert!(gltf.get("techniques").is_none());
        assert!(gltf.get("programs").is_none());
        assert!(gltf.get("shaders").is_none());
    }

    #[test]
    fn shared_program_is_not_duplicated() {
        let mut gltf = json!({
            "techniques": [
                {
                    "program": 0,
                    "attributes": {},
                    "uniforms": {}
                },
                {
                    "program": 0,
                    "attributes": {},
                    "uniforms": {}
                }
            ],
            "programs": [{ "fragmentShader": 0, "vertexShader": 0 }],
            "shaders": [{ "uri": "shader.glsl" }]
        });
        move_techniques_to_extension(&mut gltf);
        let extension = &gltf["extensions"]["KHR_techniques_webgl"];
        assert_eq!(extension["programs"].as_array().unwrap().len(), 1);
        assert_eq!(extension["shaders"].as_array().unwrap().len(), 1);
        assert_eq!(extension["techniques"][0]["program"], json!(0));
        assert_eq!(extension["techniques"][1]["program"], json!(0));
    }

    #[test]
    fn gl_extensions_used_moves_to_program() {
        let mut gltf = json!({
            "glExtensionsUsed": ["OES_standard_derivatives"],
            "techniques": [
                { "program": 0, "attributes": {}, "uniforms": {} }
            ],
            "programs": [{ "fragmentShader": 0, "vertexShader": 0 }],
            "shaders": [{ "uri": "shader.glsl" }]
        });
        move_techniques_to_extension(&mut gltf);
        assert!(gltf.get("glExtensionsUsed").is_none());
        assert_eq!(
            gltf["extensions"]["KHR_techniques_webgl"]["programs"][0]["glExtensions"],
            json!(["OES_standard_derivatives"])
        );
    }

    #[test]
    fn material_values_deleted_without_techniques() {
        let mut gltf = json!({ "materials": [{ "values": { "a": 1 } }] });
        move_techniques_to_extension(&mut gltf);
        assert!(gltf["materials"][0].get("values").is_none());
    }
}
