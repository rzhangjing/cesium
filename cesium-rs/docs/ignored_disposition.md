# Ignored 处置基线（Ignored Disposition Ledger）

对 `cesium-specs` crate 中所有 `#[ignore]` 用例的逐文件三分类处置基线。
生成脚本：`classify_ignored.ps1`（扫描 `specs/tests/**/*.rs`）。

**统计时间**：2026-08-23（Phase 0 基线，18:10 收口刷新）；2026-08-25 任务 #31 复核刷新（基线 220 → 221，新增 1 条见 (a) 节）
**扫描范围**：`specs/tests/**`（specs crate，共 221 处 `#[ignore]` 属性，与 `cargo test` 运行时 ignored 数一致：core.rs 219 + core_fidelity_batch 2）

> 说明：本次 Phase 0 已回填 9 条（`Cartesian4::from_color` ×3、`Cartesian3.fromDegrees/fromRadians` ×6），
> specs crate ignored 由 227 降至 218；收口时 Robin 地形保真度批新增 2 条（待 Track B4-3/4/5 worker），基线升至 220。
> 2026-08-25 复核再新增 1 条（`core_google_earth_enterprise_terrain_data_spec.rs` upsample worker 依赖），基线升至 221。
> 分类仅统计行首 `#[ignore]` 属性，不含文档注释中的字面提及。

---

## 三分类定义

| 分类 | 含义 | 处置 |
| --- | --- | --- |
| **(a) 可解除（unignore）** | 因依赖尚未回填而被 ignore；依赖就绪后可解禁并通过 | 归属回填批次（A 系列），列入后续批次执行 |
| **(b) 永久设计性偏差（deviation）** | Rust 静态类型/语言模型使该用例在语义上不可达或无需镜像 | 保持 `#[ignore]`，在 `deviations.md` 登记 |
| **(c) GPU-required（Track B）** | 需真实 GPU 上下文（wgpu device/queue 或 Scene Context） | 标注挂 Track B（wgpu headless 集成测试） |

---

## 总览

| 分类 | 条数 | 占比 |
| --- | ---: | ---: |
| (a) 可解除 | 14 | 6.3% |
| (b) 永久设计性偏差 | 206 | 93.2% |
| (c) GPU-required（Track B） | 1 | 0.5% |
| **合计** | **221** | 100% |

---

## 批次图例（归属批次基线）

A 系列 = CPU/领域回填批次（无 GPU 依赖）；B 系列 = GPU 批次（Track B）。
此图为 Phase 0 拟定的基线编号，leader 可按总批次规划调整映射。

| 批次 | 内容 | 对应里程碑 |
| --- | --- | --- |
| A1 | Core 椭球补全：`Ellipsoid.cartographic_array_to_cartesian_array` | M1-W2 后续 |
| A2 | Core 几何前置类型补全：Rectangle/Ellipsoid/GeographicProjection → BoundingRectangle spec | M1-W2/W3 |
| A3 | Core Matrix4 补全：`inverse_transpose` 等 | M1-W2 |
| A4 | Core 缺陷修复：Hermite `fill_coefficient_list` usize 回绕 | 即时 |
| B1 | Renderer wgpu headless 集成测试 | M2 |
| B2 | Scene wgpu headless 集成测试 | M3 |
| B3 | Scene Context 探测（WebGL2 probe / feature_detection） | M3 |
| B4 | Model/Widgets GPU | M3–M5 |
| B4-3/4/5 | Globe 瓦片管线 worker（quantized terrain mesh / upsample） | M3 |

---

## (a) 可解除（14 条）

