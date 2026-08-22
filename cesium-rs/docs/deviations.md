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
