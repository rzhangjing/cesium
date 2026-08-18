---
kind: build_system
name: CesiumJS 与 CesiumRust 双栈构建系统：Gulp+esbuild 工作区 + Cargo Workspace
category: build_system
scope:
    - '**'
source_files:
    - package.json
    - gulpfile.js
    - scripts/build.js
    - gulpfile.apps.js
    - .github/workflows/dev.yml
    - .github/workflows/prod.yml
    - .github/workflows/deploy.yml
    - .github/workflows/sandcastle-dev.yml
    - Specs/karma.conf.cjs
    - Specs/e2e/playwright.config.js
    - Tools/jsdoc/conf.json
    - tsconfig.json
    - cesiumrust/Cargo.toml
---

## 1. 整体方案

仓库包含两个独立但并行的工程：
- **CesiumJS（JavaScript/TypeScript）**：基于 Node.js 22+，使用 Gulp 作为任务编排器、esbuild 作为打包器、Karma 运行 Jasmine 单元测试、Playwright 做 e2e 测试。
- **CesiumRust（Rust/Bevy 重写）**：基于 Rust Workspace，按 domain/ports/adapters/application/specs 分层组织，用 `cargo build/test` 编译与运行。

两者通过 GitHub Actions 在 `.github/workflows/dev.yml`、`.github/workflows/prod.yml`、`.github/workflows/deploy.yml`、`.github/workflows/sandcastle-dev.yml` 中统一编排。

## 2. 关键文件与入口

| 职责 | 文件 |
|---|---|
| npm 脚本入口 | `package.json`（`scripts.*` 暴露 `build`、`build-release`、`test`、`coverage`、`release`、`website-release`、`build-ts`、`build-sandcastle`、`build-cesium-viewer` 等） |
| 任务编排 | `gulpfile.js`（定义 `build`、`buildRelease`、`release`、`test`、`coverage`、`buildDocs`、`clean`、`tsc`、`deploySetVersion`、`deployStatus` 等 gulp 任务） |
| 核心构建逻辑 | `scripts/build.js`（`bundleCesiumJs`、`bundleWorkers`、`glslToJavaScript`、`createIndexJs`、`bundleSpecs`、`buildEngine`、`buildWidgets` 等 esbuild 管线） |
| 应用构建 | `gulpfile.apps.js`（`buildCesiumViewer`、`buildSandcastle`） |
| 测试配置 | `Specs/karma.conf.cjs`、`Specs/e2e/playwright.config.js` |
| TypeScript 类型生成 | `Tools/jsdoc/conf.json`、`Tools/jsdoc/ts-conf.json`、`tsconfig.json` |
| Rust workspace | `cesiumrust/Cargo.toml`（workspace members 声明 domain/ports/adapters/application/specs） |
| CI 流水线 | `.github/workflows/*.yml` |

## 3. 架构与约定

### 3.1 JavaScript/TypeScript 构建管线

- **包结构**：根 `package.json` 通过 npm workspaces 引用 `packages/engine`、`packages/widgets`、`packages/sandcastle`。`@cesium/engine`、`@cesium/widgets` 是内部依赖，scope 为 `cesium`。
- **源码入口**：`Source/Cesium.js` 由构建时动态生成，自动扫描 `packages/engine/Source/**/*.js` 与 `packages/widgets/Source/**/*.js` 并生成 re-export 列表（见 `scripts/build.js:createCesiumJs`）。
- **多目标产物**：同一份源码经 esbuild 产出三种格式——ESM（`index.js`）、IIFE（`Build/Cesium/Cesium.js`，全局 `Cesium`）、CommonJS（`index.cjs`，Node 环境）。Worker 代码单独打包到 `Build/*/Workers`，或在 IIFE 模式下以 base64 注入 `Build/InlineWorkers.js` 并通过 `globalThis.CESIUM_WORKERS` 注入。
- **GLSL 预处理**：所有 `*.glsl` 在构建期被 `glsl-strip-comments` 处理并转成 JS 字符串模块，输出到 `packages/*/Source/Shaders/**/*.js`，同时生成 `Builtin/CzmBuiltins.js` 注册内置函数/常量/结构体。
- **增量构建**：开发模式使用 `esbuild.context` + `gulp.watch`，监听 shader 与 source files 变化后仅 rebuild 受影响 bundle；`--incremental` 标志复用上下文。
- **调试开关**：通过自定义 `stripPragmaPlugin` 匹配 `//>>includeStart(...)` / `//>>excludeStart(...)` 注释块，配合 `pragmas.debug` 在 release 构建时剔除 debug 代码。
- **类型生成**：`buildTs` 调用 JSDoc → tsd-jsdoc 为 engine 和 widgets 生成 `.d.ts`，再对根 `Source/Cesium.js` 生成顶层类型定义；`tsc` 任务通过 `npm exec --package=typescript --offline -- tsc` 分别执行根与各 workspace 的 `tsconfig.json`。
- **测试体系**：
  - 单元测试：Karma + Jasmine，`gulp test` 先构建 `Build/CesiumUnminified`，再加载 `Specs/karma-main.js` 与 `Specs/SpecList.js`；支持 `--browsers`、`--webglValidation`、`--webglStub`、`--release`、`--all` 等参数。
  - 覆盖率：`gulp coverage` 使用 istanbul-lib-instrument 对源码 instrument 后跑 Karma，输出到 `Build/Coverage/<browser>/`。
  - E2E：Playwright，`npm run test-e2e*` 系列命令，配置文件 `Specs/e2e/playwright.config.js`。
