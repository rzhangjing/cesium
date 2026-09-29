//! glTF 1.0 → 2.0 JSON 升级链。
//!
//! 镜像 CesiumJS `packages/engine/Source/Scene/GltfPipeline/updateVersion.js`
//!（版本升级真正的事实来源——`GltfLoader.js` 本身
//! 不包含升级逻辑）。入口点 [`update_version`] 复现了
//! `updateVersion`（L41），它：
//!
//! 1. 确保 `asset` / `asset.version`（默认为 `"1.0"`），
//! 2. 检测版本（[`detect_version`]，L44–63），
//! 3. 逐步应用升级函数链直到 `2.0`（或 `targetVersion`），
//! 4. 除非设置了 `keepLegacyExtensions`，否则将遗留的 technique /
//!    `KHR_materials_common` 材质模型转换为 PBR。
//!
//! 根据里程碑决定，仅实现了 `1.0 → 2.0` 这一步（[`gl_tf10to20`]，
//! 上游 L943）；`0.8 → 1.0`（`glTF08to10`）被推迟，
//! 上报为 [`GltfUpgradeError::UnsupportedVersion08`]。
//!
//! # 范围：JSON 变换 + 二进制阶段
//!
//! 升级作用于原始的 [`serde_json::Value`]（glTF 1.0 将其
//! 顶层集合存储为以对象为键的字典，基于数组的
//! 强类型 [`crate::gltf_model::GltfModel`] 无法反序列化）。仅 JSON 的
//! 入口点 [`update_version`] 运行结构变换；依赖于已解码二进制
//! buffer（`extras._pipeline.source`）的上游步骤
//! 位于 [`crate::gltf_binary_stage`]，仅通过
//! [`update_version_with_buffers`] 运行，后者将已解码的 buffer 贯穿
//! 传递经由 [`gl_tf10to20`]：
//!
//! * 从已解码源得到的 `buffer.byteLength`（`requireByteLength`，buffer 那一半），
//! * `requirePositionAccessorMinMax` / `requireAnimationAccessorMinMax` /
//!   `validatePresentAccessorMinMax`（均调用 `findAccessorMinMax`），
//! * `updateAccessorComponentTypes`（重新打包 JOINTS/WEIGHTS 数据），
//! * `removeUnusedElements`（由 `moveByteStrideToBufferView` 调用）。
//!
//! 未提供 buffer 时，本链与仅 JSON 的 M9.1 行为逐字节一致。
//!
//! # 偏差：集合排序
//!
//! 工作区锁定 `serde_json = "1"` 且**未**启用 `preserve_order`
//! 特性，因此 [`serde_json::Map`] 是一个按字母序遍历键的
//! `BTreeMap`。因此 `objectsToArrays` 按字母键序而非 JS 插入序分配
//! 数组索引。每个引用都通过同一个 `global_mapping` 重映射，
//! 因此升级后的资源保持内部一致且语义等价；对于多条目对象，
//! 仅已转换集合内元素的顺序可能不同。
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

/// 从根 `version` 或 `asset.version` 检测到的 glTF 资源版本。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GltfVersion {
    /// glTF 0.8（升级到 1.0 在此被推迟 / 不支持）。
    V08,
    /// glTF 1.0 — 由 [`gl_tf10to20`] 升级到 2.0。
    V10,
    /// glTF 2.0 — 直接透传（无结构升级）。
    V20,
    /// 无法确定；上游默认为 `1.0`。
    Unknown,
}

impl GltfVersion {
    /// 规范化的版本字符串（`"0.8"` / `"1.0"` / `"2.0"`）。
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

/// 升级链产生的错误。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GltfUpgradeError {
    /// 该资源是 glTF 0.8；仅实现了 `1.0 → 2.0`（Q5 决定）。
    #[error("glTF 0.8 -> 1.0 upgrade is deferred; only 1.0 -> 2.0 is supported")]
    UnsupportedVersion08,
    /// 根 JSON 值不是一个对象。
    #[error("glTF asset root is not a JSON object")]
    NotAnObject,
}

