---
kind: build_system
name: CesiumJS 多包工作区构建与发布编排系统
category: build_system
scope:
    - '**'
source_files:
    - package.json
    - gulpfile.js
    - scripts/build.js
    - .github/workflows/dev.yml
    - .github/workflows/prod.yml
    - gulpfile.apps.js
    - gulpfile.makezip.js
    - server.js
---

## 体系概览

CesiumJS 采用 **npm workspaces + Gulp + esbuild** 的多包工作区架构，根 `package.json` 聚合三个子包：`@cesium/engine`、`@cesium/widgets`、`packages/sandcastle`。顶层通过 Gulp 任务统一编排源码编译、GLSL→JS 转换、Worker 打包、类型声明生成、测试执行、文档生成与制品发布。

- **Node 版本要求**：`>=22.0.0`（CI 固定使用 Node 22，并额外在矩阵中验证 24）
- **包管理器**：npm（workspaces），依赖锁定由 npm 管理；第三方依赖版本通过 `overrides` 字段强制收敛

## 核心构建管线

### 1. 源码入口与模块聚合
- 根入口 `Source/Cesium.js` 由构建脚本动态生成，遍历各 workspace 的 `packages/*/Source/**/*.js`，按文件名导出为命名空间成员，形成统一的 Cesium API 树。
- 每个 workspace 也自动生成 `index.js`，将自身 Source 目录下的文件以同名导出，供外部作为独立包消费。

### 2. 编译与打包（esbuild）
- 所有 JS/TS 编译均通过 esbuild 完成，默认 target `es2020`，支持增量构建（`esbuild.context`）。
- 输出格式：
  - ESM：`Build/CesiumUnminified/index.js` / `packages/*/Build/*/index.js`
  - IIFE：`Build/CesiumUnminified/Cesium.js`（全局名 `Cesium`）
  - CommonJS：`Build/CesiumUnminified/index.cjs`（Node 环境，`platform: 'node'`，并 polyfill `TransformStream`）
- Worker 代码单独打包，支持两种模式：
  - 独立 ESM 文件输出到 `Build/Cesium*/Workers/`
  - IIFE 内联注入到主 bundle（通过 `globalThis.CESIUM_WORKERS` base64 注入）

### 3. GLSL 着色器预处理
- `glslToJavaScript` 扫描 `packages/*/Source/Shaders/**/*.glsl`，将其转译为 ES 模块字符串常量，同时生成 `Builtin/CzmBuiltins.js` 注册内置函数/结构体/常量。
- 支持 minify 模式（调用 `glsl-strip-comments` 压缩 GLSL），并通过状态文件缓存避免重复构建。

### 4. CSS 与静态资源
- CSS 通过 esbuild loader 直接打包进产物目录，保持相对路径不变。
- 静态资源（Assets、ThirdParty、Widget CSS 等）由 gulp 流复制到 `Build/` 和 `Source/` 两个位置，分别服务于浏览器运行与开发时引用。

### 5. 条件编译（Pragmas）
- 自定义 esbuild 插件 `strip-pragmas` 解析源码中的 `//>>includeStart/pragmas.xxx` 注释块，根据构建选项选择性剔除调试代码。

### 6. TypeScript 类型声明
- 基于 JSDoc + tsd-jsdoc 从源码注释生成 `.d.ts`，再经 `fixTypescriptDefinitionsSource` 进行后处理（如 `declare` → `export`、`const enum` 降级、WebGLConstants 字符串字面量修复、`defined`/`Check` 类型谓词替换）。
- 最终包裹在 `declare module "@cesium/engine"` / `"cesium"` 命名空间中，并用 tsc 校验。

### 7. 测试与覆盖率
- 单元测试：Jasmine + Karma，通过 `gulp test` 触发，支持 `--browsers`、`--webglStub`、`--release`、`--grep` 等参数。
- 覆盖率：istanbul-lib-instrument 对源码进行插桩，Karma 收集结果并输出 HTML 报告至 `Build/Coverage`。
- E2E：Playwright 位于 `Specs/e2e/`，提供 `test-e2e*` 脚本。

### 8. 文档与示例站点
- JSDoc 文档：`gulp buildDocs` 生成 `Build/Documentation`，图片资源同步复制。
- Sandcastle 示例站：`gulpfile.apps.js` 负责构建，支持 `--outer-origin` 控制部署目标。

## CI/CD 流水线（GitHub Actions）

| 工作流 | 触发分支 | 主要职责 |
|--------|----------|----------|
| `dev.yml` | `main`、PR、merge_group | lint → build → tsc → sg scan → coverage (FirefoxHeadless) → release tests (ChromeHeadless, --release) → cloc → node 22/24 包验证 |
| `prod.yml` | `cesium.com` | 构建 website-release → 部署 zip 归档到 S3 `cesium-website/cesiumjs/releases/<version>/` → 同步文档到 `ref-doc/` → 构建并部署 CesiumViewer 与 Sandcastle → 清理旧 Ion token |

- 可复用 action 位于 `.github/actions/`，用于 `verify-package`、`update-tokens` 等步骤。
- 覆盖率上传至 AWS S3，部署产物通过 `aws s3 sync` 推送，带 `Cache-Control: public, max-age=1800`。

## 关键约定与约束

1. **工作区命名与作用域**：根 `scope = "cesium"`，对应 `@cesium/engine`、`@cesium/widgets`；新增包需同步更新 `getWorkspaces()` 与 `workspaceSourceFiles`。
2. **构建产物目录**：
   - 根产物：`Build/CesiumUnminified/`、`Build/Cesium/`、`Build/Specs/`、`Build/Sandcastle*/`
   - 子包产物：`packages/*/Build/{Unminified,Minified}/`、`packages/*/Build/Specs/`
3. **增量构建**：Gulp watch 模式下返回 `esbuild.context`，监听 Shader 与 Spec 变更触发 `rebuild()`，进程退出时显式 `dispose()`。
4. **版本传播**：`postversion` 钩子自动扫描所有 `package.json`，将与被更新包存在依赖关系的条目升级到 `^<newVersion>`。
5. **第三方依赖治理**：`ThirdParty.extra.json` 配合 `buildThirdParty` 任务递归收集依赖许可证信息，生成 `ThirdParty.json`。
6. **ESM/CJS 双出口**：根 `exports` 字段区分 `import` 与 `require`，`sideEffects` 声明副作用文件以便 tree-shaking。

## 开发者常用命令

```bash
npm run build          # 全量构建（engine → widgets → cesium）
npm run build-watch    # 增量构建 + 热重载
npm run build-ts       # 生成 .d.ts 类型声明
npm run test           # Jasmine/Karma 单测
npm run coverage       # 覆盖率报告
npm run build-docs     # 生成 JSDoc 文档
npm run make-zip       # 打包发布 zip
npm run release        # 构建 + 类型 + 文档（发布前）
```