| 文件 | 条数 | ignore 原因 | 归属批次 |
| --- | ---: | --- | --- |
| `core_cartesian3_spec.rs` | 6 | `deferred: expected computed via Ellipsoid.cartographicArrayToCartesianArray` | **A1** |
| `core_bounding_rectangle_spec.rs` | 2 | `deferred: requires Rectangle/Ellipsoid/GeographicProjection` | **A2** |
| `core_plane_spec.rs` | 2 | `deferred: requires Matrix4::inverse_transpose (M1-W2)` / `requires Matrix4 (M1-W2)` | **A3** |
| `core_hermite_polynomial_approximation_spec.rs` | 1 | `usize wrapping bug in fill_coefficient_list for i>1` | **A4**（缺陷修复） |
| `terrain_fidelity_spec.rs` | 2 | `requires createVerticesFromQuantizedTerrainMesh/upsampleQuantizedTerrainMesh worker (Track B4-3/4/5)` | **B4-3/4/5**（收口时新增） |
| `core_google_earth_enterprise_terrain_data_spec.rs` | 1 | `DEVIATION 4: upsample requires the upsampleQuantizedTerrainMesh worker port`（:163） | **B4-3/4/5**（任务 #31 复核新增，与 terrain_fidelity 同家族） |

> A4 说明：该条是 `fill_coefficient_list` 在 `i > 1` 时的 `usize` 下溢回绕缺陷（非设计性偏差），
> 修复被测代码后即可解禁，不应归入永久偏差。

---

## (b) 永久设计性偏差（206 条）

按 ignore 原因家族归并。所有家族均源于 Rust 静态类型系统与 CesiumJS 动态语义的差异，
属设计性偏差，保持 `#[ignore]` 并在 `deviations.md` 登记（见下一节补登记）。

| 原因家族 | 条数 | 涉及文件 | 说明 |
| --- | ---: | --- | --- |
| JS undefined-argument DeveloperError（静态不可达） | 121 | cartesian2/3/4 等 | JS 传 `undefined` 触发 DeveloperError；Rust 类型系统使该路径不可达 |
| JS missing-result DeveloperError（result 必选） | 16 | cartesian2/3/4 | JS `result` 缺省新建对象并校验；Rust 出参 `&mut` 强制必传 |
| JS typed-array 分支（单一 `Vec<f64>`） | 6 | cartesian2/3/4 | JS `Float64Array`/普通数组双分支；Rust 单一表示 |
| math 单函数 undefined/非数（静态不可达） | 37 | math_spec | 同 undefined-argument 家族（逐函数镜像） |
| Check.typeOf 静态类型桩 | 7 | check_spec | JS `typeof` 动态判断；Rust 静态类型已保证 |
| Event listener 身份/scope（ListenerId） | 8 | event_spec | JS 函数身份 + scope；Rust 以 `ListenerId` 为键 |
| isLeapYear year 参数（`f64`，无 undefined/null/非数） | 3 | is_leap_year_spec | Rust `f64` 无 undefined/null |
| JS undefined-argument behavior（其余） | 3 | cartesian2/3/4 | 同 undefined-argument 家族 |
| binary_search 签名必选参数（comparator/item/array） | 3 | binary_search_spec | Rust 签名强制必传 |
| clone `deep` 标志 no-op | 1 | clone_spec | Rust `Clone` 按值语义 |
| `document.location.href` 依赖 | 1 | get_absolute_uri_spec | 原生构建无 document（见 deferred #4） |
| **小计** | **206** | | |

---

## (c) GPU-required / Track B（1 条）

| 文件 | 条数 | ignore 原因 | 归属 |
| --- | ---: | --- | --- |
| `core_feature_detection_spec.rs` | 1 | `requires a cesium-scene context (WebGL2 probe), ported in M3` | **B3 / Track B**（即 deferred #3） |

---

## 逐文件处置汇总

| 文件 | `#[ignore]` 总数 | 可解除 | GPU | 永久偏差 |
| --- | ---: | ---: | ---: | ---: |
| `core_cartesian2_spec.rs` | 61 | 0 | 0 | 61 |
| `core_cartesian4_spec.rs` | 56 | 0 | 0 | 56 |
| `core_math_spec.rs` | 37 | 0 | 0 | 37 |
| `core_cartesian3_spec.rs` | 35 | 6 | 0 | 29 |
| `core_event_spec.rs` | 8 | 0 | 0 | 8 |
| `core_check_spec.rs` | 7 | 0 | 0 | 7 |
| `core_binary_search_spec.rs` | 3 | 0 | 0 | 3 |
| `core_bounding_rectangle_spec.rs` | 2 | 2 | 0 | 0 |
| `core_is_leap_year_spec.rs` | 3 | 0 | 0 | 3 |
| `core_plane_spec.rs` | 2 | 2 | 0 | 0 |
| `core_clone_spec.rs` | 1 | 0 | 0 | 1 |
| `core_feature_detection_spec.rs` | 1 | 0 | 1 | 0 |
| `core_get_absolute_uri_spec.rs` | 1 | 0 | 0 | 1 |
| `core_hermite_polynomial_approximation_spec.rs` | 1 | 1 | 0 | 0 |
| `core_google_earth_enterprise_terrain_data_spec.rs` | 1 | 1 | 0 | 0 |
| `terrain_fidelity_spec.rs` | 2 | 2 | 0 | 0 |
| **合计** | **221** | **14** | **1** | **206** |

