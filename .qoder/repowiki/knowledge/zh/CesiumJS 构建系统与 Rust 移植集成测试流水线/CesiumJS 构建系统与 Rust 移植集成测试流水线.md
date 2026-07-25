---
kind: build_system
name: CesiumJS 构建系统与 Rust 移植集成测试流水线
category: build_system
scope:
    - '**'
source_files:
    - package.json
    - gulpfile.js
    - scripts/build.js
    - cesiumrust/Cargo.toml
    - Specs/karma.conf.cjs
    - Specs/e2e/playwright.config.js
    - tsconfig.json
    - .husky/pre-commit
---

## 构建系统概述

该项目采用双栈构建体系：基于 Node.js/Gulp/esbuild 的 CesiumJS 前端构建系统，以及基于 Cargo workspace 的 Rust 移植集成测试系统。两者通过 Specs 测试套件实现行为一致性验证。

### JavaScript/TypeScript 构建系统

**核心工具链：**
- **Gulp 5** - 任务编排和文件监控
- **esbuild** - 高性能模块打包和代码转换
- **Karma + Jasmine** - 浏览器端单元测试框架
- **Playwright** - E2E 端到端测试
- **TypeScript** - 类型检查和定义生成

**主要构建流程：**
1. `gulp build` - 完整构建（engine → widgets → cesium）
2. `gulp buildRelease` - 发布构建（包含 minify 和 pragma 清理）
3. `gulp test` - 运行 Karma 测试套件
4. `npm run test-e2e` - 执行 Playwright E2E 测试

**包结构管理：**
- npm workspaces 管理三个子包：`@cesium/engine`、`@cesium/widgets`、`packages/sandcastle`
- 动态生成 `Source/Cesium.js` 入口文件，自动扫描所有模块导出
- GLSL shader 编译为 JavaScript 字符串模块

### Rust 移植构建系统

**Cargo Workspace 架构：**
```toml
[workspace]
members = [
    "domain/*",      # 领域层（纯 Rust）
    "ports/*",       # 端口层（trait 接口）
    "adapters/*",    # 适配器层（Bevy/IO 实现）
    "application/*", # 应用层（Bevy App）
    "specs"          # 集成测试套件
]
```

**分层设计：**
- **Domain 层**：地理空间、时间、相机、场景等核心领域逻辑
- **Ports 层**：驱动和被驱动的 trait 接口定义
- **Adapters 层**：Bevy 渲染、解码器、网络等具体实现
- **Application 层**：Bevy 应用组装和示例程序

### 测试系统集成

**JavaScript 测试：**
- Jasmine spec 文件位于 `Specs/` 目录
- 支持 WebGL 和非 WebGL 环境测试
- 覆盖率报告通过 Istanbul 生成
- 多浏览器支持（Chrome、Firefox、Safari、Edge）

**Rust 集成测试：**
- `cesiumrust/specs/` 目录包含与 JS Specs 对应的 Rust 测试
- 使用标准库 `#[test]` 和 `assert!` 宏
- 按功能模块组织：core、datasources、renderer、scene、widgets

### 开发工作流

**本地开发命令：**
```bash
npm run build-watch     # 增量构建并监听文件变化
npm run test            # 运行单元测试
npm run test-e2e        # 运行端到端测试
npm run coverage        # 生成覆盖率报告
```

**Rust 开发命令：**
```bash
cargo test              # 运行所有测试
cargo test --package specs  # 仅运行集成测试
cargo build             # 构建应用
```

### 持续集成与发布

**版本管理：**
- 单一版本号管理整个项目（package.json version）
- 支持预发布版本标记（如 `1.143.0-beta.1`）
- 自动生成 TypeScript 定义文件（`.d.ts`）

**构建产物：**
- `Build/Cesium/` - 压缩版浏览器 bundle
- `Build/CesiumUnminified/` - 开发版 bundle
- `Build/Documentation/` - JSDoc 生成的文档
- `ThirdParty.json` - 第三方依赖许可证信息

**质量检查：**
- ESLint + TypeScript ESLint 代码检查
- Prettier 代码格式化
- Markdownlint 文档检查
- ast-grep 自定义规则扫描

### 关键约定

1. **模块化原则**：每个功能模块独立导出，通过聚合入口统一暴露
2. **渐进增强**：支持从最小依赖开始逐步添加功能
3. **向后兼容**：保持 API 稳定性，废弃功能通过 pragma 控制
4. **测试驱动**：新功能必须配套相应的 Spec 测试
5. **跨语言一致性**：Rust 实现需通过对应 JS 测试用例验证