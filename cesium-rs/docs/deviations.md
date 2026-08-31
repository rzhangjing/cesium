# 偏差登记表（Deviations Log）

记录所有无法一比一移植的偏差。规约见 [PORTING_CONVENTIONS.md](PORTING_CONVENTIONS.md) 第 6 条：
代码处必须紧邻标注 `// DEVIATION:`，并在本表登记。

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| _(示例)_ Core | `cartesian3.rs` | `Cartesian3.fromDegrees` 高程缺省值由 `undefined` 改为 `0.0` | Rust 无 undefined；语义等价（`Cartographic` 默认高程即 0） | 2026-08-20 |
| Core | `developer_error.rs` / `runtime_error.rs` | `stack` 字段恒为 `None`（JS 构造时捕获调用栈） | Rust 由 panic 基建提供原生 backtrace；`toString` 语义不变 | 2026-08-20 |
| Core | `check.rs` | `Check.typeOf.*` 仅拒绝 `None`（undefined），不再做动态 typeof 判断 | Rust 静态类型系统已保证传入值类型；错误消息格式与 JS 逐字对齐 | 2026-08-20 |
| Core | `event.rs` | 监听器以不透明 `ListenerId` 为键（JS 为函数身份+scope）；`RemoveCallback` 显式接收 event 引用 | Rust 闭包无身份可比；重入语义（raise 中 add/remove）经 `RefCell` + 延迟队列一比一保留 | 2026-08-20 |
| Core | `clone.rs` | `deep` 标志为 no-op；克隆语义由各类型 `Clone` impl 决定 | Rust `Clone` 按值定义拷贝语义；共享子对象用 `Rc` 表达 | 2026-08-20 |
| Core | `combine.rs` | 合并对象图限定为 `serde_json::Value` | CesiumJS 选项包在移植代码中以 JSON 值呈现 | 2026-08-20 |
| Core | `feature_detection.rs` | 浏览器探测（PointerEvent/Image/WebP 解码/DOM canvas/Fullscreen/Web Workers）桩化为原生等价物（恒 false 或 `cfg!` 判断）；提供可注入 `FeatureDetector` 保留完整 UA 解析逻辑 | 原生构建无 DOM/navigator；spec 可通过 `FeatureDetector::new` 直接验证解析逻辑 | 2026-08-20 |
| Core | `get_absolute_uri.rs` | 无 `document` 时相对 URI 原样返回（JS 回退 `document.baseURI`/`location.href`）；urijs 相对基址合并场景不支持 | 原生构建无 document；提供 `DocumentLike` trait 注入假 document 供 spec 验证 | 2026-08-20 |
| Core | `get_base_uri.rs` / `get_extension_from_uri.rs` / `get_filename_from_uri.rs` / `urijs.rs` | `urijs` 依赖替换为 crate 私有模块 `urijs.rs`（url crate + 手写 RFC 3986 remove_dot_segments） | 避免引入完整 urijs 等价 crate；覆盖 spec 所需路径 | 2026-08-20 |
| Core | `get_image_pixels.rs` | 不绘制 canvas 读回像素，改为接收已解码 RGBA 切片（IO 边界用 image crate 解码） | 原生构建无 2D canvas | 2026-08-20 |
| Core | `load_and_execute_script.rs` | 无 DOM `<script>` 注入；保留签名与错误语义，脚本加载为桩 | 原生构建无 document.head | 2026-08-20 |
| Core | `one_time_warning.rs` | `console.warn` 替换为可替换 sink（默认 `eprintln!`，spec 可注入捕获） | Rust 无 console；dedup 注册表语义不变 | 2026-08-20 |
| Core | `create_guid.rs` | JS 模板填充随机十六进制改为 uuid crate v4 | 同属 RFC 4122 v4 模板，格式与唯一性语义一致 | 2026-08-20 |
| Core | `frozen.rs`（取代 `defaultValue`/`freezeObject`/`isObject`） | 上游 @cesium/engine 26.1.0 已删除 `defaultValue.js`/`freezeObject.js`/`isObject.js`，改移植替代文件 `Frozen.js` | 上游 API 演进；`defaultValue(a, b)` 语义在移植代码中直接以 `Option::unwrap_or` 表达 | 2026-08-20 |
| Core | `cartesian3.rs` | `ellipsoid_radii_squared()` / `set_ellipsoid_radii_squared()` 由模块级可变缺省提升为 `pub` 访问器 | JS 中 `Ellipsoid.default` 是模块级可变全局；Rust 需要公开 setter 以便后续 `Ellipsoid` 移植与 spec 镜像模拟 `Ellipsoid.default = Ellipsoid.MOON` | 2026-08-20 |
| Core (specs) | `core_cartesian2/3/4_spec.rs` | JS typed-array 分支用例（`packArray works with typed arrays` 等）`#[ignore]` 不镜像 | Rust 只有单一 `Vec<f64>` 表示，JS 的 Float64Array/普通数组双分支不存在 | 2026-08-20 |
| Core (specs) | `core_cartesian2/3/4_spec.rs` | JS `undefined` 实参 DeveloperError 用例镜像为 `#[ignore]` 空体 stub | Rust 静态类型使该错误路径不可达；保留用例名维持 spec 表面一比一 | 2026-08-20 |
| Widgets | `cesium_widget.rs` | DOM canvas 创建/resize 替换为 winit `Window` + wgpu `Surface` 管理；`render()` 中 Scene 渲染为桩（需 wgpu render pass） | 原生构建无 DOM canvas；wgpu 渲染管线在 viewer-demo 帧循环中组装 | 2026-08-23 |
| Widgets | `viewer.rs` | UI 控件（Animation/Timeline/Geocoder 等）仅保留开关字段，不创建 DOM 元素；引擎侧逻辑（Scene 管理、DataSource 显示、Entity 跟踪）完整保留 | 原生构建无 Knockout.js/DOM；UI 层由外部应用（如 viewer-demo）提供 | 2026-08-23 |
| Widgets | `knockout.rs` | Knockout.js 替换为 `DomSurface` trait + `Observable<T>` trait；`MockDomSurface` 供测试 | Rust 无 DOM/Knockout；trait 抽象 winit/web-sys/mock 后端 | 2026-08-23 |
| Widgets | `command.rs` | `Command` 以 `Arc<dyn Fn()>` + `enabled: bool` 实现（JS 为 Knockout computable + callback） | Rust 无 Knockout 可计算属性；语义等价（条件执行回调） | 2026-08-23 |
| Widgets | `viewer_*_mixin.rs` (×5) | JS prototype mixin 替换为 Rust trait（`ViewerDragDropMixin` 等） | Rust 无 prototype chain；trait 提供相同的多态扩展能力 | 2026-08-23 |
| Widgets | `animation_view_model.rs` | shuttle ring 角度/倍速映射表完整保留；play/pause 通过 `Command` 绑定 | 语义不变 | 2026-08-23 |
| Widgets | `scene_mode_picker_view_model.rs` | `SceneMode` 枚举引用 `cesium-scene`；morph 方法为桩（需 Scene.morphTo 完整实现） | 依赖 Scene 层 morph 管线 | 2026-08-23 |
| Renderer | `context.rs` | `clear()`/`draw()`/`submit()` 为桩（wgpu 渲染通过 RenderPass 编码，非 imperative 调用） | wgpu 管线模型与 WebGL2 imperative 模型根本不同；实际渲染在 viewer-demo 帧循环中 | 2026-08-23 |
| Core (specs) | `core_math_spec.rs` / `core_is_leap_year_spec.rs` / `core_binary_search_spec.rs` / `core_plane_spec.rs` 等 | JS undefined-argument / non-number DeveloperError 用例镜像为 `#[ignore]` 空体 stub（共 ~158 条） | Rust 静态类型使该错误路径不可达；保留用例名维持 spec 表面一比一。详见 ignored_disposition.md (b) | 2026-08-23 |
| Core (specs) | `core_cartesian2/3/4_spec.rs` | JS missing-result DeveloperError 用例（`result` 缺省新建对象）镜像为 `#[ignore]` stub（16 条） | Rust 出参 `&mut` 强制必传，无“缺省 result 新建”路径。详见 ignored_disposition.md (b) | 2026-08-23 |
| Core (specs) | `core_check_spec.rs` | `Check.typeOf.*` 拒绝非目标类型（non-number/string/bool/object/func/bigint）用例镜像为 `#[ignore]` stub（~7 条） | Rust 静态类型已保证传入值类型，动态 typeof 拒绝路径不可达 | 2026-08-23 |
| Core (specs) | `core_event_spec.rs` | listener 身份/scope 相关用例（`remove_listener` null/undefined、scope 参数）镜像为 `#[ignore]` stub（~6 条） | Rust 以 `ListenerId` 为键，无 JS 函数身份/scope 概念 | 2026-08-23 |
| DataSources | `entity_collection.rs` | `collectionChanged` 载荷为实体 id 列表（`CollectionChangedArgs`），JS 传集合实例与 `Entity[]`；JS `_firing`/`_refire` 重入循环保留但对安全代码不可达 | Rust 所有权模型：实体被集合按值持有，监听器无法共享实体引用；监听器内无法再入集合可变操作，重入分支为忠实保留的死代码 | 2026-08-25 |
| DataSources | `entity.rs` | `definitionChanged` 新/旧值投影为 `PropertyResult`（graphics 子对象报 `None`）；事件经 `set_*` mutator 触发（直接字段写不触发）；`show`/`parent` 的 `isShowing` 子实体级联未实现 | JS 动态属性描述符传对象引用并维护子实体层级；Rust 值模型仅存 `parent_id`，无 children 层级 | 2026-08-25 |
| DataSources | `property.rs` | `Property::definition_changed` trait 方法返回 `Option<&Event<()>>`（默认 `None`），事件载荷为 `()` | trait 默认方法无存储；`SampledProperty`/`TimeIntervalCollectionProperty` 未实质化，靠默认实现保持接口兼容；JS 载荷为属性自身（自引用不可表达） | 2026-08-25 |
| DataSources | `property_bag.rs` | `definitionChanged` 在值模型上新增/变更/删除条目时直接触发；克隆不携带事件（克隆体以新的空事件开始） | JS 经由所含 Property 对象及其订阅冒泡；Rust 值模型无独立 Property 实例；JS 无 `PropertyBag.clone` 语义可参照 | 2026-08-25 |

