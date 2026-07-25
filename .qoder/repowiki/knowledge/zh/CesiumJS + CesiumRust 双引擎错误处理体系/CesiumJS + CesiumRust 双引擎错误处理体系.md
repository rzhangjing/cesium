---
kind: error_handling
name: CesiumJS + CesiumRust 双引擎错误处理体系
category: error_handling
scope:
    - '**'
source_files:
    - cesiumrust/ports/driven/src/lib.rs
    - cesiumrust/adapters/decoders/src/quantized_mesh_decoder.rs
    - cesiumrust/adapters/network/src/lib.rs
    - cesiumrust/crates/util/src/result_ext.rs
---

该仓库包含两个独立的代码库：CesiumJS（JavaScript）和 CesiumRust（Rust），它们在错误处理上采用不同的策略。

## Rust 部分（CesiumRust）

### 核心模式
- **thiserror 枚举错误**：使用 `#[derive(Error)]` 定义结构化错误类型，如 `QuantizedMeshError`、`NetworkError`、`PortError`
- **Result 传播**：所有可能失败的函数返回 `Result<T, E>`，通过 `?` 操作符向上层传播
- **端口抽象错误**：`PortError` 统一了网络、解码、缓存、GPU、NotFound、Cancelled 等外部依赖错误
- **panic 用于不可恢复错误**：仅在数据格式预期不匹配时使用（如颜色格式、顶点位置类型）

### 关键文件
- `cesiumrust/ports/driven/src/lib.rs`：定义 `PortError` 枚举和 `PortResult<T>` 别名
- `cesiumrust/adapters/decoders/src/quantized_mesh_decoder.rs`：地形解码错误示例
- `cesiumrust/adapters/network/src/lib.rs`：网络适配器错误处理
- `cesiumrust/crates/util/src/result_ext.rs`：`ResultExt` trait 提供 `inspect_ok` 辅助方法

### 架构约定
- 领域层通过端口接口与外部系统交互，错误在端口边界统一为 `PortError`
- 适配器层将具体实现错误转换为 `PortError` 变体
- 应用层根据 `PortError` 类型进行差异化处理（重试、降级、用户提示）

## JavaScript 部分（CesiumJS）

### 现状
- 当前仓库中 JavaScript 源代码仅包含版权头文件，实际引擎代码位于 `packages/engine` 目录
- GitHub Actions 脚本使用标准的 `throw new Error()` 和 `try-catch` 模式
- 未发现专门的错误处理框架或统一的错误类型定义

### 建议
- 考虑引入类似 Rust 的结构化错误类型
- 建立 Promise 错误的统一处理模式
- 添加错误日志记录和监控集成

## 跨语言一致性
- Rust 侧的错误处理较为完善，遵循现代 Rust 最佳实践
- JavaScript 侧需要建立与 Rust 侧对等的错误处理机制
- 建议在 FFI 边界明确错误传播策略，避免 panic 跨越语言边界