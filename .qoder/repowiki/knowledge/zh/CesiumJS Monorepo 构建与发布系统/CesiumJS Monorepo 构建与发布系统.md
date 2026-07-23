---
kind: build_system
name: CesiumJS Monorepo 构建与发布系统
category: build_system
scope:
    - '**'
source_files:
    - package.json
    - gulpfile.js
    - scripts/build.js
    - .github/workflows/update-tokens.yml
    - .github/dependabot.yml
    - Source/copyrightHeader.js
    - Specs/karma.conf.cjs
---

CesiumJS 采用基于 Node.js + Gulp + esbuild 的现代化前端构建体系，通过 npm workspaces 管理 engine、widgets、sandcastle 三个子包，并以统一的 Gulp 任务编排整个构建、测试、文档生成与发布流程。

**核心构建工具链**
- **Gulp 5**：作为顶层编排器，定义 build、buildRelease、test、coverage、buildDocs、release 等任务
- **esbuild**：高性能打包器，负责 JS/TS 编译、GLSL→JS 转换、Worker 打包、CSS 处理
- **Karma + Jasmine**：浏览器端单元测试框架，支持多浏览器（Chrome/Firefox/Safari/Edge）和 WebGL 验证模式
- **Playwright**：E2E 截图回归测试，位于 Specs/e2e/
- **JSDoc + tsd-jsdoc**：从 JSDoc 注释生成 TypeScript 类型声明
- **TypeScript 6**：类型检查与 .d.ts 生成

**Monorepo 结构**
- `packages/engine/`：核心引擎源码与测试
- `packages/widgets/`：UI 组件库
- `packages/sandcastle/`：示例站点构建器
- 根目录 `Source/Cesium.js` 动态聚合所有模块导出
- 构建产物统一输出到 `Build/` 目录

**关键构建流程**
1. `gulp prepare`：复制第三方依赖（draco3d、zip.js、prismjs、jasmine-core）到 Source/ThirdParty
2. `glslToJavaScript()`：将 GLSL 着色器编译为 JS 字符串模块，支持 minify 状态缓存
3. `createIndexJs()`：为每个 workspace 生成 index.js 导出文件
4. `bundleCesiumJs()`：并行构建 ESM/IIFE/CJS 三种格式，支持增量构建
5. `bundleWorkers()`：打包 Web Workers，支持 IIFE 内联或独立文件
6. `createCombinedSpecList()`：自动生成 SpecList.js 聚合所有测试文件

**开发工作流**
- `npm run build-watch`：监听源文件变化，增量重建 ESM/IIFE/CJS 和 Worker
- `npm run test`：运行 Karma 测试，支持 --include/--exclude 分类过滤
- `npm run coverage`：生成 Istanbul 覆盖率报告
- `npm run build-docs`：生成 JSDoc 文档

**发布流程**
- `gulp buildRelease`：构建未压缩和压缩版本，生成 Node.js CJS 包
- `gulp release`：并行生成 TypeScript 类型声明和文档
- `postversion` 钩子：自动更新依赖该包的版本号
- `make-zip`：打包发布产物

**CI/CD 集成**
- GitHub Actions 定时任务（update-tokens.yml）自动更新访问令牌
- Dependabot 每日扫描依赖更新
- 自定义 Gulp 任务通过 GitHub API 设置 PR 状态和部署链接

**约定与约束**
- 所有源码必须遵循版权头模板（Source/copyrightHeader.js）
- GLSL 着色器需放在 Source/Shaders/Builtin/ 下才能被自动识别
- 调试代码使用 `//>>pragmaStart(debug)` / `//>>pragmaEnd(debug)` 包裹，生产构建时移除
- 测试文件命名规范：*Spec.js，位于对应 Source 同级 Specs 目录
- 要求 Node.js >= 22.0.0