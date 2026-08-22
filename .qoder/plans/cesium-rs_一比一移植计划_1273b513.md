# cesium-rs：CesiumJS 一比一 Rust 移植计划

## 摘要与基线事实

- **移植范围**：`packages/engine/Source`（Core 292 .js、Renderer 47、Scene 385 + Model/GltfPipeline、DataSources 109、Shaders 318 .glsl、Workers 53、Widget 3、ThirdParty、Assets）+ `packages/widgets/Source`（119 文件）。引擎总计约 950 个源文件、~36.5 万行 JS。
- **关键基线**：`d:\Rust\cesium\cesium-rs` 目录当前**完全为空**（已核实），历史会话中"已建骨架/已移植数学模块"的产物未落盘。因此本计划以 **M0 骨架重建** 为起点，此前记忆中的移植工作将按本计划 spec 驱动重做并验证。
- **验收事实来源**：`packages/engine/Specs` 共 675 个 `*Spec.js`（Core 212 / Scene 332 / DataSources 92 / Renderer 38 / Widget 1），目录与 Source 严格一一对应；共享测试数据 `Specs\Data`（1,147 文件）只读引用。
- **环境**：rustc/cargo 1.95.0（Windows）。

## 硬性约束

1. 严格一比一镜像 CesiumJS 模块结构与 API 表面（类型名、方法名、语义对齐）。
2. 渲染后端 **wgpu**（不用 Bevy）。
3. 与 `d:\Rust\cesium\cesiumrust` 完全隔离：不得引用/复用其代码与架构，仅可参考其踩坑经验（LOD 接缝、no_data 伪影、UV 翻转等）。
4. 领域计算一律 f64；仅在 GPU 上传边界降为 f32；不引入 glam fast-math（Core 数学建议纯手写实现，不依赖 glam，保证与 JS Number 的逐位对齐）。

## Workspace 结构（M0 交付物）

```
cesium-rs/
├── Cargo.toml                     # workspace；members=crates/*；[workspace.dependencies] 集中管理
├── crates/
│   ├── cesium-core/               # ← Source/Core（294 文件）
│   ├── cesium-renderer/           # ← Source/Renderer（47 文件，wgpu 后端）
│   ├── cesium-scene/              # ← Source/Scene（508 文件含 Model/GltfPipeline）
│   ├── cesium-data-sources/       # ← Source/DataSources（109 文件）
│   ├── cesium-widgets/            # ← Source/Widget + packages/widgets/Source
│   ├── cesium-shaders/            # ← Source/Shaders（318 .glsl）
│   ├── cesium-workers/            # ← Source/Workers（53 文件）
│   └── cesium-test-utils/         # 测试支撑：approx 断言宏、spec helper（对应仓库根 Specs/ 工具）
├── specs/                         # 镜像测试：tests/core、tests/renderer、tests/scene、tests/data_sources、tests/widget
├── examples/viewer-demo/          # winit + wgpu 运行器（M5）
└── docs/
    ├── PORTING_CONVENTIONS.md     # 移植规约（见下）
    ├── MAPPING.md                 # JS→Rust 文件级对照台账（not_started/ported/tested 三态）
    ├── deviations.md              # 平台性偏差登记（wgpu/DOM 等无法一比一处）
    ├── deferred.md                # Core 逆向依赖等暂缓项登记
    └── shader-strategy.md         # 着色器转译方案 go/no-go 决策记录
```

依赖方向由 Cargo 编译期强制（单向）：`core ← renderer ← scene ← data-sources ← widgets`；shaders/workers 仅依赖 core。

## 移植规约（docs/PORTING_CONVENTIONS.md，所有 crate 遵循）

1. **文件镜像**：`Source/Core/Cartesian3.js → cesium-core/src/cartesian3.rs`；超大文件（Scene.js 172KB、Cesium3DTileset.js 145KB、Camera.js 127KB、GlobeSurfaceTileProvider.js 111KB、GeometryPipeline.js 97KB、Resource.js 88KB 等）允许拆为同名目录 `scene/scene/*.rs` 子模块，保持前缀同名以维持追溯。
2. **文件头锚定**：`//! Ported from packages/engine/Source/Core/Cartesian3.js (commit <sha>)`。
3. **debug 断言裁剪**：`//>>includeStart('debug', pragmas.debug)` → `#[cfg(debug_assertions)]` / `debug_assert!`。
4. **result 复用参数模式**：`fn add(left, right, result: &mut Self)`（返回单元类型）+ 配套分配变体 `add_new`；在规约中固化，避免逐文件争论。
5. **JS 动态特性映射表**：mixin → trait；getter → 方法；duck typing → enum/trait object；原型扩展 → 组合。
6. **偏差标注**：任何无法一比一之处在代码中标 `// DEVIATION:` 并登记 docs/deviations.md。
7. **spec 成对移植**：每个源文件与其 Spec 同批移植，禁止"先全量移植后补测试"。

