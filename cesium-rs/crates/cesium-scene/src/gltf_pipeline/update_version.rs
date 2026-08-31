//! Ported from `packages/engine/Source/Scene/GltfPipeline/updateVersion.js`.

use std::collections::HashMap;
use std::collections::HashSet;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartesian4::Cartesian4;
use cesium_core::component_datatype::ComponentDatatype;
use cesium_core::matrix4::Matrix4;
use cesium_core::quaternion::Quaternion;
use cesium_core::runtime_error::RuntimeError;
use cesium_core::webgl_constants::WebGLConstants;
use serde_json::{json, Value};

use crate::gltf_pipeline::add_extensions_used::add_extensions_used;
use crate::gltf_pipeline::add_to_array::add_to_array_value;
use crate::gltf_pipeline::defined;
use crate::gltf_pipeline::find_accessor_min_max::find_accessor_min_max;
use crate::gltf_pipeline::for_each;
use crate::gltf_pipeline::get_accessor_byte_stride::get_accessor_byte_stride;
use crate::gltf_pipeline::key_string;
use crate::gltf_pipeline::move_technique_render_states::move_technique_render_states;
use crate::gltf_pipeline::move_techniques_to_extension::move_techniques_to_extension;
use crate::gltf_pipeline::number_of_components_for_type::number_of_components_for_type;
use crate::gltf_pipeline::remove_extension::remove_extension;
use crate::gltf_pipeline::remove_unused_elements::remove_unused_elements;
use crate::gltf_pipeline::update_accessor_component_types::update_accessor_component_types;
use crate::gltf_pipeline::PipelineBufferSources;

/// Options for [`update_version`].
#[derive(Debug, Clone)]
pub struct UpdateVersionOptions {
    /// The glTF will be upgraded until it hits the specified version.
    pub target_version: Option<String>,
    /// Names of uniforms that indicate base color textures.
    pub base_color_texture_names: Option<Vec<String>>,
    /// Names of uniforms that indicate base color factors.
    pub base_color_factor_names: Option<Vec<String>>,
    /// Keep `KHR_techniques_webgl` / `KHR_materials_common` instead of
    /// converting them to PBR materials.
    pub keep_legacy_extensions: bool,
}

impl Default for UpdateVersionOptions {
    fn default() -> Self {
        Self {
            target_version: None,
            base_color_texture_names: None,
            base_color_factor_names: None,
            keep_legacy_extensions: false,
        }
    }
}

/// Updates the glTF version to the latest version (2.0), or `targetVersion`
/// if specified. Applies changes made to the glTF spec between revisions so
/// that the core library only has to handle the latest version.
///
/// # Errors
/// Propagates [`RuntimeError`] from the accessor data passes when a buffer
/// has no attached binary source.
pub fn update_version(
    gltf: &mut Value,
    sources: &mut PipelineBufferSources,
    options: Option<&UpdateVersionOptions>,
) -> Result<(), RuntimeError> {
    let default_options = UpdateVersionOptions::default();
    let options = options.unwrap_or(&default_options);
    let target_version = options.target_version.clone();

    let version = gltf.get("version").cloned();

    if !defined(gltf.get("asset")) {
        gltf["asset"] = json!({ "version": "1.0" });
    }
    if !defined(gltf["asset"].get("version")) {
        gltf["asset"]["version"] = json!("1.0");
    }
    let version_value = version
        .filter(|value| !value.is_null())
        .or_else(|| gltf["asset"].get("version").cloned())
        .unwrap_or(Value::Null);
    // toString() coercion
    let mut version = key_string(&version_value);

    // Invalid version
    if update_function_name(&version).is_none() {
        // Try truncating trailing version numbers, could be a number as well
        // if it is 0.8
        let mut truncated: String = version.chars().take(3).collect();
        if update_function_name(&truncated).is_none() {
            // Default to 1.0 if it cannot be determined
            truncated = "1.0".to_string();
        }
        version = truncated;
    }

    while let Some(update_function) = update_function_name(&version) {
        if target_version.as_deref() == Some(version.as_str()) {
            break;
        }
        match update_function {
            "glTF08to10" => gltf_08_to_10(gltf, sources)?,
            "glTF10to20" => gltf_10_to_20(gltf, sources)?,
            _ => unreachable!(),
        }
        version = key_string(&gltf["asset"]["version"]);
    }

    if !options.keep_legacy_extensions {
        convert_techniques_to_pbr(gltf, options);
        convert_materials_common_to_pbr(gltf);
    }

    Ok(())
}

fn update_function_name(version: &str) -> Option<&'static str> {
    match version {
        "0.8" => Some("glTF08to10"),
        "1.0" => Some("glTF10to20"),
        "2.0" => None,
        _ => Some(""),
    }
    .filter(|name| !name.is_empty())
}

// ---------------------------------------------------------------------------
// Legacy lookup helpers (glTF 0.8/1.0 top-level objects are id-keyed objects)
// ---------------------------------------------------------------------------

fn legacy_get<'a>(container: Option<&'a Value>, key: &str) -> Option<&'a Value> {
    let container = container.filter(|value| !value.is_null())?;
    if let Some(object) = container.as_object() {
        return object.get(key);
    }
    if let Some(list) = container.as_array() {
        return key.parse::<usize>().ok().and_then(|index| list.get(index));
    }
    None
}

fn to_f64_array(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .map(|list| {
            list.iter()
                .map(|item| item.as_f64().unwrap_or(0.0))
                .collect()
        })
        .unwrap_or_default()
}

fn component_size(component_type: u32) -> usize {
    ComponentDatatype::try_from_u32(component_type)
        .expect("accessor.componentType is not a valid ComponentDatatype")
        .size_in_bytes()
}

fn read_component(data: &[u8], byte_offset: usize, component_type: u32) -> f64 {
    match component_type {
        WebGLConstants::BYTE => data[byte_offset] as i8 as f64,
        WebGLConstants::UNSIGNED_BYTE => data[byte_offset] as f64,
        WebGLConstants::SHORT => {
            i16::from_le_bytes(data[byte_offset..byte_offset + 2].try_into().unwrap()) as f64
        }
        WebGLConstants::UNSIGNED_SHORT => {
            u16::from_le_bytes(data[byte_offset..byte_offset + 2].try_into().unwrap()) as f64
        }
        WebGLConstants::INT => {
            i32::from_le_bytes(data[byte_offset..byte_offset + 4].try_into().unwrap()) as f64
        }
        WebGLConstants::UNSIGNED_INT => {
            u32::from_le_bytes(data[byte_offset..byte_offset + 4].try_into().unwrap()) as f64
        }
        WebGLConstants::FLOAT => {
            f32::from_le_bytes(data[byte_offset..byte_offset + 4].try_into().unwrap()) as f64
        }
        WebGLConstants::DOUBLE => {
            f64::from_le_bytes(data[byte_offset..byte_offset + 8].try_into().unwrap())
        }
        _ => 0.0,
    }
}

