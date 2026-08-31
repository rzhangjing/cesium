# cesium-rs 保真度审查总报告（Phase 0–4 闭环）

- 产出：Phase 4 R13（任务 #32），计划最后一阶段
- 约束遵守：本报告为只读审查汇总产出，未修改任何源码/测试/audit 脚本/台账/既有报告/计划文件
- 配套文档：汇总矩阵 [docs/function_fidelity_matrix.md](function_fidelity_matrix.md)、Phase 3 闭环 [docs/audit/phase3_ledger_closure.md](audit/phase3_ledger_closure.md)、任务卡 [docs/audit/fix_task_cards.md](audit/fix_task_cards.md)、D 档修复明细 [docs/audit/d_tier_fixes.md](audit/d_tier_fixes.md)
- 测试基线演进：审查前 2844/326 → Phase 2 时 3065/325 → 一轮修复后 3187/330 → 二轮修复后**以最新 `cargo test` 为准**（QA 复核中，本文不重跑）

---

## 1. 审查方法回顾与各阶段验收结论

### 1.1 方法链

```
Phase 0 清点与匹配（R0）          Phase 1 逐函数五查裁决（R1–R10）
js/rust_function_inventory.csv    f1–f10 报告（11,776 函数逐行落档）
function_fidelity_matrix.csv      每批三数一致验收对账
        │                                  │
        ▼                                  ▼
Phase 2 B 档差分验证（R11）        Phase 3 台账闭环与分级（R12）
Node↔Rust 黄金差分 446 cases      台账修正 L1–L7 + 漂移 Drift-1..7
b_tier_final.csv 1304 行终态      fix_task_cards.md 37 张卡
        │                                  │
        └──────────┬───────────────────────┘
                   ▼
        修复轮次 #33–#45（两轮）→ Phase 4 汇总（R13，本文）
```

五查裁决标准：①参数顺序/可选性；②返回语义（出参 `&mut` vs `_new` 分配变体）；③边界（NaN/±Inf/空/越界）；④错误类型裁剪（DeveloperError↔`#[cfg(debug_assertions)]`，release 不得误删）；⑤精度（领域 f64，仅 GPU 边界降 f32）。

### 1.2 各阶段验收结论

| 阶段 | 验收项 | 结论 |
| --- | --- | --- |
| Phase 0（R0） | 双侧函数清点 + 自动匹配矩阵 | ✅ 通过。JS 11,776 函数 / Rust 侧全 crate 清点；name-based 矩阵建立（误配问题在 Phase 1 各批人工纠正并如实记录，见发现 F-INFRA） |
| Phase 1（R1–R10） | f1–f10 逐函数裁决零遗漏 | ✅ 通过。十批全部「inventory = 矩阵 = 裁决行」三数一致；每函数恰好一档（A 2,091 / B 1,301 / C 7,586 / D 313 / E 485） |
| Phase 2（R11） | B 档 1,304 行全终态 | ✅ 通过。升 A 215 / B-fail 15→D1–D7 / gpu-limited 390 / 无法差分 103 / no-evidence 581，合计 1,304 ✓；差分 446 cases = 416 pass / 27 fail / 3 skipped |
| Phase 3（R12） | 台账闭环 + 漂移清点 + 任务卡 | ✅ 通过。修正 L1–L7；漂移 7 项（minor 4 / info 3）全部修正或加注；37 张任务卡产出；D1–D7 处置归口完成 |
| 修复一轮（#33–#39） | workspace 全量回归 | ✅ 通过（3187 passed / 0 failed / 330 ignored，任务 #38 统一验证） |
| 修复二轮（#40–#45） | 各任务局部验证 | ✅ 各任务回报全绿（#43/#44 见 d_tier_fixes.md：cesium-core/specs/workers 全绿；差分 446 cases → 425 pass / 21 fail / 0 skipped）；workspace 全量待 QA 复核 |

---

## 2. 发现清单汇总

### 2.1 Phase 1 SEM 语义/流程发现（f1–f10，共 55 条：blocker 6 / major 30 / minor 19）

