//! glTF 1.0 → 2.0 JSON upgrade chain.
//!
//! Mirrors CesiumJS `packages/engine/Source/Scene/GltfPipeline/updateVersion.js`
//! (the real source of truth for version upgrading — `GltfLoader.js` itself
//! contains no upgrade logic). The entry point [`update_version`] reproduces
//! `updateVersion` (L41), which:
//!
//! 1. ensures `asset` / `asset.version` (defaulting to `"1.0"`),
//! 2. detects the version ([`detect_version`], L44–63),
//! 3. applies the chain of update functions until `2.0` (or `targetVersion`),
//! 4. unless `keepLegacyExtensions`, converts the legacy technique /
//!    `KHR_materials_common` material models to PBR.
//!
//! Per the milestone decision only the `1.0 → 2.0` step ([`gl_tf10to20`],
//! upstream L943) is implemented; `0.8 → 1.0` (`glTF08to10`) is deferred and
//! reported as [`GltfUpgradeError::UnsupportedVersion08`].
//!
//! # Scope: JSON transforms + binary stage
//!
//! The upgrade operates on raw [`serde_json::Value`] (glTF 1.0 stores its
//! top-level collections as object-keyed dictionaries which the array-based
//! typed [`crate::gltf_model::GltfModel`] cannot deserialize). The JSON-only
//! entry point [`update_version`] runs the structural transforms; the upstream
//! steps that depend on the decoded binary buffer (`extras._pipeline.source`)
//! live in [`crate::gltf_binary_stage`] and run only via
//! [`update_version_with_buffers`], which threads the decoded buffers through
//! [`gl_tf10to20`]:
//!
//! * `buffer.byteLength` from the decoded source (`requireByteLength`, buffer half),
//! * `requirePositionAccessorMinMax` / `requireAnimationAccessorMinMax` /
//!   `validatePresentAccessorMinMax` (all call `findAccessorMinMax`),
//! * `updateAccessorComponentTypes` (repacks JOINTS/WEIGHTS data),
//! * `removeUnusedElements` (invoked by `moveByteStrideToBufferView`).
//!
//! With no buffers supplied the chain is byte-for-byte identical to the
//! JSON-only M9.1 behaviour.
//!
//! # Deviation: collection ordering
//!
//! The workspace pins `serde_json = "1"` **without** the `preserve_order`
//! feature, so [`serde_json::Map`] is a `BTreeMap` iterating keys in
//! alphabetical order. `objectsToArrays` therefore assigns array indices in
//! alphabetical key order rather than JS insertion order. Every reference is
//! remapped through the same `global_mapping`, so the upgraded asset remains
//! internally consistent and semantically equivalent; only the ordering of
//! elements within a converted collection may differ for multi-entry objects.
//!
//! DEVIATION: serde_json 无 preserve_order → objectsToArrays 索引按 BTreeMap
//! 字母序，经同一 global_mapping 一致重映射语义等价；see docs/deviations.md#dev-012

use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};

use crate::gltf_technique_upgrade::{
    convert_materials_common_to_pbr, convert_techniques_to_pbr, move_technique_render_states,
    move_techniques_to_extension, DEFAULT_BASE_COLOR_FACTOR_NAMES, DEFAULT_BASE_COLOR_TEXTURE_NAMES,
};
use crate::gltf_upgrade_util::{
    component_size_in_bytes, get_accessor_byte_stride, number_of_components_for_type,
    object_to_array,
};

/// The glTF asset version detected on the root `version` or `asset.version`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GltfVersion {
    /// glTF 0.8 (upgrade to 1.0 is deferred / unsupported here).
    V08,
    /// glTF 1.0 — upgraded to 2.0 by [`gl_tf10to20`].
    V10,
    /// glTF 2.0 — passthrough (no structural upgrade).
    V20,
    /// Could not be determined; upstream defaults to `1.0`.
    Unknown,
}

impl GltfVersion {
    /// The canonical version string (`"0.8"` / `"1.0"` / `"2.0"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            GltfVersion::V08 => "0.8",
            GltfVersion::V10 => "1.0",
            GltfVersion::V20 => "2.0",
            GltfVersion::Unknown => "1.0",
        }
    }
}

/// Errors produced by the upgrade chain.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GltfUpgradeError {
    /// The asset is glTF 0.8; only `1.0 → 2.0` is implemented (Q5 decision).
    #[error("glTF 0.8 -> 1.0 upgrade is deferred; only 1.0 -> 2.0 is supported")]
    UnsupportedVersion08,
    /// The root JSON value is not an object.
    #[error("glTF asset root is not a JSON object")]
    NotAnObject,
}

/// Options mirroring `updateVersion(gltf, options)` (updateVersion.js L33–36).
#[derive(Debug, Clone, Default)]
pub struct UpgradeOptions {
    /// Stop upgrading once this version is reached (`options.targetVersion`).
    pub target_version: Option<String>,
    /// When set, skip the legacy technique / `KHR_materials_common` → PBR
    /// conversion (`options.keepLegacyExtensions`).
    pub keep_legacy_extensions: bool,
    /// Uniform names indicating a base-color *texture*
    /// (`options.baseColorTextureNames`); defaults to
    /// [`DEFAULT_BASE_COLOR_TEXTURE_NAMES`].
    pub base_color_texture_names: Option<Vec<String>>,
    /// Uniform names indicating a base-color *factor*
    /// (`options.baseColorFactorNames`); defaults to
    /// [`DEFAULT_BASE_COLOR_FACTOR_NAMES`].
    pub base_color_factor_names: Option<Vec<String>>,
}

/// Detects the glTF version without mutating the asset.
///
/// Mirrors the version resolution in `updateVersion` (L44–63): the root-level
/// `version` (a glTF 0.8 convention) takes precedence over `asset.version`;
/// unknown values are truncated to three characters and finally default to
/// `"1.0"`.
#[must_use]
pub fn detect_version(gltf: &Value) -> GltfVersion {
    normalize_version(&version_string(gltf))
}

/// Reads the raw version string (root `version`, else `asset.version`, else
/// `"1.0"`), tolerating a numeric root `version` (e.g. `0.8`).
fn version_string(gltf: &Value) -> String {
    if let Some(v) = gltf.get("version") {
        match v {
            Value::String(s) => return s.clone(),
            Value::Number(n) => return n.to_string(),
            _ => {}
        }
    }
    if let Some(s) = gltf.pointer("/asset/version").and_then(Value::as_str) {
        return s.to_string();
    }
    "1.0".to_string()
}

/// `updateFunctions` membership test + truncation fallback (L54–63).
fn normalize_version(v: &str) -> GltfVersion {
    match v {
        "0.8" => GltfVersion::V08,
        "1.0" => GltfVersion::V10,
        "2.0" => GltfVersion::V20,
        _ => {
            let truncated: String = v.chars().take(3).collect();
            match truncated.as_str() {
                "0.8" => GltfVersion::V08,
                "1.0" => GltfVersion::V10,
                "2.0" => GltfVersion::V20,
                // Default to 1.0 if it cannot be determined (L61).
                _ => GltfVersion::V10,
            }
        }
    }
}

/// Upgrades `gltf` in place to version 2.0 (or `options.target_version`).
///
/// Faithful port of `updateVersion` (updateVersion.js L41–82). For a plain 2.0
/// asset this is a passthrough: the update loop does not run and the trailing
/// PBR converters are no-ops when the legacy extensions are absent, leaving the
/// asset unchanged.
///
/// JSON-only: the binary-buffer-dependent steps (min/max, JOINTS/WEIGHTS
/// repackaging, unused-element removal) are skipped. Use
/// [`update_version_with_buffers`] to run the full chain against decoded
/// buffers.
///
/// # Errors
/// Returns [`GltfUpgradeError::NotAnObject`] if the root is not an object, or
/// [`GltfUpgradeError::UnsupportedVersion08`] if the asset is glTF 0.8 and the
/// target version requires the (deferred) `0.8 → 1.0` step.
pub fn update_version(gltf: &mut Value, options: &UpgradeOptions) -> Result<(), GltfUpgradeError> {
    update_version_impl(gltf, options, None)
}

/// Like [`update_version`], but also runs the binary-buffer-dependent steps of
/// `glTF10to20` (`requirePositionAccessorMinMax` / `requireAnimationAccessorMinMax`
/// / `validatePresentAccessorMinMax` / `updateAccessorComponentTypes` /
/// `removeUnusedElements` / buffer-level `requireByteLength`) against the
/// decoded buffer sources.
///
/// `buffers[i]` must be the decoded source of `gltf.buffers[i]` (for a GLB,
/// `buffers = vec![binary_chunk]`). The vec may grow when
/// `updateAccessorComponentTypes` repacks JOINTS/WEIGHTS into new buffers.
///
/// # Errors
/// Same as [`update_version`].
pub fn update_version_with_buffers(
    gltf: &mut Value,
    options: &UpgradeOptions,
    buffers: &mut Vec<Vec<u8>>,
) -> Result<(), GltfUpgradeError> {
    update_version_impl(gltf, options, Some(buffers))
}

