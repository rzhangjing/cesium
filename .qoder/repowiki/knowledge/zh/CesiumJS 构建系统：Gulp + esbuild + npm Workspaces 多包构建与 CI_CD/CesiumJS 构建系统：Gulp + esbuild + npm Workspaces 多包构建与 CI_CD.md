---
kind: build_system
name: CesiumJS 构建系统：Gulp + esbuild + npm Workspaces 多包构建与 CI/CD
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
    - Specs/karma.conf.cjs
    - Specs/karma-main.js
    - Specs/spec-main.js
    - Tools/jsdoc/conf.json
    - gulpfile.makezip.js
    - .husky/pre-commit
    - lint-staged.config.js
---

## 1. 使用的系统与工具

- **任务编排**：`gulp`（v5）作为顶层任务入口，定义 `build`、`buildRelease`、`test`、`coverage`、`release`、`website-release`、`buildDocs`、`buildTs` 等任务。
- **打包器**：`esbuild`（v0.28）是核心打包引擎，负责 ESM/IIFE/CJS 三种格式输出、增量构建（`esbuild.context`）、Worker 打包、CSS/GLSL 处理。
- **依赖与工作区**：npm workspaces 管理根仓库及三个子包 `packages/engine`、`packages/widgets`、`packages/sandcastle`；根 `package.json` 通过 `@cesium/engine`、`@cesium/widgets` 引入工作区产物。
- **测试运行器**：Karma + Jasmine（`Specs/jasmine/`），配合 `karma-chrome-launcher`、`karma-firefox-launcher`、`karma-safari-launcher`、`karma-ie-launcher`、`karma-edge-launcher` 在多个浏览器执行；E2E 使用 Playwright（`Specs/e2e/playwright.config.js`）。
- **类型生成**：`tsd-jsdoc` + `jsdoc` 从 JSDoc 注释生成 TypeScript `.d.ts`；TypeScript 编译由 `tsc` 直接调用。
- **文档**：JSDoc 配置位于 `Tools/jsdoc/conf.json`，通过 `gulp buildDocs` 生成到 `Build/Documentation/`。
- **代码质量**：ESLint（`eslint.config.js`）、Prettier、Markdownlint、ast-grep（`.github/workflows` 中 `sg scan` / `sg test`）。
- **CI/CD**：GitHub Actions 工作流位于 `.github/workflows/`：`dev.yml`（PR/main 分支 lint/build/test/coverage）、`prod.yml`（`cesium.com` 分支发布到 AWS S3）、`deploy.yml`、`sandcastle-dev.yml`、`cla.yml`、`update-tokens.yml`。

## 2. 关键文件

- `package.json`：版本、workspaces、scripts 入口（`build`、`build-release`、`test`、`coverage`、`release`、`website-release`、`build-ts`、`make-zip` 等）。
- `gulpfile.js`：顶层 Gulp 任务，编排 `scripts/build.js` 中的 `buildEngine`、`buildWidgets`、`buildCesium`、`bundleWorkers`、`glslToJavaScript`、`createCombinedSpecList`、`runCoverage`、`test`、`buildDocs`、`release`、`postversion`、`deploySetVersion`、`deployStatus`。
- `scripts/build.js`：核心构建逻辑——ESM/IIFE/CJS 打包、GLSL→JS 转换、Worker 打包、Spec 列表生成、资产复制、CSS 打包。
- `gulpfile.apps.js`：应用级构建（`buildCesiumViewer`、`buildSandcastle`、`buildApps`）。
- `gulpfile.makezip.js`：生成 `Cesium-*.zip` 发布包。
- `Specs/karma.conf.cjs`、`Specs/karma-main.js`、`Specs/spec-main.js`、`Specs/SpecList.js`：测试运行时配置。
- `.github/workflows/dev.yml`、`.github/workflows/prod.yml`：CI 流水线。
- `Tools/jsdoc/conf.json`、`Tools/jsdoc/ts-conf.json`：文档生成配置。
- `launchers/*.launch`：VS Code 调试启动配置（`build`、`clean`、`release`、`runServer` 等）。

## 3. 架构与约定

### 3.1 多包构建流程

1. **准备阶段**：`npm run prepare`（`gulp prepare`）将 `draco_decoder.wasm`、`wasm_splats_bg.wasm`、`zip-web-worker.js`、`prism.js`、`jasmine-core` 等第三方资源复制到 `packages/engine/Source/ThirdParty/` 和 `Specs/jasmine/`。
2. **GLSL 预处理**：`glslToJavaScript()` 扫描 `packages/*/Source/Shaders/**/*.glsl`，按是否位于 `Builtin/Functions|Constants|Structs` 分类，生成对应的 JS 模块并维护 `CzmBuiltins.js` 查找表；支持增量缓存（`minifyShaders.state`）。
3. **索引生成**：`createIndexJs(workspace)` 动态遍历 workspace 源文件，生成 `packages/<workspace>/index.js`，将每个文件以文件名导出（Shader 文件前缀 `_shaders`）。
4. **打包**：`bundleIndexJs()` 用 esbuild 产出 ESM（`index.js`）；`bundleCesiumJs()` 额外产出 IIFE（`Cesium.js`，全局名 `Cesium`）和 CJS（`index.cjs`，Node 平台，`TransformStream` 被 `define` 为 `null`）。
5. **Worker 打包**：`bundleWorkers()` 将 `packages/engine/Source/Workers/**` 单独打包为 ESM（带 splitting），或在 IIFE 模式下内联为 base64 注入 `globalThis.CESIUM_WORKERS`。
6. **输出目录**：
   - 开发构建：`Build/CesiumUnminified/`（IIFE + Assets + Workers + ThirdParty）。
   - 发布构建：`Build/Cesium/`（压缩 + 移除 pragmas）。
   - 工作区产物：`packages/engine/Build/Minified|Unminified/`、`packages/widgets/Build/...`。
