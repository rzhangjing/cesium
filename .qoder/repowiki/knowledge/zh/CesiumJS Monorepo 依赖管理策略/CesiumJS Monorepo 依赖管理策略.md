---
kind: dependency_management
name: CesiumJS Monorepo 依赖管理策略
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
---

## 1. 使用的系统与工具

- **包管理器**: npm（通过 `.npmrc` 显式禁用 `package-lock.json`，不生成锁文件）
- **Monorepo 编排**: npm workspaces，顶层 `package.json` 的 `workspaces` 字段聚合三个子包：`packages/engine`、`packages/widgets`、`packages/sandcastle`
- **版本更新自动化**: GitHub Dependabot（`.github/dependabot.yml`），仅对 `github-actions` 生态开启每日 PR；另有遗留的 `greenkeeper.json`（Greenkeeper 已下线，仅作历史参考）
- **Node 引擎约束**: 顶层与所有子包统一声明 `engines.node >= 22.0.0`

## 2. 关键文件与位置

| 文件 | 作用 |
|---|---|
| `package.json` | 顶层工作区入口，声明 workspace、`dependencies`（指向本地 `@cesium/engine`、`@cesium/widgets`）、`overrides` 强制 protobufjs/react 版本 |
| `packages/engine/package.json` | Cesium 核心库，声明运行时依赖（draco3d、protobufjs、dompurify 等） |
| `packages/widgets/package.json` | UI 组件库，依赖 `@cesium/engine` 与 `nosleep.js` |
| `packages/sandcastle/package.json` | 示例站点（私有包），依赖 React 生态与 Monaco Editor |
| `.npmrc` | `package-lock=false`，禁止生成 lockfile |
| `.github/dependabot.yml` | 仅对 GitHub Actions 依赖启用自动升级 PR |
| `greenkeeper.json` | 历史配置，已不再生效 |

## 3. 架构与约定

### 3.1 内部包关系
```
cesium (根)
├── @cesium/engine        ← 核心引擎，无内部依赖
├── @cesium/widgets       ← 依赖 @cesium/engine ^26.1.0
└── @cesium/sandcastle    ← 私有示例应用，不对外发布
```
- 子包之间通过 npm workspace 解析，版本号使用 `^` 范围而非精确锁定，由 workspace 机制保证一致性。
- 根 `package.json` 的 `dependencies` 以 `^26.1.0` / `^16.1.0` 引用同仓库内的 engine/widgets，利用 workspace 协议在开发时直接链接源码。

### 3.2 依赖版本策略
- **运行时依赖**：engine 与 widgets 使用较宽松的 `^` 语义化版本，便于上游小版本修复自动流入。
- **冲突收敛**：通过根级 `overrides` 强制：
  - `protobufjs` 统一为 `^8.6.5`（避免下游包引入旧版）
  - `@huggingface/transformers` 下的 `onnxruntime-web` 再覆盖其 `protobufjs` 至 `^7.6.4`
  - `allotment` → `use-resize-observer` 中的 `react`/`react-dom` 统一为 `^19.0.0`
- **Node 版本**：所有包均要求 Node ≥ 22，确保构建工具链一致。

### 3.3 锁文件与缓存
- 明确关闭 `package-lock.json` 生成，CI 中直接使用 `npm install` 从 registry 拉取最新满足范围的版本。
- CI workflow（`.github/workflows/*.yml`）全部使用 `run: npm install`，未使用 `npm ci`，意味着每次构建都会重新解析依赖树。

### 3.4 第三方资产
- 除 npm 包外，大量二进制/数据资源（glTF、3D Tiles、字体、KTX2 纹理等）以静态文件形式存放在 `Apps/SampleData`、`Specs/Data` 目录，不参与 npm 依赖管理。

## 4. 开发者应遵循的规则

1. **新增依赖**：
   - 运行时依赖放入对应包的 `dependencies`，开发期工具放入 `devDependencies`。
   - 若需跨包统一版本，优先在根 `overrides` 中声明，而非在各子包重复指定。
2. **不要提交锁文件**：`.gitignore` 已忽略 `package-lock.json` 与 `yarn.lock`，请勿手动生成并提交。
3. **保持 Node 版本一致**：新依赖不得突破 `>=22.0.0` 的 engines 约束。
4. **内部包版本同步**：修改 `@cesium/engine` 或 `@cesium/widgets` 的版本号后，需在引用方（根 package.json 及 widgets/sandcastle）同步更新 `^x.y.z` 范围。
5. **Dependabot 行为**：当前仅对 GitHub Actions 依赖开启自动 PR，对 npm 依赖未启用——如需自动升级，应在 `.github/dependabot.yml` 中添加 `package-ecosystem: "npm"` 条目。
6. **Sandcastle 私有性**：`@cesium/sandcastle` 标记为 `"private": true`，不应被外部项目作为依赖安装。
