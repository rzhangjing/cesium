---
kind: build_system
name: 构建与发布系统：Gulp + esbuild + Cargo Workspace 双栈流水线
category: build_system
scope:
    - '**'
source_files:
    - package.json
    - gulpfile.js
    - scripts/build.js
    - gulpfile.apps.js
    - Specs/karma.conf.cjs
    - tsconfig.json
    - Tools/jsdoc/conf.json
    - cesiumrust/Cargo.toml
    - .github/workflows/dev.yml
    - .github/workflows/deploy.yml
---

## 体系概览

本项目采用 **JavaScript/TypeScript 主工程（CesiumJS）+ Rust 重写子项目（cesiumrust）** 的双栈结构，两套构建系统并行存在、互不依赖：

- **JS/TS 侧**：以 `gulpfile.js` 为入口，esbuild 为核心打包器，Karma + Jasmine 运行浏览器测试，Playwright 跑 e2e，JSDoc 生成文档；npm workspaces 管理 `@cesium/engine`、`@cesium/widgets`、`sandcastle` 三个包。
- **Rust 侧**：`cesiumrust/Cargo.toml` 定义 workspace，按 DDD 分层组织 domain / ports / adapters / application 多个 crate，默认编译 `application/cesium-app`。

## 关键文件与职责

| 文件 | 作用 |
|---|---|
| `package.json` | 顶层版本、workspaces、脚本入口（`build`/`test`/`release`/`make-zip` 等） |
| `gulpfile.js` | Gulp 任务编排：构建、watch、测试、覆盖率、文档、发布 |
| `scripts/build.js` | esbuild 封装：bundleCesiumJs、bundleWorkers、glsl→js、pragma 剥离 |
| `gulpfile.apps.js` | 应用构建：CesiumViewer、Sandcastle |
| `Specs/karma.conf.cjs` | Karma 配置（多浏览器、coverage、spec 列表） |
| `tsconfig.json` + `packages/*/tsconfig.json` | TypeScript 类型检查与 d.ts 生成 |
| `Tools/jsdoc/conf.json` | JSDoc 模板与标签扩展 |
| `cesiumrust/Cargo.toml` | Rust workspace 声明、profile、workspace.dependencies |
| `.github/workflows/dev.yml` | PR/main 分支 CI：lint → build → tsc → sg scan → coverage → release-tests |
| `.github/workflows/deploy.yml` | 非 production 分支部署：set-version → make-zip → npm pack → S3 同步 |

## 构建流程与约定

### JS/TS 构建链

1. **准备阶段** `prepare`：从 node_modules 拷贝 draco/wasm-splats/zip.js worker/prism/jasmine 到 Source/ThirdParty 与 Specs/jasmine。
2. **增量构建** `build`：
   - 先 `buildEngine` → `buildWidgets` → `buildCesium`，分别产出 ESM/IIFE/CommonJS 三种产物。
   - 通过 `scripts/build.js` 中的 esbuild context 支持 watch 模式下的增量 rebuild。
   - Shader（`.glsl`）经 `glsl-strip-comments` 预处理后内联为 JS 常量。
   - 可选 `--minify`、`--removePragmas`、`--sourcemap`、`--node` 参数控制输出形态。
3. **Worker 打包** `bundleWorkers`：将 `Source/ThirdParty/Workers/**` 单独打包并注入到主 bundle 的 inline worker 映射中。
4. **类型生成** `buildTs`：基于 JSDoc + tsd-jsdoc 从源码注释生成 `*.d.ts`，再调用 `tsc` 校验。
5. **测试** `test`：先按需构建 unminified Cesium，再用 Karma 启动 Chrome/ChromeHeadless/Firefox/Safari/Edge，支持 `--include`/`--exclude` 分类过滤、`--webglValidation`/`--webglStub`/`--release` 开关。
6. **覆盖率** `coverage`：esbuild 插件 on-load 注入 istanbul instrumenter，输出 per-browser HTML。
7. **文档** `buildDocs`：JSDoc 渲染至 `Build/Documentation`，附带图片资源。
8. **发布** `release` = `buildRelease` + 并行 `buildTs` + `buildDocs`；`websiteRelease` 额外产出 Sandcastle 所需未压缩产物。
9. **打包分发** `make-zip`：生成 `Cesium-<version>.zip`；`npm pack --workspaces` 产出各包 tarball。

### Rust 构建链

- `cargo build` 默认编译 `default-members = ["application/cesium-app"]`。
- `dev` profile 开启 `split-debuginfo=unpacked`、`incremental=true`；`release` profile 使用 `lto=thin`、`codegen-units=1`。
- 所有内部 crate 通过 `[workspace.dependencies]` 集中声明路径依赖，避免版本漂移。

### CI 流水线（GitHub Actions）

- **dev.yml**：
  - `lint`：eslint + markdownlint + prettier-check + build + tsc + ast-grep test/scan。
  - `coverage`：FirefoxHeadless + webgl-stub，结果同步至 S3。
  - `release-tests`：`make-zip` 后用 ChromeHeadless + webgl-stub + release 模式跑全量 spec。
  - `node-smoke-test`：矩阵 Node 22/24，验证 `build-release` + `npm pack` + 自定义 action `verify-package`。
- **deploy.yml**：对非 protected 分支触发，设置 `BUILD_VERSION`，执行 `make-zip`、`npm pack`、`build-apps`，最后 `aws s3 sync` 到 `cesium-public-builds/cesium/$BRANCH/`，并通过 GitHub Status API 回写状态。

## 开发者应遵循的规则

1. **新增 JS 构建目标**：在 `scripts/build.js` 中复用 `defaultESBuildOptions()`，并在 `gulpfile.js` 暴露对应 gulp task，同时更新 `filesToClean` 清理规则。
2. **新增 shader**：放入 `packages/engine/Source/Shaders/**/*.glsl`，watch 会自动触发 glsl→js 转换与增量重建。
3. **新增 Rust crate**：在 `cesiumrust/Cargo.toml` 的 `members` 与 `[workspace.dependencies]` 中注册，保持 `domain/ports/adapters/application` 四层目录约定。
4. **新增 CI job**：优先复用 `dev.yml` 的 `setup-node@v6` + `npm install` 模板，Node 版本统一锁定 `22`（smoke-test 矩阵覆盖 22/24）。
5. **发布流程**：仅维护者通过 `npm run release` 或触发 `production` 分支合并；CI 自动用 `deploy-set-version` 注入 `-<sha>` 预发版本号。
