---
kind: configuration_system
name: CesiumJS 构建期配置与运行时配置系统
category: configuration_system
scope:
    - '**'
source_files:
    - gulpfile.js
    - scripts/build.js
    - gulpfile.apps.js
    - package.json
    - Apps/CesiumViewer/CesiumViewer.js
    - packages/sandcastle/templates/bucket.html
    - Specs/e2e/playwright.config.js
    - Specs/e2e/CesiumPage.js
    - .github/workflows/dev.yml
    - .github/workflows/prod.yml
    - Tools/jsdoc/cesium_template/publish.js
    - packages/engine/Source/Core/defined.js
---

## 概述

本仓库的“配置”并非传统意义上的应用运行时配置文件（如 `config.json`、`.env`），而是由 **构建期环境变量 + 运行时全局变量 + 构建期 pragma 剥离** 三层组成的混合配置体系，贯穿 CesiumJS（JS）、cesium-rs 与 cesiumrust（Rust）三个子工程。

## 1. 构建期配置（Build-time configuration）

### 1.1 Gulp + esbuild 构建管线
- 根 `gulpfile.js` 通过 `yargs` 解析命令行参数（`--minify`、`--removePragmas`、`--sourcemap`、`--node`、`--workspace`、`--browsers`、`--release`、`--webglStub` 等），并传递给 `scripts/build.js` 中的 esbuild 构建流程。
- `scripts/build.js` 维护一个 `pragmas = { debug: false }` 表，配合自定义插件 `strip-pragma-plugin`，在 esbuild 加载 `.js` 时匹配源码中形如 `//>>includeStart('debug', pragmas.debug); ... //>>includeEnd('debug', pragmas.debug);` 的多行代码块，根据 `removePragmas` 开关决定是否剥离。这是 CesiumJS 实现“调试/发布”两种构建变体的核心机制。
- 版本注入：`getVersion()` 读取根 `package.json` 的 `version`，并通过 `copyrightHeader.js` 模板 `${version}` 注入到每个产物头部；同时 `createCesiumJs()` / `createCombinedSpecList()` 会写入 `export const VERSION = '${version}';`。

### 1.2 环境变量（Node/GitHub Actions 侧）
| 变量 | 用途 | 来源 |
|---|---|---|
| `PROD` | 控制 Apps/Sandcastle 是否以生产模式构建（`gulpfile.apps.js` 中 `isProduction = process.env.PROD === "true"`） | GitHub Actions (`prod.yml` 顶部 `env: PROD: true`) |
| `DEPLOYED_URL` | 开发部署基址，用于生成 zip/tgz/npm 包 URL | `gulpfile.js` 第 50 行 |
| `CESIUM_VERSION` / `CESIUM_PACKAGES` | 文档生成时注入 JSDoc 模板 | `gulpfile.js` `buildDocs` 通过 `Object.assign({}, process.env, {...})` 传入 |
| `SANDCASTLE_ORIGIN` | Sandcastle 应用的 origin 覆盖 | `gulpfile.apps.js` |
| `CI` | Playwright e2e 测试行为开关（`forbidOnly: !!process.env.CI`） | `Specs/e2e/playwright.config.js` |
| `release` | e2e 测试中决定使用 minified 还是 unminified 的 Cesium 路径 | `Specs/e2e/CesiumPage.js` |
| `GITHUB_TOKEN` / `GITHUB_REPO` / `GITHUB_SHA` | 发布状态上报 GitHub API | `gulpfile.js` `setStatus` |
| `AWS_*` / `BRANCH` | 覆盖率/制品上传 S3 | `dev.yml` / `prod.yml` |
| `ION_TOKEN_CONTROLLER_TOKEN` / `ITWIN_*` / `GOOGLE_KEYS` / `INDIVIDUAL_CLA_SHEET_ID` / `CORPORATE_CLA_SHEET_ID` | CI 自动化脚本使用的密钥 | `.github/actions/*` |

这些变量全部集中在 `.github/workflows/*.yml` 和 `gulpfile*.js` 中，没有 `.env` 文件——依赖 GitHub Actions secrets 或本地 shell 环境。

## 2. 运行时配置（Runtime configuration）

### 2.1 浏览器全局 `window.CESIUM_BASE_URL`
- `Apps/CesiumViewer/CesiumViewer.js` 开头即检查 `window.CESIUM_BASE_URL`，若未设置则回退到 `../../Build/CesiumUnminified/`。
- Sandcastle 模板 `packages/sandcastle/templates/bucket.html` 将 `__CESIUM_BASE_URL__` 占位符替换为实际值，并在 `<script>` 中赋值给 `window.CESIUM_BASE_URL`。
- e2e 测试也通过 `process.env.release` 动态注入该全局变量，指向 minified/unminified 构建产物。

