---
kind: error_handling
name: CesiumJS 错误处理体系：JavaScript 引擎与 Rust 适配层的统一策略
category: error_handling
scope:
    - '**'
source_files:
    - Specs/MockImageryProvider.js
    - Specs/MockTerrainProvider.js
    - Specs/TestWorkers/throwError.js
    - cesiumrust/domain/**/*.rs
    - cesiumrust/adapters/**/*.rs
---

## 错误处理系统概述

CesiumJS 采用双语言错误处理架构：JavaScript/TypeScript 引擎使用自定义错误类型和 Promise 模式，Rust 适配层遵循 Rust 的 Result 惯用法。

## JavaScript 引擎错误处理

### 核心错误类型
- **DeveloperError**: 开发时错误，用于参数验证失败、API 使用不当等可恢复的错误
- **RuntimeError**: 运行时错误，用于网络请求失败、资源加载错误等运行时异常
- **AbortError**: 操作被中止时的错误类型

### 错误传播模式
- 同步函数通过 `throw new Error()` 抛出异常
- 异步函数返回 Promise，通过 `.catch()` 或 `try/catch` 处理
- 测试框架中广泛使用 `reject()` 和 `Promise.reject()` 进行错误断言

### 关键文件
- `Specs/MockImageryProvider.js`: 模拟数据提供者中的错误处理示例
- `Specs/MockTerrainProvider.js`: 地形提供者的错误处理模式
- `Specs/TestWorkers/throwError.js`: Worker 线程中的错误传播

## Rust 适配层错误处理

### Rust 惯用法
- 使用 `Result<T, E>` 类型进行错误处理
- 通过 `?` 操作符进行错误传播
- 自定义错误类型实现 `std::error::Error` trait

### 六边形架构中的错误边界
- 领域层（domain）定义纯 Rust 错误类型
- 适配器层（adapters）负责将底层错误转换为领域错误
- 应用层（application）处理用户可见的错误信息

## 统一约定

### 错误分类原则
1. **开发者错误**（DeveloperError）: API 使用不当，应通过静态分析工具检测
2. **运行时错误**（RuntimeError）: 外部依赖失败，需要优雅降级
3. **业务错误**: 特定领域的业务逻辑错误

### 最佳实践
- 避免在核心路径中使用 `panic!`，优先使用 `Result` 类型
- 为每个模块定义明确的错误类型层次结构
- 在网络请求、文件 I/O 等操作中添加适当的错误上下文
- 使用错误链（error chains）保留完整的错误历史

### 测试策略
- 单元测试覆盖正常路径和错误路径
- 集成测试验证错误传播的正确性
- 使用 mock 对象模拟各种失败场景