//! Shader preprocessor for CesiumJS GLSL → naga compatibility.
//!
//! CesiumJS shaders are not standalone GLSL files — they depend on:
//! 1. Builtin structs/functions/constants (142 files) prepended at runtime
//! 2. `out_FragColor` declaration injected for fragment shaders
//! 3. `layout(binding=X)` for uniform blocks
//! 4. GLSL 330 core compatibility (attribute→in, varying→out)
//!
//! This module provides the preprocessing pipeline to make CesiumJS GLSL
//! parseable by naga's GLSL frontend.

use std::path::Path;

/// Assembles all Builtin GLSL files into a single header string.
///
/// This concatenates:
/// - Builtin/Structs/*.glsl (czm_material, czm_materialInput, etc.)
/// - Builtin/Constants/*.glsl (czm_metersPerPixel, etc.)
/// - Builtin/Functions/*.glsl (czm_computePosition, czm_gammaCorrect, etc.)
///
/// Files are sorted topologically by dependency to ensure declarations
/// appear before use.
pub fn assemble_builtin_header(shaders_dir: &Path) -> String {
    let mut header = String::with_capacity(64 * 1024);

    header.push_str("// === CesiumJS Builtin Header (auto-assembled) ===\n\n");

    // Add automatic uniform declarations first
    header.push_str("// --- Automatic Uniforms ---\n");
    header.push_str(&assemble_automatic_uniforms(shaders_dir));
    header.push('\n');

    // Collect all Builtin files from all subdirectories
    let builtin_dir = shaders_dir.join("Builtin");
    if !builtin_dir.is_dir() {
        return header;
    }

    // Read all GLSL files and extract dependency info
    let mut entries: Vec<(String, String, Vec<String>, Vec<String>)> = Vec::new();

    for subdir in &["Structs", "Constants", "Functions"] {
        let dir = builtin_dir.join(subdir);
        if !dir.is_dir() {
            continue;
        }

        let files: Vec<_> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "glsl"))
            .collect();

        for entry in files {
            let path = entry.path();
            let name = path.file_stem().unwrap().to_str().unwrap().to_string();
            if let Ok(content) = std::fs::read_to_string(&path) {
                let stripped = strip_jsdoc(&content);
                let defines = extract_czm_definitions(&stripped);
                let uses = extract_czm_references(&stripped);
                entries.push((name, stripped, defines, uses));
            }
        }
    }

    // Topological sort
    let order = topological_sort(&entries);

    // Emit in sorted order
    header.push_str("// --- Builtin (topologically sorted) ---\n");
    for i in order {
        let (ref name, ref content, _, _) = entries[i];
        header.push_str(&format!("// -- {} --\n", name));
        header.push_str(content);
        header.push('\n');
    }

    header
}

/// Topologically sort GLSL files by dependency.
fn topological_sort(entries: &[(String, String, Vec<String>, Vec<String>)]) -> Vec<usize> {
    // Build name→index map for files that define czm_* symbols
    let mut definer: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (i, (_, _, defines, _)) in entries.iter().enumerate() {
        for sym in defines {
            definer.entry(sym.clone()).or_insert(i);
        }
    }

    // Build dependency graph
    let n = entries.len();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut in_degree: Vec<usize> = vec![0; n];

    for (i, (_, _, _, uses)) in entries.iter().enumerate() {
        for sym in uses {
            if let Some(&j) = definer.get(sym.as_str()) {
                if j != i {
                    adj[j].push(i);
                    in_degree[i] += 1;
                }
            }
        }
    }

    // Kahn's algorithm
    let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    for i in 0..n {
        if in_degree[i] == 0 {
            queue.push_back(i);
        }
    }

    let mut order = Vec::with_capacity(n);
    while let Some(i) = queue.pop_front() {
        order.push(i);
        for &j in &adj[i] {
            in_degree[j] -= 1;
            if in_degree[j] == 0 {
                queue.push_back(j);
            }
        }
    }

    // If cycle detected, fall back to alphabetical order
    if order.len() != n {
        let mut indices: Vec<usize> = (0..n).collect();
        indices.sort_by(|a, b| entries[*a].0.cmp(&entries[*b].0));
        order = indices;
    }

    order
}