| 批次 | 发现 | 级别 | 双侧位置 | 处置状态 |
| --- | --- | --- | --- | --- |
| f1 | SE-1 AssociativeArray.remove swap_remove 后哈希索引未重建 | blocker | JS AssociativeArray.js:104 ↔ associative_array.rs:48 | 在册（#37 补登 F1 节），待修（无专属任务卡，建议随 CZ 杂项） |
| f1 | SE-2 AttributeCompression.forceUint8 饱和截断 vs mod-256 环绕 | blocker | AttributeCompression.js:84 ↔ attribute_compression.rs:251 | 在册，待修（与 D6 同族） |
| f1 | SE-3 CatmullRomSpline 边界切线/2 点回退/构造校验 | major | CatmullRomSpline.js ↔ catmull_rom_spline.rs | 在册（deferred #7 族），待修 |
| f1 | SE-4 zigZagDeltaDecode 不 u16 回绕；SE-5 Clock.tick 事件载荷 | minor | AttributeCompression.js:366 / Clock.js:258 | 在册；SE-5 随 #34 事件系统部分覆盖 |
| f2 | SEM-1 大批 DEVIATION 未登记（resource/terrain/request_scheduler/fullscreen） | blocker | resource.rs 等 ↔ deviations.md 零条目 | **已闭环**（#37 补登 + F2 节 4 行） |
| f2 | SEM-2 clone_resource 丢弃 retryCallback/request | major | Resource.js:737 ↔ resource.rs:696 | 在册（deferred #23），待修 |
| f2 | SEM-3/SEM-4 地形 availability 延迟重试/瓦片请求缺失 | major | CesiumTerrainProvider.js ↔ cesium_terrain_provider.rs:663/1196 | 在册，待修 → 任务卡 CZ-03 |
| f2 | SEM-5 RequestScheduler 节流未接入 fetch | major | Resource.js:1389 ↔ resource.rs:786 | 在册，待修 → 任务卡 CZ-07 |
| f2 | SEM-6 credits 未建模；SEM-7 遗留 getDerivedResource 拼接；SEM-8 data URI 解码 | minor | resource.rs:254/595/1430 | 在册（deferred #12/#13） |
| f2 | SEM-9 Matrix4 fromCamera/computeViewportTransformation 台账声称但代码无 | minor | deferred #19/#27 ↔ matrix4.rs 无实现 | 在册（⚠ 注记），待回填或台账修正 → CZ-06 |
| f2 | SEM-10 head/options 不返回响应头 | minor | resource.rs:1020 | 在册 |
| f3 | Major#1 UniformState czm_* 状态域缺失 92/117 | major | UniformState.js ↔ uniform_state.rs | 在册（deferred #14–#16），GPU 前置 → RN 族任务卡 |
| f3 | Major#2 ShaderProgram WGSL-only 架构偏差未登记 | major | shader_program.rs（B2.2）↔ shader-strategy.md | 已登记（#37 F3 节） |
| f3 | Major#3 createUniform 族 52 类裁剪；Major#4 Texture 格式降级+mipmap 桩；Major#5 RenderState 5 处静默丢失；Major#6 | major | draw_command.rs/texture.rs/render_state.rs | 已登记（#37），行为残余 → 任务卡 RN-01/RN-03 |
| f3 | Minor×8（矩阵误配、remove_comments 等） | minor | shader_source.rs 等 | 在册（#37 部分）；误配归 F-INFRA |
| f4 | SEM-blocker#1 Scene A–G ~100 文件桩群零登记 | blocker | Scene/[A-G]*.js ↔ cesium-scene 顶层桩 | 已登记（#37 F4 节），修复 → 任务卡 SC-01..SC-06 |
| f4 | SEM-major#1 4 个 gltf loader DEVIATION 未登记 | major | gltf_*_loader.rs（25 处 DEVIATION） | **已修复 #35** + 登记 #37 |
| f4 | SEM-major#2 CameraFlightPath::createTween 缺失 | major | CameraFlightPath.js ↔ camera_flight_path.rs | **已修复 #35** |
| f4 | SEM-major#3 Cesium3DTileset::fromUrl 未直配 | major | Cesium3DTileset.js ↔ cesium3_d_tileset.rs | **已修复 #35** |
| f4 | SEM-minor#1 矩阵 name-based 系统性误配 | minor | function_fidelity_matrix.csv | 报告内逐行纠正 → F-INFRA |
| f5 | SEM-1 148 处 DEVIATION 零 Scene 台账条目 | blocker | cesium-scene/src/** ↔ deviations.md | 已登记（#37 以范围登记 F5 节） |
| f5 | SEM-2 QuadtreePrimitive all-or-nothing 语义降级 | major | QuadtreePrimitive.js ↔ quadtree_primitive.rs（B4-2） | 在册（踩坑点 #3），行为残余不修（偏差在册） |
| f5 | SEM-3 矩阵 tier=global 批量误配 | major | function_fidelity_matrix.csv | 报告内纠正 → F-INFRA |
| f5 | SEM-4 Imagery placeholder/引用计数缺失（failed/placeholder 已立约） | minor | Imagery.js ↔ imagery.rs | 通过立约核查；C 部分 → deferred #21 |
| f6 | 偏差 6 项（DEVIATION 未登记族） | — | GltfPipeline/Model 各文件 | 已登记（#37 F6 节 6 行 + deferred #22） |
| f7 | SEM-1 SampledProperty/TICProperty 时变链路纯桩 | blocker | SampledProperty.js:578 ↔ sampled_property.rs:18 | **已修复 #33**（统一验证 #38） |
| f7 | SEM-2 CZML 11 类几何包全缺 | major | CzmlDataSource.js:2212+ ↔ czml_data_source.rs | **在修 #40** |
| f7 | SEM-3 GPX 仅骨架（40/45 缺） | major | GpxDataSource.js ↔ gpx_data_source.rs | **在修 #41** |
| f7 | SEM-4 exportKml 仅桩（36/37 缺） | major | exportKml.js ↔ export_kml.rs | **在修 #41** |
| f7 | SEM-5 KML 高级特性（Tour/Track/Overlay/NetworkLink/KMZ） | major | KmlDataSource.js ↔ kml_tour*.rs 占位 | 在册，待修 → 任务卡 DS-02 |
| f7 | SEM-6 EntityCluster 34/35 缺 | major | EntityCluster.js ↔ entity_cluster.rs:33 | **在修 #42** |
| f7 | SEM-7 事件系统整体未建模（~110 处） | major | 全模块 definitionChanged ↔ 无 Event 类型 | **已修复 #34**（统一验证 #38） |
| f7 | SEM-8 96 处 DEVIATION 全部未登记 | major | cesium-data-sources/src ↔ deviations.md 零条目 | 已闭环（#37 F7 节） |
| f7 | SEM-9..12（Composite resume/Graphics merge/Property.equals/矩阵误配） | minor | 各对应文件 | 在册 → 任务卡 DS-09/DS-10/DS-11；误配归 F-INFRA |
| f8 | SEM-1 45 处字节桩 DEVIATION 未登记 | blocker | cesium-workers/src ↔ deviations.md 零条目 | 已闭环（#37 F8 节） |
| f8 | SEM-3 create*Geometry 注释失实 + 未回接 core 已实现逻辑 | major | workers create_geometry 族 | **已修复 #36/#39**（dispatch + 6 处回接缺陷） |
| f8 | SEM-4 task_processor dispatch 静默空结果 | major | task_processor.rs | **已修复 #36**（错误信号）+ #39 |
| f8 | SEM-2 矩阵 tier=global 误配（Widget 9 处等） | minor | function_fidelity_matrix.csv | 报告内纠正 → F-INFRA |
| f8 | SEM-5 combine_geometry 硬编码 Triangles/Double；SEM-6 Widget C 档阻塞 spec 镜像 | minor | workers ↔ cesium_widget.rs | 在册 → 任务卡 WK-01/WK-02 |
| f9 | SEM-1 107 行 D/E 未登记（deviations 仅 7 条） | blocker | cesium-widgets/src ↔ deviations.md | 已闭环（#37 F9 节 + widgets 54 ignored 登记） |
| f9 | SEM-2 projection_picker 默认值与 JS 相反且测试固化错误 | major | Viewer.js:644 ↔ viewer.rs:54 | **已修复 #36** |
| f9 | SEM-3 Inspector 两类 VM 整文件桩化三台账未登记 | major | cesium_inspector_view_model.rs 等 | 已登记（#37 + ignored_disposition widgets 节） |
| f9 | SEM-4 subscribe_and_evaluate 纯桩 no-op | major | subscribeAndEvaluate.js ↔ subscribe_and_evaluate.rs | 在册（#37），待修 |
| f9 | SEM-5 createDefault*ProviderViewModels 恒空列表 | minor | create_default_*.rs | 在册，待回接 |
| f10 | Major×3（czm_* WGSL 缺口阻塞关键路径 / WGSL 裁剪未逐条登记 / 自动 uniform 仅冒烟子集） | major | Shaders/Builtin/** ↔ cesium-shaders | 登记（#37 F10 节）；修复 → 任务卡 SH-01 |
| f10 | Minor×3（矩阵/口径类） | minor | — | 在册 |

### 2.2 Phase 2 差分发现 D1–D7（B-fail 15 行归并）

| 发现 | 内容 | 位置 | 处置状态 |
| --- | --- | --- | --- |
| D1 | scaleToGeodeticSurface NaN 输入死循环 | scale_to_geodetic_surface.rs:75/:103 | **已修复 #43**（逐位镜像 JS 退出条件，黄金回归 pass；d_tier_fixes.md） |
| D2 | decode_rgb565 差 1 ULP（双重舍入） | attribute_compression.rs:221 | **已修复 #43**（f64 中间量一次收窄，7 像素 bit-exact） |
| D3 | Color float_to_byte 饱和钳制 vs JS 截断/回绕 | color.rs:447 族 | **已修复 #43**（i32 语义 + ToInt32/Uint8 回绕，c6 黄金全 pass） |
| D4 | PixelFormat createTypedArray/flipY 未移植 | pixel_format.rs | 在册（deviations D1–D7 节），待修 → 任务卡 CZ-08/RN-01 |
| D5 | Cartographic Display `inf` vs JS `Infinity` | cartographic.rs:272 | 在册，待修（独立小卡） |
| D6 | debug 守卫缺失类（octEncode 归一化/范围检查、getHeight 有限性，18 差分 fail） | attribute_compression.rs/vertical_exaggeration.rs | 在册，待修（独立小卡） |
| D7 | HeadingPitchRoll.equalsEpsilon 未移植 | heading_pitch_roll.rs | **已修复 #43**（Option 镜像 undefined，4 个镜像测试） |

差分装置修复后复跑：446 cases = 425 pass / 21 fail / 0 skipped（剩余 21 fail = D6×18 + D5×1 + D4×2，全部归属未修复项）。

### 2.3 Phase 3 分级发现（R12）与基础设施发现

- **blocker：0**（历史 blocker 均已修复或登记闭环）
- **major：7**（M1 D2–D7 漏登→已补登；M2 SEM-9 台账不符；M3 RequestScheduler 接线；M4 地形 availability；M5 GeometryUpdater 族；M6 KML 高级；M7 czm_* WGSL）——详见 phase3_ledger_closure.md §4.2
- **minor：8**（台账修正 L1/L4/L5/L6/L7 与 Drift-1/2/4/5，均已修正）
- **info：6**（L3 口径差异、Drift-3/6/7、E 档豁免补登建议、spec_coverage 快照注记）
- **F-INFRA（跨批基础设施发现，minor）**：function_fidelity_matrix.csv 的 name-based/tier=global 系统性误配（f3/f4/f5/f7/f8/f9 六批均报告并在报告内人工纠正）——矩阵本身未刷新（超出各批权限），Phase 4 结论：**以 f1–f10 报告逐行裁决为准，矩阵仅作清点索引**。

---

## 3. 修复任务卡清单与优先级建议

37 张主题级卡（全表见 [docs/audit/fix_task_cards.md](audit/fix_task_cards.md)：blocker 2 / major 18 / minor 16 / info 1），按依赖序重排如下：

### 依赖序 0（立即可启动）

| 卡号 | 主题 | 优先级 | 状态 |
| --- | --- | --- | --- |
| CZ-01 | Core 几何创建/打包管线（~230） | blocker | **部分完成（#44）**：PolygonGeometry 9 项/Corridor 簇/Stereographic；移交 RectangleGeometry、GroundPolylineGeometry、createGeometry 内部 7 函数 |
| CZ-03 | TileAvailability quadtree 维护（15） | major | 待启动 |
| CZ-04 | Transforms ICRF/EOP（14） | minor | 待启动 |
| CZ-05 | EllipsoidalOccluder horizon culling（14） | major | 待启动 |
| CZ-07 | RequestScheduler fetch 节流接线（SEM-5） | major | 待启动 |
| CZ-02/06/08/09 | 时间间隔高阶/Matrix4 投影族/PixelFormat/零散簇 | minor | 待启动（CZ-08 含 D4） |

### 依赖序 1（依赖 0 层或独立可并行）

| 卡号 | 主题 | 优先级 | 状态 |
| --- | --- | --- | --- |
| DS-06 | GeometryUpdater 族 DS 侧（~180，GPU） | blocker | 待启动（依赖 CZ-01 完成） |
| DS-01 | CZML 几何包（69） | major | **在修 #40** |
| DS-03/DS-04 | GPX（40）/ exportKml（36） | minor | **在修 #41** |
| DS-05 | EntityCluster（35） | minor | **在修 #42** |
| DS-02 | KML 高级特性（55） | major | 待启动 |
| DS-08/DS-10/DS-11 | 位置属性高阶（~70）/Composite 族（40）/其余（~100） | major/major/minor | 待启动 |
| DS-07/DS-09 | SampledProperty 高阶/事件残余 | minor | **基本完成（#33/#34/#38）**，仅残余 |
| SC-05 | GeometryUpdater 族 Scene 侧（~180，GPU） | major | 待启动（依赖 CZ-01） |
| SC-03/SC-07/SC-08/SC-09 | 3D Tiles 残余/桩文件群 B（~1686）/事件残余/零散 | minor/major/minor/minor | 部分被 #9 覆盖（SC-03）；#34/#38 覆盖（SC-08）；余待启动 |
| MD-02/MD-03 | GltfPipeline 高级管线（125）/GPM（58） | major/minor | 部分被 #35 覆盖（MD-02） |
| RN-01..RN-04 | Texture-FB/Buffer-VA/状态机/其余（~334，GPU-required） | major×2/minor×2 | 待启动（wgpu headless 前置） |
| SH-01 | czm_* WGSL 补齐（143，GPU-required） | major | 待启动 |
| WK-01/WK-02 | 业务 worker 残余/Widget engine DOM | minor | **基本完成（#36/#39）**；WK-02 待 DomSurface |

### 依赖序 2（GPU/平台前置就绪后）

| 卡号 | 主题 | 优先级 | 状态 |
| --- | --- | --- | --- |
| MD-01 | Model 运行时管线（~804，GPU） | major | 待启动（依赖 RN-01/02） |
| SC-01/SC-02/SC-04 | PostProcess/ShadowMap/Primitive 绘制残余（GPU） | major×3 | SC-04 部分被 #7 覆盖 |
| SH-02 | PostProcess WGSL（26） | minor | 待启动 |
| WD-01 | Widget DOM 面（52，C+E） | major | 推迟（DomSurface trait + winit 前置） |
| SC-10 | f5 E 档 183 行豁免登记 | info | 台账动作，不占修复资源 |

**已完成卡覆盖汇总**：#33（DS-07 主体）、#34（DS-09 主体）、#35（MD-02 部分 + f4 SEM-major#1/2/3）、#36/#39（WK-01 主体 + f9 SEM-2）、#40（DS-01）、#41（DS-03/04）、#42（DS-05）、#43（D1/D2/D3/D7，差分发现非卡）、#44（CZ-01 部分）、#37/#45（台账，无档位影响）。
**残余移交项**：CZ-01 移交三项（RectangleGeometry 15 C / GroundPolylineGeometry 16 C / createGeometry 内部 7 函数）+ CorridorOutlineGeometry combine 首 corner ±1 偏差（待差分验证后处置）；D4/D5/D6 修复；E 档豁免补登（f2 73 + f5 183）。

---

## 4. 审计装置与重跑命令

全部装置保留于 `cesium-rs/audit/`，幂等可重跑（黄金 JSON 写入仓库外 `%TEMP%\cesium_audit_golden\`，不进仓库）。

### 4.1 Phase 0：清点与匹配

```powershell
Set-Location d:\Rust\cesium\cesium-rs\audit
node extract_js_functions.mjs     # JS 清点 → js_function_inventory.csv（11,776 行）
node extract_rust_functions.mjs   # Rust 清点 → rust_function_inventory.csv + rust_unmatched.csv
node match_functions.mjs          # name-based 匹配 → function_fidelity_matrix.csv
```

### 4.2 Phase 2：B 档差分验证链

```powershell
Set-Location d:\Rust\cesium\cesium-rs\audit
node extract_b_tier.mjs           # 自 f1–f9 报告解析 B 档 → b_tier_inventory.csv（1,304 行）
node classify_b_tier.mjs          # 可差分/gpu-limited/NODIFF 分类 → b_tier_classification.csv
node spec_link_b_tier.mjs         # spec 镜像挂接 → b_tier_spec_links.csv
node diff_golden.mjs              # Node 黄金生成器（446 cases，import CesiumJS 原版）
Set-Location rust_diff_harness; cargo run   # 必须 debug 模式（release 剥离 debug_assert 导致守卫不对齐）
Set-Location ..; node aggregate_final.mjs   # 聚合 → b_tier_final.csv（逐行终态）+ 对账
node extract_passing_tests.mjs    # 通过测试清单（辅助 PATH2-SPEC 挂接）
```

关键规约：`$b` 位级编码（规避 serde_json 17 位十进制 1-ULP 解析缺陷）；容差 exact/ulp2；NaN 语义相等忽略 payload；三态 pass/fail/NODIFF。

### 4.3 回归验证

```powershell
Set-Location d:\Rust\cesium\cesium-rs
cargo test --workspace            # 全量（一轮修复后基线 3187 passed / 0 failed / 330 ignored）
cargo test -p cesium-core; cargo test -p cesium-specs   # D 档/CZ-01 修复局部验证
```

---

## 5. 结论与建议

### 5.1 总体评价

- **可移植域 A 率**：审查时点 18.5% → Phase 2 后 20.4% → 修复后估算 ~23.8%（估算口径，待 QA 复核）。Core 数学/时间/坐标体系最成熟（52–67%），六个已知踩坑点 4 通过 / 1 在册 / 1 已修复，**无在册 blocker 行为缺陷**。
- **流程闭环**：两轮台账补登（#37/#45）后，D 档未登记缺口（审查时点 230 行）基本闭环；三本台账 + MAPPING + spec_coverage 计数与代码现状一致（Phase 3 漂移 7 项全部修正）。
- **验证体系**：spec 镜像（~2,300 A 档证据）+ 黄金差分（446 cases，21 fail 全归属已知未修复项）+ 37 张任务卡构成可持续的保真度回归基线。

### 5.2 最大系统性缺口

1. **Scene 顶层桩文件群**（C 4,655，f4+f5）：最大 C 池，多为叶级 DOM/渲染周边，建议按子目录分批 + GPU 前置解锁。
2. **Model 运行时管线**（C ~804 + GltfPipeline 125）：依赖 Renderer GPU 面（RN-01/02），glTF 完整渲染链路的关键阻塞。
3. **czm_* WGSL**（143 项）：shader 关键路径保真的单点阻塞（SH-01），Batch B naga 转译为既定路线。
4. **GPU 依赖面**（gpu-limited 390 + GPU-required 任务卡 12 张）：wgpu headless 集成环境是 Renderer/Scene/Model/Shaders 四模块验证的共同前置。
5. **no-evidence 581 行**：差分装置未覆盖的 B 池残余（Core D–Z 234 为最大池），需逐函数构造对称输入补强。

### 5.3 下一步路线建议

1. **第 0 层**：完成 CZ-01 移交项（RectangleGeometry/GroundPolylineGeometry/createGeometry 内部 7 函数）→ 解锁 DS-06/SC-05 两个 blocker 级下游；并行 D4/D5/D6 小卡清零剩余 21 个差分 fail。
2. **第 1 层**：#40–#42 收尾后 QA 全量复核并刷新台账（data-sources ignored 计数、D1 状态）；DS-02/DS-08/DS-10 纯逻辑卡与 SH-01（naga 独立验证）并行。
3. **第 2 层**：wgpu headless 环境就绪后按 RN → MD-01 → SC-01/02/04 → SC-05/DS-06 → SH-02/WD-01 顺序推进 GPU 族。
4. **横向**：E 档平台性豁免补登（f2 73 + f5 183 → deferred.md）；刷新 function_fidelity_matrix.csv 匹配质量（F-INFRA），为下一轮自动匹配降低人工纠正成本。
