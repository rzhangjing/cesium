---
kind: configuration_system
name: CesiumJS 配置系统 — 运行时全局变量与构建期常量注入
category: configuration_system
scope:
    - '**'
source_files:
    - packages/engine/Source/Core/buildModuleUrl.js
    - scripts/build.js
    - gulpfile.js
    - gulpfile.apps.js
    - server.js
    - Apps/CesiumViewer/CesiumViewer.js
    - Specs/karma-main.js
    - Specs/spec-main.js
    - Specs/e2e/playwright.config.js
    - cesiumrust/adapters/bevy-render/src/scene_pipeline.rs
    - cesiumrust/domain/animation/src/timeline.rs
---

## 1. 系统与工具
- 构建期：esbuild + Gulp 任务，通过 `define`/`banner`/环境变量注入编译期常量。
- 运行期：浏览器全局变量 `window.CESIUM_BASE_URL`、`CESIUM_VERSION`（由构建器注入）以及 Node.js 的 `process.env.*`。
- 测试期：Karma 通过命令行参数向 `__karma__.config.args` 传递开关；Playwright 使用 `playwright.config.js` 与环境变量控制行为。
- Rust 侧（cesiumrust）：领域对象自带 Config 结构体（如 `ScenePipelineConfig`、`TimelineConfig`），通过 Bevy Resource 或函数参数传入，无统一配置加载框架。

## 2. 关键文件与位置
- 模块基址解析：`packages/engine/Source/Core/buildModuleUrl.js`（核心逻辑，读取 `CESIUM_BASE_URL` 并回退到脚本 URL / import.meta.url / RequireJS）。
- 版本注入点：`scripts/build.js`（生成 `globalThis.CESIUM_VERSION = "${version}"`，并在 IIFE banner 中写入版权头）。
- 应用入口示例：`Apps/CesiumViewer/CesiumViewer.js`（设置 `window.CESIUM_BASE_URL` 后导入 Cesium）。
- 测试启动器：
  - `Specs/karma-main.js`（根据 release 标志设置 `window.CESIUM_BASE_URL`）
  - `Specs/spec-main.js`（同上，同时解析 query string 开关）
  - `Specs/e2e/playwright.config.js`（CI 行为、release 路径等）
- 开发服务器：`server.js`（Express 服务，监听端口、自动重建、双端口镜像 Sandcastle）。
- 构建脚本：`gulpfile.js`、`gulpfile.apps.js`、`scripts/build.js`（定义 esbuild define、copy 资源、打包 Workers）。
- Rust 配置结构体：`cesiumrust/adapters/bevy-render/src/scene_pipeline.rs`（`ScenePipelineConfig`）、`cesiumrust/domain/animation/src/timeline.rs`（`TimelineConfig`）。

## 3. 架构与约定
- **运行期基址优先顺序**（`buildModuleUrl.getCesiumBaseUrl`）：
  1) 全局 `CESIUM_BASE_URL`（最常用，由宿主页面在引入 Cesium 前设置）
  2) ESM：`import.meta.url` 推导
  3) RequireJS：`require.toUrl` 回退
  4) IIFE：扫描 `<script src="Cesium.js">` 的目录
  5) 若均失败，抛出 DeveloperError 提示设置 `CESIUM_BASE_URL`
- **版本常量**：构建时通过 `globalThis.CESIUM_VERSION` 注入，引擎代码（如 `IonResource`、`ITwinPlatform`）在运行时读取该值附加到请求头/查询参数。
- **Node 环境配置**：通过 `process.env.*` 控制（如 `SANDCASTLE_NO_EMBEDDINGS`、`PROD`、`DEPLOYED_URL`、`CESIUM_PACKAGES` 等），由 `server.js`、`gulpfile.js`、`gulpfile.apps.js` 读取。
- **测试开关**：Karma 通过 `__karma__.config.args` 数组传递 category、webglValidation、webglStub、release、debugCanvasWidth/Height；spec 页面也支持 URL query string 覆盖。
- **Rust 配置模式**：每个领域模块定义自己的 Config struct，并通过构造函数或 Bevy 的 `App::insert_resource` 注入，没有统一的配置文件格式（JSON/YAML/TOML）加载器。

## 4. 开发者应遵循的规则
- **必须显式设置 `window.CESIUM_BASE_URL`**：在引入 Cesium 之前设置，否则运行时会抛错。推荐在应用入口顶部赋值，或使用构建器的 `define: { CESIUM_BASE_URL: '"..."' }` 注入。
- **不要直接修改 `buildModuleUrl` 内部状态**：如需动态切换基址，使用 `buildModuleUrl.setBaseUrl()` 而非直接操作内部变量。
- **版本相关请求需依赖 `CESIUM_VERSION`**：不要硬编码版本号，让构建器注入，保证发布包一致性。
- **Node/CI 配置走环境变量**：所有构建与测试开关都应通过 `process.env.*` 或 CLI 参数传递，避免写死在代码中。
- **Rust 新增配置项**：为新的领域功能添加对应的 `*Config` struct，并提供合理的默认值（`Default` trait），通过 Bevy Resource 或函数参数传入，不引入全局配置单例。
- **测试中覆盖不同基址场景**：在 Spec 中通过设置 `window.CESIUM_BASE_URL` 验证资源加载路径正确性，参考 `TaskProcessorSpec` 的做法。
- **避免在生产包中保留调试开关**：构建时使用 `removePragmas` 和 `minify` 清理 debug 分支，确保生产包不包含调试代码。
