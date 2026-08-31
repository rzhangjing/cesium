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
| 19 | `Matrix4.js` | `Scene/Camera.js` | JSDoc | **消除**：JSDoc 类型引用，Rust 无需；`fromCamera` 已标注 DEVIATION | ✅ resolved ⚠ f2 SEM-9（2026-08-25）核码未发现 `fromCamera` 的 DEVIATION 标注，与声称不符，待核 |
| 20 | `PixelFormat.js` → `PixelFormat` 枚举值 | 与 `PixelDatatype` 语义重叠 | 设计 | **合并**：Core 的 `PixelFormat` 覆盖 WebGL 常量值；Renderer 的 `PixelDatatype` 覆盖数据类型枚举，两者职责分离 | ✅ resolved |
| 21 | `TerrainMesh.js` → `mode` 字段 | `SceneMode` 用于 2D/Columbus 投影 | 设计 | **消除**：`TerrainMesh` 仅存储 3D 几何数据，`SceneMode` 投影逻辑在 `GlobeSurfaceTileProvider` 中处理 | ✅ resolved |
| 22 | `TerrainPicker.js` → `SceneMode` 分支 | 不同模式下的拾取逻辑 | 设计 | **消除**：桩实现仅支持 3D，完整实现需在 `cesium-scene` 中 | ✅ resolved |
| 23 | `AttributeCompression.js` → `decodeFloat` | `AttributeType.getNumberOfComponents` | 运行时 | **消除**：Rust 通过泛型/参数传递组件数，无需 `AttributeType` 枚举 | ✅ resolved |
| 24 | `Cesium3DTilesTerrainProvider.js` → `ready` 逻辑 | 依赖隐式瓦片加载状态 | 设计 | **延迟**：`ready` 字段恒为 `false`，完整逻辑需 Scene 层支持 | ⏳ deferred |
| 25 | `VectorPipeline.js` → 矢量渲染 | 需要 `Context`/`BufferPrimitive` | 设计 | **消除**：桩实现；完整矢量管线可能整体迁移至 `cesium-scene` | ✅ resolved |
| 26 | `VectorProvider.js` → 矢量收集 | 需要 `BufferPolylineCollection` | 设计 | **消除**：桩实现；完整逻辑需在 `cesium-scene` 中 | ✅ resolved |
| 27 | `Matrix4.js` → `computeViewportTransformation` | 需要 `Camera`/`CullingVolume` | 设计 | **延迟**：已标注 DEVIATION，需要 `cesium-scene` 的 Camera 类型 | ⏳ deferred ⚠ f2 SEM-9（2026-08-25）核码未发现对应 DEVIATION 标注，与声称不符，待核 |

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

---

## 补登：F1–F10 逐函数保真度审查（任务 #37，2026-08-25）

> 本节为 `docs/audit/f1_core_a_c.md` … `docs/audit/f10_shaders.md`（审查日 2026-08-24）发现的
> 整体桩化/推迟未实现/平台性豁免未登记项的批量补登；均按“报告时点状态”登记并标注来源。
> 行为性内联 DEVIATION 见 deviations.md 同批补登节；ignored 测试处置见 ignored_disposition.md 补登节。