---

## 补登：F1–F10 逐函数保真度审查（任务 #37，2026-08-25）

> 本节为 `docs/audit/f1_core_a_c.md` … `docs/audit/f10_shaders.md`（审查日 2026-08-24）发现的
> 源码内联 `// DEVIATION:` / 桩化未登记项的批量补登（PORTING_CONVENTIONS §6 登记链修复）。
> 均按“报告时点状态”登记，未核对源码现状（任务 #33–#36 修复可能正在消除个别条目）；
> 每条注明来源报告与条目编号。标注“（待修复）”者为审查发现的语义缺陷，修复后应在本表注明完成日期。

### F1（Core A–C · f1_core_a_c.md）

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| Core | `approximate_terrain_heights.rs` | `initialize` 改为从磁盘同步读取 JSON（JS 为网络 fetch + promise；`_initPromise` 同步化）；代码 DEVIATION 注释在案但未登记 | 原生构建无网络 promise；spec 可经注入路径覆盖（来源：f1 §3.4、L135 行） | 2026-08-25 |
| Core | `arc_gis_tiled_elevation_terrain_provider.rs` | 文件头 DEVIATIONS 1–4：`fromUrl` 非 promise 化、无 RequestScheduler 节流、`getTileDataAvailable` 同步返回 `Option<bool>`（JS 返回 Promise）、ion credit 忽略 | 原生构建无 promise/调度接线；行为已被 spec 固化（来源：f1 §3.4、L162 行） | 2026-08-25 |
| Core | `associative_array.rs` | `remove`：`swap_remove` 后未修复 `indices` 哈希，多元素删除后 `get` 返回错值/可越界 panic（**SE-1 blocker**，待修复） | 审查发现的语义缺陷，非有意偏差；spec 未覆盖该路径（来源：f1 SE-1、L190 行） | 2026-08-25 |
| Core | `attribute_compression.rs` | ① `force_uint8` 饱和截断替代 JS TypedArray mod-256 环绕，致 `octEncodeToCartesian4` 编码错误且解码不可逆（**SE-2 blocker**，待修复）；② `zig_zag_delta_decode` i32 累加不做 u16 环绕（**SE-4**，待修复） | Rust 无 TypedArray 写入环绕语义；拟以 `rem_euclid` 修复（来源：f1 SE-2/SE-4、L199/L200/L212 行） | 2026-08-25 |
| Core | `catmull_rom_spline.rs` | 构造缺 points≥2 与 times 长度一致的 DeveloperError 校验；缺 `firstTangent`/`lastTangent` 选项与非对称边界切线预计算；2 点样条未退化为 lerp（**SE-3 major**，待修复） | 移植不完整；spec 布局未触发边界差异（假阴性）（来源：f1 SE-3、L513/L521 行） | 2026-08-25 |
| Core | `clock.rs` | `tick` 的 `on_tick` 事件载荷为 `()`（`Event<()>`），JS 为 `onTick.raiseEvent(this)` 传 Clock 实例（SE-5） | Rust 事件泛型模型；监听器取回 clock 引用的语义不可达（来源：f1 SE-5、L682 行） | 2026-08-25 |
| Core | `corridor_geometry.rs` / `corridor_outline_geometry.rs` | height/extrudedHeight 传参偏差（`corridor_geometry.rs:5`，`compute_positions_extruded`）与 `create_geometry` 参数集差异（`corridor_outline_geometry.rs:114`）；代码 DEVIATION 注释在案但未登记 | 参数映射取舍；需补差分（来源：f1 §3.4、L827/L860 行） | 2026-08-25 |

