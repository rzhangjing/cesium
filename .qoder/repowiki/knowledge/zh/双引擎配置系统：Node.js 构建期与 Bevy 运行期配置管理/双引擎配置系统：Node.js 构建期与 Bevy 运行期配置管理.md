---
kind: configuration_system
name: 双引擎配置系统：Node.js 构建期与 Bevy 运行期配置管理
category: configuration_system
scope:
    - '**'
source_files:
    - server.js
    - gulpfile.js
    - scripts/build.js
    - package.json
    - cesiumrust/application/cesium-app/src/main.rs
    - cesiumrust/adapters/bevy-render/src/lib.rs
    - cesiumrust/Cargo.toml
---

## 1. 使用的系统与工具

本项目包含两个独立的运行时环境，各自采用不同的配置策略：

- **CesiumJS（JavaScript/TypeScript）**：基于 Node.js 的 Gulp + esbuild 构建系统，通过 `process.env`、命令行参数（yargs）和 JSON 配置文件进行配置。
- **CesiumRust（Rust/Bevy）**：基于 Bevy 游戏引擎的插件架构，使用 Rust 结构体作为配置对象，通过 `Resource` 机制注入到应用生命周期中。

## 2. 核心文件与位置

### JavaScript 侧配置
- `server.js`：开发服务器入口，使用 yargs 解析命令行参数，读取 `process.env.SANDCASTLE_NO_EMBEDDINGS` 等环境变量
- `gulpfile.js`：主构建脚本，通过 `process.env.DEPLOYED_URL` 控制部署行为
- `scripts/build.js`：esbuild 打包配置，支持 pragma 条件编译（debug/include/exclude）
- `gulpfile.apps.js`：应用构建配置，读取 `process.env.PROD` 和 `process.env.SANDCASTLE_ORIGIN`
- `package.json`：定义 npm scripts 和工作区配置

### Rust 侧配置
- `cesiumrust/application/cesium-app/src/main.rs`：Bevy 应用组装，硬编码窗口配置
- `cesiumrust/adapters/bevy-render/src/lib.rs`：定义 `EllipsoidConfig`、`ScenePipelineConfig` 等配置结构体
- `cesiumrust/Cargo.toml`：工作区依赖管理和编译配置

## 3. 架构与设计决策

### CesiumJS 配置分层
1. **构建时配置**：通过 `pragmas` 机制在编译期裁剪代码（debug/include/exclude）
2. **运行时配置**：通过 `process.env` 环境变量控制行为（如 `SANDCASTLE_NO_EMBEDDINGS`、`DEPLOYED_URL`）
3. **启动参数**：通过 yargs 解析命令行选项（端口、public模式、production模式等）

### CesiumRust 配置模式
1. **结构体配置**：使用 Rust 结构体定义配置项，实现类型安全的配置
2. **Bevy Resource 注入**：通过 `app.init_resource::<Config>()` 将配置注入到应用上下文
3. **默认值策略**：为每个配置结构体实现 `Default` trait，提供合理的默认值
4. **常量配置**：使用 `pub const` 定义全局常量（如 `METERS_PER_RENDER_UNIT`）

## 4. 开发者应遵循的规则

### JavaScript 侧
- 环境变量命名使用大写蛇形命名法（如 `SANDCASTLE_NO_EMBEDDINGS`、`DEPLOYED_URL`）
- 敏感信息不应硬编码，应通过环境变量或外部配置文件注入
- 构建配置通过 pragmas 注释控制，避免运行时分支判断
- 使用 yargs 统一处理命令行参数，提供清晰的帮助文档

### Rust 侧
- 配置结构体必须实现 `Default` trait，确保可独立使用
- 通过 Bevy 的 `Resource` 机制传递配置，避免全局变量
- 配置变更应在应用初始化阶段完成，而非运行时动态修改
- 使用类型系统保证配置的有效性，避免运行时验证开销

### 跨语言约定
- 两个引擎的配置应该保持语义一致，便于功能对齐
- 测试环境通过环境变量区分，生产环境通过构建参数控制
- 配置文件格式优先使用 TOML（Rust）和 JSON（JavaScript），避免自定义格式
