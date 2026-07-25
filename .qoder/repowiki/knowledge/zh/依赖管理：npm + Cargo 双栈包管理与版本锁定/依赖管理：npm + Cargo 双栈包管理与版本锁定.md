---
kind: dependency_management
name: 依赖管理：npm + Cargo 双栈包管理与版本锁定
category: dependency_management
scope:
    - '**'
source_files:
    - package.json
    - .npmrc
    - ThirdParty.json
    - cesiumrust/Cargo.toml
    - cesiumrust/Cargo.lock
    - .github/dependabot.yml
    - Tools/package.json
---

本仓库采用 **npm（Node.js）与 Cargo（Rust）双栈** 的依赖管理体系，分别管理前端 JavaScript/TypeScript 生态与 Rust 移植层的第三方库。

### 1. 前端依赖管理（npm）
- **声明文件**：根 `package.json` 集中声明运行时依赖（`dependencies`）与开发依赖（`devDependencies`），并通过 `workspaces` 字段将 `packages/engine`、`packages/widgets`、`packages/sandcastle` 纳入单仓工作区。
- **版本策略**：使用语义化版本范围（如 `^8.6.5`、`^26.1.0`），未生成 `package-lock.json`（`.npmrc` 中设置 `package-lock=false`），依赖解析由 npm 按需完成。
- **依赖覆盖**：通过 `overrides` 字段强制统一子依赖版本，例如将 `protobufjs` 在 `@huggingface/transformers` 和 `allotment` 的依赖树中固定到指定版本，避免冲突。
- **许可证清单**：`ThirdParty.json` 与 `ThirdParty.extra.json` 维护所有第三方包的名称、版本、许可证与来源 URL，用于合规审计。
- **自动更新**：`.github/dependabot.yml` 配置每日扫描 GitHub Actions 依赖更新（当前仅覆盖 github-actions，尚未扩展至 npm/cargo）。

### 2. Rust 依赖管理（Cargo）
- **Workspace 结构**：`cesiumrust/Cargo.toml` 定义多 crate workspace，包含 domain（领域层）、ports（端口抽象）、adapters（适配器）、application（应用）与 specs（集成测试）等模块。
- **版本集中管理**：通过 `[workspace.dependencies]` 段集中声明所有 crate 的版本约束（如 `glam = "0.29"`、`bevy = "0.15"`、`tokio = { version = "1", features = ["full"] }`），各子 crate 通过 `path = "..."` 引用内部 crate。
- **锁定文件**：`cesiumrust/Cargo.lock` 完整记录所有依赖及其 checksum，确保构建可重现；该文件由 Cargo 自动生成并提交至版本控制。
- **构建配置**：`[profile.dev]` 启用增量编译与调试信息分离，`[profile.release]` 启用 thin LTO 与单 codegen unit 优化。

### 3. 架构与约定
- **分层隔离**：Rust 侧严格区分 domain（纯逻辑，无框架依赖）、ports（trait 接口）、adapters（Bevy/IO 实现），依赖方向单向向下，便于替换与测试。
- **测试对齐**：`cesiumrust/specs` crate 对应 CesiumJS 原生 Specs 测试套件，通过逐模块验证 Rust 实现与 JS 行为一致性。
- **工具链独立**：`Tools/package.json` 单独声明工具类依赖（如 JSDoc、ast-grep 规则），与主工程解耦。

### 4. 开发者应遵循的规则
- 新增依赖时优先添加到 `package.json` 或 `Cargo.toml` 的集中位置，避免在子 crate 中重复声明。
- 使用 `overrides`（npm）或 `[workspace.dependencies]`（Cargo）统一管理版本冲突。
- 提交前确保 `Cargo.lock` 已更新，保持依赖锁定文件与源码同步。
- 关注 Dependabot 自动创建的 PR，及时审查并合并依赖更新。