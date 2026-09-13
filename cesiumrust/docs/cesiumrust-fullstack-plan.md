# CesiumRust 全栈完善计划

> 制定日期: 2026-08-09
> 目标: 将 CesiumRust 从 "domain-heavy + adapter-thin" 推进为可用的 3D 地球渲染引擎

---

## 1. 当前状态评估

### 1.1 已完成项

| 层 | 完成度 | 说明 |
|---|---|---|
| Domain (31 crates) | **85%** | 87,000 行 Rust，6,662 个测试通过，与原版 CesiumJS bit-exact 验证 |
| Ports (2 crates) | **60%** | 8 driven traits + 8 driving traits 定义完整，但缺乏实际调用 |
| Adapters (3 crates) | **15%** | bevy-render 有基础网格转换；network 是 mock；decoders 仅量化网格 |
| Application (1 crate) | **10%** | 基础地球 + Bing 瓦片 + 轨道相机，非 domain 驱动 |
| crates/ (旧 GPUI) | **废弃** | 21 文件，未集成到 Bevy 工作空间，与 README 描述不符 |

### 1.2 核心瓶颈

1. **适配器层薄弱** — domain 逻辑无法到达 GPU
2. **应用层脱离 domain** — cesium-app 用自写逻辑代替 domain crate
3. **无 3D Tiles 渲染链路** — tileset domain 最完善但无渲染适配
4. **网络/IO mock** — HttpTileFetcher 返回 "not implemented"
5. **旧代码污染** — crates/ 与 bevy_demo 与 workspace 不一致

### 1.3 现有应用层代码的问题

`cesium-app/src/` 中所有文件都是独立的、非 domain 驱动实现：
- `dynamic_globe.rs` (616行) — 自己的 LOD 逻辑，而非调用 `cesium-tileset::traversal`
- `tile_loader.rs` (189行) — 自己的瓦片下载，而非调用 `cesium-ports-driven::TileFetcher`
- `bing_tile_loader.rs` (210行) — 硬编码 URL 模板，而非调用 `cesium-provider::ImageryProvider`
- `tile_mesh.rs` (253行) — 自己的网格生成，而非调用 `cesium-geospatial::geometry`

---

## 2. 架构策略

### 2.1 混合模式

```
┌─────────────────────────────────────────────────────────────────┐
│                     Application Layer                           │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────────┐   │
│  │ Tileset  │ │ Terrain  │ │ Entities │ │ Camera/Controls  │   │
│  │ Viewer   │ │ Viewer   │ │ Viewer   │ │                  │   │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────────┬─────────┘   │
│       │             │            │                │              │
├───────┼─────────────┼────────────┼────────────────┼──────────────┤
│       │    PORTS (trait abstraction for IO only) │              │
│  ┌────┴─────────────┴────────────┴────────────────┴─────────┐   │
│  │  TileFetcher | Decoder | ImageryProvider | TerrainProvider │   │
│  └───────────────────────────────────────────────────────────┘   │
├──────────────────────────────────────────────────────────────────┤
│                     Adapters Layer                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐   │
│  │ cesium-network│  │cesium-decoders│  │ cesium-bevy-render   │   │
│  │ (ureq+tokio) │  │ (image+draco) │  │ (ECS direct connect) │   │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘   │
│         │                 │                      │                │
├─────────┼─────────────────┼──────────────────────┼────────────────┤
│         │    DOMAIN (pure Rust, framework-free)  │                │
│  ┌──────┴─────────────────┴──────────────────────┴───────────┐   │
│  │ 31 crates: tileset, terrain, geospatial, scene, camera,   │   │
│  │ imagery, gltf, datasource, material, animation, etc.      │   │
│  └───────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────┘
```

**核心原则**:
- **Domain 类型直接作为 Bevy Component/Resource** — 无中间转换层
- **IO (网络/解码) 通过 ports trait** — mock-able, testable
- **Bevy ECS System 直接消费 domain 类型** — 性能最优

### 2.2 目标应用架构