/// 镜像 `updateVersion(gltf, options)` 的选项（updateVersion.js L33–36）。
#[derive(Debug, Clone, Default)]
pub struct UpgradeOptions {
    /// 一旦达到此版本就停止升级（`options.targetVersion`）。
    pub target_version: Option<String>,
    /// 设置时，跳过遗留的 technique / `KHR_materials_common` → PBR
    /// 转换（`options.keepLegacyExtensions`）。
    pub keep_legacy_extensions: bool,
    /// 表示 base-color *纹理* 的 uniform 名称
    ///（`options.baseColorTextureNames`）；默认为
    /// [`DEFAULT_BASE_COLOR_TEXTURE_NAMES`]。
    pub base_color_texture_names: Option<Vec<String>>,
    /// 表示 base-color *因子* 的 uniform 名称
    ///（`options.baseColorFactorNames`）；默认为
    /// [`DEFAULT_BASE_COLOR_FACTOR_NAMES`]。
    pub base_color_factor_names: Option<Vec<String>>,
}

/// 检测 glTF 版本而不改变资源。
///
/// 镜像 `updateVersion` 中的版本解析（L44–63）：根级
/// `version`（glTF 0.8 约定）优先于 `asset.version`；
/// 未知值会被截断为三个字符，最终默认为 `"1.0"`。
#[must_use]
pub fn detect_version(gltf: &Value) -> GltfVersion {
    normalize_version(&version_string(gltf))
}

/// 读取原始版本字符串（根 `version`，否则 `asset.version`，否则
/// `"1.0"`），并容忍数值型的根 `version`（例如 `0.8`）。
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

/// `updateFunctions` 成员测试 + 截断回退（L54–63）。
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
                // 无法确定时默认为 1.0（L61）。
                _ => GltfVersion::V10,
            }
        }
    }
}

/// 就地将 `gltf` 升级到版本 2.0（或 `options.target_version`）。
///
/// 对 `updateVersion`（updateVersion.js L41–82）的忠实移植。对于一个普通的 2.0
/// 资源，这是直接透传：升级循环不会运行，且当遗留扩展缺失时
/// 末尾的 PBR 转换器为无操作，使资源
/// 保持不变。
///
/// 仅 JSON：依赖于二进制 buffer 的步骤（min/max、JOINTS/WEIGHTS
/// 重打包、无用元素移除）会被跳过。使用
/// [`update_version_with_buffers`] 针对已解码 buffer
/// 运行完整链。
///
/// # 错误
/// 若根不是一个对象则返回 [`GltfUpgradeError::NotAnObject`]，或
/// 若资源是 glTF 0.8 且目标版本需要（被推迟的）`0.8 → 1.0`
/// 步骤，则返回 [`GltfUpgradeError::UnsupportedVersion08`]。
pub fn update_version(gltf: &mut Value, options: &UpgradeOptions) -> Result<(), GltfUpgradeError> {
    update_version_impl(gltf, options, None)
}

