# 函数保真度汇总矩阵（Function Fidelity Matrix · 汇总版）

- 产出：Phase 4 R13（任务 #32）
- 逐函数全量裁决表（合计约 11.8k 行）不在本文档复制，各节以链接 + 计数承载：`docs/audit/f1_core_a_c.md` … `docs/audit/f10_shaders.md`
- 审查口径：每 JS 函数五查（①参数顺序/可选性 ②返回语义 ③边界 ④错误类型裁剪 DeveloperError↔debug_assertions ⑤精度 f64 领域/f32 GPU 边界）后落档，每函数恰好一档，各批均有三数一致验收对账（inventory = 矩阵 = 报告裁决行）

## 0. 口径定义

| 口径 | 时点 | 说明 |
| --- | --- | --- |
| 审查时点 | 2026-08-24（Phase 1 f1–f10） | 各报告档位计数原值 |
| Phase 2 后 | B 档差分验证清零后（R11） | 审查时点 + 升 A 215（115 spec 挂接 + 94 差分 pass + 6 间接单测挂接）；测试基线 3065/325 |
| 修复后估算 | 修复任务 #33–#45 两轮后 | **估算口径**：按各任务回报覆盖函数簇累加（#43/#44 有 `docs/audit/d_tier_fixes.md` 明细锚点）；workspace 全量测试数以最新 `cargo test` 为准（本文不重跑） |

档位定义：**A** 已实现且验证（spec 镜像/差分/集成证据）；**A(gpu-limited)/B(gpu-limited)** GPU 资源路径依赖 wgpu 环境（计入 A/B）；**B** 已实现待验；**C** 缺失；**D** 偏差（拆分已登记/未登记）；**E** 平台性不移植（拆分已登记/未登记）。

## 1. 全局统计摘要

### 1.1 总函数数与各档计数（审查时点，f1–f10 合计）

| 档位 | 计数 | 占比 | 备注 |
| --- | ---: | ---: | --- |
| A（含 A(gpu-limited) 15） | 2,091 | 17.8% | gpu-limited：f4 14 + f6 1 |
| B（含 B(gpu-limited) 106） | 1,301 | 11.0% | gpu-limited：f4 8 + f5 15 + f6 1 + f9 82；Phase 2 装置解析口径 1,304（f8 清点 23 vs 解析 26 差 3，见 `audit/b_tier_summary.md`） |
| C | 7,586 | 64.4% | 缺失主体：Scene 桩文件群（4,655）、czm_* WGSL（143）、DataSources（771）、Renderer（334）、Model/GltfPipeline（987） |
| D | 313 | 2.7% | 已登记 83 / 未登记 230（未登记缺口经 #37/#45 两轮台账补登闭环） |
| E | 485 | 4.1% | 已登记 42 / 未登记 443（DOM/WebGL/Worker 外壳） |
| **合计** | **11,776** | 100% | JS 侧清点全量（engine Core/Renderer/Scene/DataSources/Workers/Widget/Shaders + widgets） |

### 1.2 按模块 A 档率（三口径）

| 模块 | 函数数 | 审查时点 A | A 率 | Phase 2 升 A | Phase 2 后 A | A 率 | 修复后估算 A | A 率（估算） |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Core A–C（f1） | 562 | 294 | 52.3% | +55 | 349 | 62.1% | ~370 | ~65.8% |
| Core D–Z（f2） | 1,690 | 794 | 47.0% | +90 | 884 | 52.3% | ~910 | ~53.8% |
| Renderer（f3） | 704 | 184 | 26.1% | +1 | 185 | 26.3% | ~185 | ~26.3% |
| Scene A–G（f4） | 2,453 | 162 | 6.6% | +2 | 164 | 6.7% | ~180 | ~7.3% |
| Scene H–Z（f5） | 2,892 | 79 | 2.7% | +3 | 82 | 2.8% | ~82 | ~2.8% |
| Scene Model+GltfPipeline（f6） | 1,051 | 6 | 0.6% | +6 | 12 | 1.1% | ~16 | ~1.5% |
| DataSources（f7） | 1,293 | 148 | 11.4% | +58 | 206 | 15.9% | ~485 | ~37.5% |
| Workers+Widget（f8） | 207 | 17 | 8.2% | 0 | 17 | 8.2% | ~47 | ~22.7% |
| widgets（f9） | 444 | 83 | 18.7% | 0 | 83 | 18.7% | ~93 | ~20.9% |
| Shaders（f10） | 480 | 324 | 67.5% | 0 | 324 | 67.5% | ~324 | 67.5% |
| **合计** | **11,776** | **2,091** | **17.8%** | **+215** | **2,306** | **19.6%** | **~2,690** | **~22.8%** |

