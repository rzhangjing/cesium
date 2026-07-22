---
kind: dependency_management
name: CesiumJS 多包工作区与依赖管理策略
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

## 系统概览

CesiumJS 采用 npm workspaces 多包架构，将核心库拆分为三个独立发布的包：@cesium/engine（渲染引擎）、@cesium/widgets（UI 组件）和 @cesium/sandcastle（示例画廊），由根 package.json 统一编排。依赖管理围绕 npm 生态展开，不使用 lockfile、不 vendoring，通过版本范围与 overrides 控制依赖树。

## 关键文件与位置

- package.json — 根工作区配置、顶层依赖声明、scripts 入口、overrides 依赖强制
- .npmrc — 全局禁用 package-lock.json（package-lock=false）
- .github/dependabot.yml — GitHub Actions 每日扫描 GitHub Actions 依赖更新
- packages/engine/package.json — @cesium/engine 运行时依赖（draco3d、protobufjs、dompurify 等）
- packages/widgets/package.json — @cesium/widgets 依赖（仅依赖 engine + nosleep.js）
- packages/sandcastle/package.json — 开发工具站点的 React/Vite 依赖，标记为 private: true

## 架构与约定

1. 工作区划分
   - packages/engine：纯运行时库，声明大量图形/数据解析第三方依赖（如 draco3d、meshoptimizer、lerc、topojson-client、protobufjs）。
   - packages/widgets：轻量 UI 层，仅依赖 @cesium/engine 与 nosleep.js，避免引入重型框架。
   - packages/sandcastle：私有开发站点，使用 React + Vite + Monaco Editor，不参与发布。

2. 版本策略
   - 所有包均使用 ^ 语义化版本范围，允许小版本自动升级。
   - 根与子包保持版本号同步（当前均为 26.1.0 / 16.1.0 / 0.4.1），通过 workspaces 在本地直接解析内部依赖，无需发布中间产物。

3. 依赖冲突治理
   - 根 overrides 强制解决传递性冲突：
     - 将 protobufjs 统一提升到 ^8.6.5，同时针对 @huggingface/transformers 的 onnxruntime-web 子依赖回退到 ^7.6.4。
     - 将 allotment → use-resize-observer 下的 react/react-dom 锁定到 ^19.0.0，避免沙盒中 React 版本分裂。

4. 无锁文件策略
   - .npmrc 设置 package-lock=false，仓库不提交 package-lock.json，依赖安装结果完全由 package.json 中的版本范围决定，CI 环境需确保一致的 npm 缓存或镜像。

5. 自动化更新
   - Dependabot 仅监控 github-actions 生态，未启用对 npm 包的自动 PR，意味着依赖升级由人工维护。

## 开发者应遵循的规则

- 新增依赖时明确归属：运行时依赖放入对应 packages/*/package.json 的 dependencies；构建/测试工具放入根 devDependencies。
- 不要提交 lockfile：移除任何生成的 package-lock.json，否则会被 CI 拒绝。
- 谨慎使用 overrides：仅在确实存在传递性冲突时使用，并附带注释说明原因。
- 保持 Node 版本一致：所有包声明 engines.node >= 22.0.0，本地与 CI 必须满足该要求。
- Sandcastle 是私有包：其 private: true 表明不应被外部引用，仅作为开发辅助站点。
- 依赖升级流程：由于未启用 npm 的 Dependabot，手动升级时需检查 overrides 是否仍有效，并验证各包脚本（gulp build、gulp test）是否通过。