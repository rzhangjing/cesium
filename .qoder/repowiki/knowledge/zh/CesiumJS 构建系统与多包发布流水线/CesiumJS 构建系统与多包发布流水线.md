---
kind: build_system
name: CesiumJS 构建系统与多包发布流水线
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
    - Tools/jsdoc/conf.json
    - tsconfig.json
---

## 构建系统概览

CesiumJS 采用 Gulp + esbuild 的混合构建体系，结合 npm workspaces 管理多包结构。核心构建逻辑集中在根目录的 Gulp 任务中，底层打包由 esbuild 提供高性能增量编译支持。

### 核心构建工具链

- Gulp 5: 作为构建任务编排器，定义所有构建、测试、文档生成任务
- esbuild: 核心打包器，负责 JS/TS/CSS/GLSL 的编译和打包，支持增量构建
- Karma + Jasmine: 浏览器端单元测试框架
- Playwright: 端到端测试
- JSDoc + tsd-jsdoc: TypeScript 类型定义生成
- TypeScript: 类型检查和类型声明验证

### 多包架构与工作区

项目使用 npm workspaces 管理三个核心包：
- @cesium/engine: 核心引擎库（packages/engine）
- @cesium/widgets: UI 组件库（packages/widgets）
- cesium: 主应用入口（根 package.json）

每个包独立维护 Source、Specs、Build 目录结构，通过动态生成的 index.js 聚合导出。

### 主要构建产物

Build/
├── Cesium/: 生产环境压缩版本
├── CesiumUnminified/: 开发环境未压缩版本
├── Sandcastle/: 示例应用
├── CesiumViewer/: 轻量查看器
├── Documentation/: API 文档
├── Coverage/: 测试覆盖率报告
└── Specs/: 测试运行器

### 关键构建流程

1. GLSL 预处理: 将 .glsl 着色器文件转换为 JavaScript 模块
2. 源码聚合: 动态生成 Source/Cesium.js 和包级 index.js 导出文件
3. 多格式打包: 同时生成 ESM、IIFE、CommonJS 三种格式
4. Worker 处理: 单独打包 Web Workers，支持内联或外部加载
5. 类型生成: 从 JSDoc 注释自动生成 TypeScript 声明文件
6. 第三方依赖: 自动收集并生成 ThirdParty.json 许可证信息

### CI/CD 流水线

GitHub Actions 定义了完整的持续集成流程：

- dev.yml: 开发分支触发，执行 lint、构建、测试、覆盖率收集
- prod.yml: cesium.com 分支触发，构建网站、文档、示例并部署到 S3
- deploy.yml: 预发布版本构建和部署

### 开发者命令

npm run build           # 完整构建
npm run build-watch     # 增量监听构建
npm run test            # 运行单元测试
npm run coverage        # 生成覆盖率报告
npm run build-ts        # 生成 TypeScript 定义
npm run release         # 发布准备（构建+类型+文档）

### Rust 子项目构建

cesiumrust/ 目录包含基于 Bevy 的 Rust 重写项目，使用 Cargo workspace 管理多个 crate，采用 DDD 六边形架构设计，与 JavaScript 部分并行开发。

### 构建约定与约束

- 源码必须遵循严格的 ESLint 规则
- 所有公共 API 必须有 JSDoc 注释
- GLSL 着色器文件需遵循特定命名和组织规范
- 版本号统一在 package.json 中管理，构建时自动注入版权头
- 支持 pragma 条件编译，用于调试和生产环境的代码裁剪