/// Mirrors JavaScript typed-array element assignment (truncation and modular
/// wrap for integer types, f32 rounding for FLOAT).
fn write_component(data: &mut [u8], byte_offset: usize, component_type: u32, value: f64) {
    let truncated = value.trunc();
    match component_type {
        WebGLConstants::BYTE | WebGLConstants::UNSIGNED_BYTE => {
            data[byte_offset] = truncated.rem_euclid(256.0) as u8;
        }
        WebGLConstants::SHORT | WebGLConstants::UNSIGNED_SHORT => {
            let bytes = (truncated.rem_euclid(65536.0) as u16).to_le_bytes();
            data[byte_offset..byte_offset + 2].copy_from_slice(&bytes);
        }
        WebGLConstants::INT | WebGLConstants::UNSIGNED_INT => {
            let bytes = (truncated.rem_euclid(4_294_967_296.0) as u32).to_le_bytes();
            data[byte_offset..byte_offset + 4].copy_from_slice(&bytes);
        }
        WebGLConstants::FLOAT => {
            let bytes = (value as f32).to_le_bytes();
            data[byte_offset..byte_offset + 4].copy_from_slice(&bytes);
        }
        WebGLConstants::DOUBLE => {
            data[byte_offset..byte_offset + 8].copy_from_slice(&value.to_le_bytes());
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// glTF 0.8 -> 1.0
// ---------------------------------------------------------------------------

fn gltf_08_to_10(gltf: &mut Value, sources: &mut PipelineBufferSources) -> Result<(), RuntimeError> {
    if !defined(gltf.get("asset")) {
        gltf["asset"] = json!({});
    }
    gltf["asset"]["version"] = json!("1.0");
    // Profile should be an object, not a string
    let profile = gltf["asset"].get("profile").cloned();
    if let Some(Value::String(profile_string)) = profile {
        let split: Vec<&str> = profile_string.split(' ').collect();
        gltf["asset"]["profile"] = json!({
            "api": split.first().copied(),
            "version": split.get(1).copied()
        });
    } else {
        gltf["asset"]["profile"] = json!({});
    }

    // Version property should be in asset, not on the root element
    if let Some(root) = gltf.as_object_mut() {
        root.remove("version");
    }
    // material.instanceTechnique properties should be directly on the material
    update_instance_techniques(gltf);
    // primitive.primitive should be primitive.mode
    set_primitive_modes(gltf);
    // Node rotation should be quaternion, not axis-angle
    // node.instanceSkin is deprecated
    update_nodes(gltf);
    // Animations that target rotations should be quaternion, not axis-angle
    update_animations(gltf, sources);
    // technique.pass and techniques.passes are deprecated
    remove_technique_passes(gltf);
    // gltf.allExtensions -> extensionsUsed
    if defined(gltf.get("allExtensions")) {
        let all_extensions = gltf["allExtensions"].clone();
        gltf["extensionsUsed"] = all_extensions;
        if let Some(root) = gltf.as_object_mut() {
            root.remove("allExtensions");
        }
    }
    // gltf.lights -> khrMaterialsCommon.lights
    if defined(gltf.get("lights")) {
        let lights = gltf["lights"].clone();
        if !defined(gltf.get("extensions")) {
            gltf["extensions"] = json!({});
        }
        if !defined(
            gltf["extensions"]
                .get("KHR_materials_common"),
        ) {
            gltf["extensions"]["KHR_materials_common"] = json!({});
        }
        gltf["extensions"]["KHR_materials_common"]["lights"] = lights;
        if let Some(root) = gltf.as_object_mut() {
            root.remove("lights");
        }
        add_extensions_used(gltf, "KHR_materials_common");
    }
    Ok(())
}

fn update_instance_techniques(gltf: &mut Value) {
    for_each::material(gltf, |material, _id| {
        let instance_technique = material.get("instanceTechnique").cloned();
        if let Some(instance_technique) = instance_technique.filter(|value| !value.is_null()) {
            if let Some(technique) = instance_technique.get("technique") {
                material["technique"] = technique.clone();
            }
            if let Some(values) = instance_technique.get("values") {
                material["values"] = values.clone();
            }
            if let Some(material_object) = material.as_object_mut() {
                material_object.remove("instanceTechnique");
            }
        }
        None::<()>
    });
}

fn set_primitive_modes(gltf: &mut Value) {
    for_each::mesh(gltf, |mesh, _id| {
        for_each::mesh_primitive(mesh, |primitive, _index| {
            let default_mode = primitive
                .get("primitive")
                .filter(|value| !value.is_null())
                .cloned()
                .unwrap_or(json!(WebGLConstants::TRIANGLES));
            if primitive.get("mode").map_or(true, |mode| mode.is_null()) {
                primitive["mode"] = default_mode;
            }
            if let Some(primitive_object) = primitive.as_object_mut() {
                primitive_object.remove("primitive");
            }
            None::<()>
        });
        None::<()>
    });
}

fn legacy_node_keys(gltf: &Value) -> Vec<String> {
    match gltf.get("nodes") {
        Some(Value::Object(nodes)) => nodes.keys().cloned().collect(),
        Some(Value::Array(nodes)) => (0..nodes.len()).map(|index| index.to_string()).collect(),
        _ => Vec::new(),
    }
}

fn update_nodes(gltf: &mut Value) {
    let node_keys = legacy_node_keys(gltf);
    for node_key in node_keys {
        let rotation = gltf
            .get("nodes")
            .and_then(|nodes| legacy_get(Some(nodes), &node_key))
            .and_then(|node| node.get("rotation"))
            .filter(|value| !value.is_null())
            .cloned();
        if let Some(rotation) = rotation {
            let values = to_f64_array(&rotation);
            if values.len() >= 4 {
                let mut axis = Cartesian3::ZERO;
                Cartesian3::from_array(&values, Some(0), &mut axis);
                let mut quat = Quaternion::IDENTITY;
                Quaternion::from_axis_angle(&axis, values[3], &mut quat);
                if let Some(node) = gltf
                    .get_mut("nodes")
                    .and_then(|nodes| legacy_get_mut(nodes, &node_key))
                {
                    node["rotation"] = json!([quat.x, quat.y, quat.z, quat.w]);
                }
            }
        }

        let instance_skin = gltf
            .get("nodes")
            .and_then(|nodes| legacy_get(Some(nodes), &node_key))
            .and_then(|node| node.get("instanceSkin"))
            .filter(|value| !value.is_null())
            .cloned();
        if let Some(instance_skin) = instance_skin {
            if let Some(node) = gltf
                .get_mut("nodes")
                .and_then(|nodes| legacy_get_mut(nodes, &node_key))
            {
                if let Some(skeletons) = instance_skin.get("skeletons") {
                    node["skeletons"] = skeletons.clone();
                }
                if let Some(skin) = instance_skin.get("skin") {
                    node["skin"] = skin.clone();
                }
                if let Some(meshes) = instance_skin.get("meshes") {
                    node["meshes"] = meshes.clone();
                }
                if let Some(node_object) = node.as_object_mut() {
                    node_object.remove("instanceSkin");
                }
            }
        }
    }
}

fn legacy_get_mut<'a>(container: &'a mut Value, key: &str) -> Option<&'a mut Value> {
    match container {
        Value::Object(map) => map.get_mut(key),
        Value::Array(list) => key
            .parse::<usize>()
            .ok()
            .and_then(|index| list.get_mut(index)),
        _ => None,
    }
}

fn update_animations(gltf: &Value, sources: &mut PipelineBufferSources) {
    let mut updated_accessors: HashSet<String> = HashSet::new();
    let mut rotation_accessor_ids: Vec<String> = Vec::new();

    let Some(animations) = gltf.get("animations") else {
        return;
    };
    let animation_keys: Vec<String> = match animations {
        Value::Object(map) => map.keys().cloned().collect(),
        Value::Array(list) => (0..list.len()).map(|index| index.to_string()).collect(),
        _ => return,
    };
    for animation_key in animation_keys {
        let Some(animation) = legacy_get(Some(animations), &animation_key) else {
            continue;
        };
        let (Some(channels), Some(parameters), Some(samplers)) = (
            animation.get("channels"),
            animation.get("parameters"),
            animation.get("samplers"),
        ) else {
            continue;
        };
        let channel_values: Vec<&Value> = match channels {
            Value::Array(list) => list.iter().collect(),
            Value::Object(map) => map.values().collect(),
            _ => continue,
        };
        for channel in channel_values {
            if channel.pointer("/target/path").and_then(|path| path.as_str()) != Some("rotation") {
                continue;
            }
            let Some(sampler_key) = channel.get("sampler").map(|value| key_string(value)) else {
                continue;
            };
            let Some(accessor_id_value) = legacy_get(Some(samplers), &sampler_key)
                .and_then(|sampler| sampler.get("output"))
                .and_then(|output| legacy_get(Some(parameters), &key_string(output)))
            else {
                continue;
            };
            let accessor_id = key_string(accessor_id_value);
            if updated_accessors.insert(accessor_id.clone()) {
                rotation_accessor_ids.push(accessor_id);
            }
        }
    }

    for accessor_id in rotation_accessor_ids {
        let Some(accessor) = legacy_get(gltf.get("accessors"), &accessor_id).cloned() else {
            continue;
        };
        let Some(buffer_view_key) = accessor.get("bufferView").map(|value| key_string(value))
        else {
            continue;
        };
        let Some(buffer_view) = legacy_get(gltf.get("bufferViews"), &buffer_view_key).cloned()
        else {
            continue;
        };
        let Some(buffer_key) = buffer_view.get("buffer").map(|value| key_string(value)) else {
            continue;
        };
        // The buffer index into the sources side table: buffers are still
        // id-keyed objects at this stage; the side table is aligned with
        // numeric buffer ids (parse_glb populates index-keyed buffers).
        let buffer_index = buffer_key.parse::<usize>();

        let component_type = accessor
            .get("componentType")
            .and_then(|value| value.as_u64())
            .unwrap_or(0) as u32;
        let count = accessor
            .get("count")
            .and_then(|value| value.as_u64())
            .unwrap_or(0) as usize;
        let accessor_type = accessor
            .get("type")
            .and_then(|value| value.as_str())
            .unwrap_or("VEC4");
        let components_length = number_of_components_for_type(accessor_type).unwrap_or(4);

        let Ok(buffer_index) = buffer_index else {
            continue;
        };
        let Some(source) = sources.get_mut(buffer_index).and_then(|source| source.as_mut()) else {
            continue;
        };

        let byte_offset = source.byte_offset
            + buffer_view
                .get("byteOffset")
                .and_then(|value| value.as_u64())
                .unwrap_or(0) as usize
            + accessor
                .get("byteOffset")
                .and_then(|value| value.as_u64())
                .unwrap_or(0) as usize;
        let size = component_size(component_type);

        for j in 0..count {
            let offset = byte_offset + j * components_length * size;
            if offset + 4 * size > source.buffer.len() {
                break;
            }
            let x = read_component(&source.buffer, offset, component_type);
            let y = read_component(&source.buffer, offset + size, component_type);
            let z = read_component(&source.buffer, offset + 2 * size, component_type);
            let angle = read_component(&source.buffer, offset + 3 * size, component_type);
            let axis = Cartesian3::new(x, y, z);
            let mut quat = Quaternion::IDENTITY;
            Quaternion::from_axis_angle(&axis, angle, &mut quat);
            write_component(&mut source.buffer, offset, component_type, quat.x);
            write_component(&mut source.buffer, offset + size, component_type, quat.y);
            write_component(&mut source.buffer, offset + 2 * size, component_type, quat.z);
            write_component(
                &mut source.buffer,
                offset + 3 * size,
                component_type,
                quat.w,
            );
        }
    }
}

fn remove_technique_passes(gltf: &mut Value) {
    for_each::technique(gltf, |technique, _id| {
        let passes = technique.get("passes").cloned();
        if let Some(passes) = passes.filter(|passes| !passes.is_null()) {
            let pass_name = technique
                .get("pass")
                .filter(|value| !value.is_null())
                .map(|value| key_string(value))
                .unwrap_or_else(|| "defaultPass".to_string());
            if let Some(pass) = legacy_get(Some(&passes), &pass_name) {
                let instance_program = pass.get("instanceProgram");
                if technique.get("attributes").map_or(true, |v| v.is_null()) {
                    if let Some(attributes) = instance_program.and_then(|p| p.get("attributes")) {
                        technique["attributes"] = attributes.clone();
                    }
                }
                if technique.get("program").map_or(true, |v| v.is_null()) {
                    if let Some(program) = instance_program.and_then(|p| p.get("program")) {
                        technique["program"] = program.clone();
                    }
                }
                if technique.get("uniforms").map_or(true, |v| v.is_null()) {
                    if let Some(uniforms) = instance_program.and_then(|p| p.get("uniforms")) {
                        technique["uniforms"] = uniforms.clone();
                    }
                }
                if technique.get("states").map_or(true, |v| v.is_null()) {
                    if let Some(states) = pass.get("states") {
                        technique["states"] = states.clone();
                    }
                }
            }
            if let Some(technique_object) = technique.as_object_mut() {
                technique_object.remove("passes");
                technique_object.remove("pass");
            }
        }
        None::<()>
    });
}

// ---------------------------------------------------------------------------
// glTF 1.0 -> 2.0
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
fn gltf_10_to_20(
    gltf: &mut Value,
    sources: &mut PipelineBufferSources,
) -> Result<(), RuntimeError> {
    if !defined(gltf.get("asset")) {
        gltf["asset"] = json!({});
    }
    gltf["asset"]["version"] = json!("2.0");
    // material.instanceTechnique properties should be directly on the material.
    // instanceTechnique is a gltf 0.8 property but is seen in some 1.0 models.
    update_instance_techniques(gltf);
    // animation.samplers now refers directly to accessors and
    // animation.parameters should be removed
    remove_animation_samplers_indirection(gltf);
    // Remove empty nodes and re-assign referencing indices
    remove_empty_nodes(gltf);
    // Top-level objects are now arrays referenced by index instead of id
    objects_to_arrays(gltf);
    // Animation.sampler objects cannot have names
    remove_animation_sampler_names(gltf);
    // asset.profile no longer exists
    strip_asset(gltf);
    // Move known extensions from extensionsUsed to extensionsRequired
    require_known_extensions(gltf);
    // bufferView.byteLength and buffer.byteLength are required
    require_byte_length(gltf, sources);
    // byteStride moved from accessor to bufferView
    move_byte_stride_to_buffer_view(gltf, sources);
    // accessor.min and accessor.max must be defined for accessors containing
    // POSITION attributes
    require_position_accessor_min_max(gltf, sources)?;
    // An animation sampler's input accessor must have min and max defined
    require_animation_accessor_min_max(gltf, sources)?;
    // When an accessor has a min- or max, then it is recomputed, to capture
    // the actual value, and not use the (possibly imprecise) value from the
    // input
    validate_present_accessor_min_max(gltf, sources)?;
    // buffer.type is unnecessary and should be removed
    remove_buffer_type(gltf);
    // Remove format, internalFormat, target, and type
    remove_texture_properties(gltf);
    // TEXCOORD and COLOR attributes must be written with a set index
    require_attribute_set_index(gltf);
    // Add underscores to application-specific parameters
    underscore_application_specific_semantics(gltf);
    // Accessors referenced by JOINTS_0 and WEIGHTS_0 attributes must have
    // correct component types
    update_accessor_component_types(gltf, sources)?;
    // Clamp camera parameters
    clamp_camera_parameters(gltf);
    // Move legacy technique render states to material properties and add
    // KHR_blend extension blending functions
    move_technique_render_states(gltf);
    // Add material techniques to KHR_techniques_webgl extension, removing
    // shaders, programs, and techniques
    move_techniques_to_extension(gltf);
    // Remove empty arrays
    remove_empty_arrays(gltf);
    Ok(())
}

fn remove_animation_samplers_indirection(gltf: &mut Value) {
    let animation_keys = legacy_keys(gltf.get("animations"));
    for animation_key in animation_keys {
        let has_parameters = gltf
            .get("animations")
            .and_then(|animations| legacy_get(Some(animations), &animation_key))
            .and_then(|animation| animation.get("parameters"))
            .is_some_and(|parameters| !parameters.is_null());
        if !has_parameters {
            continue;
        }
        let Some(animation) = gltf
            .get_mut("animations")
            .and_then(|animations| legacy_get_mut(animations, &animation_key))
        else {
            continue;
        };
        let parameters = animation.get("parameters").cloned();
        let sampler_keys = legacy_keys(animation.get("samplers"));
        for sampler_key in sampler_keys {
            let (input, output) = {
                let sampler = legacy_get(animation.get("samplers"), &sampler_key);
                let input = sampler
                    .and_then(|sampler| sampler.get("input"))
                    .and_then(|input| legacy_get(parameters.as_ref(), &key_string(input)))
                    .cloned();
                let output = sampler
                    .and_then(|sampler| sampler.get("output"))
                    .and_then(|output| legacy_get(parameters.as_ref(), &key_string(output)))
                    .cloned();
                (input, output)
            };
            let Some(samplers) = animation.get_mut("samplers") else {
                continue;
            };
            if let Some(sampler) = legacy_get_mut(samplers, &sampler_key) {
                sampler["input"] = input.unwrap_or(Value::Null);
                sampler["output"] = output.unwrap_or(Value::Null);
            }
        }
        if let Some(animation_object) = animation.as_object_mut() {
            animation_object.remove("parameters");
        }
    }
}

/// Keys of a legacy id-keyed object (or indices of an array, stringified).
fn legacy_keys(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Object(map)) => map.keys().cloned().collect(),
        Some(Value::Array(list)) => (0..list.len()).map(|index| index.to_string()).collect(),
        _ => Vec::new(),
    }
}

