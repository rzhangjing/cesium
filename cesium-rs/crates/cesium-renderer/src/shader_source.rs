//! Ported from `packages/engine/Source/Renderer/ShaderSource.js`.
//!
//! Processes shader source code (defines, pragmas, concatenation, dependency resolution).
//! In CesiumJS, this handles `#define` injection, `#pragma` processing, Builtin dependency
//! resolution, and source concatenation. In the Rust port, the heavy lifting of Builtin
//! injection and naga compatibility is delegated to `cesium_shaders::preprocessor`.

/// The type of shader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderType {
    /// Vertex shader.
    Vertex,
    /// Fragment shader.
    Fragment,
}

/// Options for creating a [`ShaderSource`].
pub struct ShaderSourceOptions {
    /// An array of strings containing GLSL code for the shader.
    pub sources: Vec<String>,
    /// An array of strings containing GLSL identifiers to `#define`.
    pub defines: Vec<String>,
    /// The shader type.
    pub shader_type: ShaderType,
    /// The GLSL qualifier (`"uniform"` or `"in"`) for pick color input.
    /// When defined, a pick fragment shader variant is generated.
    pub pick_color_qualifier: Option<String>,
    /// If true, referenced built-in functions will be included.
    pub include_built_ins: bool,
}

impl Default for ShaderSourceOptions {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            defines: Vec::new(),
            shader_type: ShaderType::Vertex,
            pick_color_qualifier: None,
            include_built_ins: true,
        }
    }
}

/// Processes and concatenates shader source code.
///
/// Mirrors the JS `ShaderSource` which handles `#define` injection,
/// `#pragma` processing, Builtin dependency resolution, and source concatenation.
pub struct ShaderSource {
    /// The raw source strings to concatenate.
    sources: Vec<String>,
    /// Preprocessor defines to inject.
    defines: Vec<String>,
    /// The shader type.
    shader_type: ShaderType,
    /// Pick color qualifier for generating pick shader variants.
    pick_color_qualifier: Option<String>,
    /// Whether to include built-in functions.
    include_built_ins: bool,
}

impl ShaderSource {
    /// Creates a new shader source processor from options.
    pub fn new(options: ShaderSourceOptions) -> Self {
        Self {
            sources: options.sources,
            defines: options.defines,
            shader_type: options.shader_type,
            pick_color_qualifier: options.pick_color_qualifier,
            include_built_ins: options.include_built_ins,
        }
    }

    /// Creates a new shader source processor (simple form).
    pub fn from_parts(sources: Vec<String>, defines: Vec<String>, shader_type: ShaderType) -> Self {
        Self {
            sources,
            defines,
            shader_type,
            pick_color_qualifier: None,
            include_built_ins: true,
        }
    }

    /// Returns a clone of this shader source.
    pub fn clone_source(&self) -> Self {
        Self {
            sources: self.sources.clone(),
            defines: self.defines.clone(),
            shader_type: self.shader_type,
            pick_color_qualifier: self.pick_color_qualifier.clone(),
            include_built_ins: self.include_built_ins,
        }
    }

    /// Returns the processed, concatenated shader source.
    ///
    /// This mirrors the JS `combineShader()` function:
    /// 1. Concatenates all source strings
    /// 2. Removes comments
    /// 3. Injects defines
    /// 4. Handles pick shader variant if needed
    pub fn process(&self) -> String {
        let mut result = String::new();

        // Concatenate sources with #line directives
        for src in &self.sources {
            result.push_str("\n#line 0\n");
            result.push_str(src);
        }

        // Remove JSDoc comments (preserve line count for debugging)
        result = remove_comments(&result);

        // Remove precision qualifiers (not needed in wgpu/Vulkan)
        result = remove_precision(&result);

        // Handle pick shader variant
        if let Some(ref qualifier) = self.pick_color_qualifier {
            if self.shader_type == ShaderType::Fragment {
                result = create_pick_fragment_shader_source(&result, qualifier);
            } else {
                result = create_pick_vertex_shader_source(&result);
            }
        }

        // Build final output
        let mut output = String::new();

        // Inject #defines (sorted for deterministic cache keys)
        let mut sorted_defines = self.defines.clone();
        sorted_defines.sort();
        for def in &sorted_defines {
            if !def.is_empty() {
                output.push_str(&format!("#define {def}\n"));
            }
        }

        output.push('\n');
        output.push_str(&result);

        output
    }

    /// Generates a cache key for this shader source configuration.
    ///
    /// Mirrors `ShaderSource.prototype.getCacheKey()`.
    pub fn get_cache_key(&self) -> String {
        let mut sorted_defines = self.defines.clone();
        sorted_defines.sort();
        let defines_key = sorted_defines.join(",");
        let pick_key = self.pick_color_qualifier.as_deref().unwrap_or("");
        let builtins_key = self.include_built_ins;
        let sources_key = self.sources.join("\n");

        format!("{defines_key}:{pick_key}:{builtins_key}:{sources_key}")
    }

    /// Creates the combined vertex shader with all dependencies and defines.
    ///
    /// Mirrors `ShaderSource.prototype.createCombinedVertexShader()`.
    pub fn create_combined_vertex_shader(&self) -> String {
        self.process()
    }

