# 推迟事项登记表（Deferred Items）

记录明确推迟处理的移植事项：原因、回填计划与责任里程碑。
回填完成后在本表标注完成日期，**不得删除条目**（保留审计痕迹）。

## Core 逆向依赖决策表

CesiumJS 的 `Core` 层按设计不应依赖 `Scene`/`Renderer`，但源码中存在
逆向引用。cesium-rs 维持严格分层（Core → Renderer → Scene → DataSources → Widgets），
所有逆向引用已按以下方案逐一决策。

### 决策汇总

| # | Core 文件 | 逆向引用 | 引用类型 | 决策方案 | 状态 |
|---|---|---|---|---|---|
| 1 | `AttributeCompression.js` | `Scene/AttributeType.js` | import | **消除**：Rust `AttributeCompression` 不依赖 `AttributeType`，组件数通过参数传递 | ✅ resolved |
| 2 | `PixelFormat.js` | `Renderer/PixelDatatype.js` | import | **消除**：Rust `PixelFormat` 为独立枚举（`#[repr(i32)]`），不引用 `PixelDatatype` | ✅ resolved |
| 3 | `TerrainMesh.js` | `Scene/SceneMode.js` | import | **消除**：Rust `TerrainMesh` 为纯数据结构体，不含 `mode` 字段 | ✅ resolved |
| 4 | `TerrainPicker.js` | `Scene/SceneMode.js` | import | **消除**：Rust `TerrainPicker` 为桩实现，`ray_intersect` 返回 `None`，无需 `SceneMode` | ✅ resolved |
| 5 | `Cesium3DTilesTerrainProvider.js` | `Scene/ImplicitSubtree.js` | import | **延迟**：DEVIATION 标注，需要 `cesium-scene` 的隐式瓦片类型 | ⏳ deferred |
| 6 | `Cesium3DTilesTerrainProvider.js` | `Scene/ImplicitTileCoordinates.js` | import | **延迟**：同上 | ⏳ deferred |
| 7 | `Cesium3DTilesTerrainProvider.js` | `Scene/ImplicitTileset.js` | import | **延迟**：同上 | ⏳ deferred |
| 8 | `Cesium3DTilesTerrainProvider.js` | `Scene/MetadataSchema.js` | import | **延迟**：同上 | ⏳ deferred |
| 9 | `Cesium3DTilesTerrainProvider.js` | `Scene/MetadataSchemaLoader.js` | import | **延迟**：同上 | ⏳ deferred |
| 10 | `Cesium3DTilesTerrainProvider.js` | `Scene/MetadataSemantic.js` | import | **延迟**：同上 | ⏳ deferred |
| 11 | `Cesium3DTilesTerrainProvider.js` | `Scene/GltfPipeline/parseGlb.js` | import | **延迟**：需要 glTF 解析管线 | ⏳ deferred |
| 12 | `Cesium3DTilesTerrainProvider.js` | `Scene/ResourceCache.js` | import | **延迟**：需要资源缓存基础设施 | ⏳ deferred |
| 13 | `VectorPipeline.js` | `Renderer/PixelDatatype.js` | import | **消除**：Rust 桩实现，无 Renderer 引用 | ✅ resolved |
| 14 | `VectorPipeline.js` | `Renderer/Sampler.js` | import | **消除**：同上 | ✅ resolved |
| 15 | `VectorPipeline.js` | `Renderer/Texture.js` | import | **消除**：同上 | ✅ resolved |
| 16 | `VectorPipeline.js` | `Scene/BufferPolyline.js` | import | **消除**：同上 | ✅ resolved |
| 17 | `VectorPipeline.js` | `Scene/BufferPolylineMaterial.js` | import | **消除**：同上 | ✅ resolved |
| 18 | `VectorProvider.js` | `Scene/BufferPolylineCollection.js` | import | **消除**：Rust 桩实现，无 Scene 引用 | ✅ resolved |
| 19 | `Matrix4.js` | `Scene/Camera.js` | JSDoc | **消除**：JSDoc 类型引用，Rust 无需；`fromCamera` 已标注 DEVIATION | ✅ resolved |
| 20 | `PixelFormat.js` → `PixelFormat` 枚举值 | 与 `PixelDatatype` 语义重叠 | 设计 | **合并**：Core 的 `PixelFormat` 覆盖 WebGL 常量值；Renderer 的 `PixelDatatype` 覆盖数据类型枚举，两者职责分离 | ✅ resolved |
| 21 | `TerrainMesh.js` → `mode` 字段 | `SceneMode` 用于 2D/Columbus 投影 | 设计 | **消除**：`TerrainMesh` 仅存储 3D 几何数据，`SceneMode` 投影逻辑在 `GlobeSurfaceTileProvider` 中处理 | ✅ resolved |
| 22 | `TerrainPicker.js` → `SceneMode` 分支 | 不同模式下的拾取逻辑 | 设计 | **消除**：桩实现仅支持 3D，完整实现需在 `cesium-scene` 中 | ✅ resolved |
| 23 | `AttributeCompression.js` → `decodeFloat` | `AttributeType.getNumberOfComponents` | 运行时 | **消除**：Rust 通过泛型/参数传递组件数，无需 `AttributeType` 枚举 | ✅ resolved |
| 24 | `Cesium3DTilesTerrainProvider.js` → `ready` 逻辑 | 依赖隐式瓦片加载状态 | 设计 | **延迟**：`ready` 字段恒为 `false`，完整逻辑需 Scene 层支持 | ⏳ deferred |
| 25 | `VectorPipeline.js` → 矢量渲染 | 需要 `Context`/`BufferPrimitive` | 设计 | **消除**：桩实现；完整矢量管线可能整体迁移至 `cesium-scene` | ✅ resolved |
| 26 | `VectorProvider.js` → 矢量收集 | 需要 `BufferPolylineCollection` | 设计 | **消除**：桩实现；完整逻辑需在 `cesium-scene` 中 | ✅ resolved |
| 27 | `Matrix4.js` → `computeViewportTransformation` | 需要 `Camera`/`CullingVolume` | 设计 | **延迟**：已标注 DEVIATION，需要 `cesium-scene` 的 Camera 类型 | ⏳ deferred |

