---
kind: dependency_management
name: CesiumJS + CesiumRust 双引擎依赖管理体系
category: dependency_management
scope:
    - '**'
source_files:
    - package.json
    - cesiumrust/Cargo.toml
    - cesiumrust/Cargo.lock
    - ThirdParty.json
    - .github/dependabot.yml
    - greenkeeper.json
---

该项目采用 **双语言、多工作区** 的依赖管理策略，分别针对 JavaScript (CesiumJS) 和 Rust (CesiumRust) 两套引擎独立管理第三方依赖。

## 1. JavaScript 侧（CesiumJS）
- **包管理器**: npm + Node.js >=22.0.0
- **工作区结构**: 根 `package.json` 通过 `workspaces` 声明 `packages/engine`、`packages/widgets`、`packages/sandcastle` 三个子包，实现单仓库多包管理
- **依赖声明位置**:
  - 运行时依赖: `dependencies` 中仅保留 `@cesium/engine`、`@cesium/widgets`、`protobufjs` 三个核心包
  - 构建/测试依赖: `devDependencies` 集中管理 Gulp、Karma、Playwright、ESLint、Prettier、JSDoc 等工具链
- **版本锁定**: 使用 `overrides` 字段强制统一 `protobufjs`、`react`、`react-dom` 等关键依赖的版本，避免传递依赖冲突
- **许可证追踪**: `ThirdParty.json` 与 `ThirdParty.extra.json` 完整记录所有第三方库的名称、版本、许可证类型与来源 URL，用于合规审计
- **自动更新**: `.github/dependabot.yml` 配置每日扫描 GitHub Actions 依赖更新；`greenkeeper.json` 提供历史 Greenkeeper 集成配置

## 2. Rust 侧（CesiumRust）
- **包管理器**: Cargo + Cargo.lock 精确锁定所有 crate 版本与 checksum
- **工作区结构**: `cesiumrust/Cargo.toml` 定义 workspace，按 DDD 分层组织：
  - `domain/`: 领域层纯 Rust crate（geospatial、time、camera、terrain 等 30+ 个模块）
  - `ports/`: 端口层 trait 契约（driven、driving）
  - `adapters/`: 适配器层 Bevy/IO 实现
  - `application/`: 应用组装层
  - `specs/`: 从 CesiumJS Specs 移植的集成测试 crate
- **依赖集中管理**: `[workspace.dependencies]` 统一声明 glam、bevy、serde、tokio 等公共依赖版本，各子 crate 通过名称引用而非重复声明
- **内部 crate 引用**: 使用 `{ path = "..." }` 路径引用同仓库内其他 crate，如 `cesium-geospatial`、`cesium-scene` 等
- **发布策略**: `[workspace.package]` 设置 `publish = false`，表明该 workspace 不直接发布到 crates.io

## 3. 跨语言依赖协调
- CesiumJS 通过 `@cesium/engine` 和 `@cesium/widgets` 引用内部打包产物，与 Rust 侧无直接耦合
- 两套引擎通过 WebAssembly / JS-Rust 桥接交互，但依赖管理完全解耦
- 构建脚本 (`scripts/build.js`, `gulpfile.js`) 负责协调两端构建顺序

## 开发者规范
1. **新增 JS 依赖**: 优先放入 `devDependencies`，运行时依赖才放入 `dependencies`，并通过 `overrides` 统一管理冲突版本
2. **新增 Rust crate**: 在对应层级目录创建 crate，并在 `Cargo.toml` 的 `members` 中注册，公共依赖添加到 `[workspace.dependencies]`
3. **许可证合规**: 任何新增依赖需同步更新 `ThirdParty.json`，注明许可证类型
4. **版本升级**: 依赖更新由 Dependabot 自动发起 PR，需人工审核后合并