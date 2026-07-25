---
kind: error_handling
name: CesiumJS 与 CesiumRust 双栈错误处理体系
category: error_handling
scope:
    - '**'
source_files:
    - packages/engine/Source/ThirdParty/Workers/basis_transcoder.js
    - packages/engine/Source/Core/CesiumTerrainProvider.js
    - Specs/MockTerrainProvider.js
    - Specs/getWebGLStub.js
    - cesiumrust/crates/geospatial/src/error.rs
    - cesiumrust/crates/provider/src/error.rs
    - cesiumrust/application/cesium-app/src/main.rs
---

本仓库包含两个独立代码库：浏览器端 CesiumJS（JavaScript）与基于 Bevy 的 Rust 重写子项目 cesiumrust/。两者采用完全不同的错误处理范式，以下分别说明。

## JavaScript 侧（CesiumJS 引擎）

- 自定义异常类：引擎通过 extendError 工具函数派生业务错误类型，如 InternalError、BindingError、UnboundTypeError 等，均继承自原生 Error，并统一提供 name、message、stack 属性。这些类型在 embind 绑定层及核心模块中抛出，用于区分运行时断言失败、类型绑定错误、未绑定类型调用等场景。
- WASM 边界错误：basis_transcoder.js 作为 Emscripten 生成的 WASM 胶水代码，使用 abort() 将底层错误包装为 WebAssembly.RuntimeError 并通过 Promise reject 传播；同时定义 ExitStatus 表示正常退出码。
- 测试辅助：Specs 中通过 MockTerrainProvider、getWebGLStub 等模拟对象暴露 previousError、getError 等字段，配合 Jasmine 断言验证 Provider 的错误上报路径。
- 约定：API 层不直接抛出自定义 Error，而是通过 Provider 的 TileProviderError.reportError 等回调机制异步上报，避免阻塞渲染循环。

## Rust 侧（CesiumRust DDD 重构）

- Result<T, E> 为主流：所有可能失败的 I/O、解析、网络请求返回 Result，由调用方 ? 或显式 match 处理。
- 自定义错误枚举：各 domain crate 定义具名错误类型（如 geospatial::error::GeospatialError、provider::error::ProviderError），通常实现 std::error::Error、Display、Debug，并使用 thiserror 简化 derive。
- 应用层统一转换：application/cesium-app/src/main.rs 与 crates/app/ 中将领域错误转换为 UI 可展示的消息，或通过 anyhow::Result 在顶层收集上下文。
- Bevy 插件集成：错误通过 Bevy 的 Event 系统或 App::add_event 分发到 UI 层，避免在渲染线程中 panic。
- panic 策略：仅在不可恢复的内部不变量被破坏时使用 panic!（如数组越界、空指针解引用），其余错误一律走 Result。

## 跨栈协作

- JS ↔ Rust 通过 WebAssembly 导出函数交互，错误以 Result 的 Err 分支映射为 JS 侧抛出的 BindingError，再由上层捕获并转为 TileProviderError 事件。
- 构建期错误由 Gulp + ESLint + Prettier 组合检查，运行期错误由 Karma/Jasmine 用例覆盖。

开发者应遵循：
1. JS 侧优先使用 TileProviderError 等语义化错误类型，避免裸 throw new Error。
2. Rust 侧使用具名错误枚举 + thiserror，不在领域层 panic。
3. 跨语言边界用 #[wasm_bindgen] 自动转换 Result，并在 JS 端统一 catch 后上报。