> 注 1：修复后列为**估算口径**——按 #33（时变属性 ~30）、#34（事件建模 ~110）、#35（Scene API ~19）、#36/#39（Widgets-Workers ~40）、#40（CZML 69）、#41（GPX 40 + exportKml 36）、#42（EntityCluster 35）、#43（D1/D2/D3/D7 ~8，见 d_tier_fixes.md）、#44（CZ-01 部分 ~30，见 d_tier_fixes.md）、#45（台账，不影响档位）回报覆盖簇累加；各簇中 B 档实质化部分未计入 A。**以最新 `cargo test` 与各修复任务验证记录为准**（一轮修复后全量 3187 passed / 330 ignored；二轮修复后待 QA 复核）。
> 注 2：可移植口径（剔除 E 档 485）A 率：审查时点 18.5% → Phase 2 后 20.4% → 修复后估算 ~23.8%。
> 注 3：cesium-data-sources 源码在 #40–#42 修复期间变动，本表 f7 相关数字按报告时点 + 任务回报估算，不按源码现状清点。

---

## 2. 分模块矩阵

### 2.1 Core A–C（f1，562 函数 / 56 个 JS 文件）

全量逐函数表：[docs/audit/f1_core_a_c.md](audit/f1_core_a_c.md)

| 档位 | 计数 | 拆分 |
| --- | ---: | --- |
| A | 294 | spec 镜像/差分证据 |
| B | 126 | Phase 2 终态：升 A 55 / B-fail 11（并入 D 发现）/ no-evidence 50 / no-diff 10 |
| C | 99 | 几何 pack/unpack/createGeometry 簇（CZ-01 主题） |
| D | 39 | 已登记 18 / 未登记 21（#37 已补登） |
| E | 4 | DOM 设施（buildModuleUrl 族） |

- **Phase 2 终态变化**：B 126 → 升 A 55；B-fail 11 归并 D 档发现（attribute_compression/color 族）。
- **修复轮次后**：#43 修复 D1（scaleToGeodeticSurface）/D2（decode_rgb565）/D3（Color float_to_byte 族）→ 对应 D/B-fail 行转 A；#44 CZ-01 首批（PolygonGeometry 9 项、Corridor/CorridorOutline pack 簇、Stereographic 整体，~25 C→A，含 15 个镜像新测试）。移交：RectangleGeometry、GroundPolylineGeometry、createGeometry 内部 7 函数。

### 2.2 Core D–Z（f2，1,690 函数 / 165 个 JS 文件）

全量逐函数表：[docs/audit/f2_core_d_z.md](audit/f2_core_d_z.md)

| 档位 | 计数 | 拆分 |
| --- | ---: | --- |
| A | 794 | spec 镜像（含 JulianDate 闰秒族、WebMercatorProjection 极点边界） |
| B | 332 | Phase 2 终态：升 A 90 / B-fail 4 / no-evidence 234（最大待验池）/ no-diff 4 |
| C | 444 | C-未登记 442 + C-台账不符 2（Matrix4 SEM-9） |
| D | 47 | 已登记 29 / 未登记 18（全 Resource.js，#37 已补登） |
| E | 73 | 未登记（ScreenSpaceEventHandler 29 / TaskProcessor 12 / Resource 浏览器族 11 等，豁免补登移交） |

- **Phase 2 终态变化**：B 332 → 升 A 90；闰秒表 B-256..B-259 经 PATH2-SPEC 升 A（踩坑点 #6 通过）。
- **修复轮次后**：#43 D7（HeadingPitchRoll.equalsEpsilon C→A）；#35 关联的 4 个 gltf loader 偏差（D→A）；C 档主体归任务卡 CZ-01..CZ-09（CZ-01 部分由 #44 覆盖）。

### 2.3 Renderer（f3，704 函数 / 46 个 JS 文件）

全量逐函数表：[docs/audit/f3_renderer.md](audit/f3_renderer.md)

| 档位 | 计数 | 拆分 |
| --- | ---: | --- |
| A | 184 | 帧编排/命令收集/pipeline cache/自动 uniform ring（ViewportQuad 冒烟闭环） |
| B | 108 | Phase 2 终态：升 A 1 / no-evidence 18 / gpu-limited 89 |
| C | 334 | UniformState 92 + createUniform 族 52 + Texture/FB/Atlas/VA 族（任务卡 RN-01..RN-04） |
| D | 14 | 已登记 4 / 未登记 10（#37 已补登；WGSL-only 架构偏差等 Major 6 条） |
| E | 64 | WebGL/GLSL 专属 |

- **Phase 2 终态变化**：B 108 中 89 判 gpu-limited 挂 Track B（wgpu headless 前置）；可差分池 18 保留 no-evidence。
- **修复轮次后**：无修复任务覆盖（GPU-required 全族待 Renderer 前置，任务卡 RN-01..RN-04）。