### F2（Core D–Z · f2_core_d_z.md）

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| Core | `resource.rs` | 模块头（:40-48）与正文共 18 处 DEVIATION 未登记：网络 IO 经 `HttpFetch` trait 桩化；RequestScheduler 节流未接 fetch（SEM-5）；credits 未建模（SEM-6）；`get_derived_resource` 简单 URI 拼接（SEM-7）；`decode_uri_component` 逐字节 Latin-1 解码（SEM-8）；HEAD/options 请求无响应头回读（SEM-10）；`clone` 丢 retryCallback/request 字段（SEM-2，待修复） | 原生构建无浏览器 fetch；多数为 IO 边界桩化，SEM-2 为移植遗漏（来源：f2 SEM-1/SEM-2/SEM-5/SEM-6/SEM-7/SEM-8/SEM-10、D-未登记 18 行） | 2026-08-25 |
| Core | `cesium_terrain_provider.rs` | 模块级（:33-55）6 条 + 正文 27 处 DEVIATION 未登记：availability 位图未加载即报错（SEM-3）、availability 请求不发起（SEM-4）等同源 IO/调度桩化 | 原生构建无网络调度接线（来源：f2 SEM-1/SEM-3/SEM-4） | 2026-08-25 |
| Core | `request_scheduler.rs` | 9 处 DEVIATION 未登记：无浏览器并发上限语义，请求直通（未接真实 fetch） | 原生构建无浏览器网络栈（来源：f2 SEM-1） | 2026-08-25 |
| Core | `fullscreen.rs` | 6 处 DEVIATION 未登记：Fullscreen API 桩化为恒 false/`enabled=false` | 原生构建无 DOM fullscreen（来源：f2 SEM-1） | 2026-08-25 |

### F3（Renderer · f3_renderer.md）

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| Renderer | `render_state.rs` | `apply_cull`/`apply_line_width`/`apply_polygon_offset`/`apply_depth_range`/`apply_sample_coverage`（L470/471/472/474/482）为 no-op，DEVIATION 未登记 | wgpu 管线状态在 pipeline 创建期固化，无运行时 imperative 修改路径（来源：f3 L470–482 行、Major#5） | 2026-08-25 |
| Renderer | `texture.rs` | 纹理格式降级映射（L610）与 `generate_mipmap` 桩（L636），DEVIATION 未登记 | wgpu 格式/采样器模型差异；mipmap 生成管线待接（来源：f3 L610/L636 行、Major#4） | 2026-08-25 |
| Renderer | `compute_command.rs` / `compute_engine.rs` | `execute`（L101）/ `dispatch`（L108）空桩，DEVIATION 未登记 | wgpu compute 管线待接（来源：f3 L101/L108 行） | 2026-08-25 |
| Renderer | `demodernize_shader.rs` | no-op（L304）：GLSL 降级转换未实现，DEVIATION 未登记 | Batch B naga 转译路线未启动（来源：f3 L304 行） | 2026-08-25 |
| Renderer | `shader_program.rs` | WGSL-only：GLSL 编译链路不存在（B2.2），DEVIATION 未登记 | shader-strategy.md Batch B 路线（来源：f3 Major#2） | 2026-08-25 |
| Renderer | `draw_command.rs` / `create_uniform*.rs` | uniform 上传仅 Float/Vec4/Texture 三类，缺 52 个上传类型（B2.6），DEVIATION 未登记 | 冒烟路径裁剪（来源：f3 Major#3） | 2026-08-25 |
| Renderer | `automatic_uniforms.rs` | 仅 7 项冒烟子集（约 100+ czm_* 中的 B2.5），DEVIATION 未登记 | 冒烟路径裁剪（来源：f3 L73 行、B2.5） | 2026-08-25 |

### F4（Scene A–G · f4_scene_a_g.md）

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| Scene | `gltf_index_buffer_loader.rs` / `gltf_vertex_buffer_loader.rs` / `gltf_texture_loader.rs` / `gltf_image_loader.rs` | 4 个 gltf loader 内联 DEVIATION 未登记：UNSIGNED_BYTE u8 索引扩宽 u16、Draco/Spz/量化路径延迟、deprecationWarning 裁剪；ResourceCache 去重去除、sampler 记录方式不同；外部 URI fetch 延迟、image crate 解码替代浏览器 Image | GPU 依赖与网络 IO 边界裁剪（来源：f4 SEM-major #1） | 2026-08-25 |

