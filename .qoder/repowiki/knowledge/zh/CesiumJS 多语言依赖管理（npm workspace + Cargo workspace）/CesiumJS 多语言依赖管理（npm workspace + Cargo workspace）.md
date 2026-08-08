---
kind: dependency_management
name: CesiumJS 多语言依赖管理（npm workspace + Cargo workspace）
category: dependency_management
scope:
    - '**'
source_files:
    - package.json
    - packages/engine/package.json
    - packages/widgets/package.json
    - packages/sandcastle/package.json
    - ThirdParty.json
    - ThirdParty.extra.json
    - cesiumrust/Cargo.toml
    - cesiumrust/Cargo.lock
    - .github/dependabot.yml
---

## 1. 使用的系统/方法

- **JavaScript/TypeScript 层**：使用 npm workspaces 聚合 `packages/engine`、`packages/widgets`、`packages/sandcastle` 三个子包，根 `package.json` 作为入口，通过 Gulp/esbuild/Karma/Playwright 统一构建与测试。
- **Rust 层**：`cesiumrust/` 下使用 Cargo workspace，将 domain、ports、adapters、application、specs 等模块组织为多个 crate，共享 `Cargo.toml` 中的 `[workspace.dependencies]`。
- **第三方许可证清单**：`ThirdParty.json` 与 `ThirdParty.extra.json` 集中记录所有第三方库的名称、版本、许可证与来源 URL，由构建脚本生成或维护。
- **自动更新**：GitHub Dependabot（`.github/dependabot.yml`）每日扫描 GitHub Actions 依赖；`greenkeeper.json` 保留历史配置痕迹。

## 2. 关键文件与位置

- `package.json` — 根工作区定义、顶层依赖、`workspaces`、`overrides`（强制 protobufjs/react 版本）、`scripts`（build/test/release）
- `packages/engine/package.json` — `@cesium/engine` 核心引擎包，声明运行时依赖（tween.js、draco3d、protobufjs、lerc 等）
- `packages/widgets/package.json` — `@cesium/widgets` UI 组件包，依赖 `@cesium/engine` 与 `nosleep.js`
- `packages/sandcastle/package.json` — 示例沙盒应用，依赖 React/Monaco/Vite 等开发工具链
- `ThirdParty.json` / `ThirdParty.extra.json` — 第三方库许可证与版本元数据清单
- `cesiumrust/Cargo.toml` — Rust workspace 定义，集中声明 `glam`、`bevy`、`serde`、`tokio`、`ureq` 等依赖及内部 crate 路径引用
- `cesiumrust/Cargo.lock` — 锁定所有 Rust 依赖的精确版本与 checksum
- `.github/dependabot.yml` — Dependabot 对 GitHub Actions 的每日更新策略

## 3. 架构与约定

- **分层依赖**：engine → widgets → sandcastle，逐层向上依赖，避免循环。
- **版本对齐**：engine 与 widgets 版本号保持同步（如 26.x / 16.x），根 package.json 中通过 `^` 语义化版本约束。
- **依赖覆盖**：根 `overrides` 强制解决冲突（如 `protobufjs` 在 `@huggingface/transformers` 与 `allotment` 中的不同版本需求）。
- **Rust 依赖治理**：所有外部 crate 集中在 workspace 根 `Cargo.toml` 的 `[workspace.dependencies]` 中声明，各子 crate 仅引用名称，确保版本一致。
- **二进制/源码分离**：engine 构建产物输出到 `Build/`，源码位于 `Source/`，`files` 字段控制发布内容。
- **许可证合规**：`ThirdParty.json` 由构建流程自动生成/校验，发布前需保证清单完整。

## 4. 开发者应遵循的规则

- **新增 JS 依赖**：仅在对应包的 `dependencies` 中添加，优先使用 `^` 语义化版本；若存在版本冲突，在根 `overrides` 中统一解决。
- **新增 Rust 依赖**：在 `cesiumrust/Cargo.toml` 的 `[workspace.dependencies]` 中声明，子 crate 通过 `{ path = ... }` 引用内部 crate，外部 crate 仅写名称。
- **不要手动编辑 lockfile**：`package-lock.json`（npm）与 `Cargo.lock` 均由工具自动生成，提交前确保已运行 `npm install` / `cargo build`。
- **许可证更新**：添加新第三方库后，同步更新 `ThirdParty.json` / `ThirdParty.extra.json`，确保包含名称、版本、许可证与 URL。
- **版本升级流程**：先升级 engine 包，再升级 widgets/sandcastle，最后调整根 `package.json` 的版本号与 `overrides`。
- **CI 依赖检查**：Dependabot 会创建 PR，审查时注意 `overrides` 是否引入破坏性变更。
- **Node 版本约束**：所有包通过 `engines.node >= 22.0.0` 锁定最低 Node 版本，避免环境差异。