```rust
// cesium-app/src/main.rs (目标)
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // === Cesium Core ===
        .add_plugins(CesiumCorePlugin)          // EllipsoidConfig, GlobeConfig
        // === Rendering ===
        .add_plugins(CesiumRenderPlugin)        // Globe mesh, lighting, atmosphere
        .add_plugins(CesiumMaterialPlugin)      // Fabric material → WGSL
        // === Feature Modules ===
        .add_plugins(CesiumTilesetPlugin)       // 3D Tiles loading, traversal, rendering
        .add_plugins(CesiumTerrainPlugin)       // Terrain loading, rendering
        .add_plugins(CesiumImageryPlugin)       // Imagery layer management
        .add_plugins(CesiumEntityPlugin)        // Entities (point, line, polygon, model)
        .add_plugins(CesiumDataSourcePlugin)    // CZML, GeoJSON, KML loading
        // === Camera & Interaction ===
        .add_plugins(CesiumCameraPlugin)        // Orbit, fly-to, scene modes
        .add_plugins(CesiumPickingPlugin)       // Screen-space picking
        // === Widgets ===
        .add_plugins(CesiumWidgetPlugin)        // Timeline, animation, geocoder
        .run();
}
```

---

## 3. 实施计划

### Phase 0: 基础设施清理 (预计 2-3 天)

#### P0.1 — 移除旧代码
- [ ] 删除 `crates/` 整个目录 (app, workspace, ui, theme, actions, util, bevy_demo)
- [ ] 从 workspace `Cargo.toml` 移除相关 members
- [ ] 更新 `README.md` — 反映新架构而非 GPUI
- [ ] 删除 `adapters/bevy-render/src/lib.rs` 中 `Globe` 等旧 marker component (属于 cesium-app 层)
- [ ] 验证 `cargo check --workspace` 通过

#### P0.2 — 网络适配器实现
- [ ] `HttpTileFetcher` 使用 `ureq` 实现真实 HTTP 请求
  - 支持请求调度（按 server key 限流，仿 CesiumJS RequestScheduler）
  - 支持取消
  - 支持重试
- [ ] 实现 `CesiumTerrainProvider` (QuantizedMesh 地形)
- [ ] 实现基本的 `UrlTemplateImageryProvider`
- [ ] 单元测试

#### P0.3 — 解码适配器完善
- [ ] `Decoder` trait 的 PNG/JPEG/WEBP 解码实现 (image crate)
- [ ] Gzip 解码 (flate2 crate)
- [ ] Draco 解码 — 先作为 deferred feature，后续用 draco-rs 或 C FFI
- [ ] Pnts/b3dm/i3dm 二进制解析 → 复用 `domain/tileset/content_decoder.rs`

#### P0.4 — 建立 Bevy ECS 组件体系
- [ ] `CesiumGlobe` component — globe entity marker
- [ ] `GlobeConfig` resource — ellipsoid, terrain provider, imagery layers
- [ ] `CesiumTilesetRoot` component — tileset entity entry
- [ ] `CesiumTileNode` component — individual tile entity (path, SSE, state)
- [ ] `TileContent` component — loaded content (mesh handle, texture handle, batch table)
- [ ] `RenderScale` resource — METERS_PER_RENDER_UNIT constant

---

### Phase 1: 3D Tiles 全链路 (预计 5-7 天)

> 目标: 从 URL 加载 tileset.json → 解码 b3dm/glTF → LOD 遍历 → 渲染

#### P1.1 — Tileset 加载管道
- [ ] `TilesetAsset` — Bevy Asset 类型包装 `domain::tileset::Tileset`
- [ ] `TilesetAssetLoader` — 实现 `AssetLoader` trait，异步加载 tileset.json
- [ ] `TilesetLoadingState` resource — 跟踪加载状态
- [ ] 处理 implicit tiling (subtree-based)

#### P1.2 — LOD 遍历系统
- [ ] `TileTraversalSystem` — Bevy System，每帧:
  1. 获取相机位置/方向
  2. 对每个 tileset 调用 `domain::tileset::traversal::select_tiles()`
  3. SSE 公式: `(geometricError * viewportHeight) / (distance * 2 * tan(fovY/2))`
  4. 决定 load/unload/refine
- [ ] `TilePriorityQueue` resource — 按优先级排序 pending tiles
- [ ] `TileReplacementQueue` resource — LRU 淘汰 (复用 `domain::tileset::tile_replacement_queue`)
- [ ] `TileRefinementSystem` — 处理 ADD/REPLACE 优化模式

#### P1.3 — Tile 内容加载与解码
- [ ] `TileContentLoader` — 异步加载 tile content (从 URL 或 embedded)
- [ ] `ContentDecodeSystem` — 解码 b3dm/glTF/i3dm/pnts/cmpt:
  - 检测 magic header → 调用 `domain::tileset::content_decoder`
  - 提取 feature table / batch table
  - 解码 glTF → mesh + material
  - 上传到 GPU (geometry → Bevy Mesh, texture → Bevy Image)
