//! Ported from
//! `packages/engine/Source/Scene/GltfPipeline/moveTechniqueRenderStates.js`.

use std::collections::HashMap;

use cesium_core::webgl_constants::WebGLConstants;
use serde_json::{json, Value};

use crate::gltf_pipeline::add_extensions_used::add_extensions_used;
use crate::gltf_pipeline::defined;
use crate::gltf_pipeline::for_each;
use crate::gltf_pipeline::key_string;

fn default_blend_equation() -> Value {
    json!([WebGLConstants::FUNC_ADD, WebGLConstants::FUNC_ADD])
}

fn default_blend_factors() -> Value {
    json!([
        WebGLConstants::ONE,
        WebGLConstants::ZERO,
        WebGLConstants::ONE,
        WebGLConstants::ZERO
    ])
}

fn is_state_enabled(render_states: &Value, state: u32) -> bool {
    let Some(enabled) = render_states.get("enable") else {
        return false;
    };
    let Some(list) = enabled.as_array() else {
        return false;
    };
    list.iter().any(|item| item.as_u64() == Some(state as u64))
}

const SUPPORTED_BLEND_FACTORS: &[u32] = &[
    WebGLConstants::ZERO,
    WebGLConstants::ONE,
    WebGLConstants::SRC_COLOR,
    WebGLConstants::ONE_MINUS_SRC_COLOR,
    WebGLConstants::SRC_ALPHA,
    WebGLConstants::ONE_MINUS_SRC_ALPHA,
    WebGLConstants::DST_ALPHA,
    WebGLConstants::ONE_MINUS_DST_ALPHA,
    WebGLConstants::DST_COLOR,
    WebGLConstants::ONE_MINUS_DST_COLOR,
];

// If any of the blend factors are not supported, return the default
fn get_supported_blend_factors(value: Option<&Value>, default_value: Value) -> Value {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return default_value;
    };
    let Some(list) = value.as_array() else {
        return default_value;
    };
    for item in list.iter().take(4) {
        let supported = item
            .as_u64()
            .map(|factor| SUPPORTED_BLEND_FACTORS.contains(&(factor as u32)))
            .unwrap_or(false);
        if !supported {
            return default_value;
        }
    }
    value.clone()
}

/// Moves glTF 1.0 technique render states to glTF 2.0 material properties and
/// the `KHR_blend` extension.
pub fn move_technique_render_states(gltf: &mut Value) {
    let mut blending_for_technique: HashMap<String, Value> = HashMap::new();
    let mut material_properties_for_technique: HashMap<String, Value> = HashMap::new();

    let has_techniques_legacy = defined(gltf.get("techniques"));
    if !has_techniques_legacy {
        return;
    }

    for_each::technique(gltf, |technique_legacy, technique_index| {
        let render_states = technique_legacy.get("states").cloned();
        if let Some(render_states) = render_states.filter(|states| !states.is_null()) {
            let mut material_properties = json!({});

            // If BLEND is enabled, the material should have alpha mode BLEND
            if is_state_enabled(&render_states, WebGLConstants::BLEND) {
                material_properties["alphaMode"] = json!("BLEND");

                let blend_functions = render_states.get("functions");
                let has_equation_or_func = blend_functions
                    .map(|functions| {
                        defined(functions.get("blendEquationSeparate"))
                            || defined(functions.get("blendFuncSeparate"))
                    })
                    .unwrap_or(false);
                if has_equation_or_func {
                    let blend_functions = blend_functions.expect("checked above");
                    let blend_equation = blend_functions
                        .get("blendEquationSeparate")
                        .filter(|value| !value.is_null())
                        .cloned()
                        .unwrap_or_else(default_blend_equation);
                    let blend_factors = get_supported_blend_factors(
                        blend_functions.get("blendFuncSeparate"),
                        default_blend_factors(),
                    );
                    blending_for_technique.insert(
                        technique_index.clone(),
                        json!({
                            "blendEquation": blend_equation,
                            "blendFactors": blend_factors
                        }),
                    );
                }
            }

            // If CULL_FACE is not enabled, the material should be doubleSided
            if !is_state_enabled(&render_states, WebGLConstants::CULL_FACE) {
                material_properties["doubleSided"] = json!(true);
            }

            technique_legacy.as_object_mut().map(|technique| technique.remove("states"));
            material_properties_for_technique.insert(technique_index, material_properties);
        }
        None::<()>
    });

    if !blending_for_technique.is_empty() {
        if !defined(gltf.get("extensions")) {
            gltf["extensions"] = json!({});
        }
        add_extensions_used(gltf, "KHR_blend");
    }

    for_each::material(gltf, |material, _id| {
        if let Some(technique_id) = material.get("technique").filter(|v| !v.is_null()) {
            let technique_id = key_string(technique_id);
            if let Some(material_properties) =
                material_properties_for_technique.get_mut(&technique_id)
            {
                for_each::object_legacy(material_properties, |value, property| {
                    material[property] = value.clone();
                    None::<()>
                });
            }

            if let Some(blending) = blending_for_technique.get(&technique_id) {
                if !defined(material.get("extensions")) {
                    material["extensions"] = json!({});
                }
                material["extensions"]["KHR_blend"] = blending.clone();
            }
        }
        None::<()>
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blend_technique_gltf() -> Value {
        json!({
            "techniques": [
                {
                    "states": {
                        "enable": [3042],
                        "functions": {
                            "blendFuncSeparate": [770, 771, 1, 0]
                        }
                    }
                }
            ],
            "materials": [{ "technique": 0 }]
        })
    }

    #[test]
    fn blend_state_becomes_blend_alpha_mode_and_khr_blend() {
        let mut gltf = blend_technique_gltf();
        move_technique_render_states(&mut gltf);

        let material = &gltf["materials"][0];
        assert_eq!(material["alphaMode"], json!("BLEND"));
        assert_eq!(material["doubleSided"], json!(true));
        assert_eq!(
            material["extensions"]["KHR_blend"]["blendFactors"],
            json!([770, 771, 1, 0])
        );
        assert_eq!(
            material["extensions"]["KHR_blend"]["blendEquation"],
            json!([32774, 32774])
        );
        assert!(gltf["techniques"][0].get("states").is_none());
        assert_eq!(gltf["extensionsUsed"], json!(["KHR_blend"]));
    }

    #[test]
    fn unsupported_blend_factors_fall_back_to_defaults() {
        let mut gltf = json!({
            "techniques": [
                {
                    "states": {
                        "enable": [3042],
                        "functions": {
                            "blendFuncSeparate": [770, 9999, 1, 0]
                        }
                    }
                }
            ],
            "materials": [{ "technique": 0 }]
        });
        move_technique_render_states(&mut gltf);
        assert_eq!(
            gltf["materials"][0]["extensions"]["KHR_blend"]["blendFactors"],
            json!([1, 0, 1, 0])
        );
    }

    #[test]
    fn cull_face_enabled_keeps_single_sided() {
        let mut gltf = json!({
            "techniques": [
                { "states": { "enable": [2884] } }
            ],
            "materials": [{ "technique": 0 }]
        });
        move_technique_render_states(&mut gltf);
        let material = &gltf["materials"][0];
        assert!(material.get("doubleSided").is_none());
        assert!(material.get("alphaMode").is_none());
    }

    #[test]
    fn no_techniques_is_a_noop() {
        let mut gltf = json!({ "materials": [{}] });
        move_technique_render_states(&mut gltf);
        assert_eq!(gltf, json!({ "materials": [{}] }));
    }
}