### F5（Scene H–Z · f5_scene_h_z.md）

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| Scene | cesium-scene 全 crate（含 `quadtree_primitive.rs`、`imagery_layer.rs`、`label_collection.rs`、`globe_surface_tile_provider.rs` 等） | 源码 148 处内联 `// DEVIATION:` 标记均未登记（范围性登记；逐处明细见 f5 报告正文各文件节） | 批量补登恢复登记链（来源：f5 SEM-1 blocker） | 2026-08-25 |
| Scene | `quadtree_primitive.rs` | QuadtreePrimitive all-or-nothing 就绪语义降级为单帧同步加载（B4-2）：三级加载队列与 5ms 时间片仅结构占位；`ancestorMeetsSse` fall-through 与雾衰减不触发（:21/:41/:198） | Track B4-2 冒烟路径裁剪（来源：f5 SEM-2） | 2026-08-25 |

### F6（Scene/Model + GltfPipeline · f6_scene_model.md）

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| Scene | `cesium-scene/src/gltf_pipeline/*.rs`（23 个 stub：addBuffer/addDefaults/addExtensionsRequired/addExtensionsUsed/addPipelineExtras/addToArray/findAccessorMinMax/ForEach/forEachTextureInMaterial/getAccessorByteStride/getComponentReader/moveTechniqueRenderStates/moveTechniquesToExtension/numberOfComponentsForType/readAccessorPacked/removeExtension/removeExtensionsRequired/removeExtensionsUsed/removePipelineExtras/removeUnusedElements/updateAccessorComponentTypes/updateVersion/usesExtension） | 23 个 GltfPipeline 工具模块空体 stub，DEVIATION 注释未登记 | glTF 1.0→2.0 升级管线未移植（来源：f6 各文件节） | 2026-08-25 |
| Scene | `model.rs` | 文件头注 DEVIATION：model pipeline 阶段链（skin/material/animation/node 处理等）未移植，未登记 deferred | model pipeline 整体推迟（来源：f6 L840+ 行） | 2026-08-25 |
| Scene | `model_scene_graph.rs` | 无 skinning/morph target 链（内联 DEVIATION 未登记，:约 L1362） | 依赖 model pipeline（来源：f6 L1362 行） | 2026-08-25 |
| Scene | `parse_glb.rs` | `parse_glb_version1`：v1 glTF buffers 对象归一化未实现（内联 DEVIATION 未登记，:约 L132） | v1 兼容路径裁剪（来源：f6 L132 行） | 2026-08-25 |
| Scene | `model_animation_collection.rs` | `update` 固定 1/60s 步长替代实际经过时间（DEVIATION 未登记） | 冒烟路径简化（来源：f6 L1020 行，D-未登记） | 2026-08-25 |
| Scene | `model_feature.rs` | `get_property` 恒返回 `None`（DEVIATION 未登记） | 元数据管线未移植（来源：f6 L1137 行，D-未登记） | 2026-08-25 |

### F7（DataSources · f7_data_sources.md）

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| DataSources | cesium-data-sources 全 crate（7 Visualizer × 4–5、Static*Batch × 6、DynamicGeometryBatch/Updater、GeometryUpdater × 12、SampledProperty、TimeIntervalCollectionProperty、createMaterial/Property/RawPropertyDescriptor、getElement、heightReferenceOnEntityPropertyChanged、exportKml 等，文件级分布见 f7 §4） | 96 处内联 DEVIATION 均未登记（范围性登记；子类：GPU Visualizer 族 37 + Static Batch 23 + GeometryUpdater createFill/Outline 24 + 时间动态属性桩 4 + 描述符/平台桩 7） | Visualizer/Updater/Batch GPU 依赖延迟与平台桩批量补登（来源：f7 §4、D-未登记 96 行） | 2026-08-25 |
| DataSources | `sampled_property.rs`（:18/:19/:24/:25） | `SampledProperty::add_sample` 不存储采样、`get_value` 恒 `None`；`TimeIntervalCollectionProperty::get_value` 恒 `None`（**SEM-1 blocker**，待修复） | 时变属性链路桩化（来源：f7 SEM-1；修复任务 #33） | 2026-08-25 |
| DataSources | `gpx_data_source.rs`（:44/:50） | GPX 解析 40/45 特性缺失（D-未登记） | 最小可用裁剪（来源：f7 SEM-3） | 2026-08-25 |
| DataSources | `export_kml.rs`（:7） | exportKml 36/37 特性为桩（D-未登记） | 导出管线未实质化（来源：f7 SEM-4） | 2026-08-25 |
| DataSources | `entity_cluster.rs`（:16/:51） | EntityCluster 聚类管线 34/35 为桩（D-未登记） | GPU/事件依赖延迟（来源：f7 SEM-6） | 2026-08-25 |

### F8（Workers + Widget · f8_workers.md）

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| Workers | cesium-workers 全 crate：29 个 create*Geometry 字节桩 wrapper + `create_geometry.rs` 调度器 + createVectorTile* ×5 + createVerticesFromCesium3DTilesTerrain + createVerticesFromGEEBuffer + decodeDraco + decodeI3S + draco_loader decode×2 + incrementallyBuildTerrainPicker + transcodeKTX2 + upsampleQuantizedTerrainMesh worker 入口 + upsampleVerticesFromCesium3DTilesTerrain | 45 处字节桩 DEVIATION 未登记（范围性登记）；其中 14 个 wrapper 注释 "not yet ported" 与事实不符——core 侧 16 类几何 create_geometry 已实现且 spec 全绿（SEM-3）；`task_processor` dispatch 对未知任务静默返回空 `Vec`（SEM-4） | 原生构建无 Web Worker；入口桩待回接 core 实现（来源：f8 SEM-1 blocker/SEM-3/SEM-4） | 2026-08-25 |
| Workers (core 镜像) | `create_vertices_from_quantized_terrain_mesh.rs`（426 行）/ `upsample_quantized_terrain_mesh.rs`（617 行）/ `decode_google_earth_enterprise_packet.rs`（392 行） | 三文件模块级 DEVIATION：逻辑已完整镜像至 core，worker 字节入口仍为桩 | worker 入口字节桩回接待办（来源：f8） | 2026-08-25 |
| Widgets | `cesium_widget.rs` | `resize` 内 2 处 DEVIATION：pixelRatio 恒 1.0；camera frustum 更新缺失（configureCameraFrustum 未移植，:约 L353/L389） | 冒烟路径裁剪（来源：f8 L353/L389 行） | 2026-08-25 |

