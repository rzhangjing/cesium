---
kind: error_handling
name: 基于 thiserror 的分层错误类型体系与 PortError 统一出口
category: error_handling
scope:
    - '**'
source_files:
    - cesiumrust/ports/driven/src/lib.rs
    - cesiumrust/domain/material/src/error.rs
    - cesiumrust/adapters/network/src/lib.rs
    - cesiumrust/adapters/decoders/src/quantized_mesh_decoder.rs
    - cesiumrust/domain/tileset/src/content_decoder.rs
    - cesiumrust/domain/datasource/src/czml.rs
    - cesiumrust/domain/gltf/src/custom_shader.rs
    - cesiumrust/domain/vector/src/wkt.rs
    - cesiumrust/domain/gltf/src/binary_format.rs
---

## 1. 整体方案

CesiumRust 在 Rust 层采用 **领域内枚举错误 + 端口层统一 `PortError`** 的两层模式：
- 各 domain/adapters 模块用 `thiserror::Error` 定义语义化的 `enum XxxError`，通过 `#[error("...")]` 提供人类可读的 Display。
- 跨域/跨适配器的 I/O、GPU、缓存等外部副作用统一收敛到 `cesium-ports-driven` crate 中的 `PortError`（Network / Decode / Cache / Gpu / NotFound / Cancelled），所有 trait 方法返回 `PortResult<T>` = `Result<T, PortError>`。
- 上层 Bevy/GPUI 应用层负责把 `PortError` 或领域错误转换为 UI 提示或日志；domain 层本身不依赖任何框架。

该设计遵循六边形架构：domain 只暴露自身错误，adapter 将第三方错误（如 `serde_json::Error`、网络 IO）映射为 `PortError`，再由调用方决定如何呈现。

## 2. 关键文件与错误类型

| 位置 | 错误类型 | 用途 |
|---|---|---|
| `cesiumrust/ports/driven/src/lib.rs` | `PortError` (Network / Decode / Cache / Gpu / NotFound / Cancelled) | 所有 port trait 的统一错误类型 |
| `cesiumrust/domain/material/src/error.rs` | `MaterialError` | Fabric JSON / uniform / 子材质解析错误，映射 CesiumJS `DeveloperError` |
| `cesiumrust/adapters/network/src/lib.rs` | `NetworkError` + 实现 `TileFetcher` 返回 `PortError` | HTTP 适配器错误并上抛为 `PortError::Network` |
| `cesiumrust/adapters/decoders/src/quantized_mesh_decoder.rs` | `QuantizedMeshError` | Quantized Mesh 地形二进制解析错误（BufferTooSmall / InvalidVertexCount 等） |
| `cesiumrust/domain/tileset/src/content_decoder.rs` | `DecodeError` | b3dm/i3dm/pnts/cmpt/glTF 瓦片内容解码错误 |
| `cesiumrust/domain/datasource/src/czml.rs` | `CzmlError` | CZML JSON 解析错误（含 `#[from] serde_json::Error`） |
| `cesiumrust/domain/gltf/src/custom_shader.rs` | `ShaderError` | 自定义 shader uniform/变量作用域校验错误 |
| `cesiumrust/domain/vector/src/wkt.rs` | `WktError` | WKT 几何解析错误（UnknownType / InvalidCoordinate / MissingParenthesis 等） |
| `cesiumrust/domain/gltf/src/binary_format.rs` | `BinaryFormatError` | glTF 二进制格式错误 |

## 3. 架构与约定

### 3.1 错误类型定义规范
- 使用 `#[derive(Debug, Error)]`（或 `Clone, PartialEq` 用于可比较的错误）+ `thiserror`。
- 每个变体带 `#[error("...")]` 格式化字符串，字段名直接嵌入 `{field}`。
- 对第三方库错误使用 `#[from]` 自动转换（如 `CzmlError::Json(#[from] serde_json::Error)`）。
- 对于未使用 `thiserror` 的类型（如 `DecodeError`、`WktError`），手动实现 `Display` 和 `std::error::Error`。

### 3.2 端口边界错误收敛
`PortError` 是 domain 与 adapter 之间的契约：
- 所有 `TileFetcher` / `ImageryProvider` / `TerrainProvider` / `GpuSink` / `Decoder` / `Cache` trait 方法的返回值都是 `PortResult<T>`。
- Adapter 内部可使用自己的错误枚举（如 `NetworkError`），但在 trait 实现中必须 map 为 `PortError`。
- 这使 domain 层完全解耦于具体网络/渲染后端。

### 3.3 错误传播路径
```
Adapter (NetworkError) → PortError → Domain 业务逻辑 → Application/UI
```
例如 `HttpTileFetcher::fetch` 返回 `Err(PortError::Network(...))`，被 tileset 加载器消费后向上抛出。

### 3.4 panic 的使用场景
当前代码中 `panic!` 仅出现在两类地方：
- **Bevy 渲染适配器测试/断言**：如 `entity_render.rs` 中 `panic!("Expected Srgba color")`、`lib.rs` 中 `panic!("Expected Float32x3 positions")`，用于断言 GPU 数据布局不变。
- **示例/演示代码**：`application/cesium-app` 中 `ico subdivision failed` 等 `expect` 仅在 demo 中崩溃式退出。
- **核心 domain 逻辑不使用 panic**，全部走 `Result` 返回。

### 3.5 unwrap/expect 的使用
- 测试代码广泛使用 `.unwrap()` 简化断言。
- 生产代码中 `unwrap_or_default()` 用于从配置/属性读取可选值时提供安全默认值（如 `prop.get_value(0.0).copied().unwrap_or(default)`）。
- 真正的 I/O、解析、网络调用一律返回 `Result`，不在 domain 层 unwrap。

## 4. 开发者规则

1. **新增领域错误**：在对应 domain crate 中新增 `pub enum XxxError: #[derive(thiserror::Error)]`，每个变体带 `#[error("...")]`，必要时加 `#[from]` 包装第三方错误。
2. **跨域错误**：如需跨越 adapter 边界，优先映射为 `PortError` 的某个变体（Network / Decode / Gpu / Cache / NotFound / Cancelled），不要引入新的顶层错误类型。
3. **禁止在 domain 层 panic**：domain 函数应返回 `Result`，由调用方决定是否崩溃。
4. **禁止在 domain 层 unwrap/expect**：对外部输入（JSON、二进制、网络响应）一律处理 `Result`。
5. **Display 友好**：错误消息应包含上下文信息（如 URL、字段名、期望/实际值），便于调试。
6. **测试覆盖**：对解析类错误（WKT、CZML、QuantizedMesh、b3dm/i3dm）编写单元测试，验证 `matches!(result, Err(XxxError::...))`。
7. **UI 层处理**：Bevy/GPUI 应用层捕获 `PortError` 后转换为用户可见的错误提示或降级行为（如显示占位图、重试按钮）。