| # | 事项 | 原因 | 回填里程碑 | 状态 |
|---|---|---|---|---|
| 7 | `cesium3d_tiles_terrain_data.rs` 整体桩（8 项：构造/credits/waterMask/interpolateHeight/isChildAvailable/createMesh/upsample/wasCreatedByUpsampling）+ `cesium3d_tiles_terrain_geometry_processor.rs` 空壳 unit struct | 依赖 cesium-scene 隐式瓦片/网格类型（代码注释 “Scene dependency, deferred” 未入台账） | Scene 层回填后 | ⏳ pending（来源：f1 §3.4、L529-538/L558 行） |
| 8 | `create_world_terrain_async.rs` / `create_world_bathymetry_async.rs` 桩（代码注释虚称 “Registered in deferred.md”，本条即为该登记） | 依赖 ion 资源端点与网络栈 | 网络栈就绪 | ⏳ pending（来源：f1 L878/L884 行） |
| 9 | `attribute_compression.rs` encodeRGB8/decodeRGB8（代码注释 “deferred until Color is ported”，但 Color 已移植，可回填） | 依赖已解除，待实现 | 近期批次 | ⏳ pending（来源：f1） |
| 10 | `bounding_rectangle.rs` `fromRectangle` 注释占位 + 2 条 `#[ignore]` spec | 依赖 Rectangle 交集语义补全 | 近期批次 | ⏳ pending（来源：f1） |
| 11 | Core A–C C 档 backlog 99 项 | 逐行明细见 f1 报告各文件表 C 档行 | 按批次 | ⏳ pending（来源：f1） |
| 12 | Core D–Z C 档 backlog 444 项（92 文件；缺口最大：GeometryPipeline 35/TimeIntervalCollection 19 等） | 逐行明细见 f2 报告各文件表 C 档行 | 按批次 | ⏳ pending（来源：f2） |
| 13 | Core D–Z E-未登记 73 项平台性豁免补登：ScreenSpaceEventHandler 29/TaskProcessor 12/Resource 浏览器族 11/VideoSynchronizer 10/PinBinder 8/KTX2Transcoder 3 | DOM/浏览器专属；家族模式已知但未逐文件登记 | 不回填（平台性） | ✅ 补登即处置（来源：f2） |
| 14 | Renderer `VertexArray.fromGeometry` 缺失（Major#6） | 几何→顶点缓冲转换管线未移植 | Track B 后续 | ⏳ pending（来源：f3 Major#6） |
| 15 | Renderer UniformState czm_* 92/117 缺失（Major#1） | 冒烟路径裁剪 | shader-strategy Batch B/C | ⏳ pending（来源：f3 Major#1） |
| 16 | Renderer C 档 backlog 334 项 | 逐行明细见 f3 报告各文件表 C 档行 | 按批次 | ⏳ pending（来源：f3） |
| 17 | Scene 顶层桩文件 113 个（对应 2198 条 C；清单：Appearance/ArcGisMapServerImageryProvider/Atmosphere/AttributeType/AutoExposure/Axis/Azure2DImageryProvider/B3dmParser/BatchTable/BatchTexture/Billboard/BillboardTexture/BoundingVolumeSemantics/BoxEmitter/BrdfLutGenerator/BufferLoader/BufferPoint/BufferPointCollection/BufferPointMaterial/BufferPolygon/BufferPolygonCollection/BufferPolygonMaterial/BufferPolylineCollection/BufferPrimitive/BufferPrimitiveCollection/BufferPrimitiveMaterial/buildVectorGltfFromMVT/buildVoxelCustomShader/buildVoxelDrawCommands/CameraFlightPath/Cesium3DContentGroup/Cesium3DTileContentFactory/Cesium3DTileContentType/Cesium3DTileFeatureTable/Cesium3DTileOptimizations/Cesium3DTilePass/Cesium3DTilePassState/Cesium3DTilePointFeature/Cesium3DTilesetBaseTraversal/Cesium3DTilesetHeatmap/Cesium3DTilesetMetadata/Cesium3DTilesetMostDetailedTraversal/Cesium3DTilesetSkipTraversal/Cesium3DTileStyle/Cesium3DTileStyleEngine/Cesium3DTilesVoxelProvider/Cesium3DTileVectorFeature/CircleEmitter/ClippingPolygon/ClippingPolygonCollection/CloudCollection/CloudType/ColorBlendMode/Composite3DTileContent/ConeEmitter/ContentMetadata/createBillboardPointCallback/createElevationBandMaterial/createGooglePhotorealistic3DTileset/createOsmBuildingsAsync/createTangentSpaceDebugPrimitive/createWorldImageryAsync/CubeMapPanorama/CumulusCloud/DebugAppearance/DebugCameraPrimitive/DebugInspector/DebugModelMatrixPrimitive/decodeMVT/DepthPlane/DerivedCommand/DeviceOrientationCameraController/DirectionalLight/DiscardEmptyTileImagePolicy/DiscardMissingTileImagePolicy/DracoLoader/DynamicAtmosphereLightingType/DynamicEnvironmentMapManager/EdgeFramebuffer/EllipsoidPrimitive/EllipsoidSurfaceAppearance/Empty3DTileContent/EquirectangularPanorama/findContentMetadata/findGroupMetadata/findMeshoptExtension/findTileMetadata/FrameRateMonitor/FrustumCommands/GaussianSplat3DTileContent/GaussianSplatPrimitive/GaussianSplatRenderResources/GeoJsonPrimitive/Geometry3DTileContent/getBinaryAccessor/getClipAndStyleCode/getClippingFunction/GetFeatureInfoFormat/getMeshPrimitives/getMetadataClassProperty/getMetadataProperty/GlobeTranslucency/GlobeTranslucencyFramebuffer/GltfLoaderUtil/GltfSpzLoader/GltfStructuralMetadataLoader/Google2DImageryProvider/GoogleEarthEnterpriseImageryProvider/GoogleEarthEnterpriseMapsProvider/GoogleStreetViewCubeMapPanoramaProvider/GridImageryProvider/GroundPolylinePrimitive/GroupMetadata 各 .js 对应的 Rust 桩） | Scene 全量功能分批实质化 | Track B 后续批次 | ⏳ pending（来源：f4 SEM-blocker #1） |
| 18 | Scene `CameraFlightPath::create_tween` 入口缺失（SEM-major #2） | Camera 飞行路径未实质化 | 修复任务 #35 | ⏳ pending（来源：f4 SEM-major #2） |
| 19 | Scene `Cesium3DTileset::fromUrl`/`loadJson` 入口缺失（SEM-major #3） | tileset 加载入口未实质化 | 修复任务 #35 | ⏳ pending（来源：f4 SEM-major #3） |
| 20 | Scene E-未登记 26 项平台性豁免补登：CameraEventAggregator 9/CreditDisplay 10/DeviceOrientationCameraController 5/GridImageryProvider 2 | DOM/浏览器专属；家族模式已知但未逐文件登记 | 不回填（平台性） | ✅ 补登即处置（来源：f4） |
| 21 | Scene H–Z E-未登记 183 项 + C-未登记 2457 项（Scene.js 110/Label.js 64/VoxelPrimitive.js 61/MetadataClassProperty.js 55/UrlTemplateImageryProvider.js 49/ParticleSystem.js 47 等） | E 为平台性豁免；C 逐行明细见 f5 报告各文件表 | 按批次 / 不回填（E） | ⏳ pending（C）/ ✅ 补登即处置（E）（来源：f5） |
| 22 | Model pipeline 整体空壳 76 个分节（Model/ 66 + Model/Extensions/Gpm 10：AnchorPointDirect/AnchorPointIndirect/CorrelationGroup/GltfGpmLoader/GltfGpmLocal/GltfMeshPrimitiveGpmLoader/MeshPrimitiveGpmLocal/PpeMetadata/PpeTexture/Spdcf；任务索引口径 78+12，以报告正文清点 66+10 为准） | model pipeline 未移植 | Track A9 后续 | ⏳ pending（来源：f6） |
| 23 | DataSources C 档 backlog 771 项 + SEM-2（CZML 11 类几何包全缺）+ SEM-5（KML 高级特性）+ SEM-9/10/11 minor | 逐行明细见 f7 报告各文件表 | 按批次 | ⏳ pending（来源：f7） |
| 24 | Workers+Widget E-未登记 11 项：Workers 9（getModule、5×initWorker、generateGaussianSortWorker/generateSplatTextureWorker、transferTypedArrayTest 外壳）+ Widget 2（screenSpaceEventHandler、showErrorPanel） | 平台性豁免补登 | 不回填（平台性） | ✅ 补登即处置（来源：f8） |
| 25 | Workers+Widget C 档 backlog 101 项（Workers 65：decodeI3S 35 等；Widget 36：属性访问器 17/zoom 族 7/事件族 8/实体跟踪 3/帧配置 1） | 逐行明细见 f8 报告各文件表 | 按批次 | ⏳ pending（来源：f8） |
| 26 | Inspector 三类 VM 整文件桩化（82 行 B(gpu-limited)）：`cesium_inspector_view_model.rs`（4 字段+new）/ `cesium3_d_tiles_inspector_view_model.rs`（4 字段+new）/ `voxel_inspector_view_model.rs`（1 字段+new），无内联 DEVIATION | GPU Scene 依赖阻塞（43 例 ignore 锚点；ignored 处置见 ignored_disposition.md 补登节） | GPU Scene 依赖解除后 | ⏳ pending（来源：f9 SEM-3） |
| 27 | `create_default_imagery_provider_view_models.rs` / `create_default_terrain_provider_view_models.rs` creation_function 恒返回空 provider 列表（注释自述 Track B 但本表原无条目） | 等待 provider 实质化回接（Track B4 已完成离线影像/地形，宜尽快回接） | 修复任务 #36 / 近期批次 | ⏳ pending（来源：f9 SEM-5） |
| 28 | Widgets E-未登记 82 项（16 个 widget 壳文件 DOM 构造/绑定、Animation/Timeline DOM/SVG 绘制、VR lockScreen/unlockScreen 等平台 API） | DOM/Knockout 平台性豁免补登（家族模式已知但未逐文件登记） | 不回填（平台性） | ✅ 补登即处置（来源：f9 SEM-1） |
| 29 | Widgets C-未登记 13 项（Timeline 家族整体缺口：zoomTo/zoomFrom/updateFromClock/addTrack/addHighlightRange/TimelineHighlightRange/TimelineTrack 等）+ C-台账不符 39 项（Viewer 委托属性/flyTo/zoomTo/forceResize/_dataSourceAdded 等，与 deviations.md viewer.rs 条目「引擎侧逻辑完整保留」声明不符，属台账与代码不符，待修台账或补齐实现） | Timeline 为整体性缺口；Viewer 侧声明需修订或实现回填 | 按批次 | ⏳ pending（来源：f9 §3） |
| 30 | Shaders czm_* builtin 143 项缺失（93 函数+41 常量+8 结构体+czm_eyeHeight 半缺失：Rust 有字段但未上传至 336B 缓冲/WGSL 未声明） | shader-strategy.md Batch B/C/D 待办；关键路径保真被其阻塞 | Batch B/C/D | ⏳ pending（来源：f10 ③、Major#2） |
| 31 | Shaders Batch B：naga 批量转译脚本未实现，305/318 嵌入 GLSL 无可运行路径 | shader-strategy.md Batch B | Batch B | ⏳ pending（来源：f10 Major#3） |
| 32 | Shaders 无 WGSL 的非冒烟库：Materials 19/Appearances 其余 12/Model 库 37/Voxels 16/顶层其余 53 | 不在冒烟路径；shader-strategy Batch B/C/D 待办 | Batch B/C/D | ⏳ pending（来源：f10 ②） |
| 33 | exportKml 4 项不可达函数：`createKmz` / `addExternalFilesToZip` / `getRectangleBoundaries` / `createGroundOverlay`（`export_kml.rs`） | kmz 打包无 zip 依赖；GroundOverlay/rectangle 边界依赖未就绪的值模型（行为偏差部分见 deviations.md 修复轮次二补登节） | 引入 zip 依赖 / 值模型补齐后 | ⏳ pending（来源：任务 #41 汇报） |
| 34 | CZ-01 移交项：`PolygonGeometry::createGeometry` 内部 7 个模块函数 + `RectangleGeometry` 15 项 + `GroundPolylineGeometry` 16 项（cesium-core 几何实质化缺口，由 D 档修复轮移交） | 几何实质化批次容量限制，移交后续批次 | 后续几何批次 | ⏳ pending（来源：任务 #43/#44、docs/audit/d_tier_fixes.md） |