### F9（Widgets · f9_widgets.md）

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| Widgets | `geocoder_view_model.rs`（14 处内联）、`vr_button_view_model.rs`（10）、`fullscreen_button_view_model.rs`（9）、selection_indicator/projection_picker/performance_watchdog/i3s/home_button 各 VM、`animation.rs` destroy/resize 最小桩、widget 壳文件、InspectorShared 字符串工厂等（共 25 处 D-未登记；E-未登记 82 行见 deferred.md） | 内联 `// DEVIATION:` 未在 deviations.md 登记（范围性登记；逐处明细见 f9 §6 各文件表） | 批量补登恢复登记链（来源：f9 SEM-1 blocker） | 2026-08-25 |
| Widgets | `subscribe_and_evaluate.rs` | 纯桩 no-op：`fn subscribe_and_evaluate<F: Fn()>(_callback: F)` 空体，签名与 JS `subscribeAndEvaluate(target, callback, scope)` 不符（**SEM-4 major**，待修复） | knockout-es5 响应式求值无等价物；当前无调用方，风险为潜在（来源：f9 SEM-4） | 2026-08-25 |
| Widgets | `provider_view_model.rs`（:182-191）/ `geocoder_view_model.rs`（:383）/ `base_layer_picker_view_model.rs`（:145） | DeveloperError 检查未加 `#[cfg(debug_assertions)]` 门，release 构建同样 panic（对照 home_button_view_model.rs:56/122 的正确做法） | 违反移植规约 §3 错误裁剪约定（来源：f9 SEM-6） | 2026-08-25 |

### F10（Shaders · f10_shaders.md）

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| Shaders | `cesium-shaders/wgsl/`：viewport_quad_vs.wgsl、viewport_quad_color_fs.wgsl、viewport_quad_texture_fs.wgsl、globe_vs.wgsl（TEXONLY，17/18 czm_* 缺）、globe_fs.wgsl（TEXONLY，45/47 缺）、billboard_vs.wgsl（25/28 缺）、billboard_fs.wgsl（10/11 缺）、model_color_vs.wgsl、model_textured_vs.wgsl、model_color_fs.wgsl、model_textured_fs.wgsl、primitive_vs.wgsl、primitive_fs.wgsl | 13 个手写 WGSL 均为冒烟路径裁剪变体（文件头 DEVIATION 引用 shader-strategy.md），裁剪明细未在 deviations.md 逐条登记（范围性登记；逐文件裁剪项见 f10 §二覆盖矩阵） | shader-strategy.md Batch C/D 裁剪路线（来源：f10 ②、D-未登记 13） | 2026-08-25 |
| Shaders | `PostProcessStages/*.glsl` ×26（FXAA/Bloom/AmbientOcclusion/AcesTonemapping/DepthOfField/LensFlare/Silhouette 等） | 26 项后处理整体无 WGSL 实现（GLSL 仅嵌入）；shader-strategy.md Batch D 要求登记 deviations.md | Batch D 待手动 WGSL（来源：f10 ②） | 2026-08-25 |

---

## 补登对账（任务 #37）

> 口径："报告 D/E/SEM 未登记行数"为各报告正文明示的未登记计数；"deviations.md 新增行数"为本节各 F 小节的表格行数（范围性登记一行可覆盖多行报告条目）；未计入本表的项已分流至 deferred.md / ignored_disposition.md，见各台账对应补登节。

| 报告 | 报告未登记项（索引） | deviations.md 新增行数 | 分流说明 |
| --- | --- | --- | --- |
| f1_core_a_c.md | D-未登记 21 + SE-1..SE-5 | 7 | corridor_outline L860 并入 corridor 行；terrain_data/geometry_processor/createWorld* 桩、C 档 99 backlog → deferred.md |
| f2_core_d_z.md | D-未登记 18 + SEM-1..SEM-10 | 4 | SEM-9（deferred.md #19/#27 台账不符）→ deferred.md 注记；E-未登记 73、C 档 444 → deferred.md |
| f3_renderer.md | D-未登记 10 + Major#1–#6 | 7 | Major#1/#6 及 C 档 334 → deferred.md |
| f4_scene_a_g.md | 桩文件 113（2198 条 C）+ 4 loader | 1 | 桩文件/C 档整体 → deferred.md；CameraFlightPath/Cesium3DTileset::fromUrl 入口缺失 → deferred.md（修复任务 #35） |
| f5_scene_h_z.md | 148 处 DEVIATION + SEM-2 | 2 | E-未登记 183、C 档 2457 → deferred.md |
| f6_scene_model.md | 23 stub + 5 内联 + 76 空壳 | 6 | model pipeline 空壳 76（任务索引口径 78+12，报告正文清点 Model/ 66 + Gpm 10 = 76）→ deferred.md |
| f7_data_sources.md | D-未登记 96 + SEM-1..SEM-7 | 5 | SEM-2/5 及 C 档 771 → deferred.md；SEM-7（事件系统建模缺失）已由修复任务 #34 处置（已完成），归修复记录不入 deferred；SEM-9/10/11 → deferred.md #23 |
| f8_workers.md | 45 桩 + 3 镜像 + 2 resize | 3 | E-未登记 11、C 档 101 → deferred.md |
| f9_widgets.md | D/E 未登记 107 行 + SEM-1..SEM-6 | 3 | SEM-3 Inspector 桩化 → deferred.md + ignored_disposition.md；SEM-5 createDefault* → deferred.md；E-未登记 82、C-未登记 13 → deferred.md；SEM-2 projection_picker 默认值为缺陷（非偏差），归修复任务 #36 |
| f10_shaders.md | D-未登记 13 + PostProcess 26 | 2 | Major#2（143 czm_* 缺失）、Major#3（Batch B 未实现）→ deferred.md |

