//! Puncture experiment v2: test naga GLSL-in parsing WITH preprocessor.
//!
//! This tests the full preprocessing pipeline:
//! 1. Assemble Builtin header (142 files)
//! 2. Prepend to each shader
//! 3. Inject out_FragColor for fragment shaders
//! 4. Parse with naga → validate → emit WGSL

use std::path::Path;

fn shader_stage_from_filename(name: &str) -> Option<naga::ShaderStage> {
    if name.ends_with("VS.glsl") {
        Some(naga::ShaderStage::Vertex)
    } else if name.ends_with("FS.glsl") {
        Some(naga::ShaderStage::Fragment)
    } else if name.ends_with("CS.glsl") {
        Some(naga::ShaderStage::Compute)
    } else {
        None
    }
}

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
fn puncture_v2_with_preprocessor() {
    let shaders_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("shaders");
    assert!(shaders_dir.exists(), "shaders/ directory not found");

    // Step 1: Assemble Builtin header
    let builtin_header = cesium_shaders::preprocessor::assemble_builtin_header(&shaders_dir);
    eprintln!("Builtin header size: {} bytes", builtin_header.len());

    let files = collect_shader_files(&shaders_dir);
    assert!(!files.is_empty());

    let mut parse_ok = 0u32;
    let mut parse_fail = 0u32;
    let mut validate_ok = 0u32;
    let mut validate_fail = 0u32;
    let mut wgsl_ok = 0u32;
    let mut wgsl_fail = 0u32;
    let mut skipped = 0u32;

    let mut parse_failures = Vec::new();
    let mut validate_failures = Vec::new();

    for path in &files {
        let name = path.file_name().unwrap().to_str().unwrap();
        let source = std::fs::read_to_string(path).unwrap();

        let Some(stage) = shader_stage_from_filename(name) else {
            skipped += 1;
            continue;
        };

        // Preprocess
        let preprocessed = cesium_shaders::preprocessor::preprocess_shader(
            &source, stage, &builtin_header,
        );

        // Parse
        let module = match cesium_shaders::preprocessor::parse_with_naga(&preprocessed, stage) {
            Ok(m) => {
                parse_ok += 1;
                m
            }
            Err(e) => {
                parse_fail += 1;
                // Truncate error for readability
                let short_err = if e.len() > 200 { &e[..200] } else { &e };
                parse_failures.push(format!("{}: {}", name, short_err));
                continue;
            }
        };

        // Validate + emit WGSL
        match cesium_shaders::preprocessor::validate_and_emit_wgsl(&module) {
            Ok(_wgsl) => {
                validate_ok += 1;
                wgsl_ok += 1;
            }
            Err(e) => {
                if e.contains("validation") {
                    validate_fail += 1;
                } else {
                    validate_ok += 1;
                    wgsl_fail += 1;
                }
                let short_err = if e.len() > 200 { &e[..200] } else { &e };
                validate_failures.push(format!("{}: {}", name, short_err));
            }
        }
    }

    let total = files.len();
    let parseable = parse_ok + parse_fail;

    eprintln!("\n=== Puncture v2 Results (with preprocessor) ===");
    eprintln!("Total GLSL files:     {}", total);
    eprintln!("Skipped (includes):   {}", skipped);
    eprintln!("Parseable shaders:    {}", parseable);
    eprintln!("  Parse OK:           {}", parse_ok);
    eprintln!("  Parse FAIL:         {}", parse_fail);
    eprintln!("  Validate OK:        {}", validate_ok);
    eprintln!("  Validate FAIL:      {}", validate_fail);
    eprintln!("  WGSL emit OK:       {}", wgsl_ok);
    eprintln!("  WGSL emit FAIL:     {}", wgsl_fail);
    if parseable > 0 {
        eprintln!("Parse success rate:   {:.1}%", parse_ok as f64 / parseable as f64 * 100.0);
        eprintln!("Full pipeline rate:   {:.1}%", wgsl_ok as f64 / parseable as f64 * 100.0);
    }

    if !parse_failures.is_empty() {
        eprintln!("\n--- Parse failures (first 15) ---");
        for f in parse_failures.iter().take(15) {
            eprintln!("  {}", f);
        }
    }

    if !validate_failures.is_empty() {
        eprintln!("\n--- Validate/emit failures (first 15) ---");
        for f in validate_failures.iter().take(15) {
            eprintln!("  {}", f);
        }
    }

    // Diagnostic test — always passes
}
