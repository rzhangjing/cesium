---
kind: dependency_management
name: 双栈依赖管理：npm workspaces + Cargo workspace（含 Dependabot 与锁定策略）
category: dependency_management
scope:
    - '**'
source_files:
    - package.json
    - .npmrc
    - .github/dependabot.yml
    - packages/engine/package.json
    - packages/widgets/package.json
    - packages/sandcastle/package.json
    - cesiumrust/Cargo.toml
    - cesiumrust/Cargo.lock
---

## 1. 使用的系统/工具

本仓库是一个**混合语言工程**，同时维护两套独立的依赖管理体系：

- **JavaScript/TypeScript 栈**：使用 npm（`package.json` + `workspaces`），通过 Gulp 构建脚本驱动。未启用 `package-lock.json`（`.npmrc` 中 `package-lock=false`），依赖版本以 `^` 语义化范围声明。
- **Rust 栈**：使用 Cargo workspace（`cesiumrust/Cargo.toml`），通过 `Cargo.lock` 锁定所有 crate 的精确版本与 checksum，全部从 crates.io 拉取。

自动化更新由 GitHub Actions 的 **Dependabot**（`.github/dependabot.yml`）负责，当前仅配置了 `github-actions` 生态的每日更新。

## 2. 关键文件

| 文件 | 作用 |
|---|---|
| `package.json` | 根工作区入口，定义 `workspaces`、`dependencies`、`devDependencies`、`overrides` |
| `packages/engine/package.json` | `@cesium/engine` 子包发布元数据与运行时依赖 |
| `packages/widgets/package.json` | `@cesium/widgets` 子包（同结构） |
| `packages/sandcastle/package.json` | Sandcastle 示例应用依赖 |
| `.npmrc` | 禁用 `package-lock` |
| `cesiumrust/Cargo.toml` | Workspace 定义、`workspace.dependencies` 集中声明、profile 优化 |
| `cesiumrust/Cargo.lock` | Rust 依赖完整锁定表（6000+ 行） |
| `.github/dependabot.yml` | Dependabot 配置 |
| `Tools/package.json` | 构建工具链独立 CJS 环境 |

## 3. 架构与约定

### 3.1 JavaScript 层（npm workspaces）

- 根 `package.json` 通过 `workspaces: ["packages/engine", "packages/widgets", "packages/sandcastle"]` 将三个子包纳入同一依赖图。
- 根包自身只保留极少量运行时依赖（`@cesium/engine`、`@cesium/widgets`、`protobufjs`），其余均为 `devDependencies`（Gulp、Karma、Playwright、ESLint、Prettier 等）。
- 通过 `overrides` 字段强制解决传递依赖冲突：例如把 `@huggingface/transformers` 下的 `onnxruntime-web` 的 `protobufjs` 回退到 `^7.6.4`，并把 `allotment` 的 `react`/`react-dom` 提升到 `^19.0.0`。
- 引擎核心库 `@cesium/engine` 在 `packages/engine/package.json` 中单独声明其运行时依赖（draco3d、meshoptimizer、pako、protobufjs 等），与根包的 dev 依赖解耦。
- Node 引擎要求 `>=22.0.0`，统一通过 `engines` 字段约束。

### 3.2 Rust 层（Cargo workspace）

- `cesiumrust/Cargo.toml` 使用 `[workspace]` 并显式列出 35 个 member crate，按领域分层组织：`domain/*`（纯业务逻辑）、`ports/*`（trait 契约）、`adapters/*`（Bevy/IO 实现）、`application/cesium-app`（组装入口）、`specs`（集成测试）。
- 所有外部 crate 的版本集中在 `[workspace.dependencies]` 中声明（如 `bevy = "0.15"`、`glam = { version = "0.29", features = ["serde"] }`、`tokio = { version = "1", features = ["full"] }`、`ureq = "2"`、`image = { version = "0.25", default-features = false, features = ["png", "jpeg"] }`），各子 crate 通过 `{ workspace = true }` 引用，避免版本漂移。
- 内部 crate 同样通过 `workspace.dependencies` 以路径引用（如 `cesium-geospatial = { path = "domain/geospatial" }`），保证跨 crate 接口一致。
- `glam` 明确关闭 `fast-math` feature，注释说明这是为了与原版 CesiumJS 的 IEEE-754 两舍入算术保持位级一致——这是该仓库最重要的依赖决策之一。
- 发布策略：`[workspace.package]` 设置 `publish = false`，所有 crate 均不直接发布到 crates.io，仅作为 workspace 内部依赖。
- 构建 profile：`dev` 开启增量编译与 unpacked debuginfo；`release` 启用 thin LTO、单 codegen unit 以获得最大优化。

### 3.3 锁定与更新策略

- **Rust**：`Cargo.lock` 被提交到版本控制，确保全团队和 CI 使用完全相同的依赖树与 checksum。
- **JavaScript**：`.npmrc` 中 `package-lock=false`，意味着不生成锁文件；依赖版本以 `^` 语义范围允许自动升级，配合 Dependabot 推送 PR。
- **GitHub Actions**：Dependabot 仅监控 `github-actions` 生态，未配置 npm 或 cargo 的自动更新（需手动触发或扩展配置）。

## 4. 开发者应遵循的规则

1. **新增 Rust crate**：先在 `cesiumrust/Cargo.toml` 的 `[workspace.members]` 中添加路径，再在 `[workspace.dependencies]` 中声明版本，子 crate 通过 `{ workspace = true }` 引用，不要在各 crate 内重复写版本号。
2. **新增 JS 依赖**：区分运行时依赖（放入 `dependencies`）与开发依赖（放入 `devDependencies`）。若为子包专用，优先放入对应 `packages/*/package.json`。
3. **解决依赖冲突**：使用根 `package.json` 的 `overrides` 字段进行精准覆盖，而非修改子包依赖。
4. **不要手动编辑 `Cargo.lock`**：它由 Cargo 自动生成，变更通过 `cargo update` 产生。
5. **不要启用 glam 的 `fast-math`**：会破坏与 CesiumJS Specs 的数值一致性。
6. **Node 版本**：必须使用 `>=22.0.0`，CI 与本地均应满足此约束。
7. **发布**：所有 crate `publish = false`，仅通过 npm workspace 机制发布 `@cesium/engine`、`@cesium/widgets` 等 npm 包。
8. **更新依赖**：运行 `cargo update` 更新 Rust 依赖；对 JS 依赖可使用 `npm outdated` 查看差异后手动调整版本范围，或通过扩展 Dependabot 配置实现自动 PR。
