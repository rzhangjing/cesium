---
kind: configuration_system
name: CesiumJS Monorepo 配置系统：基于环境变量与构建期参数的轻量编排
category: configuration_system
scope:
    - '**'
source_files:
    - gulpfile.js
    - scripts/build.js
    - server.js
    - Specs/karma.conf.cjs
    - Specs/e2e/playwright.config.js
    - gulpfile.apps.js
    - Tools/jsdoc/cesium_template/publish.js
    - .github/actions/check-for-CLA/index.js
    - .github/actions/update-tokens/iTwinShareUpdater.js
    - .github/actions/update-tokens/ionTokenDeleter.js
    - .github/actions/update-tokens/ionTokenUpdater.js
---

## 1. 采用的方式与工具链
- 没有统一的运行时配置中心或配置文件格式（无 .env、application.properties、yaml/toml 等集中式配置）。
- 通过 **Node.js 进程参数 + 环境变量** 驱动开发/测试/发布流水线，核心由 Gulp 任务与 esbuild 构建脚本组合而成。
- 关键工具：yargs（CLI 参数解析）、Karma（浏览器单元测试）、Playwright（E2E 截图回归）、esbuild（打包）、JSDoc（文档生成）。

## 2. 关键文件与入口
- 顶层编排与脚本入口
  - `gulpfile.js`：Gulp 任务总入口，聚合 build/watch/test/docs/release 等流程，并注入 CESIUM_* 环境变量给 JSDoc。
  - `scripts/build.js`：esbuild 封装层，负责 Cesium/engine/widgets 的 ESM/IIFE/CJS 产物、Worker 打包、GLSL→JS、SpecList 生成等。
  - `server.js`：本地开发服务器，监听源码变化触发增量构建，并通过 yargs 暴露 --port/--public/--production/--embeddings 等选项。
- 测试配置
  - `Specs/karma.conf.cjs`：Karma 默认配置（端口、browsers、files/proxies 等），实际运行参数由 gulpfile 动态覆盖。
  - `Specs/e2e/playwright.config.js`：Playwright 多浏览器项目定义，读取 CI 环境变量控制 reporter/retries。
- 应用/示例配置
  - `gulpfile.apps.js`：Sandcastle 示例站点构建，读取 `process.env.PROD`、`SANDCASTLE_ORIGIN` 等。
  - `Tools/jsdoc/cesium_template/publish.js`：JSDoc 模板，读取 `CESIUM_VERSION`、`CESIUM_PACKAGES` 注入到文档输出。
- GitHub Actions 侧
  - `.github/actions/check-for-CLA/index.js`、`.github/actions/update-tokens/*.js`：直接读取 `GITHUB_TOKEN`、`ION_TOKEN_CONTROLLER_TOKEN`、`GOOGLE_KEYS` 等环境变量。

## 3. 架构与约定
- 分层来源
  1) CLI 参数（yargs）：如 `--minify`、`--workspace`、`--include`、`--exclude`、`--webglValidation`、`--release`、`--debugCanvasWidth/Height` 等，用于精确控制单次任务行为。
  2) 环境变量（process.env）：区分环境/平台/CI，例如 `CI`、`DEPLOYED_URL`、`PROD`、`SANDCASTLE_ORIGIN`、`SANDCASTLE_NO_EMBEDDINGS`、`CESIUM_VERSION`、`CESIUM_PACKAGES`、各类 GitHub Action Secret。
  3) 硬编码默认值：当未提供参数/变量时，各脚本给出合理默认（如 dev server 默认 8080、Playwright 默认 chromium/firefox/webkit 三项目）。
- 构建期 vs 运行期
  - 绝大多数“配置”在构建期生效（是否 minify、是否 removePragmas、是否 node、sourcemap、iife、external 依赖等），最终产物是静态 JS/CSS/WASM，不存在运行时加载外部配置的机制。
  - 浏览器端通过全局常量 `globalThis.CESIUM_VERSION` 暴露版本信息；没有运行时 feature flag 开关。
- 工作区与包名作用域
  - 根 `package.json` 的 `workspaces` 与 `scope = 'cesium'` 配合，统一以 `@cesium/engine`、`@cesium/widgets` 引用子包，构建脚本据此决定 external 依赖与导出路径。
- 增量与缓存
  - 开发模式下通过 chokidar 监听 Source/Shaders/Specs 变更，结合 esbuild context 实现增量 rebuild；`Build/minifyShaders.state` 记录 GLSL 压缩状态，避免重复处理。

## 4. 开发者应遵循的规则
- 新增构建/运行开关优先使用 yargs 参数，并在 `gulpfile.js` / `server.js` 中声明默认值与帮助说明；仅在跨进程/跨工具共享时才考虑环境变量。
- 环境变量命名建议遵循现有风格：大写蛇形，按用途分组（`CESIUM_*`、`SANDCASTLE_*`、`DEPLOYED_URL`、`PROD`、`CI`、GitHub Action Secret 等）；敏感凭据仅出现在 CI 上下文。
- 不要引入新的运行时配置加载逻辑；如需可插拔能力，应在构建期通过 pragma 或条件编译剔除代码（参见 `stripPragmaPlugin`）。
- 修改 Karma/Playwright 行为时，保持 `karma.conf.cjs` 只放静态默认，动态项从 gulpfile 传入；Playwright 的 CI 分支判断沿用 `process.env.CI` 模式。
- 对 Sandcastle/JSDoc 等独立工具，若需注入版本/包列表，请通过 `gulpfile.js` 的 `execSync(..., { env: Object.assign({}, process.env, {...}) })` 注入，而非让工具自行查找配置文件。
- 新增环境变量后，请在对应 README/Contributors 文档中补充说明，确保贡献者能在本地复现相同构建行为。