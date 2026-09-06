//! Ported from `packages/engine/Source/Scene/getMeshPrimitives.js`.

use serde_json::Value;

/// Get an array of primitives for a given mesh. If the
/// `EXT_mesh_primitive_restart` extension is present, use it to combine
/// groups of primitives. If the extension is not present or its spec is
/// violated, return the original `mesh.primitives`.
///
/// Mirrors `getMeshPrimitives(mesh)`.
pub fn get_mesh_primitives(mesh: &Value) -> Vec<Value> {
    let mesh_primitives = match mesh.get("primitives").and_then(|p| p.as_array()) {
        Some(arr) => arr.clone(),
        None => return Vec::new(),
    };

    let primitive_restart = mesh
        .get("extensions")
        .and_then(|ext| ext.get("EXT_mesh_primitive_restart"));

    let primitive_restart = match primitive_restart {
        Some(ext) => ext,
        None => return mesh_primitives,
    };

    let primitive_groups = match primitive_restart
        .get("primitiveGroups")
        .and_then(|g| g.as_array())
    {
        Some(groups) => groups,
        None => return mesh_primitives,
    };

    // Start with a copy of mesh.primitives
    let mut primitives: Vec<Option<Value>> =
        mesh_primitives.iter().map(|p| Some(p.clone())).collect();

    // Supported topologies for primitive restart
    const TRIANGLE_FAN: u64 = 6;
    const TRIANGLE_STRIP: u64 = 5;
    const LINE_STRIP: u64 = 3;
    const LINE_LOOP: u64 = 2;

    for group in primitive_groups {
        let group_primitives = match group.get("primitives").and_then(|p| p.as_array()) {
            Some(arr) if !arr.is_empty() => arr,
            _ => return mesh_primitives.into_iter().collect(),
        };

        let first_index = match group_primitives[0].as_u64() {
            Some(idx) => idx as usize,
            None => return mesh_primitives.into_iter().collect(),
        };

        if first_index >= mesh_primitives.len() {
            return mesh_primitives.into_iter().collect();
        }

        let first_primitive = &mesh_primitives[first_index];
        let mode = first_primitive
            .get("mode")
            .and_then(|m| m.as_u64())
            .unwrap_or(4); // default: TRIANGLES

        // Only certain topologies support primitive restart
        match mode {
            TRIANGLE_FAN | TRIANGLE_STRIP | LINE_STRIP | LINE_LOOP => {}
            _ => return mesh_primitives.into_iter().collect(),
        }

        // Build the merged primitive
        let mut merged = first_primitive.clone();
        if let Some(indices) = group.get("indices") {
            if let Some(obj) = merged.as_object_mut() {
                obj.insert("indices".to_string(), indices.clone());
            }
        }

        // Validate and mark group primitives
        for prim_val in group_primitives {
            let idx = match prim_val.as_u64() {
                Some(i) => i as usize,
                None => return mesh_primitives.into_iter().collect(),
            };

            if idx >= primitives.len() || primitives[idx].is_none() {
                return mesh_primitives.into_iter().collect();
            }

            // Check same topology
            let this_mode = primitives[idx]
                .as_ref()
                .unwrap()
                .get("mode")
                .and_then(|m| m.as_u64())
                .unwrap_or(4);

            if this_mode != mode {
                return mesh_primitives.into_iter().collect();
            }

            // Must have indexed geometry
            let has_indices = primitives[idx]
                .as_ref()
                .unwrap()
                .get("indices")
                .is_some();
            if !has_indices {
                return mesh_primitives.into_iter().collect();
            }

            primitives[idx] = None;
        }

        primitives[first_index] = Some(merged);
    }

    primitives.into_iter().flatten().collect()
}