## 里程碑与任务分解

### M0 — 骨架重建与基建
1. 创建 workspace `cesium-rs/Cargo.toml`：8 个 crate + `specs` + `examples`；`[workspace.dependencies]` 统一 wgpu（features: webgl）、naga（glsl-in）、rayon、tokio、futures、reqwest、approx、serde、image、wasm-bindgen/web-sys（条件 feature）。
2. 8 个 crate 骨架（Cargo.toml + lib.rs 桩），依赖方向写死。
3. `cesium-test-utils`：`assert_approx!`/`assert_epsilon!` 宏（ULP 分级：纯算术 0 容差、超越函数 ≤2 ULP）、spec 公共 helper（对应仓库根 `Specs/addDefaultMatchers.js` 等）。
4. 测试数据接入：环境变量 `CESIUM_SPECS_DATA` 指向 `d:\Rust\cesium\Specs\Data`（只读引用，不复制），`specs/src/data.rs` 路径解析（对应 `Specs/absolutize.js`）。
5. 编写 docs/PORTING_CONVENTIONS.md、MAPPING.md（全量文件对照表）、deviations.md、deferred.md 初始版本。
6. **验收**：`cargo build --workspace && cargo test --workspace` 全绿；git tag `m0-skeleton`（作为回退点）。

### M1 — cesium-core（294 文件，分 4 批，spec 212 个同步镜像到 specs/tests/core/）
- **W1 基础工具**（~30 文件）：defined、Check、DeveloperError、RuntimeError、defaultValue、clone、freezeObject、FeatureDetection、formatError、createGuid、Event 等。
- **W2 数学与时间**（风险最高）：CesiumMath/math、Cartesian2/3/4、Matrix2/3/4、Quaternion、Ray、Plane、Intersect、IntersectionTests、BoundingSphere、BoundingRectangle、OrientedBoundingBox、CullingVolume、Ellipsoid、Cartographic、Geographic/WebMercatorProjection、Transforms、JulianDate（44KB，闰秒/TAI/UTC）、Clock、TimeInterval(Collection)、Spline 族、多项式求根族。移植模板基准：`Source/Core/Cartesian3.js`（class + 静态方法 + result 参数 + debug 断言四模式）。
- **W3 几何系统**：Geometry、GeometryAttribute、GeometryPipeline（97KB，拆 mod 保持 API）、全部 ~60 个 `*Geometry.js`/`*OutlineGeometry.js`、AttributeCompression。
- **W4 投影/地形数据/IO**：CesiumTerrainProvider、ArcGISTiledElevationTerrainProvider、Heightmap/QuantizedMeshTerrainData、Credit、AssociativeArray；Resource（88KB，reqwest 映射 fetch 语义，网络 spec 用 `Specs\Data` 本地数据替代 ion 端点）。
- **Core 逆向依赖环处置**（27 行跨层 import，逐个显式决策，登记 deferred.md，禁止循环依赖进 Cargo）：`AttributeCompression→Scene/AttributeType`（常量下沉）、`PixelFormat→Renderer/PixelDatatype`（下沉或延迟）、`TerrainMesh/TerrainPicker→SceneMode`（延迟回填）、`Cesium3DTilesTerrainProvider→Scene 9 处`（延迟到 M3-S4 回填）、`VectorPipeline/VectorProvider`（延迟）。
- **暂缓项**写入 docs/deferred.md，M3 回填。
- **验收**：specs/tests/core 对原版 212 个 spec 全量处置（通过，或 `#[ignore]` 注明依赖后续里程碑）；tag `m1-core`。