- [ ] 处理 `TILESET_CONTENT_LOADING` / `TILESET_CONTENT_READY` / `TILESET_CONTENT_FAILED` 状态

#### P1.4 — 3D Tiles 渲染系统
- [ ] `TilesetRenderSystem` — Bevy System，遍历 tileset 中 visible + ready 的 tiles:
  - 对每个 tile 创建 `PbrBundle` 或自定义 material
  - 设置 transform (tile 的 model matrix)
  - 处理 RTC (Relative To Center) 坐标变换
- [ ] `BatchTableRenderSystem` — 从 batch table 提取 feature 属性:
  - 存储为 instance-level uniform buffer
  - 支持 `_BATCHID` 用于 picking
- [ ] `TileStyleApplySystem` — 应用 `Cesium3DTileStyle`:
  - 实现 `conditions` 表达式求值 (show/color/pointSize)
  - 更新 per-feature shader uniforms
- [ ] `InstancedTileRendering` — 同质 tile 批量渲染 (instanced draw)

#### P1.5 — 3D Tiles Picking
- [ ] `TilesetPickingSystem` — 屏幕坐标 → tile + feature:
  1. Ray casting from camera through screen point
  2. Intersect with tile bounding volumes (OBB/AABB/Sphere)
  3. GPU picking for exact triangle/feature within tile
  4. Return `PickResult::TileFeature { tileset_id, feature_id }`
- [ ] 利用 `domain::scene::picking` + `batch_table` 返回 metadata

#### P1.6 — Debug 工具
- [ ] `TilesetDebugPlugin` — 可切换的调试视觉:
  - [ ] Bounding volume wireframe (color by SSE)
  - [ ] Tile 坐标文本 overlay
  - [ ] SSE 热力图
  - [ ] Tile 内容边界标记
- [ ] `TilesetInspector` resource — 实时统计 (loaded tiles, pending, cache hits)

---

### Phase 2: 地形系统 (预计 3-4 天)

#### P2.1 — 地形 Provider 适配
- [ ] `TerrainProviderAdapter` — 封装 `cesium-ports-driven::TerrainProvider`
- [ ] `CesiumTerrainProvider` — 从 Cesium Ion / 自定义 URL 加载 QuantizedMesh
- [ ] `EllipsoidTerrainProvider` — 纯椭球地形 (fallback)
- [ ] `ArcGISTerrainProvider` — ArcGIS 高程服务

#### P2.2 — 地形瓦片渲染
- [ ] `TerrainTileSystem` — 加载+解码 QuantizedMesh → `TerrainMesh`
- [ ] `TerrainRenderingSystem`:
  - 地形网格 → Bevy Mesh
  - 应用 imagery 纹理 (从 ImageryLayer)
  - 处理 skirt (避免瓦片间裂缝)
  - 地形夸张 (vertical exaggeration)
- [ ] `TerrainLODSystem` — 基于相机距离的四叉树 LOD
- [ ] 地球表面法线光照 (基于 local tangent plane)

---

### Phase 3: 影像图层 (预计 2-3 天)

#### P3.1 — 影像 Provider
- [ ] `ImageryProviderRegistry` resource — 管理多个 imagery provider
- [ ] `UrlTemplateImageryProvider` — WMTS/XYZ/TMS 瓦片
- [ ] `BingMapsImageryProvider` — Bing Maps
- [ ] `IonImageryProvider` — Cesium Ion
- [ ] `SingleTileImageryProvider` — 单张大图

#### P3.2 — 影像图层管理
- [ ] `ImageryLayerManager` resource — 多层影像管理:
  - 添加/移除/排序图层
  - 透明度控制
  - visible 切换
  - split 方向 (左右对比)
- [ ] `ImageryBlendingSystem` — 多图层混合:
  - 按 z-order 合成
  - alpha blending
  - 复用 `domain::imagery::blending`

#### P3.3 — 影像瓦片渲染
- [ ] `ImageryTileSystem` — 加载+解码影像瓦片
- [ ] 瓦片纹理 → Bevy Image → 绑定到 terrain tile material
- [ ] 纹理缓存 (`TextureCache` with LRU)

---

### Phase 4: 实体系统 + 数据源 (预计 4-5 天)

