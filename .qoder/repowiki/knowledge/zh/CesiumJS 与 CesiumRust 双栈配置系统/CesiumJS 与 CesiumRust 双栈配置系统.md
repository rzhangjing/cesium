---
kind: configuration_system
name: CesiumJS 与 CesiumRust 双栈配置系统
category: configuration_system
scope:
    - '**'
source_files:
    - gulpfile.js
    - gulpfile.apps.js
    - Specs/e2e/playwright.config.js
    - packages/sandcastle/vite.config.dev.ts
    - package.json
    - cesiumrust/Cargo.toml
    - cesiumrust/application/cesium-app/src/main.rs
---

本仓库包含两个独立子项目，各自采用不同的配置策略：

## CesiumJS（JavaScript/TypeScript）

- **构建期配置**：通过 gulpfile.js、gulpfile.apps.js 以及各包的 vite.config.dev.ts 等文件集中管理。构建选项（minify、sourcemap、node 模式等）通过命令行参数 yargs(process.argv) 注入，并支持 process.env.DEPLOYED_URL、process.env.PROD、process.env.SANDCASTLE_ORIGIN 等环境变量覆盖。
- **测试配置**：Playwright 使用 Specs/e2e/playwright.config.js 定义多浏览器项目；Karma 使用 Specs/karma.conf.cjs，并通过 __karma__.config.args 传入类别过滤、WebGL 验证等开关。
- **运行时配置**：Sandcastle 示例应用通过 packages/sandcastle/scripts/buildStatic.js 中的 createSandcastleConfig 生成配置对象，并在运行时由 AIClientFactory 等模块从用户设置中读取 API Key（Gemini、Anthropic、Vertex AI），未找到统一的 .env 加载器，密钥通常由外部工具（如 GitHub Actions 中的 dotenvx）注入到 CI 环境。
- **约定**：所有构建脚本统一通过 process.env.* + yargs(argv) 双层注入，无持久化配置文件；.env 仅出现在 .github/actions/update-tokens/ 的 README 说明中，用于本地开发辅助。

## CesiumRust（Bevy/DOM 重构）

- **工程级配置**：cesiumrust/Cargo.toml 以 workspace 形式声明 domain/ports/adapters/application 四层 crate 成员及共享依赖，无运行时配置加载逻辑。
- **应用入口**：application/cesium-app/src/main.rs 直接硬编码 Bevy WindowPlugin 分辨率与标题，未见任何 config.toml、.env 或 serde 反序列化结构体作为全局配置。
- **领域配置**：domain 层存在若干纯数据配置结构体（如 TimelineConfig、EllipsoidConfig、ScenePipelineConfig），但均为在代码中以字面量构造或通过函数参数传递，不存在从文件或环境变量解析的配置子系统。
- **结论**：Rust 侧当前处于早期 DDD 骨架阶段，尚未引入 config、dotenv、toml 等配置框架，所有“配置”均以 Rust struct 常量或构造函数参数形式内联。

## 开发者约定

1. JS 侧新增构建/测试开关优先走 yargs(argv) 参数，其次才用 process.env，避免散落的环境变量。
2. Rust 侧如需引入运行时配置，建议在 application/cesium-app 层新增 config.rs，使用 serde + toml/dotenv 统一加载，再注入为 Bevy Resource，保持 domain 层纯净。