//! Binary-buffer-dependent steps of the glTF 1.0 → 2.0 upgrade chain.
//!
//! M9.1 implemented the JSON-only transforms in [`crate::gltf_upgrade`] and
//! deferred the handful of upstream `updateVersion.js` steps that need the
//! *decoded* binary buffer (`buffer.extras._pipeline.source`). This module
//! lands those deferred steps. They are driven by M9.2 and only run when the
//! caller supplies decoded buffers ([`crate::gltf_upgrade::update_version_with_buffers`]);
//! the JSON-only entry point [`crate::gltf_upgrade::update_version`] passes no
//! buffers and therefore behaves byte-for-byte exactly as in M9.1.
//!
//! Mirrors CesiumJS `packages/engine/Source/Scene/GltfPipeline/`:
//! `findAccessorMinMax.js`, `readAccessorPacked.js`, `getComponentReader.js`,
//! `addBuffer.js`, `updateAccessorComponentTypes.js`, `removeUnusedElements.js`,
//! and the binary branches of `requireByteLength` / `requirePositionAccessorMinMax`
//! / `requireAnimationAccessorMinMax` / `validatePresentAccessorMinMax` in
//! `updateVersion.js`.
//!
//! # Buffer model
//!
//! Upstream stores each buffer's decoded bytes in `buffer.extras._pipeline.source`
//! (a `Buffer` with `.length`, `.byteOffset`, `.buffer`). Here the decoded
//! sources live in a parallel `buffers: &[Vec<u8>]` whose indices align 1:1 with
//! the `gltf.buffers` array (`buffers[i]` is the source of `gltf.buffers[i]`).
//! For a GLB the adapter supplies `buffers = vec![binary_chunk]`, so buffer 0 is
//! the embedded chunk. Because `buffers[i]` already *is* the exact source slice,
//! upstream's `source.byteOffset` term is always `0` in this model.
//!
//! # Precision
//!
//! `Accessor.min` / `Accessor.max` are `Vec<f64>` in the typed model; the binary
//! payload is `f32` (glTF `FLOAT`). Widening `f32 → f64` is lossless, so the
//! domain-f64 invariant holds (no precision is dropped).
//!
//! # Deviations
//!
//! * `removeUnusedElements` implements the **core** reference graph
//!   (accessor / bufferView / buffer). The upstream extension branches
//!   (draco, meshopt, EXT_feature_metadata, EXT_structural_metadata,
//!   EXT_mesh_gpu_instancing, CESIUM_primitive_outline) are skipped: the
//!   1.0 → 2.0 upgrade chain never produces them. See docs/deviations.md.
//! * `findAccessorMinMax` / `readAccessorPacked` reuse the typed readers in
//!   [`crate::gltf_model`] (`read_f32_data` / `read_u16_data` / `read_u32_data`)
//!   for the realistic `FLOAT` / `UNSIGNED_SHORT` / `UNSIGNED_INT` cases and fall
//!   back to a general component reader for the remaining integer types.
//! * Only buffers already decoded by the caller are readable; an external-`uri`
//!   buffer that was not fetched resolves to zeros (out of scope for the GLB
//!   embedded-chunk path this milestone targets).

use serde_json::{json, Value};
use std::collections::HashSet;

use crate::gltf_model::{Accessor, BufferView, ComponentType};
use crate::gltf_upgrade_util::{
    component_size_in_bytes, get_accessor_byte_stride, index_into, number_of_components_for_type,
    webgl,
};

// --------------------------- component reading ----------------------------

/// Reads one component as `f64` at `off` (mirrors `getComponentReader.js`).
/// Returns `None` for an unknown component type or an out-of-range read.
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

/// `readAccessorPacked.js`: the accessor's values in a contiguous
/// element-major array (`values[i * numComp + j]`), read with the accessor's
/// *own* component type. Zeros when the buffer view / buffer is missing.
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
    // source.byteOffset is 0 in this model (buffers[i] is the exact source).
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

/// Primary min/max path: reuse the typed readers in [`crate::gltf_model`]
/// (`read_f32_data` / `read_u16_data` / `read_u32_data`). Returns `None` for
/// component types they do not cover (signed / multi-component integer), which
/// the caller then handles with [`read_accessor_packed`].
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

