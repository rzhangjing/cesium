---
kind: configuration_system
name: CesiumJS 构建期与运行期配置系统：环境变量、命令行参数与 CESIUM_BASE_URL
category: configuration_system
scope:
    - '**'
source_files:
    - gulpfile.js
    - scripts/build.js
    - gulpfile.apps.js
    - server.js
    - packages/engine/Source/Core/buildModuleUrl.js
    - Specs/karma-main.js
    - Specs/spec-main.js
    - Specs/e2e/playwright.config.js
    - Specs/e2e/CesiumPage.js
    - Apps/CesiumViewer/CesiumViewer.js
    - package.json
    - sgconfig.yml
---

## 1. 总体方案

CesiumJS 仓库没有引入统一的运行时配置库（如 dotenv、config、conf），而是采用**“构建期 + 运行期”双层配置**模式：
- **构建期/工具链配置**：通过 `process.env` 环境变量、`yargs` 命令行参数以及 Gulp/esbuild 的 `define` 注入，控制是否压缩、是否剥离调试代码、是否生成文档、是否启用 Sandcastle 嵌入等。
- **浏览器运行期配置**：核心引擎通过全局变量 `window.CESIUM_BASE_URL` 决定 Cesium 资源（Workers、Assets、ThirdParty）的加载基址；应用层（Apps/CesiumViewer、Sandcastle）再覆盖该值。

整个系统的入口集中在根目录的构建脚本和开发服务器中，而不是某个集中式配置文件。因此“配置”在 CesiumJS 中更多是**构建选项与环境变量的组合**，而非一个可被应用动态读取的配置对象。

## 2. 关键文件与位置

| 作用 | 关键文件 | 说明 |
|---|---|---|
| 主构建编排 | `gulpfile.js` | 定义 `build` / `buildRelease` / `test` / `coverage` / `buildDocs` 等任务，解析 `argv` 并调用 `scripts/build.js` |
| 实际打包逻辑 | `scripts/build.js` | 封装 esbuild，提供 `defaultESBuildOptions()`、`bundleCesiumJs`、`bundleWorkers`，实现 pragma 剥离与多目标构建 |
| 应用构建 | `gulpfile.apps.js` | 构建 CesiumViewer 与 Sandcastle，设置 `CESIUM_BASE_URL` define 与 `SANDCASTLE_*` 环境变量 |
| 开发服务器 | `server.js` | Express 服务，监听端口、自动增量构建、镜像 Sandcastle 页面 |
| 引擎资源定位 | `packages/engine/Source/Core/buildModuleUrl.js` | 读取 `window.CESIUM_BASE_URL`，计算 Worker/Asset 路径 |
| 测试启动器 | `Specs/karma-main.js`、`Specs/spec-main.js` | 为 Karma/Jasmine 测试设置 `window.CESIUM_BASE_URL` |
| E2E 测试 | `Specs/e2e/playwright.config.js` | 根据 `process.env.CI` 切换 reporter、retries、forbidOnly |
| CI/Actions 密钥 | `.github/actions/**/*.js` | 通过 `process.env.GITHUB_TOKEN`、`GOOGLE_KEYS`、`ION_TOKEN_CONTROLLER_TOKEN` 等注入密钥 |
| 安全扫描规则 | `sgconfig.yml`、`Tools/ast-grep/rules/*.yml` | ast-grep 规则目录与测试目录配置 |

## 3. 架构与约定

### 3.1 构建期配置来源（三层叠加）

1. **`package.json` scripts + npm workspaces**：`npm run build`、`build-release`、`test`、`test-e2e-*` 等命令统一入口，workspaces 声明 `packages/engine`、`packages/widgets`、`packages/sandcastle`。
2. **Gulp + yargs 命令行参数**：例如 `--minify`、`--removePragmas`、`--sourcemap`、`--node`、`--workspace`、`--browsers`、`--include`、`--exclude`、`--release`、`--webglStub`、`--production`、`--port`、`--public`、`--embeddings` 等，由 `yargs(process.argv).options(...)` 解析后传入构建函数。
3. **Node 进程环境变量**：
   - `PROD`：`gulpfile.apps.js` 用 `process.env.PROD === "true"` 切换生产/开发输出目录。
   - `CI`：`gulpfile.js`、`playwright.config.js` 据此决定是否打开覆盖率报告、是否 forbidOnly。
   - `DEPLOYED_URL`：`gulpfile.js` 用于构造部署状态 URL。
   - `CESIUM_VERSION`、`CESIUM_PACKAGES`：`gulpfile.js` 在 `buildDocs` 时通过 `env: Object.assign({}, process.env, { CESIUM_VERSION, CESIUM_PACKAGES })` 注入给 JSDoc。
   - `SANDCASTLE_ORIGIN`、`SANDCASTLE_NO_EMBEDDINGS`：`gulpfile.apps.js` 与 `server.js` 共同使用，覆盖 Sandcastle 的 outer/inner origin 与嵌入生成开关。
   - `release`：`Specs/e2e/CesiumPage.js` 中通过 `process.env.release` 切换使用发布版或开发版 Cesium。

