---
kind: dependency_management
name: CesiumJS 与 CesiumRust 双栈依赖管理（npm + Cargo workspace）
category: dependency_management
scope:
    - '**'
source_files:
    - package.json
    - .npmrc
    - ThirdParty.json
    - ThirdParty.extra.json
    - .github/dependabot.yml
    - greenkeeper.json
    - cesiumrust/Cargo.toml
    - cesiumrust/Cargo.lock
---

## 1. 使用的系统/工具

本仓库包含两个独立的技术栈，各自维护一套依赖：

- **JavaScript/TypeScript 栈**：使用 npm 作为包管理器，通过 `package.json` 声明运行时依赖与开发依赖；通过 npm workspaces 将根工程与 `packages/engine`、`packages/widgets`、`packages/sandcastle` 三个子包统一管理；通过 `.npmrc` 禁用 `package-lock`（即不使用 `package-lock.json`），版本锁定由上游包的语义化版本范围控制。
- **Rust 重写栈（cesiumrust）**：使用 Cargo workspace，根 `cesiumrust/Cargo.toml` 集中声明 workspace members（domain / ports / adapters / application / specs），并通过 `[workspace.dependencies]` 统一外部 crate 的版本，各子 crate 仅引用内部 crate 的 path 依赖。

更新策略：
- GitHub Actions 通过 `.github/dependabot.yml` 每日扫描并自动为 GitHub Actions 生态创建 PR（当前仅配置了 `github-actions` 生态，未配置 npm/Rust 生态）。
- 遗留的 `greenkeeper.json` 表明历史上曾使用 Greenkeeper 做依赖升级，现已迁移到 Dependabot。

## 2. 关键文件

| 文件 | 作用 |
|---|---|
| `package.json` | 根 npm 工作区入口，声明运行时依赖（`@cesium/engine`、`@cesium/widgets`、`protobufjs`）、开发依赖、workspaces、`overrides`（强制 protobufjs 版本、allotment 的 react 版本）以及 Node 引擎要求（`>=22.0.0`） |
| `.npmrc` | 全局禁用 `package-lock.json` |
| `ThirdParty.json` | 构建产物中随分发的第三方库清单（名称、许可证、版本、URL），用于生成 `ThirdParty.js` 等分发元数据 |
| `ThirdParty.extra.json` | 额外第三方清单（与 `ThirdParty.json` 配合） |
| `.github/dependabot.yml` | 自动化依赖更新（当前仅针对 GitHub Actions） |
| `greenkeeper.json` | 历史 Greenkeeper 配置（已弃用） |
| `cesiumrust/Cargo.toml` | Rust workspace 根，集中声明所有 workspace-level 依赖（glam、bevy、serde、tokio、ureq、image 等）及内部 crate 路径映射 |
| `cesiumrust/Cargo.lock` | Rust 依赖精确锁文件 |

## 3. 架构与约定

### JavaScript 侧
- **运行时依赖最小化**：根 `package.json` 的 `dependencies` 仅保留 3 个运行时包（`@cesium/engine`、`@cesium/widgets`、`protobufjs`），其余全部放入 `devDependencies`，确保发布产物尽可能精简。
- **NPM Workspaces**：通过 `workspaces` 字段将 `packages/engine`、`packages/widgets`、`packages/sandcastle` 纳入同一工作区，便于本地联调与版本对齐。
- **版本覆盖（overrides）**：通过 `overrides` 强制解决传递依赖冲突——例如把 `@huggingface/transformers` 下的 `onnxruntime-web` 的 `protobufjs` 回退到 `^7.6.4`，并把 `allotment` 的 `react`/`react-dom` 升级到 `^19.0.0`。这是该仓库处理依赖冲突的核心手段。
- **Node 版本约束**：`engines.node >= 22.0.0` 在 CI 和开发者环境强制一致。
- **第三方分发清单**：`ThirdParty.json` 是构建期生成的权威清单，记录每个被打包进 Cesium 发行物的第三方库及其许可证，供发布流程校验合规性。

### Rust 侧（cesiumrust）
- **Workspace 模式**：`resolver = "2"`，所有 crate 共享一个 `Cargo.lock`，避免版本漂移。
- **集中式依赖声明**：`[workspace.dependencies]` 定义 glam、bevy、serde、tokio、ureq、image 等公共 crate 的版本，各子 crate 通过 `workspace = true` 引用，保证全仓版本一致。
- **内部 crate 以 path 依赖组织**：按领域分层（domain / ports / adapters / application / specs），内部 crate 之间不发布到 crates.io（`publish = false`），仅通过 path 引用。
- **Bevy 特性裁剪**：显式关闭默认 features，只启用渲染所需的最小 feature 集（`bevy_asset`、`bevy_render`、`bevy_pbr` 等），并禁用 `fast-math` 以保证与 CesiumJS 规格位级一致。

## 4. 约定与约束

- **禁止使用 package-lock.json**：`.npmrc` 中 `package-lock=false` 明确禁用，版本解析完全依赖 `package.json` 中的语义化版本范围。
- **运行时依赖必须声明在 `dependencies`，开发依赖必须在 `devDependencies`**：从 `package.json` 可见，只有 3 个包进入运行时依赖，其余如 gulp、karma、eslint、playwright 等均放在开发依赖中。
- **传递依赖冲突通过 `overrides` 显式解决**：仓库没有使用 pnpm/yarn 的 resolution 机制，而是通过 npm 原生 `overrides` 字段强制覆盖下游依赖版本。
- **Rust 依赖版本集中在 workspace 层**：任何新增的外部 crate 应添加到 `cesiumrust/Cargo.toml` 的 `[workspace.dependencies]`，再由子 crate 引用，避免重复声明。
- **Bevy 默认功能必须显式开启**：注释明确要求“Default features disabled”，因为音频相关系统库（alsa-sys、libudev-sys）在无头 CI 环境中不可用。
- **glam 的 fast-math 必须保持禁用**：注释说明这是为了保证与 CesiumJS 规格的 IEEE-754 两位舍入一致性，属于硬性约束。
- **Dependabot 仅覆盖 GitHub Actions**：当前 `.github/dependabot.yml` 只配置了 `github-actions` 生态，npm 和 Rust 依赖尚未纳入自动更新流程。
- **私有注册表/镜像**：仓库中未发现 `.npmrc` 中配置 registry、`//registry.npmjs.org/:_authToken` 或 Cargo 的 `config.toml` 中配置 source 替换，因此当前依赖来源均为公开源（npm registry、crates.io）。