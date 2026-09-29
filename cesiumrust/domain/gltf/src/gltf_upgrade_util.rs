//! glTF 1.0 → 2.0 升级链共享的底层辅助函数。
//!
//! 镜像 CesiumJS `packages/engine/Source/Scene/GltfPipeline/`：
//! `ForEach.js`、`addToArray.js`、`addExtensionsUsed.js`、
//! `addExtensionsRequired.js`、`removeExtensionsUsed.js`、
//! `removeExtensionsRequired.js`、`removeExtension.js`、`usesExtension.js`、
//! `numberOfComponentsForType.js`、`getAccessorByteStride.js`，以及来自
//! `Core/WebGLConstants.js` 的数值常量。
//!
//! 升级作用于原始 [`serde_json::Value`] 而非强类型的
//! [`crate::gltf_model::GltfModel`]：glTF 1.0 将其顶层集合
//! 存储为对象键字典（`"accessors": { "myAccessor": { .. } }`），
//! 基于数组的 2.0 model 无法反序列化它，因此先升级 JSON，
//! 然后才解析。因此每个辅助函数都在无类型 JSON 上工作，
//! 同时处理 1.0 对象形式和 2.0 数组形式，与上游
//! `ForEach.topLevel`（`Array.isArray` 分支）完全一致。

use serde_json::{Map, Value};
use std::collections::HashMap;

/// WebGL / glTF 数值常量。
///
/// 真实来源：`packages/engine/Source/Core/WebGLConstants.js`。这是完整
/// 常量枚举的忠实镜像；仅引用了 glTF 1.0 → 2.0 升级路径
/// 所使用的那部分子集，因此加 `#[allow(dead_code)]`。
#[allow(dead_code)]
pub(crate) mod webgl {
    /// `BYTE`（有符号 8 位）分量类型。
    pub const BYTE: u64 = 0x1400;
    /// `UNSIGNED_BYTE` 分量类型。
    pub const UNSIGNED_BYTE: u64 = 0x1401;
    /// `SHORT`（有符号 16 位）分量类型。
    pub const SHORT: u64 = 0x1402;
    /// `UNSIGNED_SHORT` 分量类型。
    pub const UNSIGNED_SHORT: u64 = 0x1403;
    /// `INT`（有符号 32 位）分量类型。
    pub const INT: u64 = 0x1404;
    /// `UNSIGNED_INT` 分量类型。
    pub const UNSIGNED_INT: u64 = 0x1405;
    /// `FLOAT` 分量类型。
    pub const FLOAT: u64 = 0x1406;
    /// `DOUBLE`（64 位浮点）分量类型。
    pub const DOUBLE: u64 = 0x140A;
    /// `TRIANGLES` primitive 模式。
    pub const TRIANGLES: u64 = 0x0004;
    /// 混合因子 `ZERO`。
    pub const ZERO: u64 = 0;
    /// 混合因子 `ONE`。
    pub const ONE: u64 = 1;
    /// 混合因子 `SRC_COLOR`。
    pub const SRC_COLOR: u64 = 0x0300;
    /// 混合因子 `ONE_MINUS_SRC_COLOR`。
    pub const ONE_MINUS_SRC_COLOR: u64 = 0x0301;
    /// 混合因子 `SRC_ALPHA`。
    pub const SRC_ALPHA: u64 = 0x0302;
    /// 混合因子 `ONE_MINUS_SRC_ALPHA`。
    pub const ONE_MINUS_SRC_ALPHA: u64 = 0x0303;
    /// 混合因子 `DST_ALPHA`。
    pub const DST_ALPHA: u64 = 0x0304;
    /// 混合因子 `ONE_MINUS_DST_ALPHA`。
    pub const ONE_MINUS_DST_ALPHA: u64 = 0x0305;
    /// 混合因子 `DST_COLOR`。
    pub const DST_COLOR: u64 = 0x0306;
    /// 混合因子 `ONE_MINUS_DST_COLOR`。
    pub const ONE_MINUS_DST_COLOR: u64 = 0x0307;
    /// 混合方程 `FUNC_ADD`。
    pub const FUNC_ADD: u64 = 0x8006;
    /// 渲染状态 `CULL_FACE`。
    pub const CULL_FACE: u64 = 0x0b44;
    /// 渲染状态 `BLEND`。
    pub const BLEND: u64 = 0x0be2;
    /// buffer 目标 `ARRAY_BUFFER`。
    pub const ARRAY_BUFFER: u64 = 0x8892;
    /// buffer 目标 `ELEMENT_ARRAY_BUFFER`。
    pub const ELEMENT_ARRAY_BUFFER: u64 = 0x8893;
}