#### P4.1 — 实体 Bevy 组件
- [ ] `CesiumEntity` component — entity 通用属性 (position, orientation, show, availability)
- [ ] `EntityGraphics` — graphics descriptors:
  - [ ] `PointGraphics` (pixelSize, color, outlineColor, outlineWidth)
  - [ ] `PolylineGraphics` (width, material, clampToGround)
  - [ ] `PolygonGraphics` (height, extrudedHeight, material, outline)
  - [ ] `BillboardGraphics` (image, scale, rotation, alignedAxis)
  - [ ] `LabelGraphics` (text, font, fillColor, outlineColor, style)
  - [ ] `ModelGraphics` (uri, scale, minimumPixelSize, maximumScale)
  - [ ] `BoxGraphics`, `CylinderGraphics`, `EllipsoidGraphics`, `WallGraphics`, etc.

#### P4.2 — Entity Visualizer
- [ ] `EntityVisualizerSystem` — 将 entity graphics 描述转为 Bevy mesh/material:
  - Point → small quad/circle mesh (screen-space size in vertex shader)
  - Polyline → line strip mesh (with width extrusion)
  - Polygon → triangulated mesh (earcut)
  - Billboard → quad mesh (always facing camera)
  - Label → glyph mesh or texture (deferred to Phase 7)
  - Model → glTF asset load

#### P4.3 — 数据源加载器
- [ ] `DataSourceLoader` — 通用异步数据源加载:
  - [ ] **CZML** — 解析 `domain::datasource::czml` → 创建 entities
    - 处理 packet types (document, entity, delete)
    - 时间动态属性 (SampledProperty, TimeIntervalCollection)
    - 插值系统集成 (Linear, Hermite, Lagrange)
  - [ ] **GeoJSON** — 解析 `domain::datasource::geojson` → entities
    - Point → PointPrimitive
    - LineString → Polyline
    - Polygon → Polygon (with holes via earcut)
    - Multi* geometries → multiple entities
  - [ ] **KML** — 解析 `domain::kml::parser` → entities
    - Placemark, LineString, Polygon, MultiGeometry
    - Style 继承
    - NetworkLink (pointer to external KML)
  - [ ] **GPX** — 解析 `domain::gpx::parser` → polyline + waypoints

#### P4.4 — 时间动态系统
- [ ] `AnimationClock` resource — 全局时钟 (start/stop/pause/seek/multiplier)
- [ ] `EntityTimeUpdateSystem` — 每帧更新 entity 属性:
  - 根据 clock.currentTime 插值 entity.position/orientation/color
  - 调用 `domain::datasource::property::Property::getValue(time)`
  - 处理 availability (显示/隐藏)
- [ ] 与 `cesium-widgets::animation` timeline 集成

---

### Phase 5: 相机与交互 (预计 3-4 天)

#### P5.1 — 相机系统
- [ ] `CesiumCamera` component — 封装 `domain::camera::Camera`:
  - 位置 (Cartographic + height)
  - 方向 (heading/pitch/roll)
  - frustum (PerspectiveFrustum / OrthographicFrustum)
  - 2D/3D/COLUMBUS_VIEW 模式
- [ ] `CameraUpdateSystem` — 每帧更新:
  - 计算 view matrix, projection matrix
  - 更新 frustum planes (用于剔除)
  - 处理 scene mode 切换时 morphing

#### P5.2 — 相机控制
- [ ] `ScreenSpaceCameraController` — 鼠标/触摸交互:
  - 左键拖动 → 旋转 (orbit)
  - 右键拖动 → 缩放 (zoom)
  - 中键拖动 → 平移 (pan)
  - 滚轮 → 缩放
  - 碰撞检测 (避免穿过地球)
  - 惯性 (momentum)
- [ ] `CameraFlightSystem` — 飞行路径:
  - `flyTo(destination, options)` — 自动计算路径+动画
  - 支持 heading/pitch/roll 终点
  - 基于 `domain::interaction::flight`
- [ ] `KeyboardNavigationSystem` — 键盘 WASD 移动

#### P5.3 — 场景模式
- [ ] `SceneMode`资源 — `SceneMode3D | SceneMode2D | SceneModeColumbusView`
- [ ] `SceneModeTransitionSystem` — 模式切换动画 (morphing)
- [ ] `MapMode2D` — 2D 地图投影 (WebMercator)

---

### Phase 6: 材质与着色 (预计 3-4 天)

