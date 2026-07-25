---
kind: error_handling
name: 错误处理体系：Rust thiserror + PortError 与 JS throw 的混合策略
category: error_handling
scope:
    - '**'
source_files:
    - cesiumrust/ports/driven/src/lib.rs
    - cesiumrust/adapters/decoders/src/quantized_mesh_decoder.rs
    - cesiumrust/adapters/network/src/lib.rs
    - Specs/TestWorkers/throwError.js
    - Specs/spec-main.js
---

## 1. 系统/方法概述
本仓库包含两套并行的代码库：CesiumJS 原生 JavaScript 测试套件（Specs）和 cesiumrust Rust 移植。两者在错误处理上采用不同但互补的策略：
- **Rust 侧**：使用 `thiserror` 定义结构化错误枚举，通过 `Result<T, E>` 传播错误，关键路径使用 `panic!` 作为不可恢复错误的最后手段。
- **JavaScript 侧**：使用标准的 `throw new Error(...)` 抛出异常，配合 Worker 中的 `createTaskProcessorWorker` 进行异步任务错误传播。

## 2. 核心文件与包
### Rust 错误类型定义
- `cesiumrust/ports/driven/src/lib.rs`：定义统一的 `PortError` 枚举和 `PortResult<T>` 类型别名，涵盖 Network、Decode、Cache、Gpu、NotFound、Cancelled 等错误类别
- `cesiumrust/adapters/decoders/src/quantized_mesh_decoder.rs`：使用 `thiserror::Error` 定义 `QuantizedMeshError`，包含 BufferTooSmall、InvalidVertexCount、InvalidTriangleCount、InvalidIndex 等具体错误
- `cesiumrust/adapters/network/src/lib.rs`：定义 `NetworkError` 枚举（HttpError、IoError、Timeout、Cancelled），并在适配器中转换为 `PortError`

### JavaScript 错误处理
- `Specs/TestWorkers/throwError.js`：测试用 Worker，演示如何通过 `createTaskProcessorWorker` 抛出错误
- `Specs/spec-main.js`、`Specs/karma.conf.cjs`：Jasmine/Karma 测试框架的错误捕获配置

## 3. 架构与约定
### Hexagonal Architecture 错误分层
```rust
// ports/driven - 领域层依赖的错误抽象
pub enum PortError {
    Network(String),
    Decode(String),
    Cache(String),
    Gpu(String),
    NotFound(String),
    Cancelled,
}
pub type PortResult<T> = Result<T, PortError>;
```

### Adapter 层错误转换
适配器将具体实现错误转换为统一的 `PortError`：
```rust
// network adapter 示例
Err(PortError::Network(format!("...")))
Err(PortError::NotFound(format!("No mock response for URL: {}", url)))
Err(PortError::Cancelled)
```

### Decoder 层专用错误
解码器使用细粒度的错误类型：
```rust
#[derive(Debug, Error)]
pub enum QuantizedMeshError {
    #[error("Buffer too small: expected at least {expected} bytes, got {actual}")]
    BufferTooSmall { expected: usize, actual: usize },
    #[error("Invalid vertex count: {0}")]
    InvalidVertexCount(usize),
    // ...
}
```

### panic! 的使用场景
仅在绝对不可能发生的编程错误时使用：
- `bevy-render` 中期望特定数据类型时的断言
- 测试代码中的 `unwrap_or_else(|e| panic!("failed to build {}: {}", type_name, e))`

## 4. 开发者应遵循的规则
1. **优先使用 Result 而非 panic**：所有可能失败的操作返回 `Result<T, E>`
2. **使用 thiserror 定义错误类型**：为每个模块定义具体的错误枚举
3. **统一端口错误**：外部接口使用 `PortError`，内部实现可定义更具体的错误类型
4. **谨慎使用 panic**：仅用于表示程序逻辑错误或调试期断言失败
5. **错误消息要包含上下文**：使用格式化字符串提供足够的诊断信息
6. **JavaScript 侧统一使用 throw**：避免使用 console.error 代替错误处理
7. **测试覆盖错误路径**：确保每个错误分支都有对应的测试用例