/// Assemble automatic uniform declarations from AutomaticUniforms.js.
///
/// This parses the JS file and extracts uniform declarations like:
/// `uniform mat4 czm_viewportTransformation;`
///
/// For naga/Vulkan GLSL compatibility, uniforms are wrapped in a block.
fn assemble_automatic_uniforms(shaders_dir: &Path) -> String {
    let mut result = String::new();

    // Navigate from shaders dir to AutomaticUniforms.js
    // shaders_dir = workspace/cesium-rs/crates/cesium-shaders/shaders
    // target = workspace/packages/engine/Source/Renderer/AutomaticUniforms.js
    let auto_uniforms_path = shaders_dir
        .parent() // cesium-shaders
        .and_then(|p| p.parent()) // crates
        .and_then(|p| p.parent()) // cesium-rs
        .and_then(|p| p.parent()) // workspace root
        .map(|p| p.join("packages").join("engine").join("Source").join("Renderer").join("AutomaticUniforms.js"));

    let auto_uniforms_path = match auto_uniforms_path {
        Some(p) if p.exists() => p,
        _ => return result,
    };

    let content = match std::fs::read_to_string(&auto_uniforms_path) {
        Ok(c) => c,
        Err(_) => return result,
    };

    // Parse uniform declarations from JSDoc comments
    // Pattern: "* uniform <type> czm_<name>;"
    let mut regular_uniforms = Vec::new();
    let mut sampler_uniforms = Vec::new();
    let mut binding_counter = 0u32;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("* uniform ") && line.contains("czm_") {
            // Extract the uniform declaration
            let decl_start = line.find("uniform ").unwrap() + 8;
            let decl_end = line.find(';').unwrap_or(line.len());
            let decl = line[decl_start..decl_end].trim();

            // Separate sampler uniforms (they can't be in uniform blocks in Vulkan GLSL)
            if decl.contains("sampler") || decl.contains("image") {
                sampler_uniforms.push((binding_counter, decl.to_string()));
                binding_counter += 1;
            } else {
                regular_uniforms.push(decl.to_string());
            }
        }
    }

    // For Vulkan GLSL/naga compatibility:
    // 1. Regular uniforms go in a uniform block
    // 2. Samplers are declared separately (naga will assign bindings automatically)

    // Sampler uniforms (no explicit binding - let naga handle it)
    for (_binding, uniform) in sampler_uniforms {
        result.push_str(&format!("uniform {};\n", uniform));
    }

    // Regular uniforms in a block
    if !regular_uniforms.is_empty() {
        result.push_str("layout(binding=0, std140) uniform CesiumAutomaticUniforms {\n");
        for uniform in regular_uniforms {
            result.push_str(&format!("    {};\n", uniform));
        }
        result.push_str("};\n");
    }

    result
}

/// Append all .glsl files from a directory (sorted for determinism).
fn append_glsl_files(output: &mut String, dir: &Path) {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "glsl"))
        .collect();
    files.sort_by_key(|e| e.file_name());

    for entry in files {
        let path = entry.path();
        let name = path.file_stem().unwrap().to_str().unwrap();
        if let Ok(content) = std::fs::read_to_string(&path) {
            output.push_str(&format!("// -- {} --\n", name));
            let stripped = strip_jsdoc(&content);
            output.push_str(&stripped);
            output.push('\n');
        }
    }
}

/// Extract `czm_*` symbol definitions from GLSL source.
///
/// Matches patterns like:
/// - `const float czm_foo = ...;`
/// - `float czm_foo(...) {`
/// - `struct czm_foo {`
fn extract_czm_definitions(source: &str) -> Vec<String> {
    let mut defs = Vec::new();

    // Simple pattern matching without regex
    for line in source.lines() {
        let line = line.trim();

        // const type czm_name = ...
        if line.starts_with("const ") {
            if let Some(czm_pos) = line.find("czm_") {
                let rest = &line[czm_pos..];
                let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(rest.len());
                defs.push(rest[..end].to_string());
            }
        }

        // struct czm_name {
        if line.starts_with("struct ") {
            if let Some(czm_pos) = line.find("czm_") {
                let rest = &line[czm_pos..];
                let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(rest.len());
                defs.push(rest[..end].to_string());
            }
        }

        // type czm_name(...) — function definition
        if !line.starts_with("const ") && !line.starts_with("struct ") {
            if let Some(czm_pos) = line.find("czm_") {
                if let Some(paren_pos) = line.find('(') {
                    if czm_pos < paren_pos {
                        let rest = &line[czm_pos..paren_pos];
                        let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')
                            .unwrap_or(rest.len());
                        defs.push(rest[..end].to_string());
                    }
                }
            }
        }
    }

    defs
}

/// Extract `czm_*` symbol references (uses) from GLSL source.
fn extract_czm_references(source: &str) -> Vec<String> {
    let mut refs = Vec::new();

    for line in source.lines() {
        let mut pos = 0;
        while let Some(idx) = line[pos..].find("czm_") {
            let start = pos + idx;
            let rest = &line[start..];
            let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')
                .unwrap_or(rest.len());
            refs.push(rest[..end].to_string());
            pos = start + end;
        }
    }

    refs
}

