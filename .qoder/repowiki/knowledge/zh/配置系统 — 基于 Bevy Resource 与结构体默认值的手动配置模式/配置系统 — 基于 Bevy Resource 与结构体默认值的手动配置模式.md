---
kind: configuration_system
name: 配置系统 — 基于 Bevy Resource 与结构体默认值的手动配置模式
category: configuration_system
scope:
    - '**'
source_files:
    - cesiumrust/application/cesium-app/src/main.rs
    - cesiumrust/adapters/bevy-render/src/lib.rs
    - cesiumrust/adapters/bevy-render/src/scene_pipeline.rs
    - cesiumrust/domain/atmosphere/src/scattering.rs
    - cesiumrust/domain/effects/src/post_process.rs
    - cesiumrust/domain/globe/src/atmosphere.rs
    - cesiumrust/domain/globe/src/surface.rs
    - cesiumrust/crates/app/src/main.rs
    - cesiumrust/Cargo.toml
---

## 概述
CesiumRust（DDD + 六边形架构 × Bevy 全功能重构）在 Rust/Cargo workspace 中**没有引入统一的配置框架**（如 `config`、`dotenv`、`serde_yaml` 等），而是采用一种轻量、显式的配置方式：通过领域层/适配器层的 `struct XxxConfig` 配合 `Default` trait，并在应用启动时以 Bevy `Resource` 注入。运行时配置来源主要是代码内硬编码与命令行参数，未实现外部配置文件或环境变量加载。

## 关键文件与位置
- `cesiumrust/application/cesium-app/src/main.rs` — Bevy App 入口，仅通过 `WindowPlugin` 设置窗口标题与分辨率，无外部配置读取。
- `cesiumrust/adapters/bevy-render/src/lib.rs` — 定义 `EllipsoidConfig`（椭球体渲染配置），作为 Bevy `Resource` 注册；`setup_globe` 系统通过 `Res<EllipsoidConfig>` 消费。
- `cesiumrust/adapters/bevy-render/src/scene_pipeline.rs` — 定义 `ScenePipelineConfig`（视场角、裁剪开关等），由渲染管线函数直接接收引用。
- `cesiumrust/domain/atmosphere/src/scattering.rs` — 定义 `SkyBoxConfig`、`LightingConfig`，均提供 `Default` 实现。
- `cesiumrust/domain/effects/src/post_process.rs` — 定义 `BloomConfig`、`AmbientOcclusionConfig`、`FogConfig`、`ToneMappingConfig`、`ColorCorrectionConfig` 等后处理配置结构体，全部带 `Default`。
- `cesiumrust/domain/globe/src/atmosphere.rs` / `domain/globe/src/surface.rs` — 定义 `SkyAtmosphereConfig`、`GlobeConfig` 等。
- `cesiumrust/crates/app/src/main.rs` — 使用 `env_logger::init()` 初始化日志，但未见 `log::info!` 以外的配置相关调用。
- `cesiumrust/Cargo.toml` — Workspace 根清单，仅声明依赖与成员包，不包含任何 `[package.metadata.config]` 或类似字段。

## 架构与约定
1. **配置即结构体**：每个子系统暴露一个或多个 `XxxConfig` struct，字段均为公开且可配置的参数（如 `enabled: bool`、`intensity: f64`、`sources: [Option<String>; 6]` 等）。
2. **默认值优先**：所有 `XxxConfig` 都实现 `Default`，给出“开箱即用”的合理默认值，使模块在无外部配置时仍可运行。
3. **Bevy Resource 注入**：仅在渲染适配器层将配置包装为 `#[derive(Resource)]` 并通过 `app.init_resource::<XxxConfig>()` 注册到 Bevy 容器，供系统以 `Res<XxxConfig>` 读取。
4. **无集中配置聚合器**：不存在全局 `AppConfig`、`load_config()`、`parse_config()` 之类的统一入口；各子系统各自持有自己的配置结构体。
5. **无外部配置源**：仓库中未发现 `.env`、`config.toml`、`settings.yaml` 等配置文件，也未见 `std::env::var`、`dotenv`、`config` crate 的使用。CLI 参数解析也未发现。
6. **日志配置独立**：唯一的外部化配置是 `env_logger::init()`，用于控制日志级别，属于基础设施而非业务配置。

## 开发者应遵循的规则
- 新增子系统配置时，定义 `pub struct XxxConfig { ... }` 并实现 `impl Default for XxxConfig`，确保默认值能驱动最小可用场景。
- 若该配置需要跨系统共享，将其包装为 Bevy `Resource` 并通过插件的 `build` 阶段 `init_resource` 注册；否则直接以函数参数传递。
- 避免在领域层（`domain/*`）引入 I/O 或外部配置库，保持纯函数式与可测试性；配置加载逻辑应放在应用层或适配器层。
- 如需支持外部配置文件或环境变量，应在 `application/cesium-app` 或新的 `crates/config` 包中实现，并通过类型安全的结构体向上层暴露，而不是散落各处调用 `env::var`。
- 现有 `LightingConfig`、`FogConfig`、`BloomConfig` 等已展示良好范式：字段命名清晰、`Default` 给出工程经验值、附带计算辅助方法（如 `compute_fog_factor`、`apply`）。

## 置信度评估
medium — 代码中存在大量 `XxxConfig` 结构体与 `Default` 实现，构成一致的“配置即结构体”约定；但缺乏统一的配置加载机制、外部配置源与文档化的规则说明，因此体系尚处于早期手工阶段。