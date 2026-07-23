---
kind: dependency_management
name: 双栈依赖管理：npm workspaces + Cargo workspace 协同治理
category: dependency_management
scope:
    - '**'
source_files:
    - package.json
    - .npmrc
    - .github/dependabot.yml
    - greenkeeper.json
    - packages/engine/package.json
    - packages/widgets/package.json
    - packages/sandcastle/package.json
    - cesiumrust/Cargo.toml
    - cesiumrust/Cargo.lock
---

本仓库同时维护 CesiumJS（JavaScript/TypeScript）与 CesiumRust（Rust/Bevy）两套引擎，各自采用语言生态的标准包管理器进行依赖声明、版本锁定与更新自动化。

## JavaScript/TypeScript 栈（根目录 + packages/*）

- **包管理器**：npm（Node ≥22），使用 npm workspaces 组织多包结构。
- **工作区定义**：`package.json` 中 `workspaces: ["packages/engine", "packages/widgets", "packages/sandcastle"]`，其中 `@cesium/engine` 与 `@cesium/widgets` 为对外发布包，`@cesium/sandcastle` 为私有示例应用。
- **依赖声明**：运行时依赖集中在 `dependencies`，如 `protobufjs`、`draco3d`、`meshoptimizer`、`dompurify` 等；构建/测试/文档工具放在 `devDependencies`，包括 gulp、karma、playwright、eslint、prettier、typescript 等；通过 `overrides` 对传递依赖做强制版本对齐（例如将 `protobufjs` 统一至 `^8.6.5`，并针对 `@huggingface/transformers` 的 `onnxruntime-web` 子依赖单独覆盖）。
- **锁文件策略**：`.npmrc` 设置 `package-lock=false`，不生成 lockfile，依赖解析由 CI 或开发者本地完成。
- **更新自动化**：`.github/dependabot.yml` 仅配置了 `github-actions` 生态的日常更新；遗留的 `greenkeeper.json` 已不再被使用（Dependabot 已取代 Greenkeeper）。
- **版本同步**：根 `package.json` 与 `packages/*/package.json` 中的版本号需保持一致（根 `1.143.0` 对应 engine `26.1.0` / widgets `16.1.0`），发布流程通过 `gulp release` / `websiteRelease` 脚本驱动。

## Rust 栈（cesiumrust/）

- **包管理器**：Cargo，采用单 workspace 多 crate 结构，`resolver = "2"`。
- **Workspace 成员**：按 DDD 分层组织——`domain/*`（纯领域逻辑）、`ports/*`（trait 契约）、`adapters/*`（Bevy/IO 实现）、`application/cesium-app`（装配入口），全部在根 `Cargo.toml` 的 `[workspace] members` 中集中声明。
- **依赖集中化**：通过 `[workspace.dependencies]` 统一管理第三方 crate 版本（如 `bevy = "0.15"`、`glam = "0.29"`、`serde = "1"`、`tokio = "1"` 等），各子 crate 仅引用名称而不重复指定版本，确保全仓一致。
- **内部 crate 引用**：所有内部 crate 以 `{ path = "domain/xxx" }` 形式通过 workspace dependency 别名引入（如 `cesium-geospatial`、`cesium-scene` 等），禁止直接路径依赖绕过版本约束。
- **锁文件**：`Cargo.lock` 已提交到版本库，包含完整依赖树与 checksum，保证可复现构建。
- **发布策略**：`[workspace.package].publish = false`，整个 Rust 子项目不向 crates.io 发布，仅作为仓库内共享库。

## 跨栈协作与约定

- 两套依赖系统完全独立，无交叉引用；CesiumJS 通过 WASM 模块间接调用 Rust 产物。
- 开发环境要求 Node ≥22，Rust 工具链由各自生态管理。
- 未使用私有 npm registry 或 Cargo registry 镜像，均从默认源拉取。
- 未启用 vendoring（既无 `vendor/` 目录，也无 `cargo vendor` 相关配置）。

## 开发者应遵循的规则

1. 新增 JS 依赖：仅在根 `package.json` 或对应 `packages/*/package.json` 的 `dependencies`/`devDependencies` 中声明，避免在子包间重复；必要时通过 `overrides` 解决冲突。
2. 新增 Rust crate：先在 `cesiumrust/Cargo.toml` 的 `[workspace.dependencies]` 中声明版本，再在目标 crate 的 `Cargo.toml` 中以名称引用，不要写死版本。
3. 保持锁文件一致性：Rust 侧修改后务必提交 `Cargo.lock`；JS 侧因禁用 lockfile，建议在 PR 中附带 `npm ls --depth=0` 输出以便审查。
4. 版本升级：优先依赖 Dependabot 自动 PR；手动升级时注意 `overrides` 与 `engines.node` 的联动影响。
5. 不发布 Rust crate：所有内部 crate 保持 `publish = false`，如需对外暴露请改为独立的 crates.io 仓库。