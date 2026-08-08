---
kind: build_system
name: CesiumJS 构建系统：Gulp + esbuild + npm workspace 多包构建与发布
category: build_system
scope:
    - '**'
source_files:
    - package.json
    - gulpfile.js
    - scripts/build.js
    - gulpfile.apps.js
    - tsconfig.json
    - .npmrc
    - server.js
---

## 构建系统与工具链

CesiumJS 采用 **npm workspace 多包架构**，以 Gulp 为编排器、esbuild 为核心打包器，结合 Karma/Playwright 测试框架和 JSDoc 文档生成，形成完整的 JavaScript/TypeScript 引擎构建流水线。

### 核心构建流程

**入口与编排**：`gulpfile.js` 定义所有构建任务，通过 `scripts/build.js` 中的 esbuild 配置实现模块化打包。根 `package.json` 声明 Node.js >= 22.0.0 环境要求。

**多包结构**：
- `packages/engine` - 核心引擎库（@cesium/engine）
- `packages/widgets` - UI 组件库（@cesium/widgets）
- `packages/sandcastle` - 示例应用
- 根工程聚合三个子包，通过 `workspaces` 字段统一管理

**构建产物**：
- ESM 模块：`Build/Cesium/index.js` 和 `packages/*/Build/*/index.js`
- IIFE 全局：`Build/Cesium/Cesium.js`（浏览器直接引用）
- CommonJS：`Build/Cesium/index.cjs`（Node.js 环境）
- SourceMap：`.js.map` 文件用于调试
- Worker 脚本：`Build/Cesium/Workers/**` 独立线程处理

### 关键构建步骤

1. **GLSL Shader 编译**：`glslToJavaScript()` 将 `.glsl` 着色器代码转换为 JavaScript 字符串模块，支持 minify 模式压缩
2. **源码预处理**：通过 `stripPragmaPlugin` 处理 `//>>includeStart` / `//>>excludeEnd` 条件编译标记
3. **增量构建**：使用 `esbuild.context` 支持热重载，监听文件变化自动重建
4. **Worker 打包**：`bundleWorkers()` 将 Web Workers 单独打包，支持 IIFE 内联或外部加载
5. **类型生成**：`buildTs()` 通过 tsd-jsdoc 从 JSDoc 注释生成 TypeScript 定义文件
6. **测试构建**：`bundleCombinedSpecs()` 聚合所有 Spec 文件供 Karma 运行

### 测试与质量检查

**单元测试**：Karma + Jasmine 框架，支持多浏览器（Chrome/Firefox/Safari/Edge），可通过 `--webglStub` 参数在无 WebGL 环境下运行

**端到端测试**：Playwright 框架，位于 `Specs/e2e/` 目录，支持 Chromium 浏览器自动化测试

**覆盖率统计**：istanbul-lib-instrument 注入代码覆盖率，生成 HTML 报告

**代码质量**：ESLint + Prettier + Markdownlint + ast-grep 规则检查

### 发布流程

**版本管理**：通过 `postversion` 钩子自动更新依赖版本，支持 `--buildVersion` 参数添加构建标识

**制品生成**：
- `npm run build-release`：生成完整构建产物
- `npm run make-zip`：打包为 zip 分发文件
- `npm run release`：同时生成 TypeScript 定义和文档

**CI/CD**：GitHub Actions 工作流集成，支持 PR 检查、自动部署和状态回写

### Rust 适配层构建

`cesiumrust/` 目录包含基于 Bevy 的 Rust 实现，使用 Cargo workspace 管理多个 crate，通过 `Cargo.toml` 定义依赖关系和构建配置。

### 开发者约定

- 使用 `npm run build-watch` 进行开发时增量构建
- 通过 `--workspace=@cesium/engine` 指定单包构建
- GLSL 着色器文件放在 `Source/Shaders/` 目录自动被编译
- 条件编译使用 `//>>includeStart('debug', pragmas.debug);` 语法
- 测试文件遵循 `*Spec.js` 命名规范自动被发现