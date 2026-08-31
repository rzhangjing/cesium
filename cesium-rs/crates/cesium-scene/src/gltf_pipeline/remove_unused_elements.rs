//! Ported from
//! `packages/engine/Source/Scene/GltfPipeline/removeUnusedElements.js`.

use std::collections::HashSet;

use serde_json::{json, Value};

use crate::gltf_pipeline::defined;
use crate::gltf_pipeline::for_each;
use crate::gltf_pipeline::for_each_texture_in_material::for_each_texture_in_material;
use crate::gltf_pipeline::uses_extension::uses_extension;
use crate::gltf_pipeline::PipelineBufferSources;

const ALL_ELEMENT_TYPES: [&str; 9] = [
    "mesh",
    "node",
    "material",
    "accessor",
    "bufferView",
    "buffer",
    "texture",
    "sampler",
    "image",
];

/// Removes unused elements from the glTF asset.
///
/// `element_types` defaults to all element types; it needs to be a subset of
/// `["mesh", "node", "material", "accessor", "bufferView", "buffer",
/// "texture", "sampler", "image"]`, other items are ignored.
///
/// The binary sources side table is kept aligned with `gltf["buffers"]`
/// (`Remove.buffer` splices both).
pub fn remove_unused_elements(
    gltf: &mut Value,
    sources: &mut PipelineBufferSources,
    element_types: Option<&[&str]>,
) {
    let requested: Vec<&str> = element_types
        .map(|types| types.to_vec())
        .unwrap_or_else(|| ALL_ELEMENT_TYPES.to_vec());
    for element_type in ALL_ELEMENT_TYPES {
        if requested.contains(&element_type) {
            remove_unused_elements_by_type(gltf, sources, element_type);
        }
    }
}

fn type_to_gltf_element_name(element_type: &str) -> &'static str {
    match element_type {
        "accessor" => "accessors",
        "buffer" => "buffers",
        "bufferView" => "bufferViews",
        "image" => "images",
        "node" => "nodes",
        "material" => "materials",
        "mesh" => "meshes",
        "sampler" => "samplers",
        "texture" => "textures",
        _ => "",
    }
}

