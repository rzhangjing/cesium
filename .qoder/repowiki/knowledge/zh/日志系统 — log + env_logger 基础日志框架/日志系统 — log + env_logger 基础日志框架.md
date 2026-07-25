---
kind: logging_system
name: 日志系统 — log + env_logger 基础日志框架
category: logging_system
scope:
    - '**'
source_files:
    - cesiumrust/crates/app/src/main.rs
    - cesiumrust/Cargo.toml
    - Specs/spec-main.js
---

本仓库包含两套代码：CesiumJS 原生 JavaScript 测试套件（Specs）与 cesiumrust Rust 移植集成测试。在日志系统方面，两者均没有专门的日志框架或结构化日志配置，仅使用最基础的输出方式。

**Rust 侧（cesiumrust）**
- 仅在 `crates/app/src/main.rs` 中初始化了 `env_logger::init()` 并使用 `log::info!` 输出一条启动信息，属于最小化示例用法。
- 工作区依赖通过 `[workspace.dependencies]` 声明了 `log` 和 `env_logger`，但 domain、adapters、ports 等核心 crate 均未引入日志依赖。
- 其余 Rust 代码中未出现 `println!`、`eprintln!`、`dbg!` 以外的日志调用，错误处理主要依赖 `thiserror` 的 `#[error(...)]` 派生宏返回 Result，而非日志记录。
- 根目录存在 `app_err.log` 和 `app_out.log` 两个空日志文件，表明可能有外部进程重定向 stdout/stderr，但仓库内无相关配置。

**JavaScript 侧（Specs）**
- CesiumJS 源码本身未引入第三方日志库；测试入口 `Specs/spec-main.js` 仅配置 Jasmine/Karma 环境，无日志初始化。
- GitHub Actions 脚本中使用 `console.log` / `console.error` 进行 CI 输出，属于工具脚本临时打印，非应用日志。
- Specs 测试套件中未发现统一的日志断言或日志收集机制。

**结论**
该仓库在当前状态下**没有建立正式的日志系统**。Rust 部分仅在最外层 binary 中演示了 `log` + `env_logger` 的最小用法，domain 层未接入任何日志框架；JavaScript 部分完全依赖浏览器/Node 的原生 `console.*` 输出，无结构化日志、无级别管理、无统一 sink。若需完善，应在 workspace 顶层统一引入 `tracing` 或 `log4rs`，并在各 crate 中按模块划分日志类别与级别。