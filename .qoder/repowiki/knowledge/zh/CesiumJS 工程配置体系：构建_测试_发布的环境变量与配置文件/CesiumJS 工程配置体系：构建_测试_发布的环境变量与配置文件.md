---
kind: configuration_system
name: CesiumJS 工程配置体系：构建/测试/发布的环境变量与配置文件
category: configuration_system
scope:
    - '**'
source_files:
    - package.json
    - gulpfile.js
    - gulpfile.apps.js
    - server.js
    - Specs/e2e/playwright.config.js
    - Specs/karma.conf.cjs
    - Tools/jsdoc/conf.json
    - sgconfig.yml
    - tsconfig.json
    - eslint.config.js
    - cesiumrust/Cargo.toml
    - cesiumrust/application/cesium-app/Cargo.toml
---

## 1. 使用的系统与工具

本仓库的“配置系统”并非运行时应用配置，而是围绕 **构建、测试、文档生成与发布** 的全局环境配置，采用以下手段分层管理：

- **npm workspaces + package.json scripts**：根 `package.json` 定义 `workspaces: ["packages/engine", "packages/widgets", "packages/sandcastle"]`，并通过 `scripts` 暴露 `build`、`test`、`coverage`、`release`、`build-docs`、`build-apps`、`make-zip`、`deploy-status`、`deploy-set-version` 等统一入口。
- **Gulp 任务编排**：`gulpfile.js`、`gulpfile.apps.js`、`gulpfile.makezip.js` 是核心配置中枢，通过 `yargs(process.argv)` 解析命令行参数（如 `--minify`、`--removePragmas`、`--sourcemap`、`--node`、`--workspace`、`--browsers`、`--include`、`--exclude`、`--webglValidation`、`--webglStub`、`--release`、`--debug`、`--production`），并组合出构建选项对象传入 `scripts/build.js` 中的 `buildEngine` / `buildWidgets` / `buildCesium` / `bundleWorkers`。
- **Node 进程环境变量 (`process.env`)**：所有跨脚本共享的开关均通过环境变量注入，而非 `.env` 文件。
- **Karma + Playwright**：单元测试由 Karma 驱动（`Specs/karma.conf.cjs` 被 `karma.config.parseConfig` 动态加载），端到端截图测试由 Playwright 驱动（`Specs/e2e/playwright.config.js`）。
- **JSDoc/TSD 文档生成**：`Tools/jsdoc/conf.json` 配合 `gulp buildDocs` 注入 `CESIUM_VERSION`、`CESIUM_PACKAGES` 环境变量。
- **AST 规则扫描**：`sgconfig.yml` 将 ast-grep 规则目录指向 `Tools/ast-grep/rules`。
- **Rust 侧**：`cesiumrust/Cargo.toml` 以 workspace 形式声明 31 个 domain crate、ports、adapters、application、specs；`[profile.dev]` / `[profile.release]` 控制编译期行为（`split-debuginfo`、`incremental`、`lto`、`codegen-units`）。应用层 `cesiumrust/application/cesium-app/Cargo.toml` 仅声明依赖，不内嵌配置。

## 2. 关键文件与位置