/// `numberOfComponentsForType.js`：每个元素的标量分量数。
pub(crate) fn number_of_components_for_type(gl_type: &str) -> usize {
    match gl_type {
        "SCALAR" => 1,
        "VEC2" => 2,
        "VEC3" => 3,
        "VEC4" | "MAT2" => 4,
        "MAT3" => 9,
        "MAT4" => 16,
        _ => 0,
    }
}

/// `ComponentDatatype.getSizeInBytes`：分量类型枚举的字节宽度。
pub(crate) fn component_size_in_bytes(component_type: u64) -> usize {
    match component_type {
        webgl::BYTE | webgl::UNSIGNED_BYTE => 1,
        webgl::SHORT | webgl::UNSIGNED_SHORT => 2,
        webgl::UNSIGNED_INT | webgl::FLOAT => 4,
        _ => 0,
    }
}

/// `getAccessorByteStride.js`：accessor 的字节 stride。
///
/// 当 `bufferView.byteStride` 存在且为正时使用它，否则
/// 计算 `componentSize * numberOfComponentsForType`。
pub(crate) fn get_accessor_byte_stride(gltf: &Value, accessor: &Value) -> usize {
    if let Some(bv_id) = accessor.get("bufferView").and_then(Value::as_u64) {
        if let Some(bv) = index_into(gltf, "bufferViews", bv_id) {
            if let Some(stride) = bv.get("byteStride").and_then(Value::as_u64) {
                if stride > 0 {
                    return stride as usize;
                }
            }
        }
    }
    let component_type = accessor.get("componentType").and_then(Value::as_u64).unwrap_or(0);
    let gl_type = accessor.get("type").and_then(Value::as_str).unwrap_or("");
    component_size_in_bytes(component_type) * number_of_components_for_type(gl_type)
}

/// 读取 `gltf[collection][index]`，无论该集合是 2.0 数组还是
/// 以（字符串化的）索引为键的 1.0 对象键字典。
pub(crate) fn index_into<'a>(gltf: &'a Value, collection: &str, index: u64) -> Option<&'a Value> {
    match gltf.get(collection)? {
        Value::Array(arr) => arr.get(index as usize),
        Value::Object(obj) => obj.get(index.to_string().as_str()),
        _ => None,
    }
}

/// `addToArray.js`：追加 `element`，返回其索引。当设置了 `check_dup` 时，
/// 返回已存在的相等元素的索引而非追加。
pub(crate) fn add_to_array(arr: &mut Vec<Value>, element: Value, check_dup: bool) -> usize {
    if check_dup {
        if let Some(idx) = arr.iter().position(|x| *x == element) {
            return idx;
        }
    }
    arr.push(element);
    arr.len() - 1
}

/// `objectToArray.js`（updateVersion.js L291）：将一个对象键集合
/// 转换为数组，在缺失时从键赋值 `name`，并
/// 返回 `id -> array index` 映射。
pub(crate) fn object_to_array(obj: Map<String, Value>) -> (Vec<Value>, HashMap<String, usize>) {
    let mut arr = Vec::with_capacity(obj.len());
    let mut mapping = HashMap::with_capacity(obj.len());
    for (id, mut value) in obj {
        let index = arr.len();
        mapping.insert(id.clone(), index);
        if let Some(o) = value.as_object_mut() {
            if !o.contains_key("name") {
                o.insert("name".to_string(), Value::String(id));
            }
        }
        arr.push(value);
    }
    (arr, mapping)
}