/// Strip JSDoc-style comments that naga can't parse.
fn strip_jsdoc(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut in_block_comment = false;

    let chars: Vec<char> = source.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if in_block_comment {
            if i + 1 < len && chars[i] == '*' && chars[i + 1] == '/' {
                in_block_comment = false;
                i += 2;
            } else {
                i += 1;
            }
        } else if i + 1 < len && chars[i] == '/' && chars[i + 1] == '/' {
            // Line comment — keep it (naga handles // comments)
            while i < len && chars[i] != '\n' {
                result.push(chars[i]);
                i += 1;
            }
        } else if i + 1 < len && chars[i] == '/' && chars[i + 1] == '*' {
            in_block_comment = true;
            i += 2;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Preprocess a single GLSL shader source for naga compatibility.
///
/// This applies the following transformations:
/// 1. Prepends the Builtin header
/// 2. Injects `layout(location = 0) out vec4 out_FragColor;` for fragment shaders
/// 3. Adds `layout(binding=X)` for uniform blocks
/// 4. Adds `#version 460 core` (naga only supports Vulkan GLSL 440/450/460)
pub fn preprocess_shader(
    source: &str,
    stage: naga::ShaderStage,
    builtin_header: &str,
) -> String {
    let mut result = String::with_capacity(builtin_header.len() + source.len() + 256);

    // Version declaration — naga only supports Vulkan GLSL 440/450/460
    result.push_str("#version 460 core\n");

    // Builtin header
    result.push_str(builtin_header);

    // For fragment shaders, inject out_FragColor if needed
    if stage == naga::ShaderStage::Fragment
        && source.contains("out_FragColor")
        && !source.contains("out vec4 out_FragColor")
    {
        result.push_str("layout(location = 0) out vec4 out_FragColor;\n");
    }

    result.push('\n');

    // Normalize GLSL qualifiers: attribute→in, varying→out
    let source = normalize_qualifiers(source);

    // Inject layout(binding=X) for uniform blocks
    let source = inject_uniform_block_layouts(&source);
    result.push_str(&source);

    result
}

/// Normalize GLSL variable qualifiers for Vulkan GLSL compatibility.
///
/// Converts:
/// - `attribute` → `in` (vertex shader inputs)
/// - `varying` → `out` (vertex shader outputs / fragment shader inputs)
fn normalize_qualifiers(source: &str) -> String {
    let mut result = String::with_capacity(source.len());

    for line in source.lines() {
        let mut normalized = line.to_string();

        // Replace 'attribute' with 'in' (word boundary aware)
        normalized = normalized
            .replace("attribute ", "in ")
            .replace("\tattribute ", "\tin ");

        // Replace 'varying' with 'out' for vertex shaders or 'in' for fragment shaders
        // For now, use 'out' (works for vertex shaders)
        normalized = normalized
            .replace("varying ", "out ")
            .replace("\tvarying ", "\tout ");

        result.push_str(&normalized);
        result.push('\n');
    }

    result
}

/// Inject `layout(binding=X)` for uniform blocks that don't have it.
///
/// naga requires all uniform blocks to have explicit binding locations.
/// This function scans for `uniform <Name> { ... }` patterns and adds
/// `layout(binding=X, std140) uniform <Name> { ... }` if missing.
fn inject_uniform_block_layouts(source: &str) -> String {
    let mut result = String::with_capacity(source.len() + 256);
    let mut binding_counter = 0u32;
    let mut in_uniform_block = false;
    let mut brace_depth = 0;

    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Check if this line starts a uniform block
        if !in_uniform_block && trimmed.starts_with("uniform ") && trimmed.contains('{') {
            // Check if it already has layout()
            if !trimmed.contains("layout(") {
                // Extract the block name
                let block_name = trimmed
                    .strip_prefix("uniform ")
                    .and_then(|s| s.split('{').next())
                    .map(|s| s.trim())
                    .unwrap_or("Unknown");

                // Add layout declaration
                result.push_str(&format!(
                    "layout(binding={}, std140) uniform {} {{\n",
                    binding_counter, block_name
                ));
                binding_counter += 1;
                in_uniform_block = true;
                brace_depth = 1;
                i += 1;
                continue;
            }
        }

        // Track brace depth for uniform blocks
        if in_uniform_block {
            for ch in trimmed.chars() {
                if ch == '{' {
                    brace_depth += 1;
                } else if ch == '}' {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        in_uniform_block = false;
                    }
                }
            }
        }

        result.push_str(line);
        result.push('\n');
        i += 1;
    }

    result
}

/// Attempt to parse preprocessed GLSL with naga.
pub fn parse_with_naga(
    source: &str,
    stage: naga::ShaderStage,
) -> Result<naga::Module, String> {
    let options = naga::front::glsl::Options {
        stage,
        defines: Default::default(),
    };
    let mut parser = naga::front::glsl::Frontend::default();
    parser.parse(&options, source).map_err(|e| format!("{:?}", e))
}

/// Validate a naga module and emit WGSL.
pub fn validate_and_emit_wgsl(module: &naga::Module) -> Result<String, String> {
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(module)
    .map_err(|e| format!("validation: {:?}", e))?;

    naga::back::wgsl::write_string(
        module,
        &info,
        naga::back::wgsl::WriterFlags::empty(),
    )
    .map_err(|e| format!("wgsl emit: {:?}", e))
}