/// Per-component min/max over an element-major flat array. An empty array
/// (`count == 0`) leaves `min = +inf` / `max = -inf`, matching upstream (the
/// read loop never executes).
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

/// `findAccessorMinMax.js`: the min/max of every component of `accessor`.
///
/// Returns `None` only when the accessor `type` is unknown (0 components). When
/// `bufferView` is undefined the spec mandates zeros, so `(zeros, zeros)` is
/// returned. Values stay in the f64 domain.
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

/// The buffer-level half of `requireByteLength` (updateVersion.js L746–750):
/// `buffer.byteLength = source.length` when absent. The bufferView-level half
/// stays in [`crate::gltf_upgrade::require_byte_length`] (JSON-only, M9.1).
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

// --------------------------- min/max require steps ---------------------------

/// Collects accessor ids whose primitive attribute semantic starts with
/// `semantic` (`ForEach.accessorWithSemantic`, deduped, mesh/primitive/attribute
/// iteration order).
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

/// Mutable lookup of `accessors[id]` (array form post-`objectsToArrays`, object
/// form tolerated).
fn accessor_mut(gltf: &mut Value, id: u64) -> Option<&mut Value> {
    match gltf.get_mut("accessors")? {
        Value::Array(a) => a.get_mut(id as usize),
        Value::Object(o) => o.get_mut(id.to_string().as_str()),
        _ => None,
    }
}

/// Element count of a top-level collection (array or object form).
fn collection_len(gltf: &Value, name: &str) -> usize {
    match gltf.get(name) {
        Some(Value::Array(a)) => a.len(),
        Some(Value::Object(o)) => o.len(),
        _ => 0,
    }
}

/// Sets `accessor.min` / `accessor.max` from the decoded buffer when either is
/// missing (`!defined(min) || !defined(max)` → compute both).
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

/// `requirePositionAccessorMinMax` (updateVersion.js L840).
pub(crate) fn require_position_accessor_min_max(gltf: &mut Value, buffers: &[Vec<u8>]) {
    for id in collect_accessor_ids_with_semantic(gltf, "POSITION") {
        fill_min_max_if_missing(gltf, id, buffers);
    }
}

/// Input accessor ids of every animation sampler (`ForEach.animation` →
/// `ForEach.animationSampler`, `sampler.input`).
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

/// `requireAnimationAccessorMinMax` (updateVersion.js L916).
pub(crate) fn require_animation_accessor_min_max(gltf: &mut Value, buffers: &[Vec<u8>]) {
    for id in animation_sampler_input_ids(gltf) {
        fill_min_max_if_missing(gltf, id, buffers);
    }
}

/// `validatePresentAccessorMinMax` (updateVersion.js L929): for every accessor
/// that already has `min` or `max`, recompute from the buffer and overwrite the
/// present field(s) with the precise value.
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

/// `addBuffer.js`: append `data` as a new buffer + a matching buffer view,
/// returning the new buffer-view id. The decoded bytes are pushed onto the
/// parallel `buffers` vec so index alignment with `gltf.buffers` is preserved.
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

/// `ComponentDatatype.createTypedArray(newType, values)` byte encoding for the
/// two targets `updateAccessorComponentTypes` converts to. Mirrors the JS
/// `ToUint8` / `ToUint16` wrap (truncate toward zero, then modulo 2^width).
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

/// `convertType` (updateAccessorComponentTypes.js L42): repack the accessor's
/// data into `new_type`, store it in a fresh buffer, and repoint the accessor.
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

/// `updateAccessorComponentTypes` (updateVersion.js L980): JOINTS_0 must be
/// `UNSIGNED_BYTE` / `UNSIGNED_SHORT`; WEIGHTS_0 must not be signed.
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

/// Decrements an integer reference `v` when it points past the removed `id`
/// (`Remove.*`'s `if (ref > id) ref--`).
fn dec_if_gt(v: &mut Value, id: usize) {
    if let Some(n) = v.as_u64() {
        if n > id as u64 {
            *v = json!(n - 1);
        }
    }
}

