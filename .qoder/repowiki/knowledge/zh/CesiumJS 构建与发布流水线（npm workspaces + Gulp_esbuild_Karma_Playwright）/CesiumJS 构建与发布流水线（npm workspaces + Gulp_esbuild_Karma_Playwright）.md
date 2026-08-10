---
kind: build_system
name: CesiumJS 构建与发布流水线（npm workspaces + Gulp/esbuild/Karma/Playwright）
category: build_system
scope:
    - '**'
source_files:
    - package.json
    - gulpfile.js
    - gulpfile.apps.js
    - scripts/build.js
    - scripts/buildSandcastle.js
    - Specs/karma.conf.cjs
    - Specs/spec-main.js
    - Specs/e2e/playwright.config.js
    - .github/workflows/prod.yml
    - .github/workflows/dev.yml
    - .github/workflows/deploy.yml
    - Tools/jsdoc/conf.json
    - Tools/jsdoc/ts-conf.json
    - cesiumrust/Cargo.toml
    - launches/release.launch
    - launches/build.launch
---

## 1. 使用的系统与方法

- **包管理与工作区**：根 `package.json` 使用 npm workspaces 管理三个子包 `packages/engine`、`packages/widgets`、`packages/sandcastle`，并通过 `workspaces` 字段声明；依赖版本集中在根 `package.json`，通过 `overrides` 统一覆盖第三方包的子依赖。
- **构建编排**：Gulp 5 (`gulpfile.js`、`gulpfile.apps.js`、`gulpfile.makezip.js`) 作为顶层任务入口，组合 esbuild、Karma、JSDoc、esbuild 插件等工具完成编译、打包、类型生成、文档生成与测试。
- **源码打包**：核心打包逻辑在 `scripts/build.js`，基于 esbuild 同时产出 ESM、IIFE、Node(CJS) 三种产物，并支持增量构建 (`esbuild.context`)、SourceMap、minify、pragma 剥离。
- **测试**：单元测试通过 Karma + Jasmine (`Specs/karma.conf.cjs`、`Specs/spec-main.js`) 在浏览器中运行；端到端截图测试通过 Playwright (`Specs/e2e/playwright.config.js`)；覆盖率由 istanbul-lib-instrument 注入并在 esbuild 加载阶段处理。
- **CI/CD**：GitHub Actions 位于 `.github/workflows/`，包含 `dev.yml`、`prod.yml`、`deploy.yml`、`sandcastle-dev.yml`、`update-tokens.yml`、`cla.yml` 等流程，分别负责 lint、构建、部署到 S3 (cesium.com / sandcastle.cesium.com)、清理 Ion token 等。
- **Rust 引擎**：`cesiumrust/` 是独立的 Rust 工程，使用 Cargo workspace (`Cargo.toml`、`Cargo.lock`) 组织 31 个领域 crate、端口适配与应用层，通过 `cargo test`、`cargo build` 独立构建，与 JS 构建管线解耦。

## 2. 关键文件与脚本

- `package.json`：版本、workspaces、scripts、engines (node >= 22)、依赖与 devDependencies。
- `gulpfile.js`：定义 `build`、`buildRelease`、`release`、`test`、`coverage`、`buildDocs`、`clean`、`prepare`、`buildTs`、`tsc`、`websiteRelease`、`deploySetVersion`、`deployStatus` 等任务。
- `scripts/build.js`：`bundleCesiumJs`、`buildEngine`、`buildWidgets`、`bundleWorkers`、`glslToJavaScript`、`createCombinedSpecList` 等核心构建函数。
- `gulpfile.apps.js`：`buildCesiumViewer`、`buildSandcastle`、`buildApps` 应用级构建。
- `Specs/karma.conf.cjs`、`Specs/spec-main.js`、`Specs/jasmine/*`：Karma/Jasmine 测试运行时。
- `Specs/e2e/playwright.config.js`：Playwright e2e 配置。
- `.github/workflows/prod.yml`、`.github/workflows/dev.yml`、`.github/workflows/deploy.yml`：CI 流水线。
- `Tools/jsdoc/conf.json`、`Tools/jsdoc/ts-conf.json`：JSDoc/TSD 文档生成配置。
- `cesiumrust/Cargo.toml`：Rust 侧 workspace 根配置。

## 3. 架构与约定