### M2 — cesium-shaders + cesium-renderer（最大技术风险，独立隔离）
1. **着色器穿刺验证（全项目最大 go/no-go 决策点）**：取 `Source/Shaders/ViewportQuadVS.glsl`+FS，在 wgpu 上画出全屏四边形；结论写入 docs/shader-strategy.md（候选：naga glsl-in 程序化转译 / naga-cross / 运行时编译）。
2. `cesium-shaders`：318 个 .glsl 镜像六个子目录（Builtin 142、Model 39、PostProcessStages 26、Materials 19、Voxels 16、Appearances 14 + 顶层 63），`include_str!` 嵌入，文件名一比一；`ShaderSource.js` 的预处理（defines/pragmas/拼接）用 Rust 字符串处理镜像后再转译；先跑 naga 全量转译冒烟，失败文件清单化逐个修正。
3. `cesium-renderer` 移植顺序（47 文件）：`Context.js`（1,733 行，封装 wgpu::Device/Queue，ContextLimits 从 wgpu::Limits 填充，含 ShaderCache/TextureCache）→ RenderState（可哈希状态描述 → RenderPipeline 缓存 key，防 pipeline 爆炸）→ ShaderProgram/ShaderSource/ShaderCache → Buffer/Texture/CubeMap/Texture3D/Framebuffer(Manager) → DrawCommand/ClearCommand/ComputeCommand/Pass/PassState（保留命令模式与 execute() 语义，帧内按 pass→pipeline→texture 排序）→ UniformState/AutomaticUniforms（per-frame uniform buffer 增量更新）→ VertexArray/VertexArrayFacade → TextureAtlas。`Core/WebGLConstants` 枚举一比一保留用于 API 对齐。
4. Renderer spec 分流（38 个）：纯逻辑的移植为单元测试；依赖真实 GL 的改为 wgpu headless（WARP 软件适配器）集成测试，无法等价的 `#[ignore]` + 登记 deviations.md。
5. **验收**：ViewportQuad 渲染冒烟通过 + 38 个 Renderer spec 100% 处置；tag `m2-renderer`。

### M3 — cesium-scene（508 文件，按子系统切片）
- **S1 基础**：SceneMode、FrameState、CreditDisplay、Camera（127KB 单独一批）、Scene.js 骨架（172KB：先构造/update/render 主干，功能开关逐步点亮）；回填 M1 暂缓的 SceneMode 相关文件。
- **S2 地球/瓦片管线**（历史 bug 高发区，spec 先行）：Globe、GlobeSurfaceTile(Provider 111KB)、QuadtreePrimitive/Tile（三级加载队列 + TileReplacementQueue LRU + 5ms 时间片语义保留）、ImageryLayer(Collection)（重投影管线 L1363-1503）、TerrainFillMesh；同步移植 `Specs/Scene/GlobeSurfaceTileProviderSpec.js` 等先行验证。
- **S3 绘制原语**：Primitive（83KB）、Appearance 族、Material（64KB）、BillboardCollection、Label、PointPrimitiveCollection、PolylineCollection、ViewportQuad、SkyBox/SkyAtmosphere/Sun/Moon。
- **S4 3D Tiles / glTF / 其余**：Cesium3DTileset（145KB）、Cesium3DTile、GltfLoader（101KB）、Model/（99 文件）、GltfPipeline/（24）、ShadowMap、PostProcess、Voxel、Expression、GaussianSplat（可选最后）；回填 Cesium3DTilesTerrainProvider。ThirdParty 4 个 wasm（draco/basis/zip/splats）选型 Rust 原生替代并登记豁免。
- **建议暂停点**：S2 完成后冻结检查（瓦片管线历史 bug 高发）。
- **验收**：地球影像+地形在本地离线数据下渲染正确 + 332 个 Scene spec 处置清单完成；tag `m3-scene`。

### M4 — cesium-workers + cesium-data-sources
- **Workers**：`TaskProcessor.js` 一比一 API（maximum_active_tasks、task_completed_event）；native 后端 rayon/线程池 + channel 零拷贝 move（对应 transferable 语义）；wasm 后端 web workers（feature 隔离）。53 个 worker 移植为纯函数 + 薄入口适配，`createGeometry.js` 分发器保留；`Specs/TestWorkers` 7 个 mock 同步移植。
- **DataSources**（109 文件）：Entity/Property 体系 → CzmlDataSource → GeoJsonDataSource → KmlDataSource → GpxDataSource → DataSourceDisplay；spec 直接消费 `Specs/Data` 对应目录。
- **验收**：92 个 DataSources spec 处置完成；tag `m4-datasources`。

### M5 — cesium-widgets + 端到端
- **Widgets**：`Source/Widget/CesiumWidget.js`（引擎侧 1,552 行）+ widgets 包 20 个子目录；**边界划定**：ViewModel 逻辑（ClockViewModel、createCommand 等）一比一保留，DOM/Knockout 视图层做适配（桌面 winit + 最小 DomSurface trait 或占位），Viewer.js 的 5 个 mixin 映射为 trait；DOM 偏离全部登记 deviations.md。
- **examples/viewer-demo**：winit + wgpu Surface 组装帧循环，复刻 Sandcastle HelloWorld（本地离线影像）。
- **收尾**：全量 `cargo test --workspace`；生成 spec 覆盖矩阵（675 个原版 spec → 通过/ignore+原因/未移植 三态清单）；tag `m5-widgets`。

