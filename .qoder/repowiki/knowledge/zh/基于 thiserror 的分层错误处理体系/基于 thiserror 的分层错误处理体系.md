---
kind: error_handling
name: 基于 thiserror 的分层错误处理体系
category: error_handling
scope:
    - '**'
source_files:
    - cesiumrust/ports/driven/src/lib.rs
    - cesiumrust/adapters/network/src/lib.rs
    - cesiumrust/domain/datasource/src/czml.rs
    - cesiumrust/domain/datasource/src/geojson.rs
    - cesiumrust/domain/gltf/src/binary_format.rs
    - cesiumrust/crates/util/src/result_ext.rs
---

CesiumRust 在 Rust 重写中采用分层、类型安全的错误处理策略，以 thiserror 为核心，结合六边形架构的端口抽象实现跨边界的错误传播。

## 1. 核心系统与工具
- thiserror：所有领域与适配器层的错误枚举均使用 #[derive(Debug, Error)] + #[error(...)] 派生，提供自动 Display/Debug 实现和来源链（#[from]）
- PortError 统一端口错误：ports/driven/src/lib.rs 定义全局 PortError 枚举，作为驱动端口对外暴露的统一错误面
- ResultExt 辅助扩展：crates/util/src/result_ext.rs 提供 inspect_ok 等轻量 Result 操作符，避免引入重型 crate

## 2. 关键文件与包
- ports/driven/src/lib.rs：定义 PortError 及 PortResult<T> 别名，统一网络/解码/GPU/缓存等外部依赖的错误语义
- adapters/network/src/lib.rs：NetworkError → PortError::Network 映射，HTTP/IO/超时/取消等具体错误归一化
- adapters/decoders/src/quantized_mesh_decoder.rs：量化网格解码器专用错误
- domain/datasource/src/czml.rs：CzmlError，通过 #[from serde_json::Error] 透传 JSON 解析错误
- domain/datasource/src/geojson.rs：GeoJsonError，同上模式
- domain/gltf/src/binary_format.rs：BinaryFormatError，包含 BufferTooShort/InvalidMagic/UnsupportedVersion/Utf8Error 等结构化字段

## 3. 架构与约定
错误传播路径：底层库错误 (serde_json/std) → #[from] 包装为领域错误 (CzmlError/GeoJsonError/BinaryFormatError) → 适配器层捕获并转换为 PortError → 调用方通过 Result<T, PortError> 处理。

PortError 按外部依赖域划分变体：Network(String)、Decode(String)、Cache(String)、Gpu(String)、NotFound(String)、Cancelled。二进制格式错误携带结构化上下文（如 BufferTooShort { expected, actual }），便于诊断；网络/解码错误使用 String 承载原始消息，由上层决定展示策略；所有错误实现 Debug，测试可直接断言匹配。

panic 仅用于内部不变量破坏（如 Bevy 渲染适配器的 mesh attribute 类型检查），不用于可恢复的外部输入错误。

## 4. 开发者规则
- 领域函数返回具体错误类型：解析器/转换器使用各自 *Error 枚举，不要直接返回 String 或 anyhow::Error
- 适配器层归一到 PortError：外部依赖错误必须在适配器边界转换为 PortError，禁止泄漏到领域层
- 优先使用结构化错误字段：对需要调试信息的错误（如长度校验、魔数验证）使用具名字段而非格式化字符串
- 谨慎使用 unwrap/expect：仓库中存在多处 unwrap() 调用，但应仅限于测试代码或绝对不可能失败的内部逻辑
- 利用 thiserror 的 #[from]：让编译器自动完成底层错误到领域错误的转换，减少样板代码