未覆盖项说明：无。全部报告未登记清单均已落入三本台账之一；f9 SEM-2 经判定属语义缺陷而非登记偏差，不属三本台账范畴，已注明归修复批次。

> 任务 #31 复核修正（2026-08-25）：
> ① f7 行原分流说明称 SEM-7 → deferred.md，与 deferred.md #23 实际内容（仅 SEM-2/5/9/10/11）不符；SEM-7 属行为缺陷且已由修复任务 #34 完成，已按上表更正归属。
> ② 本节补登表格实际行数为 40 行（7+4+7+1+2+6+5+3+3+2），与任务背景口径“42 行”的差为口径差异（逐处 DEVIATION 与范围性合并登记的计数方式不同），逐报告分流对账以本节各行数为准。

---

## 补登：Phase 2 B 档差分发现 D1–D7（行为缺陷）（任务 #31，2026-08-25）

> 来源：`docs/audit/phase2_b_tier_verification.md` §6 发现清单（审查 R11）。D1–D7 均属行为缺陷：
> 已有修复任务者归修复记录并标注；截至本次复核无修复任务的未处置项登记如下。

| 发现 | 模块/文件 | 偏差描述 | 处置状态 | 日期 |
| --- | --- | --- | --- | --- |
| D1 | Core `scale_to_geodetic_surface.rs`（:75/:103） | NaN/非收敛输入死循环（无迭代上限；JS 有限迭代后 throw DeveloperError） | **在修**：修复任务 #43 在途（差分用例 carto.fromCartesian.c8 等 skipped） | 2026-08-25 |
| D2 | Core `attribute_compression.rs:221` `decode_rgb565` | 与 JS 存在 1-ULP 舍入差（纯 f32 运算 vs JS f64 中间量后收窄） | 待处置（minor，差分 case ac.decodeRGB565.a0；B-27） | 2026-08-25 |
| D3 | Core `color.rs:447` `float_to_byte` | 越界分量饱和钳制（`as u8`）vs JS `Math.round` 后按位截断无钳制；toBytes/toCssHexString/toRgba 发散 | 待处置（B-82/B-83/B-85） | 2026-08-25 |
| D4 | Core `pixel_format.rs` | `PixelFormat.createTypedArray` / `PixelFormat.flipY` 未移植（差分 missing-symbol） | 待处置（B-309/B-310） | 2026-08-25 |
| D5 | Core `cartographic.rs:272-276` | `Display` 格式与 JS toString 不一致（Infinity 打印为 `inf` vs JS `Infinity`） | 待处置（minor，B-54） | 2026-08-25 |
| D6 | Core `attribute_compression.rs`（:27/:125/:150）、`vertical_exaggeration.rs:12` | debug 守卫缺失类：JS `includeStart('debug')` 检查未映射为 `debug_assert`（非单位向量静默归一化、packed 值范围检查缺失、有限性检查缺失） | 待处置（按 PORTING_CONVENTIONS §3 应补 debug_assert；B-18/B-22/B-23/B-25/B-445） | 2026-08-25 |
| D7 | Core `heading_pitch_roll.rs` | `HeadingPitchRoll.equalsEpsilon` 未移植（missing-symbol） | 待处置（B-230） | 2026-08-25 |

---

## 修复轮次二补登（任务 #41/#42/#40/#43/#44，2026-08-25）

> 本节为修复轮次二（任务 #40–#44）实质化过程中新增的有意偏差登记（来源：各修复代理汇报 +
> 代码内 DEVIATION 注释）；整体推迟/不可达项见 deferred.md 补登节（#33 起）。
> gpu-limited 项标注 Track B 解禁条件。

### 任务 #41 GPX 实质化（`gpx_data_source.rs`）

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| DataSources | `gpx_data_source.rs` | 5 项：① waypoint `name` 无值时返回空串 `""`（JS 为 `undefined`）；② 自定义 waypoint 图像以字符串路径表达（JS 为 HTML Image 对象）；③ `loading` 事件无载荷（JS raiseEvent 携带数据源实例）；④ 时变 track 位置保留首采样点（无 `SampledPositionProperty` 插值模型）；⑤ 默认 track 材质仅主色（JS 为 `PolylineOutlineMaterial` 含 outline 参数） | Rust 无 DOM Image/undefined 语义；时变位置依赖 SampledPositionProperty 实质化；材质管线裁剪（来源：任务 #41 汇报） | 2026-08-25 |

### 任务 #41 exportKml 实质化（`export_kml.rs`）

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| DataSources | `export_kml.rs` | 8 项：① DOM 树序列化改为 `KmlElement` 树序列化；② `ValueGetter` 退化为常量读取（JS 支持时变求值）；③ graphics sentinel 以默认值模拟 JS 懒创建语义；④ `heading` 输出以 `alignedAxis == UNIT_Z` 门控（JS 按模型朝向属性）；⑤ `<fill>` 元素永不输出；⑥ rectangle/zHeightReference/drawOrder 无值模型对应而不输出；⑦ kmz 导出返回 `Err`（未引入 zip 依赖；关联 4 项不可达函数见 deferred.md #33）；⑧ Blob 输出改为 `Vec<u8>` | Rust 无 DOM/Blob；值模型与材质表达裁剪；zip 依赖待引入（来源：任务 #41 汇报） | 2026-08-25 |

### 任务 #42 EntityCluster 实质化（`entity_cluster.rs`）

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| DataSources | `entity_cluster.rs` | 6 项：① `_clusterDirty` microtask 延迟置脏改为立即置脏；② 簇标签数值格式化 `toLocaleString` 区域化改为纯十进制；③ `ready` 不含 glyph/atlas 就绪检查（**gpu-limited**：glyph 光栅化/纹理图集需 GPU，Track B 条件解除后回填）；④ zoom-in 遮挡检测并入投影回调（JS 为独立遮挡查询）；⑤ 相机缩放 `amount < 0.05` 阈值未建模；⑥ `disableCollectionClustering` 作用域与 JS 存在差异 | Rust 无 microtask/区域化 locale；遮挡/投影渲染边界 gpu-limited（依赖 primitive update 与投影回调，Track B 条件：wgpu Scene 上下文就绪）（来源：任务 #42 汇报） | 2026-08-25 |

