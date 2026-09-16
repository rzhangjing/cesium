//! glTF 1.0 technique / material → glTF 2.0 PBR material migration.
//!
//! Mirrors CesiumJS `packages/engine/Source/Scene/GltfPipeline/`:
//! `moveTechniqueRenderStates.js`, `moveTechniquesToExtension.js`, and the two
//! legacy-extension converters at the tail of `updateVersion.js`
//! (`convertTechniquesToPbr` L1040, `convertMaterialsCommonToPbr` L1084).
//!
//! These run after the structural 1.0 → 2.0 transform, so top-level collections
//! are already arrays. Everything operates on raw [`serde_json::Value`].

use serde_json::{json, Map, Value};
use std::collections::HashMap;

use crate::gltf_upgrade_util::{
    add_extensions_required, add_extensions_used, add_to_array, for_each_material, is_texture,
    is_vec4, remove_extension, srgb_to_linear, webgl,
};

/// `defaultBaseColorTextureNames` (updateVersion.js L994): uniform names in a
/// glTF 1.0 technique material that indicate a base color *texture*.
pub(crate) const DEFAULT_BASE_COLOR_TEXTURE_NAMES: [&str; 4] =
    ["u_tex", "u_diffuse", "u_emission", "u_diffuse_tex"];

/// `defaultBaseColorFactorNames` (updateVersion.js L1000): uniform names that
/// indicate a base color *factor* (an sRGB vec4).
pub(crate) const DEFAULT_BASE_COLOR_FACTOR_NAMES: [&str; 2] = ["u_diffuse", "u_diffuse_mat"];

/// Blend factors accepted by `KHR_blend` (moveTechniqueRenderStates.js L24).
const SUPPORTED_BLEND_FACTORS: [u64; 10] = [
    webgl::ZERO,
    webgl::ONE,
    webgl::SRC_COLOR,
    webgl::ONE_MINUS_SRC_COLOR,
    webgl::SRC_ALPHA,
    webgl::ONE_MINUS_SRC_ALPHA,
    webgl::DST_ALPHA,
    webgl::ONE_MINUS_DST_ALPHA,
    webgl::DST_COLOR,
    webgl::ONE_MINUS_DST_COLOR,
];

/// `moveTechniqueRenderStates.js`: move glTF 1.0 technique render states to
/// glTF 2.0 material properties (`alphaMode` / `doubleSided`) and the `KHR_blend`
/// extension, then delete `technique.states`.
pub(crate) fn move_technique_render_states(gltf: &mut Value) {
    if gltf.get("techniques").is_none() {
        return;
    }

    let mut blending_for_technique: HashMap<usize, Value> = HashMap::new();
    let mut material_props_for_technique: HashMap<usize, Map<String, Value>> = HashMap::new();

    if let Some(Value::Array(techniques)) = gltf.get_mut("techniques") {
        for (index, technique) in techniques.iter_mut().enumerate() {
            let Some(states) = technique.get("states").cloned() else {
                continue;
            };
            let enable: Vec<u64> = states
                .get("enable")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_u64).collect())
                .unwrap_or_default();

            let mut props = Map::new();
            if enable.contains(&webgl::BLEND) {
                props.insert("alphaMode".to_string(), Value::String("BLEND".to_string()));
                if let Some(functions) = states.get("functions") {
                    let has_equation = functions.get("blendEquationSeparate").is_some();
                    let has_func = functions.get("blendFuncSeparate").is_some();
                    if has_equation || has_func {
                        let blend_equation = functions
                            .get("blendEquationSeparate")
                            .cloned()
                            .unwrap_or_else(|| json!([webgl::FUNC_ADD, webgl::FUNC_ADD]));
                        let blend_factors =
                            supported_blend_factors(functions.get("blendFuncSeparate"));
                        blending_for_technique.insert(
                            index,
                            json!({ "blendEquation": blend_equation, "blendFactors": blend_factors }),
                        );
                    }
                }
            }
            if !enable.contains(&webgl::CULL_FACE) {
                props.insert("doubleSided".to_string(), Value::Bool(true));
            }
            material_props_for_technique.insert(index, props);

            if let Some(obj) = technique.as_object_mut() {
                obj.remove("states");
            }
        }
    }

    if !blending_for_technique.is_empty() {
        if let Some(obj) = gltf.as_object_mut() {
            obj.entry("extensions")
                .or_insert_with(|| Value::Object(Map::new()));
        }
        add_extensions_used(gltf, "KHR_blend");
    }

    for_each_material(gltf, &mut |material| {
        let Some(tech_index) = material.get("technique").and_then(Value::as_u64) else {
            return;
        };
        let tech_index = tech_index as usize;
        if let Some(props) = material_props_for_technique.get(&tech_index) {
            if let Some(obj) = material.as_object_mut() {
                for (key, value) in props {
                    obj.insert(key.clone(), value.clone());
                }
            }
        }
        if let Some(blending) = blending_for_technique.get(&tech_index) {
            if let Some(obj) = material.as_object_mut() {
                let exts = obj
                    .entry("extensions")
                    .or_insert_with(|| Value::Object(Map::new()));
                if let Some(exts) = exts.as_object_mut() {
                    exts.insert("KHR_blend".to_string(), blending.clone());
                }
            }
        }
    });
}