/// `usesExtension.js`：`extensionsUsed` 是否包含 `extension`。
pub(crate) fn uses_extension(gltf: &Value, extension: &str) -> bool {
    gltf
        .get("extensionsUsed")
        .and_then(Value::as_array)
        .map(|a| a.iter().any(|x| x.as_str() == Some(extension)))
        .unwrap_or(false)
}

/// `addExtensionsUsed.js`：将 `extension` 加入 `extensionsUsed`（去重）。
pub(crate) fn add_extensions_used(gltf: &mut Value, extension: &str) {
    let Some(obj) = gltf.as_object_mut() else { return };
    let slot = obj
        .entry("extensionsUsed")
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Some(arr) = slot.as_array_mut() {
        add_to_array(arr, Value::String(extension.to_string()), true);
    }
}

/// `addExtensionsRequired.js`：将 `extension` 加入 `extensionsRequired`
/// （去重）以及 `extensionsUsed`。
pub(crate) fn add_extensions_required(gltf: &mut Value, extension: &str) {
    if let Some(obj) = gltf.as_object_mut() {
        let slot = obj
            .entry("extensionsRequired")
            .or_insert_with(|| Value::Array(Vec::new()));
        if let Some(arr) = slot.as_array_mut() {
            add_to_array(arr, Value::String(extension.to_string()), true);
        }
    }
    add_extensions_used(gltf, extension);
}

/// `removeExtensionsRequired.js`：从 `extensionsRequired` 剪辑掉 `extension`，
/// 当数组变空时删除它。
pub(crate) fn remove_extensions_required(gltf: &mut Value, extension: &str) {
    let Some(obj) = gltf.as_object_mut() else { return };
    let mut emptied = false;
    if let Some(arr) = obj.get_mut("extensionsRequired").and_then(Value::as_array_mut) {
        if let Some(idx) = arr.iter().position(|x| x.as_str() == Some(extension)) {
            arr.remove(idx);
        }
        emptied = arr.is_empty();
    }
    if emptied {
        obj.remove("extensionsRequired");
    }
}

/// `removeExtensionsUsed.js`：从 `extensionsUsed`（以及
/// `extensionsRequired`）剪辑掉 `extension`，当数组变空时删除它。
pub(crate) fn remove_extensions_used(gltf: &mut Value, extension: &str) {
    let Some(obj) = gltf.as_object_mut() else { return };
    let mut emptied = false;
    let present = obj.get("extensionsUsed").and_then(Value::as_array).is_some();
    if let Some(arr) = obj.get_mut("extensionsUsed").and_then(Value::as_array_mut) {
        if let Some(idx) = arr.iter().position(|x| x.as_str() == Some(extension)) {
            arr.remove(idx);
        }
        emptied = arr.is_empty();
    }
    if present {
        remove_extensions_required(gltf, extension);
        if emptied {
            if let Some(obj) = gltf.as_object_mut() {
                obj.remove("extensionsUsed");
            }
        }
    }
}

/// `removeExtension.js`：从 `extensionsUsed` /
/// `extensionsRequired` 以及树中每个 `extensions` 对象中移除 `extension`。也
/// 镜像了 `CESIUM_RTC` technique-uniform 语义修正。
pub(crate) fn remove_extension(gltf: &mut Value, extension: &str) {
    remove_extensions_used(gltf, extension);
    if extension == "CESIUM_RTC" {
        remove_cesium_rtc(gltf);
    }
    remove_extension_and_traverse(gltf, extension);
}

