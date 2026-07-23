---
kind: dependency_management
name: 多语言依赖管理：npm workspace + Cargo workspace 双栈治理
category: dependency_management
scope:
    - '**'
source_files:
    - package.json
    - cesiumrust/Cargo.toml
    - cesiumrust/Cargo.lock
    - ThirdParty.json
    - .github/dependabot.yml
    - packages/engine/package.json
---

## 系统概览

CesiumRust 仓库同时维护两套独立的技术栈，各自采用其生态的标准依赖管理方案：

- **JavaScript/TypeScript 侧**（根目录）：基于 npm workspaces，使用 `package.json` 声明运行时与开发期依赖，配合 `overrides` 解决子树版本冲突，并通过 `ThirdParty.json` 集中记录第三方组件的许可证信息。
- **Rust 侧**（`cesiumrust/`）：基于 Cargo workspace，通过顶层 `Cargo.toml` 的 `[workspace.dependencies]` 统一收敛 crate 版本，`Cargo.lock` 锁定完整依赖图。

两套体系互不耦合，分别由各自的工具链负责解析、安装与更新。

## 关键文件与位置

- JavaScript 层
  - `package.json`：根工作区入口，定义 `workspaces`、`dependencies`、`devDependencies`、`overrides` 以及构建脚本。
  - `packages/engine/package.json`、`packages/widgets/package.json`、`packages/sandcastle/package.json`：子包清单，被根 workspaces 引用。
  - `ThirdParty.json`：第三方库清单，记录名称、许可证、版本与来源 URL，用于发布合规审计。
  - `.github/dependabot.yml`：仅对 GitHub Actions 启用每日自动 PR，未覆盖 npm/Cargo。
- Rust 层
  - `cesiumrust/Cargo.toml`：workspace 根，声明 `members`、`default-members`、`[workspace.package]` 与 `[workspace.dependencies]` 统一版本。
  - `cesiumrust/Cargo.lock`：完整锁文件，包含 crates.io 源与每个包的 checksum。
  - 各子 crate 的 `Cargo.toml`（如 `domain/geospatial/Cargo.toml` 等）仅引用 workspace 中已声明的依赖名，不再重复指定版本。

## 架构与约定

1. **版本收敛策略**
   - JS：根 `package.json` 通过 `overrides` 强制提升关键子依赖（如 `protobufjs`、`react`），确保所有子包共享一致版本；`engines.node >= 22.0.0` 约束运行环境。
   - Rust：`[workspace.dependencies]` 集中声明 glam、bevy、serde、tokio 等公共 crate 的版本号，子 crate 以 `{ path = "..." }` 形式引用内部 crate，避免版本漂移。

2. **依赖分层**
   - JS 将运行时依赖（`dependencies`）与构建/测试依赖（`devDependencies`）严格分离，`sideEffects` 列表帮助打包器做 tree-shaking。
   - Rust 按 DDD 限界上下文划分 crate（`domain/*`、`ports/*`、`adapters/*`、`application/*`），仅应用层依赖 Bevy 等框架，领域层保持纯 Rust 无外部渲染绑定。

3. **许可证合规**
   - `ThirdParty.json` 作为单一事实来源，列出所有对外发布的第三方库及其许可证，供 release 流程生成 NOTICE 文件。
   - 该清单与 `@cesium/engine` 的 `dependencies` 基本一一对应，新增依赖需同步更新两处。

4. **更新机制**
   - 当前仅配置了 Dependabot 针对 GitHub Actions 的每日检查，npm 与 Cargo 依赖尚未接入自动化升级。
   - 手动升级路径：修改对应 `package.json` / `Cargo.toml` → 重新安装生成 lockfile → 提交变更。

## 开发者应遵循的规则

- **新增依赖时**
  - JS：在根 `package.json` 或对应子包中添加条目，若影响多个子包则优先放入根 `dependencies`；必要时在 `overrides` 中处理冲突。
  - Rust：在 `cesiumrust/Cargo.toml` 的 `[workspace.dependencies]` 中声明版本，子 crate 仅写依赖名。
  - 两者都需在 `ThirdParty.json` 补充许可证元数据。
- **不要**在子 crate 的 `Cargo.toml` 或子包 `package.json` 中重复声明已在 workspace 根管理的依赖版本。
- **不要**直接编辑 `Cargo.lock` 或任何 lockfile，应由包管理器自动生成。
- 升级后务必运行 `gulp build` / `cargo build` 验证编译与测试通过，再提交变更。
