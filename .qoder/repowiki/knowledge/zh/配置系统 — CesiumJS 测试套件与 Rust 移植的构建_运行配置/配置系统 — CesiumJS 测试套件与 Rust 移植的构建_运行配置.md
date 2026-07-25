---
kind: configuration_system
name: 配置系统 — CesiumJS 测试套件与 Rust 移植的构建/运行配置
category: configuration_system
scope:
    - '**'
source_files:
    - Specs/karma.conf.cjs
    - Specs/e2e/playwright.config.js
    - gulpfile.js
    - package.json
    - cesiumrust/Cargo.toml
    - application/cesium-app/src/main.rs
    - crates/app/src/main.rs
---

本仓库包含两套独立的配置体系：前端 CesiumJS 测试套件（Jasmine/Karma + Playwright）和 Rust 移植 crate（Bevy + Cargo workspace）。两者均通过环境变量、命令行参数和配置文件组合实现运行时配置。

## 前端测试配置（Karma + Jasmine + Playwright）
- **Karma 配置**：`Specs/karma.conf.cjs` 定义浏览器启动器、文件监听、代理映射、超时等，通过 `__karma__.config.args` 接收 gulp 传入的 `includeCategory/excludeCategory/webglValidation/webglStub/release/debugCanvasWidth/debugCanvasHeight` 等参数。
- **Playwright E2E 配置**：`Specs/e2e/playwright.config.js` 使用 yargs 解析 `--update-snapshots` 参数，通过 `process.env.CI` 切换 reporter，`process.env.release` 控制是否加载 release 版本，baseURL 固定为 `http://localhost:8080`，webServer 命令为 `npm run start -- --production`。
- **Gulp 构建入口**：`gulpfile.js` 通过 yargs 解析 `--workspace/--minify/--removePragmas/--sourcemap/--node` 等参数，读取 `package.json` 中的 workspaces 和 dependencies，动态决定构建范围。
- **环境变量约定**：`DEPLOYED_URL`（部署地址）、`PROD`（生产模式）、`SANDCASTLE_ORIGIN`（Sandcastle 源）、`CESIUM_PACKAGES`/`CESIUM_VERSION`（文档生成）、`release`（E2E release 模式）、`CI`（CI 环境标志）。
- **ESLint 配置**：`eslint.config.js` 继承 `@cesium/eslint-config` 的 base/node/browser 预设。

## Rust 移植配置（Cargo Workspace + Bevy）
- **Workspace 结构**：`cesiumrust/Cargo.toml` 声明 domain/ports/adapters/application/specs 多层架构，所有 crate 共享 `workspace.dependencies`（glam 0.29、bevy 0.15、serde、tokio full、thiserror 2 等），默认成员为 `application/cesium-app`。
- **应用入口**：`application/cesium-app/src/main.rs` 使用 Bevy App 组装插件（CesiumRenderPlugin、MaterialShowcasePlugin、GeometryShowcasePlugin），窗口分辨率硬编码为 (1280, 720)。
- **日志配置**：`crates/app/src/main.rs` 调用 `env_logger::init()` 启用基于 `RUST_LOG` 环境变量的日志级别控制。
- **Profile 优化**：dev profile 开启 split-debuginfo/unpacked 和 incremental；release profile 启用 thin LTO、codegen-units=1、limited debug。
- **无外部配置文件**：Rust 侧未使用 `.env`、`.toml` 或 JSON 配置文件，所有配置通过编译期常量、命令行参数和环境变量注入。

## 开发者规范
- 前端新增测试需遵循 Karma 文件监听模式，E2E 测试通过 playwright.config.js 的 projects 多浏览器配置运行。
- Rust 新增 crate 需在 workspace members 中注册，依赖统一在 `[workspace.dependencies]` 声明。
- 敏感信息（如 GitHub Token、Google Keys、ION Token）通过 CI 环境变量注入，禁止硬编码。
- 构建参数统一通过 yargs 解析，避免散落的 `process.argv` 处理。