fn remove_unused_elements_by_type(gltf: &mut Value, sources: &mut PipelineBufferSources, element_type: &str) {
    let name = type_to_gltf_element_name(element_type);
    let is_array = gltf.get(name).map(|value| value.is_array()).unwrap_or(false);
    if !is_array {
        return;
    }

    let mut removed = 0usize;
    let used_ids = get_list_of_elements_ids_in_use(gltf, element_type);
    let length = gltf[name].as_array().expect("checked above").len();

    for i in 0..length {
        if !used_ids.contains(&(i as u64)) {
            remove_element(gltf, sources, element_type, i - removed);
            removed += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Remove.* — element removal with index shifting
// ---------------------------------------------------------------------------

fn remove_element(
    gltf: &mut Value,
    sources: &mut PipelineBufferSources,
    element_type: &str,
    id: usize,
) {
    match element_type {
        "accessor" => remove_accessor(gltf, id),
        "buffer" => remove_buffer(gltf, sources, id),
        "bufferView" => remove_buffer_view(gltf, id),
        "image" => remove_image(gltf, id),
        "mesh" => remove_mesh(gltf, id),
        "node" => remove_node(gltf, id),
        "material" => remove_material(gltf, id),
        "sampler" => remove_sampler(gltf, id),
        "texture" => remove_texture(gltf, id),
        _ => {}
    }
}

fn splice(array: &mut Value, id: usize) {
    if let Some(list) = array.as_array_mut() {
        if id < list.len() {
            list.remove(id);
        }
    }
}

/// `if (value > threshold) value--;`
fn decrement_if_greater(value: &mut Value, threshold: usize) {
    if let Some(number) = value.as_u64() {
        if number > threshold as u64 {
            *value = json!(number - 1);
        }
    }
}

fn remove_accessor(gltf: &mut Value, accessor_id: usize) {
    if let Some(accessors) = gltf.get_mut("accessors") {
        splice(accessors, accessor_id);
    }

    for_each::mesh(gltf, |mesh, _| {
        for_each::mesh_primitive(mesh, |primitive, _| {
            // Update accessor ids for the primitives.
            let semantics: Vec<String> = primitive
                .get("attributes")
                .and_then(|attributes| attributes.as_object())
                .map(|attributes| attributes.keys().cloned().collect())
                .unwrap_or_default();
            for semantic in semantics {
                if let Some(attribute) = primitive
                    .get_mut("attributes")
                    .and_then(|attributes| attributes.get_mut(&semantic))
                {
                    decrement_if_greater(attribute, accessor_id);
                }
            }

            // Update accessor ids for the targets.
            for_each::mesh_primitive_target(primitive, |target, _| {
                let semantics: Vec<String> = target
                    .as_object()
                    .map(|object| object.keys().cloned().collect())
                    .unwrap_or_default();
                for semantic in semantics {
                    decrement_if_greater(&mut target[semantic.as_str()], accessor_id);
                }
                None::<()>
            });

            if let Some(indices) = primitive.get_mut("indices") {
                decrement_if_greater(indices, accessor_id);
            }

            if let Some(outline_indices) = primitive
                .pointer_mut("/extensions/CESIUM_primitive_outline/indices")
            {
                decrement_if_greater(outline_indices, accessor_id);
            }
            None::<()>
        });
        None::<()>
    });

    for_each::skin(gltf, |skin, _| {
        if let Some(inverse_bind_matrices) = skin.get_mut("inverseBindMatrices") {
            decrement_if_greater(inverse_bind_matrices, accessor_id);
        }
        None::<()>
    });

    for_each::animation(gltf, |animation, _| {
        for_each::animation_sampler(animation, |sampler, _| {
            if let Some(input) = sampler.get_mut("input") {
                decrement_if_greater(input, accessor_id);
            }
            if let Some(output) = sampler.get_mut("output") {
                decrement_if_greater(output, accessor_id);
            }
            None::<()>
        });
        None::<()>
    });
}

fn remove_buffer(gltf: &mut Value, sources: &mut PipelineBufferSources, buffer_id: usize) {
    if let Some(buffers) = gltf.get_mut("buffers") {
        splice(buffers, buffer_id);
    }
    // Keep the binary sources side table aligned with gltf["buffers"].
    if buffer_id < sources.len() {
        sources.remove(buffer_id);
    }

    for_each::buffer_view(gltf, |buffer_view, _| {
        if let Some(buffer) = buffer_view.get_mut("buffer") {
            decrement_if_greater(buffer, buffer_id);
        }

        let has_extensions = buffer_view
            .get("extensions")
            .map(|extensions| !extensions.is_null())
            .unwrap_or(false);
        if has_extensions {
            if let Some(ext_buffer) =
                buffer_view.pointer_mut("/extensions/EXT_meshopt_compression/buffer")
            {
                decrement_if_greater(ext_buffer, buffer_id);
            }
            if let Some(khr_buffer) =
                buffer_view.pointer_mut("/extensions/KHR_meshopt_compression/buffer")
            {
                decrement_if_greater(khr_buffer, buffer_id);
            }
        }
        None::<()>
    });
}

fn remove_buffer_view(gltf: &mut Value, buffer_view_id: usize) {
    if let Some(buffer_views) = gltf.get_mut("bufferViews") {
        splice(buffer_views, buffer_view_id);
    }

    for_each::accessor(gltf, |accessor, _| {
        if let Some(buffer_view) = accessor.get_mut("bufferView") {
            decrement_if_greater(buffer_view, buffer_view_id);
        }
        None::<()>
    });

    for_each::shader(gltf, |shader, _| {
        if let Some(buffer_view) = shader.get_mut("bufferView") {
            decrement_if_greater(buffer_view, buffer_view_id);
        }
        None::<()>
    });

    for_each::image(gltf, |image, _| {
        if let Some(buffer_view) = image.get_mut("bufferView") {
            decrement_if_greater(buffer_view, buffer_view_id);
        }
        None::<()>
    });

    if uses_extension(gltf, "KHR_draco_mesh_compression") {
        for_each::mesh(gltf, |mesh, _| {
            for_each::mesh_primitive(mesh, |primitive, _| {
                if let Some(draco_buffer_view) = primitive
                    .pointer_mut("/extensions/KHR_draco_mesh_compression/bufferView")
                {
                    decrement_if_greater(draco_buffer_view, buffer_view_id);
                }
                None::<()>
            });
            None::<()>
        });
    }

    if uses_extension(gltf, "EXT_feature_metadata") {
        let property_pointers = collect_object_property_pointers(
            gltf,
            "/extensions/EXT_feature_metadata/featureTables",
            true,
        );
        for pointer in property_pointers {
            for field in ["bufferView", "arrayOffsetBufferView", "stringOffsetBufferView"] {
                let field_pointer = format!("{pointer}/{field}");
                if let Some(value) = gltf.pointer_mut(&field_pointer) {
                    decrement_if_greater(value, buffer_view_id);
                }
            }
        }
    }

    if uses_extension(gltf, "EXT_structural_metadata") {
        let property_pointers = collect_table_property_pointers(
            gltf,
            "/extensions/EXT_structural_metadata/propertyTables",
        );
        for pointer in property_pointers {
            for field in ["values", "arrayOffsets", "stringOffsets"] {
                let field_pointer = format!("{pointer}/{field}");
                if let Some(value) = gltf.pointer_mut(&field_pointer) {
                    decrement_if_greater(value, buffer_view_id);
                }
            }
        }
    }
}

fn remove_image(gltf: &mut Value, image_id: usize) {
    if let Some(images) = gltf.get_mut("images") {
        splice(images, image_id);
    }

    for_each::texture(gltf, |texture, _| {
        if let Some(source) = texture.get_mut("source") {
            decrement_if_greater(source, image_id);
        }
        let webp_source = texture
            .pointer_mut("/extensions/EXT_texture_webp/source")
            .map(|source| {
                decrement_if_greater(source, image_id);
            });
        if webp_source.is_none() {
            if let Some(basisu_source) = texture.pointer_mut("/extensions/KHR_texture_basisu/source")
            {
                decrement_if_greater(basisu_source, image_id);
            }
        }
        None::<()>
    });
}

fn remove_mesh(gltf: &mut Value, mesh_id: usize) {
    if let Some(meshes) = gltf.get_mut("meshes") {
        splice(meshes, mesh_id);
    }

    for_each::node(gltf, |node, _| {
        if defined(node.get("mesh")) {
            let mesh_value = node.get("mesh").expect("checked above").as_u64();
            match mesh_value {
                Some(mesh) if mesh > mesh_id as u64 => {
                    node["mesh"] = json!(mesh - 1);
                }
                Some(mesh) if mesh == mesh_id as u64 => {
                    // Remove reference to deleted mesh
                    if let Some(object) = node.as_object_mut() {
                        object.remove("mesh");
                    }
                }
                _ => {}
            }
        }
        None::<()>
    });
}

fn remove_node(gltf: &mut Value, node_id: usize) {
    if let Some(nodes) = gltf.get_mut("nodes") {
        splice(nodes, node_id);
    }

    // Shift all node references
    for_each::skin(gltf, |skin, _| {
        if let Some(skeleton) = skin.get_mut("skeleton") {
            decrement_if_greater(skeleton, node_id);
        }
        if let Some(joints) = skin.get_mut("joints").and_then(|joints| joints.as_array_mut()) {
            for joint in joints.iter_mut() {
                decrement_if_greater(joint, node_id);
            }
        }
        None::<()>
    });

    for_each::animation(gltf, |animation, _| {
        for_each::animation_channel(animation, |channel, _| {
            if let Some(target_node) = channel.pointer_mut("/target/node") {
                decrement_if_greater(target_node, node_id);
            }
            None::<()>
        });
        None::<()>
    });

    for_each::technique(gltf, |technique, _| {
        for_each::technique_uniform(technique, |uniform, _| {
            if let Some(uniform_node) = uniform.get_mut("node") {
                decrement_if_greater(uniform_node, node_id);
            }
            None::<()>
        });
        None::<()>
    });

    for_each::node(gltf, |node, _| {
        if defined(node.get("children")) {
            filter_and_shift(&mut node["children"], node_id);
        }
        None::<()>
    });

    for_each::scene(gltf, |scene, _| {
        if defined(scene.get("nodes")) {
            filter_and_shift(&mut scene["nodes"], node_id);
        }
        None::<()>
    });
}

/// `array = array.filter(x => x !== id).map(x => x > id ? x - 1 : x)`
fn filter_and_shift(array: &mut Value, removed_id: usize) {
    let Some(list) = array.as_array_mut() else {
        return;
    };
    list.retain(|item| item.as_u64() != Some(removed_id as u64));
    for item in list.iter_mut() {
        decrement_if_greater(item, removed_id);
    }
}

fn remove_material(gltf: &mut Value, material_id: usize) {
    if let Some(materials) = gltf.get_mut("materials") {
        splice(materials, material_id);
    }

    // Shift other material ids
    for_each::mesh(gltf, |mesh, _| {
        for_each::mesh_primitive(mesh, |primitive, _| {
            if let Some(material) = primitive.get_mut("material") {
                decrement_if_greater(material, material_id);
            }
            None::<()>
        });
        None::<()>
    });
}

fn remove_sampler(gltf: &mut Value, sampler_id: usize) {
    if let Some(samplers) = gltf.get_mut("samplers") {
        splice(samplers, sampler_id);
    }

    for_each::texture(gltf, |texture, _| {
        if let Some(sampler) = texture.get_mut("sampler") {
            decrement_if_greater(sampler, sampler_id);
        }
        None::<()>
    });
}

fn remove_texture(gltf: &mut Value, texture_id: usize) {
    if let Some(textures) = gltf.get_mut("textures") {
        splice(textures, texture_id);
    }

    for_each::material(gltf, |material, _| {
        for_each_texture_in_material(material, |_index, texture_info| {
            if let Some(index) = texture_info.get_mut("index") {
                decrement_if_greater(index, texture_id);
            }
            None::<()>
        });
        None::<()>
    });

    if uses_extension(gltf, "EXT_feature_metadata") {
        for_each::mesh(gltf, |mesh, _| {
            for_each::mesh_primitive(mesh, |primitive, _| {
                let feature_id_textures_length = primitive
                    .get("extensions")
                    .and_then(|extensions| extensions.get("EXT_feature_metadata"))
                    .and_then(|extension| extension.get("featureIdTextures"))
                    .and_then(|feature_id_textures| feature_id_textures.as_array())
                    .map(|feature_id_textures| feature_id_textures.len())
                    .unwrap_or(0);
                for i in 0..feature_id_textures_length {
                    if let Some(index) = primitive.pointer_mut(&format!(
                        "/extensions/EXT_feature_metadata/featureIdTextures/{i}/featureIds/texture/index"
                    )) {
                        decrement_if_greater(index, texture_id);
                    }
                }
                None::<()>
            });
            None::<()>
        });

        let feature_texture_property_pointers = collect_object_property_pointers(
            gltf,
            "/extensions/EXT_feature_metadata/featureTextures",
            true,
        );
        for pointer in feature_texture_property_pointers {
            let index_pointer = format!("{pointer}/texture/index");
            if let Some(index) = gltf.pointer_mut(&index_pointer) {
                decrement_if_greater(index, texture_id);
            }
        }
    }

    if uses_extension(gltf, "EXT_mesh_features") {
        for_each::mesh(gltf, |mesh, _| {
            for_each::mesh_primitive(mesh, |primitive, _| {
                let feature_ids_length = primitive
                    .get("extensions")
                    .and_then(|extensions| extensions.get("EXT_mesh_features"))
                    .and_then(|extension| extension.get("featureIds"))
                    .and_then(|feature_ids| feature_ids.as_array())
                    .map(|feature_ids| feature_ids.len())
                    .unwrap_or(0);
                for i in 0..feature_ids_length {
                    if let Some(index) = primitive.pointer_mut(&format!(
                        "/extensions/EXT_mesh_features/featureIds/{i}/texture/index"
                    )) {
                        decrement_if_greater(index, texture_id);
                    }
                }
                None::<()>
            });
            None::<()>
        });
    }

    if uses_extension(gltf, "EXT_structural_metadata") {
        let property_textures_length = gltf
            .pointer("/extensions/EXT_structural_metadata/propertyTextures")
            .and_then(|property_textures| property_textures.as_array())
            .map(|property_textures| property_textures.len())
            .unwrap_or(0);
        for i in 0..property_textures_length {
            let property_keys: Vec<String> = gltf
                .pointer(&format!(
                    "/extensions/EXT_structural_metadata/propertyTextures/{i}/properties"
                ))
                .and_then(|properties| properties.as_object())
                .map(|properties| properties.keys().cloned().collect())
                .unwrap_or_default();
            for property_id in property_keys {
                if let Some(index) = gltf.pointer_mut(&format!(
                    "/extensions/EXT_structural_metadata/propertyTextures/{i}/properties/{property_id}/index"
                )) {
                    decrement_if_greater(index, texture_id);
                }
            }
        }
    }
}

/// Collects JSON pointers to the per-property objects of either an
/// object-of-objects (`featureTables`/`featureTextures`, when
/// `tables_are_object` is true) or an array-of-tables with object-keyed
/// `properties` (`propertyTables`).
fn collect_object_property_pointers(gltf: &Value, tables_pointer: &str, tables_are_object: bool) -> Vec<String> {
    let mut pointers = Vec::new();
    let Some(tables) = gltf.pointer(tables_pointer) else {
        return pointers;
    };
    if tables_are_object {
        if let Some(tables_map) = tables.as_object() {
            for (table_id, table) in tables_map {
                if let Some(properties) = table.get("properties").and_then(|p| p.as_object()) {
                    for property_id in properties.keys() {
                        pointers.push(format!(
                            "{tables_pointer}/{table_id}/properties/{property_id}"
                        ));
                    }
                }
            }
        }
    }
    pointers
}

fn collect_table_property_pointers(gltf: &Value, tables_pointer: &str) -> Vec<String> {
    let mut pointers = Vec::new();
    let Some(tables) = gltf.pointer(tables_pointer).and_then(|tables| tables.as_array()) else {
        return pointers;
    };
    for (table_index, table) in tables.iter().enumerate() {
        if let Some(properties) = table.get("properties").and_then(|p| p.as_object()) {
            for property_id in properties.keys() {
                pointers.push(format!("{tables_pointer}/{table_index}/properties/{property_id}"));
            }
        }
    }
    pointers
}

// ---------------------------------------------------------------------------
// getListOfElementsIdsInUse.* — immutable used-id collection
// ---------------------------------------------------------------------------

fn get_list_of_elements_ids_in_use(gltf: &Value, element_type: &str) -> HashSet<u64> {
    match element_type {
        "accessor" => used_accessor_ids(gltf),
        "buffer" => used_buffer_ids(gltf),
        "bufferView" => used_buffer_view_ids(gltf),
        "image" => used_image_ids(gltf),
        "mesh" => used_mesh_ids(gltf),
        "node" => used_node_ids(gltf),
        "material" => used_material_ids(gltf),
        "texture" => used_texture_ids(gltf),
        "sampler" => used_sampler_ids(gltf),
        _ => HashSet::new(),
    }
}

fn record(used: &mut HashSet<u64>, value: Option<&Value>) {
    if let Some(id) = value.and_then(|value| value.as_u64()) {
        used.insert(id);
    }
}

fn used_accessor_ids(gltf: &Value) -> HashSet<u64> {
    let mut used = HashSet::new();

    if let Some(meshes) = gltf.get("meshes").and_then(|meshes| meshes.as_array()) {
        for mesh in meshes {
            if let Some(primitives) = mesh.get("primitives").and_then(|p| p.as_array()) {
                for primitive in primitives {
                    if let Some(attributes) = primitive.get("attributes").and_then(|a| a.as_object())
                    {
                        for accessor_id in attributes.values() {
                            record(&mut used, Some(accessor_id));
                        }
                    }
                    if let Some(targets) = primitive.get("targets").and_then(|t| t.as_array()) {
                        for target in targets {
                            if let Some(target_attributes) = target.as_object() {
                                for accessor_id in target_attributes.values() {
                                    record(&mut used, Some(accessor_id));
                                }
                            }
                        }
                    }
                    record(&mut used, primitive.get("indices"));
                }
            }
        }
    }

    if let Some(skins) = gltf.get("skins").and_then(|skins| skins.as_array()) {
        for skin in skins {
            record(&mut used, skin.get("inverseBindMatrices"));
        }
    }

    if let Some(animations) = gltf.get("animations").and_then(|animations| animations.as_array()) {
        for animation in animations {
            if let Some(samplers) = animation.get("samplers").and_then(|s| s.as_array()) {
                for sampler in samplers {
                    record(&mut used, sampler.get("input"));
                    record(&mut used, sampler.get("output"));
                }
            }
        }
    }

    if uses_extension(gltf, "EXT_mesh_gpu_instancing") {
        if let Some(nodes) = gltf.get("nodes").and_then(|nodes| nodes.as_array()) {
            for node in nodes {
                if let Some(attributes) = node
                    .get("extensions")
                    .and_then(|extensions| extensions.get("EXT_mesh_gpu_instancing"))
                    .and_then(|instancing| instancing.get("attributes"))
                    .and_then(|attributes| attributes.as_object())
                {
                    for accessor_id in attributes.values() {
                        record(&mut used, Some(accessor_id));
                    }
                }
            }
        }
    }

    if uses_extension(gltf, "CESIUM_primitive_outline") {
        if let Some(meshes) = gltf.get("meshes").and_then(|meshes| meshes.as_array()) {
            for mesh in meshes {
                if let Some(primitives) = mesh.get("primitives").and_then(|p| p.as_array()) {
                    for primitive in primitives {
                        record(
                            &mut used,
                            primitive
                                .get("extensions")
                                .and_then(|extensions| extensions.get("CESIUM_primitive_outline"))
                                .and_then(|extension| extension.get("indices")),
                        );
                    }
                }
            }
        }
    }

    used
}

fn used_buffer_ids(gltf: &Value) -> HashSet<u64> {
    let mut used = HashSet::new();
    if let Some(buffer_views) = gltf.get("bufferViews").and_then(|views| views.as_array()) {
        for buffer_view in buffer_views {
            record(&mut used, buffer_view.get("buffer"));
            let extensions = buffer_view.get("extensions");
            if defined(extensions) {
                record(
                    &mut used,
                    extensions.and_then(|extensions| extensions.get("EXT_meshopt_compression")).and_then(|ext| ext.get("buffer")),
                );
                record(
                    &mut used,
                    extensions.and_then(|extensions| extensions.get("KHR_meshopt_compression")).and_then(|khr| khr.get("buffer")),
                );
            }
        }
    }
    used
}

fn used_buffer_view_ids(gltf: &Value) -> HashSet<u64> {
    let mut used = HashSet::new();

    if let Some(accessors) = gltf.get("accessors").and_then(|accessors| accessors.as_array()) {
        for accessor in accessors {
            record(&mut used, accessor.get("bufferView"));
        }
    }
    for shader in shaders_of(gltf) {
        record(&mut used, shader.get("bufferView"));
    }
    if let Some(images) = gltf.get("images").and_then(|images| images.as_array()) {
        for image in images {
            record(&mut used, image.get("bufferView"));
        }
    }

    if uses_extension(gltf, "KHR_draco_mesh_compression") {
        if let Some(meshes) = gltf.get("meshes").and_then(|meshes| meshes.as_array()) {
            for mesh in meshes {
                if let Some(primitives) = mesh.get("primitives").and_then(|p| p.as_array()) {
                    for primitive in primitives {
                        record(
                            &mut used,
                            primitive
                                .get("extensions")
                                .and_then(|extensions| extensions.get("KHR_draco_mesh_compression"))
                                .and_then(|draco| draco.get("bufferView")),
                        );
                    }
                }
            }
        }
    }

    if uses_extension(gltf, "EXT_feature_metadata") {
        for pointer in
            collect_object_property_pointers(gltf, "/extensions/EXT_feature_metadata/featureTables", true)
        {
            for field in ["bufferView", "arrayOffsetBufferView", "stringOffsetBufferView"] {
                record(&mut used, gltf.pointer(&format!("{pointer}/{field}")));
            }
        }
    }

    if uses_extension(gltf, "EXT_structural_metadata") {
        for pointer in collect_table_property_pointers(
            gltf,
            "/extensions/EXT_structural_metadata/propertyTables",
        ) {
            for field in ["values", "arrayOffsets", "stringOffsets"] {
                record(&mut used, gltf.pointer(&format!("{pointer}/{field}")));
            }
        }
    }

    used
}

fn used_image_ids(gltf: &Value) -> HashSet<u64> {
    let mut used = HashSet::new();
    if let Some(textures) = gltf.get("textures").and_then(|textures| textures.as_array()) {
        for texture in textures {
            record(&mut used, texture.get("source"));
            let extensions = texture.get("extensions");
            if extensions
                .and_then(|extensions| extensions.get("EXT_texture_webp"))
                .map(|value| !value.is_null())
                .unwrap_or(false)
            {
                record(&mut used, extensions.and_then(|e| e.get("EXT_texture_webp")).and_then(|ext| ext.get("source")));
            } else if extensions
                .and_then(|extensions| extensions.get("KHR_texture_basisu"))
                .map(|value| !value.is_null())
                .unwrap_or(false)
            {
                record(&mut used, extensions.and_then(|e| e.get("KHR_texture_basisu")).and_then(|ext| ext.get("source")));
            }
        }
    }
    used
}

fn used_mesh_ids(gltf: &Value) -> HashSet<u64> {
    let mut used = HashSet::new();
    if let Some(nodes) = gltf.get("nodes").and_then(|nodes| nodes.as_array()) {
        for node in nodes {
            let meshes_defined = defined(gltf.get("meshes"));
            if defined(node.get("mesh")) && meshes_defined {
                if let Some(mesh_id) = node.get("mesh").and_then(|mesh| mesh.as_u64()) {
                    let mesh = gltf
                        .get("meshes")
                        .and_then(|meshes| meshes.get(mesh_id as usize));
                    let has_primitives = mesh
                        .and_then(|mesh| mesh.get("primitives"))
                        .and_then(|primitives| primitives.as_array())
                        .map(|primitives| !primitives.is_empty())
                        .unwrap_or(false);
                    if mesh.is_some() && has_primitives {
                        used.insert(mesh_id);
                    }
                }
            }
        }
    }
    used
}

// Check if node is empty. It is considered empty if neither referencing
// mesh, camera, extensions and has no children
fn node_is_empty(gltf: &Value, node_id: usize, used_node_ids: &HashSet<u64>) -> bool {
    let Some(node) = gltf.get("nodes").and_then(|nodes| nodes.get(node_id)) else {
        return true;
    };
    if defined(node.get("mesh"))
        || defined(node.get("camera"))
        || defined(node.get("skin"))
        || defined(node.get("weights"))
        || defined(node.get("extras"))
        || node
            .get("extensions")
            .and_then(|extensions| extensions.as_object())
            .map(|extensions| !extensions.is_empty())
            .unwrap_or(false)
        || used_node_ids.contains(&(node_id as u64))
    {
        return false;
    }

    // Empty if no children or children are all empty nodes
    match node.get("children").and_then(|children| children.as_array()) {
        None => true,
        Some(children) => children.iter().all(|child| {
            child
                .as_u64()
                .map(|child_id| node_is_empty(gltf, child_id as usize, used_node_ids))
                .unwrap_or(true)
        }),
    }
}

fn used_node_ids(gltf: &Value) -> HashSet<u64> {
    let mut used = HashSet::new();

    if let Some(skins) = gltf.get("skins").and_then(|skins| skins.as_array()) {
        for skin in skins {
            record(&mut used, skin.get("skeleton"));
            if let Some(joints) = skin.get("joints").and_then(|joints| joints.as_array()) {
                for joint in joints {
                    record(&mut used, Some(joint));
                }
            }
        }
    }

    if let Some(animations) = gltf.get("animations").and_then(|animations| animations.as_array()) {
        for animation in animations {
            if let Some(channels) = animation.get("channels").and_then(|c| c.as_array()) {
                for channel in channels {
                    record(&mut used, channel.get("target").and_then(|target| target.get("node")));
                }
            }
        }
    }

    for_each_technique_free(gltf, &mut |technique| {
        if let Some(uniforms) = technique.get("uniforms").and_then(|uniforms| uniforms.as_object()) {
            for uniform in uniforms.values() {
                record(&mut used, uniform.get("node"));
            }
        }
    });

    if let Some(nodes) = gltf.get("nodes").and_then(|nodes| nodes.as_array()) {
        let nodes_length = nodes.len();
        for node_id in 0..nodes_length {
            if !node_is_empty(gltf, node_id, &used) {
                used.insert(node_id as u64);
            }
        }
    }

    used
}

fn used_material_ids(gltf: &Value) -> HashSet<u64> {
    let mut used = HashSet::new();
    if let Some(meshes) = gltf.get("meshes").and_then(|meshes| meshes.as_array()) {
        for mesh in meshes {
            if let Some(primitives) = mesh.get("primitives").and_then(|p| p.as_array()) {
                for primitive in primitives {
                    record(&mut used, primitive.get("material"));
                }
            }
        }
    }
    used
}

fn used_texture_ids(gltf: &Value) -> HashSet<u64> {
    let mut used = HashSet::new();

    if let Some(materials) = gltf.get("materials").and_then(|materials| materials.as_array()) {
        for material in materials {
            // Clone so the shared traversal helper (which takes &mut) can run
            // over an immutable snapshot; collection is read-only.
            let mut material = material.clone();
            for_each_texture_in_material(&mut material, |index, _info| {
                record(&mut used, Some(index));
                None::<()>
            });
        }
    }

    if uses_extension(gltf, "EXT_feature_metadata") {
        if let Some(meshes) = gltf.get("meshes").and_then(|meshes| meshes.as_array()) {
            for mesh in meshes {
                if let Some(primitives) = mesh.get("primitives").and_then(|p| p.as_array()) {
                    for primitive in primitives {
                        if let Some(feature_id_textures) = primitive
                            .get("extensions")
                            .and_then(|extensions| extensions.get("EXT_feature_metadata"))
                            .and_then(|extension| extension.get("featureIdTextures"))
                            .and_then(|feature_id_textures| feature_id_textures.as_array())
                        {
                            for feature_id_texture in feature_id_textures {
                                record(
                                    &mut used,
                                    feature_id_texture.pointer("/featureIds/texture/index"),
                                );
                            }
                        }
                    }
                }
            }
        }

        for pointer in collect_object_property_pointers(
            gltf,
            "/extensions/EXT_feature_metadata/featureTextures",
            true,
        ) {
            record(&mut used, gltf.pointer(&format!("{pointer}/texture/index")));
        }
    }

    if uses_extension(gltf, "EXT_mesh_features") {
        if let Some(meshes) = gltf.get("meshes").and_then(|meshes| meshes.as_array()) {
            for mesh in meshes {
                if let Some(primitives) = mesh.get("primitives").and_then(|p| p.as_array()) {
                    for primitive in primitives {
                        if let Some(feature_ids) = primitive
                            .get("extensions")
                            .and_then(|extensions| extensions.get("EXT_mesh_features"))
                            .and_then(|extension| extension.get("featureIds"))
                            .and_then(|feature_ids| feature_ids.as_array())
                        {
                            for feature_id in feature_ids {
                                if defined(feature_id.get("texture")) {
                                    record(&mut used, feature_id.pointer("/texture/index"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if uses_extension(gltf, "EXT_structural_metadata") {
        if let Some(property_textures) = gltf
            .pointer("/extensions/EXT_structural_metadata/propertyTextures")
            .and_then(|property_textures| property_textures.as_array())
        {
            for property_texture in property_textures {
                if let Some(properties) =
                    property_texture.get("properties").and_then(|p| p.as_object())
                {
                    for property in properties.values() {
                        record(&mut used, property.get("index"));
                    }
                }
            }
        }
    }

    used
}

fn used_sampler_ids(gltf: &Value) -> HashSet<u64> {
    let mut used = HashSet::new();
    if let Some(textures) = gltf.get("textures").and_then(|textures| textures.as_array()) {
        for texture in textures {
            record(&mut used, texture.get("sampler"));
        }
    }
    used
}

// Immutable technique traversal (top-level or KHR_techniques_webgl), mirroring
// ForEach.technique.
fn for_each_technique_free(gltf: &Value, handler: &mut impl FnMut(&Value)) {
    if uses_extension(gltf, "KHR_techniques_webgl") {
        if let Some(techniques) = gltf
            .pointer("/extensions/KHR_techniques_webgl/techniques")
            .and_then(|techniques| techniques.as_array())
        {
            for technique in techniques {
                handler(technique);
            }
        }
        return;
    }
    match gltf.get("techniques") {
        Some(Value::Array(techniques)) => {
            for technique in techniques {
                handler(technique);
            }
        }
        Some(Value::Object(techniques)) => {
            for technique in techniques.values() {
                handler(technique);
            }
        }
        _ => {}
    }
}

// Immutable shader traversal, mirroring ForEach.shader (top-level or
// KHR_techniques_webgl extension shaders).
fn shaders_of(gltf: &Value) -> Vec<&Value> {
    let mut shaders = Vec::new();
    if uses_extension(gltf, "KHR_techniques_webgl") {
        if let Some(extension_shaders) = gltf
            .pointer("/extensions/KHR_techniques_webgl/shaders")
            .and_then(|shaders| shaders.as_array())
        {
            shaders.extend(extension_shaders.iter());
        }
        return shaders;
    }
    if let Some(top_level_shaders) = gltf.get("shaders").and_then(|shaders| shaders.as_array()) {
        shaders.extend(top_level_shaders.iter());
        return shaders;
    }
    if let Some(top_level_shaders) = gltf.get("shaders").and_then(|shaders| shaders.as_object()) {
        shaders.extend(top_level_shaders.values());
    }
    shaders
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gltf_pipeline::PipelineBufferSource;

    #[test]
    fn removes_unused_mesh_and_shifts_references() {
        let mut gltf = json!({
            "meshes": [
                { "primitives": [] },
                { "primitives": [{ "attributes": {} }] }
            ],
            "nodes": [{ "mesh": 0 }, { "mesh": 1 }],
            "scenes": [{ "nodes": [0, 1] }]
        });
        let mut sources = Vec::new();
        remove_unused_elements(&mut gltf, &mut sources, Some(&["mesh", "node"]));
        // Mesh 0 has no primitives → unused; node 0 references it but is
        // itself empty → both removed.
        assert_eq!(gltf["meshes"].as_array().unwrap().len(), 1);
        assert_eq!(gltf["nodes"][0]["mesh"], json!(0));
    }

    #[test]
    fn removes_unused_accessors_and_shifts_indices() {
        let mut gltf = json!({
            "accessors": [
                { "bufferView": 0 },
                { "bufferView": 0 }
            ],
            "bufferViews": [{ "buffer": 0 }],
            "meshes": [
                { "primitives": [{ "attributes": { "POSITION": 1 } }] }
            ]
        });
        let mut sources = Vec::new();
        remove_unused_elements(&mut gltf, &mut sources, Some(&["accessor"]));
        assert_eq!(gltf["accessors"].as_array().unwrap().len(), 1);
        assert_eq!(gltf["meshes"][0]["primitives"][0]["attributes"]["POSITION"], json!(0));
    }

    #[test]
    fn removes_unused_buffers_and_keeps_sources_aligned() {
        let mut gltf = json!({
            "buffers": [{ "byteLength": 1 }, { "byteLength": 2 }, { "byteLength": 3 }],
            "bufferViews": [{ "buffer": 2 }]
        });
        let mut sources: PipelineBufferSources = vec![
            Some(PipelineBufferSource::new(vec![1])),
            None,
            Some(PipelineBufferSource::new(vec![3])),
        ];
        remove_unused_elements(&mut gltf, &mut sources, Some(&["buffer"]));
        assert_eq!(gltf["buffers"].as_array().unwrap().len(), 1);
        assert_eq!(gltf["bufferViews"][0]["buffer"], json!(0));
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].as_ref().unwrap().buffer, vec![3]);
    }

    #[test]
    fn removes_unused_textures_and_shifts_material_references() {
        let mut gltf = json!({
            "textures": [{}, {}, {}],
            "materials": [
                { "pbrMetallicRoughness": { "baseColorTexture": { "index": 2 } } }
            ]
        });
        let mut sources = Vec::new();
        remove_unused_elements(&mut gltf, &mut sources, Some(&["texture"]));
        assert_eq!(gltf["textures"].as_array().unwrap().len(), 1);
        assert_eq!(
            gltf["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"]["index"],
            json!(0)
        );
    }

    #[test]
    fn empty_nodes_are_removed_and_children_shifted() {
        let mut gltf = json!({
            "nodes": [
                { "mesh": 0 },
                {},
                { "children": [0, 1] }
            ],
            "meshes": [{ "primitives": [{ "attributes": {} }] }]
        });
        let mut sources = Vec::new();
        remove_unused_elements(&mut gltf, &mut sources, Some(&["node"]));
        // Node 1 is empty → removed; node 2 keeps child 0, drops 1, no shift
        // above the removed id.
        assert_eq!(gltf["nodes"].as_array().unwrap().len(), 2);
        assert_eq!(gltf["nodes"][1]["children"], json!([0]));
    }
}
