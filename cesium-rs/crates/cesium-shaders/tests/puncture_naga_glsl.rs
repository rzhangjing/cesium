//! Puncture experiment: test naga GLSL-in parsing of all 318 CesiumJS shaders.
//!
//! This is the M2 go/no-go decision point. We try to parse every GLSL file
//! with naga's GLSL frontend and record success/failure rates.

use std::path::Path;

/// Attempt to parse a GLSL source string with naga.
fn try_parse_glsl(source: &str, stage: naga::ShaderStage, entry: &str) -> Result<naga::Module, String> {
    let options = naga::front::glsl::Options {
        stage,
        defines: Default::default(),
    };
    let mut parser = naga::front::glsl::Frontend::default();
    let module = parser.parse(&options, source).map_err(|errors| {
        format!("  {:?}", errors)
    })?;
    Ok(module)
}

/// Attempt to translate a parsed naga module to WGSL.
fn try_emit_wgsl(module: &naga::Module) -> Result<String, String> {
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(module)
    .map_err(|e| format!("  validation: {:?}", e))?;

    let wgsl = naga::back::wgsl::write_string(
        module,
        &info,
        naga::back::wgsl::WriterFlags::empty(),
    )
    .map_err(|e| format!("  wgsl emit: {:?}", e))?;

    Ok(wgsl)
}

/// Determine shader stage from filename convention.
fn shader_stage_from_filename(name: &str) -> Option<naga::ShaderStage> {
    if name.ends_with("VS.glsl") {
        Some(naga::ShaderStage::Vertex)
    } else if name.ends_with("FS.glsl") {
        Some(naga::ShaderStage::Fragment)
    } else if name.ends_with("CS.glsl") {
        Some(naga::ShaderStage::Compute)
    } else {
        // Shared includes (e.g. AtmosphereCommon.glsl, PolylineCommon.glsl)
        None
    }
}

/// Collect all .glsl files from the shaders directory.
fn collect_shader_files(root: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if root.is_dir() {
        for entry in std::fs::read_dir(root).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_shader_files(&path));
            } else if path.extension().is_some_and(|e| e == "glsl") {
                files.push(path);
            }
        }
    }
    files
}

#[test]
fn puncture_naga_parse_all_shaders() {
    let shaders_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("shaders");
    assert!(shaders_dir.exists(), "shaders/ directory not found");

    let files = collect_shader_files(&shaders_dir);
    assert!(!files.is_empty(), "No .glsl files found");

    let mut parse_ok = 0u32;
    let mut parse_fail = 0u32;
    let mut validate_ok = 0u32;
    let mut validate_fail = 0u32;
    let mut wgsl_ok = 0u32;
    let mut wgsl_fail = 0u32;
    let mut skipped = 0u32;

    let mut failures = Vec::new();

    for path in &files {
        let name = path.file_name().unwrap().to_str().unwrap();
        let source = std::fs::read_to_string(path).unwrap();

        let Some(stage) = shader_stage_from_filename(name) else {
            skipped += 1;
            continue;
        };

        // Try parse
        let module = match try_parse_glsl(&source, stage, "main") {
            Ok(m) => {
                parse_ok += 1;
                m
            }
            Err(e) => {
                parse_fail += 1;
                failures.push(format!("PARSE FAIL {}: {}", name, e.trim()));
                continue;
            }
        };

        // Try validate + emit WGSL
        match try_emit_wgsl(&module) {
            Ok(_wgsl) => {
                validate_ok += 1;
                wgsl_ok += 1;
            }
            Err(e) => {
                // Separate validation vs emit failures
                if e.contains("validation") {
                    validate_fail += 1;
                } else {
                    validate_ok += 1; // passed validation but failed emit
                    wgsl_fail += 1;
                }
                failures.push(format!("VALIDATE/EMIT FAIL {}: {}", name, e.trim()));
            }
        }
    }

    let total = files.len();
    let parseable = parse_ok + parse_fail;

    // Print summary
    eprintln!("\n=== Naga GLSL-in Puncture Experiment Results ===");
    eprintln!("Total GLSL files:     {}", total);
    eprintln!("Skipped (includes):   {}", skipped);
    eprintln!("Parseable shaders:    {}", parseable);
    eprintln!("  Parse OK:           {}", parse_ok);
    eprintln!("  Parse FAIL:         {}", parse_fail);
    eprintln!("  Validate OK:        {}", validate_ok);
    eprintln!("  Validate FAIL:      {}", validate_fail);
    eprintln!("  WGSL emit OK:       {}", wgsl_ok);
    eprintln!("  WGSL emit FAIL:     {}", wgsl_fail);
    eprintln!("Parse success rate:   {:.1}%", parse_ok as f64 / parseable as f64 * 100.0);
    eprintln!("Full pipeline rate:   {:.1}%", wgsl_ok as f64 / parseable as f64 * 100.0);

    if !failures.is_empty() {
        eprintln!("\n--- First 20 failures ---");
        for f in failures.iter().take(20) {
            eprintln!("{}", f);
        }
    }

    // The test always passes — it's a diagnostic, not a gate.
    // The real go/no-go decision is based on the printed numbers.
}

#[test]
fn puncture_simplest_vertex_shader() {
    // The absolute simplest shader: ViewportQuadVS.glsl
    let source = r#"
in vec4 position;
in vec2 textureCoordinates;
out vec2 v_textureCoordinates;
void main() {
    gl_Position = position;
    v_textureCoordinates = textureCoordinates;
}
"#;
    let module = try_parse_glsl(source, naga::ShaderStage::Vertex, "main")
        .expect("ViewportQuadVS should parse with naga");

    // Note: validation may fail with BindingCollision because naga assigns
    // the same location to multiple attributes. This is expected for raw
    // CesiumJS GLSL — the preprocessor will fix this in production.
    match try_emit_wgsl(&module) {
        Ok(wgsl) => {
            eprintln!("=== ViewportQuadVS.glsl → WGSL ===");
            eprintln!("{}", wgsl);
        }
        Err(e) => {
            eprintln!("=== ViewportQuadVS.glsl validation/emit failed (expected) ===");
            eprintln!("{}", e);
        }
    }
}
