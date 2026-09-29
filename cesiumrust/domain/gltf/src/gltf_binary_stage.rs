//! glTF 1.0 → 2.0 升级链中依赖于二进制 buffer 的步骤。
//!
//! M9.1 在 [`crate::gltf_upgrade`] 中实现了仅 JSON 的变换，并
//! 推迟了上游 `updateVersion.js` 中那几个需要*已解码*
//! 二进制 buffer（`buffer.extras._pipeline.source`）的步骤。本模块
//! 落地这些被推迟的步骤。它们由 M9.2 驱动，仅当调用方
//! 提供已解码的 buffer（[`crate::gltf_upgrade::update_version_with_buffers`]）时才运行；
//! 仅 JSON 的入口点 [`crate::gltf_upgrade::update_version`] 不传递任何
//! buffer，因此行为与 M9.1 逐字节完全一致。
//!
//! 镜像 CesiumJS `packages/engine/Source/Scene/GltfPipeline/`：
//! `findAccessorMinMax.js`、`readAccessorPacked.js`、`getComponentReader.js`、
//! `addBuffer.js`、`updateAccessorComponentTypes.js`、`removeUnusedElements.js`，
//! 以及 `updateVersion.js` 中 `requireByteLength` / `requirePositionAccessorMinMax`
//! / `requireAnimationAccessorMinMax` / `validatePresentAccessorMinMax` 的二进制分支。
//!
//! # buffer 模型
//!
//! 上游将每个 buffer 的已解码字节存储在 `buffer.extras._pipeline.source`
//! （一个带有 `.length`、`.byteOffset`、`.buffer` 的 `Buffer`）。在此，已解码的
//! 源存储在一个平行的 `buffers: &[Vec<u8>]` 中，其索引与 `gltf.buffers` 数组
//! 1:1 对齐（`buffers[i]` 就是 `gltf.buffers[i]` 的源）。
//! 对于一个 GLB，适配器提供 `buffers = vec![binary_chunk]`，因此 buffer 0 就
//! 是嵌入的 chunk。因为 `buffers[i]` 本身*就是*精确的源切片，
//! 上游的 `source.byteOffset` 项在本模型中始终为 `0`。
//!
//! # 精度
//!
//! `Accessor.min` / `Accessor.max` 在强类型模型中是 `Vec<f64>`；二进制
//! payload 是 `f32`（glTF `FLOAT`）。`f32 → f64` 加宽是无损的，因此
//! domain-f64 不变量成立（不会丢失精度）。
//!
//! # 偏差
//!
//! * `removeUnusedElements` 实现了**核心**引用图
//!   （accessor / bufferView / buffer）。上游的扩展分支
//!   （draco、meshopt、EXT_feature_metadata、EXT_structural_metadata、
//!   EXT_mesh_gpu_instancing、CESIUM_primitive_outline）被跳过：
//!   1.0 → 2.0 升级链从不产生它们。参见 docs/deviations.md。
//! * `findAccessorMinMax` / `readAccessorPacked` 复用 [`crate::gltf_model`] 中的强类型读取器
//!   （`read_f32_data` / `read_u16_data` / `read_u32_data`）
//!   处理真实的 `FLOAT` / `UNSIGNED_SHORT` / `UNSIGNED_INT` 情形，并对
//!   剩余的整数类型回退到一个通用分量读取器。
//! * 只有调用方已解码的 buffer 才可读；一个未被获取的外部 `uri`
//!   buffer 会解析为零值（不在本里程碑针对的 GLB
//!   嵌入 chunk 路径范围内）。

use serde_json::{json, Value};
use std::collections::HashSet;

use crate::gltf_model::{Accessor, BufferView, ComponentType};
use crate::gltf_upgrade_util::{
    component_size_in_bytes, get_accessor_byte_stride, index_into, number_of_components_for_type,
    webgl,
};

// --------------------------- 分量读取 ----------------------------

/// 在 `off` 处读取一个分量为 `f64`（镜像 `getComponentReader.js`）。
/// 对于未知的分量类型或越界读取返回 `None`。
fn read_component_f64(src: &[u8], off: usize, component_type: u64) -> Option<f64> {
    let size = match component_type {
        webgl::BYTE | webgl::UNSIGNED_BYTE => 1,
        webgl::SHORT | webgl::UNSIGNED_SHORT => 2,
        webgl::INT | webgl::UNSIGNED_INT | webgl::FLOAT => 4,
        webgl::DOUBLE => 8,
        _ => return None,
    };
    if off + size > src.len() {
        return None;
    }
    let b = &src[off..off + size];
    let v = match component_type {
        webgl::BYTE => i8::from_le_bytes([b[0]]) as f64,
        webgl::UNSIGNED_BYTE => b[0] as f64,
        webgl::SHORT => i16::from_le_bytes([b[0], b[1]]) as f64,
        webgl::UNSIGNED_SHORT => u16::from_le_bytes([b[0], b[1]]) as f64,
        webgl::INT => i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
        webgl::UNSIGNED_INT => u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
        webgl::FLOAT => f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
        webgl::DOUBLE => f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]),
        _ => return None,
    };
    Some(v)
}