#### P6.1 — Fabric 材质 WGSL 端口
- [ ] 将现有 `fabric_material.wgsl` 扩展到覆盖所有 25 种内置材质:
  - [x] Color (已完成)
  - [x] Image (已完成)
  - [x] Checkerboard (已完成)
  - [ ] Stripe
  - [ ] Grid
  - [ ] Dot
  - [ ] Fade
  - [ ] PolylineArrow
  - [ ] PolylineDash
  - [ ] PolylineGlow
  - [ ] PolylineOutline
  - [ ] ElevationContour
  - [ ] ElevationRamp
  - [ ] AspectRamp
  - [ ] SlopeRamp
  - [ ] NormalMap
  - [ ] BumpMap
  - [ ] Water
  - [ ] RimLighting
  - [ ] ElevationBand
  - [ ] WaterMask
- [ ] `FabricMaterialBuilderSystem` — 从 domain Material 构建 Bevy material:
  - 翻译 GLSL uniforms → WGSL struct
  - 绑定纹理 (image, normal map, cube map)
  - 设置 translucency 模式

#### P6.2 — 自定义着色器支持
- [ ] `CustomShader` component — 用户自定义 GLSL/WGSL
- [ ] `ShaderIntegrationSystem` — 集成到 Fabric 管道
- [ ] Uniform/Varying 自动注入

#### P6.3 — 大气与光照
- [ ] `AtmosphereRenderingSystem`:
  - Rayleigh/Mie 散射 (复用 `domain::atmosphere::scattering`)
  - 大气辉光 (horizon glow)
  - 天空盒 (sky atmosphere)
- [ ] `CelestialSystem` — 太阳/月亮位置:
  - 基于时间计算方向
  - `domain::atmosphere::celestial`
- [ ] `ImageBasedLightingSystem` — IBL 环境贴图
- [ ] 星空背景 (复用现有 `starfield.rs`)

---

### Phase 7: 交互与 Widgets (预计 2-3 天)

#### P7.1 — 拾取系统
- [ ] `PickingPlugin` — 屏幕拾取管道:
  - [ ] GPU picking (render-to-texture with entity ID)
  - [ ] Ray casting fallback (for non-GPU objects)
- [ ] `PickResultHandler` resource — 缓存拾取结果
- [ ] `SelectionIndicatorSystem` — 高亮选中 entity

#### P7.2 — Widgets
- [ ] `TimelineWidget` — 时间轴 (复用 `domain::widgets::timeline`)
- [ ] `AnimationWidget` — 播放/暂停/速度控制
- [ ] `GeocoderWidget` — 地名搜索
- [ ] `BaseLayerPickerWidget` — 底图切换
- [ ] `SceneModePickerWidget` — 2D/3D 切换按钮
- [ ] `NavigationHelpWidget` — 鼠标/键盘操作提示
- [ ] `InfoBox` — 选中 entity 信息显示
- [ ] `SelectionIndicator` — 选中框

#### P7.3 — 性能监控
- [ ] `PerformanceDisplay` — FPS 显示
- [ ] `TileLoadStatsDisplay` — 瓦片加载统计
- [ ] `MemoryStatsDisplay` — 缓存状态

---

### Phase 8: 高级特性 (预计 5-7 天)

#### P8.1 — 粒子系统
- [ ] `ParticleSystemPlugin` — 从 `domain::effects::particles`:
  - Emitter shapes (box, circle, cone, sphere)
  - Particle forces (gravity, wind)
  - Burst/spawn rates
  - GPU particle update (compute shader)

#### P8.2 — 后处理
- [ ] `PostProcessPlugin`:
  - [ ] Bloom
  - [ ] Ambient Occlusion
  - [ ] Fog
  - [ ] ToneMapping (Reinhard, ACES)
  - [ ] FXAA
  - [ ] Color correction (brightness/contrast/saturation)

#### P8.3 — 阴影
- [ ] `ShadowMapSystem`:
  - Cascaded shadow maps
  - PCF filtering
  - Terrain + entity shadow casting

#### P8.4 — Voxel 支持
- [ ] `VoxelPrimitivePlugin`:
  - Box/Cylinder/Ellipsoid shapes
  - Voxel traversal
  - Voxel rendering (ray marching or mesh generation)

#### P8.5 — Vector Tiles
- [ ] `VectorTilePlugin`:
  - MVT 解码
  - Point/LineString/Polygon → entity 转换
  - Clamped-to-ground 渲染

#### P8.6 — OIT (Order Independent Transparency)
- [ ] `OITPlugin`:
  - Weighted Blended OIT
  - Multi-pass rendering

---

### Phase 9: 测试与文档 (持续)