### 3.2 构建期常量注入（esbuild define）

`gulpfile.apps.js` 在构建 CesiumViewer 时设置：
```js
config.define = { CESIUM_BASE_URL: `"."` };
```
这使编译后的 JS 中直接出现字符串字面量 `CESIUM_BASE_URL`，从而让 Cesium 在浏览器中以相对路径加载资源。这是将“构建期配置”注入到最终产物中的方式。

### 3.3 运行期配置（浏览器）

核心机制在 `packages/engine/Source/Core/buildModuleUrl.js`：
- 优先读取全局 `window.CESIUM_BASE_URL`；
- 若未定义，尝试从 `<script>` 标签的 `src` 推导；
- 仍失败则抛出错误提示用户设置 `CESIUM_BASE_URL`。

各环境设置该全局变量的方式不同：
- 示例应用 `Apps/CesiumViewer/CesiumViewer.js`：`window.CESIUM_BASE_URL = window.CESIUM_BASE_URL ? ... : ...`，允许外部覆盖。
- Karma 测试 `Specs/karma-main.js`：`window.CESIUM_BASE_URL = "base/Build/CesiumUnminified"`。
- Jasmine 单测 `Specs/spec-main.js`：`window.CESIUM_BASE_URL = "../Build/CesiumUnminified"`。
- E2E 测试 `Specs/e2e/CesiumPage.js`：根据 `process.env.release` 选择 `../../Build/Cesium/` 或 `../../Build/CesiumUnminified/`。

### 3.4 开发服务器配置

`server.js` 使用 `yargs` 解析 `--port`（默认 8080）、`--public`、`--production`、`--embeddings`，并通过 chokidar 监听源码变化触发增量构建。非 production 模式下还会启动第二个 8081 端口的镜像服务器，用于 Sandcastle iframe 的跨源隔离。

### 3.5 测试配置

- **Karma**：通过 `karma.conf.cjs` 与 `runCoverage` 动态注入 browsers、specReporter、coverageReporter、files/proxies。
- **Playwright**：`Specs/e2e/playwright.config.js` 根据 `process.env.CI` 切换 reporter、retries、forbidOnly，并通过 `webServer.command = "npm run start -- --production"` 复用根服务器的构建管线。

## 4. 约定与约束

| 约定/约束 | 证据来源 | 说明 |
|---|---|---|
| 构建产物基址必须通过 `CESIUM_BASE_URL` 指定 | `packages/engine/Source/Core/buildModuleUrl.js` 报错信息 | 若无法自动推断，会要求显式设置全局变量，否则 Worker/Asset 加载失败 |
| 示例应用应允许外部覆盖 `CESIUM_BASE_URL` | `Apps/CesiumViewer/CesiumViewer.js` | 先检查已有值，再回退到默认路径 |
| 生产构建需设置 `PROD=true` 以输出到 `Build/CesiumViewer` | `gulpfile.apps.js` 第 11、42 行 | 非生产输出到 `Build/Apps/CesiumViewer` |
| CI 下 Playwright 禁止 `forbidOnly` 且关闭 HTML reporter | `Specs/e2e/playwright.config.js` 第 17、32 行 | 仅本地开发才生成 HTML 报告 |
| 文档版本来自 `package.json.version` 并通过 `CESIUM_VERSION` 注入 | `gulpfile.js` 第 33-37、385-389 行 | JSDoc 模板中使用 `{version}` 占位符 |
| 工作区范围通过 `scope = "cesium"` 硬编码匹配 `@cesium/*` | `gulpfile.js` 第 28-30 行 | 与 `package.json.workspaces` 及依赖 scope 保持一致 |
| 安全令牌不入库，全部通过 CI `process.env.*` 注入 | `.github/actions/check-for-CLA/index.js`、`.github/actions/update-tokens/*.js` | 使用 `GITHUB_TOKEN`、`GOOGLE_KEYS`、`ION_TOKEN_CONTROLLER_TOKEN` 等 |
| 静态资源 MIME 类型映射需同步更新到 `web.config` | `server.js` 注释 `*NOTE* Any changes you make here must be mirrored in web.config.` | 保证 IIS 与 Node 服务器行为一致 |

## 5. 总结

CesiumJS 的配置系统可以概括为：**“无集中配置对象，靠环境变量 + 命令行参数 + esbuild define + 全局 `CESIUM_BASE_URL` 共同驱动”**。构建期通过 Gulp/yargs/esbuild 把开关编译进产物；运行期通过全局变量告诉引擎从哪里加载资源。这种设计使得同一份源码可以在开发、测试、发布、Sandcastle、CesiumViewer 等多种上下文中复用，但代价是配置散落在多个脚本文件中，需要阅读 `gulpfile.js`、`scripts/build.js`、`server.js`、`gulpfile.apps.js` 才能完整掌握所有可用开关。