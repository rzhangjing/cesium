---
kind: dependency_management
name: CesiumJS 多语言仓库依赖管理（npm workspaces + Rust Cargo workspace）
category: dependency_management
scope:
    - '**'
source_files:
    - package.json
    - .npmrc
    - packages/engine/package.json
    - packages/widgets/package.json
    - packages/sandcastle/package.json
    - ThirdParty.json
    - cesiumrust/Cargo.toml
    - cesiumrust/Cargo.lock
    - .github/dependabot.yml
---

## 1. 使用的系统与工具

本仓库是一个**多语言 monorepo**，同时包含 JavaScript/TypeScript 核心库与 Rust 重实现（cesiumrust），因此采用两套并行的依赖管理系统：

- **JavaScript/TypeScript 侧**：使用 npm + npm workspaces。根 `package.json` 通过 `workspaces` 字段声明三个子包：`packages/engine`、`packages/widgets`、`packages/sandcastle`。构建和测试脚本通过 `--workspace <name>` 参数在对应子包内执行。
- **Rust 侧**：使用 Cargo workspace（`cesiumrust/Cargo.toml` 中 `[workspace]` 声明了 domain / ports / adapters / application / specs 等成员 crate），并通过 `[workspace.dependencies]` 集中声明所有第三方 crate 的版本。

## 2. 关键文件

| 文件 | 作用 |
|---|---|
| `package.json` | 根工作区入口，声明运行时依赖 `@cesium/engine`、`@cesium/widgets`、`protobufjs`，以及大量 devDependencies（gulp、karma、playwright、eslint、prettier、typescript 等） |
| `packages/engine/package.json` | `@cesium/engine` 包定义，声明核心运行时依赖（draco3d、meshoptimizer、dompurify、pako、lerc、protobufjs、topojson-client 等） |
| `packages/widgets/package.json` | `@cesium/widgets` 包定义，仅依赖 `@cesium/engine` 与 `nosleep.js` |
| `packages/sandcastle/package.json` | 示例/文档应用（私有包），依赖 React、Monaco Editor、Vite、`@huggingface/transformers` 等 |
| `.npmrc` | 设置 `package-lock=false`，即不生成 `package-lock.json` |
| `ThirdParty.json` | 人工维护的第三方许可证清单，记录每个发布产物所用第三方包的名称、版本、许可证及来源 URL |
| `cesiumrust/Cargo.toml` | Rust workspace 根，集中声明 `glam`、`bevy`、`serde`、`tokio`、`ureq`、`image` 等 crate 版本 |
| `cesiumrust/Cargo.lock` | Rust 依赖锁定文件（由 cargo 自动生成） |
| `.github/dependabot.yml` | GitHub Dependabot 配置，仅对 GitHub Actions 每日检查更新 |
| `greenkeeper.json` | 遗留的 Greenkeeper 配置文件（已弃用，当前实际使用 Dependabot） |

## 3. 架构与约定

### 3.1 JavaScript 依赖分层
- **运行时依赖**集中在 `packages/engine/package.json` 的 `dependencies` 中，这些是最终用户引入 Cesium 时需要的最小集。
- **开发/构建依赖**集中在根 `package.json` 的 `devDependencies` 中，包括 Gulp 构建管线、Karma/Jasmine 单元测试、Playwright E2E、ESLint/Prettier 代码质量工具链。
- **内部包间依赖**通过 npm workspaces 解析：根包依赖 `@cesium/engine` 和 `@cesium/widgets`，widgets 反向依赖 engine，sandcastle 为私有示例应用。
- **依赖冲突解决**：根 `overrides` 字段强制将 `protobufjs` 统一为 `^8.6.5`，并对 `allotment` 下的 `react`/`react-dom` 提升到 `^19.0.0`，以适配 sandcastle 的 React 19。

### 3.2 Rust 依赖集中化
- 所有外部 crate 版本在 `cesiumrust/Cargo.toml` 的 `[workspace.dependencies]` 中**单一声明**，各成员 crate 通过 `cesium-*` 路径引用内部 crate，通过 crate 名引用外部 crate，避免版本漂移。
- Bevy 默认关闭 audio/gilrs 特性，以避免 headless CI 缺少 alsa-sys/libudev-sys 系统库导致编译失败。
- glam 显式禁用 `fast-math` 特性，以保证与 CesiumJS 原始 IEEE-754 双舍入算术的位级一致性。

### 3.3 许可证合规
- `ThirdParty.json` 作为发布产物附带清单，由构建流程生成或手动维护，记录每个第三方组件的名称、许可证、版本与源码地址，用于分发时的法律合规。

## 4. 约定与约束

- **不使用 lockfile**：`.npmrc` 中 `package-lock=false` 明确禁止生成 `package-lock.json`，依赖版本完全由 `package.json` 中的语义化版本范围决定。
- **Node 版本锁定**：根与 `engine`、`widgets` 包均声明 `engines.node >= 22.0.0`，确保构建环境一致。
- **依赖更新策略**：通过 GitHub Dependabot 每日扫描 GitHub Actions；JavaScript 生态未启用 Dependabot（仅 GitHub Actions），意味着 JS 依赖升级主要依赖人工 PR 或历史遗留的 Greenkeeper 配置。
- **私有 registry**：未发现 `.npmrc` 中配置私有 npm registry 或 `//registry.npmjs.org/` 等镜像，所有依赖从公共 npm 获取。
- **vendoring**：JavaScript 侧不 vendoring 第三方源码，全部通过 npm 安装；Rust 侧通过 `Cargo.lock` 锁定具体版本，也不 vendoring。
- **版本同步**：`@cesium/engine` 与 `@cesium/widgets` 版本号保持对齐（均为 26.x / 16.x），通过 monorepo 的 workspace 机制保证内部依赖版本一致。
- **安全/漏洞扫描**：未发现 npm audit 或类似步骤集成到构建脚本中；代码风格与安全规则通过 ESLint（含 `eslint-seatbelt`）、ast-grep rules 在提交前检查。

## 5. 总结

CesiumJS 仓库采用**双栈依赖管理**：JavaScript 部分基于 npm workspaces 的多包 monorepo，通过 `overrides` 解决依赖冲突，通过 `ThirdParty.json` 维护许可证清单；Rust 部分基于 Cargo workspace，通过 `[workspace.dependencies]` 集中管理版本。两者均未使用 lockfile 锁定 JS 依赖（npm lock 被禁用），而 Rust 使用 `Cargo.lock`。依赖更新主要由 Dependabot 驱动 GitHub Actions，JS 生态依赖人工维护。