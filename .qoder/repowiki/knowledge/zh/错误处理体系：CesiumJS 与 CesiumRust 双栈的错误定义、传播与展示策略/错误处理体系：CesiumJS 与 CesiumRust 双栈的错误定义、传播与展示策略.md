---
kind: error_handling
name: 错误处理体系：CesiumJS 与 CesiumRust 双栈的错误定义、传播与展示策略
category: error_handling
scope:
    - '**'
source_files:
    - packages/engine/Source/Core/CesiumError.js
    - packages/engine/Source/Core/DeveloperError.js
    - packages/engine/Source/Core/RuntimeError.js
    - packages/engine/Source/Core/Resource.js
    - packages/engine/Source/Core/RequestErrorEvent.js
    - packages/engine/Source/Core/LoadErrorEvent.js
    - cesiumrust/domain/geospatial/src/error.rs
    - cesiumrust/domain/tileset/src/error.rs
    - cesiumrust/adapters/network/src/error.rs
---

本仓库包含两套并行实现，错误处理策略分别遵循各自语言生态的约定：

## CesiumJS（JavaScript/TypeScript）
- 使用自定义错误类型与语义化错误码。核心错误类集中在 `packages/engine/Source/Core` 下的 `CesiumError`、`DeveloperError`、`RuntimeError` 等，通过抛出带明确 message 和可选 stack 的对象来传递异常。
- 网络与资源加载错误由 `Resource`、`UrlTemplateImageryProvider`、`Tileset` 等模块统一包装为 `RequestErrorEvent` / `LoadErrorEvent` 事件，供上层订阅而非直接 throw，避免中断渲染循环。
- 异步错误通过 Promise rejection 与 `Task` 对象上的 `error` 属性传播；测试中大量使用 `toThrow` / `rejects` 断言。
- 浏览器端不捕获全局 `unhandledrejection`，而是依赖各 Provider 的 `error` 事件回调；开发者可通过 `Viewer` 的 `screenSpaceEventHandler` 或自定义中间件拦截。
- 调试期使用 `defined`、`isArray` 等辅助函数配合 `DeveloperError` 做参数校验，生产构建时通过 pragma 剥离。

## CesiumRust（Rust）
- 采用标准库 `Result<T, E>` + 自定义错误枚举（如 `geospatial::error::GeospatialError`、`tileset::error::TilesetError`），通过 `?` 运算符向上传播。
- 错误类型实现 `std::error::Error`、`Display`、`Debug`，并使用 `thiserror` 派生简化样板代码。
- 异步 I/O 错误通过 `tokio::task::JoinError` 与自定义 `NetworkError` 组合，在适配器层转换为上层可消费的枚举。
- 无 `panic/recover` 用于业务流控制，仅对不可恢复的内部不变量违反使用 `panic!`；所有外部输入均先验证再构造。
- 日志与错误上报通过 `tracing` 记录结构化上下文，便于在 Bevy 应用层聚合输出。

## 跨栈协作点
- WASM 边界错误通过 `wasm-bindgen` 将 Rust `Result` 转为 JS `Error`，并在 Sandcastle 示例中以 try/catch 包裹调用。
- 测试套件（Specs）同时覆盖 JS 抛错路径与 Rust panic 场景，确保两端错误语义一致。

开发者应遵循：
- JS 侧优先使用事件模型（`error` 事件）而非 throw，避免阻塞帧循环。
- Rust 侧禁止用 `unwrap()` 处理外部输入，必须返回 `Err` 并携带上下文。
- 新增错误类型需同步更新 JSDoc 与 TypeScript 声明，保证 IDE 提示完整。