fn update_version_impl(
    gltf: &mut Value,
    options: &UpgradeOptions,
    mut buffers: Option<&mut Vec<Vec<u8>>>,
) -> Result<(), GltfUpgradeError> {
    // Ensure asset + asset.version (L46–50).
    {
        let root = gltf.as_object_mut().ok_or(GltfUpgradeError::NotAnObject)?;
        if root.get("asset").is_none() {
            root.insert("asset".to_string(), json!({ "version": "1.0" }));
        }
        if let Some(asset) = root.get_mut("asset").and_then(Value::as_object_mut) {
            if asset.get("version").is_none() {
                asset.insert("version".to_string(), json!("1.0"));
            }
        }
    }

    let target = options.target_version.as_deref();
    let mut version = detect_version(gltf);

    // while (defined(updateFunction)) { if (version === targetVersion) break; ... }
    loop {
        match version {
            GltfVersion::V20 | GltfVersion::Unknown => break,
            GltfVersion::V08 => {
                if target == Some("0.8") {
                    break;
                }
                // glTF08to10 is deferred (Q5: only 1.0 -> 2.0).
                return Err(GltfUpgradeError::UnsupportedVersion08);
            }
            GltfVersion::V10 => {
                if target == Some("1.0") {
                    break;
                }
                gl_tf10to20(gltf, buffers.take());
                // version = gltf.asset.version (now "2.0") -> loop exits.
                version = normalize_version(
                    gltf.pointer("/asset/version")
                        .and_then(Value::as_str)
                        .unwrap_or("2.0"),
                );
            }
        }
    }

    if !options.keep_legacy_extensions {
        let texture_names = options
            .base_color_texture_names
            .clone()
            .unwrap_or_else(|| DEFAULT_BASE_COLOR_TEXTURE_NAMES.iter().map(|s| (*s).to_string()).collect());
        let factor_names = options
            .base_color_factor_names
            .clone()
            .unwrap_or_else(|| DEFAULT_BASE_COLOR_FACTOR_NAMES.iter().map(|s| (*s).to_string()).collect());
        convert_techniques_to_pbr(gltf, &texture_names, &factor_names);
        convert_materials_common_to_pbr(gltf);
    }

    Ok(())
}

/// `glTF10to20` (updateVersion.js L943–989): the structural 1.0 → 2.0 transform.
///
/// Steps that require the decoded binary buffer are skipped here and documented
/// inline; see the module header for the full deferral list.
fn gl_tf10to20(gltf: &mut Value, mut buffers: Option<&mut Vec<Vec<u8>>>) {
    // gltf.asset.version = "2.0" (L944–945).
    if let Some(root) = gltf.as_object_mut() {
        let asset = root
            .entry("asset")
            .or_insert_with(|| Value::Object(Map::new()));
        if let Some(asset) = asset.as_object_mut() {
            asset.insert("version".to_string(), json!("2.0"));
        }
    }

    update_instance_techniques(gltf); // L947
    remove_animation_samplers_indirection(gltf); // L949
    remove_empty_nodes(gltf); // L951
    objects_to_arrays(gltf); // L953
    remove_animation_sampler_names(gltf); // L955
    strip_asset(gltf); // L957
    require_known_extensions(gltf); // L959
    require_byte_length(gltf, buffers.as_deref()); // L961 (buffer half needs decoded source)
    move_byte_stride_to_buffer_view(gltf); // L963
    // Binary-buffer-dependent steps (M9.2): run only when decoded buffers are
    // supplied; with `None` the chain is byte-for-byte the JSON-only M9.1 path.
    if let Some(bufs) = buffers.as_mut() {
        crate::gltf_binary_stage::remove_unused_elements(gltf, bufs); // L837 (end of moveByteStride)
        crate::gltf_binary_stage::require_position_accessor_min_max(gltf, bufs); // L965
        crate::gltf_binary_stage::require_animation_accessor_min_max(gltf, bufs); // L967
        crate::gltf_binary_stage::validate_present_accessor_min_max(gltf, bufs); // L970
    }
    remove_buffer_type(gltf); // L972
    remove_texture_properties(gltf); // L974
    require_attribute_set_index(gltf); // L976
    underscore_application_specific_semantics(gltf); // L978
    if let Some(bufs) = buffers.as_mut() {
        crate::gltf_binary_stage::update_accessor_component_types(gltf, bufs); // L980
    }
    clamp_camera_parameters(gltf); // L982
    move_technique_render_states(gltf); // L984 (gltf_technique_upgrade.rs)
    move_techniques_to_extension(gltf); // L986 (gltf_technique_upgrade.rs)
    remove_empty_arrays(gltf); // L988
}

/// `updateInstanceTechniques` (L84): hoist `material.instanceTechnique`
/// (`technique` + `values`) onto the material itself.
fn update_instance_techniques(gltf: &mut Value) {
    for_each_material_mut(gltf, &mut |material| {
        let Some(obj) = material.as_object_mut() else {
            return;
        };
        let Some(instance) = obj.remove("instanceTechnique") else {
            return;
        };
        if let Some(technique) = instance.get("technique") {
            obj.insert("technique".to_string(), technique.clone());
        }
        if let Some(values) = instance.get("values") {
            obj.insert("values".to_string(), values.clone());
        }
    });
}

/// `removeAnimationSamplersIndirection` (L270): resolve `sampler.input` /
/// `sampler.output` through `animation.parameters`, then delete `parameters`.
fn remove_animation_samplers_indirection(gltf: &mut Value) {
    for_each_top_level_mut_named(gltf, "animations", &mut |animation| {
        let Some(obj) = animation.as_object_mut() else {
            return;
        };
        let Some(parameters) = obj.remove("parameters") else {
            return;
        };
        if let Some(samplers) = obj.get_mut("samplers") {
            for_each_sampler_mut(samplers, |sampler| {
                if let Some(sobj) = sampler.as_object_mut() {
                    for field in ["input", "output"] {
                        if let Some(key) = sobj.get(field).and_then(Value::as_str).map(String::from)
                        {
                            if let Some(resolved) = parameters.get(&key) {
                                sobj.insert(field.to_string(), resolved.clone());
                            }
                        }
                    }
                }
            });
        }
    });
}

/// `removeEmptyNodes` (L906) + `isNodeEmpty` (L851) + `deleteNode` (L874).
/// Runs before `objectsToArrays`, so nodes are still object-keyed by id.
fn remove_empty_nodes(gltf: &mut Value) {
    let ids: Vec<String> = match gltf.get("nodes") {
        Some(Value::Object(nodes)) => nodes.keys().cloned().collect(),
        _ => return,
    };
    for id in ids {
        let empty = gltf
            .get("nodes")
            .and_then(|n| n.get(&id))
            .map(is_node_empty)
            .unwrap_or(false);
        if empty {
            delete_node(gltf, &id);
        }
    }
}

/// `isNodeEmpty` (L851): a node with no content and identity/no transform.
fn is_node_empty(node: &Value) -> bool {
    let Some(obj) = node.as_object() else {
        return false;
    };
    let arr_nonempty = |k: &str| {
        obj.get(k)
            .and_then(Value::as_array)
            .map(|a| !a.is_empty())
            .unwrap_or(false)
    };
    if arr_nonempty("children") || arr_nonempty("meshes") {
        return false;
    }
    for key in ["camera", "skin", "skeletons", "jointName", "extensions", "extras"] {
        if is_defined(obj, key) {
            return false;
        }
    }
    if let Some(t) = obj.get("translation").and_then(Value::as_array) {
        if !number_array_eq(t, &[0.0, 0.0, 0.0]) {
            return false;
        }
    }
    if let Some(s) = obj.get("scale").and_then(Value::as_array) {
        if !number_array_eq(s, &[1.0, 1.0, 1.0]) {
            return false;
        }
    }
    if let Some(r) = obj.get("rotation").and_then(Value::as_array) {
        if !number_array_eq(r, &[0.0, 0.0, 0.0, 1.0]) {
            return false;
        }
    }
    if let Some(m) = obj.get("matrix").and_then(Value::as_array) {
        if !is_identity_matrix(m) {
            return false;
        }
    }
    true
}

/// `deleteNode` (L874): remove a node from scenes and parents, recursing into
/// parents that become empty, then delete it from `gltf.nodes`.
fn delete_node(gltf: &mut Value, node_id: &str) {
    // Remove from every scene's node list.
    remove_id_from_children(gltf, "scenes", "nodes", node_id);

    // Remove from parents' children; record which parents referenced it.
    let mut parents: Vec<String> = Vec::new();
    if let Some(Value::Object(nodes)) = gltf.get_mut("nodes") {
        for (pid, parent) in nodes.iter_mut() {
            if let Some(children) = parent.get_mut("children").and_then(Value::as_array_mut) {
                if let Some(pos) = children
                    .iter()
                    .position(|x| x.as_str() == Some(node_id))
                {
                    children.remove(pos);
                    parents.push(pid.clone());
                }
            }
        }
    }

    // Delete the node itself.
    if let Some(Value::Object(nodes)) = gltf.get_mut("nodes") {
        nodes.remove(node_id);
    }

    // Recurse into parents that are now empty.
    for pid in parents {
        let parent_empty = gltf
            .get("nodes")
            .and_then(|n| n.get(&pid))
            .map(is_node_empty)
            .unwrap_or(false);
        if parent_empty {
            delete_node(gltf, &pid);
        }
    }
}