| 类别 | 关键文件 | 作用 |
|---|---|---|
| 顶层工作区 | `package.json` | 版本、workspaces、devDependencies、`scripts` 入口、`engines.node >= 22` |
| 构建编排 | `gulpfile.js` | 主 Gulp 任务：build / test / coverage / release / buildDocs / deployStatus / deploySetVersion |
| 应用构建 | `gulpfile.apps.js` | CesiumViewer 与 Sandcastle 构建，读取 `PROD`、`SANDCASTLE_ORIGIN`、`SANDCASTLE_NO_EMBEDDINGS` |
| 开发服务器 | `server.js` | Express 开发服务器，`--port`、`--public`、`--production`、`--embeddings` 参数，监听 8080/8081 |
| E2E 测试 | `Specs/e2e/playwright.config.js` | 多浏览器项目（chromium/firefox/webkit）、`baseURL http://localhost:8080`、`CI` 下禁用 `forbidOnly` |
| 单元测试 | `Specs/karma.conf.cjs` | Karma 启动器、文件列表、代理映射、覆盖率输出 |
| 文档生成 | `Tools/jsdoc/conf.json` | JSDoc 配置，配合 `CESIUM_VERSION`、`CESIUM_PACKAGES` 注入 |
| AST 规则 | `sgconfig.yml` | ast-grep 规则目录与测试目录 |
| TypeScript | `tsconfig.json` | `moduleResolution: bundler`、`noEmit`、`strict`、`checkJs: false` |
| ESLint | `eslint.config.js` | 基于 `@cesium/eslint-config` 的 browser/node/sandcastle/specs 多段配置 |
| Rust workspace | `cesiumrust/Cargo.toml` | workspace members、workspace.dependencies、profile.dev/release |
| Rust 应用 | `cesiumrust/application/cesium-app/Cargo.toml` | 组装层依赖声明 |

## 3. 架构与约定

### 3.1 配置来源优先级

1. **命令行参数**（`yargs`）：最高优先级。例如 `gulp build --minify --removePragmas --sourcemap=false --node=false`、`gulp test --browsers Chrome,Firefox --include WebGL --grep MySpec`、`node server.js --port 9000 --public --no-embeddings`、`playwright test -u`。
2. **环境变量**：次级覆盖。例如 `PROD=true` 切换生产构建路径、`DEPLOYED_URL` 决定部署状态 URL、`CI` 改变 Playwright reporter 与 `forbidOnly`、`CESIUM_VERSION` / `CESIUM_PACKAGES` 注入文档生成、`SANDCASTLE_ORIGIN` 覆盖 Sandcastle 内外 origin、`SANDCASTLE_NO_EMBEDDINGS=1` 跳过语义搜索 embedding 生成、`GITHUB_TOKEN` / `GITHUB_REPO` / `GITHUB_SHA` 控制 GitHub Status API 调用。
3. **配置文件**：作为默认值。`package.json` 的 `scripts`、`workspaces`、`overrides`；`tsconfig.json`；`eslint.config.js`；`sgconfig.yml`；`Specs/karma.conf.cjs`；`Specs/e2e/playwright.config.js`；`Tools/jsdoc/conf.json`；`cesiumrust/Cargo.toml` 的 profile。
4. **硬编码默认值**：如 `server.js` 默认端口 `8080`、Playwright `baseURL` 固定为 `http://localhost:8080`、`gulpfile.js` 中 `scope = "cesium"`、`CESIUM_BASE_URL: "."` 在 esbuild define 中写入。

### 3.2 构建期配置传递链

```
yargs(argv) → buildOptions 对象 → scripts/build.js (buildEngine/buildWidgets/buildCesium/bundleWorkers)
                ↓
            esbuild / gulp / karma / jsdoc / playwright / cargo
```

- `gulp build` 把 `argv.minify`、`argv.removePragmas`、`argv.sourcemap`、`argv.node` 封装成 `buildOptions` 传给 `buildEngine` / `buildWidgets` / `buildCesium`。
- `gulp buildRelease` 顺序执行：先 `buildCesium({ node: true, sourcemap: false })` 生成 Node 可用产物，再 `buildCesium({ minify: true, removePragmas: true })` 生成压缩版。
- `gulp websiteRelease` 额外执行 `buildDocs`，并把 `CESIUM_VERSION`、`CESIUM_PACKAGES` 注入到 `process.env` 传给 jsdoc。
- `gulp buildDocsWatch` 复用 `buildDocs` 并 watch sourceFiles。

### 3.3 测试配置