## 横切策略

- **精度验证**：领域层 f64（Rust f64 与 JS double 同为 IEEE-754，算术逐位一致；超越函数以 ULP 容差 + Specs 黄金向量验证）；GPU 边界降 f32；禁用一切 fast-math。
- **TaskWorker/Resource 双后端 trait**：IO 与并发的抽象在 M1 期预留，为后续 WASM 目标（wasm-bindgen + wgpu webgl feature）留门，但不阻塞桌面主线。
- **每个里程碑 git tag = 回退点**；任一里程碑失败只需回退上一 tag，无跨层债务（依赖方向编译期强制）。

## 测试计划

| 层 | 策略 |
|---|---|
| Core/DataSources | 纯逻辑单元测试，`approx` ULP 分级断言，消费 `Specs/Data` |
| Renderer | 纯逻辑单测 + wgpu headless（WARP）集成测试，不可等价者 ignore+登记 |
| Scene | spec 先行（尤其 S2 瓦片管线）；渲染类走 headless 或本地离线数据冒烟 |
| Widgets | ViewModel 逻辑单测；视图层适配以 viewer-demo 冒烟验收 |

## 风险与缓解

| 风险 | 等级 | 缓解 |
|---|---|---|
| GLSL→WGSL 转译（318 文件，自定义预处理/宏拼接） | 极高 | M2 第一天穿刺实验定 go/no-go；失败文件清单化；动态拼接逻辑留在 shader_source.rs |
| WebGL 状态机 → wgpu 语义错位（RenderState 组合爆炸） | 极高 | RenderState→Pipeline 哈希缓存独立可测；WARP headless 消除 CI 硬件差异；偏差登记制 |
| 地形/影像瓦片管线（历史：LOD 接缝、no_data 伪影、UV 翻转） | 高 | spec 先行 + 吸收 cesiumrust 教训（288 阈值、failed/placeholder 区分、祖先纹理继承）但独立实现；S2 后设暂停点 |
| 时间系统（闰秒/TAI/UTC/f64 双字） | 高 | W2 出口条件 = 时间类 spec 全量通过 |
| Resource 网络语义（CORS/重定向/重试/blob 多态） | 高 | 按方法逐个映射；本地 mock 数据替代真实端点；偏差登记 |
| Core 逆向依赖环（10 文件） | 中 | 逐个决策：常量下沉或延迟回填，登记 deferred.md，禁止循环进 Cargo |
| 巨型单文件（6 个 100KB+） | 中 | API 一比一、内部拆 mod，spec 驱动逐方法点亮 |
| ThirdParty wasm（draco/basis/zip/splats） | 中 | Rust 原生替代，登记为一比一豁免项 |
| Widgets DOM/Knockout 无对应物 | 中 | 明确"ViewModel 一比一、View 适配"边界，最后实施 |
| wgpu WebGL2 后端无 compute | 中 | 重投影提供 fragment-shader 回退（CesiumJS 本身有先例） |

## 被否决的备选方案

1. **假设已有骨架、跳过 M0 直接续写**——已核实 cesium-rs 为空目录且 git 无相关记录，基线不成立，必须重建。
2. **复用/改造 cesiumrust（DDD+Bevy）架构**——违反硬性约束 3，且 Bevy 与"一比一镜像 CesiumJS API"目标冲突；仅参考其踩坑经验。
3. **手写全部 WGSL（不经转译）**——318 个文件规模下不可控，失去与 GLSL 原文的逐文件追溯性；仅在穿刺实验证明 naga 路线完全不可行时回退此方案。
4. **先全量移植源码、后补测试**——失去逐里程碑绿线与回退点，返工风险不可接受；坚持文件级成对移植。
5. **引入 glam 作为数学底座**——fast-math/f32 混入风险破坏位级一致性目标；Core 数学手写 f64 实现。

## 关键文件（移植锚点）

- `packages/engine/Source/Renderer/Context.js`（1,733 行）— wgpu 适配核心分歧点
- `packages/engine/Source/Scene/Scene.js`（172KB）— 最大单体，决定 M3 切片方式
- `packages/engine/Source/Core/JulianDate.js`（44KB）— 时间系统基石，M1-W2 出口门槛
- `packages/engine/Source/Scene/GlobeSurfaceTileProvider.js`（111KB）— 瓦片管线核心，M3-S2 验收焦点
- `packages/engine/Specs/`（675 个 Spec + `Specs/Data` 1,147 文件）— 全部里程碑的唯一验收标准