/// Removes `id` from `gltf[collection][*][children_key]` arrays (object-keyed
/// or array form of the collection).
fn remove_id_from_children(gltf: &mut Value, collection: &str, children_key: &str, id: &str) {
    match gltf.get_mut(collection) {
        Some(Value::Object(map)) => {
            for (_k, item) in map.iter_mut() {
                if let Some(arr) = item.get_mut(children_key).and_then(Value::as_array_mut) {
                    if let Some(pos) = arr.iter().position(|x| x.as_str() == Some(id)) {
                        arr.remove(pos);
                    }
                }
            }
        }
        Some(Value::Array(list)) => {
            for item in list.iter_mut() {
                if let Some(arr) = item.get_mut(children_key).and_then(Value::as_array_mut) {
                    if let Some(pos) = arr.iter().position(|x| x.as_str() == Some(id)) {
                        arr.remove(pos);
                    }
                }
            }
        }
        _ => {}
    }
}

/// `objectsToArrays` (L306–563): convert every object-keyed top-level collection
/// to an array and rewrite all id references to array indices.
fn objects_to_arrays(gltf: &mut Value) {
    const COLLECTIONS: [&str; 16] = [
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
    let Some(root) = gltf.as_object_mut() else {
        return;
    };

    // jointName -> node id (read before conversion, L327–338).
    let mut joint_name_to_id: HashMap<String, String> = HashMap::new();
    if let Some(Value::Object(nodes)) = root.get("nodes") {
        for (id, node) in nodes {
            if let Some(jn) = node.get("jointName").and_then(Value::as_str) {
                joint_name_to_id.insert(jn.to_string(), id.clone());
            }
        }
    }

    // Convert each object-keyed collection to an array (L340–351).
    let mut global_mapping: HashMap<&str, HashMap<String, usize>> = HashMap::new();
    for coll in COLLECTIONS {
        let mut mapping = HashMap::new();
        if let Some(Value::Object(_)) = root.get(coll) {
            if let Some(Value::Object(obj)) = root.remove(coll) {
                let (arr, m) = object_to_array(obj);
                mapping = m;
                root.insert(coll.to_string(), Value::Array(arr));
            }
        }
        global_mapping.insert(coll, mapping);
    }

    // jointName -> node index (L353–358).
    let node_map = &global_mapping["nodes"];
    let mut joint_name_to_index: HashMap<String, usize> = HashMap::new();
    for (jn, id) in &joint_name_to_id {
        if let Some(idx) = node_map.get(id) {
            joint_name_to_index.insert(jn.clone(), *idx);
        }
    }

    // ---- Fix references (L360–562) ----
    let empty = HashMap::new();
    let m = |coll: &str| -> &HashMap<String, usize> { global_mapping.get(coll).unwrap_or(&empty) };

    // gltf.scene (L361).
    if let Some(sid) = root.get("scene").and_then(Value::as_str).map(String::from) {
        if let Some(idx) = m("scenes").get(&sid) {
            root.insert("scene".to_string(), json!(*idx));
        }
    }

    // bufferView.buffer (L364).
    remap_field(root, "bufferViews", "buffer", m("buffers"));

    // accessor.bufferView (L369).
    remap_field(root, "accessors", "bufferView", m("bufferViews"));

    // shader.extensions.KHR_binary_glTF.bufferView -> shader.bufferView (L374).
    if let Some(Value::Array(shaders)) = root.get_mut("shaders") {
        let bvs = m("bufferViews");
        for shader in shaders.iter_mut() {
            let Some(obj) = shader.as_object_mut() else {
                continue;
            };
            let binary = obj
                .get_mut("extensions")
                .and_then(|e| e.get_mut("KHR_binary_glTF"))
                .and_then(|b| b.get("bufferView"))
                .and_then(Value::as_str)
                .map(String::from);
            if let Some(bv) = binary {
                if let Some(idx) = bvs.get(&bv) {
                    obj.insert("bufferView".to_string(), json!(*idx));
                }
                if let Some(exts) = obj.get_mut("extensions").and_then(Value::as_object_mut) {
                    exts.remove("KHR_binary_glTF");
                    if exts.is_empty() {
                        obj.remove("extensions");
                    }
                }
            }
        }
    }

    // program.vertexShader / fragmentShader (L387).
    remap_field(root, "programs", "vertexShader", m("shaders"));
    remap_field(root, "programs", "fragmentShader", m("shaders"));

    // technique.program + parameters (L395).
    if let Some(Value::Array(techniques)) = root.get_mut("techniques") {
        let programs = m("programs");
        let nodes = m("nodes");
        let textures = m("textures");
        for technique in techniques.iter_mut() {
            remap_field_in(technique, "program", programs);
            if let Some(params) = technique.get_mut("parameters").and_then(Value::as_object_mut) {
                for (_name, param) in params.iter_mut() {
                    remap_field_in(param, "node", nodes);
                    let value_is_string = param
                        .get("value")
                        .and_then(Value::as_str)
                        .map(String::from);
                    if let Some(v) = value_is_string {
                        if let Some(idx) = textures.get(&v) {
                            if let Some(pobj) = param.as_object_mut() {
                                pobj.insert("value".to_string(), json!({ "index": *idx }));
                            }
                        }
                    }
                }
            }
        }
    }

    // mesh primitives (L411).
    if let Some(Value::Array(meshes)) = root.get_mut("meshes") {
        let accessors = m("accessors");
        let materials = m("materials");
        for mesh in meshes.iter_mut() {
            if let Some(prims) = mesh.get_mut("primitives").and_then(Value::as_array_mut) {
                for prim in prims.iter_mut() {
                    remap_field_in(prim, "indices", accessors);
                    remap_field_in(prim, "material", materials);
                    if let Some(attrs) = prim.get_mut("attributes").and_then(Value::as_object_mut) {
                        for (_sem, acc) in attrs.iter_mut() {
                            let mapped = map_id(accessors, acc);
                            *acc = mapped;
                        }
                    }
                }
            }
        }
    }

    // nodes (L427) — collects skin.skeleton assignments and pending mesh nodes.
    let mut skeleton_assignments: Vec<(u64, u64)> = Vec::new();
    if let Some(Value::Array(nodes)) = root.get_mut("nodes") {
        let nodes_map = m("nodes");
        let meshes_map = m("meshes");
        let cameras_map = m("cameras");
        let skins_map = m("skins");
        let original_len = nodes.len();
        let mut next_index = original_len;
        let mut pending: Vec<Value> = Vec::new();
        for node in nodes.iter_mut().take(original_len) {
            let Some(obj) = node.as_object_mut() else {
                continue;
            };
            // children
            if let Some(children) = obj.get_mut("children").and_then(Value::as_array_mut) {
                for child in children.iter_mut() {
                    let mapped = map_id(nodes_map, child);
                    *child = mapped;
                }
            }
            // meshes -> mesh (+ extra mesh nodes)
            let meshes_val = obj.remove("meshes");
            if let Some(Value::Array(mesh_ids)) = meshes_val {
                if !mesh_ids.is_empty() {
                    if let Some(first) = mesh_ids.first() {
                        obj.insert("mesh".to_string(), map_id(meshes_map, first));
                    }
                    for extra in mesh_ids.iter().skip(1) {
                        let mesh_node = json!({ "mesh": map_id(meshes_map, extra) });
                        let mesh_node_id = next_index;
                        next_index += 1;
                        pending.push(mesh_node);
                        let children = obj
                            .entry("children")
                            .or_insert_with(|| Value::Array(Vec::new()));
                        if let Some(children) = children.as_array_mut() {
                            children.push(json!(mesh_node_id));
                        }
                    }
                }
            }
            // camera / skin
            remap_field_in_obj(obj, "camera", cameras_map);
            remap_field_in_obj(obj, "skin", skins_map);
            // skeletons -> skin.skeleton
            if let Some(Value::Array(skeletons)) = obj.remove("skeletons") {
                if let (Some(first), Some(skin_idx)) = (
                    skeletons.first().and_then(Value::as_str),
                    obj.get("skin").and_then(Value::as_u64),
                ) {
                    if let Some(node_idx) = nodes_map.get(first) {
                        skeleton_assignments.push((skin_idx, *node_idx as u64));
                    }
                }
            }
            obj.remove("jointName");
        }
        nodes.extend(pending);
    }

    // skins (L475) — inverseBindMatrices, jointNames -> joints, + skeleton.
    if let Some(Value::Array(skins)) = root.get_mut("skins") {
        let accessors = m("accessors");
        for (i, skin) in skins.iter_mut().enumerate() {
            let Some(obj) = skin.as_object_mut() else {
                continue;
            };
            for (skin_idx, node_idx) in &skeleton_assignments {
                if *skin_idx == i as u64 {
                    obj.insert("skeleton".to_string(), json!(*node_idx));
                }
            }
            remap_field_in_obj(obj, "inverseBindMatrices", accessors);
            if let Some(Value::Array(joint_names)) = obj.remove("jointNames") {
                let joints: Vec<Value> = joint_names
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|jn| joint_name_to_index.get(jn).map_or(Value::Null, |x| json!(*x)))
                    .collect();
                obj.insert("joints".to_string(), Value::Array(joints));
            }
        }
    }

    // scene.nodes (L491).
    if let Some(Value::Array(scenes)) = root.get_mut("scenes") {
        let nodes_map = m("nodes");
        for scene in scenes.iter_mut() {
            if let Some(list) = scene.get_mut("nodes").and_then(Value::as_array_mut) {
                for n in list.iter_mut() {
                    let mapped = map_id(nodes_map, n);
                    *n = mapped;
                }
            }
        }
    }

    // animations (L500).
    if let Some(Value::Array(anims)) = root.get_mut("animations") {
        let accessors = m("accessors");
        let nodes_map = m("nodes");
        for anim in anims.iter_mut() {
            let Some(obj) = anim.as_object_mut() else {
                continue;
            };
            let mut sampler_mapping: HashMap<String, usize> = HashMap::new();
            if let Some(Value::Object(sobj)) = obj.get("samplers").cloned() {
                let (arr, sm) = object_to_array(sobj);
                sampler_mapping = sm;
                obj.insert("samplers".to_string(), Value::Array(arr));
            }
            if let Some(samplers) = obj.get_mut("samplers") {
                for_each_sampler_mut(samplers, |sampler| {
                    remap_field_in(sampler, "input", accessors);
                    remap_field_in(sampler, "output", accessors);
                });
            }
            if let Some(Value::Array(channels)) = obj.get_mut("channels") {
                for channel in channels.iter_mut() {
                    let Some(cobj) = channel.as_object_mut() else {
                        continue;
                    };
                    if let Some(sid) = cobj.get("sampler").cloned() {
                        cobj.insert("sampler".to_string(), map_id(&sampler_mapping, &sid));
                    }
                    if let Some(target) = cobj.get_mut("target").and_then(Value::as_object_mut) {
                        if let Some(id) = target.remove("id") {
                            target.insert("node".to_string(), map_id(nodes_map, &id));
                        }
                    }
                }
            }
        }
    }

    // materials (L516).
    if let Some(Value::Array(materials)) = root.get_mut("materials") {
        let techniques = m("techniques");
        let textures = m("textures");
        for material in materials.iter_mut() {
            remap_field_in(material, "technique", techniques);
            remap_material_values(material.get_mut("values"), textures);
            let common_values = material
                .get_mut("extensions")
                .and_then(|e| e.get_mut("KHR_materials_common"))
                .and_then(|c| c.get_mut("values"));
            remap_material_values(common_values, textures);
        }
    }

    // images (L541).
    if let Some(Value::Array(images)) = root.get_mut("images") {
        let bvs = m("bufferViews");
        for image in images.iter_mut() {
            let Some(obj) = image.as_object_mut() else {
                continue;
            };
            let binary = obj.get_mut("extensions").and_then(|e| e.get_mut("KHR_binary_glTF"));
            if let Some(binary) = binary {
                let bv = binary.get("bufferView").and_then(Value::as_str).map(String::from);
                let mime = binary.get("mimeType").cloned();
                if let Some(bv) = bv {
                    if let Some(idx) = bvs.get(&bv) {
                        obj.insert("bufferView".to_string(), json!(*idx));
                    }
                }
                if let Some(mime) = mime {
                    obj.insert("mimeType".to_string(), mime);
                }
                if let Some(exts) = obj.get_mut("extensions").and_then(Value::as_object_mut) {
                    exts.remove("KHR_binary_glTF");
                    if exts.is_empty() {
                        obj.remove("extensions");
                    }
                }
            }
        }
    }

    // textures (L555).
    remap_field(root, "textures", "sampler", m("samplers"));
    remap_field(root, "textures", "source", m("images"));
}