fn is_node_empty(node: &Value) -> bool {
    let list_empty = |key: &str| match node.get(key).filter(|value| !value.is_null()) {
        None => true,
        Some(Value::Array(list)) => list.is_empty(),
        Some(_) => false,
    };
    let undefined = |key: &str| node.get(key).map_or(true, |value| value.is_null());
    list_empty("children")
        && list_empty("meshes")
        && undefined("camera")
        && undefined("skin")
        && undefined("skeletons")
        && undefined("jointName")
        && (undefined("translation") || {
            let mut translation = Cartesian3::ZERO;
            Cartesian3::from_array(&to_f64_array(&node["translation"]), Some(0), &mut translation);
            Cartesian3::equals(Some(&translation), Some(&Cartesian3::ZERO))
        })
        && (undefined("scale") || {
            let mut scale = Cartesian3::ZERO;
            Cartesian3::from_array(&to_f64_array(&node["scale"]), Some(0), &mut scale);
            Cartesian3::equals(Some(&scale), Some(&Cartesian3::new(1.0, 1.0, 1.0)))
        })
        && (undefined("rotation") || {
            let rotation = Cartesian4::from_array_new(&to_f64_array(&node["rotation"]), Some(0));
            Cartesian4::equals(Some(&rotation), Some(&Cartesian4::UNIT_W))
        })
        && (undefined("matrix") || {
            let matrix = Matrix4::from_column_major_array_new(&to_f64_array(&node["matrix"]));
            Matrix4::equals(&matrix, &Matrix4::IDENTITY)
        })
        && undefined("extensions")
        && undefined("extras")
}

fn delete_node(gltf: &mut Value, node_id: &str) {
    // Remove from list of nodes in scene
    let scene_keys = legacy_keys(gltf.get("scenes"));
    for scene_key in scene_keys {
        let remove_index = gltf
            .get("scenes")
            .and_then(|scenes| legacy_get(Some(scenes), &scene_key))
            .and_then(|scene| scene.get("nodes"))
            .and_then(|nodes| nodes.as_array())
            .and_then(|nodes| nodes.iter().position(|node| key_string(node) == node_id));
        if let Some(index) = remove_index {
            if let Some(scene) = gltf
                .get_mut("scenes")
                .and_then(|scenes| legacy_get_mut(scenes, &scene_key))
            {
                if let Some(nodes) = scene.get_mut("nodes").and_then(|nodes| nodes.as_array_mut())
                {
                    if index < nodes.len() {
                        nodes.remove(index);
                    }
                }
            }
        }
    }

    // Remove parent node's reference to this node, and delete the parent if
    // also empty
    let node_keys = legacy_keys(gltf.get("nodes"));
    for parent_node_id in node_keys {
        let child_index = gltf
            .get("nodes")
            .and_then(|nodes| legacy_get(Some(nodes), &parent_node_id))
            .and_then(|parent| parent.get("children"))
            .and_then(|children| children.as_array())
            .and_then(|children| {
                children
                    .iter()
                    .position(|child| key_string(child) == node_id)
            });
        if let Some(index) = child_index {
            let parent_empty = if let Some(parent) = gltf
                .get_mut("nodes")
                .and_then(|nodes| legacy_get_mut(nodes, &parent_node_id))
            {
                if let Some(children) = parent
                    .get_mut("children")
                    .and_then(|children| children.as_array_mut())
                {
                    if index < children.len() {
                        children.remove(index);
                    }
                }
                is_node_empty(parent)
            } else {
                false
            };
            if parent_empty {
                delete_node(gltf, &parent_node_id);
            }
        }
    }

    if let Some(Value::Object(nodes)) = gltf.get_mut("nodes") {
        nodes.remove(node_id);
    }
}

fn remove_empty_nodes(gltf: &mut Value) {
    // DEVIATION: ids of empty nodes are collected up front (the JS
    // interleaves the emptiness check and deletion). A node that only becomes
    // empty through a deletion is still removed because `delete_node`
    // recurses into emptied parents.
    let empty_node_ids: Vec<String> = legacy_keys(gltf.get("nodes"))
        .into_iter()
        .filter(|node_id| {
            gltf.get("nodes")
                .and_then(|nodes| legacy_get(Some(nodes), node_id))
                .map_or(false, is_node_empty)
        })
        .collect();
    for node_id in empty_node_ids {
        delete_node(gltf, &node_id);
    }
}

fn object_to_array(object: Value, mapping: &mut HashMap<String, usize>) -> Value {
    let mut array: Vec<Value> = Vec::new();
    let entries: Vec<(String, Value)> = match object {
        Value::Object(map) => map.into_iter().collect(),
        Value::Array(list) => list
            .into_iter()
            .enumerate()
            .map(|(index, value)| (index.to_string(), value))
            .collect(),
        _ => Vec::new(),
    };
    for (id, value) in entries {
        mapping.insert(id.clone(), array.len());
        array.push(value);
        let value = array.last_mut().expect("pushed above");
        if value.is_object() && !defined(value.get("name")) {
            value["name"] = json!(id);
        }
    }
    Value::Array(array)
}