#### P9.1 — Domain 测试延续
- [ ] Phase 2 剩余几何测试 (~516 用例)
- [ ] Scene 可移植逻辑补充
- [ ] 渲染无关的 Scene 逻辑全量覆盖

#### P9.2 — 集成测试
- [ ] Bevy 集成测试框架搭建
- [ ] Tileset 加载 → 渲染 端到端测试
- [ ] Terrain + Imagery 端到端测试
- [ ] Entity + DataSource 端到端测试
- [ ] Camera 操作集成测试

#### P9.3 — 文档
- [ ] 架构文档 (Architecture Decision Records)
- [ ] API 文档 (每个 Bevy plugin 的使用方法)
- [ ] 示例代码 (examples/ 目录)
- [ ] 贡献指南

---

## 4. 依赖关系图

```
P0 (基础设施)
 │
 ├─→ P1 (3D Tiles) ──────────────────────────┐
 │      │                                       │
 │      ├─→ P2 (Terrain) ─────────────────────┤
 │      │      │                                │
 │      │      ├─→ P3 (Imagery)                │
 │      │      │                                │
 │      │      └─→ P6.3 (Atmosphere/Lighting)   │
 │      │                                       │
 │      └─→ P4 (Entities + DataSources) ───────┤
 │             │                                │
 │             ├─→ P4.4 (Time Dynamic)         │
 │             │                                │
 │             └─→ P6.1 (Fabric Materials) ────┤
 │                                              │
 ├─→ P5 (Camera + Interaction) ────────────────┤
 │      │                                       │
 │      ├─→ P7.1 (Picking)                     │
 │      │                                       │
 │      └─→ P7.2 (Widgets)                     │
 │                                              │
 └─→ P8 (Advanced Features) ───────────────────┘
        │
        └─→ P9 (Tests + Docs, continuous)
```

---

## 5. 关键技术决策

| 决策点 | 选择 | 理由 |
|---|---|---|
| 渲染架构 | Bevy ECS 直连 domain 类型 | 性能最优，减少中间转换；IO 仍通过 trait 隔离 |
| 材质系统 | Domain GLSL assembly + Adapter WGSL port | 保持 domain 纯 Rust，适配器做语言映射 |
| 网络库 | `ureq` (同步) + `tokio::spawn_blocking` | 简单可靠，避免重量级 reqwest |
| Draco 解码 | deferred feature (先 skipping) | Rust Draco crate 不成熟，后续可选 C FFI |
| 坐标精度 | f64 domain + METERS_PER_RENDER_UNIT 缩放 | 避免 f32 地球规模精度损失 |
| 旧代码 | 完全移除 crates/ | 清理混淆，统一工作空间 |
| 测试策略 | Domain 用纯 Rust 测试；渲染层用 Bevy 集成测试 | 保持 domain 测试独立快速 |

---

## 6. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| Draco 解码不可用 | 部分 3D Tiles 无法加载 | 先 skip，后续 C FFI 或 WASM |
| Bevy API 不稳定 (0.15) | 升级时需大量改动 | 锁定版本，后续逐步跟进 |
| WGSL → GLSL 翻译不完整 | 自定义 Fabric 材质失败 | 先覆盖内置 25 种，自定义 deferred |
| glTF 2.0 完整支持 | 复杂模型渲染异常 | 使用 `bevy_gltf` 插件，逐步增强 |
| libudev 等系统依赖 | Linux 编译可能失败 | 文档化依赖，提供 Docker 开发环境 |

---

## 7. 成功标准

### Phase 1 完成标准
- [ ] 从 Cesium Ion URL 加载 tileset.json
- [ ] 正确的 LOD 遍历 (与 CesiumJS 行为一致)
- [ ] b3dm + glTF tile 渲染可见
- [ ] Batch table 属性可查询
- [ ] Cesium3DTileStyle 着色生效
- [ ] 屏幕拾取返回 tile feature metadata

### Phase 2-3 完成标准
- [ ] 地形渲染 (QuantizedMesh)
- [ ] 多影像图层叠合
- [ ] 与 CesiumJS 视觉效果一致

### Phase 4-5 完成标准
- [ ] CzML/GeoJSON/KML 加载
- [ ] 实体渲染 (点/线/面/模型)
- [ ] 时间动画
- [ ] 相机飞行

### 整体完成标准
- [ ] `cesium-app` 可加载 CesiumJS 示例场景
- [ ] 所有 domain 测试通过 (100% Phase 2-4 A 类用例)
- [ ] 渲染层集成测试通过
- [ ] 架构文档完整