/// Rewrites `material.values[name]` (and `KHR_materials_common.values`) string
/// references into `{ "index": <texture index> }` (L520–538).
fn remap_material_values(values: Option<&mut Value>, textures: &HashMap<String, usize>) {
    let Some(Value::Object(map)) = values else {
        return;
    };
    for (_name, value) in map.iter_mut() {
        if let Some(s) = value.as_str().map(String::from) {
            if let Some(idx) = textures.get(&s) {
                *value = json!({ "index": *idx });
            }
        }
    }
}

/// `removeAnimationSamplerNames` (L565).
fn remove_animation_sampler_names(gltf: &mut Value) {
    for_each_top_level_mut_named(gltf, "animations", &mut |animation| {
        if let Some(samplers) = animation.get_mut("samplers") {
            for_each_sampler_mut(samplers, |sampler| {
                if let Some(obj) = sampler.as_object_mut() {
                    obj.remove("name");
                }
            });
        }
    });
}

/// `removeEmptyArrays` (L573): delete empty top-level arrays and empty
/// `node.children`.
fn remove_empty_arrays(gltf: &mut Value) {
    let Some(root) = gltf.as_object_mut() else {
        return;
    };
    let empty_keys: Vec<String> = root
        .iter()
        .filter(|(_k, v)| v.as_array().map(|a| a.is_empty()).unwrap_or(false))
        .map(|(k, _v)| k.clone())
        .collect();
    for key in empty_keys {
        root.remove(&key);
    }
    if let Some(Value::Array(nodes)) = root.get_mut("nodes") {
        for node in nodes.iter_mut() {
            if let Some(obj) = node.as_object_mut() {
                if obj
                    .get("children")
                    .and_then(Value::as_array)
                    .map(|a| a.is_empty())
                    .unwrap_or(false)
                {
                    obj.remove("children");
                }
            }
        }
    }
}

/// `stripAsset` (L589): delete `asset.profile` and `asset.premultipliedAlpha`.
fn strip_asset(gltf: &mut Value) {
    if let Some(asset) = gltf.get_mut("asset").and_then(Value::as_object_mut) {
        asset.remove("profile");
        asset.remove("premultipliedAlpha");
    }
}

/// `requireKnownExtensions` (L595–612): promote the known legacy extensions from
/// `extensionsUsed` to `extensionsRequired`.
fn require_known_extensions(gltf: &mut Value) {
    const KNOWN: [&str; 3] = ["CESIUM_RTC", "KHR_materials_common", "WEB3D_quantized_attributes"];
    let used: Vec<String> = gltf
        .get("extensionsUsed")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .filter(|e| KNOWN.contains(e))
                .map(|e| e.to_string())
                .collect()
        })
        .unwrap_or_default();
    for ext in used {
        add_extensions_required_local(gltf, &ext);
    }
}

/// `removeBufferType` (L614): delete `buffer.type`.
fn remove_buffer_type(gltf: &mut Value) {
    for_each_top_level_mut_named(gltf, "buffers", &mut |buffer| {
        if let Some(obj) = buffer.as_object_mut() {
            obj.remove("type");
        }
    });
}

/// `removeTextureProperties` (L620): delete `format` / `internalFormat` /
/// `target` / `type` from each texture.
fn remove_texture_properties(gltf: &mut Value) {
    for_each_top_level_mut_named(gltf, "textures", &mut |texture| {
        if let Some(obj) = texture.as_object_mut() {
            for key in ["format", "internalFormat", "target", "type"] {
                obj.remove(key);
            }
        }
    });
}

/// `requireAttributeSetIndex` (L629): `TEXCOORD` → `TEXCOORD_0`, `COLOR` →
/// `COLOR_0` on mesh primitive attributes and technique parameter semantics.
fn require_attribute_set_index(gltf: &mut Value) {
    if let Some(Value::Array(meshes)) = gltf.get_mut("meshes") {
        for mesh in meshes.iter_mut() {
            if let Some(prims) = mesh.get_mut("primitives").and_then(Value::as_array_mut) {
                for prim in prims.iter_mut() {
                    let Some(attrs) = prim.get_mut("attributes").and_then(Value::as_object_mut) else {
                        continue;
                    };
                    // Insert renamed keys, then delete the un-suffixed originals.
                    let renamed_semantics: Vec<(String, Value)> = attrs
                        .iter()
                        .filter(|(sem, _)| matches!(sem.as_str(), "TEXCOORD" | "COLOR"))
                        .map(|(sem, acc)| {
                            let new_sem = if sem == "TEXCOORD" { "TEXCOORD_0" } else { "COLOR_0" };
                            (new_sem.to_string(), acc.clone())
                        })
                        .collect();
                    for (sem, acc) in renamed_semantics {
                        attrs.insert(sem, acc);
                    }
                    attrs.remove("TEXCOORD");
                    attrs.remove("COLOR");
                }
            }
        }
    }
    for_each_technique_mut(gltf, &mut |technique| {
        if let Some(params) = technique.get_mut("parameters").and_then(Value::as_object_mut) {
            for (_name, param) in params.iter_mut() {
                if let Some(obj) = param.as_object_mut() {
                    let sem = obj.get("semantic").and_then(Value::as_str).map(String::from);
                    if let Some(sem) = sem {
                        let new = match sem.as_str() {
                            "TEXCOORD" => Some("TEXCOORD_0"),
                            "COLOR" => Some("COLOR_0"),
                            _ => None,
                        };
                        if let Some(new) = new {
                            obj.insert("semantic".to_string(), json!(new));
                        }
                    }
                }
            }
        }
    });
}