/// `getSupportedBlendFactors` (moveTechniqueRenderStates.js L38): return the
/// value when all four factors are supported, otherwise the default.
fn supported_blend_factors(value: Option<&Value>) -> Value {
    let default = json!([webgl::ONE, webgl::ZERO, webgl::ONE, webgl::ZERO]);
    let Some(arr) = value.and_then(Value::as_array) else {
        return default;
    };
    if arr.len() < 4 {
        return default;
    }
    for item in arr.iter().take(4) {
        match item.as_u64() {
            Some(x) if SUPPORTED_BLEND_FACTORS.contains(&x) => {}
            _ => return default,
        }
    }
    value.cloned().unwrap_or(default)
}

/// `moveTechniquesToExtension.js`: move glTF 1.0 techniques / programs / shaders
/// into the `KHR_techniques_webgl` extension and rewrite each material's
/// `technique` + `values` into `material.extensions.KHR_techniques_webgl`.
pub(crate) fn move_techniques_to_extension(gltf: &mut Value) {
    // techniqueId -> (parameterName -> uniformName)
    let mut mapped_uniforms: HashMap<usize, HashMap<String, String>> = HashMap::new();
    // old technique index -> new index inside the extension
    let mut updated_technique_indices: HashMap<usize, usize> = HashMap::new();
    // old program index -> new index inside the extension
    let mut seen_programs: HashMap<u64, usize> = HashMap::new();

    if gltf.get("techniques").is_some() {
        let gl_extensions = gltf
            .as_object_mut()
            .and_then(|o| o.remove("glExtensionsUsed"));

        // Snapshot the legacy collections so the extension can be built without
        // aliasing the root.
        let techniques = gltf.get("techniques").cloned().unwrap_or(Value::Null);
        let programs = gltf.get("programs").cloned().unwrap_or(Value::Null);
        let shaders = gltf.get("shaders").cloned().unwrap_or(Value::Null);

        let mut ext_programs: Vec<Value> = Vec::new();
        let mut ext_shaders: Vec<Value> = Vec::new();
        let mut ext_techniques: Vec<Value> = Vec::new();

        if let Value::Array(technique_list) = techniques {
            for (technique_id, technique_legacy) in technique_list.iter().enumerate() {
                let parameters = technique_legacy.get("parameters").cloned().unwrap_or(Value::Null);

                let mut new_attributes = Map::new();
                if let Some(attrs) = technique_legacy.get("attributes").and_then(Value::as_object) {
                    for (attribute_name, parameter_name) in attrs {
                        let Some(parameter_name) = parameter_name.as_str() else { continue };
                        let semantic = parameters
                            .get(parameter_name)
                            .and_then(|p| p.get("semantic"))
                            .cloned();
                        let mut entry = Map::new();
                        if let Some(semantic) = semantic {
                            entry.insert("semantic".to_string(), semantic);
                        }
                        new_attributes
                            .insert(attribute_name.clone(), Value::Object(entry));
                    }
                }

                let mut new_uniforms = Map::new();
                let mut param_to_uniform = HashMap::new();
                if let Some(uniforms) = technique_legacy.get("uniforms").and_then(Value::as_object) {
                    for (uniform_name, parameter_name) in uniforms {
                        let Some(parameter_name) = parameter_name.as_str() else { continue };
                        let parameter_legacy = parameters.get(parameter_name).cloned().unwrap_or(Value::Null);
                        let mut entry = Map::new();
                        for field in ["count", "node", "type", "semantic", "value"] {
                            if let Some(v) = parameter_legacy.get(field) {
                                entry.insert(field.to_string(), v.clone());
                            }
                        }
                        new_uniforms.insert(uniform_name.clone(), Value::Object(entry));
                        param_to_uniform
                            .insert(parameter_name.to_string(), uniform_name.clone());
                    }
                }
                mapped_uniforms.insert(technique_id, param_to_uniform);

                let mut technique = Map::new();
                if let Some(name) = technique_legacy.get("name") {
                    technique.insert("name".to_string(), name.clone());
                }
                technique.insert("attributes".to_string(), Value::Object(new_attributes));
                technique.insert("uniforms".to_string(), Value::Object(new_uniforms));

                let legacy_program = technique_legacy.get("program").and_then(Value::as_u64);
                let program_index = match legacy_program {
                    Some(p) if seen_programs.contains_key(&p) => seen_programs[&p],
                    Some(p) => {
                        let program_legacy = index_value(&programs, p);
                        let mut program = Map::new();
                        if let Some(name) = program_legacy.get("name") {
                            program.insert("name".to_string(), name.clone());
                        }
                        if let Some(gl_ext) = &gl_extensions {
                            program.insert("glExtensions".to_string(), gl_ext.clone());
                        }
                        let fs = program_legacy
                            .get("fragmentShader")
                            .and_then(Value::as_u64)
                            .map(|i| index_value(&shaders, i).clone())
                            .unwrap_or(Value::Null);
                        let vs = program_legacy
                            .get("vertexShader")
                            .and_then(Value::as_u64)
                            .map(|i| index_value(&shaders, i).clone())
                            .unwrap_or(Value::Null);
                        let fs_index = add_to_array(&mut ext_shaders, fs, true);
                        let vs_index = add_to_array(&mut ext_shaders, vs, true);
                        program.insert("fragmentShader".to_string(), json!(fs_index));
                        program.insert("vertexShader".to_string(), json!(vs_index));
                        let new_program_index =
                            add_to_array(&mut ext_programs, Value::Object(program), false);
                        seen_programs.insert(p, new_program_index);
                        new_program_index
                    }
                    None => 0,
                };
                technique.insert("program".to_string(), json!(program_index));

                let new_technique_index =
                    add_to_array(&mut ext_techniques, Value::Object(technique), false);
                updated_technique_indices.insert(technique_id, new_technique_index);
            }
        }

        if !ext_techniques.is_empty() {
            let extension = json!({
                "programs": ext_programs,
                "shaders": ext_shaders,
                "techniques": ext_techniques,
            });
            if let Some(obj) = gltf.as_object_mut() {
                let exts = obj
                    .entry("extensions")
                    .or_insert_with(|| Value::Object(Map::new()));
                if let Some(exts) = exts.as_object_mut() {
                    exts.insert("KHR_techniques_webgl".to_string(), extension);
                }
            }
            add_extensions_used(gltf, "KHR_techniques_webgl");
            add_extensions_required(gltf, "KHR_techniques_webgl");
        }
    }

    // Rewrite materials.
    for_each_material(gltf, &mut |material| {
        let tech_index = material.get("technique").and_then(Value::as_u64);
        if let Some(tech_index) = tech_index {
            let mut material_extension = Map::new();
            if let Some(new_index) = updated_technique_indices.get(&(tech_index as usize)) {
                material_extension.insert("technique".to_string(), json!(*new_index));
            }
            let values = material.get("values").cloned().unwrap_or(Value::Null);
            if let Value::Object(values) = values {
                let mut new_values = Map::new();
                let param_map = mapped_uniforms.get(&(tech_index as usize));
                for (parameter_name, value) in values {
                    if let Some(uniform_name) = param_map.and_then(|m| m.get(&parameter_name)) {
                        new_values.insert(uniform_name.clone(), value);
                    }
                }
                if !new_values.is_empty() {
                    material_extension.insert("values".to_string(), Value::Object(new_values));
                }
            }
            if let Some(obj) = material.as_object_mut() {
                let exts = obj
                    .entry("extensions")
                    .or_insert_with(|| Value::Object(Map::new()));
                if let Some(exts) = exts.as_object_mut() {
                    exts.insert(
                        "KHR_techniques_webgl".to_string(),
                        Value::Object(material_extension),
                    );
                }
                obj.remove("technique");
                obj.remove("values");
            }
        }
    });

    if let Some(obj) = gltf.as_object_mut() {
        obj.remove("techniques");
        obj.remove("programs");
        obj.remove("shaders");
    }
}