    /// Creates the combined fragment shader with all dependencies and defines.
    ///
    /// Mirrors `ShaderSource.prototype.createCombinedFragmentShader()`.
    pub fn create_combined_fragment_shader(&self) -> String {
        self.process()
    }

    /// Returns the shader type.
    pub fn shader_type(&self) -> ShaderType { self.shader_type }

    /// Returns the raw sources.
    pub fn sources(&self) -> &[String] { &self.sources }

    /// Returns the defines.
    pub fn defines(&self) -> &[String] { &self.defines }

    /// Returns the pick color qualifier.
    pub fn pick_color_qualifier(&self) -> Option<&str> {
        self.pick_color_qualifier.as_deref()
    }

    /// Returns whether builtins are included.
    pub fn include_built_ins(&self) -> bool { self.include_built_ins }

    /// Finds the normal varying name in this shader source.
    ///
    /// Mirrors `ShaderSource.findNormalVarying()`.
    pub fn find_normal_varying(&self) -> Option<&str> {
        // Check for #ifdef HAS_NORMALS pattern
        if self.contains_string("#ifdef HAS_NORMALS") {
            if self.contains_define("HAS_NORMALS") {
                return Some("v_normalEC");
            }
            return None;
        }

        let normal_names = ["v_normalEC", "v_normal"];
        for name in &normal_names {
            if self.contains_string(name) {
                return Some(name);
            }
        }
        None
    }

    /// Finds the position varying name in this shader source.
    ///
    /// Mirrors `ShaderSource.findPositionVarying()`.
    pub fn find_position_varying(&self) -> Option<&str> {
        if self.contains_string("v_positionEC") {
            Some("v_positionEC")
        } else {
            None
        }
    }

    fn contains_define(&self, define: &str) -> bool {
        self.defines.iter().any(|d| d == define)
    }

    fn contains_string(&self, string: &str) -> bool {
        self.sources.iter().any(|s| s.contains(string))
    }
}

/// Removes comments from GLSL source, preserving line count for debugging.
///
/// Mirrors the JS `removeComments()` function.
fn remove_comments(source: &str) -> String {
    let mut result = String::with_capacity(source.len());

    // Remove single-line comments
    for line in source.lines() {
        if let Some(pos) = line.find("//") {
            result.push_str(&line[..pos]);
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }

    // Remove multi-line JSDoc comments (preserve line count)
    let mut output = String::with_capacity(result.len());
    let mut in_block = false;
    for line in result.lines() {
        if in_block {
            if let Some(end_pos) = line.find("*/") {
                in_block = false;
                output.push_str(&line[end_pos + 2..]);
            }
            output.push('\n'); // Preserve line count
        } else if let Some(start_pos) = line.find("/**") {
            in_block = true;
            output.push_str(&line[..start_pos]);
            if let Some(end_pos) = line[start_pos + 3..].find("*/") {
                in_block = false;
                output.push_str(&line[start_pos + 3 + end_pos + 2..]);
            }
            output.push('\n');
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }

    output
}

/// Removes precision qualifiers (not needed in wgpu/Vulkan GLSL).
fn remove_precision(source: &str) -> String {
    // Remove patterns like "precision lowp float;", "precision mediump int;", etc.
    let mut result = source.to_string();
    let precision_patterns = [
        "precision lowp float;",
        "precision mediump float;",
        "precision highp float;",
        "precision lowp int;",
        "precision mediump int;",
        "precision highp int;",
        "precision lowp sampler2D;",
        "precision mediump sampler2D;",
        "precision highp sampler2D;",
        "precision lowp sampler3D;",
        "precision mediump sampler3D;",
        "precision highp sampler3D;",
    ];
    for pattern in &precision_patterns {
        result = result.replace(pattern, "");
    }
    result
}

/// Renames `void main()` to `void <new_name>()` in shader source.
///
/// Mirrors `ShaderSource.replaceMain()`.
pub fn replace_main(source: &str, new_name: &str) -> String {
    // Simple regex-free replacement
    let pattern = "void main()";
    let replacement = format!("void {new_name}()");
    source.replace(pattern, &replacement)
}

/// Creates a pick vertex shader variant.
///
/// Mirrors `ShaderSource.createPickVertexShaderSource()`.
fn create_pick_vertex_shader_source(vertex_shader_source: &str) -> String {
    let renamed_vs = replace_main(vertex_shader_source, "czm_old_main");
    let pick_main = "\
in vec4 pickColor;
out vec4 czm_pickColor;
void main()
{
    czm_old_main();
    czm_pickColor = pickColor;
}";

    format!("{renamed_vs}\n{pick_main}")
}

/// Creates a pick fragment shader variant.
///
/// Mirrors `ShaderSource.createPickFragmentShaderSource()`.
fn create_pick_fragment_shader_source(
    fragment_shader_source: &str,
    pick_color_qualifier: &str,
) -> String {
    let renamed_fs = replace_main(fragment_shader_source, "czm_old_main");
    let pick_main = format!(
        "{pick_color_qualifier} vec4 czm_pickColor;\n\
         void main()\n\
         {{\n\
         \x20   czm_old_main();\n\
         \x20   if (out_FragColor.a == 0.0) {{\n\
         \x20      discard;\n\
         \x20   }}\n\
         \x20   out_FragColor = czm_pickColor;\n\
         }}"
    );

    format!("{renamed_fs}\n{pick_main}")
}