/// `readAccessorPacked.js`：将 accessor 的值以连续的
/// 元素主序数组（`values[i * numComp + j]`）返回，使用 accessor 的
/// *自身* 分量类型读取。当 buffer view / buffer 缺失时为零。
fn read_accessor_packed(gltf: &Value, accessor: &Value, buffers: &[Vec<u8>]) -> Vec<f64> {
    let num_comp =
        number_of_components_for_type(accessor.get("type").and_then(Value::as_str).unwrap_or(""));
    let count = accessor.get("count").and_then(Value::as_u64).unwrap_or(0) as usize;
    let component_type = accessor
        .get("componentType")
        .and_then(Value::as_u64)
        .unwrap_or(webgl::FLOAT);
    let comp_byte_len = component_size_in_bytes(component_type);
    let mut values = vec![0.0f64; num_comp * count];

    let Some(bv_id) = accessor.get("bufferView").and_then(Value::as_u64) else {
        return values; // !defined(bufferView) -> fill(0)
    };
    let Some(bv) = index_into(gltf, "bufferViews", bv_id) else {
        return values;
    };
    let Some(buffer_id) = bv.get("buffer").and_then(Value::as_u64) else {
        return values;
    };
    let Some(src) = buffers.get(buffer_id as usize) else {
        return values;
    };

    let byte_stride = get_accessor_byte_stride(gltf, accessor);
    let bv_byte_offset = bv.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize;
    let acc_byte_offset = accessor.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize;
    // 在本模型中 source.byteOffset 为 0（buffers[i] 就是精确的源）。
    let mut byte_offset = acc_byte_offset + bv_byte_offset;

    for i in 0..count {
        for j in 0..num_comp {
            if let Some(v) = read_component_f64(src, byte_offset + j * comp_byte_len, component_type)
            {
                values[i * num_comp + j] = v;
            }
        }
        byte_offset += byte_stride;
    }
    values
}

/// 主要的 min/max 路径：复用 [`crate::gltf_model`] 中的强类型读取器
/// （`read_f32_data` / `read_u16_data` / `read_u32_data`）。对于它们未覆盖的
/// 分量类型（有符号 / 多分量整数）返回 `None`，由调用方
/// 改用 [`read_accessor_packed`] 处理。
fn read_via_typed_readers(
    gltf: &Value,
    accessor: &Value,
    buffers: &[Vec<u8>],
    num_comp: usize,
) -> Option<Vec<f64>> {
    let ct = accessor.get("componentType").and_then(Value::as_u64)?;
    let covered = ct == webgl::FLOAT
        || ((ct == webgl::UNSIGNED_SHORT || ct == webgl::UNSIGNED_INT) && num_comp == 1);
    if !covered {
        return None;
    }
    let acc: Accessor = serde_json::from_value(accessor.clone()).ok()?;
    let buffer_views: Vec<BufferView> =
        serde_json::from_value(gltf.get("bufferViews").cloned()?).ok()?;
    let flat: Vec<f64> = match acc.component_type {
        ComponentType::F32 => acc
            .read_f32_data(buffers, &buffer_views)
            .into_iter()
            .map(f64::from)
            .collect(),
        ComponentType::U16 => acc
            .read_u16_data(buffers, &buffer_views)
            .into_iter()
            .map(f64::from)
            .collect(),
        ComponentType::U32 => acc
            .read_u32_data(buffers, &buffer_views)
            .into_iter()
            .map(f64::from)
            .collect(),
        _ => return None,
    };
    Some(flat)
}

/// 对元素主序扁平数组逐分量求 min/max。空数组
/// （`count == 0`）使 `min = +inf` / `max = -inf`，与上游一致（
/// 读取循环从不执行）。
fn minmax_from_flat(flat: &[f64], num_comp: usize) -> (Vec<f64>, Vec<f64>) {
    let mut min = vec![f64::INFINITY; num_comp];
    let mut max = vec![f64::NEG_INFINITY; num_comp];
    for chunk in flat.chunks_exact(num_comp) {
        for (j, &v) in chunk.iter().enumerate() {
            if v < min[j] {
                min[j] = v;
            }
            if v > max[j] {
                max[j] = v;
            }
        }
    }
    (min, max)
}

/// `findAccessorMinMax.js`：`accessor` 每个分量的 min/max。
///
/// 仅当 accessor 的 `type` 未知（0 个分量）时返回 `None`。当
/// `bufferView` 未定义时规范要求填零，因此返回 `(zeros, zeros)`。
/// 数值保持在 f64 域内。
pub(crate) fn find_accessor_min_max(
    gltf: &Value,
    accessor: &Value,
    buffers: &[Vec<u8>],
) -> Option<(Vec<f64>, Vec<f64>)> {
    let num_comp =
        number_of_components_for_type(accessor.get("type").and_then(Value::as_str).unwrap_or(""));
    if num_comp == 0 {
        return None;
    }
    if accessor.get("bufferView").is_none() {
        return Some((vec![0.0; num_comp], vec![0.0; num_comp]));
    }
    let flat = match read_via_typed_readers(gltf, accessor, buffers, num_comp) {
        Some(f) => f,
        None => read_accessor_packed(gltf, accessor, buffers),
    };
    Some(minmax_from_flat(&flat, num_comp))
}

// ------------------------- requireByteLength (buffer) -------------------------

/// `requireByteLength` 的 buffer 层部分（updateVersion.js L746–750）：
/// 缺失时 `buffer.byteLength = source.length`。bufferView 层部分
/// 保留在 [`crate::gltf_upgrade::require_byte_length`]（仅 JSON，M9.1）。
pub(crate) fn require_byte_length_buffers(gltf: &mut Value, buffers: &[Vec<u8>]) {
    if let Some(Value::Array(bufs)) = gltf.get_mut("buffers") {
        for (i, b) in bufs.iter_mut().enumerate() {
            if let Some(obj) = b.as_object_mut() {
                if !obj.contains_key("byteLength") {
                    if let Some(src) = buffers.get(i) {
                        obj.insert("byteLength".to_string(), json!(src.len()));
                    }
                }
            }
        }
    }
}