/// `underscoreApplicationSpecificSemantics` (L660–721).
fn underscore_application_specific_semantics(gltf: &mut Value) {
    const KNOWN: [&str; 3] = ["POSITION", "NORMAL", "TANGENT"];
    // strippedSemantic -> indexed replacement prefix
    let indexed = |s: &str| -> Option<&'static str> {
        match s {
            "COLOR" => Some("COLOR"),
            "JOINT" | "JOINTS" => Some("JOINTS"),
            "TEXCOORD" => Some("TEXCOORD"),
            "WEIGHT" | "WEIGHTS" => Some("WEIGHTS"),
            _ => None,
        }
    };
    let mut mapped_semantics: HashMap<String, String> = HashMap::new();

    if let Some(Value::Array(meshes)) = gltf.get_mut("meshes") {
        for mesh in meshes.iter_mut() {
            if let Some(prims) = mesh.get_mut("primitives").and_then(Value::as_array_mut) {
                for prim in prims.iter_mut() {
                    let Some(attrs) = prim.get_mut("attributes").and_then(Value::as_object_mut) else {
                        continue;
                    };
                    // Compute the semantic remapping from the current attribute set.
                    let mut local_renames: Vec<(String, String)> = Vec::new();
                    for sem in attrs.keys() {
                        if sem.starts_with('_') {
                            continue;
                        }
                        // JS: semantic.search(/_[0-9]+/g) -> first "_<digits>".
                        let stripped;
                        let suffix;
                        match find_indexed_suffix(sem) {
                            Some(pos) => {
                                stripped = sem[..pos].to_string();
                                suffix = sem[pos..].to_string();
                            }
                            None => {
                                stripped = sem.clone();
                                suffix = "_0".to_string();
                            }
                        }
                        let new_semantic;
                        if let Some(idx_sem) = indexed(&stripped) {
                            new_semantic = format!("{idx_sem}{suffix}");
                            mapped_semantics.insert(sem.clone(), new_semantic.clone());
                        } else if !KNOWN.contains(&stripped.as_str()) {
                            new_semantic = format!("_{sem}");
                            mapped_semantics.insert(sem.clone(), new_semantic.clone());
                        } else {
                            continue;
                        }
                        local_renames.push((sem.clone(), new_semantic));
                    }
                    for (old, new) in local_renames {
                        if let Some(acc) = attrs.remove(&old) {
                            attrs.insert(new, acc);
                        }
                    }
                }
            }
        }
    }

    for_each_technique_mut(gltf, &mut |technique| {
        if let Some(params) = technique.get_mut("parameters").and_then(Value::as_object_mut) {
            for (_name, param) in params.iter_mut() {
                if let Some(obj) = param.as_object_mut() {
                    let sem = obj.get("semantic").and_then(Value::as_str).map(String::from);
                    if let Some(sem) = sem {
                        if let Some(mapped) = mapped_semantics.get(&sem) {
                            obj.insert("semantic".to_string(), json!(mapped));
                        }
                    }
                }
            }
        }
    });
}

/// Finds the start of the first `_<digits>` suffix in `semantic`
/// (JS `semantic.search(/_[0-9]+/g)`), returning `None` when absent.
fn find_indexed_suffix(semantic: &str) -> Option<usize> {
    let bytes = semantic.as_bytes();
    (0..bytes.len()).find(|&i| {
        bytes[i] == b'_' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit()
    })
}

/// `clampCameraParameters` (L723): drop a zero `aspectRatio`, coerce a zero
/// `yfov` to `1.0`.
fn clamp_camera_parameters(gltf: &mut Value) {
    for_each_top_level_mut_named(gltf, "cameras", &mut |camera| {
        let Some(perspective) = camera.get_mut("perspective").and_then(Value::as_object_mut) else {
            return;
        };
        if perspective
            .get("aspectRatio")
            .and_then(Value::as_f64)
            .map(|a| a == 0.0)
            .unwrap_or(false)
        {
            perspective.remove("aspectRatio");
        }
        if perspective
            .get("yfov")
            .and_then(Value::as_f64)
            .map(|y| y == 0.0)
            .unwrap_or(false)
        {
            perspective.insert("yfov".to_string(), json!(1.0));
        }
    });
}

/// `requireByteLength` (L745): `buffer.byteLength` from the decoded source
/// (when `buffers` is supplied) and `bufferView.byteLength = max(existing,
/// accessor.byteOffset + count * stride)`.
///
/// The buffer-level half needs the decoded buffers and is delegated to
/// [`crate::gltf_binary_stage::require_byte_length_buffers`]; with `None`
/// (JSON-only path) it is skipped, preserving M9.1 behaviour byte-for-byte.
fn require_byte_length(gltf: &mut Value, buffers: Option<&Vec<Vec<u8>>>) {
    if let Some(buffers) = buffers {
        crate::gltf_binary_stage::require_byte_length_buffers(gltf, buffers);
    }
    let mut bv_end: HashMap<u64, u64> = HashMap::new();
    if let Some(Value::Array(accessors)) = gltf.get("accessors") {
        for acc in accessors {
            if let Some(bv) = acc.get("bufferView").and_then(Value::as_u64) {
                let stride = compute_accessor_byte_stride(gltf, acc) as u64;
                let byte_offset = acc.get("byteOffset").and_then(Value::as_u64).unwrap_or(0);
                let count = acc.get("count").and_then(Value::as_u64).unwrap_or(0);
                let end = byte_offset + count * stride;
                let entry = bv_end.entry(bv).or_insert(0);
                if end > *entry {
                    *entry = end;
                }
            }
        }
    }
    if let Some(Value::Array(bvs)) = gltf.get_mut("bufferViews") {
        for (i, bv) in bvs.iter_mut().enumerate() {
            if let Some(&end) = bv_end.get(&(i as u64)) {
                if let Some(obj) = bv.as_object_mut() {
                    let existing = obj.get("byteLength").and_then(Value::as_u64).unwrap_or(0);
                    obj.insert("byteLength".to_string(), json!(existing.max(end)));
                }
            }
        }
    }
}

/// `computeAccessorByteStride` (L739): `accessor.byteStride` when present and
/// non-zero, else the tight stride from the buffer view / component layout.
fn compute_accessor_byte_stride(gltf: &Value, accessor: &Value) -> usize {
    if let Some(bs) = accessor.get("byteStride").and_then(Value::as_u64) {
        if bs != 0 {
            return bs as usize;
        }
    }
    get_accessor_byte_stride(gltf, accessor)
}

/// `moveByteStrideToBufferView` (L766): move `accessor.byteStride` onto the
/// buffer view, splitting a buffer view when its accessors use differing
/// strides.
///
/// DEVIATION: upstream clones the buffer view for *every* run and relies on
/// `removeUnusedElements` (deferred) to drop the now-dead original. To avoid
/// that dependency the first run mutates the original buffer view in place and
/// only subsequent (differing-stride) runs append new buffer views. For the
/// common single-stride case the output matches upstream + `removeUnusedElements`
/// exactly (accessor keeps `bufferView = <original index>`).
fn move_byte_stride_to_buffer_view(gltf: &mut Value) {
    // Accessors referenced by mesh primitive attributes / morph targets.
    let mut vertex_accessors: HashSet<u64> = HashSet::new();
    if let Some(Value::Array(meshes)) = gltf.get("meshes") {
        for mesh in meshes {
            if let Some(prims) = mesh.get("primitives").and_then(Value::as_array) {
                for prim in prims {
                    collect_attribute_accessors(prim.get("attributes"), &mut vertex_accessors);
                    if let Some(targets) = prim.get("targets").and_then(Value::as_array) {
                        for target in targets {
                            collect_attribute_accessors(Some(target), &mut vertex_accessors);
                        }
                    }
                }
            }
        }
    }

    let Some(root) = gltf.as_object_mut() else {
        return;
    };
    let mut accessors = take_array(root, "accessors");
    let mut buffer_views = take_array(root, "bufferViews");
    let original_buffer_views = buffer_views.clone();

    // accessor index -> bufferView index; bufferView -> vertex-attribute flag.
    let mut bv_has_vertex_attrs: HashSet<u64> = HashSet::new();
    let mut bv_to_accessors: HashMap<u64, Vec<usize>> = HashMap::new();
    for (i, acc) in accessors.iter().enumerate() {
        if let Some(bv) = acc.get("bufferView").and_then(Value::as_u64) {
            if vertex_accessors.contains(&(i as u64)) {
                bv_has_vertex_attrs.insert(bv);
            }
            bv_to_accessors.entry(bv).or_default().push(i);
        }
    }

    // Process buffer views in ascending index order to match JS object key
    // iteration (integer-like keys iterate numerically) and keep the appended
    // buffer-view indices deterministic.
    let mut bv_ids: Vec<u64> = bv_to_accessors.keys().copied().collect();
    bv_ids.sort_unstable();
    for bv_id in bv_ids {
        let Some(mut acc_indices) = bv_to_accessors.remove(&bv_id) else {
            continue;
        };
        let Some(bv_id_usize) = usize::try_from(bv_id).ok() else {
            continue;
        };
        if bv_id_usize >= buffer_views.len() {
            continue;
        }
        acc_indices.sort_by_key(|&ai| {
            accessors[ai].get("byteOffset").and_then(Value::as_u64).unwrap_or(0)
        });
        let original_byte_offset = original_buffer_views[bv_id_usize]
            .get("byteOffset")
            .and_then(Value::as_u64)
            .unwrap_or(0);

        let mut current_byte_offset: u64 = 0;
        let mut current_index: usize = 0;
        let mut first_run = true;
        let n = acc_indices.len();
        for i in 0..n {
            let stride = accessor_stride(&accessors[acc_indices[i]], &original_buffer_views) as u64;
            let byte_offset = accessors[acc_indices[i]]
                .get("byteOffset")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let count = accessors[acc_indices[i]]
                .get("count")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let byte_length = count * stride;
            // delete accessor.byteStride (L806)
            if let Some(obj) = accessors[acc_indices[i]].as_object_mut() {
                obj.remove("byteStride");
            }
            let has_next = i + 1 < n;
            let next_stride = if has_next {
                Some(accessor_stride(&accessors[acc_indices[i + 1]], &original_buffer_views) as u64)
            } else {
                None
            };
            if Some(stride) != next_stride {
                let new_byte_offset = original_byte_offset + current_byte_offset;
                let new_byte_length = byte_offset + byte_length - current_byte_offset;
                let target_bv = if first_run {
                    if let Some(obj) = buffer_views[bv_id_usize].as_object_mut() {
                        if bv_has_vertex_attrs.contains(&bv_id) {
                            obj.insert("byteStride".to_string(), json!(stride));
                        }
                        obj.insert("byteOffset".to_string(), json!(new_byte_offset));
                        obj.insert("byteLength".to_string(), json!(new_byte_length));
                    }
                    first_run = false;
                    bv_id
                } else {
                    let mut new_bv = original_buffer_views[bv_id_usize].clone();
                    if let Some(obj) = new_bv.as_object_mut() {
                        if bv_has_vertex_attrs.contains(&bv_id) {
                            obj.insert("byteStride".to_string(), json!(stride));
                        }
                        obj.insert("byteOffset".to_string(), json!(new_byte_offset));
                        obj.insert("byteLength".to_string(), json!(new_byte_length));
                    }
                    buffer_views.push(new_bv);
                    (buffer_views.len() - 1) as u64
                };
                for ai in acc_indices.iter().take(i + 1).skip(current_index) {
                    let ai = *ai;
                    if let Some(obj) = accessors[ai].as_object_mut() {
                        obj.insert("bufferView".to_string(), json!(target_bv));
                        let off = obj.get("byteOffset").and_then(Value::as_u64).unwrap_or(0);
                        obj.insert("byteOffset".to_string(), json!(off - current_byte_offset));
                    }
                }
                current_byte_offset = if has_next {
                    accessors[acc_indices[i + 1]]
                        .get("byteOffset")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                } else {
                    0
                };
                current_index = i + 1;
            }
        }
    }

    root.insert("accessors".to_string(), Value::Array(accessors));
    root.insert("bufferViews".to_string(), Value::Array(buffer_views));
}