### 任务 #40 CZML 实质化（`czml_property.rs` / `czml_unwrap.rs` / `czml_processing.rs` / `czml_geometry.rs`）

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| DataSources | `czml_property.rs` / `czml_unwrap.rs` / `czml_processing.rs` / `czml_geometry.rs` | 5 项：① 时变几何数据存储于 sidecar `CzmlGeometryStore` 而非 `Entity/*Graphics` 属性链；② `SampledProperty` 无 `Clone`/`Debug` 时的降级路径；③ `Cartesian2`/`BoundingRectangle`/`DistanceDisplayCondition` 采样数据不摄入；④ holes 的 references per-hole interval 不支持；⑤ 数据变更后 `definitionChanged` 不触发 | Entity/*Graphics 时变属性链未全量就绪（修复任务 #33 在途）；事件链依赖任务 #34 建模（来源：任务 #40 汇报） | 2026-08-25 |

### 任务 #43/#44 D 档修复（`docs/audit/d_tier_fixes.md`）

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| Core | `corridor_outline_geometry.rs` | combine 首 corner 索引与 JS 存在 ±1 偏差 | **已闭环**（2026-08-31，任务 #45 后续差分）：Rust 已对齐 JS，差分通过（来源：任务 #43/#44、d_tier_fixes.md；闭环确认于三轮统一验证 #51） | 2026-08-25 |

> D1 表述修正备注（任务 #43/#44，仅备注）：`docs/audit/d_tier_fixes.md` 对上节 D1（`scale_to_geodetic_surface.rs` NaN/非收敛死循环）的处置状态表述已修正，实质修复与差分用例状态以修复任务 #43 记录为准，本表不新增条目。
> CZ-01 移交项（PolygonGeometry/RectangleGeometry/GroundPolylineGeometry 内部函数缺口）归 deferred.md #34。

---

## 修复轮次二补登对账（任务 #45）

| 任务号 | 清单条目数 | deviations.md 新增行数 | 分流说明 |
| --- | ---: | ---: | --- |
| #41 GPX | 5 | 1（合并行，含 5 项明细） | 全部为行为偏差 |
| #41 exportKml | 8 + 4 不可达 | 1（合并行，含 8 项明细） | createKmz/addExternalFilesToZip/getRectangleBoundaries/createGroundOverlay 4 项不可达 → deferred.md #33 |
| #42 EntityCluster | 6 + gpu 渲染边界 | 1（合并行，含 6 项明细 + gpu-limited 标注） | 渲染边界（投影/遮挡/primitive update）gpu-limited 已随本行登记，Track B 条件标注 |
| #40 CZML | 5 | 1（合并行，含 5 项明细） | 全部为行为偏差 |
| #43/#44 | D1 备注 + CorridorOutline 1 + CZ-01 移交 3 组 | 1（CorridorOutline 行，标注在查）+ D1 备注块 | CZ-01 移交项 → deferred.md #34 |

未覆盖项说明：无。本轮全部清单均已落入 deviations.md 或 deferred.md；gpu-limited 项均已标注 Track B 解禁条件。

---

## 修复轮次三补登（任务 #46/#47/#48/#49/#50，2026-08-31）

> 本节为修复轮次三（任务 #46–#50）新增的有意偏差登记（来源：各修复代理汇报 + 源码内联注释逐条核对）。
> 三轮统一验证（任务 #51）基线：3618 passed / 0 failed / 323 ignored，差分 444/446（2 fail 为已登记 D4 假性失败）。
> 注：上节修复轮次二 CorridorOutline ±1 条目已同步更新为已闭环。

### 任务 #46（Taylor）：GroundPolylineGeometry 残余（`ground_polyline_geometry.rs`）

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| Core | `ground_polyline_geometry.rs` | `unpack` 返回克隆值而非 JS 语义引用（JS 返回同一对象引用，调用方可观测到就地修改） | Rust 值语义；以 `Clone` 表达（来源：任务 #46 汇报） | 2026-08-31 |

### 任务 #47（Felix）：TileAvailability/Transforms/Matrix4/PixelFormat 等（`transforms.rs` / `matrix4.rs` / `pixel_format.rs` 等）

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| Core | `transforms.rs`（:539/:566 等 3 处） | EOP/XYS 链 3 处：① IAU 2006 XYS 采样数据改为静态内置（JS 异步下载）；② EOP 样本未加载时返回 `None` 镜像 JS “未就绪”语义（`EarthOrientationParameters#compute`，:566）；③ 同链加载分支语义替代 | 原生构建无异步下载链；静态数据供差分验证（来源：任务 #47 汇报） | 2026-08-31 |
| Core | 对应模块（差分用例镜像层） | throws-without-parameter 类用例 6 处：JS 无参调用抛 `DeveloperError`，Rust 静态类型使该路径不可达，无法镜像 | 与 specs crate (b) 家族同源（静态类型设计性偏差）（来源：任务 #47 汇报） | 2026-08-31 |
| Core | `pixel_format.rs` | `create_typed_array` 返回新 `Vec`（JS 按类型构造对应 TypedArray 变体） | Rust 单一 `Vec` 表示，无 TypedArray 族（来源：任务 #47 汇报） | 2026-08-31 |
| Core | `matrix4.rs`（:1310） | `from_camera` 参数为 Camera 抽象 trait（JS 接收 Scene 实体 `Camera`） | Core 层不可依赖 cesium-scene；以抽象解耦逆向依赖（来源：任务 #47 汇报） | 2026-08-31 |
| Core | `transforms.rs`（:419） | `new Matrix3(...)` 分支镜像上游怪癖（upstream quirk 注记，非移植缺陷） | 一比一镜像 CesiumJS 既有行为（来源：任务 #47 汇报） | 2026-08-31 |
| Core | `pixel_format.rs`（:97） | pixel datatype 以裸 WebGL 常量 `u32` 接收（CZ-08；含 `HALF_FLOAT = HALF_FLOAT_OES (0x8D61)` 镜像） | **分层约束**：cesium-core 不可依赖 cesium-renderer 的 `PixelDatatype` 枚举（来源：任务 #47 汇报） | 2026-08-31 |

### 任务 #48（Jimmy）：Resource/RequestScheduler 调度链（`resource.rs` / `request_scheduler.rs`）

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| Core | `resource.rs`（:101） | fetch 被节流时返回 `Err(RequestThrottled)` 替代 JS `undefined`（JS 静默不入队） | Rust 错误模型显式化；调用方可判别重试（来源：任务 #48 汇报） | 2026-08-31 |
| Core | `request_scheduler.rs`（:37 等） | tracked 请求以 id 表替代 JS 对象身份（Map by object identity） | Rust 无对象身份可比；与 `event.rs` ListenerId 同源设计（来源：任务 #48 汇报） | 2026-08-31 |
| Core | `request_scheduler.rs`（:40/:200） | `requestCompletedEvent` 以线程安全监听器注册表实现（JS 单线程 `Event`，支持监听器重入） | Rust 多线程模型；重入语义经延迟队列保留（来源：任务 #48 汇报） | 2026-08-31 |
| Core | `request_scheduler.rs`（:30/:44/:831/:851 等） | `cancelFunction`/`requestFunction`/deferred promise 流未移植：取消不 reject 任何 promise，调度到 ACTIVE 后不执行请求体 | 无 promise/deferred 基建；调度状态机本体已镜像（来源：任务 #48 汇报） | 2026-08-31 |
| Core | `resource.rs` | blob URI 未实现（原生构建无 Blob/URL.createObjectURL） | 与 `get_absolute_uri.rs` 无 document 同源（来源：任务 #48 汇报） | 2026-08-31 |

### 任务 #49（Robin）：KML 高级特性 12 条（`kml_data_source.rs`）

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| DataSources | `kml_data_source.rs` | 12 条：① KMZ 归档不可达（无 zip 依赖，同 deferred #33 同源）；② NetworkLink 不抓取链接文档（:663 无 fetch）；③ NetworkLink 无刷新计时器（:678 仅登记待刷新链接）；④ `onExpire`/`onStop` 语义裁剪；⑤ NetworkLink 集合全量登记（替代 JS 增量维护）；⑥ 查询无相机分支（:1060 无 live camera/canvas）；⑦ `[cameraAlt]` 等实体替换 quirk 镜像；⑧ ScreenOverlay 以简化值模型替代（:687 无 DOM `<img>` 屏幕坐标系）；⑨ Tour 播放未实质化（:2194 Track/MultiTrack/Model 同未实质化）；⑩ `gx:LatLonQuad` 投影/`zIndex` 丢弃；⑪ BalloonStyle HTML 逐字存储不渲染（:2089）/ListStyle 仅 radioFolder 警告裁剪（:1518）；⑫ `colorMode="random"` 以确定性基色替代（:304） | 原生构建无 DOM/fetch/计时器；值模型简化（来源：任务 #49 汇报） | 2026-08-31 |

### 任务 #50（Jay）：MD-02/DS-09 残余（gltf 解码链 + EntityCollection）

| 模块 | 文件 | 偏差描述 | 原因 | 日期 |
| --- | --- | --- | --- | --- |
| Scene / Workers | `gltf_index_buffer_loader.rs`（:10/:197）、`gltf_vertex_buffer_loader.rs`（:10/:203）、`draco_loader.rs`（:7）、`transcode_ktx2.rs`（:23，cesium-workers） | MD-02：DRACO/KTX2 解码本体依赖——job scheduler 与 Draco 解码延迟至 GPU 集成；`draco_loader.rs` 以 Rust 原生解码器替代 CesiumJS WASM 模块；KTX2/Basis 转码需原生库（未接入） | **gpu-limited**：Track B（GPU 集成）条件解除后回填解码本体；转码库待引入（来源：任务 #50 汇报） | 2026-08-31 |
| DataSources | `entity_collection.rs`（:162） | DS-09：`owner` getter 层级豁免（JS 返回 DataSource 或 EntityCollection 实链，Rust 裁剪） | 简化所有权链；行为面影响轻微（来源：任务 #50 汇报） | 2026-08-31 |

### 运维注记（三轮验证 #51 结论）

> 差分装置（`audit/rust_diff_harness`）必须以 **debug 模式**构建运行：release 构建关闭 `debug_assertions`
> 门控，导致 D6 类（debug 守卫缺失类）用例产生假性回归。另：`audit/rust_diff_harness/main.rs` 头部注释对构建模式的描述具误导性，
> 以本注记为准（注记属台账登记；源码/注释按任务约束不修改）。

---

## 修复轮次三补登对账（任务 #52）

| 任务号 | 清单条目数 | deviations.md 新增行数 | 分流/备注 |
| --- | ---: | ---: | --- |
| #46（Taylor） | 1 | 1 | GroundPolylineGeometry::unpack 克隆语义 |
| #47（Felix） | 6（EOP/XYS 3 处合并 1 行 + throws 6 处合并 1 行 + 4 单项） | 6 | CZ-08 pixel datatype u32 随本批登记 |
| #48（Jimmy） | 5 | 5 | fetch 节流/tracked id/事件注册表/deferred 流/blob URI |
| #49（Robin） | 12 | 1（合并行，含 12 项明细） | 均为行为偏差，源码内联注释已核对 |
| #50（Jay） | 2（MD-02 + DS-09） | 2 | MD-02 标注 gpu-limited Track B 条件 |
| 状态更新 | — | — | 修复轮次二 CorridorOutline ±1 条目更新为已闭环 |
| 运维注记 | — | 1 注记块 | 差分装置必须 debug 模式；rust_diff_harness main.rs 头注误导性声明 |
| 卡片状态 | — | — | fix_task_cards.md 16 张卡处置状态列更新（CZ-01/03/04/05/06/07/08、DS-02/07/08/09/10、MD-02、WK-01、SC-08、SC-10；详见任务 #52 汇报） |

未覆盖项说明：无。本轮全部清单均已登记；`docs/function_fidelity_matrix.md` 与 `docs/fidelity_review_report.md` 已由用户手动修改，本任务只读未写。