### 2.4 Scene A–G（f4，2,453 函数 / 155 个 JS 顶层文件）

全量逐函数表：[docs/audit/f4_scene_a_g.md](audit/f4_scene_a_g.md)

| 档位 | 计数 | 拆分 |
| --- | ---: | --- |
| A | 162 | 含 A(gpu-limited) 14；camera/quadtree/3D Tiles/表达式/glTF 批次证据 |
| B | 57 | 含 B(gpu-limited) 8；Phase 2 终态：升 A 2 / no-evidence 23 / no-diff 5 / gpu 27 |
| C | 2,198 | Scene 顶层桩文件群主体（任务卡 SC-01..SC-06） |
| D | 10 | 已登记（deferred 决策 #16/#17）；4 个 gltf loader DEVIATION 未登记由 #35/#37 处置 |
| E | 26 | DOM/浏览器事件 |

- **Phase 2 终态变化**：B 池小（57），升 A 2，其余按终态分类归档。
- **修复轮次后**：#35 补齐 CameraFlightPath::createTween、Cesium3DTileset::fromUrl（~15 C→A）并修复 4 个 gltf loader 偏差（D→A）；C 档主体（~2,180）归任务卡 SC-01..SC-06。

### 2.5 Scene H–Z（f5，2,892 函数 / 182 个 JS 顶层文件）

全量逐函数表：[docs/audit/f5_scene_h_z.md](audit/f5_scene_h_z.md)

| 档位 | 计数 | 拆分 |
| --- | ---: | --- |
| A | 79 | spec/内联/批测试全绿 |
| B | 173 | 158 + B(gpu-limited) 15；Phase 2 终态：升 A 3 / no-evidence 102 / no-diff 1 / gpu 67 |
| C | 2,457 | 全部 C-未登记（#37 以 148 处范围登记 + 任务卡 SC-07..SC-09 承载） |
| D | 0 | SEM-1 的 148 处 DEVIATION 以范围登记入 deviations（F5 节） |
| E | 183 | 全部未登记（豁免补登移交，任务卡 SC-10） |

- **Phase 2 终态变化**：B 173 → 升 A 3；67 判 gpu-limited。
- **修复轮次后**：无直接覆盖；踩坑点核查（UV v 翻转/LOD/failed-placeholder）通过，all-or-nothing 降级在册。

### 2.6 Scene Model + GltfPipeline（f6，1,051 函数）

全量逐函数表：[docs/audit/f6_scene_model.md](audit/f6_scene_model.md)（纯表格报告，无汇总头；计数经逐行解析）

| 档位 | 计数 | 拆分 |
| --- | ---: | --- |
| A | 6 | 5 + A(gpu-limited) 1 |
| B | 56 | 55 + B(gpu-limited) 1；Phase 2 终态：升 A 6 / no-evidence 12 / gpu 38 |
| C | 987 | GltfPipeline 125 + Model 804 + Gpm 58（任务卡 MD-01..MD-03；计划口径 922，双口径见 phase3_ledger_closure.md Drift-7） |
| D | 2 | 未登记（#37 补登节 F6 含其余偏差 6 行登记） |
| E | 0 | — |

- **Phase 2 终态变化**：B 56 → 升 A 6，38 判 gpu-limited（skin/morph/纹理运行时）。
- **修复轮次后**：#35 修复 4 个 gltf loader 偏差；C 档主体归任务卡 MD-01..MD-03（GPU 依赖 RN-01/RN-02 前置）。

### 2.7 DataSources（f7，1,293 函数 / 106 个 JS 文件）

全量逐函数表：[docs/audit/f7_data_sources.md](audit/f7_data_sources.md)（按报告时点；#40–#42 修复期间不按源码现状清点）

| 档位 | 计数 | 拆分 |
| --- | ---: | --- |
| A | 148 | czml 43 / kml 86 / geojson 62 / display 16 / data_sources_specs 65（证据重叠计入口径见报告） |
| B | 278 | Phase 2 终态：升 A 58 / B-fail 17 / no-evidence 116 / no-diff 17 / gpu 87（visualizer 族） |
| C | 771 | 主题分布：CZML 69 / KML 高级 55 / GPX 40 / exportKml 36 / EntityCluster 35 / GeometryUpdater ~180 / SampledProperty 26 / 位置属性高阶 ~70 / 事件 ~110 / Composite ~40 / 其余 ~100 |
| D | 96 | 全部未登记（#37 已补登 5 行 + SEM-7 归 #34 修复记录） |
| E | 0 | 平台性函数均以桩形式存在归 D |