- **单元/集成测试**：`gulp test` 通过 `karma.config.parseConfig(karmaConfigFile, { ... })` 动态注入 `browsers`、`specReporter`、`files`、`proxies`、`client.args`（包含 include/exclude category、webglValidation、webglStub、release、debugCanvasWidth/Height、grep）。
- **覆盖率**：`gulp coverage` 使用 `istanbul-lib-instrument` 对 `packages/*/Source/**/*.js` 注入 instrumenter，输出到 `Build/Coverage` 或 `packages/*/Build/Coverage`。
- **E2E 测试**：`playwright.config.js` 定义 chromium/firefox/webkit 三个 project，`webServer.command = "npm run start -- --production"` 自动拉起开发服务器，`reporter` 在非 CI 下输出 HTML 报告到 `Build/Specs/e2e/report`。

### 3.4 Rust 侧配置

- 通过 `Cargo.toml` workspace 集中声明 31 个 domain crate、2 个 ports crate、3 个 adapters crate、1 个 application crate 和 specs crate。
- `default-members = ["application/cesium-app"]` 使 `cargo build` 默认只编译应用。
- `profile.dev` 开启 `incremental`、`split-debuginfo = unpacked`；`profile.release` 开启 `lto = thin`、`codegen-units = 1`、`debug = limited`。
- 领域 crate 之间通过 `path = "domain/*"` 引用，不引入外部配置框架，保持纯 Rust 可测试性。

## 4. 约定与约束

| 约定 | 证据位置 | 说明 |
|---|---|---|
| 新增构建开关应通过 `yargs` 解析后进入 `buildOptions` | `gulpfile.js` 多处 `const xxx = argv.xxx ?? default` | 避免直接散落 `process.env` 判断 |
| 环境变量名遵循大写常量风格 | `process.env.PROD`、`DEPLOYED_URL`、`CI`、`CESIUM_VERSION`、`CESIUM_PACKAGES`、`SANDCASTLE_ORIGIN`、`SANDCASTLE_NO_EMBEDDINGS`、`GITHUB_TOKEN`、`ITWIN_SERVICE_APP_CLIENT_ID` | 全局可识别 |
| 开发服务器端口默认 8080，Sandcastle 镜像服务固定 8081 | `server.js` yargs 默认值与 `app.listen(8081, "localhost")` | 不可随意更改，否则 e2e 测试 baseURL 失效 |
| E2E 测试必须依赖本地 `http://localhost:8080` 已启动 | `Specs/e2e/playwright.config.js` 中 `baseURL` 与 `webServer.command` | 测试前需 `npm run start -- --production` |
| 文档版本号来自根 `package.json.version`，经 `postversion` 钩子传播 | `gulpfile.js` 读取 `packageJson.version`，`postversion` 遍历 `./package.json` 与 `./packages/*/package.json` 更新依赖 | 版本变更需走 npm lifecycle |
| 新 AST 规则放入 `Tools/ast-grep/rules`，对应测试放入 `Tools/ast-grep/tests` | `sgconfig.yml` | 通过 `npm run sg-scan` 统一执行 |
| Rust 领域 crate 不得引入框架依赖 | `cesiumrust/Cargo.toml` workspace 成员注释 “Domain layer (pure Rust, no framework dependency)” | 保证可独立测试 |
| 引擎模块命名 scope 固定为 `cesium` | `gulpfile.js` 顶部 `const scope = "cesium"` | 与 `@cesium/engine`、`@cesium/widgets` 包名绑定 |
| Node 版本要求 ≥ 22 | `package.json.engines.node` | 旧 Node 无法运行构建脚本 |

## 5. 结论

该仓库没有传统意义上的“运行时配置中心”，而是将配置集中在 **构建/测试/发布阶段**：通过 `package.json` 脚本入口、`gulpfile.js` 任务编排、`yargs` 命令行参数、`process.env` 环境变量以及各工具的配置文件（Karma、Playwright、JSDoc、ESLint、ast-grep、Cargo）共同构成一个层次化的配置系统。新增配置项应优先选择命令行参数，其次才是环境变量，最后才考虑修改静态配置文件。