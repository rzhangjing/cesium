//! Shared low-level helpers for the glTF 1.0 → 2.0 upgrade chain.
//!
//! Mirrors CesiumJS `packages/engine/Source/Scene/GltfPipeline/`:
//! `ForEach.js`, `addToArray.js`, `addExtensionsUsed.js`,
//! `addExtensionsRequired.js`, `removeExtensionsUsed.js`,
//! `removeExtensionsRequired.js`, `removeExtension.js`, `usesExtension.js`,
//! `numberOfComponentsForType.js`, `getAccessorByteStride.js`, and the numeric
//! constants from `Core/WebGLConstants.js`.
//!
//! The upgrade operates on raw [`serde_json::Value`] rather than the typed
//! [`crate::gltf_model::GltfModel`]: glTF 1.0 stores its top-level collections
//! as object-keyed dictionaries (`"accessors": { "myAccessor": { .. } }`) which
//! the array-based 2.0 model cannot deserialize, so the JSON is upgraded first
//! and only then parsed. Every helper therefore works on untyped JSON and
//! handles both the 1.0 object form and the 2.0 array form, exactly like
//! upstream `ForEach.topLevel` (`Array.isArray` branch).

use serde_json::{Map, Value};
use std::collections::HashMap;

/// WebGL / glTF numeric constants.
///
/// Source of truth: `packages/engine/Source/Core/WebGLConstants.js`. This is a
/// faithful mirror of the full constant enum; only the subset exercised by the
/// glTF 1.0 → 2.0 upgrade path is referenced, hence `#[allow(dead_code)]`.
#[allow(dead_code)]
pub(crate) mod webgl {
    /// `BYTE` (signed 8-bit) component type.
    pub const BYTE: u64 = 0x1400;
    /// `UNSIGNED_BYTE` component type.
    pub const UNSIGNED_BYTE: u64 = 0x1401;
    /// `SHORT` (signed 16-bit) component type.
    pub const SHORT: u64 = 0x1402;
    /// `UNSIGNED_SHORT` component type.
    pub const UNSIGNED_SHORT: u64 = 0x1403;
    /// `INT` (signed 32-bit) component type.
    pub const INT: u64 = 0x1404;
    /// `UNSIGNED_INT` component type.
    pub const UNSIGNED_INT: u64 = 0x1405;
    /// `FLOAT` component type.
    pub const FLOAT: u64 = 0x1406;
    /// `DOUBLE` (64-bit float) component type.
    pub const DOUBLE: u64 = 0x140A;
    /// `TRIANGLES` primitive mode.
    pub const TRIANGLES: u64 = 0x0004;
    /// Blend factor `ZERO`.
    pub const ZERO: u64 = 0;
    /// Blend factor `ONE`.
    pub const ONE: u64 = 1;
    /// Blend factor `SRC_COLOR`.
    pub const SRC_COLOR: u64 = 0x0300;
    /// Blend factor `ONE_MINUS_SRC_COLOR`.
    pub const ONE_MINUS_SRC_COLOR: u64 = 0x0301;
    /// Blend factor `SRC_ALPHA`.
    pub const SRC_ALPHA: u64 = 0x0302;
    /// Blend factor `ONE_MINUS_SRC_ALPHA`.
    pub const ONE_MINUS_SRC_ALPHA: u64 = 0x0303;
    /// Blend factor `DST_ALPHA`.
    pub const DST_ALPHA: u64 = 0x0304;
    /// Blend factor `ONE_MINUS_DST_ALPHA`.
    pub const ONE_MINUS_DST_ALPHA: u64 = 0x0305;
    /// Blend factor `DST_COLOR`.
    pub const DST_COLOR: u64 = 0x0306;
    /// Blend factor `ONE_MINUS_DST_COLOR`.
    pub const ONE_MINUS_DST_COLOR: u64 = 0x0307;
    /// Blend equation `FUNC_ADD`.
    pub const FUNC_ADD: u64 = 0x8006;
    /// Render state `CULL_FACE`.
    pub const CULL_FACE: u64 = 0x0b44;
    /// Render state `BLEND`.
    pub const BLEND: u64 = 0x0be2;
    /// Buffer target `ARRAY_BUFFER`.
    pub const ARRAY_BUFFER: u64 = 0x8892;
    /// Buffer target `ELEMENT_ARRAY_BUFFER`.
    pub const ELEMENT_ARRAY_BUFFER: u64 = 0x8893;
}

/// `numberOfComponentsForType.js`: number of scalar components per element.
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

/// `ComponentDatatype.getSizeInBytes`: byte width of a component type enum.
pub(crate) fn component_size_in_bytes(component_type: u64) -> usize {
    match component_type {
        webgl::BYTE | webgl::UNSIGNED_BYTE => 1,
        webgl::SHORT | webgl::UNSIGNED_SHORT => 2,
        webgl::UNSIGNED_INT | webgl::FLOAT => 4,
        _ => 0,
    }
}