- **Phase 2 终态变化**：B 278 → 升 A 58；B-fail 17 归并 D 档发现处置链。
- **修复轮次后（据任务回报）**：#33 时变属性链路（SEM-1，~30 C/D→A）；#34 事件系统建模（SEM-7，~110 C→A/B）；#40 CZML 11 类几何包（69 C→A/B）；#41 GPX 40 + exportKml 36（C→A/B）；#42 EntityCluster 35（C→A/B）。残余：GeometryUpdater 族（DS-06）、位置属性高阶（DS-08）、Composite 族（DS-10）归任务卡。

### 2.8 Workers + Widget（f8，207 函数 = Workers 148 + Widget 59）

全量逐函数表：[docs/audit/f8_workers.md](audit/f8_workers.md)

| 档位 | 计数 | 拆分 |
| --- | ---: | --- |
| A | 17 | Workers 13 + Widget 4（rayon 化口径：字节入口 + `_unpacked` 变体） |
| B | 23 | Phase 2 解析 26（清点差 3，见 b_tier_summary.md）；终态 no-evidence 26 |
| C | 101 | Workers 65（decodeI3S 35 / vector tile 族 / draco / ktx2）+ Widget 36 |
| D | 48 | 已登记 3（Widget）/ 未登记 45（字节桩群，#37 已补登） |
| E | 18 | 已登记 7 / 未登记 11（onmessage/postMessage 外壳） |

- **Phase 2 终态变化**：B 池全部 no-evidence（mock 框架覆盖不足，无专项差分）。
- **修复轮次后**：#36 TaskProcessor 错误信号 + worker 回接 + create_geometry（SEM-3/SEM-4，~30 D/C→A）；#39 dispatch 登记缺失修正 + 6 个 create_geometry 回接缺陷；#44 workers 一处回接（create_coplanar_polygon_geometry）。残余归任务卡 WK-01/WK-02。

### 2.9 widgets（f9，444 函数 / 52 个 JS 文件）

全量逐函数表：[docs/audit/f9_widgets.md](audit/f9_widgets.md)

| 档位 | 计数 | 拆分 |
| --- | ---: | --- |
| A | 83 | 符号引用级覆盖（widgets 集成测试 133 passed / 54 ignored 实测） |
| B | 148 | 66 可差分（Phase 2 全判 NODIFF-DOM）+ B(gpu-limited) 82（Inspector 两类 VM） |
| C | 52 | C-未登记 13 + C-台账不符 39（deviations viewer.rs 声明不符，Widget DOM 面） |
| D | 44 | 已登记 19 / 未登记 25（#37 已补登） |
| E | 117 | 已登记 35 / 未登记 82（DOM/Knockout） |

- **Phase 2 终态变化**：B 池 66 可差分全部 NODIFF-DOM（Knockout/DOM 依赖无法离线差分），82 gpu-limited 维持。
- **修复轮次后**：#36 projection_picker 默认值修复（SEM-2）；#17/#18 GeocoderService/computeFlyTo/fly_home/morph/credits 回接（审查前批次）；C 档 52 归任务卡 WD-01（DomSurface 前置）。

### 2.10 Shaders（f10，480 项 = 318 GLSL + 13 WGSL + 149 czm_* 符号）

全量清单与裁决：[docs/audit/f10_shaders.md](audit/f10_shaders.md)

| 维度 | 档位 | 计数 | 说明 |
| --- | --- | ---: | --- |
| GLSL 嵌入（318） | A | 318 | 双向 diff 为空 + SHA256 318/318 一致 |
| 手写 WGSL（13） | D | 13 | 全部 TEXONLY/冒烟裁剪变体，已登记 + 未登记双重标注 |
| czm_* 符号（149） | A | 6 | czm_modelViewProjection/modelView/projection/view/model/viewport |
| czm_* 符号（149） | C | 143 | 93 函数 + 41 常量 + 8 结构体 + czm_eyeHeight（任务卡 SH-01） |

- **Phase 2 终态变化**：无 B 档，不参与差分验证。
- **修复轮次后**：无修复任务覆盖；czm_* 143 项归任务卡 SH-01（GPU-required，naga 验证），PostProcess 26 归 SH-02。

---

## 3. 交叉索引

- Phase 2 逐行终态明细：`audit/b_tier_final.csv`（1,304 行）+ [docs/audit/phase2_b_tier_verification.md](audit/phase2_b_tier_verification.md)
- 台账闭环与漂移：[docs/audit/phase3_ledger_closure.md](audit/phase3_ledger_closure.md)（deviations/deferred/ignored/MAPPING/spec_coverage 修正清单 L1–L7、Drift-1..7）
- C 档修复任务卡（37 张）：[docs/audit/fix_task_cards.md](audit/fix_task_cards.md)
- D 档与 CZ-01 修复明细：[docs/audit/d_tier_fixes.md](audit/d_tier_fixes.md)
- 审查总报告：[docs/fidelity_review_report.md](fidelity_review_report.md)