- **分层构建**：先构建底层 `@cesium/engine`，再构建 `@cesium/widgets`，最后聚合为 CesiumJS 主包 (`Source/Cesium.js` 入口)，三者顺序体现在 `gulpfile.js` 的 `build()` 与 `buildRelease` 中。
- **多目标产物**：同一份源码经 esbuild 输出 ESM (`Build/CesiumUnminified/index.js`)、IIFE (`Build/CesiumUnminified/Cesium.js`)、Node CJS (`Build/Cesium/Cesium.js`)，以及对应的 minified 版本 (`Build/Cesium`)。
- **Worker 打包**：`scripts/build.js` 中的 `bundleWorkers` 将 `packages/*/Source/ThirdParty/Workers/*.js` 单独打包到 `Build/*/Workers/`，供 IIFE/ESM 产物引用。
- **Shader 预处理**：所有 `*.glsl` 通过 `glsl-strip-comments` 在构建期转为内联 JS，路径由 `sourceFiles` 排除 `Shaders/**` 后由 watch 任务触发 `glslToJavaScript`。
- **Pragma 条件编译**：自定义 esbuild 插件 `strip-pragmas` 识别 `//>>includeStart(...)/ //>>includeEnd(...)` 注释块，release 构建时移除 debug 分支。
- **TypeScript 类型生成**：`buildTs` 调用 JSDoc → tsd-jsdoc 生成 `packages/*/index.d.ts`，再由 `createTypeScriptDefinitions` 生成根 `Source/Cesium.d.ts`；`fixTypescriptDefinitionsSource` 对输出做 post-process（如 `declare`→`export`、`const enum`→`enum` 等）。
- **版本策略**：版本号来自根 `package.json.version`，release 流程会执行 `postversion` 钩子同步更新各 workspace 的依赖版本；`deploySetVersion` 允许追加 `-<buildVersion>` 后缀用于 CI 临时构建。
- **应用构建**：`Apps/CesiumViewer` 与 `Apps/Sandcastle` 通过 `gulpfile.apps.js` 独立构建，支持 `--outer-origin`/`--inner-origin` 环境变量控制跨域 iframe 地址。
- **Rust 构建**：`cesiumrust/` 使用标准 Cargo 工作区，每个 domain crate 有独立 `Cargo.toml`，specs 通过 `cargo test` 运行，与 JS 构建完全分离。

## 4. 约定与约束

- **Node 版本要求**：`package.json.engines.node >= 22.0.0`，CI 也固定使用 Node 22。
- **构建命令约定**：`npm run build` 开发构建，`npm run build-release` 生成 release 产物，`npm run build-ts` 仅生成类型，`npm run tsc` 执行 TypeScript 类型检查，`npm run test`/`test-all`/`test-webgl`/`test-non-webgl`/`test-e2e` 区分不同测试集。
- **Workspace 范围**：构建脚本通过 `scope = "cesium"` 与 `getWorkspaces(onlyDependencies)` 动态决定参与构建的子包，避免未声明依赖的 workspace 被误构建。
- **清理约定**：`clean` 任务删除 `Build/` 目录及 `Source/Cesium.js`、`Source/Shaders/**/*.js`、`Source/**/*.d.ts`、`Specs/SpecList.js`、`Cesium-*.zip`、`cesium-*.tgz`、`packages/**/*.tgz` 等生成物。
- **CI 门禁**：`prod.yml` 的 `lint` job 依次执行 `eslint`、`markdownlint`、`prettier-check`、`build`、`tsc`、`sg scan`，全部通过后才进入部署 job。
- **部署产物**：`website-release` 产出 `Build/CesiumUnminified`、`Build/Cesium`、`Build/Documentation`、`Build/Sandcastle2`、`Build/CesiumViewer`，通过 `aws s3 sync` 推送到 cesium.com / sandcastle.cesium.com 桶。
- **覆盖率输出**：`coverage` 任务将报告写入 `Build/Coverage/<browser>/`，非 CI 环境自动打开 index.html。
- **Husky 预提交**：`.husky/pre-commit` 配合 `lint-staged.config.js` 在提交前执行格式化与 ESLint。
- **AST 规则**：`Tools/ast-grep/rules/*.yml` 定义代码风格规则，通过 `npm run sg-scan` 与 CI 的 `sg scan` 强制校验。