/// Reads `collection[index]` for an array-form collection (post `objectsToArrays`).
fn index_value(collection: &Value, index: u64) -> &Value {
    collection
        .as_array()
        .and_then(|a| a.get(index as usize))
        .unwrap_or(&Value::Null)
}

/// `convertTechniquesToPbr` (updateVersion.js L1040): build PBR base color from
/// common glTF 1.0 technique uniform names, then drop the legacy extensions.
pub(crate) fn convert_techniques_to_pbr(
    gltf: &mut Value,
    base_color_texture_names: &[String],
    base_color_factor_names: &[String],
) {
    for_each_material(gltf, &mut |material| {
        let values = collect_material_values(material);
        for (name, value) in values {
            if base_color_texture_names.contains(&name) && is_texture(&value) {
                initialize_pbr_material(material);
                set_pbr_field(material, "baseColorTexture", value);
            } else if base_color_factor_names.contains(&name) && is_vec4(&value) {
                let linear: Vec<f64> = value
                    .as_array()
                    .map(|a| a.iter().filter_map(Value::as_f64).collect())
                    .unwrap_or_default();
                let linear = srgb_to_linear(&linear);
                initialize_pbr_material(material);
                set_pbr_field(material, "baseColorFactor", json!(linear));
            }
        }
    });

    remove_extension(gltf, "KHR_techniques_webgl");
    remove_extension(gltf, "KHR_blend");
}