// --------------------------- min/max require 步骤 ---------------------------

/// 收集 primitive attribute 语义以 `semantic` 开头的
/// accessor id（`ForEach.accessorWithSemantic`，去重，按 mesh/primitive/attribute
/// 迭代顺序）。
fn collect_accessor_ids_with_semantic(gltf: &Value, semantic: &str) -> Vec<u64> {
    let mut visited: HashSet<u64> = HashSet::new();
    let mut out: Vec<u64> = Vec::new();
    if let Some(Value::Array(meshes)) = gltf.get("meshes") {
        for mesh in meshes {
            if let Some(prims) = mesh.get("primitives").and_then(Value::as_array) {
                for prim in prims {
                    if let Some(attrs) = prim.get("attributes").and_then(Value::as_object) {
                        for (sem, acc) in attrs {
                            // attributeSemantic.indexOf(semantic) === 0
                            if sem.starts_with(semantic) {
                                if let Some(id) = acc.as_u64() {
                                    if visited.insert(id) {
                                        out.push(id);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

/// `accessors[id]` 的可变查找（`objectsToArrays` 后的数组形式，
/// 对象形式也予以容忍）。
fn accessor_mut(gltf: &mut Value, id: u64) -> Option<&mut Value> {
    match gltf.get_mut("accessors")? {
        Value::Array(a) => a.get_mut(id as usize),
        Value::Object(o) => o.get_mut(id.to_string().as_str()),
        _ => None,
    }
}

/// 顶层集合的元素数量（数组或对象形式）。
fn collection_len(gltf: &Value, name: &str) -> usize {
    match gltf.get(name) {
        Some(Value::Array(a)) => a.len(),
        Some(Value::Object(o)) => o.len(),
        _ => 0,
    }
}

/// 当二者之一缺失时（`!defined(min) || !defined(max)` → 两者都计算），
/// 由已解码 buffer 设置 `accessor.min` / `accessor.max`。
fn fill_min_max_if_missing(gltf: &mut Value, id: u64, buffers: &[Vec<u8>]) {
    let Some(av) = index_into(gltf, "accessors", id).cloned() else {
        return;
    };
    let has_min = av.get("min").is_some();
    let has_max = av.get("max").is_some();
    if has_min && has_max {
        return;
    }
    if let Some((min, max)) = find_accessor_min_max(gltf, &av, buffers) {
        if let Some(obj) = accessor_mut(gltf, id).and_then(Value::as_object_mut) {
            obj.insert("min".to_string(), json!(min));
            obj.insert("max".to_string(), json!(max));
        }
    }
}

/// `requirePositionAccessorMinMax`（updateVersion.js L840）。
pub(crate) fn require_position_accessor_min_max(gltf: &mut Value, buffers: &[Vec<u8>]) {
    for id in collect_accessor_ids_with_semantic(gltf, "POSITION") {
        fill_min_max_if_missing(gltf, id, buffers);
    }
}

/// 每个 animation sampler 的输入 accessor id（`ForEach.animation` →
/// `ForEach.animationSampler`，`sampler.input`）。
fn animation_sampler_input_ids(gltf: &Value) -> Vec<u64> {
    let mut out = Vec::new();
    if let Some(Value::Array(anims)) = gltf.get("animations") {
        for anim in anims {
            if let Some(samplers) = anim.get("samplers").and_then(Value::as_array) {
                for s in samplers {
                    if let Some(inp) = s.get("input").and_then(Value::as_u64) {
                        out.push(inp);
                    }
                }
            }
        }
    }
    out
}

/// `requireAnimationAccessorMinMax`（updateVersion.js L916）。
pub(crate) fn require_animation_accessor_min_max(gltf: &mut Value, buffers: &[Vec<u8>]) {
    for id in animation_sampler_input_ids(gltf) {
        fill_min_max_if_missing(gltf, id, buffers);
    }
}

/// `validatePresentAccessorMinMax`（updateVersion.js L929）：对每个
/// 已有 `min` 或 `max` 的 accessor，从 buffer 重新计算，并用精确值
/// 覆盖已存在的字段。
pub(crate) fn validate_present_accessor_min_max(gltf: &mut Value, buffers: &[Vec<u8>]) {
    let n = collection_len(gltf, "accessors");
    for id in 0..n as u64 {
        let Some(av) = index_into(gltf, "accessors", id).cloned() else {
            continue;
        };
        let has_min = av.get("min").is_some();
        let has_max = av.get("max").is_some();
        if !has_min && !has_max {
            continue;
        }
        if let Some((min, max)) = find_accessor_min_max(gltf, &av, buffers) {
            if let Some(obj) = accessor_mut(gltf, id).and_then(Value::as_object_mut) {
                if has_min {
                    obj.insert("min".to_string(), json!(min));
                }
                if has_max {
                    obj.insert("max".to_string(), json!(max));
                }
            }
        }
    }
}

// ----------------------- updateAccessorComponentTypes -----------------------

/// `addBuffer.js`：将 `data` 作为新 buffer 加上一个匹配的 buffer view 追加，
/// 返回新的 buffer-view id。已解码字节被推入
/// 平行的 `buffers` vec，从而保持与 `gltf.buffers` 的索引对齐。
fn add_buffer(gltf: &mut Value, data: Vec<u8>, buffers: &mut Vec<Vec<u8>>) -> Option<u64> {
    let len = data.len();
    let root = gltf.as_object_mut()?;
    if !matches!(root.get("buffers"), Some(Value::Array(_)))
        || !matches!(root.get("bufferViews"), Some(Value::Array(_)))
    {
        return None;
    }
    buffers.push(data);
    let buffer_id = {
        let arr = root.get_mut("buffers").and_then(Value::as_array_mut)?;
        arr.push(json!({ "byteLength": len }));
        (arr.len() - 1) as u64
    };
    let arr = root.get_mut("bufferViews").and_then(Value::as_array_mut)?;
    arr.push(json!({ "buffer": buffer_id, "byteOffset": 0, "byteLength": len }));
    Some((arr.len() - 1) as u64)
}

/// `ComponentDatatype.createTypedArray(newType, values)` 针对
/// `updateAccessorComponentTypes` 会转换到的两个目标的字节编码。镜像 JS 的
/// `ToUint8` / `ToUint16` 环绕（向零截断，再对 2^width 取模）。
fn encode_typed(values: &[f64], new_type: u64) -> Vec<u8> {
    let comp_size = component_size_in_bytes(new_type);
    let mut out = Vec::with_capacity(values.len() * comp_size);
    for &v in values {
        match new_type {
            webgl::UNSIGNED_BYTE => out.push(v as i64 as u8),
            webgl::UNSIGNED_SHORT => out.extend_from_slice(&(v as i64 as u16).to_le_bytes()),
            _ => {}
        }
    }
    out
}

/// `convertType`（updateAccessorComponentTypes.js L42）：将 accessor 的数据
/// 重新打包为 `new_type`，存入一个新 buffer，并重新指向该 accessor。
fn convert_type(gltf: &mut Value, accessor_id: u64, new_type: u64, buffers: &mut Vec<Vec<u8>>) {
    let Some(av) = index_into(gltf, "accessors", accessor_id).cloned() else {
        return;
    };
    let values = read_accessor_packed(gltf, &av, buffers);
    let encoded = encode_typed(&values, new_type);
    let Some(new_bv) = add_buffer(gltf, encoded, buffers) else {
        return;
    };
    if let Some(obj) = accessor_mut(gltf, accessor_id).and_then(Value::as_object_mut) {
        obj.insert("bufferView".to_string(), json!(new_bv));
        obj.insert("componentType".to_string(), json!(new_type));
        obj.insert("byteOffset".to_string(), json!(0));
    }
}

/// `updateAccessorComponentTypes`（updateVersion.js L980）：JOINTS_0 必须为
/// `UNSIGNED_BYTE` / `UNSIGNED_SHORT`；WEIGHTS_0 不得为有符号。
pub(crate) fn update_accessor_component_types(gltf: &mut Value, buffers: &mut Vec<Vec<u8>>) {
    for (semantic, is_joints) in [("JOINTS_0", true), ("WEIGHTS_0", false)] {
        for id in collect_accessor_ids_with_semantic(gltf, semantic) {
            let Some(av) = index_into(gltf, "accessors", id).cloned() else {
                continue;
            };
            let ct = av.get("componentType").and_then(Value::as_u64).unwrap_or(0);
            let new_type = if is_joints {
                if ct == webgl::BYTE {
                    Some(webgl::UNSIGNED_BYTE)
                } else if ct != webgl::UNSIGNED_BYTE && ct != webgl::UNSIGNED_SHORT {
                    Some(webgl::UNSIGNED_SHORT)
                } else {
                    None
                }
            } else if ct == webgl::BYTE {
                Some(webgl::UNSIGNED_BYTE)
            } else if ct == webgl::SHORT {
                Some(webgl::UNSIGNED_SHORT)
            } else {
                None
            };
            if let Some(nt) = new_type {
                convert_type(gltf, id, nt, buffers);
            }
        }
    }
}

// -------------------------- removeUnusedElements --------------------------

/// 当一个整数引用 `v` 指向被移除 `id` 之后时对其 decrement
/// （`Remove.*` 的 `if (ref > id) ref--`）。
fn dec_if_gt(v: &mut Value, id: usize) {
    if let Some(n) = v.as_u64() {
        if n > id as u64 {
            *v = json!(n - 1);
        }
    }
}

/// `getListOfElementsIdsInUse.accessor`（核心引用图；扩展分支被跳过）。
fn used_accessor_ids(gltf: &Value) -> HashSet<usize> {
    let mut used = HashSet::new();
    if let Some(Value::Array(meshes)) = gltf.get("meshes") {
        for mesh in meshes {
            if let Some(prims) = mesh.get("primitives").and_then(Value::as_array) {
                for prim in prims {
                    if let Some(attrs) = prim.get("attributes").and_then(Value::as_object) {
                        for (_s, v) in attrs {
                            if let Some(id) = v.as_u64() {
                                used.insert(id as usize);
                            }
                        }
                    }
                    if let Some(targets) = prim.get("targets").and_then(Value::as_array) {
                        for t in targets {
                            if let Some(obj) = t.as_object() {
                                for (_s, v) in obj {
                                    if let Some(id) = v.as_u64() {
                                        used.insert(id as usize);
                                    }
                                }
                            }
                        }
                    }
                    if let Some(id) = prim.get("indices").and_then(Value::as_u64) {
                        used.insert(id as usize);
                    }
                }
            }
        }
    }
    if let Some(Value::Array(skins)) = gltf.get("skins") {
        for skin in skins {
            if let Some(id) = skin.get("inverseBindMatrices").and_then(Value::as_u64) {
                used.insert(id as usize);
            }
        }
    }
    if let Some(Value::Array(anims)) = gltf.get("animations") {
        for anim in anims {
            if let Some(samplers) = anim.get("samplers").and_then(Value::as_array) {
                for s in samplers {
                    if let Some(id) = s.get("input").and_then(Value::as_u64) {
                        used.insert(id as usize);
                    }
                    if let Some(id) = s.get("output").and_then(Value::as_u64) {
                        used.insert(id as usize);
                    }
                }
            }
        }
    }
    used
}

/// `getListOfElementsIdsInUse.bufferView`（核心引用图；扩展分支被跳过）。
fn used_buffer_view_ids(gltf: &Value) -> HashSet<usize> {
    let mut used = HashSet::new();
    if let Some(Value::Array(accs)) = gltf.get("accessors") {
        for a in accs {
            if let Some(id) = a.get("bufferView").and_then(Value::as_u64) {
                used.insert(id as usize);
            }
        }
    }
    for coll in ["shaders", "images"] {
        if let Some(Value::Array(items)) = gltf.get(coll) {
            for it in items {
                if let Some(id) = it.get("bufferView").and_then(Value::as_u64) {
                    used.insert(id as usize);
                }
            }
        }
    }
    used
}

/// `getListOfElementsIdsInUse.buffer`（核心引用图；扩展分支被跳过）。
fn used_buffer_ids(gltf: &Value) -> HashSet<usize> {
    let mut used = HashSet::new();
    if let Some(Value::Array(bvs)) = gltf.get("bufferViews") {
        for bv in bvs {
            if let Some(id) = bv.get("buffer").and_then(Value::as_u64) {
                used.insert(id as usize);
            }
        }
    }
    used
}

/// `Remove.accessor`：剪辑掉 `accessors[id]` 并移位每个引用它的索引。
fn remove_accessor(gltf: &mut Value, id: usize) {
    if let Some(Value::Array(accs)) = gltf.get_mut("accessors") {
        if id < accs.len() {
            accs.remove(id);
        }
    }
    if let Some(Value::Array(meshes)) = gltf.get_mut("meshes") {
        for mesh in meshes.iter_mut() {
            if let Some(prims) = mesh.get_mut("primitives").and_then(Value::as_array_mut) {
                for prim in prims.iter_mut() {
                    if let Some(attrs) = prim.get_mut("attributes").and_then(Value::as_object_mut) {
                        for (_s, v) in attrs.iter_mut() {
                            dec_if_gt(v, id);
                        }
                    }
                    if let Some(targets) = prim.get_mut("targets").and_then(Value::as_array_mut) {
                        for t in targets.iter_mut() {
                            if let Some(obj) = t.as_object_mut() {
                                for (_s, v) in obj.iter_mut() {
                                    dec_if_gt(v, id);
                                }
                            }
                        }
                    }
                    if let Some(idx) = prim.get_mut("indices") {
                        dec_if_gt(idx, id);
                    }
                }
            }
        }
    }
    if let Some(Value::Array(skins)) = gltf.get_mut("skins") {
        for skin in skins.iter_mut() {
            if let Some(ibm) = skin.get_mut("inverseBindMatrices") {
                dec_if_gt(ibm, id);
            }
        }
    }
    if let Some(Value::Array(anims)) = gltf.get_mut("animations") {
        for anim in anims.iter_mut() {
            if let Some(samplers) = anim.get_mut("samplers").and_then(Value::as_array_mut) {
                for s in samplers.iter_mut() {
                    if let Some(inp) = s.get_mut("input") {
                        dec_if_gt(inp, id);
                    }
                    if let Some(outp) = s.get_mut("output") {
                        dec_if_gt(outp, id);
                    }
                }
            }
        }
    }
}

/// `Remove.bufferView`：剪辑掉 `bufferViews[id]` 并移位引用它的索引。
fn remove_buffer_view(gltf: &mut Value, id: usize) {
    if let Some(Value::Array(bvs)) = gltf.get_mut("bufferViews") {
        if id < bvs.len() {
            bvs.remove(id);
        }
    }
    if let Some(Value::Array(accs)) = gltf.get_mut("accessors") {
        for a in accs.iter_mut() {
            if let Some(bv) = a.get_mut("bufferView") {
                dec_if_gt(bv, id);
            }
        }
    }
    for coll in ["shaders", "images"] {
        if let Some(Value::Array(items)) = gltf.get_mut(coll) {
            for it in items.iter_mut() {
                if let Some(bv) = it.get_mut("bufferView") {
                    dec_if_gt(bv, id);
                }
            }
        }
    }
}

/// `Remove.buffer`：剪辑掉 `buffers[id]`（以及平行的已解码源）并
/// 移位 `bufferView.buffer` 引用。
fn remove_buffer(gltf: &mut Value, buffers: &mut Vec<Vec<u8>>, id: usize) {
    if let Some(Value::Array(bufs)) = gltf.get_mut("buffers") {
        if id < bufs.len() {
            bufs.remove(id);
        }
    }
    if id < buffers.len() {
        buffers.remove(id);
    }
    if let Some(Value::Array(bvs)) = gltf.get_mut("bufferViews") {
        for bv in bvs.iter_mut() {
            if let Some(b) = bv.get_mut("buffer") {
                dec_if_gt(b, id);
            }
        }
    }
}

/// 针对一种类型的 `removeUnusedElementsByType`（遍历原始索引，
/// 在递增的当前索引处剪辑，每次移除后移位其后的引用）。
fn remove_unused_by(
    gltf: &mut Value,
    buffers: &mut Vec<Vec<u8>>,
    name: &str,
    used: HashSet<usize>,
    kind: ElementKind,
) {
    let n = collection_len(gltf, name);
    let mut removed = 0usize;
    for i in 0..n {
        if !used.contains(&i) {
            match kind {
                ElementKind::Accessor => remove_accessor(gltf, i - removed),
                ElementKind::BufferView => remove_buffer_view(gltf, i - removed),
                ElementKind::Buffer => remove_buffer(gltf, buffers, i - removed),
            }
            removed += 1;
        }
    }
}

#[derive(Clone, Copy)]
enum ElementKind {
    Accessor,
    BufferView,
    Buffer,
}

/// `removeUnusedElements(gltf, ["accessor", "bufferView", "buffer"])`
/// （updateVersion.js L837，在 `moveByteStrideToBufferView` 末尾调用）。
/// 各类型按 `allElementTypes` 顺序处理：accessor → bufferView →
/// buffer，因此每一遍都看到上一遍产生的引用。
pub(crate) fn remove_unused_elements(gltf: &mut Value, buffers: &mut Vec<Vec<u8>>) {
    let used = used_accessor_ids(gltf);
    remove_unused_by(gltf, buffers, "accessors", used, ElementKind::Accessor);
    let used = used_buffer_view_ids(gltf);
    remove_unused_by(gltf, buffers, "bufferViews", used, ElementKind::BufferView);
    let used = used_buffer_ids(gltf);
    remove_unused_by(gltf, buffers, "buffers", used, ElementKind::Buffer);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f32_bytes(vals: &[f32]) -> Vec<u8> {
        let mut out = Vec::with_capacity(vals.len() * 4);
        for v in vals {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }

    fn f64s(v: &Value) -> Vec<f64> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect()
    }

    #[test]
    fn find_min_max_position_f32_vec3() {
        let buffers = vec![f32_bytes(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0])];
        let gltf = json!({
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }],
            "accessors": [{
                "bufferView": 0, "byteOffset": 0,
                "componentType": 5126, "type": "VEC3", "count": 3
            }]
        });
        let acc = index_into(&gltf, "accessors", 0).unwrap();
        let (min, max) = find_accessor_min_max(&gltf, acc, &buffers).unwrap();
        assert_eq!(min, vec![0.0, 0.0, 0.0]);
        assert_eq!(max, vec![1.0, 1.0, 0.0]);
    }

    #[test]
    fn find_min_max_undefined_buffer_view_is_zeros() {
        let gltf = json!({
            "accessors": [{ "componentType": 5126, "type": "VEC3", "count": 2 }]
        });
        let acc = index_into(&gltf, "accessors", 0).unwrap();
        let (min, max) = find_accessor_min_max(&gltf, acc, &[]).unwrap();
        assert_eq!(min, vec![0.0, 0.0, 0.0]);
        assert_eq!(max, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn find_min_max_u16_scalar_via_typed_reader() {
        // indices: 5, 2, 9（UNSIGNED_SHORT SCALAR）-> min 2, max 9
        let mut bin = Vec::new();
        for v in [5u16, 2, 9] {
            bin.extend_from_slice(&v.to_le_bytes());
        }
        let gltf = json!({
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": 6 }],
            "accessors": [{
                "bufferView": 0, "byteOffset": 0,
                "componentType": 5123, "type": "SCALAR", "count": 3
            }]
        });
        let acc = index_into(&gltf, "accessors", 0).unwrap();
        let (min, max) = find_accessor_min_max(&gltf, acc, &[bin]).unwrap();
        assert_eq!(min, vec![2.0]);
        assert_eq!(max, vec![9.0]);
    }

    #[test]
    fn require_position_fills_missing_min_max() {
        let buffers = vec![f32_bytes(&[0.0, 0.0, 0.0, 2.0, 3.0, 4.0])];
        let mut gltf = json!({
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": 24 }],
            "accessors": [{
                "bufferView": 0, "byteOffset": 0,
                "componentType": 5126, "type": "VEC3", "count": 2
            }],
            "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0 } }] }]
        });
        require_position_accessor_min_max(&mut gltf, &buffers);
        assert_eq!(f64s(gltf.pointer("/accessors/0/min").unwrap()), vec![0.0, 0.0, 0.0]);
        assert_eq!(f64s(gltf.pointer("/accessors/0/max").unwrap()), vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn require_position_preserves_existing_min_max() {
        let buffers = vec![f32_bytes(&[0.0, 0.0, 0.0, 2.0, 3.0, 4.0])];
        let mut gltf = json!({
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": 24 }],
            "accessors": [{
                "bufferView": 0, "byteOffset": 0, "componentType": 5126, "type": "VEC3",
                "count": 2, "min": [-9.0, -9.0, -9.0], "max": [9.0, 9.0, 9.0]
            }],
            "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0 } }] }]
        });
        require_position_accessor_min_max(&mut gltf, &buffers);
        // 两者都存在 -> 不动（上游 `!defined(min) || !defined(max)`）。
        assert_eq!(f64s(gltf.pointer("/accessors/0/min").unwrap()), vec![-9.0, -9.0, -9.0]);
        assert_eq!(f64s(gltf.pointer("/accessors/0/max").unwrap()), vec![9.0, 9.0, 9.0]);
    }

    #[test]
    fn require_animation_fills_input_min_max() {
        // animation input：时间值 0.0, 0.5, 1.0（FLOAT SCALAR）
        let buffers = vec![f32_bytes(&[0.0, 0.5, 1.0])];
        let mut gltf = json!({
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": 12 }],
            "accessors": [{
                "bufferView": 0, "byteOffset": 0,
                "componentType": 5126, "type": "SCALAR", "count": 3
            }],
            "animations": [{ "samplers": [{ "input": 0, "output": 0 }] }]
        });
        require_animation_accessor_min_max(&mut gltf, &buffers);
        assert_eq!(f64s(gltf.pointer("/accessors/0/min").unwrap()), vec![0.0]);
        assert_eq!(f64s(gltf.pointer("/accessors/0/max").unwrap()), vec![1.0]);
    }

    #[test]
    fn validate_present_recomputes_imprecise_min_max() {
        let buffers = vec![f32_bytes(&[0.0, 0.0, 0.0, 2.0, 3.0, 4.0])];
        let mut gltf = json!({
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": 24 }],
            "accessors": [{
                "bufferView": 0, "byteOffset": 0, "componentType": 5126, "type": "VEC3",
                "count": 2, "min": [-1.0, -1.0, -1.0], "max": [99.0, 99.0, 99.0]
            }]
        });
        validate_present_accessor_min_max(&mut gltf, &buffers);
        assert_eq!(f64s(gltf.pointer("/accessors/0/min").unwrap()), vec![0.0, 0.0, 0.0]);
        assert_eq!(f64s(gltf.pointer("/accessors/0/max").unwrap()), vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn require_byte_length_buffers_fills_from_source() {
        let buffers = vec![vec![7u8; 20]];
        let mut gltf = json!({ "buffers": [{ "uri": "data:..." }] });
        require_byte_length_buffers(&mut gltf, &buffers);
        assert_eq!(gltf.pointer("/buffers/0/byteLength").and_then(Value::as_u64), Some(20));
        // 已存在的 byteLength 保留。
        let mut gltf2 = json!({ "buffers": [{ "byteLength": 5 }] });
        require_byte_length_buffers(&mut gltf2, &buffers);
        assert_eq!(gltf2.pointer("/buffers/0/byteLength").and_then(Value::as_u64), Some(5));
    }

    #[test]
    fn update_component_types_joints_byte_to_unsigned_byte() {
        // JOINTS_0: BYTE VEC4，一个元素 [0, 1, 2, 3]
        let mut gltf = json!({
            "asset": { "version": "2.0" },
            "buffers": [{ "byteLength": 4 }],
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": 4 }],
            "accessors": [{
                "bufferView": 0, "byteOffset": 0,
                "componentType": 5120, "type": "VEC4", "count": 1
            }],
            "meshes": [{ "primitives": [{ "attributes": { "JOINTS_0": 0 } }] }]
        });
        let mut buffers = vec![vec![0u8, 1, 2, 3]];
        update_accessor_component_types(&mut gltf, &mut buffers);

        // Accessor 重新指向一个全新的 buffer view，componentType 为 UNSIGNED_BYTE。
        assert_eq!(gltf.pointer("/accessors/0/componentType").and_then(Value::as_u64), Some(5121));
        assert_eq!(gltf.pointer("/accessors/0/bufferView").and_then(Value::as_u64), Some(1));
        assert_eq!(gltf.pointer("/accessors/0/byteOffset").and_then(Value::as_u64), Some(0));
        // 追加了一个新 buffer + buffer view，已解码源也如此。
        assert_eq!(gltf.pointer("/buffers").and_then(Value::as_array).unwrap().len(), 2);
        assert_eq!(gltf.pointer("/bufferViews").and_then(Value::as_array).unwrap().len(), 2);
        assert_eq!(gltf.pointer("/bufferViews/1/buffer").and_then(Value::as_u64), Some(1));
        assert_eq!(buffers.len(), 2);
        assert_eq!(buffers[1], vec![0u8, 1, 2, 3]);
    }

    #[test]
    fn update_component_types_weights_short_to_unsigned_short() {
        // WEIGHTS_0: SHORT VEC4 -> UNSIGNED_SHORT
        let mut bin = Vec::new();
        for v in [10i16, 20, 30, 40] {
            bin.extend_from_slice(&v.to_le_bytes());
        }
        let mut gltf = json!({
            "asset": { "version": "2.0" },
            "buffers": [{ "byteLength": 8 }],
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": 8 }],
            "accessors": [{
                "bufferView": 0, "byteOffset": 0,
                "componentType": 5122, "type": "VEC4", "count": 1
            }],
            "meshes": [{ "primitives": [{ "attributes": { "WEIGHTS_0": 0 } }] }]
        });
        let mut buffers = vec![bin];
        update_accessor_component_types(&mut gltf, &mut buffers);
        assert_eq!(gltf.pointer("/accessors/0/componentType").and_then(Value::as_u64), Some(5123));
        assert_eq!(buffers[1], {
            let mut e = Vec::new();
            for v in [10u16, 20, 30, 40] {
                e.extend_from_slice(&v.to_le_bytes());
            }
            e
        });
    }

    #[test]
    fn update_component_types_leaves_unsigned_short_joints() {
        let mut gltf = json!({
            "buffers": [{ "byteLength": 8 }],
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": 8 }],
            "accessors": [{
                "bufferView": 0, "byteOffset": 0,
                "componentType": 5123, "type": "VEC4", "count": 1
            }],
            "meshes": [{ "primitives": [{ "attributes": { "JOINTS_0": 0 } }] }]
        });
        let mut buffers = vec![vec![0u8; 8]];
        update_accessor_component_types(&mut gltf, &mut buffers);
        // 已是 UNSIGNED_SHORT -> 不转换，无新 buffer。
        assert_eq!(gltf.pointer("/accessors/0/componentType").and_then(Value::as_u64), Some(5123));
        assert_eq!(gltf.pointer("/buffers").and_then(Value::as_array).unwrap().len(), 1);
        assert_eq!(buffers.len(), 1);
    }

    #[test]
    fn remove_unused_drops_orphan_accessor_and_shifts_refs() {
        let mut gltf = json!({
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "type": "VEC3", "count": 1 },
                { "bufferView": 0, "componentType": 5126, "type": "VEC3", "count": 1 }
            ],
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": 12 }],
            "buffers": [{ "byteLength": 12 }],
            "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0 }, "indices": 1 }] }]
        });
        // accessor 1 作为 indices 被使用 -> 两者都被使用；改为使 accessor 1 成为孤儿。
        gltf.pointer_mut("/meshes/0/primitives/0")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("indices");
        let mut buffers = vec![vec![0u8; 12]];
        remove_unused_elements(&mut gltf, &mut buffers);
        // 孤儿 accessor 1 被移除；accessor 0（POSITION）保留。
        assert_eq!(gltf.pointer("/accessors").and_then(Value::as_array).unwrap().len(), 1);
        assert_eq!(gltf.pointer("/meshes/0/primitives/0/attributes/POSITION").and_then(Value::as_u64), Some(0));
        // bufferView 0 / buffer 0 仍被 accessor 0 使用 -> 不变。
        assert_eq!(gltf.pointer("/bufferViews").and_then(Value::as_array).unwrap().len(), 1);
        assert_eq!(gltf.pointer("/buffers").and_then(Value::as_array).unwrap().len(), 1);
        assert_eq!(buffers.len(), 1);
    }

    #[test]
    fn remove_unused_shifts_higher_accessor_refs() {
        // accessor 0 为孤儿，accessor 1 被 POSITION 使用 -> 移除后 POSITION 变为 0。
        let mut gltf = json!({
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "type": "VEC3", "count": 1 },
                { "bufferView": 0, "componentType": 5126, "type": "VEC3", "count": 1 }
            ],
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": 12 }],
            "buffers": [{ "byteLength": 12 }],
            "meshes": [{ "primitives": [{ "attributes": { "POSITION": 1 } }] }]
        });
        let mut buffers = vec![vec![0u8; 12]];
        remove_unused_elements(&mut gltf, &mut buffers);
        assert_eq!(gltf.pointer("/accessors").and_then(Value::as_array).unwrap().len(), 1);
        assert_eq!(gltf.pointer("/meshes/0/primitives/0/attributes/POSITION").and_then(Value::as_u64), Some(0));
    }

    #[test]
    fn remove_unused_drops_orphan_buffer_and_buffer_view() {
        // bufferView 1 / buffer 1 为孤儿（无 accessor 引用它们）。
        let mut gltf = json!({
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "type": "VEC3", "count": 1 }
            ],
            "bufferViews": [
                { "buffer": 0, "byteOffset": 0, "byteLength": 12 },
                { "buffer": 1, "byteOffset": 0, "byteLength": 8 }
            ],
            "buffers": [{ "byteLength": 12 }, { "byteLength": 8 }],
            "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0 } }] }]
        });
        let mut buffers = vec![vec![0u8; 12], vec![0u8; 8]];
        remove_unused_elements(&mut gltf, &mut buffers);
        assert_eq!(gltf.pointer("/bufferViews").and_then(Value::as_array).unwrap().len(), 1);
        assert_eq!(gltf.pointer("/buffers").and_then(Value::as_array).unwrap().len(), 1);
        assert_eq!(buffers.len(), 1);
        assert_eq!(buffers[0].len(), 12);
    }

    /// 带已解码 buffer 的完整 1.0 → 2.0 升级：POSITION accessor 从二进制 chunk
    /// 计算得到精确的 min/max（M9.2 被推迟的步骤）。
    #[test]
    fn full_upgrade_with_buffers_computes_min_max() {
        let mut gltf = json!({
            "asset": { "version": "1.0" },
            "buffers": { "buf": { "byteLength": 36 } },
            "bufferViews": { "bv": { "buffer": "buf", "byteOffset": 0, "byteLength": 36 } },
            "accessors": { "acc": {
                "bufferView": "bv", "byteOffset": 0,
                "componentType": 5126, "type": "VEC3", "count": 3
            } },
            "meshes": { "mesh": { "primitives": [{ "attributes": { "POSITION": "acc" }, "mode": 4 }] } },
            "nodes": { "node": { "meshes": ["mesh"] } },
            "scenes": { "scene": { "nodes": ["node"] } },
            "scene": "scene"
        });
        let mut buffers = vec![f32_bytes(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0])];
        crate::gltf_upgrade::update_version_with_buffers(
            &mut gltf,
            &crate::gltf_upgrade::UpgradeOptions::default(),
            &mut buffers,
        )
        .unwrap();

        assert_eq!(gltf.pointer("/asset/version").and_then(Value::as_str), Some("2.0"));
        assert_eq!(f64s(gltf.pointer("/accessors/0/min").unwrap()), vec![0.0, 0.0, 0.0]);
        assert_eq!(f64s(gltf.pointer("/accessors/0/max").unwrap()), vec![1.0, 1.0, 0.0]);
    }
}
