---
kind: logging_system
name: 日志系统：基于 log + env_logger 的极简控制台输出
category: logging_system
scope:
    - '**'
source_files:
    - cesiumrust/crates/app/src/main.rs
    - cesiumrust/Cargo.toml
---

该仓库在 Rust 部分（`cesiumrust/`）仅实现了最基础的日志能力，未引入结构化日志框架或集中式日志管理。

1. **使用的框架与工具**
- `log` crate：作为统一的日志门面接口，通过 `log::info!` 等宏输出。
- `env_logger`：作为 `log` 的后端实现，负责将日志写入标准错误输出（stderr），并通过环境变量控制日志级别。
- 未在 workspace 根 `Cargo.toml` 中声明 `log` / `env_logger` 依赖，仅在应用入口 `crates/app/Cargo.toml` 中引入。

2. **关键文件**
- `crates/app/src/main.rs`：唯一调用 `env_logger::init()` 并输出第一条 `log::info!` 的位置，是全局日志后端的初始化点。
- `crates/app/Cargo.toml`：包含 `log`、`env_logger` 依赖声明（workspace 顶层未统一）。

3. **架构与约定**
- 采用“应用层一次性初始化”模式：仅在 `main` 中调用 `env_logger::init()`，domain/crates/adapters/ports 各层均不直接依赖日志后端，符合 DDD 分层解耦原则。
- 当前无自定义日志字段、无结构化 JSON 输出、无多 sink（文件/网络）路由，也未定义日志级别策略或过滤规则。
- 所有模块若需记录日志，应通过 `log` crate 的 `debug!` / `info!` / `warn!` / `error!` 宏间接使用，由 `env_logger` 根据环境变量（如 `RUST_LOG`）决定是否输出。

4. **开发者应遵循的规则**
- 不要在 domain 层引入 `env_logger` 具体实现，仅依赖 `log` 抽象；具体后端由应用层装配。
- 如需新增日志级别或结构化字段，应在应用层替换 `env_logger` 为更强大的后端（如 `tracing-subscriber`），并在 `main` 中集中配置。
- 目前仓库未对日志输出格式、时间戳、模块路径等做统一约定，建议后续在 `env_logger::init()` 处添加 `fmt::default().with_env_filter(...)` 以规范输出。