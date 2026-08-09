---
kind: logging_system
name: 基于 env_logger + log 的极简日志系统
category: logging_system
scope:
    - '**'
source_files:
    - cesiumrust/crates/app/src/main.rs
    - cesiumrust/crates/app/Cargo.toml
---

## 1. 使用的框架与工具

仓库在 Rust 部分采用 `log` crate 作为统一日志接口，配合 `env_logger` 作为后端实现。这是 Rust 生态中最轻量、最标准的组合：
- `log`：定义统一的 `info!` / `debug!` / `warn!` / `error!` / `trace!` 宏和 `Logger` trait。
- `env_logger`：通过环境变量（如 `RUST_LOG=info`）控制日志级别，输出到标准错误流，无需额外配置。

该组合仅出现在应用入口 crate `crates/app` 中；domain、ports、adapters 等核心库均未引入 `log`/`env_logger` 依赖，保持领域层无日志耦合。

## 2. 关键文件

- `cesiumrust/crates/app/src/main.rs`：唯一调用 `env_logger::init()` 并写入第一条 `log::info!` 的位置。
- `cesiumrust/crates/app/Cargo.toml`：声明 `log.workspace = true` 与 `env_logger.workspace = true`。
- `cesiumrust/application/cesium-app/src/main.rs` 与 `tile_loader.rs`：仍在使用原始的 `println!` / `eprintln!` 进行调试输出，尚未迁移到 `log` 体系。

## 3. 架构与约定

- **初始化位置单一**：仅在二进制 crate 的 `main()` 开头调用 `env_logger::init()`，其他 crate 不得重复初始化。
- **日志级别策略**：当前代码仅使用 `info!` 记录启动信息，未建立统一的 level 分级规范（如 debug 用于详细流程、warn 用于可恢复异常、error 用于致命错误）。
- **结构化字段缺失**：所有日志均为纯字符串消息，没有使用 `log` 的 key-value 参数或 JSON 序列化字段，无法被外部日志聚合系统解析。
- **sink 固定为 stderr**：`env_logger` 默认输出到标准错误，没有配置文件、文件轮转、远程上报等 sink 扩展。
- **领域层解耦**：domain crates 不依赖 `log`，避免将日志实现绑定到业务逻辑；如需日志应在 adapter 层或应用层捕获并输出。

## 4. 开发者应遵循的规则

1. **不要在 domain / ports 层引入 `log` 依赖**：这些 crate 应保持无副作用、无 I/O，便于独立测试与复用。
2. **日志调用集中在 application / adapters 层**：在 Bevy 组件、网络请求、渲染管线等靠近 I/O 的地方使用 `log::info!` / `log::debug!` 等宏。
3. **不要混用 `println!` / `eprintln!`**：`application/cesium-app` 中仍有大量原始 println，应逐步替换为 `log` 宏以便统一控制级别。
4. **通过 `RUST_LOG` 控制级别**：运行前设置环境变量（如 `RUST_LOG=debug`），无需修改代码即可切换详细程度。
5. **未来扩展建议**：若需结构化日志或集中收集，可在 `main.rs` 中将 `env_logger::init()` 替换为 `tracing-subscriber` + `env_filter`，并在各 crate 中使用 `tracing` 宏，但当前阶段保持现状即可。
6. **错误路径优先用 `Result` 返回**：日志不应替代错误处理；仅在需要记录上下文时附加 `log::warn!` / `log::error!`。