/// `getListOfElementsIdsInUse.accessor` (core graph; extension branches skipped).
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

/// `getListOfElementsIdsInUse.bufferView` (core graph; extension branches skipped).
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

/// `getListOfElementsIdsInUse.buffer` (core graph; extension branches skipped).
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

/// `Remove.accessor`: splice `accessors[id]` and shift every referencing index.
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

/// `Remove.bufferView`: splice `bufferViews[id]` and shift referencing indices.
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

/// `Remove.buffer`: splice `buffers[id]` (and the parallel decoded source) and
/// shift `bufferView.buffer` references.
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

/// `removeUnusedElementsByType` for one type (iterate original indices, splice
/// at the running current index, shift references after each removal).
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
/// (updateVersion.js L837, invoked at the end of `moveByteStrideToBufferView`).
/// Types are processed in `allElementTypes` order: accessor → bufferView →
/// buffer, so each pass sees the references the previous pass produced.
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
        // indices: 5, 2, 9 (UNSIGNED_SHORT SCALAR) -> min 2, max 9
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
        // Both present -> untouched (upstream `!defined(min) || !defined(max)`).
        assert_eq!(f64s(gltf.pointer("/accessors/0/min").unwrap()), vec![-9.0, -9.0, -9.0]);
        assert_eq!(f64s(gltf.pointer("/accessors/0/max").unwrap()), vec![9.0, 9.0, 9.0]);
    }

    #[test]
    fn require_animation_fills_input_min_max() {
        // animation input: time values 0.0, 0.5, 1.0 (FLOAT SCALAR)
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
        // Existing byteLength is preserved.
        let mut gltf2 = json!({ "buffers": [{ "byteLength": 5 }] });
        require_byte_length_buffers(&mut gltf2, &buffers);
        assert_eq!(gltf2.pointer("/buffers/0/byteLength").and_then(Value::as_u64), Some(5));
    }

    #[test]
    fn update_component_types_joints_byte_to_unsigned_byte() {
        // JOINTS_0: BYTE VEC4, one element [0, 1, 2, 3]
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

        // Accessor repointed to a fresh buffer view, componentType UNSIGNED_BYTE.
        assert_eq!(gltf.pointer("/accessors/0/componentType").and_then(Value::as_u64), Some(5121));
        assert_eq!(gltf.pointer("/accessors/0/bufferView").and_then(Value::as_u64), Some(1));
        assert_eq!(gltf.pointer("/accessors/0/byteOffset").and_then(Value::as_u64), Some(0));
        // A new buffer + buffer view were appended, and the decoded source too.
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
        // Already UNSIGNED_SHORT -> no conversion, no new buffer.
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
        // accessor 1 is used as indices -> both used; make accessor 1 orphan instead.
        gltf.pointer_mut("/meshes/0/primitives/0")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("indices");
        let mut buffers = vec![vec![0u8; 12]];
        remove_unused_elements(&mut gltf, &mut buffers);
        // Orphan accessor 1 removed; accessor 0 (POSITION) kept.
        assert_eq!(gltf.pointer("/accessors").and_then(Value::as_array).unwrap().len(), 1);
        assert_eq!(gltf.pointer("/meshes/0/primitives/0/attributes/POSITION").and_then(Value::as_u64), Some(0));
        // bufferView 0 / buffer 0 still used by accessor 0 -> unchanged.
        assert_eq!(gltf.pointer("/bufferViews").and_then(Value::as_array).unwrap().len(), 1);
        assert_eq!(gltf.pointer("/buffers").and_then(Value::as_array).unwrap().len(), 1);
        assert_eq!(buffers.len(), 1);
    }

    #[test]
    fn remove_unused_shifts_higher_accessor_refs() {
        // accessor 0 orphan, accessor 1 used by POSITION -> after removal POSITION becomes 0.
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
        // bufferView 1 / buffer 1 are orphan (no accessor references them).
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

    /// Full 1.0 → 2.0 upgrade with decoded buffers: the POSITION accessor gets
    /// precise min/max computed from the binary chunk (the M9.2 deferred steps).
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
