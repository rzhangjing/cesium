//! `gen_offline_assets` —— 确定性离线资产生成器。
//!
//! 生成 M3.1 离线 fetcher（`FileTileFetcher` / `FileTerrainFetcher`）
//! 读回时使用的磁盘 fixture：
//!
//! * 一个**影像** XYZ 金字塔，由过程化 256×256 PNG 构成，位于
//!   `specs/fixtures/offline-imagery/{level}/{x}/{y}.png`，以及
//! * 一个 **heightmap-1.0 地形** tileset（`layer.json` + `.terrain` 瓦片），位于
//!   `specs/fixtures/offline-terrain/{level}/{x}/{disk_y}.terrain`。
//!
//! 生成是瓦片坐标的纯函数 —— 无 RNG、无时钟、无网络 —— 且幂等
//! （除非传入 `--force`，否则已存在的输出保持不动）。总输出约 6 MB，
//! 远低于 50 MB 预算。
//!
//! ## 用法
//! ```text
//! gen_offline_assets [OPTIONS]
//! ```
//!
//! ## 选项
//! - `--imagery-root <PATH>`    影像输出目录（默认：`<ws>/specs/fixtures/offline-imagery`）
//! - `--terrain-root <PATH>`    地形输出目录（默认：`<ws>/specs/fixtures/offline-terrain`）
//! - `--imagery-max-level <N>`  影像最深级别（默认：3）
//! - `--terrain-max-level <N>`  地形最深级别（默认：4）
//! - `--force`                  即使输出已存在也重新生成
//! - `--verify`                 通过真实 fetcher 读回瓦片并断言成功
//! - `-h`, `--help`             显示帮助
//!
//! ## 退出码
//! - `0` 成功
//! - `2` 错误（IO 失败、参数非法、校验不一致）

mod generate;
mod verify;

use std::path::{Path, PathBuf};
use std::process;

/// 从编译期 manifest 目录解析 workspace 根，使默认 fixture 路径
/// 无论进程工作目录如何都落在 `cesiumrust/specs/fixtures`。
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <ws>/tools/gen_offline_assets
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent() // tools
        .and_then(Path::parent) // <ws>
        .unwrap_or(manifest)
        .to_path_buf()
}

struct Options {
    imagery_root: PathBuf,
    terrain_root: PathBuf,
    imagery_max_level: u32,
    terrain_max_level: u32,
    force: bool,
    verify: bool,
}

impl Options {
    fn defaults() -> Self {
        let ws = workspace_root();
        Self {
            imagery_root: ws.join("specs/fixtures/offline-imagery"),
            terrain_root: ws.join("specs/fixtures/offline-terrain"),
            imagery_max_level: generate::IMAGERY_DEFAULT_MAX_LEVEL,
            terrain_max_level: generate::TERRAIN_DEFAULT_MAX_LEVEL,
            force: false,
            verify: false,
        }
    }
}

fn print_help() {
    eprintln!(
        "Usage: gen_offline_assets [OPTIONS]\n\
         \n\
         Deterministically generate the offline imagery + heightmap-1.0 terrain\n\
         fixtures consumed by FileTileFetcher / FileTerrainFetcher.\n\
         \n\
         Options:\n\
         \x20 --imagery-root <PATH>    Imagery output dir (default: <ws>/specs/fixtures/offline-imagery)\n\
         \x20 --terrain-root <PATH>    Terrain output dir (default: <ws>/specs/fixtures/offline-terrain)\n\
         \x20 --imagery-max-level <N>  Deepest imagery level (default: 3)\n\
         \x20 --terrain-max-level <N>  Deepest terrain level (default: 4)\n\
         \x20 --force                  Regenerate even when output already exists\n\
         \x20 --verify                 Read tiles back through the fetchers and assert success\n\
         \x20 -h, --help               Show this help\n\
         \n\
         Exit codes:\n\
         \x20 0  success\n\
         \x20 2  error (IO failure, bad arguments, verification mismatch)"
    );
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut opts = Options::defaults();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--imagery-root" => {
                i += 1;
                opts.imagery_root = PathBuf::from(next_value(args, i, "--imagery-root")?);
            }
            "--terrain-root" => {
                i += 1;
                opts.terrain_root = PathBuf::from(next_value(args, i, "--terrain-root")?);
            }
            "--imagery-max-level" => {
                i += 1;
                opts.imagery_max_level = parse_level(next_value(args, i, "--imagery-max-level")?)?;
            }
            "--terrain-max-level" => {
                i += 1;
                opts.terrain_max_level = parse_level(next_value(args, i, "--terrain-max-level")?)?;
            }
            "--force" => opts.force = true,
            "--verify" => opts.verify = true,
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }
    Ok(opts)
}

fn next_value(args: &[String], i: usize, flag: &str) -> Result<String, String> {
    args.get(i)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn parse_level(value: String) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|_| format!("invalid level value: {value}"))
}

/// 将字节数格式化为人类可读的 MB 字符串（2 位小数）。
fn fmt_mb(bytes: u64) -> String {
    format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
}

fn run(opts: &Options) -> Result<(), String> {
    println!("gen_offline_assets: generating deterministic offline fixtures");

    let imagery = generate::ensure_imagery(&opts.imagery_root, opts.imagery_max_level, opts.force)
        .map_err(|e| format!("imagery generation failed: {e}"))?;
    println!(
        "  imagery : {} tiles (levels 0..={}) -> {} [{}]",
        imagery.tiles,
        opts.imagery_max_level,
        opts.imagery_root.display(),
        if imagery.skipped {
            "skipped (exists)".to_string()
        } else {
            fmt_mb(imagery.bytes)
        }
    );

    let terrain = generate::ensure_terrain(&opts.terrain_root, opts.terrain_max_level, opts.force)
        .map_err(|e| format!("terrain generation failed: {e}"))?;
    println!(
        "  terrain : {} tiles (levels 0..={}) -> {} [{}]",
        terrain.tiles,
        opts.terrain_max_level,
        opts.terrain_root.display(),
        if terrain.skipped {
            "skipped (exists)".to_string()
        } else {
            fmt_mb(terrain.bytes)
        }
    );

    if !imagery.skipped || !terrain.skipped {
        let total = imagery.bytes + terrain.bytes;
        println!("  payload : {} written this run", fmt_mb(total));
    }

    if opts.verify {
        match verify::verify(&opts.imagery_root, &opts.terrain_root, opts.terrain_max_level) {
            Ok(checks) => println!("  verify  : {checks} read-back checks PASSED"),
            Err(e) => return Err(format!("verification failed: {e}")),
        }
    }

    Ok(())
}

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        process::exit(0);
    }

    let opts = match parse_args(&raw) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {e}");
            print_help();
            process::exit(2);
        }
    };

    if let Err(e) = run(&opts) {
        eprintln!("error: {e}");
        process::exit(2);
    }
}
