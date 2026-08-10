---
kind: dependency_management
name: CesiumJS 多语言依赖管理：npm workspaces + Cargo workspace + 第三方许可清单
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
    - ThirdParty.json
    - ThirdParty.extra.json
    - cesiumrust/Cargo.toml
    - cesiumrust/Cargo.lock
---

## 1. 使用的系统与工具

本仓库是一个混合语言工程，包含两套独立的依赖管理系统：
- **JavaScript/TypeScript**：使用 npm（无 lockfile）+ npm workspaces，配合 Gulp 构建脚本与 Dependabot 自动化更新。
- **Rust（cesiumrust）**：使用 Cargo workspace，通过 `Cargo.toml` 的 `[workspace.dependencies]` 集中声明版本，并以 `Cargo.lock` 锁定全部传递依赖。

此外，仓库还维护一份手工编排的 **ThirdParty.json / ThirdParty.extra.json** 清单，用于生成发布产物中的第三方组件许可信息。

## 2. 关键文件

- `package.json`：根工作区入口，声明 `workspaces: ["packages/engine", "packages/widgets", "packages/sandcastle"]`、`engines.node >=22.0.0`、运行时依赖 `@cesium/engine ^26.1.0`、`@cesium/widgets ^16.1.0`、`protobufjs ^8.6.5`，并通过 `overrides` 强制子依赖的 `protobufjs`、`react`/`react-dom` 版本。
- `packages/engine/package.json`：`@cesium/engine` 包，声明其直接运行时依赖（如 `draco3d`、`dompurify`、`protobufjs`、`meshoptimizer` 等）。
- `packages/widgets/package.json`：`@cesium/widgets` 包，仅依赖 `@cesium/engine` 与 `nosleep.js`。
- `packages/sandcastle/package.json`：私有示例应用，依赖 React 生态与 `@huggingface/transformers`，并通过自身 `overrides` 解决 `allotment` 对 `react` 版本的冲突。
- `.npmrc`：`package-lock=false`，明确禁用 npm lockfile。
- `.github/dependabot.yml`：仅对 `github-actions` 启用每日更新到 `main` 分支；未配置 npm 依赖自动更新。
- `gulpfile.js`：`buildThirdParty()` 任务将 `ThirdParty.extra.json` 与 `package.json` 合并生成 `ThirdParty.json`。
- `ThirdParty.json` / `ThirdParty.extra.json`：手工维护的第三方组件清单（名称、许可证、版本、URL），由构建流程产出。
- `cesiumrust/Cargo.toml`：workspace 根，定义 31 个 domain crate、2 个 ports crate、3 个 adapters crate、application 与 specs，并在 `[workspace.dependencies]` 中统一声明 `glam 0.29`、`bevy 0.15`、`serde`、`tokio`、`ureq`、`image 0.25` 等核心 crate 的版本。
- `cesiumrust/Cargo.lock`：被提交到仓库的完整锁文件，锁定所有 crates.io 依赖及其传递依赖的精确版本与 checksum。

## 3. 架构与约定

### JavaScript 层
- **Monorepo 结构**：通过 npm workspaces 将 `engine`、`widgets`、`sandcastle` 三个包纳入同一安装图。根 `package.json` 同时作为聚合包，`dependencies` 中引用 `@cesium/engine` 和 `@cesium/widgets`，使 `cesium` 主包可独立发布。
- **版本策略**：各包均使用语义化版本号（engine `26.1.0`、widgets `16.1.0`、sandcastle `0.4.1`），并通过 `^` 范围允许小版本升级。根级 `overrides` 强制解决冲突依赖（如 `protobufjs`、`react`）。
- **Node 引擎约束**：根与各子包均声明 `engines.node >=22.0.0`，确保运行环境一致。
- **无 lockfile 策略**：`.npmrc` 设置 `package-lock=false`，依赖解析完全由 npm registry 决定；更新依赖需手动编辑 `package.json` 并重新安装。
- **第三方许可清单**：`ThirdParty.extra.json` 列出所有需要归因的第三方库（含非 npm 来源如 `basis_universal`、`Knockout`），构建时由 `gulp buildThirdParty` 合并为 `ThirdParty.json`，随发布物分发。

### Rust 层（cesiumrust）
- **Workspace 内聚**：`cesiumrust/Cargo.toml` 以 workspace 形式组织 31 个领域 crate，按 domain/ports/adapters/application/specs 分层，内部 crate 通过 `path = ...` 引用，外部 crate 通过 `[workspace.dependencies]` 集中声明版本。
- **严格锁定**：`Cargo.lock` 已提交至版本控制，保证跨平台、跨时间构建的可重复性；每个 crate 的 `Cargo.toml` 不单独声明 `version`，由 workspace 统一管理。
- **最小特性集**：对外部 crate（如 `bevy`、`image`）显式关闭默认特性（如 audio、gilrs），避免在无系统库的 CI 环境中失败。
- **数值精度约束**：注释明确禁止启用 `glam` 的 `fast-math` 特性，以保证与 CesiumJS 原始 Specs 的 IEEE-754 位级一致性。

## 4. 约定与约束

- **npm 依赖必须通过 `package.json` 声明**：根与子包的 `dependencies`/`devDependencies` 是依赖的唯一来源，不存在 `node_modules` 或 vendored 源码。
- **禁止使用 lockfile**：`.npmrc` 的 `package-lock=false` 是硬性约束，不得引入 `package-lock.json`。
- **版本冲突通过 `overrides` 解决**：根 `package.json` 与 `packages/sandcastle/package.json` 使用 `overrides` 强制子依赖树中的 `protobufjs`、`react`/`react-dom` 版本，避免依赖分裂。
- **Dependabot 仅覆盖 GitHub Actions**：当前 `.github/dependabot.yml` 只配置了 `github-actions` 生态的每日更新，未配置 npm 依赖的自动 PR，因此 JS 依赖升级需人工发起。
- **Rust 依赖通过 workspace 集中管理**：新增 crate 必须在 `members` 列表注册，外部依赖必须在 `[workspace.dependencies]` 声明后由各子 crate 引用，禁止在子 crate 中重复声明版本。
- **Rust 锁文件必须同步提交**：`Cargo.lock` 已入库，修改依赖后需提交该文件以确保可重现构建。
- **第三方组件许可必须登记**：任何进入最终发布的第三方代码（包括非 npm 来源）需在 `ThirdParty.extra.json` 登记名称、许可证、版本与 URL，否则 `buildThirdParty` 无法生成合规的 `ThirdParty.json`。