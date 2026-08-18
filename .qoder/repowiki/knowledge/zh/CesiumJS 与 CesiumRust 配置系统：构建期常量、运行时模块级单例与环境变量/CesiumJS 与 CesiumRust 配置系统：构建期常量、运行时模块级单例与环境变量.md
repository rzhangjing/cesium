---
kind: configuration_system
name: CesiumJS 与 CesiumRust 配置系统：构建期常量、运行时模块级单例与环境变量
category: configuration_system
scope:
    - '**'
source_files:
    - scripts/build.js
    - gulpfile.js
    - gulpfile.apps.js
    - server.js
    - packages/engine/Source/Core/buildModuleUrl.js
    - packages/engine/Source/Core/Ion.js
    - packages/engine/Source/Core/TrustedServers.js
    - Specs/karma.conf.cjs
    - package.json
---

## 1. 整体方案

本仓库包含两套代码，配置方式截然不同：

- **CesiumJS（JavaScript/TypeScript）**：没有统一的 `Cesium.env` 或集中式配置文件。配置以**三种形态**存在——构建期常量（通过 esbuild `define`）、运行时全局变量（如 `CESIUM_BASE_URL`）、以及各模块暴露的**可写单例属性**（如 `Ion.defaultAccessToken`、`TrustedServers`）。
- **CesiumRust（Rust/Bevy）**：位于 `cesiumrust/application/cesium-app/`，采用 Rust 生态常见的 Cargo feature + 命令行参数 + 环境变量组合；未见 `.env` 文件或统一配置 crate，配置逻辑分散在各示例应用入口中。

## 2. 关键文件与位置

| 层面 | 关键文件 | 作用 |
|---|---|---|
| 构建期常量注入 | `scripts/build.js`、`gulpfile.js`、`gulpfile.apps.js` | 通过 esbuild `define` 注入 `CESIUM_BASE_URL`、`debug` pragma 等 |
| 运行时基址解析 | `packages/engine/Source/Core/buildModuleUrl.js` | 按优先级读取 `CESIUM_BASE_URL` → `import.meta.url` → RequireJS → `<script>` 标签 |
| Ion 服务配置 | `packages/engine/Source/Core/Ion.js` | 默认 token、默认服务器地址 (`https://api.cesium.com`) |
| 可信服务器白名单 | `packages/engine/Source/Core/TrustedServers.js` | 运行时注册允许发送凭据的 host:port |
| 开发/测试服务器 | `server.js`、`Specs/karma.conf.cjs` | Express 开发服务器、Karma 测试浏览器配置 |
| CI/CD 环境变量 | `gulpfile.js` 中 `process.env.GITHUB_TOKEN`、`process.env.DEPLOYED_URL`、`CESIUM_VERSION`、`CESIUM_PACKAGES` | GitHub Actions 状态上报、文档版本注入 |
| Sandcastle 应用 | `gulpfile.apps.js`、`server.js` | 通过 `SANDCASTLE_ORIGIN`、`SANDCASTLE_NO_EMBEDDINGS` 控制沙盒 origin 与嵌入生成 |
| Rust 应用入口 | `cesiumrust/application/cesium-app/src/main.rs` 及同目录示例 | Bevy App 启动、命令行参数解析 |

## 3. 架构与约定

### 3.1 CesiumJS：分层配置加载顺序

1. **构建期常量**：esbuild 在打包时通过 `define: { CESIUM_BASE_URL: '"."' }` 将常量内联到产物中（见 `gulpfile.apps.js` 第 60 行）。`removePragmas` 选项配合自定义 `stripPragmaPlugin`（`scripts/build.js` 中的 `//>>includeStart('debug', pragmas.debug)` 注释块）可在发布构建中剔除调试代码。
2. **运行时全局变量**：`buildModuleUrl.getCesiumBaseUrl()` 优先检查全局 `CESIUM_BASE_URL`，否则回退到 ESM `import.meta.url`、RequireJS `require.toUrl` 或 `<script src="Cesium.js">` 所在目录。若均不可用则抛出 `DeveloperError`。
3. **模块级单例**：每个子系统暴露一个可写的对象作为“配置点”，例如：
   - `Ion.defaultAccessToken` / `Ion.defaultServer`
   - `TrustedServers.add(host, port)` / `remove` / `clear`
   - `buildModuleUrl.setBaseUrl(value)`
