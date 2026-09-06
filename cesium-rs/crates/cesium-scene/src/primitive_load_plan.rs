//! Ported from `packages/engine/Source/Scene/PrimitiveLoadPlan.js`.
//!
//! Tracks how each attribute and index buffer of a primitive will be
//! loaded (as GPU buffer, typed array, or both).

/// Simple struct for tracking whether an attribute will be loaded as a
/// GPU buffer or typed array after post-processing.
///
/// Mirrors CesiumJS `PrimitiveLoadPlan.AttributeLoadPlan`.
#[derive(Debug, Clone)]
pub struct AttributeLoadPlan {
    /// The attribute's glTF name (e.g., "POSITION", "NORMAL", "TEXCOORD_0").
    pub attribute_name: String,
    /// Whether this attribute will be loaded as a GPU buffer.
    pub load_buffer: bool,
    /// Whether this attribute will be loaded as a packed typed array.
    pub load_typed_array: bool,
}

impl AttributeLoadPlan {
    /// Creates a new `AttributeLoadPlan` for the given attribute name.
    pub fn new(attribute_name: &str) -> Self {
        Self {
            attribute_name: attribute_name.to_string(),
            load_buffer: false,
            load_typed_array: false,
        }
    }
}

/// Simple struct for tracking whether an index buffer will be loaded as
/// a GPU buffer or typed array after post-processing.
///
/// Mirrors CesiumJS `PrimitiveLoadPlan.IndicesLoadPlan`.
#[derive(Debug, Clone)]
pub struct IndicesLoadPlan {
    /// The number of indices.
    pub count: usize,
    /// The index component datatype (e.g., "UNSIGNED_SHORT", "UNSIGNED_INT").
    pub component_type: String,
    /// Whether this index buffer will be loaded as a GPU buffer.
    pub load_buffer: bool,
    /// Whether this index buffer will be loaded as a packed typed array.
    pub load_typed_array: bool,
}

impl IndicesLoadPlan {
    /// Creates a new `IndicesLoadPlan`.
    pub fn new(count: usize, component_type: &str) -> Self {
        Self {
            count,
            component_type: component_type.to_string(),
            load_buffer: false,
            load_typed_array: false,
        }
    }
}

/// A plan for loading a primitive.
///
/// Mirrors CesiumJS `PrimitiveLoadPlan` (304 lines):
/// tracks attribute and index buffer load strategies.
#[derive(Debug, Clone)]
pub struct PrimitiveLoadPlan {
    /// The attribute load plans.
    pub attribute_plans: Vec<AttributeLoadPlan>,
    /// The indices load plan (if the primitive is indexed).
    pub indices_plan: Option<IndicesLoadPlan>,
    /// The primitive's topology (e.g., TRIANGLES, LINES, POINTS).
    pub primitive_type: i32,
}

impl PrimitiveLoadPlan {
    /// Creates a new `PrimitiveLoadPlan`.
    pub fn new(primitive_type: i32) -> Self {
        Self {
            attribute_plans: Vec::new(),
            indices_plan: None,
            primitive_type,
        }
    }

    /// Adds an attribute load plan.
    pub fn add_attribute_plan(&mut self, plan: AttributeLoadPlan) {
        self.attribute_plans.push(plan);
    }

    /// Returns the number of attribute plans.
    pub fn attributes_length(&self) -> usize {
        self.attribute_plans.len()
    }

    /// Whether this primitive has indexed geometry.
    pub fn is_indexed(&self) -> bool {
        self.indices_plan.is_some()
    }
}

impl Default for PrimitiveLoadPlan {
    fn default() -> Self {
        Self::new(4) // TRIANGLES = 4 (matches PrimitiveType::TRIANGLES)
    }
}