/// `removeCesiumRTC`（removeExtension.js L23）：将 `CESIUM_RTC_MODELVIEW`
/// technique uniform 语义重写为 `MODELVIEW`。
fn remove_cesium_rtc(gltf: &mut Value) {
    for_each_technique(gltf, &mut |technique| {
        if let Some(uniforms) = technique.get_mut("uniforms").and_then(Value::as_object_mut) {
            for (_name, uniform) in uniforms.iter_mut() {
                if let Some(obj) = uniform.as_object_mut() {
                    if obj.get("semantic").and_then(Value::as_str) == Some("CESIUM_RTC_MODELVIEW") {
                        obj.insert("semantic".to_string(), Value::String("MODELVIEW".to_string()));
                    }
                }
            }
        }
    });
}

/// `removeExtensionAndTraverse`（removeExtension.js L33）：递归地从树中
/// 每个普通对象删除 `extensions[extension]`。
fn remove_extension_and_traverse(value: &mut Value, extension: &str) {
    match value {
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                remove_extension_and_traverse(item, extension);
            }
        }
        Value::Object(obj) => {
            let mut emptied = false;
            if let Some(exts) = obj.get_mut("extensions").and_then(Value::as_object_mut) {
                exts.remove(extension);
                emptied = exts.is_empty();
            }
            if emptied {
                obj.remove("extensions");
            }
            for (_key, child) in obj.iter_mut() {
                remove_extension_and_traverse(child, extension);
            }
        }
        _ => {}
    }
}

/// `ForEach.topLevel`（可变）：访问顶层集合的每个元素，
/// 同时处理 2.0 数组形式和 1.0 对象键形式。第二个
/// 闭包参数是位置索引（对数组有意义）。
pub(crate) fn for_each_top_level_mut(gltf: &mut Value, name: &str, f: &mut impl FnMut(&mut Value, usize)) {
    let mut i = 0;
    if let Some(coll) = gltf.get_mut(name) {
        match coll {
            Value::Array(arr) => {
                for item in arr.iter_mut() {
                    f(item, i);
                    i += 1;
                }
            }
            Value::Object(obj) => {
                for (_key, item) in obj.iter_mut() {
                    f(item, i);
                    i += 1;
                }
            }
            _ => {}
        }
    }
}

/// `ForEach.technique`（可变）：当使用了
/// 该扩展时访问 `KHR_techniques_webgl.techniques`，否则访问顶层 `techniques`。
pub(crate) fn for_each_technique(gltf: &mut Value, f: &mut impl FnMut(&mut Value)) {
    if uses_extension(gltf, "KHR_techniques_webgl") {
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
    for_each_top_level_mut(gltf, "techniques", &mut |item, _i| f(item));
}

/// `ForEach.material`（可变）：访问每个 material（数组或对象形式）。
pub(crate) fn for_each_material(gltf: &mut Value, f: &mut impl FnMut(&mut Value)) {
    for_each_top_level_mut(gltf, "materials", &mut |item, _i| f(item));
}

/// `srgbToLinear`（updateVersion.js L1019）：将一个 sRGB RGBA 颜色转换为
/// 线性空间（alpha 逐字保留）。
pub(crate) fn srgb_to_linear(srgb: &[f64]) -> Vec<f64> {
    let mut linear = vec![0.0f64; srgb.len()];
    if srgb.len() == 4 {
        linear[3] = srgb[3];
    }
    for i in 0..srgb.len().min(3) {
        let c = srgb[i];
        linear[i] = if c <= 0.04045 {
            c * 0.077_399_380_804_953_56
        } else {
            ((c + 0.055) * 0.947_867_298_578_199_1).powf(2.4)
        };
    }
    linear
}

/// `isVec4`（updateVersion.js L1015）：一个恰好包含四个数字的 JSON 数组。
pub(crate) fn is_vec4(value: &Value) -> bool {
    value.as_array().map(|a| a.len() == 4).unwrap_or(false)
}

/// `isTexture`（updateVersion.js L1011）：一个带有已定义 `index` 的对象。
pub(crate) fn is_texture(value: &Value) -> bool {
    value.get("index").is_some()
}
