---
kind: configuration_system
name: CesiumJS 运行时配置体系
category: configuration_system
scope:
    - '**'
source_files:
    - packages/engine/Source/Core/Ion.js
    - packages/engine/Source/Core/ITwinPlatform.js
    - .github/actions/update-tokens/replacements.js
---

CesiumJS 仓库中不存在统一的“应用级配置文件”（如 .yaml/.toml/.env）或集中式配置加载器。其运行时配置主要通过以下机制完成：

1. **全局单例对象**：核心配置集中在 `packages/engine/Source/Core/Ion.js` 暴露的 `Cesium.Ion` 命名空间上，包括 `defaultAccessToken`（默认 ion token，内置一个仅用于评估的示例值）与 `defaultServer`（默认 ion API 地址）。所有使用 ion 资源的模块（`IonResource`、`IonGeocoderService`、`createWorldImagery`、`createWorldTerrain` 等）均从该对象读取凭据，若未显式传入则回退到全局默认值。
2. **平台级默认凭据**：`ITwinPlatform.defaultAccessToken` / `defaultShareKey` 采用相同模式，作为 iTwin 相关 API 的全局默认凭据入口。
3. **构建期注入**：仓库通过 esbuild/Gulp 管线在打包阶段替换常量（见 `.github/actions/update-tokens/replacements.js`），将 CI 环境中的真实 token 注入到发行包中；开发时仍使用源码内嵌的示例 token。
4. **测试与 E2E 环境变量**：Playwright 与 Karma 测试通过 `process.env.CI`、`__karma__.config.args` 等控制行为，但这些属于测试框架配置而非运行时应用配置。
5. **Sandcastle 画廊**：示例页面通过 `<script>` 标签注入 `ION_ACCESS_TOKEN` 变量后赋值给 `Cesium.Ion.defaultAccessToken`，演示了外部注入方式。

**约定与约束**：
- 不要在业务代码中硬编码 token，应通过部署时的构建注入或运行前设置 `Cesium.Ion.defaultAccessToken`。
- 生产环境必须覆盖默认 token，否则会触发警告 Credit。
- 新增对外部服务的默认凭据应遵循 `XxxPlatform.defaultAccessToken` 模式，并在对应模块中统一回退读取。
- 仓库未提供集中式配置解析器（无 config/ 目录、无 .env 加载逻辑），所有配置均为 JS 全局对象属性。