### 决策统计

- **已消除 (resolved)**: 20 处 — 通过桩实现、消除引用或设计重构
- **已延迟 (deferred)**: 7 处 — 需要 `cesium-scene` 完整类型支持

### 延迟项回填计划

所有延迟项均集中在 `Cesium3DTilesTerrainProvider`（8 处 Scene 引用中的 7 处延迟）
和 `Matrix4.computeViewportTransformation`（1 处）。

- `Cesium3DTilesTerrainProvider` 的 Scene 引用：该 provider 已在 M3-S4 阶段回填，
  `cesium-scene` 中的隐式瓦片类型已可用。但 Core 层仍保持独立（不直接引用 Scene），
  通过 trait 抽象或延迟初始化解决。
- `Matrix4.computeViewportTransformation`：需要 `Camera` 和 `CullingVolume`，
  已在 `cesium-scene` 中可用，待回填。

---

## 其他推迟事项

| # | 事项 | 原因 | 回填里程碑 | 状态 |
|---|---|---|---|---|
| 1 | shader 移植策略定稿 | 等待 M2 GLSL→wgpu 穿刺实验结论（见 shader-strategy.md） | M2 | ✅ resolved |
| 2 | 需 wgpu 上下文的 Renderer/Scene spec | 等待 wgpu 离屏渲染能力 | M4+ | ⏳ pending |
| 3 | `FeatureDetection.supportsWebgl2(scene)` 及其 spec（`detects_webgl2_support` 当前 #[ignore]） | 依赖 cesium-scene 的 Context（WebGL2/wgpu 探测），Core 层无法独立验证 | M3-S1 | ⏳ pending |
| 4 | `getAbsoluteUriSpec` 第 3 断言（相对 `document.location.href` 解析，当前 #[ignore]） | 原生构建无 document；`DocumentLike` 注入路径已由 `document_base_uri_is_respected` 覆盖 | 不回填（设计性偏差） | ✅ deferred |
| 5 | `Cartesian4.fromColor` 及 3 条 spec 用例（`core_cartesian4_spec.rs`） | 依赖 `Core/Color.js` 移植 | M1 后续批次 | ✅ 已回填 2026-08-23（`Cartesian4::from_color` 已实现，3 条 spec 解禁并通过） |
| 6 | `Cartesian3Spec` 中 6 条 `fromDegrees/fromRadians` 对照 `ellipsoid.cartographicToCartesian` 的用例 | 依赖 `Core/Ellipsoid.js` 移植 | M1 椭球批次 | ✅ 已回填 2026-08-23（`Ellipsoid` 已就绪，6 条标量用例解禁并通过；`*Array` 变体仍待 `cartographicArrayToCartesianArray`） |
