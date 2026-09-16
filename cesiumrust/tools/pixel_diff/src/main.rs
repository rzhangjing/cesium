//! pixel_diff CLI — 像素级图像回归门工具
//!
//! ## 用法
//! ```text
//! pixel_diff <baseline.png> <candidate.png> [--threshold <dB>] [--json]
//! ```
//!
//! ## Exit codes
//! - 0: PASS (PSNR ≥ threshold)
//! - 1: FAIL (PSNR < threshold)
//! - 2: ERROR (文件不存在、尺寸不一致、解码失败等)
//!
//! ## 约定
//! - Alpha 通道被忽略，仅比对 RGB
//! - 两图尺寸必须一致，否则报错 exit 2
//! - PSNR = Infinity 时 JSON 输出 `"inf"`

use std::process;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() || args.contains(&"--help".to_owned()) || args.contains(&"-h".to_owned()) {
        eprintln!(
            "Usage: pixel_diff <baseline.png> <candidate.png> [--threshold <dB>] [--json]\n\
             \n\
             Compare two images pixel-by-pixel and report PSNR/SSIM/MAE.\n\
             \n\
             Options:\n\
             \x20 --threshold <dB>   PSNR pass threshold (default: 40.0 dB)\n\
             \x20 --json             Output in JSON format\n\
             \x20 -h, --help         Show this help\n\
             \n\
             Exit codes:\n\
             \x20 0  PASS (PSNR >= threshold)\n\
             \x20 1  FAIL (PSNR < threshold)\n\
             \x20 2  ERROR (invalid input, size mismatch, decode failure)"
        );
        process::exit(if args.is_empty() { 2 } else { 0 });
    }

    // Parse positional args and flags
    let mut positional: Vec<String> = Vec::new();
    let mut threshold: f64 = 40.0;
    let mut json_output = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--threshold" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --threshold requires a value");
                    process::exit(2);
                }
                threshold = match args[i].parse::<f64>() {
                    Ok(v) => v,
                    Err(_) => {
                        eprintln!("error: invalid threshold value: {}", args[i]);
                        process::exit(2);
                    }
                };
            }
            "--json" => {
                json_output = true;
            }
            other => {
                positional.push(other.to_owned());
            }
        }
        i += 1;
    }

    if positional.len() != 2 {
        eprintln!("error: expected exactly 2 image paths, got {}", positional.len());
        process::exit(2);
    }

    let baseline_path = &positional[0];
    let candidate_path = &positional[1];

    // Load images
    let baseline = match image::open(baseline_path) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("error: failed to load baseline '{}': {}", baseline_path, e);
            process::exit(2);
        }
    };
    let candidate = match image::open(candidate_path) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("error: failed to load candidate '{}': {}", candidate_path, e);
            process::exit(2);
        }
    };

    // Compare
    let metrics = match pixel_diff::compare_images(&baseline, &candidate, threshold) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(2);
        }
    };

    // Output
    if json_output {
        println!("{}", pixel_diff::metrics_to_json(&metrics));
    } else {
        println!("{}", pixel_diff::metrics_to_human(&metrics));
    }

    // Exit code
    process::exit(if metrics.pass { 0 } else { 1 });
}
