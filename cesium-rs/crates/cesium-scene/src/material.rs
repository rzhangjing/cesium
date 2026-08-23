//! Ported from `packages/engine/Source/Scene/Material.js`.
//!
//! A material defines the appearance of a surface.

use cesium_core::color::Color;
use std::collections::HashMap;

/// A material defines the appearance of a surface.
///
/// Materials are composed of uniforms, textures, and a GLSL shader.
pub struct Material {
    /// The material type name.
    pub type_name: String,
    /// The uniforms for this material.
    pub uniforms: HashMap<String, MaterialUniform>,
    /// The shader source.
    pub shader_source: String,
}

/// A uniform value for a material.
pub enum MaterialUniform {
    /// A scalar float value.
    Float(f64),
    /// A color value.
    Color(Color),
    /// A texture reference.
    Texture(String),
    /// A 2D vector.
    Vec2(f64, f64),
    /// A 3D vector.
    Vec3(f64, f64, f64),
    /// A 4D vector.
    Vec4(f64, f64, f64, f64),
    /// A matrix.
    Matrix(Vec<f64>),
}

impl Material {
    /// Creates a new material with the given type.
    pub fn new(type_name: &str) -> Self {
        Self {
            type_name: type_name.to_string(),
            uniforms: HashMap::new(),
            shader_source: String::new(),
        }
    }

    /// Creates a color material.
    pub fn from_color(color: Color) -> Self {
        let mut mat = Self::new("Color");
        mat.uniforms.insert("color".to_string(), MaterialUniform::Color(color));
        mat
    }

    /// Creates an image material.
    pub fn from_image(image_url: &str) -> Self {
        let mut mat = Self::new("Image");
        mat.uniforms.insert("image".to_string(), MaterialUniform::Texture(image_url.to_string()));
        mat
    }

    /// Sets a uniform value.
    pub fn set_uniform(&mut self, name: &str, value: MaterialUniform) {
        self.uniforms.insert(name.to_string(), value);
    }
}
