//! Ported from `packages/engine/Source/Scene/GltfPipeline/ForEach.js`.
//!
//! Contains traversal functions for processing elements of the glTF
//! hierarchy. The JS `ForEach` constructor is represented by this module.
//!
//! DEVIATION: the JS handlers receive `(object, id)` and may return any
//! defined value to short-circuit; the Rust handlers return `Option<T>` and
//! the traversal returns the first `Some(..)` (the JS `ForEach.object`
//! early-out behavior).
//!
//! DEVIATION: legacy glTF 1.0 object-form collections are iterated in
//! `serde_json::Map` order (sorted keys without the `preserve_order`
//! feature), whereas JavaScript iterates insertion order.

use serde_json::Value;

use crate::gltf_pipeline::uses_extension::uses_extension;

/// Fallback for glTF 1.0: iterates an object keyed by element id.
pub fn object_legacy<T>(
    objects: &mut Value,
    mut handler: impl FnMut(&mut Value, String) -> Option<T>,
) -> Option<T> {
    let keys: Vec<String> = objects
        .as_object()
        .map(|map| map.keys().cloned().collect())
        .unwrap_or_default();
    for key in keys {
        if let Some(object) = objects.get_mut(&key) {
            if !object.is_null() {
                if let Some(value) = handler(object, key.clone()) {
                    return Some(value);
                }
            }
        }
    }
    None
}

/// Iterates an array of objects; the handler receives `(object, index)`.
pub fn object<T>(
    array_of_objects: &mut Value,
    mut handler: impl FnMut(&mut Value, usize) -> Option<T>,
) -> Option<T> {
    let length = array_of_objects.as_array().map(|list| list.len())?;
    for index in 0..length {
        let object = &mut array_of_objects[index];
        if !object.is_null() {
            if let Some(value) = handler(object, index) {
                return Some(value);
            }
        }
    }
    None
}

/// Supports glTF 1.0 and 2.0: iterates `gltf[name]`, dispatching to
/// [`object_legacy`] for object-form (glTF 1.0) properties and [`object`]
/// for arrays. The handler receives the element id stringified (numbers
/// become their decimal string form, mirroring JavaScript key coercion).
pub fn top_level<T>(
    gltf: &mut Value,
    name: &str,
    mut handler: impl FnMut(&mut Value, String) -> Option<T>,
) -> Option<T> {
    let legacy = matches!(gltf.get(name), Some(property) if !property.is_null() && !property.is_array());
    if legacy {
        let property = gltf.get_mut(name).expect("checked above");
        return object_legacy(property, handler);
    }

    let length = gltf
        .get(name)
        .and_then(|property| property.as_array())
        .map(|list| list.len())
        .unwrap_or(0);
    let property = gltf.get_mut(name)?;
    let list = property.as_array_mut()?;
    for index in 0..length {
        let object = &mut list[index];
        if !object.is_null() {
            if let Some(value) = handler(object, index.to_string()) {
                return Some(value);
            }
        }
    }
    None
}

pub fn accessor<T>(gltf: &mut Value, handler: impl FnMut(&mut Value, String) -> Option<T>) -> Option<T> {
    top_level(gltf, "accessors", handler)
}