---

## 刷新说明

- 重跑 `classify_ignored.ps1` 可再生成本基线的逐文件/逐原因统计。
- 每次有批次（A/B）完成解禁后，应更新本表对应条目并同步 `spec_coverage.md` 的 ignored 数。
- specs crate 之外（如 `cesium-data-sources` crate 内 76 条 CZML/GeoJSON/KML/GPX 占位 ignore）
  不在本基线范围，另行跟踪（见下节补登）。

---

## 补登：非 specs crate ignored 处置（任务 #37，2026-08-25）

> 来源：`docs/audit/f9_widgets.md` SEM-3（审查日 2026-08-24）。本基线原仅覆盖 specs crate；
> f9 审查发现 cesium-widgets 的 54 例 ignored 无处置登记，按同口径三分类补登如下（按报告时点状态）。

### cesium-widgets（实测基线 133 passed / 0 failed / 54 ignored）

| 分类 | 条数 | 涉及 | 处置 |
| --- | ---: | --- | --- |
| (c) GPU-required（Track B4） | 43 | 29 例 Cesium3DTilesInspectorViewModel + 13 例 CesiumInspectorViewModel + 1 例 createScene render；测试体仅 `let _ = XxxViewModel::default();`，对应三类 Inspector VM 整文件桩化（见 deferred.md #26） | 挂 Track B4（GPU Scene 依赖解除后解禁；VM 实质化同批） |
| 其余未分类 | 11 | 54 − 43；逐条明细见 f9 报告 §0 证据基线与测试文件 | 待逐条归类（GPU/依赖/平台性），暂维持 `#[ignore]` |
| **合计** | **54** | | |

### cesium-data-sources（另行跟踪项确认）

- 76 条 CZML/GeoJSON/KML/GPX 占位 ignore 维持“另行跟踪”现状（f7 审查确认其属占位性质，
  未列入本基线；实质化批次解禁时应同步更新本节与 spec_coverage.md）。

> 说明：f9 SEM-2（projection_picker 默认值错误）为语义缺陷而非 ignored 处置事项，不在本表范围，归修复任务 #36。

---

## 基线外 ignored 清点（任务 #31 复核，2026-08-25）

> 本基线仅覆盖 specs crate；以下为复核时对基线外 crate 的 `#[ignore]` 实测清点与处置标注。

| crate | 实测数 | 台账口径 | 处置/说明 |
| --- | ---: | --- | --- |
| cesium-widgets（tests/） | 54 | 上节补登节已覆盖（43 GPU + 11 待逐条） | 一致，无漂移 |
| cesium-scene（tests/ + src/ 内联） | 5 | 原未登记 | 3 例 GPU smoke harness（billboard_collection.rs:614 / label_collection.rs:421 / point_primitive_collection.rs:381，均 `requires a wgpu device`）挂 Track B；1 例 ground_primitive.rs:183（terrain draping/classification 待 M4）挂依赖批次；1 例 diag_box_textured.rs:25（诊断用非功能测试）不计功能覆盖。本行即补登 |
| cesium-data-sources（tests/ + src/） | 42（复核时点） | Phase 0 口径 76 占位；spec_coverage 2026-08-24 实测 38 | 该 crate 正被修复任务 #40–#42 修改，ignored 数处于变动中；按报告时点状态维持“另行跟踪”，解禁批次完成后再刷新本行与 spec_coverage.md |
| cesium-test-utils / cesium-core / cesium-workers | 0（属性级） | — | 无 `#[ignore]` 属性（doc-test ignored 另见 spec_coverage.md） |