4. **请求层配置**：`RequestScheduler`、`Resource` 等底层模块通过传入的 `Resource` 对象携带 base URL、headers、token 等，不依赖全局状态。

### 3.2 CesiumRust：Cargo Feature + 命令行 + 环境变量

- 使用 Cargo workspace 结构（`domain/*`、`adapters/*`、`ports/*`、`application/cesium-app`），每个 crate 通过 `Cargo.toml` 声明 feature flag。
- 应用入口 `cesiumrust/application/cesium-app/src/main.rs` 负责组装 Bevy App 并读取命令行参数（如 tileset URL、相机初始位置等）。
- 未见集中式配置 crate；跨 crate 共享的配置通过函数参数、组件字段或 trait 注入。

### 3.3 构建/测试流水线中的配置

- `gulp build` 调用 `scripts/build.js::bundleCesiumJs`，根据 `argv.minify`、`argv.removePragmas`、`argv.sourcemap`、`argv.node` 决定输出产物。
- `gulp test` 通过 Karma 启动 Chrome，把 `--include`、`--exclude`、`--webglValidation`、`--release` 等参数经 `client.args` 传给测试运行器（`gulpfile.js` 第 918–928 行）。
- `Specs/karma.conf.cjs` 固定了 Jasmine 框架、Chrome 启动器、超时等测试环境配置。
- `server.js` 提供开发服务器，监听 8080（主页面）和 8081（Sandcastle iframe mirror），并通过 chokidar 热重载 Source 变更。

## 4. 约定与约束

| 规则 | 来源/证据 |
|---|---|
| 生产构建必须设置 `CESIUM_BASE_URL` 或通过 `<script>` 引入 Cesium.js，否则 `buildModuleUrl` 抛 `DeveloperError` | `packages/engine/Source/Core/buildModuleUrl.js` 第 63–68 行 |
| Ion 访问令牌必须替换为自有账户 token；内置默认 token 仅用于评估，首次使用时会显示警告 Credit | `packages/engine/Source/Core/Ion.js` 第 6–13、39–55 行 |
| 调试代码通过 `//>>includeStart('debug', pragmas.debug)` 包裹，发布构建需启用 `removePragmas` 才能剔除 | `scripts/build.js` 第 52–91 行 |
| 文档生成通过 `CESIUM_VERSION`、`CESIUM_PACKAGES` 环境变量注入版本号与包列表 | `gulpfile.js` 第 385–389 行、`Tools/jsdoc/cesium_template/publish.js` 第 374–377 行 |
| 开发服务器端口、public 模式、是否跳过构建由 `server.js` 的 yargs 选项控制（`--port`、`--public`、`--production`） | `server.js` 第 24–45 行 |
| Sandcastle 的 outer/inner origin 可通过 `SANDCASTLE_ORIGIN` 环境变量覆盖 | `gulpfile.apps.js` 第 150–153 行、`server.js` 第 133–141 行 |
| CI 中 GitHub API 状态上报需要 `GITHUB_TOKEN`、`GITHUB_REPO`、`GITHUB_SHA` 环境变量 | `gulpfile.js` 第 529–555 行 |
| 测试分类执行通过 `--include` / `--exclude` 传递类别名给 Karma client args | `gulpfile.js` 第 918–928 行 |

## 5. 总结

CesiumJS 仓库没有传统意义上的“配置文件”（如 `config.json`、`.env`），而是采用**构建期常量 + 运行时全局变量 + 模块级可写单例**的分层模式；所有构建行为由 `gulpfile.js` + `scripts/build.js` 驱动，测试行为由 Karma 配置集中管理。CesiumRust 侧尚未形成统一的配置抽象，当前以 Cargo feature 和命令行参数为主。新增运行时配置项应优先考虑复用现有模式：对构建期常量使用 esbuild `define`，对运行时开关使用模块单例或 `process.env`，避免引入新的全局状态。