- **应用构建**：`gulpfile.apps.js` 将 `Apps/CesiumViewer` 与 `Apps/Sandcastle` 分别用 esbuild 打包，复制静态资源到 `Build/CesiumViewer` / `Build/Sandcastle2`。

### 3.2 Rust 构建（CesiumRust）

- **Workspace 结构**：`cesiumrust/Cargo.toml` 声明 workspace members 分为四层：
  - `domain/*`：纯领域库（geospatial、time、camera、scene、tileset、gltf、material、atmosphere、widgets 等），无框架依赖。
  - `ports/driven`、`ports/driving`：trait 契约层。
  - `adapters/*`：具体实现（`bevy-render`、`decoders`、`network`）。
  - `application/cesium-app`：Bevy App 组装入口，含 examples。
  - `specs`：集成测试套件，对应 CesiumJS Specs。
- **默认成员**：`default-members = ["application/cesium-app"]`，即 `cargo build` 默认只编译示例应用。
- **依赖管理**：`[workspace.dependencies]` 集中声明 glam、bevy 0.15、tokio、ureq、image 等版本；各 crate 通过 `cesium-*` 别名引用同 workspace 内的子 crate。
- **编译优化**：`profile.release` 启用 `lto = "thin"`、`codegen-units = 1`、`debug = "limited"`；`profile.dev` 开启 `split-debuginfo = "unpacked"` 与 `incremental = true`。
- **重要约束**：glam 的 `fast-math` 特性**故意禁用**，以保证与 CesiumJS 的 IEEE-754 双舍入算术 bit-exact 一致（见注释说明）。

### 3.3 CI/CD 流水线

- **dev 流水线**（`dev.yml`）：push 到 main 或 PR 触发，依次执行 lint（eslint + markdownlint + prettier-check）、build、tsc、ast-grep 规则测试与扫描、coverage（FirefoxHeadless + webgl-stub）、release-tests（ChromeHeadless + webgl-stub + release 构建）、node-smoke-test（Node 22/24 矩阵验证 npm pack 产物）。
- **prod 流水线**（`prod.yml`）：push 到 `cesium.com` 分支触发，构建 website-release + types + sandcastle，部署到 AWS S3（`cesium-website` bucket），同步文档到 `ref-doc/`，构建 cesium-viewer 与 sandcastle 并部署到各自 bucket，最后清理旧 ion token。
- **版本策略**：版本号来自 `package.json.version`；`deploySetVersion` 支持追加 `-<buildVersion>` 后缀用于 CI 构建；`postversion` 钩子会更新所有依赖该 workspace 的 package.json 中的版本引用。

## 4. 约定与约束

- **Node 版本要求**：`package.json.engines.node >= 22.0.0`，CI 固定使用 Node 22。
- **构建产物目录**：所有 JS 构建产物统一输出到 `Build/` 下（`Build/Cesium`、`Build/CesiumUnminified`、`Build/Specs`、`Build/Coverage`、`Build/Sandcastle2`、`Build/CesiumViewer`）；各 workspace 产物位于 `packages/*/Build/`。
- **源码不可直接发布**：`package.json.exports` 将 `./Source/*.js` 设为 `null`，禁止直接 import 源码；对外发布的是 `Build/Cesium` 与 `index.cjs`。
- **第三方 Worker 与 WASM 预拷贝**：`prepare` 任务从 `node_modules` 拷贝 `draco_decoder.wasm`、`wasm_splats_bg.wasm`、`zip-web-worker.js`、`zip-module.wasm`、`prism.js`、`prism.min.css`、jasmine runner 到源码树中，确保发布包自包含。
- **Shader 文件必须放在 `Shaders/Builtin/{Functions,Constants,Structs}` 下才会被注册为内置项**，否则仅作为普通字符串模块导出。
- **Rust 侧禁止 fast-math**：`cesiumrust/Cargo.toml` 中 glam 明确禁用 `fast-math`，这是为了保证与 CesiumJS 规格一致的数值行为。
- **AST 规则检查**：通过 `@ast-grep/cli` 在 `Tools/ast-grep/rules/` 中定义规则（如 `no-export-object-freeze`、`require-jsdoc-memberof` 等），CI 中执行 `sg test` 与 `sg scan`。
- **Husky pre-commit**：`.husky/pre-commit` 在提交前触发格式化与校验（结合 `lint-staged.config.js`）。
- **Zip 发布**：`make-zip` 任务通过 `gulp -f gulpfile.makezip.js makeZip` 生成 `Cesium-<version>.zip`，供 prod 流水线下载并解压部署。
