# 推迟事项登记表（Deferred Items）

记录明确推迟处理的移植事项：原因、回填计划与责任里程碑。
回填完成后在本表标注完成日期，**不得删除条目**（保留审计痕迹）。

## Core 逆向依赖待回填项

CesiumJS 的 `Core` 层按设计不应依赖 `Scene`/`Renderer`，但源码中存在
少量逆向引用（已在 CesiumJS 代码库审计中确认）。cesium-rs 维持严格分层
（Core → Renderer → Scene → DataSources → Widgets），这些引用按下表方案
处理，回填窗口为 **M3-S4**（cesium-scene 具备对应类型后）：

| # | Core 文件 | 逆向引用目标 | 处理方案 | 回填里程碑 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | `Core/AttributeCompression.js` | `Scene/AttributeType.js` | **常量下沉**：将 `AttributeType` 的常量/语义下沉到 `cesium-core`（Core 私有模块），Scene 侧改为从 Core 引用 | M3-S4 | not_started |
| 2 | `Core/PixelFormat.js` | `Renderer/PixelDatatype.js` | 常量下沉至 `cesium-core`（或与 PixelDatatype 一起在 cesium-renderer 重新归位，Core 只留纯枚举） | M3-S4 | not_started |
| 3 | `Core/TerrainMesh.js` | `Scene/SceneMode.js` | `SceneMode` 枚举下沉至 `cesium-core`（纯值枚举，无 Scene 依赖） | M3-S4 | not_started |
| 4 | `Core/TerrainPicker.js` | `Scene/SceneMode.js` | 同上（共享 #3 的下沉结果） | M3-S4 | not_started |
| 5 | `Core/Cesium3DTilesTerrainProvider.js` | Scene 模块共 9 处引用 | 逐处评估：可下沉的常量下沉；确需 Scene 类型的逻辑延后到该 provider 移植时（依赖 cesium-scene 已就绪）拆分归位 | M3-S4 | not_started |
| 6 | `Core/VectorPipeline.js` / `Core/VectorProvider.js` | Scene/矢量渲染相关 | 随矢量管线整体移植评估归位（可能整体迁移至 cesium-scene） | M3-S4 | not_started |

## 其他推迟事项

| # | 事项 | 原因 | 回填里程碑 | 状态 |
| --- | --- | --- | --- | --- |
| 1 | shader 移植策略定稿 | 等待 M2 GLSL→wgpu 穿刺实验结论（见 shader-strategy.md） | M2 | pending |
| 2 | 需 WebGL 上下文的 Renderer/Scene spec | 等待 wgpu 离屏渲染能力 | M4+ | pending |
| 3 | `FeatureDetection.supportsWebgl2(scene)` 及其 spec（`detects_webgl2_support` 当前 #[ignore]） | 依赖 cesium-scene 的 Context（WebGL2/wgpu 探测），Core 层无法独立验证 | M3-S1 | pending |
| 4 | `getAbsoluteUriSpec` 第 3 断言（相对 `document.location.href` 解析，当前 #[ignore]） | 原生构建无 document；`DocumentLike` 注入路径已由 `document_base_uri_is_respected` 覆盖 | 不回填（设计性偏差） | deferred |
| 5 | `Cartesian4.fromColor` 及 3 条 spec 用例（`core_cartesian4_spec.rs` 当前 #[ignore]） | 依赖 `Core/Color.js` 移植（本批未含） | W2 后续批次 Color 移植后回填 | pending |
| 6 | `Cartesian3Spec` 中 6 条 `fromDegrees/fromRadians` 对照 `ellipsoid.cartographicToCartesian` 的用例（`core_cartesian3_spec.rs` 当前 #[ignore]） | 依赖 `Core/Ellipsoid.js` 移植（W2 椭球批次） | W2 椭球投影批次 | pending |