/// Collects accessor ids referenced by primitive attributes and morph
/// targets whose semantic starts with `semantic`, deduplicated in traversal
/// order (the shared `visited` bookkeeping of
/// `ForEach.accessorWithSemantic`).
fn collect_accessor_ids(gltf: &Value, semantic: Option<&str>) -> Vec<usize> {
    let mut visited = vec![false; gltf.get("accessors").and_then(|a| a.as_array()).map(|a| a.len()).unwrap_or(0) + 1];
    let mut ids = Vec::new();
    let mut record = |accessor_id: usize| {
        if accessor_id >= visited.len() {
            visited.resize(accessor_id + 1, false);
        }
        if !visited[accessor_id] {
            visited[accessor_id] = true;
            ids.push(accessor_id);
        }
    };

    if let Some(meshes) = gltf.get("meshes").and_then(|meshes| meshes.as_array()) {
        for mesh in meshes {
            if let Some(primitives) = mesh.get("primitives").and_then(|p| p.as_array()) {
                for primitive in primitives {
                    if let Some(attributes) =
                        primitive.get("attributes").and_then(|a| a.as_object())
                    {
                        for (attribute_semantic, accessor_value) in attributes {
                            if semantic.map_or(true, |s| attribute_semantic.starts_with(s)) {
                                if let Some(accessor_id) = accessor_value.as_u64() {
                                    record(accessor_id as usize);
                                }
                            }
                        }
                    }
                    if let Some(targets) = primitive.get("targets").and_then(|t| t.as_array()) {
                        for target in targets {
                            if let Some(target_attributes) = target.as_object() {
                                for (attribute_semantic, accessor_value) in target_attributes {
                                    if semantic.map_or(true, |s| attribute_semantic.starts_with(s)) {
                                        if let Some(accessor_id) = accessor_value.as_u64() {
                                            record(accessor_id as usize);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    ids
}

/// Returns the accessor ids referenced by attributes whose semantic starts
/// with `semantic` (the collection phase of [`accessor_with_semantic`],
/// exposed for callers that need to release the borrow on the glTF between
/// reads and writes).
pub fn accessor_ids_with_semantic(gltf: &Value, semantic: &str) -> Vec<usize> {
    collect_accessor_ids(gltf, Some(semantic))
}

/// Calls `handler(accessorId)` for each accessor used by a mesh primitive
/// attribute (or morph target attribute) whose semantic starts with
/// `semantic`, each accessor visited only once.
///
/// DEVIATION: the ids are collected up front (the JS interleaves collection
/// and handler invocation); observable behavior is identical as long as the
/// handler does not rewrite primitive attribute mappings, which no caller
/// does.
pub fn accessor_with_semantic<T>(
    gltf: &mut Value,
    semantic: &str,
    mut handler: impl FnMut(usize) -> Option<T>,
) -> Option<T> {
    let ids = collect_accessor_ids(gltf, Some(semantic));
    for accessor_id in ids {
        if let Some(value) = handler(accessor_id) {
            return Some(value);
        }
    }
    None
}

/// Calls `handler(accessorId)` for each accessor containing vertex
/// attribute data (primitive attributes and morph targets), each accessor
/// visited only once.
pub fn accessor_containing_vertex_attribute_data<T>(
    gltf: &mut Value,
    mut handler: impl FnMut(usize) -> Option<T>,
) -> Option<T> {
    let ids = collect_accessor_ids(gltf, None);
    for accessor_id in ids {
        if let Some(value) = handler(accessor_id) {
            return Some(value);
        }
    }
    None
}

/// Accessor ids containing vertex attribute data (the collection phase of
/// [`accessor_containing_vertex_attribute_data`], exposed for callers that
/// need to release the borrow on the glTF between reads and writes).
pub fn vertex_attribute_accessor_ids(gltf: &Value) -> Vec<usize> {
    collect_accessor_ids(gltf, None)
}

/// Calls `handler(accessorId)` for each accessor containing index data,
/// each accessor visited only once.
pub fn accessor_containing_index_data<T>(
    gltf: &mut Value,
    mut handler: impl FnMut(usize) -> Option<T>,
) -> Option<T> {
    let ids = index_data_accessor_ids(gltf);
    for accessor_id in ids {
        if let Some(value) = handler(accessor_id) {
            return Some(value);
        }
    }
    None
}

/// Accessor ids containing index data (the collection phase of
/// [`accessor_containing_index_data`], exposed for callers that need to
/// release the borrow on the glTF between reads and writes).
pub fn index_data_accessor_ids(gltf: &Value) -> Vec<usize> {
    let mut visited = std::collections::HashSet::new();
    let mut ids = Vec::new();
    if let Some(meshes) = gltf.get("meshes").and_then(|meshes| meshes.as_array()) {
        for mesh in meshes {
            if let Some(primitives) = mesh.get("primitives").and_then(|p| p.as_array()) {
                for primitive in primitives {
                    if let Some(indices) =
                        primitive.get("indices").and_then(|indices| indices.as_u64())
                    {
                        if visited.insert(indices) {
                            ids.push(indices as usize);
                        }
                    }
                }
            }
        }
    }
    ids
}

pub fn animation<T>(gltf: &mut Value, handler: impl FnMut(&mut Value, String) -> Option<T>) -> Option<T> {
    top_level(gltf, "animations", handler)
}

pub fn animation_channel<T>(
    animation: &mut Value,
    handler: impl FnMut(&mut Value, usize) -> Option<T>,
) -> Option<T> {
    object(&mut animation["channels"], handler)
}

pub fn animation_sampler<T>(
    animation: &mut Value,
    handler: impl FnMut(&mut Value, usize) -> Option<T>,
) -> Option<T> {
    object(&mut animation["samplers"], handler)
}

pub fn buffer<T>(gltf: &mut Value, handler: impl FnMut(&mut Value, String) -> Option<T>) -> Option<T> {
    top_level(gltf, "buffers", handler)
}

pub fn buffer_view<T>(gltf: &mut Value, handler: impl FnMut(&mut Value, String) -> Option<T>) -> Option<T> {
    top_level(gltf, "bufferViews", handler)
}

pub fn camera<T>(gltf: &mut Value, handler: impl FnMut(&mut Value, String) -> Option<T>) -> Option<T> {
    top_level(gltf, "cameras", handler)
}

pub fn image<T>(gltf: &mut Value, handler: impl FnMut(&mut Value, String) -> Option<T>) -> Option<T> {
    top_level(gltf, "images", handler)
}

pub fn material<T>(gltf: &mut Value, handler: impl FnMut(&mut Value, String) -> Option<T>) -> Option<T> {
    top_level(gltf, "materials", handler)
}

/// Iterates the values of a material (`material.values`, or the
/// `KHR_techniques_webgl` extension values when present). The handler
/// receives `(value, name)`.
pub fn material_value<T>(
    material: &mut Value,
    mut handler: impl FnMut(&mut Value, String) -> Option<T>,
) -> Option<T> {
    let has_techniques_values = material
        .get("extensions")
        .and_then(|extensions| extensions.get("KHR_techniques_webgl"))
        .and_then(|techniques| techniques.get("values"))
        .is_some_and(|values| !values.is_null());
    let values = if has_techniques_values {
        material.pointer_mut("/extensions/KHR_techniques_webgl/values")?
    } else {
        material.get_mut("values")?
    };

    let keys: Vec<String> = values
        .as_object()
        .map(|map| map.keys().cloned().collect())
        .unwrap_or_default();
    for name in keys {
        if let Some(value) = values.get_mut(&name) {
            if let Some(result) = handler(value, name) {
                return Some(result);
            }
        }
    }
    None
}

pub fn mesh<T>(gltf: &mut Value, handler: impl FnMut(&mut Value, String) -> Option<T>) -> Option<T> {
    top_level(gltf, "meshes", handler)
}

pub fn mesh_primitive<T>(
    mesh: &mut Value,
    mut handler: impl FnMut(&mut Value, usize) -> Option<T>,
) -> Option<T> {
    let length = mesh
        .get("primitives")
        .and_then(|primitives| primitives.as_array())
        .map(|primitives| primitives.len())
        .unwrap_or(0);
    let primitives = mesh.get_mut("primitives")?;
    let list = primitives.as_array_mut()?;
    for index in 0..length {
        let primitive = &mut list[index];
        if let Some(value) = handler(primitive, index) {
            return Some(value);
        }
    }
    None
}

/// Iterates primitive attributes; the handler receives
/// `(accessorId, semantic)` where `accessorId` is the mutable JSON number
/// value (callers may rewrite it in place).
pub fn mesh_primitive_attribute<T>(
    primitive: &mut Value,
    mut handler: impl FnMut(&mut Value, String) -> Option<T>,
) -> Option<T> {
    let keys: Vec<String> = primitive
        .get("attributes")
        .and_then(|attributes| attributes.as_object())
        .map(|attributes| attributes.keys().cloned().collect())
        .unwrap_or_default();
    let Some(attributes) = primitive.get_mut("attributes") else {
        return None;
    };
    for semantic in keys {
        if let Some(accessor_id) = attributes.get_mut(&semantic) {
            if let Some(value) = handler(accessor_id, semantic) {
                return Some(value);
            }
        }
    }
    None
}

pub fn mesh_primitive_target<T>(
    primitive: &mut Value,
    mut handler: impl FnMut(&mut Value, usize) -> Option<T>,
) -> Option<T> {
    let length = primitive
        .get("targets")
        .and_then(|targets| targets.as_array())
        .map(|targets| targets.len())
        .unwrap_or(0);
    let Some(targets) = primitive.get_mut("targets") else {
        return None;
    };
    let list = targets.as_array_mut()?;
    for index in 0..length {
        let target = &mut list[index];
        if let Some(value) = handler(target, index) {
            return Some(value);
        }
    }
    None
}

pub fn mesh_primitive_target_attribute<T>(
    target: &mut Value,
    mut handler: impl FnMut(&mut Value, String) -> Option<T>,
) -> Option<T> {
    let keys: Vec<String> = target
        .as_object()
        .map(|map| map.keys().cloned().collect())
        .unwrap_or_default();
    for semantic in keys {
        if let Some(accessor_id) = target.get_mut(&semantic) {
            if let Some(value) = handler(accessor_id, semantic) {
                return Some(value);
            }
        }
    }
    None
}

pub fn node<T>(gltf: &mut Value, handler: impl FnMut(&mut Value, String) -> Option<T>) -> Option<T> {
    top_level(gltf, "nodes", handler)
}

fn node_in_tree_impl<T>(
    gltf: &mut Value,
    node_ids: &[usize],
    handler: &mut impl FnMut(&mut Value, usize) -> Option<T>,
) -> Option<T> {
    let nodes_length = gltf
        .get("nodes")
        .and_then(|nodes| nodes.as_array())
        .map(|nodes| nodes.len())
        .unwrap_or(0);
    for &node_id in node_ids {
        if node_id >= nodes_length {
            continue;
        }
        // Take the node out so the handler (and the recursion) can also
        // access `gltf`; restored on every exit path.
        let mut node = gltf["nodes"][node_id].take();
        if node.is_null() {
            continue;
        }

        let result = handler(&mut node, node_id);
        if result.is_some() {
            gltf["nodes"][node_id] = node;
            return result;
        }

        let children: Vec<usize> = node
            .get("children")
            .and_then(|children| children.as_array())
            .map(|children| {
                children
                    .iter()
                    .filter_map(|child| child.as_u64().map(|id| id as usize))
                    .collect()
            })
            .unwrap_or_default();
        if !children.is_empty() {
            let result = node_in_tree_impl(gltf, &children, handler);
            if result.is_some() {
                gltf["nodes"][node_id] = node;
                return result;
            }
        }
        gltf["nodes"][node_id] = node;
    }
    None
}

/// Depth-first traversal of the given node trees; the handler receives
/// `(node, nodeId)`.
pub fn node_in_tree<T>(
    gltf: &mut Value,
    node_ids: &[usize],
    mut handler: impl FnMut(&mut Value, usize) -> Option<T>,
) -> Option<T> {
    node_in_tree_impl(gltf, node_ids, &mut handler)
}

/// Traverses all nodes of the scene at `scene_id` (the JS takes the scene
/// object; the Rust port takes its index to avoid aliasing `gltf`).
pub fn node_in_scene<T>(
    gltf: &mut Value,
    scene_id: usize,
    handler: impl FnMut(&mut Value, usize) -> Option<T>,
) -> Option<T> {
    let scene_node_ids: Vec<usize> = gltf
        .get("scenes")
        .and_then(|scenes| scenes.get(scene_id))
        .and_then(|scene| scene.get("nodes"))
        .and_then(|nodes| nodes.as_array())
        .map(|nodes| {
            nodes
                .iter()
                .filter_map(|node| node.as_u64().map(|id| id as usize))
                .collect()
        })
        .unwrap_or_default();
    if scene_node_ids.is_empty() {
        return None;
    }
    node_in_tree(gltf, &scene_node_ids, handler)
}

pub fn program<T>(gltf: &mut Value, mut handler: impl FnMut(&mut Value, String) -> Option<T>) -> Option<T> {
    if uses_extension(gltf, "KHR_techniques_webgl") {
        let Some(programs) = gltf
            .get_mut("extensions")
            .and_then(|extensions| extensions.get_mut("KHR_techniques_webgl"))
            .and_then(|techniques| techniques.get_mut("programs"))
        else {
            return None;
        };
        return object(programs, |value, index| handler(value, index.to_string()));
    }

    top_level(gltf, "programs", handler)
}

pub fn sampler<T>(gltf: &mut Value, handler: impl FnMut(&mut Value, String) -> Option<T>) -> Option<T> {
    top_level(gltf, "samplers", handler)
}

pub fn scene<T>(gltf: &mut Value, handler: impl FnMut(&mut Value, String) -> Option<T>) -> Option<T> {
    top_level(gltf, "scenes", handler)
}

pub fn shader<T>(gltf: &mut Value, mut handler: impl FnMut(&mut Value, String) -> Option<T>) -> Option<T> {
    if uses_extension(gltf, "KHR_techniques_webgl") {
        let Some(shaders) = gltf
            .get_mut("extensions")
            .and_then(|extensions| extensions.get_mut("KHR_techniques_webgl"))
            .and_then(|techniques| techniques.get_mut("shaders"))
        else {
            return None;
        };
        return object(shaders, |value, index| handler(value, index.to_string()));
    }

    top_level(gltf, "shaders", handler)
}

pub fn skin<T>(gltf: &mut Value, handler: impl FnMut(&mut Value, String) -> Option<T>) -> Option<T> {
    top_level(gltf, "skins", handler)
}

/// Iterates the joints of a skin; the handler receives the mutable joint
/// node id value.
pub fn skin_joint<T>(
    skin: &mut Value,
    mut handler: impl FnMut(&mut Value) -> Option<T>,
) -> Option<T> {
    let length = skin
        .get("joints")
        .and_then(|joints| joints.as_array())
        .map(|joints| joints.len())
        .unwrap_or(0);
    let Some(joints) = skin.get_mut("joints") else {
        return None;
    };
    let list = joints.as_array_mut()?;
    for index in 0..length {
        let joint = &mut list[index];
        if let Some(value) = handler(joint) {
            return Some(value);
        }
    }
    None
}

pub fn technique_attribute<T>(
    technique: &mut Value,
    mut handler: impl FnMut(&mut Value, String) -> Option<T>,
) -> Option<T> {
    let keys: Vec<String> = technique
        .get("attributes")
        .and_then(|attributes| attributes.as_object())
        .map(|attributes| attributes.keys().cloned().collect())
        .unwrap_or_default();
    let Some(attributes) = technique.get_mut("attributes") else {
        return None;
    };
    for attribute_name in keys {
        if let Some(value) = attributes.get_mut(&attribute_name) {
            if let Some(result) = handler(value, attribute_name) {
                return Some(result);
            }
        }
    }
    None
}

pub fn technique_uniform<T>(
    technique: &mut Value,
    mut handler: impl FnMut(&mut Value, String) -> Option<T>,
) -> Option<T> {
    let keys: Vec<String> = technique
        .get("uniforms")
        .and_then(|uniforms| uniforms.as_object())
        .map(|uniforms| uniforms.keys().cloned().collect())
        .unwrap_or_default();
    let Some(uniforms) = technique.get_mut("uniforms") else {
        return None;
    };
    for uniform_name in keys {
        if let Some(uniform) = uniforms.get_mut(&uniform_name) {
            if let Some(result) = handler(uniform, uniform_name) {
                return Some(result);
            }
        }
    }
    None
}

pub fn technique_parameter<T>(
    technique: &mut Value,
    mut handler: impl FnMut(&mut Value, String) -> Option<T>,
) -> Option<T> {
    let keys: Vec<String> = technique
        .get("parameters")
        .and_then(|parameters| parameters.as_object())
        .map(|parameters| parameters.keys().cloned().collect())
        .unwrap_or_default();
    let Some(parameters) = technique.get_mut("parameters") else {
        return None;
    };
    for parameter_name in keys {
        if let Some(parameter) = parameters.get_mut(&parameter_name) {
            if let Some(result) = handler(parameter, parameter_name) {
                return Some(result);
            }
        }
    }
    None
}

pub fn technique<T>(gltf: &mut Value, mut handler: impl FnMut(&mut Value, String) -> Option<T>) -> Option<T> {
    if uses_extension(gltf, "KHR_techniques_webgl") {
        let Some(techniques) = gltf
            .get_mut("extensions")
            .and_then(|extensions| extensions.get_mut("KHR_techniques_webgl"))
            .and_then(|extension| extension.get_mut("techniques"))
        else {
            return None;
        };
        return object(techniques, |value, index| handler(value, index.to_string()));
    }

    top_level(gltf, "techniques", handler)
}

pub fn texture<T>(gltf: &mut Value, handler: impl FnMut(&mut Value, String) -> Option<T>) -> Option<T> {
    top_level(gltf, "textures", handler)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn top_level_iterates_arrays_with_string_ids() {
        let mut gltf = json!({ "accessors": [{ "count": 1 }, { "count": 2 }] });
        let mut seen = Vec::new();
        accessor(&mut gltf, |value, id| {
            seen.push((id, value["count"].as_u64().unwrap()));
            None::<()>
        });
        assert_eq!(seen, vec![("0".to_string(), 1), ("1".to_string(), 2)]);
    }

    #[test]
    fn top_level_iterates_legacy_objects_by_key() {
        let mut gltf = json!({ "buffers": { "bin": { "byteLength": 8 } } });
        let mut keys = Vec::new();
        buffer(&mut gltf, |_, id| {
            keys.push(id);
            None::<()>
        });
        assert_eq!(keys, vec!["bin"]);
    }

    #[test]
    fn handler_returning_some_short_circuits() {
        let mut gltf = json!({ "materials": [{}, {}, {}] });
        let index = material(&mut gltf, |_, id| Some(id));
        assert_eq!(index, Some("0".to_string()));
    }

    #[test]
    fn accessor_with_semantic_dedupes_and_prefix_matches() {
        let mut gltf = json!({
            "meshes": [{
                "primitives": [
                    { "attributes": { "POSITION": 0, "TEXCOORD_0": 1 } },
                    { "attributes": { "POSITION": 0, "TEXCOORD_1": 2 } },
                    { "targets": [{ "POSITION": 3 }] }
                ]
            }]
        });
        let mut visited = Vec::new();
        accessor_with_semantic(&mut gltf, "TEXCOORD", |id| {
            visited.push(id);
            None::<()>
        });
        assert_eq!(visited, vec![1, 2]);

        let mut positions = Vec::new();
        accessor_with_semantic(&mut gltf, "POSITION", |id| {
            positions.push(id);
            None::<()>
        });
        assert_eq!(positions, vec![0, 3]);
    }

    #[test]
    fn accessor_containing_vertex_attribute_data_visits_once() {
        let mut gltf = json!({
            "meshes": [{
                "primitives": [
                    { "attributes": { "POSITION": 0, "NORMAL": 1 } },
                    { "attributes": { "POSITION": 0 } }
                ]
            }]
        });
        let mut visited = Vec::new();
        accessor_containing_vertex_attribute_data(&mut gltf, |id| {
            visited.push(id);
            None::<()>
        });
        // The contract is "each accessor visited once"; the visit order
        // follows `serde_json::Map` key order (see module DEVIATION), so
        // compare as a set.
        visited.sort_unstable();
        assert_eq!(visited, vec![0, 1]);
    }

    #[test]
    fn accessor_containing_index_data_visits_once() {
        let mut gltf = json!({
            "meshes": [{
                "primitives": [{ "indices": 5 }, { "indices": 5 }, { "indices": 6 }]
            }]
        });
        let mut visited = Vec::new();
        accessor_containing_index_data(&mut gltf, |id| {
            visited.push(id);
            None::<()>
        });
        assert_eq!(visited, vec![5, 6]);
    }

    #[test]
    fn mesh_primitive_attribute_allows_in_place_rewrite() {
        let mut primitive = json!({ "attributes": { "POSITION": 3 } });
        mesh_primitive_attribute(&mut primitive, |accessor_id, _semantic| {
            if let Some(id) = accessor_id.as_u64() {
                *accessor_id = json!(id - 1);
            }
            None::<()>
        });
        assert_eq!(primitive["attributes"]["POSITION"], 2);
    }

    #[test]
    fn node_in_tree_traverses_children_depth_first() {
        let mut gltf = json!({
            "nodes": [
                { "children": [1, 2] },
                {},
                { "children": [3] },
                {}
            ]
        });
        let mut order = Vec::new();
        node_in_tree(&mut gltf, &[0], |_, node_id| {
            order.push(node_id);
            None::<()>
        });
        assert_eq!(order, vec![0, 1, 2, 3]);
    }

    #[test]
    fn node_in_scene_uses_scene_roots() {
        let mut gltf = json!({
            "scenes": [{ "nodes": [1] }],
            "nodes": [{}, { "children": [2] }, {}]
        });
        let mut order = Vec::new();
        node_in_scene(&mut gltf, 0, |_, node_id| {
            order.push(node_id);
            None::<()>
        });
        assert_eq!(order, vec![1, 2]);
    }

    #[test]
    fn technique_routes_through_khr_techniques_webgl() {
        let mut gltf = json!({
            "extensionsUsed": ["KHR_techniques_webgl"],
            "extensions": {
                "KHR_techniques_webgl": { "techniques": [{ "name": "t0" }] }
            }
        });
        let mut names = Vec::new();
        technique(&mut gltf, |technique, _| {
            names.push(technique["name"].as_str().unwrap().to_string());
            None::<()>
        });
        assert_eq!(names, vec!["t0"]);
    }

    #[test]
    fn material_value_prefers_khr_techniques_values() {
        let mut material = json!({
            "values": { "u_diffuse": [0.0, 0.0, 0.0, 1.0] },
            "extensions": {
                "KHR_techniques_webgl": { "values": { "u_tex": { "index": 0 } } }
            }
        });
        let mut names = Vec::new();
        material_value(&mut material, |_, name| {
            names.push(name);
            None::<()>
        });
        assert_eq!(names, vec!["u_tex"]);
    }

    #[test]
    fn skin_joint_iterates_joint_ids() {
        let mut skin = json!({ "joints": [4, 5] });
        let mut joints = Vec::new();
        skin_joint(&mut skin, |joint| {
            joints.push(joint.as_u64().unwrap());
            None::<()>
        });
        assert_eq!(joints, vec![4, 5]);
    }
}
