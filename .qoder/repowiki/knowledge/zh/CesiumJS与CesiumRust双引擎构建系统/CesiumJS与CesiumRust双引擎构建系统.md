---
kind: build_system
name: CesiumJS与CesiumRust双引擎构建系统
category: build_system
scope:
    - '**'
source_files:
    - package.json
    - gulpfile.js
    - scripts/build.js
    - cesiumrust/Cargo.toml
    - gulpfile.apps.js
    - gulpfile.makezip.js
---

## 构建系统概述

该项目采用**双引擎架构**，包含CesiumJS原版JavaScript引擎和基于DDD/六边形架构的Rust重构（cesiumrust），通过统一的Gulp + esbuild工具链进行构建、测试与发布。

## 核心构建工具链

### JavaScript/TypeScript部分
- **构建工具**: Gulp 5.0 + esbuild 0.28.0
- **包管理**: npm workspaces (Node.js >= 22.0.0)
- **测试框架**: Karma + Jasmine (单元测试), Playwright (端到端测试)
- **文档生成**: JSDoc 3.6.7
- **代码质量**: ESLint 10.4.1, Prettier 3.9.4, Markdownlint

### Rust部分
- **构建工具**: Cargo Workspace (Cargo.toml)
- **渲染引擎**: Bevy 0.15
- **数学库**: glam 0.29 (f64精度)
- **异步运行时**: Tokio 1.x

## 项目结构组织

### NPM工作空间结构
```
packages/
├── engine/          # CesiumJS核心引擎
├── widgets/         # UI组件库
└── sandcastle/      # 示例应用
```

### Rust Workspace结构
```
cesiumrust/
├── domain/          # 领域层 (纯Rust)
├── ports/           # 端口层 (trait接口)
├── adapters/        # 适配器层 (Bevy实现)
├── application/     # 应用层 (Bevy App组装)
└── specs/           # 集成测试套件
```

## 主要构建命令

### JavaScript构建
- `npm run build` - 完整构建
- `npm run build-release` - 发布构建
- `npm run test` - 运行Karma测试
- `npm run test-e2e` - 运行Playwright E2E测试
- `npm run build-docs` - 生成JSDoc文档
- `npm run make-zip` - 创建发布压缩包

### Rust构建
- `cargo build` - 构建默认成员
- `cargo test` - 运行所有测试
- `cargo build --release` - 发布构建

## 构建流程详解

### CesiumJS构建流程
1. **预处理**: 复制第三方依赖文件到Source目录
2. **GLSL编译**: 将Shader文件转换为JavaScript模块
3. **模块打包**: 使用esbuild生成ESM/IIFE/CJS格式
4. **Worker处理**: 独立打包Web Workers
5. **类型生成**: 从JSDoc生成TypeScript定义文件
6. **文档生成**: 使用JSDoc生成API文档

### Rust构建流程
1. **Workspace解析**: Cargo自动解析所有crate依赖
2. **增量编译**: 启用incremental编译加速开发
3. **LTO优化**: 发布模式启用thin LTO
4. **调试信息**: dev模式启用unpacked调试信息

## CI/CD流水线

### GitHub Actions工作流
- **PR检查**: 代码质量检查、单元测试、类型检查
- **构建验证**: 多平台构建验证
- **发布流程**: 版本管理、包发布、文档部署

### 本地开发工作流
- **热重载**: gulp watch监听文件变化
- **增量构建**: esbuild context支持快速重建
- **并行任务**: gulp.parallel优化构建性能

## 关键配置文件

### package.json脚本配置
- 定义了完整的构建、测试、发布生命周期
- 支持workspace级别的命令分发
- 集成了husky git钩子

### Gulp任务架构
- **gulpfile.js**: 主构建逻辑
- **gulpfile.apps.js**: 应用构建
- **gulpfile.makezip.js**: 发布打包
- **scripts/build.js**: 核心构建函数

### Rust Workspace配置
- **Cargo.toml**: workspace成员声明
- **profile配置**: dev/release优化选项
- **dependency管理**: 统一版本控制

## 开发者规范

### 代码质量标准
- 强制ESLint规则检查
- Prettier代码格式化
- Markdown文档lint检查
- ast-grep自定义规则扫描

### 测试策略
- 单元测试: Jasmine + Karma
- 集成测试: Rust原生测试
- E2E测试: Playwright跨浏览器
- 覆盖率报告: Istanbul + Karma

### 构建优化
- Tree-shaking支持
- SourceMap生成
- 增量编译缓存
- 并行构建执行

该系统提供了完整的现代化前端+后端构建解决方案，支持双引擎并行开发和测试，确保了代码质量和构建效率。