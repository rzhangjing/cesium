---
kind: build_system
name: 双栈构建系统：Gulp+esbuild 驱动 CesiumJS + Cargo Workspace 驱动 Rust Bevy/GPUI 重写
category: build_system
scope:
    - '**'
source_files:
    - package.json
    - gulpfile.js
    - scripts/build.js
    - gulpfile.apps.js
    - gulpfile.makezip.js
    - Specs/karma.conf.cjs
    - Specs/e2e/playwright.config.js
    - cesiumrust/Cargo.toml
    - cesiumrust/application/cesium-app/Cargo.toml
    - cesiumrust/specs/Cargo.toml
    - Source/copyrightHeader.js
    - .husky/pre-commit
    - .github/dependabot.yml
---

## 1. 整体架构

本仓库是一个**双语言、双构建系统**的混合工程：
- **CesiumJS（JavaScript/TypeScript）**：沿用原版 CesiumJS 的 Gulp + esbuild 构建管线，产出浏览器端 IIFE/ESM/CJS bundle、Worker、文档与测试套件。
- **cesiumrust（Rust）**：基于 Cargo workspace 的领域驱动分层工程，使用 Bevy 0.15 作为渲染引擎，按 `domain` / `ports` / `adapters` / `application` 四层组织，specs 子 crate 复用原版 CesiumJS 的 Specs 数据作为验收基准。

两个子系统通过目录隔离共存于同一仓库，由根 `package.json` 的 npm scripts 统一编排 JS 侧流程，Rust 侧通过 `cargo build/test` 独立管理。

## 2. 关键文件与工具链

### JavaScript / TypeScript 侧
- `package.json`：定义版本 `1.143.0`、npm workspaces (`packages/engine`, `packages/widgets`, `packages/sandcastle`)、所有构建脚本入口（`build`、`build-release`、`test`、`coverage`、`release`、`make-zip` 等）。
- `gulpfile.js`：核心构建编排。调用 `scripts/build.js` 中的 `buildEngine` / `buildWidgets` / `buildCesium` / `bundleWorkers` / `glslToJavaScript` / `createCombinedSpecList` 等任务；集成 Karma 运行 Jasmine specs、Istanbul 覆盖率、Playwright e2e。
- `scripts/build.js`：esbuild 封装，支持 ESM/IIFE/CJS/Node bundle、pragma 剥离（`//>>includeStart(...)` 语法）、增量构建（`esbuild.context`）、Worker 打包。
- `gulpfile.apps.js`：应用产物构建——`buildCesiumViewer`（打包 Apps/CesiumViewer）和 `buildSandcastle`（示例沙盒），输出到 `Build/CesiumViewer` / `Build/Sandcastle`。
- `gulpfile.makezip.js`：生成 `Cesium-*.zip` 发布包。
- `tsconfig.json`、`Tools/jsconf.json`：TypeScript 类型声明生成（JSDoc → `.d.ts`）。
- `Specs/karma.conf.cjs`：Karma 多浏览器配置（Chrome/Firefox/Edge/Safari/IE），支持 WebGL validation/stub/release 模式。
- `Specs/e2e/playwright.config.js`：Playwright e2e 测试。

### Rust 侧
- `cesiumrust/Cargo.toml`：workspace 根，声明成员 `domain/*`、`ports/*`、`adapters/*`、`application/cesium-app`、`specs`；`default-members = ["application/cesium-app"]`；统一依赖版本集中管理；`profile.release` 启用 `lto = "thin"`、`codegen-units = 1`。
- `cesiumrust/domain/*`：纯 Rust 领域层（geospatial、terrain、tileset、scene、material、gltf、imagery、atmosphere、shadow、voxel 等），无框架依赖。
- `cesiumrust/ports/*`：trait 契约层（`driven`/`driving`）。
- `cesiumrust/adapters/*`：Bevy/GPU/IO 实现（`bevy-render`、`decoders`、`network`）。
- `cesiumrust/application/cesium-app`：Bevy App 组装入口（`main.rs`）。
- `cesiumrust/specs`：Rust 版 specs，对应原版 CesiumJS 的 Specs 数据目录。

## 3. 构建流程与约定

### JS 构建流水线
1. `npm run prepare`：复制 Draco/Wasm/Prism/Jasmine 第三方资源到 `Source/ThirdParty` 与 `Specs/jasmine`。
2. `npm run build`：依次 `buildEngine` → `buildWidgets` → `buildCesium`，输出 `Build/CesiumUnminified` 与 `Build/Cesium`（minify + pragma 剥离）。
3. `npm run build-watch`：esbuild context 增量构建，监听 Source/Shader/Spec 变更。
4. `npm run build-release`：生成 Node 兼容 bundle（`node: true`，无 sourcemap）。
5. `npm run release`：`buildRelease` + 并行 `buildTs`（JSDoc → `.d.ts`）+ `buildDocs`（JSDoc 文档）。
6. `npm run test`：先 build Cesium，再用 Karma 跑 Specs（支持 `--all`、`--include`、`--exclude`、`--webglValidation`、`--webglStub`、`--release`、`--browsers`）。
7. `npm run coverage`：Istanbul 注入覆盖率，输出 `Build/Coverage`。
8. `npm run make-zip`：打包发布 zip。
9. `npm run test-e2e*`：Playwright e2e，支持 `release=true` 用 minified bundle 跑。

### Rust 构建流水线
- `cargo build`：编译默认成员 `application/cesium-app`。
- `cargo test -p specs`：运行 Rust specs。
- `cargo build --release`：启用 LTO thin、单 codegen unit 优化。
- 工作区依赖版本集中在根 `Cargo.toml` 的 `[workspace.dependencies]` 中统一管理。

### 版本与发布
- JS 版本来自 `package.json.version`，构建时读取并注入版权头模板 `Source/copyrightHeader.js`。
- `deploySetVersion` 支持追加 `-<buildVersion>` 后缀用于 CI 制品标记。
- `websiteRelease` 流水线：engine → widgets → Cesium (unminified) → Cesium (minified) → Sandcastle → Docs。

## 4. 开发者规则

1. **新增 JS 模块**：放入 `packages/engine/Source` 或 `packages/widgets/Source`，确保被 `Source/Cesium.js` / 对应入口导出；shader 放 `Shaders/**/*.glsl` 会被自动转 JS 并参与 watch。
2. **新增 Rust crate**：在 `cesiumrust/domain|ports|adapters|application|specs` 下新建目录并添加 `Cargo.toml`，在根 workspace 的 `members` 中注册，在 `[workspace.dependencies]` 声明路径依赖。
3. **保持 IEEE-754 精度**：Rust 侧 glam 显式禁用 `fast-math` 特性，以匹配原版 CesiumJS 的浮点行为——这是 Specs 验收的关键约束。
4. **测试策略**：JS 侧用 Jasmine + Karma（`Specs/**Spec.js`），Rust 侧用 `cargo test`；两者共享 `Specs/Data` 下的测试数据。
5. **环境变量**：`PROD=true` 切换应用构建产物目录；`SANDCASTLE_ORIGIN`、`DEPLOYED_URL`、`CESIUM_VERSION`、`CESIUM_PACKAGES` 控制构建行为；CI 中通过 `GITHUB_TOKEN`、`GITHUB_REPO`、`GITHUB_SHA` 上报部署状态。
6. **不要直接修改 `Build/`**：该目录由 gulp/esbuild 生成，应修改源码后重新构建。
7. **依赖升级**：JS 依赖走 npm workspaces + `overrides` 解决冲突；Rust 依赖集中在根 `Cargo.toml`，避免各 crate 各自锁定版本。