### 2.2 Query-string 驱动的示例应用配置
`CesiumViewer.js` 通过 `queryToObject(window.location.search.substring(1))` 解析查询参数作为运行时开关：`source`、`sourceType`、`flyTo`、`tmsImageryUrl`、`lookAt`、`stats`、`inspector`、`debug`、`theme`、`scene3DOnly`、`view`、`saveCamera` 等，属于演示/示例应用的轻量级运行时配置方式。

### 2.3 引擎内部 `defined` 工具函数
`packages/engine/Source/Core/defined.js` 提供 `defined(value)` 空值检测，被大量源码用作可选配置的守卫（例如 `if (defined(source))`）。它不是配置加载器，而是配置访问时的防御性编程约定。

## 3. Rust 侧（cesium-rs / cesiumrust）

- cesium-rs 是 JS 源码的一比一镜像移植，其 `Cargo.toml` workspace 仅声明 crate 依赖，未见集中式配置加载逻辑；运行参数通常通过 CLI 或示例程序直接构造对象。
- cesiumrust 下的 `application/cesium-app/src/main.rs` 及各 domain crate 同样未发现统一的 `Config` struct + 文件/env 加载层；配置以构造函数参数或默认值形式内联。
- 因此 Rust 侧在本仓库范围内**不存在独立的配置系统**，与 JS 侧解耦。

## 4. 架构与约定

1. **构建期 vs 运行期严格分离**：所有可变的构建选项（minify、pragma 剥离、sourcemap、目标平台）通过 `gulpfile.js` → `scripts/build.js` → esbuild 链传递，不污染源码；运行时行为通过 `window.CESIUM_BASE_URL` 与 query string 暴露。
2. **Pragma 剥离替代 `process.env.DEBUG`**：CesiumJS 不使用 Node 风格的 `process.env.*` 做运行时分支，而是用 `//>>includeStart('debug', pragmas.debug);` 注释包裹调试代码，由构建期移除，从而保证发布产物零开销。
3. **环境变量集中管理于 CI**：所有敏感信息（AWS、GitHub token、Ion token、Google keys）均通过 GitHub Actions secrets 注入，不在仓库中留存明文。
4. **无持久化配置文件**：仓库中没有 `config.json`、`.env`、`settings.yaml` 等文件；所有“配置”要么来自 npm/Gulp/esbuild 参数，要么来自浏览器全局变量/URL 查询串，要么来自 CI secrets。

## 5. 约束与规则

- 构建产物必须经过 `npm run build` / `gulp build` 流程，禁止直接复制 `packages/*/Source` 作为发布物（`gulpfile.js` 强制通过 esbuild 打包并注入版权头）。
- 发布构建必须开启 `--removePragmas`（见 `website-release` 任务链），确保调试代码被剥离。
- 文档生成必须通过 `npm run build-docs`，以便注入 `CESIUM_VERSION` 与 `CESIUM_PACKAGES`。
- 任何新增的构建期开关应优先通过 `yargs` 参数 + `scripts/build.js` 的 `pragmas` 表扩展，而非引入新的 `process.env.*` 分支。
- 运行时可配置项应通过 `window.CESIUM_BASE_URL` 或 URL 查询串暴露，避免在引擎核心中硬编码路径。

## 关键文件

- `gulpfile.js` — 主构建入口，解析 yargs 参数、调用 build、配置 Karma/JSDoc
- `scripts/build.js` — esbuild 封装、pragma 剥离插件、VERSION 注入、bundle 生成
- `gulpfile.apps.js` — Apps/Sandcastle 构建，读取 `PROD`、`SANDCASTLE_ORIGIN`
- `package.json` — 版本号、workspaces、scripts 入口
- `Apps/CesiumViewer/CesiumViewer.js` — 运行时 `window.CESIUM_BASE_URL` 与 query-string 配置
- `packages/sandcastle/templates/bucket.html` — Sandcastle 模板中 `__CESIUM_BASE_URL__` 占位
- `Specs/e2e/playwright.config.js` / `CesiumPage.js` — e2e 测试环境变量 `CI`、`release`
- `.github/workflows/dev.yml` / `prod.yml` — CI 环境变量与 secrets 注入
- `Tools/jsdoc/cesium_template/publish.js` — 文档生成读取 `CESIUM_VERSION`、`CESIUM_PACKAGES`
- `packages/engine/Source/Core/defined.js` — 运行时可选配置守卫工具