/// Resolves a legacy id reference through an id->index mapping. Unresolved
/// references become `null` (the JS produces `undefined`).
fn mapped(mapping: &HashMap<String, usize>, reference: &Value) -> Value {
    mapping
        .get(&key_string(reference))
        .map(|index| json!(*index))
        .unwrap_or(Value::Null)
}

const TOP_LEVEL_ARRAY_PROPERTIES: [&str; 16] = [
    "accessors",
    "animations",
    "buffers",
    "bufferViews",
    "cameras",
    "images",
    "materials",
    "meshes",
    "nodes",
    "programs",
    "samplers",
    "scenes",
    "shaders",
    "skins",
    "textures",
    "techniques",
];

#[allow(clippy::too_many_lines)]
fn objects_to_arrays(gltf: &mut Value) {
    let empty_mapping: HashMap<String, usize> = HashMap::new();
    let mut global_mapping: HashMap<&'static str, HashMap<String, usize>> = HashMap::new();

    // Map joint names to id names
    let mut joint_name_to_id: HashMap<String, String> = HashMap::new();
    if let Some(Value::Object(nodes)) = gltf.get("nodes") {
        for (id, node) in nodes {
            if let Some(joint_name) = node.get("jointName").and_then(|value| value.as_str()) {
                joint_name_to_id.insert(joint_name.to_string(), id.clone());
            }
        }
    }

    // Convert top level objects to arrays
    for top_level_id in TOP_LEVEL_ARRAY_PROPERTIES {
        if defined(gltf.get(top_level_id)) {
            let object = gltf[top_level_id].take();
            let mut object_mapping = HashMap::new();
            gltf[top_level_id] = object_to_array(object, &mut object_mapping);
            global_mapping.insert(top_level_id, object_mapping);
        }
    }

    // Remap joint names to array indexes
    let mut joint_name_to_index: HashMap<String, usize> = HashMap::new();
    if let Some(nodes_mapping) = global_mapping.get("nodes") {
        for (joint_name, old_id) in joint_name_to_id {
            if let Some(index) = nodes_mapping.get(&old_id) {
                joint_name_to_index.insert(joint_name, *index);
            }
        }
    }

    // Fix references
    if defined(gltf.get("scene")) {
        let reference = gltf["scene"].clone();
        gltf["scene"] = mapped(global_mapping.get("scenes").unwrap_or(&empty_mapping), &reference);
    }
    for_each::buffer_view(gltf, |buffer_view, _id| {
        if defined(buffer_view.get("buffer")) {
            let reference = buffer_view["buffer"].clone();
            buffer_view["buffer"] =
                mapped(global_mapping.get("buffers").unwrap_or(&empty_mapping), &reference);
        }
        None::<()>
    });
    for_each::accessor(gltf, |accessor, _id| {
        if defined(accessor.get("bufferView")) {
            let reference = accessor["bufferView"].clone();
            accessor["bufferView"] = mapped(
                global_mapping.get("bufferViews").unwrap_or(&empty_mapping),
                &reference,
            );
        }
        None::<()>
    });
    for_each::shader(gltf, |shader, _id| {
        let extensions = shader.get("extensions").cloned();
        if let Some(mut extensions) = extensions.filter(|value| !value.is_null()) {
            let binary_gltf = extensions.get("KHR_binary_glTF").cloned();
            if let Some(binary_gltf) = binary_gltf.filter(|value| !value.is_null()) {
                if let Some(buffer_view) = binary_gltf.get("bufferView") {
                    shader["bufferView"] = mapped(
                        global_mapping.get("bufferViews").unwrap_or(&empty_mapping),
                        buffer_view,
                    );
                }
                if let Some(mime_type) = binary_gltf.get("mimeType") {
                    shader["mimeType"] = mime_type.clone();
                }
                if let Some(extensions_object) = extensions.as_object_mut() {
                    extensions_object.remove("KHR_binary_glTF");
                }
            }
            if extensions.as_object().map_or(false, |map| map.is_empty()) {
                if let Some(shader_object) = shader.as_object_mut() {
                    shader_object.remove("extensions");
                }
            } else {
                shader["extensions"] = extensions;
            }
        }
        None::<()>
    });
    for_each::program(gltf, |program, _id| {
        if defined(program.get("vertexShader")) {
            let reference = program["vertexShader"].clone();
            program["vertexShader"] =
                mapped(global_mapping.get("shaders").unwrap_or(&empty_mapping), &reference);
        }
        if defined(program.get("fragmentShader")) {
            let reference = program["fragmentShader"].clone();
            program["fragmentShader"] =
                mapped(global_mapping.get("shaders").unwrap_or(&empty_mapping), &reference);
        }
        None::<()>
    });
    for_each::technique(gltf, |technique, _id| {
        if defined(technique.get("program")) {
            let reference = technique["program"].clone();
            technique["program"] =
                mapped(global_mapping.get("programs").unwrap_or(&empty_mapping), &reference);
        }
        for_each::technique_parameter(technique, |parameter, _name| {
            if defined(parameter.get("node")) {
                let reference = parameter["node"].clone();
                parameter["node"] =
                    mapped(global_mapping.get("nodes").unwrap_or(&empty_mapping), &reference);
            }
            if parameter.get("value").map_or(false, |value| value.is_string()) {
                let reference = parameter["value"].clone();
                parameter["value"] = json!({
                    "index": mapped(global_mapping.get("textures").unwrap_or(&empty_mapping), &reference)
                });
            }
            None::<()>
        });
        None::<()>
    });
    for_each::mesh(gltf, |mesh, _id| {
        for_each::mesh_primitive(mesh, |primitive, _index| {
            if defined(primitive.get("indices")) {
                let reference = primitive["indices"].clone();
                primitive["indices"] = mapped(
                    global_mapping.get("accessors").unwrap_or(&empty_mapping),
                    &reference,
                );
            }
            for_each::mesh_primitive_attribute(primitive, |accessor_id, _semantic| {
                let reference = accessor_id.clone();
                *accessor_id = mapped(
                    global_mapping.get("accessors").unwrap_or(&empty_mapping),
                    &reference,
                );
                None::<()>
            });
            if defined(primitive.get("material")) {
                let reference = primitive["material"].clone();
                primitive["material"] = mapped(
                    global_mapping.get("materials").unwrap_or(&empty_mapping),
                    &reference,
                );
            }
            None::<()>
        });
        None::<()>
    });

    // Nodes may spawn extra mesh nodes (JS addToArray during ForEach.node);
    // collect those requests and apply them after iteration.
    struct MeshSplit {
        node_index: usize,
        mesh_indices: Vec<Value>,
    }
    let mut mesh_splits: Vec<MeshSplit> = Vec::new();
    let mut skeleton_assignments: Vec<(usize, Value)> = Vec::new();
    for_each::node(gltf, |node, node_id| {
        let node_index = node_id.parse::<usize>().unwrap_or(0);
        let children_length = node
            .get("children")
            .and_then(|children| children.as_array())
            .map(|children| children.len())
            .unwrap_or(0);
        for index in 0..children_length {
            let reference = node["children"][index].clone();
            node["children"][index] =
                mapped(global_mapping.get("nodes").unwrap_or(&empty_mapping), &reference);
        }
        let children_defined = defined(node.get("children"));
        let meshes_value = node.get("meshes").cloned();
        if let Some(meshes_value) = meshes_value.filter(|value| !value.is_null()) {
            let meshes: Vec<Value> = meshes_value.as_array().cloned().unwrap_or_default();
            if !meshes.is_empty() {
                node["mesh"] =
                    mapped(global_mapping.get("meshes").unwrap_or(&empty_mapping), &meshes[0]);
                let extras: Vec<Value> = meshes[1..]
                    .iter()
                    .map(|reference| {
                        mapped(global_mapping.get("meshes").unwrap_or(&empty_mapping), reference)
                    })
                    .collect();
                if !extras.is_empty() {
                    mesh_splits.push(MeshSplit {
                        node_index,
                        mesh_indices: extras,
                    });
                }
                if !children_defined {
                    node["children"] = json!([]);
                }
            }
            if let Some(node_object) = node.as_object_mut() {
                node_object.remove("meshes");
            }
        }
        if defined(node.get("camera")) {
            let reference = node["camera"].clone();
            node["camera"] =
                mapped(global_mapping.get("cameras").unwrap_or(&empty_mapping), &reference);
        }
        if defined(node.get("skin")) {
            let reference = node["skin"].clone();
            node["skin"] =
                mapped(global_mapping.get("skins").unwrap_or(&empty_mapping), &reference);
        }
        let skeletons_value = node.get("skeletons").cloned();
        if let Some(skeletons_value) = skeletons_value.filter(|value| !value.is_null()) {
            let skeletons: Vec<Value> = skeletons_value.as_array().cloned().unwrap_or_default();
            if !skeletons.is_empty() && defined(node.get("skin")) {
                if let Some(skin_index) = node.get("skin").and_then(|value| value.as_u64()) {
                    // Assign skeletons to skins
                    let skeleton = mapped(
                        global_mapping.get("nodes").unwrap_or(&empty_mapping),
                        &skeletons[0],
                    );
                    skeleton_assignments.push((skin_index as usize, skeleton));
                }
            }
            if let Some(node_object) = node.as_object_mut() {
                node_object.remove("skeletons");
            }
        }
        if let Some(node_object) = node.as_object_mut() {
            node_object.remove("jointName");
        }
        None::<()>
    });
    for split in mesh_splits {
        for mesh_index in split.mesh_indices {
            let mesh_node = json!({ "mesh": mesh_index });
            let Some(nodes_value) = gltf.get_mut("nodes") else {
                continue;
            };
            let mesh_node_id = add_to_array_value(nodes_value, mesh_node, false);
            if let Some(children) = gltf["nodes"][split.node_index]
                .get_mut("children")
                .and_then(|children| children.as_array_mut())
            {
                children.push(json!(mesh_node_id));
            }
        }
    }
    for (skin_index, skeleton) in skeleton_assignments {
        if let Some(skin) = gltf
            .get_mut("skins")
            .and_then(|skins| skins.as_array_mut())
            .and_then(|skins| skins.get_mut(skin_index))
        {
            skin["skeleton"] = skeleton;
        }
    }

    for_each::skin(gltf, |skin, _id| {
        if defined(skin.get("inverseBindMatrices")) {
            let reference = skin["inverseBindMatrices"].clone();
            skin["inverseBindMatrices"] = mapped(
                global_mapping.get("accessors").unwrap_or(&empty_mapping),
                &reference,
            );
        }
        let joint_names = skin.get("jointNames").cloned();
        if let Some(joint_names) = joint_names.filter(|value| !value.is_null()) {
            let names: Vec<Value> = joint_names.as_array().cloned().unwrap_or_default();
            let joints: Vec<Value> = names
                .iter()
                .map(|name| {
                    joint_name_to_index
                        .get(&key_string(name))
                        .map(|index| json!(*index))
                        .unwrap_or(Value::Null)
                })
                .collect();
            skin["joints"] = Value::Array(joints);
            if let Some(skin_object) = skin.as_object_mut() {
                skin_object.remove("jointNames");
            }
        }
        None::<()>
    });
    for_each::scene(gltf, |scene, _id| {
        let scene_nodes_length = scene
            .get("nodes")
            .and_then(|nodes| nodes.as_array())
            .map(|nodes| nodes.len())
            .unwrap_or(0);
        for index in 0..scene_nodes_length {
            let reference = scene["nodes"][index].clone();
            scene["nodes"][index] =
                mapped(global_mapping.get("nodes").unwrap_or(&empty_mapping), &reference);
        }
        None::<()>
    });
    for_each::animation(gltf, |animation, _id| {
        let mut sampler_mapping: HashMap<String, usize> = HashMap::new();
        if defined(animation.get("samplers")) {
            let samplers_value = animation["samplers"].take();
            animation["samplers"] = object_to_array(samplers_value, &mut sampler_mapping);
            for_each::animation_sampler(animation, |sampler, _index| {
                if defined(sampler.get("input")) {
                    let reference = sampler["input"].clone();
                    sampler["input"] = mapped(
                        global_mapping.get("accessors").unwrap_or(&empty_mapping),
                        &reference,
                    );
                }
                if defined(sampler.get("output")) {
                    let reference = sampler["output"].clone();
                    sampler["output"] = mapped(
                        global_mapping.get("accessors").unwrap_or(&empty_mapping),
                        &reference,
                    );
                }
                None::<()>
            });
        }
        if defined(animation.get("channels")) {
            for_each::animation_channel(animation, |channel, _index| {
                if defined(channel.get("sampler")) {
                    let key = key_string(&channel["sampler"]);
                    channel["sampler"] = sampler_mapping
                        .get(&key)
                        .map(|index| json!(*index))
                        .unwrap_or(Value::Null);
                }
                if defined(channel.get("target")) {
                    if defined(channel["target"].get("id")) {
                        let reference = channel["target"]["id"].clone();
                        channel["target"]["node"] = mapped(
                            global_mapping.get("nodes").unwrap_or(&empty_mapping),
                            &reference,
                        );
                    }
                    if let Some(target) = channel["target"].as_object_mut() {
                        target.remove("id");
                    }
                }
                None::<()>
            });
        }
        None::<()>
    });
    for_each::material(gltf, |material, _id| {
        if defined(material.get("technique")) {
            let reference = material["technique"].clone();
            material["technique"] = mapped(
                global_mapping.get("techniques").unwrap_or(&empty_mapping),
                &reference,
            );
        }
        let mut rewrites: Vec<(String, Value)> = Vec::new();
        for_each::material_value(material, |value, name| {
            if value.is_string() {
                let index =
                    mapped(global_mapping.get("textures").unwrap_or(&empty_mapping), value);
                rewrites.push((name, json!({ "index": index })));
            }
            None::<()>
        });
        if !rewrites.is_empty() {
            if !defined(material.get("values")) {
                material["values"] = json!({});
            }
            for (name, value) in rewrites {
                material["values"][name.as_str()] = value;
            }
        }
        if material
            .pointer("/extensions/KHR_materials_common/values")
            .is_some_and(|values| !values.is_null())
        {
            let materials_common = material
                .pointer_mut("/extensions/KHR_materials_common")
                .expect("checked above");
            let mut rewrites: Vec<(String, Value)> = Vec::new();
            for_each::material_value(materials_common, |value, name| {
                if value.is_string() {
                    let index = mapped(
                        global_mapping.get("textures").unwrap_or(&empty_mapping),
                        value,
                    );
                    rewrites.push((name, json!({ "index": index })));
                }
                None::<()>
            });
            for (name, value) in rewrites {
                materials_common["values"][name.as_str()] = value;
            }
        }
        None::<()>
    });
    for_each::image(gltf, |image, _id| {
        let extensions = image.get("extensions").cloned();
        if let Some(mut extensions) = extensions.filter(|value| !value.is_null()) {
            let binary_gltf = extensions.get("KHR_binary_glTF").cloned();
            if let Some(binary_gltf) = binary_gltf.filter(|value| !value.is_null()) {
                if let Some(buffer_view) = binary_gltf.get("bufferView") {
                    image["bufferView"] = mapped(
                        global_mapping.get("bufferViews").unwrap_or(&empty_mapping),
                        buffer_view,
                    );
                }
                if let Some(mime_type) = binary_gltf.get("mimeType") {
                    image["mimeType"] = mime_type.clone();
                }
                if let Some(extensions_object) = extensions.as_object_mut() {
                    extensions_object.remove("KHR_binary_glTF");
                }
            }
            if extensions.as_object().map_or(false, |map| map.is_empty()) {
                if let Some(image_object) = image.as_object_mut() {
                    image_object.remove("extensions");
                }
            } else {
                image["extensions"] = extensions;
            }
        }
        None::<()>
    });
    for_each::texture(gltf, |texture, _id| {
        if defined(texture.get("sampler")) {
            let reference = texture["sampler"].clone();
            texture["sampler"] =
                mapped(global_mapping.get("samplers").unwrap_or(&empty_mapping), &reference);
        }
        if defined(texture.get("source")) {
            let reference = texture["source"].clone();
            texture["source"] =
                mapped(global_mapping.get("images").unwrap_or(&empty_mapping), &reference);
        }
        None::<()>
    });
}