/// `ForEach.materialValue` (ForEach.js L204): material values live either on
/// `material.values` (glTF 1.0) or `material.extensions.KHR_techniques_webgl.values`.
fn collect_material_values(material: &Value) -> Vec<(String, Value)> {
    let values = material
        .pointer("/extensions/KHR_techniques_webgl/values")
        .or_else(|| material.get("values"));
    match values.and_then(Value::as_object) {
        Some(obj) => obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        None => Vec::new(),
    }
}

/// `initializePbrMaterial` (updateVersion.js L1002): ensure `pbrMetallicRoughness`
/// exists with `roughnessFactor = 1.0` and `metallicFactor = 0.0`.
fn initialize_pbr_material(material: &mut Value) {
    let Some(obj) = material.as_object_mut() else { return };
    let pbr = obj
        .entry("pbrMetallicRoughness")
        .or_insert_with(|| Value::Object(Map::new()));
    if let Some(pbr) = pbr.as_object_mut() {
        pbr.insert("roughnessFactor".to_string(), json!(1.0));
        pbr.insert("metallicFactor".to_string(), json!(0.0));
    }
}

fn set_pbr_field(material: &mut Value, field: &str, value: Value) {
    if let Some(pbr) = material
        .as_object_mut()
        .and_then(|o| o.get_mut("pbrMetallicRoughness"))
        .and_then(Value::as_object_mut)
    {
        pbr.insert(field.to_string(), value);
    }
}

