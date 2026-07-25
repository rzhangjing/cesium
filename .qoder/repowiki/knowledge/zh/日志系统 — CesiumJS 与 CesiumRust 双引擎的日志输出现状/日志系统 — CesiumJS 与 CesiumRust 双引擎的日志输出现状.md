---
kind: logging_system
name: 日志系统 — CesiumJS 与 CesiumRust 双引擎的日志输出现状
category: logging_system
scope:
    - '**'
source_files:
    - Apps/CesiumViewer/CesiumViewer.js
    - cesiumrust/application/cesium-app/src/main.rs
    - cesiumrust/Cargo.toml
    - cesiumrust/app_err.log
---

本仓库包含两套独立的代码库：CesiumJS（JavaScript）与 CesiumRust（Rust），它们各自使用不同的日志方式，且均未建立统一的日志框架或集中式日志配置。

**CesiumJS 部分（JavaScript）**
- 核心引擎源码位于 `Source/`，但当前目录仅含 `copyrightHeader.js`，实际引擎代码通过 npm 包 `@cesium/engine`、`@cesium/widgets` 引入，未在本仓库中暴露源码。
- 示例应用 `Apps/CesiumViewer/CesiumViewer.js` 在初始化失败时使用 `console.error(message)` 输出错误，并通过 `viewer.cesiumWidget.showErrorPanel()` 在 UI 上展示；调试模式下启用 `context.logShaderCompilation = true` 等 WebGL 调试开关，但未定义统一的日志级别或结构化字段。
- GitHub Actions 脚本（`.github/actions/`）直接使用 `console.log` / `console.error` 进行 CI 流程输出，属于工具脚本而非业务日志。
- 未发现任何 `log`、`tracing`、`winston`、`pino` 等第三方日志库依赖，也未见 `package.json` 中的相关依赖声明。

**CesiumRust 部分（Rust）**
- Workspace 根 `Cargo.toml` 列出了所有 crate，但未声明 `log`、`tracing`、`slog`、`env_logger`、`fern` 等任何日志 crate 依赖。
- 应用入口 `application/cesium-app/src/main.rs` 仅组装 Bevy App 和插件，未初始化任何日志后端。
- 运行时产生的日志来自底层依赖（如 `wgpu_hal::vulkan::instance`、`bevy_render::renderer`），写入到 `app_err.log` 文件，格式为 ANSI 彩色转义序列，包含时间戳、级别（WARN/INFO）、模块名和消息，这是 wgpu/Bevy 默认行为，并非项目自定义日志系统。
- 领域层 crate（`domain/*`）和适配器层（`adapters/*`）中未发现 `println!`、`eprintln!`、`dbg!` 或结构化日志调用，说明 Rust 侧目前完全未主动输出日志。

**结论**
该仓库没有实现跨语言的统一日志系统。CesiumJS 依赖浏览器原生 `console.*` API，CesiumRust 依赖 wgpu/Bevy 的默认 stderr 输出。两者均无日志级别管理、结构化字段、集中式 sink 或可配置的日志后端，不符合“logging_system”类别对系统化日志架构的要求。