/// `getAccessorByteStride.js`: byte stride of an accessor.
///
/// Uses `bufferView.byteStride` when it is present and positive, otherwise
/// computes `componentSize * numberOfComponentsForType`.
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

/// Reads `gltf[collection][index]` whether the collection is a 2.0 array or a
/// 1.0 object-keyed dictionary keyed by the (stringified) index.
pub(crate) fn index_into<'a>(gltf: &'a Value, collection: &str, index: u64) -> Option<&'a Value> {
    match gltf.get(collection)? {
        Value::Array(arr) => arr.get(index as usize),
        Value::Object(obj) => obj.get(index.to_string().as_str()),
        _ => None,
    }
}

/// `addToArray.js`: append `element`, returning its index. When `check_dup` is
/// set, an existing equal element's index is returned instead of appending.
pub(crate) fn add_to_array(arr: &mut Vec<Value>, element: Value, check_dup: bool) -> usize {
    if check_dup {
        if let Some(idx) = arr.iter().position(|x| *x == element) {
            return idx;
        }
    }
    arr.push(element);
    arr.len() - 1
}

/// `objectToArray.js` (updateVersion.js L291): convert an object-keyed
/// collection into an array, assigning `name` from the key when absent, and
/// returning the `id -> array index` mapping.
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

/// `usesExtension.js`: whether `extensionsUsed` contains `extension`.
pub(crate) fn uses_extension(gltf: &Value, extension: &str) -> bool {
    gltf
        .get("extensionsUsed")
        .and_then(Value::as_array)
        .map(|a| a.iter().any(|x| x.as_str() == Some(extension)))
        .unwrap_or(false)
}

/// `addExtensionsUsed.js`: add `extension` to `extensionsUsed` (deduped).
pub(crate) fn add_extensions_used(gltf: &mut Value, extension: &str) {
    let Some(obj) = gltf.as_object_mut() else { return };
    let slot = obj
        .entry("extensionsUsed")
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Some(arr) = slot.as_array_mut() {
        add_to_array(arr, Value::String(extension.to_string()), true);
    }
}

/// `addExtensionsRequired.js`: add `extension` to `extensionsRequired`
/// (deduped) and to `extensionsUsed`.
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

/// `removeExtensionsRequired.js`: splice `extension` from `extensionsRequired`,
/// deleting the array when it becomes empty.
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

/// `removeExtensionsUsed.js`: splice `extension` from `extensionsUsed` (and
/// `extensionsRequired`), deleting the array when it becomes empty.
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

/// `removeExtension.js`: remove `extension` from `extensionsUsed` /
/// `extensionsRequired` and from every `extensions` object in the tree. The
/// `CESIUM_RTC` technique-uniform semantic fix is mirrored too.
pub(crate) fn remove_extension(gltf: &mut Value, extension: &str) {
    remove_extensions_used(gltf, extension);
    if extension == "CESIUM_RTC" {
        remove_cesium_rtc(gltf);
    }
    remove_extension_and_traverse(gltf, extension);
}

/// `removeCesiumRTC` (removeExtension.js L23): rewrite the `CESIUM_RTC_MODELVIEW`
/// technique uniform semantic to `MODELVIEW`.
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

/// `removeExtensionAndTraverse` (removeExtension.js L33): recursively delete
/// `extensions[extension]` from every plain object in the tree.
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

/// `ForEach.topLevel` (mutable): visit every element of a top-level collection,
/// handling both the 2.0 array form and the 1.0 object-keyed form. The second
/// closure argument is the positional index (meaningful for arrays).
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

/// `ForEach.technique` (mutable): visits `KHR_techniques_webgl.techniques` when
/// that extension is used, otherwise the top-level `techniques`.
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

/// `ForEach.material` (mutable): visits every material (array or object form).
pub(crate) fn for_each_material(gltf: &mut Value, f: &mut impl FnMut(&mut Value)) {
    for_each_top_level_mut(gltf, "materials", &mut |item, _i| f(item));
}

/// `srgbToLinear` (updateVersion.js L1019): convert an sRGB RGBA color to
/// linear space (alpha is preserved verbatim).
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

/// `isVec4` (updateVersion.js L1015): a JSON array of exactly four numbers.
pub(crate) fn is_vec4(value: &Value) -> bool {
    value.as_array().map(|a| a.len() == 4).unwrap_or(false)
}

/// `isTexture` (updateVersion.js L1011): an object with a defined `index`.
pub(crate) fn is_texture(value: &Value) -> bool {
    value.get("index").is_some()
}