/// `convertMaterialsCommonToPbr` (updateVersion.js L1084): convert the
/// `KHR_materials_common` extension to a PBR material (adding
/// `KHR_materials_unlit` for the `CONSTANT` technique), then drop the extension.
pub(crate) fn convert_materials_common_to_pbr(gltf: &mut Value) {
    let mut used_unlit = false;

    for_each_material(gltf, &mut |material| {
        let common = material
            .pointer("/extensions/KHR_materials_common")
            .cloned();
        let Some(common) = common else { return };

        let values = common.get("values").cloned().unwrap_or(Value::Null);
        let ambient = values.get("ambient").cloned();
        let diffuse = values.get("diffuse").cloned();
        let emission = values.get("emission").cloned();
        let transparency = values.get("transparency").and_then(Value::as_f64);
        let double_sided = common.get("doubleSided").and_then(Value::as_bool);
        let transparent = common.get("transparent").and_then(Value::as_bool);
        let technique = common.get("technique").and_then(Value::as_str).map(str::to_string);

        initialize_pbr_material(material);

        if technique.as_deref() == Some("CONSTANT") {
            used_unlit = true;
            if let Some(obj) = material.as_object_mut() {
                let exts = obj
                    .entry("extensions")
                    .or_insert_with(|| Value::Object(Map::new()));
                if let Some(exts) = exts.as_object_mut() {
                    exts.insert(
                        "KHR_materials_unlit".to_string(),
                        Value::Object(Map::new()),
                    );
                }
            }
            assign_as_base_color(material, emission.as_ref());
            assign_as_base_color(material, ambient.as_ref());
        } else {
            assign_as_base_color(material, diffuse.as_ref());
            assign_as_emissive(material, ambient.as_ref());
            assign_as_emissive(material, emission.as_ref());
        }

        if let Some(obj) = material.as_object_mut() {
            if let Some(double_sided) = double_sided {
                obj.insert("doubleSided".to_string(), Value::Bool(double_sided));
            }
            if let Some(transparency) = transparency {
                let existing = obj
                    .get("pbrMetallicRoughness")
                    .and_then(|p| p.get("baseColorFactor"))
                    .and_then(Value::as_array)
                    .cloned();
                let factor = match existing {
                    Some(mut f) if f.len() == 4 => {
                        let alpha = f[3].as_f64().unwrap_or(1.0) * transparency;
                        f[3] = json!(alpha);
                        f
                    }
                    _ => vec![json!(1.0), json!(1.0), json!(1.0), json!(transparency)],
                };
                if let Some(pbr) = obj
                    .get_mut("pbrMetallicRoughness")
                    .and_then(Value::as_object_mut)
                {
                    pbr.insert("baseColorFactor".to_string(), Value::Array(factor));
                }
            }
            if let Some(transparent) = transparent {
                obj.insert(
                    "alphaMode".to_string(),
                    Value::String(if transparent { "BLEND" } else { "OPAQUE" }.to_string()),
                );
            }
        }
    });

    if used_unlit {
        add_extensions_used(gltf, "KHR_materials_unlit");
    }
    remove_extension(gltf, "KHR_materials_common");
}

/// `assignAsBaseColor` (updateVersion.js L1064).
fn assign_as_base_color(material: &mut Value, base_color: Option<&Value>) {
    let Some(base_color) = base_color else { return };
    if is_vec4(base_color) {
        let rgba: Vec<f64> = base_color
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_f64).collect())
            .unwrap_or_default();
        let linear = srgb_to_linear(&rgba);
        set_pbr_field(material, "baseColorFactor", json!(linear));
    } else if is_texture(base_color) {
        set_pbr_field(material, "baseColorTexture", base_color.clone());
    }
}

/// `assignAsEmissive` (updateVersion.js L1074).
fn assign_as_emissive(material: &mut Value, emissive: Option<&Value>) {
    let Some(emissive) = emissive else { return };
    if is_vec4(emissive) {
        let rgb: Vec<f64> = emissive
            .as_array()
            .map(|a| a.iter().take(3).filter_map(Value::as_f64).collect())
            .unwrap_or_default();
        if let Some(obj) = material.as_object_mut() {
            obj.insert("emissiveFactor".to_string(), json!(rgb));
        }
    } else if is_texture(emissive) {
        if let Some(obj) = material.as_object_mut() {
            obj.insert("emissiveTexture".to_string(), emissive.clone());
        }
    }
}