fn remove_animation_sampler_names(gltf: &mut Value) {
    for_each::animation(gltf, |animation, _id| {
        if defined(animation.get("samplers")) {
            for_each::animation_sampler(animation, |sampler, _index| {
                if let Some(sampler_object) = sampler.as_object_mut() {
                    sampler_object.remove("name");
                }
                None::<()>
            });
        }
        None::<()>
    });
}

fn remove_empty_arrays(gltf: &mut Value) {
    let empty_keys: Vec<String> = gltf
        .as_object()
        .map(|map| {
            map.iter()
                .filter(|(_, value)| value.as_array().map_or(false, |list| list.is_empty()))
                .map(|(key, _)| key.clone())
                .collect()
        })
        .unwrap_or_default();
    for key in empty_keys {
        if let Some(root) = gltf.as_object_mut() {
            root.remove(&key);
        }
    }
    for_each::node(gltf, |node, _id| {
        let children_empty = node
            .get("children")
            .and_then(|children| children.as_array())
            .map_or(false, |children| children.is_empty());
        if children_empty {
            if let Some(node_object) = node.as_object_mut() {
                node_object.remove("children");
            }
        }
        None::<()>
    });
}

fn strip_asset(gltf: &mut Value) {
    if let Some(asset) = gltf.get_mut("asset").and_then(|asset| asset.as_object_mut()) {
        asset.remove("profile");
        asset.remove("premultipliedAlpha");
    }
}

const KNOWN_EXTENSIONS: [&str; 3] = [
    "CESIUM_RTC",
    "KHR_materials_common",
    "WEB3D_quantized_attributes",
];

fn require_known_extensions(gltf: &mut Value) {
    if !defined(gltf.get("extensionsRequired")) {
        gltf["extensionsRequired"] = json!([]);
    }
    let extensions_used = gltf.get("extensionsUsed").cloned();
    if let Some(extensions_used) = extensions_used.filter(|value| !value.is_null()) {
        if let Some(list) = extensions_used.as_array() {
            for extension in list {
                if let Some(extension) = extension.as_str() {
                    if KNOWN_EXTENSIONS.contains(&extension) {
                        if let Some(required) = gltf
                            .get_mut("extensionsRequired")
                            .and_then(|value| value.as_array_mut())
                        {
                            required.push(json!(extension));
                        }
                    }
                }
            }
        }
    }
}

fn remove_buffer_type(gltf: &mut Value) {
    for_each::buffer(gltf, |buffer, _id| {
        if let Some(buffer_object) = buffer.as_object_mut() {
            buffer_object.remove("type");
        }
        None::<()>
    });
}

fn remove_texture_properties(gltf: &mut Value) {
    for_each::texture(gltf, |texture, _id| {
        if let Some(texture_object) = texture.as_object_mut() {
            texture_object.remove("format");
            texture_object.remove("internalFormat");
            texture_object.remove("target");
            texture_object.remove("type");
        }
        None::<()>
    });
}

fn require_attribute_set_index(gltf: &mut Value) {
    for_each::mesh(gltf, |mesh, _id| {
        for_each::mesh_primitive(mesh, |primitive, _index| {
            let mut renames: Vec<(String, Value)> = Vec::new();
            for_each::mesh_primitive_attribute(primitive, |accessor_id, semantic| {
                if semantic == "TEXCOORD" {
                    renames.push(("TEXCOORD_0".to_string(), accessor_id.clone()));
                } else if semantic == "COLOR" {
                    renames.push(("COLOR_0".to_string(), accessor_id.clone()));
                }
                None::<()>
            });
            for (new_semantic, accessor_id) in renames {
                primitive["attributes"][new_semantic.as_str()] = accessor_id;
            }
            if let Some(attributes) =
                primitive.get_mut("attributes").and_then(|attributes| attributes.as_object_mut())
            {
                attributes.remove("TEXCOORD");
                attributes.remove("COLOR");
            }
            None::<()>
        });
        None::<()>
    });
    for_each::technique(gltf, |technique, _id| {
        for_each::technique_parameter(technique, |parameter, _name| {
            match parameter.get("semantic").and_then(|value| value.as_str()) {
                Some("TEXCOORD") => parameter["semantic"] = json!("TEXCOORD_0"),
                Some("COLOR") => parameter["semantic"] = json!("COLOR_0"),
                _ => {}
            }
            None::<()>
        });
        None::<()>
    });
}