---

## 修复轮次二补登对账（任务 #45）

| 任务号 | 归 deferred 条目 | 新增编号 |
| --- | --- | --- |
| #41 exportKml | createKmz/addExternalFilesToZip/getRectangleBoundaries/createGroundOverlay 4 项不可达（合并 1 条） | #33 |
| #43/#44 | CZ-01 移交：PolygonGeometry 7 + RectangleGeometry 15 + GroundPolylineGeometry 16（合并 1 条） | #34 |

> 行为性偏差（GPX 5 / exportKml 8 / EntityCluster 6 / CZML 5 / CorridorOutline 1）见 deviations.md 修复轮次二补登节；本轮未涉及 ignored_disposition.md。

---

## Scene H–Z E 档平台性豁免节（任务 #50 · SC-10，2026-08-25）

> 本节为 `docs/audit/f5_scene_h_z.md`（审查日 2026-08-24）E 档 **183 行**的平台性豁免逐文件补登
> （DOM/canvas/WebGL/GPU 渲染面：帧缓冲/后处理/拾取缓冲/canvas 尺寸/DOM 事件等平台 API，
> headless/winit+wgpu 架构下无对应移植目标）。上表 #21 已作汇总登记，本节为按文件明细补登；
> 处置口径与 #13/#20/#24/#28 一致：**不回填（平台性）**，豁免前置为 DomSurface/GPU 平台层就绪（仅当平台语义可替代时重新评估）。
> 行号依据：f5 报告各文件表内 `| E |` 档行（合计 183 行，已对账）。