/// 类似 [`update_version`]，但还针对已解码的 buffer 源运行
/// `glTF10to20` 中依赖于二进制 buffer 的步骤
///（`requirePositionAccessorMinMax` / `requireAnimationAccessorMinMax`
/// / `validatePresentAccessorMinMax` / `updateAccessorComponentTypes` /
/// `removeUnusedElements` / buffer 级的 `requireByteLength`）。
///
/// `buffers[i]` 必须是 `gltf.buffers[i]` 的已解码源（对于 GLB，
/// `buffers = vec![binary_chunk]`）。当 `updateAccessorComponentTypes`
/// 将 JOINTS/WEIGHTS 重打包到新 buffer 时，该 vec 可能增长。
///
/// # 错误
/// 与 [`update_version`] 相同。
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
    // 确保 asset + asset.version（L46–50）。
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
                // glTF08to10 被推迟（Q5：仅 1.0 -> 2.0）。
                return Err(GltfUpgradeError::UnsupportedVersion08);
            }
            GltfVersion::V10 => {
                if target == Some("1.0") {
                    break;
                }
                gl_tf10to20(gltf, buffers.take());
                // version = gltf.asset.version（现为 "2.0"）-> 循环退出。
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

/// `glTF10to20`（updateVersion.js L943–989）：结构上的 1.0 → 2.0 变换。
///
/// 需要已解码二进制 buffer 的步骤在此跳过并内联注释；
/// 完整推迟列表见模块头。
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
    require_byte_length(gltf, buffers.as_deref()); // L961（buffer 那一半需要已解码源）
    move_byte_stride_to_buffer_view(gltf); // L963
    // 依赖于二进制 buffer 的步骤（M9.2）：仅当提供了已解码 buffer 时运行；
    // 为 `None` 时本链逐字节等同于仅 JSON 的 M9.1 路径。
    if let Some(bufs) = buffers.as_mut() {
        crate::gltf_binary_stage::remove_unused_elements(gltf, bufs); // L837（moveByteStride 末尾）
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

/// `updateInstanceTechniques`（L84）：将 `material.instanceTechnique`
///（`technique` + `values`）提升到 material 自身之上。
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

/// `removeAnimationSamplersIndirection`（L270）：通过 `animation.parameters`
/// 解析 `sampler.input` / `sampler.output`，然后删除 `parameters`。
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

/// `removeEmptyNodes`（L906）+ `isNodeEmpty`（L851）+ `deleteNode`（L874）。
/// 在 `objectsToArrays` 之前运行，因此节点仍以 id 为对象键。
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

/// `isNodeEmpty`（L851）：无内容且为单位/无变换的节点。
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

/// `deleteNode`（L874）：从各 scene 和父节点中移除一个节点，递归处理
/// 变为空的父节点，然后从 `gltf.nodes` 中删除它。
fn delete_node(gltf: &mut Value, node_id: &str) {
    // 从每个 scene 的节点列表中移除。
    remove_id_from_children(gltf, "scenes", "nodes", node_id);

    // 从父节点的 children 中移除；记录哪些父节点引用了它。
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

    // 删除节点自身。
    if let Some(Value::Object(nodes)) = gltf.get_mut("nodes") {
        nodes.remove(node_id);
    }

    // 递归处理现在已为空的父节点。
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

/// 从 `gltf[collection][*][children_key]` 数组（集合的对象键
/// 或数组形式）中移除 `id`。
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

/// `objectsToArrays`（L306–563）：将每个以对象为键的顶层集合
/// 转为数组，并将所有 id 引用重写为数组索引。
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

    // ---- 修正引用（L360–562）----
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

    // nodes（L427）— 收集 skin.skeleton 赋值和待处理的 mesh 节点。
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
            // meshes -> mesh（+ 额外的 mesh 节点）
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

    // skins（L475）— inverseBindMatrices，jointNames -> joints，+ skeleton。
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

/// 将 `material.values[name]`（以及 `KHR_materials_common.values`）的字符串
/// 引用重写为 `{ "index": <texture index> }`（L520–538）。
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

/// `removeEmptyArrays`（L573）：删除空白的顶层数组和空
/// `node.children`。
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

/// `stripAsset`（L589）：删除 `asset.profile` 和 `asset.premultipliedAlpha`。
fn strip_asset(gltf: &mut Value) {
    if let Some(asset) = gltf.get_mut("asset").and_then(Value::as_object_mut) {
        asset.remove("profile");
        asset.remove("premultipliedAlpha");
    }
}

/// `requireKnownExtensions`（L595–612）：将已知的遗留扩展从
/// `extensionsUsed` 提升为 `extensionsRequired`。
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

/// `removeBufferType`（L614）：删除 `buffer.type`。
fn remove_buffer_type(gltf: &mut Value) {
    for_each_top_level_mut_named(gltf, "buffers", &mut |buffer| {
        if let Some(obj) = buffer.as_object_mut() {
            obj.remove("type");
        }
    });
}

/// `removeTextureProperties`（L620）：从每个纹理删除 `format` / `internalFormat` /
/// `target` / `type`。
fn remove_texture_properties(gltf: &mut Value) {
    for_each_top_level_mut_named(gltf, "textures", &mut |texture| {
        if let Some(obj) = texture.as_object_mut() {
            for key in ["format", "internalFormat", "target", "type"] {
                obj.remove(key);
            }
        }
    });
}

/// `requireAttributeSetIndex`（L629）：在 mesh 图元属性和 technique 参数语义上
/// 将 `TEXCOORD` → `TEXCOORD_0`，`COLOR` → `COLOR_0`。
fn require_attribute_set_index(gltf: &mut Value) {
    if let Some(Value::Array(meshes)) = gltf.get_mut("meshes") {
        for mesh in meshes.iter_mut() {
            if let Some(prims) = mesh.get_mut("primitives").and_then(Value::as_array_mut) {
                for prim in prims.iter_mut() {
                    let Some(attrs) = prim.get_mut("attributes").and_then(Value::as_object_mut) else {
                        continue;
                    };
                    // 插入重命名后的键，然后删除未加后缀的原始键。
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
    // strippedSemantic -> 带索引的替换前缀
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
                    // 从当前属性集计算语义重映射。
                    let mut local_renames: Vec<(String, String)> = Vec::new();
                    for sem in attrs.keys() {
                        if sem.starts_with('_') {
                            continue;
                        }
                        // JS：semantic.search(/_[0-9]+/g) -> 第一个 "_<digits>"。
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

/// 找到 `semantic` 中第一个 `_<digits>` 后缀的起始位置
///（JS `semantic.search(/_[0-9]+/g)`），不存在时返回 `None`。
fn find_indexed_suffix(semantic: &str) -> Option<usize> {
    let bytes = semantic.as_bytes();
    (0..bytes.len()).find(|&i| {
        bytes[i] == b'_' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit()
    })
}

/// `clampCameraParameters`（L723）：丢弃零值的 `aspectRatio`，将零值的
/// `yfov` 强制为 `1.0`。
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

/// `requireByteLength`（L745）：从已解码源得到 `buffer.byteLength`
///（当提供了 `buffers` 时）且 `bufferView.byteLength = max(现有值,
/// accessor.byteOffset + count * stride)`。
///
/// buffer 级的那一半需要已解码的 buffer，委托给
/// [`crate::gltf_binary_stage::require_byte_length_buffers`]；为 `None`
///（仅 JSON 路径）时跳过，逐字节保留 M9.1 行为。
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

/// `computeAccessorByteStride`（L739）：存在且非零时使用 `accessor.byteStride`，
/// 否则为来自 buffer view / 分量布局的紧凑 stride。
fn compute_accessor_byte_stride(gltf: &Value, accessor: &Value) -> usize {
    if let Some(bs) = accessor.get("byteStride").and_then(Value::as_u64) {
        if bs != 0 {
            return bs as usize;
        }
    }
    get_accessor_byte_stride(gltf, accessor)
}

/// `moveByteStrideToBufferView`（L766）：将 `accessor.byteStride` 移到
/// buffer view 上，当 buffer view 的各 accessor 使用不同 stride 时
/// 拆分该 buffer view。
///
/// 偏差：上游为*每次*运行都克隆 buffer view，并依赖
/// `removeUnusedElements`（被推迟）丢弃现已失效的原始项。为避免
/// 该依赖，首次运行就地修改原始 buffer view，
/// 仅后续（stride 不同的）运行追加新的 buffer view。对于
/// 常见的单 stride 情形，输出与上游 + `removeUnusedElements`
/// 完全一致（accessor 保留 `bufferView = <原始索引>`）。
fn move_byte_stride_to_buffer_view(gltf: &mut Value) {
    // 被 mesh 图元属性 / morph target 引用的 accessor。
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

    // accessor 索引 -> bufferView 索引；bufferView -> 顶点属性标志。
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

    // 按索引升序处理 buffer view，以匹配 JS 对象键遍历
    //（整数类键按数值顺序遍历）并保持追加的
    // buffer-view 索引确定。
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
            // 删除 accessor.byteStride（L806）
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

/// 从图元 `attributes`（或 morph `target`）对象中收集 accessor 索引。
fn collect_attribute_accessors(attrs: Option<&Value>, out: &mut HashSet<u64>) {
    if let Some(obj) = attrs.and_then(Value::as_object) {
        for (_sem, acc) in obj {
            if let Some(i) = acc.as_u64() {
                out.insert(i);
            }
        }
    }
}

/// 相对于固定 buffer-view 快照的 accessor stride（在活动 buffer views
/// 正在被修改时使用）。
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

// ----------------------------- 共享辅助函数 -----------------------------

/// 通过 `mapping` 将一个字符串 id 映射为索引；非字符串（已索引的
/// 2.0 值）原样透传。
fn map_id(mapping: &HashMap<String, usize>, value: &Value) -> Value {
    match value.as_str() {
        Some(id) => match mapping.get(id) {
            Some(idx) => json!(*idx),
            None => Value::Null,
        },
        None => value.clone(),
    }
}

/// 对于数组形式的集合，重映射 `gltf[collection][*][field]`（字符串 id → 索引）。
fn remap_field(root: &mut Map<String, Value>, collection: &str, field: &str, mapping: &HashMap<String, usize>) {
    if let Some(Value::Array(list)) = root.get_mut(collection) {
        for item in list.iter_mut() {
            remap_field_in(item, field, mapping);
        }
    }
}

/// 在单个 JSON 值内重映射 `value[field]`（字符串 id → 索引）。
fn remap_field_in(value: &mut Value, field: &str, mapping: &HashMap<String, usize>) {
    if let Some(obj) = value.as_object_mut() {
        remap_field_in_obj(obj, field, mapping);
    }
}

/// 作于 [`remap_field_in`]，作用于一个已解包的对象。
fn remap_field_in_obj(obj: &mut Map<String, Value>, field: &str, mapping: &HashMap<String, usize>) {
    if let Some(current) = obj.get(field).cloned() {
        if current.is_string() {
            obj.insert(field.to_string(), map_id(mapping, &current));
        }
    }
}

/// `defined()` 语义（Cesium）：存在且非 null。
fn is_defined(obj: &Map<String, Value>, key: &str) -> bool {
    obj.get(key).map(|v| !v.is_null()).unwrap_or(false)
}

/// 将一个 JSON 数值数组与一个 `f64` 切片比较（完全相等）。
fn number_array_eq(arr: &[Value], expected: &[f64]) -> bool {
    if arr.len() != expected.len() {
        return false;
    }
    arr.iter()
        .zip(expected)
        .all(|(v, e)| v.as_f64().map(|x| x == *e).unwrap_or(false))
}

/// 对于 16 元素的列主序单位矩阵返回 true。
fn is_identity_matrix(m: &[Value]) -> bool {
    if m.len() != 16 {
        return false;
    }
    let identity = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    number_array_eq(m, &identity)
}

/// 移除 `root[key]` 并以 `Vec<Value>` 返回（缺失或非数组时为空）。
fn take_array(root: &mut Map<String, Value>, key: &str) -> Vec<Value> {
    match root.remove(key) {
        Some(Value::Array(a)) => a,
        Some(other) => {
            // 将非数组值原样放回。
            root.insert(key.to_string(), other);
            Vec::new()
        }
        None => Vec::new(),
    }
}

/// 可变遍历 `animation.samplers`，无论它是对象（glTF 1.0，
/// 以 sampler id 为键）还是数组（`objectsToArrays` 之后）。
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

/// 本地的 `addExtensionsRequired`（避免循环导入 util crate 的
/// 辅助函数，同时保持行为一致）。
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

/// 对一个顶层集合的可变遍历（数组或对象形式）。
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

/// 对 `materials` 的可变遍历（数组或对象形式）。
fn for_each_material_mut(gltf: &mut Value, f: &mut impl FnMut(&mut Value)) {
    for_each_top_level_mut_named(gltf, "materials", f);
}

/// 对 techniques 的可变遍历，当使用了 `KHR_techniques_webgl.techniques`
/// 扩展时优先遍历它（镜像 `ForEach.technique`）。
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

    /// 来自 `Specs/Scene/GltfJsonLoaderSpec.js`（L17-132）的 `gltf1` fixture：
    /// 一个带有基于 technique 的红色材质的 glTF 1.0 资源。
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
        // 缺失的 version 默认为 1.0（updateVersion.js L50/L61）。
        assert_eq!(detect_version(&json!({})), GltfVersion::V10);
        // 截断为三个字符（L57）。
        assert_eq!(detect_version(&json!({"asset":{"version":"2.0.0"}})), GltfVersion::V20);
        assert_eq!(detect_version(&json!({"asset":{"version":"1.0.1"}})), GltfVersion::V10);
        // 未知 -> 默认 1.0（L61）。
        assert_eq!(detect_version(&json!({"asset":{"version":"3.5"}})), GltfVersion::V10);
        assert_eq!(GltfVersion::V20.as_str(), "2.0");
    }

    #[test]
    fn version_08_upgrade_is_deferred() {
        let mut gltf = json!({"version":"0.8"});
        let err = update_version(&mut gltf, &opts()).unwrap_err();
        assert_eq!(err, GltfUpgradeError::UnsupportedVersion08);

        // targetVersion 0.8 在被推迟的步骤前停下 -> Ok，版本不变。
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

    /// 一个普通的 2.0 资源原样透传不变（升级循环被跳过，且
    /// 没有遗留扩展时 PBR 转换器为无操作）。
    #[test]
    fn gltf2_passthrough_is_unchanged() {
        let original = gltf2_fixture();
        let mut gltf = original.clone();
        update_version(&mut gltf, &opts()).unwrap();
        assert_eq!(gltf, original, "2.0 passthrough must not mutate the asset");
    }

    /// 对 `gltf1` fixture 的完整 1.0 -> 2.0 升级：objectsToArrays、
    /// 引用回填、byteStride 移动、technique -> PBR 材质。
    #[test]
    fn gltf1_full_upgrade() {
        let mut gltf = gltf1_fixture();
        update_version(&mut gltf, &opts()).unwrap();

        // 版本已提升。
        assert_eq!(gltf.pointer("/asset/version").and_then(Value::as_str), Some("2.0"));

        // objectsToArrays：集合变为数组；字符串 id 变成索引。
        assert!(gltf.get("accessors").and_then(Value::as_array).is_some());
        assert_eq!(gltf.pointer("/accessors/0/bufferView").and_then(Value::as_u64), Some(0));
        assert_eq!(gltf.pointer("/bufferViews/0/buffer").and_then(Value::as_u64), Some(0));
        assert_eq!(gltf.pointer("/meshes/0/primitives/0/attributes/POSITION").and_then(Value::as_u64), Some(0));
        assert_eq!(gltf.pointer("/meshes/0/primitives/0/material").and_then(Value::as_u64), Some(0));
        // node.meshes -> node.mesh；scene/scene.nodes 重映射。
        assert_eq!(gltf.pointer("/nodes/0/mesh").and_then(Value::as_u64), Some(0));
        assert!(gltf.pointer("/nodes/0/meshes").is_none());
        assert_eq!(gltf.pointer("/scene").and_then(Value::as_u64), Some(0));
        assert_eq!(gltf.pointer("/scenes/0/nodes/0").and_then(Value::as_u64), Some(0));
        // name 从对象键赋值。
        assert_eq!(gltf.pointer("/accessors/0/name").and_then(Value::as_str), Some("accessor"));

        // requireByteLength + moveByteStrideToBufferView（仅 JSON 部分）。
        // accessor：FLOAT VEC3 count 1 -> 紧凑 stride 12。
        assert_eq!(gltf.pointer("/bufferViews/0/byteLength").and_then(Value::as_u64), Some(12));
        assert_eq!(gltf.pointer("/bufferViews/0/byteStride").and_then(Value::as_u64), Some(12));
        // componentType 在整个升级过程中保留。
        assert_eq!(gltf.pointer("/accessors/0/componentType").and_then(Value::as_u64), Some(5126));

        // technique -> PBR：材质有 pbrMetallicRoughness，无 technique/values。
        assert_eq!(gltf.pointer("/materials/0/name").and_then(Value::as_str), Some("red"));
        assert!(gltf.pointer("/materials/0/technique").is_none());
        assert!(gltf.pointer("/materials/0/values").is_none());
        let roughness = gltf.pointer("/materials/0/pbrMetallicRoughness/roughnessFactor").and_then(Value::as_f64);
        let metallic = gltf.pointer("/materials/0/pbrMetallicRoughness/metallicFactor").and_then(Value::as_f64);
        assert_eq!(roughness, Some(1.0));
        assert_eq!(metallic, Some(0.0));
        // srgbToLinear(0.8) == 0.6038273388553378（Spec gltf2 材质）。
        let bcf = gltf.pointer("/materials/0/pbrMetallicRoughness/baseColorFactor").and_then(Value::as_array).cloned();
        let bcf = bcf.expect("baseColorFactor present");
        assert_close(bcf[0].as_f64().unwrap(), 0.6038273388553378, "baseColorFactor.r");
        assert_close(bcf[1].as_f64().unwrap(), 0.0, "baseColorFactor.g");
        assert_close(bcf[2].as_f64().unwrap(), 0.0, "baseColorFactor.b");
        assert_close(bcf[3].as_f64().unwrap(), 1.0, "baseColorFactor.a");

        // 遗留的 technique 集合 + 过渡性扩展都已移除。
        assert!(gltf.get("techniques").is_none());
        assert!(gltf.get("programs").is_none());
        assert!(gltf.get("shaders").is_none());
        assert!(gltf.pointer("/extensions/KHR_techniques_webgl").is_none());
        assert!(!crate::gltf_upgrade_util::uses_extension(&gltf, "KHR_techniques_webgl"));
    }

    /// `KHR_materials_common`（PHONG）-> PBR base color + alphaMode/doubleSided。
    #[test]
    fn gltf1_materials_common_to_pbr() {
        let mut gltf = gltf1_materials_common_fixture();
        update_version(&mut gltf, &opts()).unwrap();

        assert_eq!(gltf.pointer("/asset/version").and_then(Value::as_str), Some("2.0"));
        assert_eq!(gltf.pointer("/materials/0/name").and_then(Value::as_str), Some("red"));
        // diffuse [0.8,0,0,1] -> 线性 base color。
        let bcf = gltf.pointer("/materials/0/pbrMetallicRoughness/baseColorFactor").and_then(Value::as_array).cloned();
        let bcf = bcf.expect("baseColorFactor present");
        assert_close(bcf[0].as_f64().unwrap(), 0.6038273388553378, "mc.baseColorFactor.r");
        assert_eq!(gltf.pointer("/materials/0/alphaMode").and_then(Value::as_str), Some("OPAQUE"));
        assert_eq!(gltf.pointer("/materials/0/doubleSided").and_then(Value::as_bool), Some(false));
        // 扩展已从材质和 extensionsUsed/Required 中移除。
        assert!(gltf.pointer("/materials/0/extensions/KHR_materials_common").is_none());
        assert!(!crate::gltf_upgrade_util::uses_extension(&gltf, "KHR_materials_common"));
    }

    /// `removeAnimationSamplersIndirection` + 针对动画的 objectsToArrays。
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

        // parameters 间接引用已移除；samplers 按索引引用 accessor。
        assert!(gltf.pointer("/animations/0/parameters").is_none());
        assert!(gltf.pointer("/animations/0/samplers").and_then(Value::as_array).is_some());
        // accTime -> 0，accValue -> 1（字母键序）。
        assert_eq!(gltf.pointer("/animations/0/samplers/0/input").and_then(Value::as_u64), Some(0));
        assert_eq!(gltf.pointer("/animations/0/samplers/0/output").and_then(Value::as_u64), Some(1));
        // sampler name 已被剔除。
        assert!(gltf.pointer("/animations/0/samplers/0/name").is_none());
        // channel.sampler -> 索引；target.id -> target.node。
        assert_eq!(gltf.pointer("/animations/0/channels/0/sampler").and_then(Value::as_u64), Some(0));
        assert_eq!(gltf.pointer("/animations/0/channels/0/target/node").and_then(Value::as_u64), Some(0));
        assert!(gltf.pointer("/animations/0/channels/0/target/id").is_none());
    }

    /// accessor.byteStride（1.0）移到 bufferView（2.0）上；componentType
    /// 保留，且 bufferView.byteLength 由 accessor 跨度推导。
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

        // byteStride 已从 accessor 移除……
        assert!(gltf.pointer("/accessors/0/byteStride").is_none());
        // ……并移到 bufferView 上。
        assert_eq!(gltf.pointer("/bufferViews/0/byteStride").and_then(Value::as_u64), Some(24));
        // byteLength = byteOffset(0) + count(2) * stride(24) = 48。
        assert_eq!(gltf.pointer("/bufferViews/0/byteLength").and_then(Value::as_u64), Some(48));
        // componentType 保留；bufferView 重映射为索引。
        assert_eq!(gltf.pointer("/accessors/0/componentType").and_then(Value::as_u64), Some(5126));
        assert_eq!(gltf.pointer("/accessors/0/bufferView").and_then(Value::as_u64), Some(0));
    }

    /// `keepLegacyExtensions` 保留 technique 扩展，而非
    /// 将其转换为 PBR。
    #[test]
    fn keep_legacy_extensions_preserves_techniques() {
        let mut gltf = gltf1_fixture();
        let o = UpgradeOptions { keep_legacy_extensions: true, ..Default::default() };
        update_version(&mut gltf, &o).unwrap();
        // 结构升级仍已运行（版本 2.0，数组）……
        assert_eq!(gltf.pointer("/asset/version").and_then(Value::as_str), Some("2.0"));
        // ……但 technique 仍存在于 KHR_techniques_webgl，而非 PBR。
        assert!(gltf.pointer("/extensions/KHR_techniques_webgl/techniques/0").is_some());
        assert!(gltf.pointer("/materials/0/extensions/KHR_techniques_webgl").is_some());
        assert!(gltf.pointer("/materials/0/pbrMetallicRoughness").is_none());
    }

    /// `targetVersion = "1.0"` 在 1.0 -> 2.0 步骤前停下。
    #[test]
    fn target_version_1_0_stops_upgrade() {
        let mut gltf = gltf1_fixture();
        let o = UpgradeOptions { target_version: Some("1.0".to_string()), ..Default::default() };
        update_version(&mut gltf, &o).unwrap();
        // 仍为 1.0；集合仍以对象为键（objectsToArrays 未运行）。
        assert_eq!(gltf.pointer("/asset/version").and_then(Value::as_str), Some("1.0"));
        assert!(gltf.get("accessors").and_then(Value::as_object).is_some());
    }

    /// `requireAttributeSetIndex`：TEXCOORD/COLOR 获得 `_0` 集合索引。
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
