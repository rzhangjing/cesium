//! Ported from `packages/engine/Source/Scene/GltfPipeline/numberOfComponentsForType.js`.

/// Utility function for retrieving the number of components in a given type.
///
/// Returns `None` for unknown types (the JS function returns `undefined`).
pub fn number_of_components_for_type(gltf_type: &str) -> Option<usize> {
    match gltf_type {
        "SCALAR" => Some(1),
        "VEC2" => Some(2),
        "VEC3" => Some(3),
        "VEC4" | "MAT2" => Some(4),
        "MAT3" => Some(9),
        "MAT4" => Some(16),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_of_components_for_type_covers_all_gl_types() {
        assert_eq!(number_of_components_for_type("SCALAR"), Some(1));
        assert_eq!(number_of_components_for_type("VEC2"), Some(2));
        assert_eq!(number_of_components_for_type("VEC3"), Some(3));
        assert_eq!(number_of_components_for_type("VEC4"), Some(4));
        assert_eq!(number_of_components_for_type("MAT2"), Some(4));
        assert_eq!(number_of_components_for_type("MAT3"), Some(9));
        assert_eq!(number_of_components_for_type("MAT4"), Some(16));
        assert_eq!(number_of_components_for_type("UNKNOWN"), None);
    }
}