7. **TypeScript 声明**：`buildTs` 对 engine/widgets 分别调用 `generateTypeScriptDefinitions`，再为根 CesiumJS 生成 `Source/Cesium.d.ts`；`fixTypescriptDefinitionsSource()` 修正 `declare`→`export`、`module "Math"`→`namespace Math`、`const enum` 等。
8. **文档**：`buildDocs` 执行 `jsdoc --configure Tools/jsdoc/conf.json`，环境变量注入 `CESIUM_VERSION`、`CESIUM_PACKAGES`。

### 3.2 测试体系

- **单元测试**：`gulp test` → Karma 加载 `Build/CesiumUnminified/Cesium.js` + `Build/Specs/karma-main.js` + `Build/Specs/SpecList.js`；支持 `--include`/`--exclude` 分类过滤、`--webglValidation`、`--webglStub`、`--release`、`--browsers`、`--debug`、`--failTaskOnError`。
- **覆盖率**：`gulp coverage` 通过 `istanbul-lib-instrument` 在 esbuild 的 `onLoad` 钩子中对源码进行插桩，输出到 `Build/Coverage/`（或各 workspace 对应目录）。
- **E2E**：Playwright 配置文件 `Specs/e2e/playwright.config.js`，命令 `npm run test-e2e`。

### 3.3 发布与部署

- `npm run make-zip`：调用 `gulpfile.makezip.js` 生成 `Cesium-<version>.zip`。
- `npm run release`：顺序执行 `buildRelease`（engine → widgets → Cesium Unminified + Minified）+ 并行 `buildTs` + `buildDocs`。
- `npm run website-release`：构建非压缩版用于网站，再构建压缩版，最后生成 Sandcastle 所需的不压缩副本。
- **CI 发布**：`prod.yml` 在 `cesium.com` 分支上执行 `website-release` + `build-ts` + `build-sandcastle`，并通过 `aws s3 sync` 部署到 `cesium-website` 桶（`cesiumjs/releases/[version]/`、`ref-doc/`、`cesium-viewer/`、`cesium-sandcastle-website/`）。
- **开发 CI**：`dev.yml` 在 PR/main 上执行 eslint、markdownlint、prettier-check、build、tsc、ast-grep、coverage（FirefoxHeadless + webgl-stub）、release-tests（ChromeHeadless + webgl-stub + release 模式）、cloc、以及 Node 22/24 双版本的包验证。

## 4. 约定与约束

- **Node 版本要求**：`package.json.engines.node >= 22.0.0`，CI 固定使用 Node 22。
- **版本来源**：所有构建产物版本号来自根 `package.json.version`，通过 `getVersion()` 读取并在 banner、`Source/Cesium.js` 的 `VERSION`、`globalThis.CESIUM_VERSION` 中注入。
- **Pragmas 裁剪**：源码中使用 `//>>includeStart(...)/ //>>includeEnd(...)` 或 `//>>excludeStart(...)/ //>>excludeEnd(...)` 包裹条件代码，构建时通过自定义 `stripPragmaPlugin` 正则匹配移除。
- **增量构建**：开发时使用 `esbuild.context` + `rebuild()`，监听 `gulp.watch` 触发 shader、source、spec 变更时的局部重建。
- **工作区隔离**：`buildEngine`/`buildWidgets` 各自独立生成 `packages/*/Build/` 产物，根 `build` 会依次构建 engine → widgets → cesium，避免重复打包。
- **测试隔离**：Spec 通过 `SpecList.js` 动态聚合，`createCombinedSpecList()` 每次 spec 增删时重新生成；Karma 通过 proxies 将 `/base/Build/CesiumUnminified/Assets|ThirdParty|Widget|Workers` 映射回源码路径，使单测可直接引用源码。
- **发布包结构**：zip 包含 `Build/Cesium/`（IIFE 产物）、`Build/CesiumUnminified/`（调试用）、`Build/Documentation/`、`Build/Sandcastle*/`、`Build/CesiumViewer/` 等，由 `prod.yml` 通过 `curl` 下载 GitHub Release 的 zip 后解压再同步到 S3。
- **安全扫描**：CI 强制运行 `npm exec @ast-grep/cli sg scan --context 3` 与 `sg test`，规则位于 `Tools/ast-grep/rules/`。
- **Husky 钩子**：`.husky/pre-commit` 在提交前执行格式化检查（结合 `lint-staged.config.js`）。