/// Collects accessor indices from a primitive `attributes` (or morph `target`)
/// object.
fn collect_attribute_accessors(attrs: Option<&Value>, out: &mut HashSet<u64>) {
    if let Some(obj) = attrs.and_then(Value::as_object) {
        for (_sem, acc) in obj {
            if let Some(i) = acc.as_u64() {
                out.insert(i);
            }
        }
    }
}

/// Stride of an accessor against a fixed buffer-view snapshot (used while the
/// live buffer views are being mutated).
fn accessor_stride(accessor: &Value, buffer_views: &[Value]) -> usize {
    if let Some(bs) = accessor.get("byteStride").and_then(Value::as_u64) {
        if bs != 0 {
            return bs as usize;
        }
    }
    if let Some(bv_id) = accessor.get("bufferView").and_then(Value::as_u64) {
        if let Some(bv) = buffer_views.get(bv_id as usize) {
            if let Some(s) = bv.get("byteStride").and_then(Value::as_u64) {
                if s > 0 {
                    return s as usize;
                }
            }
        }
    }
    let ct = accessor.get("componentType").and_then(Value::as_u64).unwrap_or(0);
    let ty = accessor.get("type").and_then(Value::as_str).unwrap_or("");
    component_size_in_bytes(ct) * number_of_components_for_type(ty)
}

// ----------------------------- shared helpers -----------------------------

/// Maps a string id through `mapping` to an index; non-strings (already-indexed
/// 2.0 values) pass through unchanged.
fn map_id(mapping: &HashMap<String, usize>, value: &Value) -> Value {
    match value.as_str() {
        Some(id) => match mapping.get(id) {
            Some(idx) => json!(*idx),
            None => Value::Null,
        },
        None => value.clone(),
    }
}

/// Remaps `gltf[collection][*][field]` (string id → index) for an array-form
/// collection.
fn remap_field(root: &mut Map<String, Value>, collection: &str, field: &str, mapping: &HashMap<String, usize>) {
    if let Some(Value::Array(list)) = root.get_mut(collection) {
        for item in list.iter_mut() {
            remap_field_in(item, field, mapping);
        }
    }
}

/// Remaps `value[field]` (string id → index) within a single JSON value.
fn remap_field_in(value: &mut Value, field: &str, mapping: &HashMap<String, usize>) {
    if let Some(obj) = value.as_object_mut() {
        remap_field_in_obj(obj, field, mapping);
    }
}

/// [`remap_field_in`] for an already-unwrapped object.
fn remap_field_in_obj(obj: &mut Map<String, Value>, field: &str, mapping: &HashMap<String, usize>) {
    if let Some(current) = obj.get(field).cloned() {
        if current.is_string() {
            obj.insert(field.to_string(), map_id(mapping, &current));
        }
    }
}

/// `defined()` semantics (Cesium): present and not null.
fn is_defined(obj: &Map<String, Value>, key: &str) -> bool {
    obj.get(key).map(|v| !v.is_null()).unwrap_or(false)
}

/// Compares a JSON number array against an `f64` slice (exact equality).
fn number_array_eq(arr: &[Value], expected: &[f64]) -> bool {
    if arr.len() != expected.len() {
        return false;
    }
    arr.iter()
        .zip(expected)
        .all(|(v, e)| v.as_f64().map(|x| x == *e).unwrap_or(false))
}

/// True for a 16-element column-major identity matrix.
fn is_identity_matrix(m: &[Value]) -> bool {
    if m.len() != 16 {
        return false;
    }
    let identity = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    number_array_eq(m, &identity)
}

/// Removes `root[key]` and returns it as a `Vec<Value>` (empty when absent or
/// not an array).
fn take_array(root: &mut Map<String, Value>, key: &str) -> Vec<Value> {
    match root.remove(key) {
        Some(Value::Array(a)) => a,
        Some(other) => {
            // Put back non-array values untouched.
            root.insert(key.to_string(), other);
            Vec::new()
        }
        None => Vec::new(),
    }
}

/// Iterates `animation.samplers` mutably whether it is an object (glTF 1.0,
/// keyed by sampler id) or an array (post-`objectsToArrays`).
fn for_each_sampler_mut(samplers: &mut Value, mut f: impl FnMut(&mut Value)) {
    match samplers {
        Value::Array(a) => {
            for s in a.iter_mut() {
                f(s);
            }
        }
        Value::Object(o) => {
            for s in o.values_mut() {
                f(s);
            }
        }
        _ => {}
    }
}

/// Local `addExtensionsRequired` (avoids a cyclic import of the util crate
/// helper while keeping behaviour identical).
fn add_extensions_required_local(gltf: &mut Value, extension: &str) {
    let Some(root) = gltf.as_object_mut() else {
        return;
    };
    let required = root
        .entry("extensionsRequired")
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Some(arr) = required.as_array_mut() {
        let value = Value::String(extension.to_string());
        if !arr.contains(&value) {
            arr.push(value);
        }
    }
    let used = root
        .entry("extensionsUsed")
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Some(arr) = used.as_array_mut() {
        let value = Value::String(extension.to_string());
        if !arr.contains(&value) {
            arr.push(value);
        }
    }
}

/// Mutable traversal of a top-level collection (array or object form).
fn for_each_top_level_mut_named(gltf: &mut Value, name: &str, f: &mut impl FnMut(&mut Value)) {
    if let Some(coll) = gltf.get_mut(name) {
        match coll {
            Value::Array(arr) => {
                for item in arr.iter_mut() {
                    f(item);
                }
            }
            Value::Object(obj) => {
                for (_k, item) in obj.iter_mut() {
                    f(item);
                }
            }
            _ => {}
        }
    }
}

/// Mutable traversal of `materials` (array or object form).
fn for_each_material_mut(gltf: &mut Value, f: &mut impl FnMut(&mut Value)) {
    for_each_top_level_mut_named(gltf, "materials", f);
}