| # | JS 文件 | E 行数 | 平台性要点（f5 结论摘要） |
|---|---|---:|---|
| 1 | Scene.js | 18 | canvas/DOM 尺寸、WebGL context 属性、drawingBuffer 族 |
| 2 | Picking.js | 14 | 拾取帧缓冲/canvas 拾取像素读回 |
| 3 | OIT.js | 12 | 半透明帧缓冲/WebGL 扩展探测 |
| 4 | ShadowMap.js | 11 | 阴影贴图帧缓冲/深度纹理 |
| 5 | PostProcessStage.js | 6 | 后处理帧缓冲/shader 编译面 |
| 6 | Material.js | 5 | shader fabric/WebGL uniform 面 |
| 7 | SceneFramebuffer.js | 5 | 场景离屏帧缓冲 |
| 8 | PostProcessStageTextureCache.js | 5 | 后处理纹理缓存（GPU 纹理） |
| 9 | MaterialAppearance.js | 4 | 封闭材质 shader/WebGL 面 |
| 10 | PerInstanceColorAppearance.js | 4 | 逐实例颜色 shader 面 |
| 11 | PolylineColorAppearance.js | 4 | polyline 颜色 shader 面 |
| 12 | PolylineMaterialAppearance.js | 4 | polyline 材质 shader 面 |
| 13 | PointCloudEyeDomeLighting.js | 4 | EDL 帧缓冲/shader 面 |
| 14 | ResourceCacheKey.js | 4 | 缓存键平台细节（引用计数资源面） |
| 15 | Vector3DTileClampedPolylines.js | 4 | 矢量瓦片 GPU 缓冲/绘制面 |
| 16 | Vector3DTilePrimitive.js | 4 | 矢量瓦片 primitive 绘制面 |
| 17 | ImageBasedLighting.js | 3 | IBL 纹理/LUT GPU 面 |
| 18 | ImageryLayer.js | 3 | 影像纹理/WebGL 采样面 |
| 19 | PickDepth.js | 3 | 深度拾取帧缓冲 |
| 20 | PointCloud.js | 3 | 点云绘制/缓冲面 |
| 21 | Primitive.js | 3 | 绘制命令/VAO 面 |
| 22 | SceneTransforms.js | 3 | drawingBuffer 坐标变换（canvas 面） |
| 23 | TranslucentTileClassification.js | 3 | 分类帧缓冲面 |
| 24 | Vector3DTilePolylines.js | 3 | 矢量 polyline GPU 面 |
| 25 | InvertClassification.js | 2 | 反转分类帧缓冲 |
| 26 | MetadataClassProperty.js | 2 | 纹理化元数据 GPU 读取面 |
| 27 | PickDepthFramebuffer.js | 2 | 深度帧缓冲 |
| 28 | PostProcessStageCollection.js | 2 | 后处理集合 GPU 面 |
| 29 | QuadtreeTileProvider.js | 2 | 瓦片绘制/调试绘制面 |
| 30 | ResourceCache.js | 2 | GPU 资源缓存面 |
| 31 | ShadowMapShader.js | 2 | 阴影 shader 生成面 |
| 32 | ShadowVolumeAppearance.js | 2 | 阴影体积 shader 面 |
| 33 | SunPostProcess.js | 2 | 太阳后处理帧缓冲 |
| 34 | TimeDynamicPointCloud.js | 2 | 时变点云缓冲面 |
| 35 | I3SDataProvider.js | 1 | I3S 数据面（平台探测） |
| 36 | Implicit3DTileContent.js | 1 | 隐式瓦片内容面 |
| 37 | JobScheduler.js | 1 | 帧预算调度（渲染循环平台面） |
| 38 | LabelCollection.js | 1 | canvas 文本纹理面 |
| 39 | ModelComponents.js | 1 | 模型 GPU 组件面 |
| 40 | Multiple3DTileContent.js | 1 | 复合内容绘制面 |
| 41 | OrderedGroundPrimitiveCollection.js | 1 | 地面 primitive 排序绘制面 |
| 42 | Particle.js | 1 | 粒子绘制面 |
| 43 | PerformanceDisplay.js | 1 | DOM 性能面板 |
| 44 | PntsParser.js | 1 | pnts Draco/量化解码平台面 |
| 45 | Polyline.js | 1 | polyline 缓冲绘制面 |
| 46 | ScreenSpaceCameraController.js | 1 | DOM 输入事件面 |
| 47 | SpecularEnvironmentCubeMap.js | 1 | 环境立方体贴图 GPU 面 |
| 48 | Splitter.js | 1 | DOM splitter 面 |
| 49 | TerrainFillMesh.js | 1 | 地形填充网格绘制面 |
| 50 | TileBoundingSphere.js | 1 | 裁剪平面 uniform 面 |
| 51 | TileOrientedBoundingBox.js | 1 | 裁剪平面 uniform 面 |
| 52 | Tileset3DTileContent.js | 1 | tileset 内容绘制面 |
| 53 | UrlTemplate3DTilesDataProvider.js | 1 | 模板 URL 平台面 |
| 54 | Vector3DTileContent.js | 1 | 矢量内容绘制面 |
| 55 | Vector3DTileGeometry.js | 1 | 矢量几何 GPU 面 |
| 56 | Vector3DTilePoints.js | 1 | 矢量点 GPU 面 |
| 57 | Vector3DTilePolygons.js | 1 | 矢量多边形 GPU 面 |
| 58 | VectorGltf3DTileContent.js | 1 | 矢量 glTF 内容面 |
| 59 | VoxelBoundsCollection.js | 1 | voxel 体素 GPU 面 |
| 60 | VoxelBoxShape.js | 1 | voxel 形状 shader 面 |
| 61 | VoxelContent.js | 1 | voxel 内容 GPU 面 |
| 62 | VoxelCylinderShape.js | 1 | voxel 形状 shader 面 |
| 63 | VoxelEllipsoidShape.js | 1 | voxel 形状 shader 面 |
| 64 | VoxelShape.js | 1 | voxel 形状 shader 面 |
| 65 | VoxelTraversal.js | 1 | voxel LOD 绘制面 |
| **合计** | **65 文件** | **183** | 与 f5 报告 E 行总数对账一致 |