fn indexed_semantic(stripped: &str) -> Option<&'static str> {
    match stripped {
        "COLOR" => Some("COLOR"),
        "JOINT" | "JOINTS" => Some("JOINTS"),
        "TEXCOORD" => Some("TEXCOORD"),
        "WEIGHT" | "WEIGHTS" => Some("WEIGHTS"),
        _ => None,
    }
}

/// Position of the `_N` set-index suffix (mirrors `semantic.search(/_[0-9]+/g)`).
fn set_index_position(semantic: &str) -> Option<usize> {
    let bytes = semantic.as_bytes();
    (0..bytes.len())
        .find(|&index| bytes[index] == b'_' && bytes.get(index + 1).map_or(false, |c| c.is_ascii_digit()))
}

fn underscore_application_specific_semantics(gltf: &mut Value) {
    let mut mapped_semantics: HashMap<String, String> = HashMap::new();
    for_each::mesh(gltf, |mesh, _id| {
        for_each::mesh_primitive(mesh, |primitive, _index| {
            for_each::mesh_primitive_attribute(primitive, |_accessor_id, semantic| {
                if !semantic.starts_with('_') {
                    let (stripped, suffix) = match set_index_position(&semantic) {
                        Some(set_index) => (
                            semantic[..set_index].to_string(),
                            semantic[set_index..].to_string(),
                        ),
                        None => (semantic.clone(), "_0".to_string()),
                    };
                    if let Some(indexed) = indexed_semantic(&stripped) {
                        mapped_semantics.insert(semantic.clone(), format!("{indexed}{suffix}"));
                    } else if !matches!(stripped.as_str(), "POSITION" | "NORMAL" | "TANGENT") {
                        mapped_semantics.insert(semantic.clone(), format!("_{semantic}"));
                    }
                }
                None::<()>
            });
            for (semantic, mapped_semantic) in &mapped_semantics {
                let accessor_id = primitive
                    .get("attributes")
                    .and_then(|attributes| attributes.get(semantic))
                    .cloned();
                if let Some(accessor_id) = accessor_id.filter(|value| !value.is_null()) {
                    if let Some(attributes) = primitive
                        .get_mut("attributes")
                        .and_then(|attributes| attributes.as_object_mut())
                    {
                        attributes.remove(semantic);
                        attributes.insert(mapped_semantic.clone(), accessor_id);
                    }
                }
            }
            None::<()>
        });
        None::<()>
    });
    for_each::technique(gltf, |technique, _id| {
        for_each::technique_parameter(technique, |parameter, _name| {
            if let Some(semantic) = parameter.get("semantic").and_then(|value| value.as_str()) {
                if let Some(mapped_semantic) = mapped_semantics.get(semantic) {
                    parameter["semantic"] = json!(mapped_semantic);
                }
            }
            None::<()>
        });
        None::<()>
    });
}

fn clamp_camera_parameters(gltf: &mut Value) {
    for_each::camera(gltf, |camera, _id| {
        if defined(camera.get("perspective")) {
            let aspect_zero = camera
                .pointer("/perspective/aspectRatio")
                .map_or(false, |value| value.as_f64() == Some(0.0));
            if aspect_zero {
                if let Some(perspective) =
                    camera.get_mut("perspective").and_then(|p| p.as_object_mut())
                {
                    perspective.remove("aspectRatio");
                }
            }
            let yfov_zero = camera
                .pointer("/perspective/yfov")
                .map_or(false, |value| value.as_f64() == Some(0.0));
            if yfov_zero {
                camera["perspective"]["yfov"] = json!(1.0);
            }
        }
        None::<()>
    });
}

fn compute_accessor_byte_stride(gltf: &Value, accessor: &Value) -> usize {
    match accessor.get("byteStride").filter(|value| !value.is_null()) {
        Some(stride) if stride.as_u64().map_or(false, |stride| stride != 0) => {
            stride.as_u64().unwrap() as usize
        }
        _ => get_accessor_byte_stride(gltf, accessor),
    }
}

fn require_byte_length(gltf: &mut Value, sources: &PipelineBufferSources) {
    // buffer.byteLength defaults to the length of the attached binary source
    // (the JS reads `buffer.extras._pipeline.source.length`).
    let buffer_count = gltf
        .get("buffers")
        .and_then(|buffers| buffers.as_array())
        .map(|buffers| buffers.len())
        .unwrap_or(0);
    for buffer_index in 0..buffer_count {
        let has_byte_length = defined(gltf["buffers"][buffer_index].get("byteLength"));
        if !has_byte_length {
            if let Some(Some(source)) = sources.get(buffer_index) {
                gltf["buffers"][buffer_index]["byteLength"] = json!(source.buffer.len());
            }
        }
    }
    // bufferView.byteLength must cover every accessor's byte range
    let mut updates: Vec<(usize, usize)> = Vec::new();
    let accessor_count = gltf
        .get("accessors")
        .and_then(|accessors| accessors.as_array())
        .map(|accessors| accessors.len())
        .unwrap_or(0);
    for accessor_index in 0..accessor_count {
        let accessor = gltf["accessors"][accessor_index].clone();
        if let Some(buffer_view_id) = accessor
            .get("bufferView")
            .filter(|value| !value.is_null())
            .and_then(|value| value.as_u64())
        {
            let stride = compute_accessor_byte_stride(gltf, &accessor);
            let byte_offset = accessor
                .get("byteOffset")
                .and_then(|value| value.as_u64())
                .unwrap_or(0) as usize;
            let count = accessor.get("count").and_then(|value| value.as_u64()).unwrap_or(0) as usize;
            updates.push((buffer_view_id as usize, byte_offset + count * stride));
        }
    }
    for (buffer_view_id, byte_end) in updates {
        if let Some(buffer_view) = gltf
            .get_mut("bufferViews")
            .and_then(|views| views.as_array_mut())
            .and_then(|views| views.get_mut(buffer_view_id))
        {
            let existing = buffer_view
                .get("byteLength")
                .and_then(|value| value.as_u64())
                .unwrap_or(0) as usize;
            buffer_view["byteLength"] = json!(existing.max(byte_end));
        }
    }
}

#[allow(clippy::too_many_lines)]
fn move_byte_stride_to_buffer_view(gltf: &mut Value, sources: &mut PipelineBufferSources) {
    let mut buffer_view_has_vertex_attributes: HashSet<usize> = HashSet::new();
    // Borrow conflict: collect ids through a cloned snapshot of the glTF.
    let mut snapshot = gltf.clone();
    for_each::accessor_containing_vertex_attribute_data(&mut snapshot, |accessor_id| {
        if let Some(view_id) = gltf["accessors"][accessor_id]
            .get("bufferView")
            .filter(|value| !value.is_null())
            .and_then(|value| value.as_u64())
        {
            buffer_view_has_vertex_attributes.insert(view_id as usize);
        }
        None::<()>
    });

    // Map buffer views to a list of accessors
    let mut buffer_view_map: HashMap<usize, Vec<usize>> = HashMap::new();
    for_each::accessor(gltf, |accessor, accessor_id| {
        if let Some(view_id) = accessor
            .get("bufferView")
            .filter(|value| !value.is_null())
            .and_then(|value| value.as_u64())
        {
            buffer_view_map
                .entry(view_id as usize)
                .or_default()
                .push(accessor_id.parse::<usize>().unwrap_or(0));
        }
        None::<()>
    });

    // Split accessors with different byte strides
    let buffer_view_ids: Vec<usize> = buffer_view_map.keys().copied().collect();
    for buffer_view_id in buffer_view_ids {
        let Some(buffer_view) = gltf
            .get("bufferViews")
            .and_then(|views| views.as_array())
            .and_then(|views| views.get(buffer_view_id))
            .cloned()
        else {
            continue;
        };
        let mut accessor_indices = buffer_view_map.remove(&buffer_view_id).unwrap_or_default();
        accessor_indices.sort_by_key(|&accessor_index| {
            gltf["accessors"][accessor_index]
                .get("byteOffset")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
        });
        // Precompute per-accessor info (the JS computes strides before each
        // accessor's own byteStride is deleted; precomputing is equivalent).
        let accessor_infos: Vec<(usize, usize, usize, usize)> = accessor_indices
            .iter()
            .map(|&accessor_index| {
                let accessor = gltf["accessors"][accessor_index].clone();
                let stride = compute_accessor_byte_stride(gltf, &accessor);
                let byte_offset = accessor
                    .get("byteOffset")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0) as usize;
                let count =
                    accessor.get("count").and_then(|value| value.as_u64()).unwrap_or(0) as usize;
                (accessor_index, stride, byte_offset, count * stride)
            })
            .collect();

        let has_vertex_attributes = buffer_view_has_vertex_attributes.contains(&buffer_view_id);
        let mut current_byte_offset = 0usize;
        let mut current_index = 0usize;
        let accessors_length = accessor_infos.len();
        for i in 0..accessors_length {
            let (accessor_index, accessor_byte_stride, accessor_byte_offset, accessor_byte_length) =
                accessor_infos[i];
            if let Some(accessor) = gltf
                .get_mut("accessors")
                .and_then(|accessors| accessors.as_array_mut())
                .and_then(|accessors| accessors.get_mut(accessor_index))
            {
                if let Some(accessor_object) = accessor.as_object_mut() {
                    accessor_object.remove("byteStride");
                }
            }

            let has_next_accessor = i < accessors_length - 1;
            let next_accessor_byte_stride = has_next_accessor.then(|| accessor_infos[i + 1].1);
            if next_accessor_byte_stride != Some(accessor_byte_stride) {
                let mut new_buffer_view = buffer_view.clone();
                if has_vertex_attributes {
                    new_buffer_view["byteStride"] = json!(accessor_byte_stride);
                }
                // DEVIATION: the JS computes `newBufferView.byteOffset +=
                // currentByteOffset`, which is NaN when byteOffset is
                // undefined; treat a missing byteOffset as 0.
                let existing_byte_offset = new_buffer_view
                    .get("byteOffset")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0) as usize;
                new_buffer_view["byteOffset"] = json!(existing_byte_offset + current_byte_offset);
                new_buffer_view["byteLength"] =
                    json!(accessor_byte_offset + accessor_byte_length - current_byte_offset);
                let Some(buffer_views) = gltf.get_mut("bufferViews") else {
                    continue;
                };
                let new_buffer_view_id = add_to_array_value(buffer_views, new_buffer_view, false);
                for j in current_index..=i {
                    let (j_accessor_index, _, j_byte_offset, _) = accessor_infos[j];
                    if let Some(accessor) = gltf
                        .get_mut("accessors")
                        .and_then(|accessors| accessors.as_array_mut())
                        .and_then(|accessors| accessors.get_mut(j_accessor_index))
                    {
                        accessor["bufferView"] = json!(new_buffer_view_id);
                        accessor["byteOffset"] = json!(j_byte_offset - current_byte_offset);
                    }
                }
                // Set current byte offset to next accessor's byte offset
                current_byte_offset =
                    has_next_accessor.then(|| accessor_infos[i + 1].2).unwrap_or(0);
                current_index = i + 1;
            }
        }
    }

    // Remove unused buffer views
    remove_unused_elements(gltf, sources, Some(&["accessor", "bufferView", "buffer"]));
}