/// Mutable traversal of techniques, preferring `KHR_techniques_webgl.techniques`
/// when that extension is used (mirrors `ForEach.technique`).
fn for_each_technique_mut(gltf: &mut Value, f: &mut impl FnMut(&mut Value)) {
    let uses_ext = gltf
        .get("extensionsUsed")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .any(|x| x.as_str() == Some("KHR_techniques_webgl"))
        })
        .unwrap_or(false);
    if uses_ext {
        if let Some(arr) = gltf
            .pointer_mut("/extensions/KHR_techniques_webgl/techniques")
            .and_then(Value::as_array_mut)
        {
            for item in arr.iter_mut() {
                f(item);
            }
        }
        return;
    }
    for_each_top_level_mut_named(gltf, "techniques", f);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> UpgradeOptions {
        UpgradeOptions::default()
    }

    fn assert_close(a: f64, b: f64, ctx: &str) {
        assert!((a - b).abs() < 1e-12, "{ctx}: {a} != {b}");
    }

    /// The `gltf1` fixture from `Specs/Scene/GltfJsonLoaderSpec.js` (L17-132):
    /// a glTF 1.0 asset with a technique-based red material.
    fn gltf1_fixture() -> Value {
        json!({
            "asset": { "version": "1.0" },
            "buffers": { "buffer": { "uri": "external.bin" } },
            "bufferViews": { "bufferView": { "buffer": "buffer", "byteOffset": 0 } },
            "accessors": {
                "accessor": {
                    "bufferView": "bufferView", "byteOffset": 0,
                    "componentType": 5126, "type": "VEC3", "count": 1
                }
            },
            "meshes": {
                "mesh": { "primitives": [ { "attributes": { "POSITION": "accessor" }, "material": "red" } ] }
            },
            "nodes": { "node": { "meshes": ["mesh"] } },
            "scene": "scene",
            "scenes": { "scene": { "nodes": ["node"] } },
            "shaders": {
                "Box0FS": { "type": 35632, "uri": "data:text/plain;base64," },
                "Box0VS": { "type": 35633, "uri": "data:text/plain;base64," }
            },
            "programs": {
                "program_0": { "attributes": ["a_position"], "fragmentShader": "Box0FS", "vertexShader": "Box0VS" }
            },
            "materials": {
                "red": {
                    "technique": "technique0",
                    "values": { "diffuse": [0.8, 0, 0, 1], "shininess": 256, "specular": [0.2, 0.2, 0.2, 1] }
                }
            },
            "techniques": {
                "technique0": {
                    "attributes": { "a_position": "position" },
                    "parameters": {
                        "diffuse": { "type": 35666 },
                        "modelViewMatrix": { "semantic": "MODELVIEW", "type": 35676 },
                        "position": { "semantic": "POSITION", "type": 35665 },
                        "projectionMatrix": { "semantic": "PROJECTION", "type": 35676 },
                        "shininess": { "type": 5126 },
                        "specular": { "type": 35666 }
                    },
                    "program": "program_0",
                    "states": { "enable": [2929, 2884] },
                    "uniforms": {
                        "u_diffuse": "diffuse", "u_modelViewMatrix": "modelViewMatrix",
                        "u_projectionMatrix": "projectionMatrix", "u_shininess": "shininess",
                        "u_specular": "specular"
                    }
                }
            }
        })
    }

    /// The `gltf1MaterialsCommon` fixture (Spec L134-199).
    fn gltf1_materials_common_fixture() -> Value {
        json!({
            "asset": { "version": "1.0" },
            "materials": {
                "red": {
                    "extensions": {
                        "KHR_materials_common": {
                            "doubleSided": false, "jointCount": 0, "technique": "PHONG",
                            "transparent": false,
                            "values": { "diffuse": [0.8, 0, 0, 1], "shininess": 256, "specular": [0.2, 0.2, 0.2, 1] }
                        }
                    }
                }
            },
            "extensionsUsed": ["KHR_materials_common"]
        })
    }

    /// The plain `gltf2` fixture (Spec L201-269) — already 2.0.
    fn gltf2_fixture() -> Value {
        json!({
            "asset": { "version": "2.0" },
            "buffers": [ { "name": "buffer", "uri": "external.bin", "byteLength": 12 } ],
            "bufferViews": [ { "name": "bufferView", "buffer": 0, "byteOffset": 0, "byteLength": 12 } ],
            "accessors": [ {
                "name": "accessor", "bufferView": 0, "byteOffset": 0,
                "componentType": 5126, "type": "VEC3", "count": 1, "min": [0, 0, 0], "max": [0, 0, 0]
            } ],
            "materials": [ {
                "name": "red",
                "pbrMetallicRoughness": { "roughnessFactor": 1, "metallicFactor": 0, "baseColorFactor": [0.6038273388553378, 0, 0, 1] }
            } ],
            "meshes": [ { "name": "mesh", "primitives": [ { "attributes": { "POSITION": 0 }, "material": 0 } ] } ],
            "nodes": [ { "name": "node", "mesh": 0 } ],
            "scene": 0,
            "scenes": [ { "name": "scene", "nodes": [0] } ]
        })
    }

    #[test]
    fn detect_version_variants() {
        assert_eq!(detect_version(&json!({"asset":{"version":"1.0"}})), GltfVersion::V10);
        assert_eq!(detect_version(&json!({"asset":{"version":"2.0"}})), GltfVersion::V20);
        assert_eq!(detect_version(&json!({"version":"0.8"})), GltfVersion::V08);
        assert_eq!(detect_version(&json!({"version":0.8})), GltfVersion::V08);
        // Absent version defaults to 1.0 (updateVersion.js L50/L61).
        assert_eq!(detect_version(&json!({})), GltfVersion::V10);
        // Truncation to three characters (L57).
        assert_eq!(detect_version(&json!({"asset":{"version":"2.0.0"}})), GltfVersion::V20);
        assert_eq!(detect_version(&json!({"asset":{"version":"1.0.1"}})), GltfVersion::V10);
        // Unknown -> default 1.0 (L61).
        assert_eq!(detect_version(&json!({"asset":{"version":"3.5"}})), GltfVersion::V10);
        assert_eq!(GltfVersion::V20.as_str(), "2.0");
    }

    #[test]
    fn version_08_upgrade_is_deferred() {
        let mut gltf = json!({"version":"0.8"});
        let err = update_version(&mut gltf, &opts()).unwrap_err();
        assert_eq!(err, GltfUpgradeError::UnsupportedVersion08);

        // targetVersion 0.8 stops before the deferred step -> Ok, unchanged version.
        let mut gltf = json!({"version":"0.8"});
        let o = UpgradeOptions { target_version: Some("0.8".to_string()), ..Default::default() };
        assert!(update_version(&mut gltf, &o).is_ok());
        assert_eq!(gltf.get("version").and_then(Value::as_str), Some("0.8"));
    }

    #[test]
    fn non_object_root_errors() {
        let mut gltf = json!([1, 2, 3]);
        assert_eq!(
            update_version(&mut gltf, &opts()).unwrap_err(),
            GltfUpgradeError::NotAnObject
        );
    }

    /// A plain 2.0 asset passes through unchanged (update loop is skipped and
    /// the PBR converters are no-ops without the legacy extensions).
    #[test]
    fn gltf2_passthrough_is_unchanged() {
        let original = gltf2_fixture();
        let mut gltf = original.clone();
        update_version(&mut gltf, &opts()).unwrap();
        assert_eq!(gltf, original, "2.0 passthrough must not mutate the asset");
    }

    /// Full 1.0 -> 2.0 upgrade of the `gltf1` fixture: objectsToArrays,
    /// reference backfill, byteStride move, technique -> PBR material.
    #[test]
    fn gltf1_full_upgrade() {
        let mut gltf = gltf1_fixture();
        update_version(&mut gltf, &opts()).unwrap();

        // Version bumped.
        assert_eq!(gltf.pointer("/asset/version").and_then(Value::as_str), Some("2.0"));

        // objectsToArrays: collections are arrays; string ids became indices.
        assert!(gltf.get("accessors").and_then(Value::as_array).is_some());
        assert_eq!(gltf.pointer("/accessors/0/bufferView").and_then(Value::as_u64), Some(0));
        assert_eq!(gltf.pointer("/bufferViews/0/buffer").and_then(Value::as_u64), Some(0));
        assert_eq!(gltf.pointer("/meshes/0/primitives/0/attributes/POSITION").and_then(Value::as_u64), Some(0));
        assert_eq!(gltf.pointer("/meshes/0/primitives/0/material").and_then(Value::as_u64), Some(0));
        // node.meshes -> node.mesh; scene/scene.nodes remapped.
        assert_eq!(gltf.pointer("/nodes/0/mesh").and_then(Value::as_u64), Some(0));
        assert!(gltf.pointer("/nodes/0/meshes").is_none());
        assert_eq!(gltf.pointer("/scene").and_then(Value::as_u64), Some(0));
        assert_eq!(gltf.pointer("/scenes/0/nodes/0").and_then(Value::as_u64), Some(0));
        // name assigned from the object key.
        assert_eq!(gltf.pointer("/accessors/0/name").and_then(Value::as_str), Some("accessor"));

        // requireByteLength + moveByteStrideToBufferView (JSON-only parts).
        // accessor: FLOAT VEC3 count 1 -> tight stride 12.
        assert_eq!(gltf.pointer("/bufferViews/0/byteLength").and_then(Value::as_u64), Some(12));
        assert_eq!(gltf.pointer("/bufferViews/0/byteStride").and_then(Value::as_u64), Some(12));
        // componentType preserved through the upgrade.
        assert_eq!(gltf.pointer("/accessors/0/componentType").and_then(Value::as_u64), Some(5126));

        // technique -> PBR: material has pbrMetallicRoughness, no technique/values.
        assert_eq!(gltf.pointer("/materials/0/name").and_then(Value::as_str), Some("red"));
        assert!(gltf.pointer("/materials/0/technique").is_none());
        assert!(gltf.pointer("/materials/0/values").is_none());
        let roughness = gltf.pointer("/materials/0/pbrMetallicRoughness/roughnessFactor").and_then(Value::as_f64);
        let metallic = gltf.pointer("/materials/0/pbrMetallicRoughness/metallicFactor").and_then(Value::as_f64);
        assert_eq!(roughness, Some(1.0));
        assert_eq!(metallic, Some(0.0));
        // srgbToLinear(0.8) == 0.6038273388553378 (Spec gltf2 material).
        let bcf = gltf.pointer("/materials/0/pbrMetallicRoughness/baseColorFactor").and_then(Value::as_array).cloned();
        let bcf = bcf.expect("baseColorFactor present");
        assert_close(bcf[0].as_f64().unwrap(), 0.6038273388553378, "baseColorFactor.r");
        assert_close(bcf[1].as_f64().unwrap(), 0.0, "baseColorFactor.g");
        assert_close(bcf[2].as_f64().unwrap(), 0.0, "baseColorFactor.b");
        assert_close(bcf[3].as_f64().unwrap(), 1.0, "baseColorFactor.a");

        // Legacy technique collections + the transitional extension are gone.
        assert!(gltf.get("techniques").is_none());
        assert!(gltf.get("programs").is_none());
        assert!(gltf.get("shaders").is_none());
        assert!(gltf.pointer("/extensions/KHR_techniques_webgl").is_none());
        assert!(!crate::gltf_upgrade_util::uses_extension(&gltf, "KHR_techniques_webgl"));
    }

    /// `KHR_materials_common` (PHONG) -> PBR base color + alphaMode/doubleSided.
    #[test]
    fn gltf1_materials_common_to_pbr() {
        let mut gltf = gltf1_materials_common_fixture();
        update_version(&mut gltf, &opts()).unwrap();

        assert_eq!(gltf.pointer("/asset/version").and_then(Value::as_str), Some("2.0"));
        assert_eq!(gltf.pointer("/materials/0/name").and_then(Value::as_str), Some("red"));
        // diffuse [0.8,0,0,1] -> linear base color.
        let bcf = gltf.pointer("/materials/0/pbrMetallicRoughness/baseColorFactor").and_then(Value::as_array).cloned();
        let bcf = bcf.expect("baseColorFactor present");
        assert_close(bcf[0].as_f64().unwrap(), 0.6038273388553378, "mc.baseColorFactor.r");
        assert_eq!(gltf.pointer("/materials/0/alphaMode").and_then(Value::as_str), Some("OPAQUE"));
        assert_eq!(gltf.pointer("/materials/0/doubleSided").and_then(Value::as_bool), Some(false));
        // Extension removed from the material and from extensionsUsed/Required.
        assert!(gltf.pointer("/materials/0/extensions/KHR_materials_common").is_none());
        assert!(!crate::gltf_upgrade_util::uses_extension(&gltf, "KHR_materials_common"));
    }

    /// `removeAnimationSamplersIndirection` + objectsToArrays for animations.
    #[test]
    fn animation_samplers_deindirection() {
        let mut gltf = json!({
            "asset": { "version": "1.0" },
            "accessors": {
                "accTime": { "componentType": 5126, "type": "SCALAR", "count": 2 },
                "accValue": { "componentType": 5126, "type": "VEC3", "count": 2 }
            },
            "nodes": { "node0": { "translation": [1.0, 2.0, 3.0] } },
            "animations": {
                "anim": {
                    "parameters": { "time": "accTime", "value": "accValue" },
                    "samplers": { "sampler0": { "input": "time", "output": "value", "interpolation": "LINEAR", "name": "s0" } },
                    "channels": [ { "sampler": "sampler0", "target": { "id": "node0", "path": "translation" } } ]
                }
            }
        });
        update_version(&mut gltf, &opts()).unwrap();

        // parameters indirection removed; samplers reference accessors by index.
        assert!(gltf.pointer("/animations/0/parameters").is_none());
        assert!(gltf.pointer("/animations/0/samplers").and_then(Value::as_array).is_some());
        // accTime -> 0, accValue -> 1 (alphabetical key order).
        assert_eq!(gltf.pointer("/animations/0/samplers/0/input").and_then(Value::as_u64), Some(0));
        assert_eq!(gltf.pointer("/animations/0/samplers/0/output").and_then(Value::as_u64), Some(1));
        // sampler name stripped.
        assert!(gltf.pointer("/animations/0/samplers/0/name").is_none());
        // channel.sampler -> index; target.id -> target.node.
        assert_eq!(gltf.pointer("/animations/0/channels/0/sampler").and_then(Value::as_u64), Some(0));
        assert_eq!(gltf.pointer("/animations/0/channels/0/target/node").and_then(Value::as_u64), Some(0));
        assert!(gltf.pointer("/animations/0/channels/0/target/id").is_none());
    }

    /// accessor.byteStride (1.0) moves onto the bufferView (2.0); componentType
    /// is preserved and bufferView.byteLength is derived from the accessor span.
    #[test]
    fn accessor_byte_stride_moves_to_buffer_view() {
        let mut gltf = json!({
            "asset": { "version": "1.0" },
            "buffers": { "b": { "uri": "x.bin" } },
            "bufferViews": { "bv": { "buffer": "b", "byteOffset": 0 } },
            "accessors": { "a": { "bufferView": "bv", "byteOffset": 0, "byteStride": 24, "componentType": 5126, "type": "VEC3", "count": 2 } },
            "meshes": { "m": { "primitives": [ { "attributes": { "POSITION": "a" } } ] } },
            "nodes": { "n": { "meshes": ["m"] } },
            "scenes": { "s": { "nodes": ["n"] } },
            "scene": "s"
        });
        update_version(&mut gltf, &opts()).unwrap();

        // byteStride removed from the accessor ...
        assert!(gltf.pointer("/accessors/0/byteStride").is_none());
        // ... and moved onto the bufferView.
        assert_eq!(gltf.pointer("/bufferViews/0/byteStride").and_then(Value::as_u64), Some(24));
        // byteLength = byteOffset(0) + count(2) * stride(24) = 48.
        assert_eq!(gltf.pointer("/bufferViews/0/byteLength").and_then(Value::as_u64), Some(48));
        // componentType preserved; bufferView remapped to index.
        assert_eq!(gltf.pointer("/accessors/0/componentType").and_then(Value::as_u64), Some(5126));
        assert_eq!(gltf.pointer("/accessors/0/bufferView").and_then(Value::as_u64), Some(0));
    }

    /// `keepLegacyExtensions` preserves the technique extension instead of
    /// converting it to PBR.
    #[test]
    fn keep_legacy_extensions_preserves_techniques() {
        let mut gltf = gltf1_fixture();
        let o = UpgradeOptions { keep_legacy_extensions: true, ..Default::default() };
        update_version(&mut gltf, &o).unwrap();
        // Structural upgrade still ran (version 2.0, arrays) ...
        assert_eq!(gltf.pointer("/asset/version").and_then(Value::as_str), Some("2.0"));
        // ... but the technique lives on in KHR_techniques_webgl, not PBR.
        assert!(gltf.pointer("/extensions/KHR_techniques_webgl/techniques/0").is_some());
        assert!(gltf.pointer("/materials/0/extensions/KHR_techniques_webgl").is_some());
        assert!(gltf.pointer("/materials/0/pbrMetallicRoughness").is_none());
    }

    /// `targetVersion = "1.0"` stops before the 1.0 -> 2.0 step.
    #[test]
    fn target_version_1_0_stops_upgrade() {
        let mut gltf = gltf1_fixture();
        let o = UpgradeOptions { target_version: Some("1.0".to_string()), ..Default::default() };
        update_version(&mut gltf, &o).unwrap();
        // Still 1.0; collections remain object-keyed (objectsToArrays not run).
        assert_eq!(gltf.pointer("/asset/version").and_then(Value::as_str), Some("1.0"));
        assert!(gltf.get("accessors").and_then(Value::as_object).is_some());
    }

    /// `requireAttributeSetIndex`: TEXCOORD/COLOR gain the `_0` set index.
    #[test]
    fn attribute_set_index_added() {
        let mut gltf = json!({
            "asset": { "version": "1.0" },
            "accessors": { "uv": { "componentType": 5126, "type": "VEC2", "count": 1 }, "col": { "componentType": 5126, "type": "VEC3", "count": 1 } },
            "meshes": { "m": { "primitives": [ { "attributes": { "TEXCOORD": "uv", "COLOR": "col" } } ] } },
            "nodes": { "n": { "meshes": ["m"] } },
            "scenes": { "s": { "nodes": ["n"] } },
            "scene": "s"
        });
        update_version(&mut gltf, &opts()).unwrap();
        let attrs = gltf.pointer("/meshes/0/primitives/0/attributes").and_then(Value::as_object).cloned();
        let attrs = attrs.expect("attributes");
        assert!(attrs.contains_key("TEXCOORD_0"));
        assert!(attrs.contains_key("COLOR_0"));
        assert!(!attrs.contains_key("TEXCOORD"));
        assert!(!attrs.contains_key("COLOR"));
    }
}
