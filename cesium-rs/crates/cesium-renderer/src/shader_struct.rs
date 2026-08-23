//! Ported from `packages/engine/Source/Renderer/ShaderStruct.js`.
//!
//! Describes a struct within a shader program.

/// A field within a shader struct.
pub struct ShaderStructField {
    /// The GLSL type name (e.g. "vec3", "mat4", "float").
    pub glsl_type: String,
    /// The field name.
    pub name: String,
}

/// Describes a struct defined within a shader program.
pub struct ShaderStruct {
    /// The struct name.
    pub name: String,
    /// The fields.
    pub fields: Vec<ShaderStructField>,
}

impl ShaderStruct {
    /// Creates a new shader struct.
    pub fn new(name: String) -> Self {
        Self { name, fields: Vec::new() }
    }

    /// Adds a field to the struct.
    pub fn add_field(&mut self, glsl_type: String, name: String) {
        self.fields.push(ShaderStructField { glsl_type, name });
    }

    /// Generates the GLSL struct declaration.
    pub fn to_glsl(&self) -> String {
        let mut result = format!("struct {} {{\n", self.name);
        for field in &self.fields {
            result.push_str(&format!("    {} {};\n", field.glsl_type, field.name));
        }
        result.push_str("};\n");
        result
    }
}