fn require_position_accessor_min_max(
    gltf: &mut Value,
    sources: &PipelineBufferSources,
) -> Result<(), RuntimeError> {
    let accessor_ids = for_each::accessor_ids_with_semantic(gltf, "POSITION");
    for accessor_id in accessor_ids {
        let Some(accessor) = gltf
            .get("accessors")
            .and_then(|accessors| accessors.as_array())
            .and_then(|accessors| accessors.get(accessor_id))
        else {
            continue;
        };
        if defined(accessor.get("min")) && defined(accessor.get("max")) {
            continue;
        }
        let accessor_clone = accessor.clone();
        let min_max = find_accessor_min_max(gltf, &accessor_clone, sources)?;
        if let Some(accessor) = gltf
            .get_mut("accessors")
            .and_then(|accessors| accessors.as_array_mut())
            .and_then(|accessors| accessors.get_mut(accessor_id))
        {
            accessor["min"] = json!(min_max.min);
            accessor["max"] = json!(min_max.max);
        }
    }
    Ok(())
}

fn require_animation_accessor_min_max(
    gltf: &mut Value,
    sources: &PipelineBufferSources,
) -> Result<(), RuntimeError> {
    // Collect sampler input accessor ids first (for_each borrows gltf).
    let mut input_accessor_ids: Vec<usize> = Vec::new();
    for_each::animation(gltf, |animation, _id| {
        let sampler_count = animation
            .get("samplers")
            .and_then(|samplers| samplers.as_array())
            .map(|samplers| samplers.len())
            .unwrap_or(0);
        for index in 0..sampler_count {
            if let Some(input) = animation["samplers"][index]
                .get("input")
                .and_then(|value| value.as_u64())
            {
                input_accessor_ids.push(input as usize);
            }
        }
        None::<()>
    });
    for accessor_id in input_accessor_ids {
        let Some(accessor) = gltf
            .get("accessors")
            .and_then(|accessors| accessors.as_array())
            .and_then(|accessors| accessors.get(accessor_id))
        else {
            continue;
        };
        if defined(accessor.get("min")) && defined(accessor.get("max")) {
            continue;
        }
        let accessor_clone = accessor.clone();
        let min_max = find_accessor_min_max(gltf, &accessor_clone, sources)?;
        if let Some(accessor) = gltf
            .get_mut("accessors")
            .and_then(|accessors| accessors.as_array_mut())
            .and_then(|accessors| accessors.get_mut(accessor_id))
        {
            accessor["min"] = json!(min_max.min);
            accessor["max"] = json!(min_max.max);
        }
    }
    Ok(())
}

fn validate_present_accessor_min_max(
    gltf: &mut Value,
    sources: &PipelineBufferSources,
) -> Result<(), RuntimeError> {
    let accessor_count = gltf
        .get("accessors")
        .and_then(|accessors| accessors.as_array())
        .map(|accessors| accessors.len())
        .unwrap_or(0);
    for accessor_id in 0..accessor_count {
        let (had_min, had_max) = {
            let accessor = &gltf["accessors"][accessor_id];
            (defined(accessor.get("min")), defined(accessor.get("max")))
        };
        if !had_min && !had_max {
            continue;
        }
        let accessor_clone = gltf["accessors"][accessor_id].clone();
        let min_max = find_accessor_min_max(gltf, &accessor_clone, sources)?;
        if had_min {
            gltf["accessors"][accessor_id]["min"] = json!(min_max.min);
        }
        if had_max {
            gltf["accessors"][accessor_id]["max"] = json!(min_max.max);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Legacy extension -> PBR material conversion
// ---------------------------------------------------------------------------

// It's not possible to upgrade glTF 1.0 shaders to 2.0 PBR materials in a
// generic way, but we can look for certain uniform names that are commonly
// found in glTF 1.0 assets and create PBR materials out of those.
const DEFAULT_BASE_COLOR_TEXTURE_NAMES: [&str; 4] =
    ["u_tex", "u_diffuse", "u_emission", "u_diffuse_tex"];
const DEFAULT_BASE_COLOR_FACTOR_NAMES: [&str; 2] = ["u_diffuse", "u_diffuse_mat"];

fn initialize_pbr_material(material: &mut Value) {
    if !defined(material.get("pbrMetallicRoughness")) {
        material["pbrMetallicRoughness"] = json!({});
    }
    material["pbrMetallicRoughness"]["roughnessFactor"] = json!(1.0);
    material["pbrMetallicRoughness"]["metallicFactor"] = json!(0.0);
}

fn is_texture(value: &Value) -> bool {
    defined(value.get("index"))
}

fn is_vec4(value: &Value) -> bool {
    value.as_array().map_or(false, |list| list.len() == 4)
}

fn srgb_to_linear(srgb: &Value) -> Value {
    let components = to_f64_array(srgb);
    let mut linear = vec![0.0; 4];
    linear[3] = components.get(3).copied().unwrap_or(0.0);
    for index in 0..3 {
        let c = components.get(index).copied().unwrap_or(0.0);
        linear[index] = if c <= 0.04045 {
            // 1 / 12.92
            c * 0.07739938080495356037151702786378
        } else {
            // 1 / 1.055
            ((c + 0.055) * 0.94786729857819905213270142180095).powf(2.4)
        };
    }
    json!(linear)
}

fn convert_techniques_to_pbr(gltf: &mut Value, options: &UpdateVersionOptions) {
    let base_color_texture_names: Vec<String> = options
        .base_color_texture_names
        .clone()
        .unwrap_or_else(|| DEFAULT_BASE_COLOR_TEXTURE_NAMES.iter().map(|s| s.to_string()).collect());
    let base_color_factor_names: Vec<String> = options
        .base_color_factor_names
        .clone()
        .unwrap_or_else(|| DEFAULT_BASE_COLOR_FACTOR_NAMES.iter().map(|s| s.to_string()).collect());

    // Future work: convert other values like emissive, specular, etc. Only
    // handling diffuse right now.
    for_each::material(gltf, |material, _id| {
        // Borrow conflict: collect rewrites first, apply afterwards.
        let mut rewrites: Vec<(&'static str, Value)> = Vec::new();
        for_each::material_value(material, |value, name| {
            if base_color_texture_names.iter().any(|candidate| candidate == &name)
                && is_texture(value)
            {
                rewrites.push(("baseColorTexture", value.clone()));
            } else if base_color_factor_names.iter().any(|candidate| candidate == &name)
                && is_vec4(value)
            {
                rewrites.push(("baseColorFactor", srgb_to_linear(value)));
            }
            None::<()>
        });
        for (property, value) in rewrites {
            initialize_pbr_material(material);
            material["pbrMetallicRoughness"][property] = value;
        }
        None::<()>
    });

    remove_extension(gltf, "KHR_techniques_webgl");
    remove_extension(gltf, "KHR_blend");
}

fn assign_as_base_color(material: &mut Value, base_color: Option<&Value>) {
    if let Some(base_color) = base_color.filter(|value| !value.is_null()) {
        if is_vec4(base_color) {
            material["pbrMetallicRoughness"]["baseColorFactor"] = srgb_to_linear(base_color);
        } else if is_texture(base_color) {
            material["pbrMetallicRoughness"]["baseColorTexture"] = base_color.clone();
        }
    }
}

fn assign_as_emissive(material: &mut Value, emissive: Option<&Value>) {
    if let Some(emissive) = emissive.filter(|value| !value.is_null()) {
        if is_vec4(emissive) {
            let components: Vec<Value> = emissive
                .as_array()
                .expect("checked by is_vec4")
                .iter()
                .take(3)
                .cloned()
                .collect();
            material["emissiveFactor"] = Value::Array(components);
        } else if is_texture(emissive) {
            material["emissiveTexture"] = emissive.clone();
        }
    }
}

fn convert_materials_common_to_pbr(gltf: &mut Value) {
    // Future work: convert KHR_materials_common lights to KHR_lights_punctual
    let mut needs_unlit = false;
    for_each::material(gltf, |material, _id| {
        let materials_common = material.pointer("/extensions/KHR_materials_common").cloned();
        let Some(materials_common) = materials_common.filter(|value| !value.is_null()) else {
            // Nothing to do
            return None;
        };

        let values = materials_common.get("values").filter(|value| !value.is_null());
        let ambient = values.and_then(|values| values.get("ambient")).cloned();
        let diffuse = values.and_then(|values| values.get("diffuse")).cloned();
        let emission = values.and_then(|values| values.get("emission")).cloned();
        let transparency =
            values.and_then(|values| values.get("transparency")).cloned();

        // These actually exist on the extension object, not the values object
        // despite what's shown in the spec
        let double_sided = materials_common.get("doubleSided").cloned();
        let transparent = materials_common.get("transparent").cloned();

        // Ignore specular and shininess for now because the conversion to PBR
        // isn't straightforward and depends on the technique
        initialize_pbr_material(material);

        let technique = materials_common
            .get("technique")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        if technique == "CONSTANT" {
            // Add the KHR_materials_unlit extension
            needs_unlit = true;
            if !defined(material.get("extensions")) {
                material["extensions"] = json!({});
            }
            material["extensions"]["KHR_materials_unlit"] = json!({});

            // The CONSTANT technique does not support 'diffuse', so assign
            // either the 'emission' or the 'ambient' as the base color
            assign_as_base_color(material, emission.as_ref());
            assign_as_base_color(material, ambient.as_ref());
        } else {
            // Assign the 'diffuse' as the base color, and the 'ambient' or
            // 'emissive' as the emissive part if they are present.
            assign_as_base_color(material, diffuse.as_ref());
            assign_as_emissive(material, ambient.as_ref());
            assign_as_emissive(material, emission.as_ref());
        }

        if let Some(double_sided) = double_sided.filter(|value| !value.is_null()) {
            material["doubleSided"] = double_sided;
        }
        if let Some(transparency) = transparency
            .filter(|value| !value.is_null())
            .and_then(|value| value.as_f64())
        {
            if defined(material.pointer("/pbrMetallicRoughness/baseColorFactor")) {
                if let Some(factor) = material
                    .pointer_mut("/pbrMetallicRoughness/baseColorFactor")
                    .and_then(|factor| factor.as_array_mut())
                {
                    if factor.len() > 3 {
                        let alpha = factor[3].as_f64().unwrap_or(0.0);
                        factor[3] = json!(alpha * transparency);
                    }
                }
            } else {
                material["pbrMetallicRoughness"]["baseColorFactor"] =
                    json!([1.0, 1.0, 1.0, transparency]);
            }
        }
        if let Some(transparent) = transparent
            .filter(|value| !value.is_null())
            .and_then(|value| value.as_bool())
        {
            material["alphaMode"] = json!(if transparent { "BLEND" } else { "OPAQUE" });
        }
        None::<()>
    });
    // DEVIATION: the JS calls addExtensionsUsed per material; calling it once
    // after the loop is equivalent because it deduplicates.
    if needs_unlit {
        add_extensions_used(gltf, "KHR_materials_unlit");
    }

    remove_extension(gltf, "KHR_materials_common");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gltf_pipeline::PipelineBufferSource;

    fn float_buffer(values: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(values.len() * 4);
        for value in values {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn source(buffer: Vec<u8>) -> PipelineBufferSources {
        vec![Some(PipelineBufferSource {
            buffer,
            byte_offset: 0,
        })]
    }

    fn minimal_gltf_10() -> Value {
        json!({
            "asset": { "version": "1.0" },
            "buffers": {
                "buf0": { "uri": "data:," }
            },
            "bufferViews": {
                "bv0": { "buffer": "buf0", "byteOffset": 0, "byteLength": 24 }
            },
            "accessors": {
                "acc0": {
                    "bufferView": "bv0",
                    "byteOffset": 0,
                    "componentType": 5126,
                    "count": 2,
                    "type": "VEC3"
                }
            },
            "meshes": {
                "mesh0": {
                    "primitives": [{ "attributes": { "POSITION": "acc0" }, "mode": 4 }]
                }
            },
            "nodes": { "node0": { "meshes": ["mesh0"] } },
            "scenes": { "scene0": { "nodes": ["node0"] } },
            "scene": "scene0"
        })
    }

    #[test]
    fn update_version_upgrades_10_to_20() {
        let mut gltf = minimal_gltf_10();
        let mut sources = source(float_buffer(&[0.0, -1.0, 2.0, 3.0, 4.0, 5.0]));
        update_version(&mut gltf, &mut sources, None).unwrap();

        assert_eq!(gltf["asset"]["version"], json!("2.0"));
        // Top-level objects became arrays
        assert_eq!(gltf["accessors"][0]["name"], json!("acc0"));
        assert_eq!(gltf["buffers"][0]["byteLength"], json!(24));
        assert_eq!(gltf["bufferViews"][0]["buffer"], json!(0));
        // POSITION accessor gained min/max
        assert_eq!(gltf["accessors"][0]["min"], json!([0.0, -1.0, 2.0]));
        assert_eq!(gltf["accessors"][0]["max"], json!([3.0, 4.0, 5.0]));
        // node.meshes split onto node.mesh; scene references numeric indices
        assert_eq!(gltf["nodes"][0]["mesh"], json!(0));
        assert!(gltf["nodes"][0].get("meshes").is_none());
        assert_eq!(gltf["scene"], json!(0));
        assert_eq!(gltf["scenes"][0]["nodes"], json!([0]));
    }

    #[test]
    fn update_version_converts_axis_angle_rotation() {
        let mut gltf = json!({
            "version": "0.8",
            "nodes": {
                "node0": { "rotation": [0.0, 0.0, 1.0, std::f64::consts::FRAC_PI_2] }
            }
        });
        let mut sources = PipelineBufferSources::new();
        update_version(&mut gltf, &mut sources, Some(&UpdateVersionOptions {
            target_version: Some("1.0".to_string()),
            ..Default::default()
        }))
        .unwrap();

        // glTF 1.0 keeps top-level collections as id-keyed objects; the
        // array conversion happens in the 1.0 -> 2.0 step.
        let rotation = &gltf["nodes"]["node0"]["rotation"];
        assert_eq!(rotation.as_array().map(|list| list.len()), Some(4));
        let sin_half = (std::f64::consts::FRAC_PI_4).sin();
        let cos_half = (std::f64::consts::FRAC_PI_4).cos();
        assert!((rotation[2].as_f64().unwrap() - sin_half).abs() < 1e-12);
        assert!((rotation[3].as_f64().unwrap() - cos_half).abs() < 1e-12);
    }

    #[test]
    fn objects_to_arrays_remaps_references() {
        let mut gltf = json!({
            "asset": { "version": "1.0" },
            "textures": { "texA": { "source": "imgA" } },
            "images": { "imgA": {} },
            "materials": {
                "matA": { "values": { "diffuse": "texA" } }
            },
            "nodes": { "nodeA": {} },
            "scenes": { "sceneA": { "nodes": ["nodeA"] } }
        });
        objects_to_arrays(&mut gltf);
        assert_eq!(gltf["textures"][0]["source"], json!(0));
        assert_eq!(gltf["materials"][0]["values"]["diffuse"], json!({ "index": 0 }));
        assert_eq!(gltf["scenes"][0]["nodes"], json!([0]));
        // names default to the legacy ids
        assert_eq!(gltf["images"][0]["name"], json!("imgA"));
    }

    #[test]
    fn srgb_to_linear_converts_components_and_preserves_alpha() {
        let linear = srgb_to_linear(&json!([0.5, 0.04, 0.0, 0.75]));
        let list = linear.as_array().unwrap();
        let expected = ((0.5_f64 + 0.055) * 0.94786729857819905213270142180095).powf(2.4);
        assert!((list[0].as_f64().unwrap() - expected).abs() < 1e-12);
        assert!((list[1].as_f64().unwrap() - 0.04 * 0.07739938080495356037151702786378).abs() < 1e-12);
        assert_eq!(list[2], json!(0.0));
        assert_eq!(list[3], json!(0.75));
    }

    #[test]
    fn convert_materials_common_to_pbr_constant_becomes_unlit() {
        let mut gltf = json!({
            "asset": { "version": "2.0" },
            "materials": [
                {
                    "extensions": {
                        "KHR_materials_common": {
                            "technique": "CONSTANT",
                            "values": {
                                "emission": [1.0, 0.0, 0.0, 1.0],
                                "transparency": 0.5
                            },
                            "transparent": true
                        }
                    }
                }
            ]
        });
        convert_materials_common_to_pbr(&mut gltf);

        let material = &gltf["materials"][0];
        assert!(material.pointer("/extensions/KHR_materials_unlit").is_some());
        assert_eq!(material["alphaMode"], json!("BLEND"));
        // emission assigned as base color, then transparency scales alpha
        let factor = material.pointer("/pbrMetallicRoughness/baseColorFactor").unwrap();
        assert!((factor[0].as_f64().unwrap() - 1.0).abs() < 1e-12);
        assert!((factor[3].as_f64().unwrap() - 0.5).abs() < 1e-12);
        assert!(gltf["extensionsUsed"]
            .as_array()
            .unwrap()
            .iter()
            .any(|extension| extension == "KHR_materials_unlit"));
        // KHR_materials_common removed from used/required and the material
        assert!(material.pointer("/extensions/KHR_materials_common").is_none());
    }

    #[test]
    fn remove_empty_nodes_deletes_empty_identity_nodes() {
        let mut gltf = json!({
            "nodes": {
                "empty": {},
                "parent": { "children": ["empty"] },
                "keeper": { "translation": [1.0, 0.0, 0.0] }
            },
            "scenes": { "scene0": { "nodes": ["empty", "keeper"] } }
        });
        remove_empty_nodes(&mut gltf);
        // `empty` and its now-empty `parent` are both removed
        assert!(gltf["nodes"].get("empty").is_none());
        assert!(gltf["nodes"].get("parent").is_none());
        assert!(gltf["nodes"].get("keeper").is_some());
        assert_eq!(gltf["scenes"]["scene0"]["nodes"], json!(["keeper"]));
    }
}
