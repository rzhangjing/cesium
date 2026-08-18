---
kind: error_handling
name: CesiumJS 错误处理体系：DeveloperError/RuntimeError/事件化 Provider 错误
category: error_handling
scope:
    - '**'
source_files:
    - packages/engine/Source/Core/DeveloperError.js
    - packages/engine/Source/Core/RuntimeError.js
    - packages/engine/Source/Core/Check.js
    - packages/engine/Source/Core/RequestErrorEvent.js
    - packages/engine/Source/Core/TileProviderError.js
    - packages/engine/Source/Core/formatError.js
    - packages/engine/Source/Core/Request.js
---

## 1. 整体方案

CesiumJS（`packages/engine/Source/Core`）采用**“开发期断言 + 运行期异常 + 事件化 Provider 错误”**三层混合策略，没有统一的中间件或全局 catch 框架，而是通过一组约定良好的类型与工具函数在各模块中协作。

- **开发期参数校验**：集中放在 `Check.js`，所有非法参数调用统一抛出 `DeveloperError`。该文件提供 `Check.defined`、`Check.typeOf.func/string/number/object/boolean/bigint`、`Check.typeOf.number.lessThan/greaterThan/equals` 等 API，全部以 `throw new DeveloperError(...)` 终止执行。
- **开发者错误类型**：`DeveloperError.js` —— 继承自 `Error`，`name = 'DeveloperError'`，用于表示“调用方 bug”（如参数越界、未定义）。文档明确说明它“should only be thrown during development; it usually indicates a bug in the calling code. This exception should never be caught”。
- **运行期错误类型**：`RuntimeError.js` —— 同样继承 `Error`，`name = 'RuntimeError'`，用于“可能发生在运行时的错误”（如内存不足、shader 编译失败），调用方应准备捕获。
- **网络/Provider 错误**：不抛异常，而是通过事件对象传递：
  - `RequestErrorEvent.js`：封装 HTTP 请求失败的 `statusCode`、`response`、`responseHeaders`，由上层 Request/Resource 机制触发事件。
  - `TileProviderError.js`：封装 ImageryProvider/TerrainProvider 的错误，包含 `provider`、`message`、`x/y/level`、`timesRetried`、`retry`、`error` 等字段；并提供静态方法 `reportError(previousError, provider, event, message, x, y, level, errorDetails)` 统一上报。
- **错误格式化**：`formatError.js` 提供 `formatError(object)`，优先使用 `name: message\nstack` 格式，否则回退到 `toString()`。

## 2. 关键文件与职责

| 文件 | 职责 |
|---|---|
| `Core/DeveloperError.js` | 开发者错误类型，不可被调用方捕获 |
| `Core/RuntimeError.js` | 运行期错误类型，调用方应可捕获 |
| `Core/Check.js` | 参数校验入口，所有断言均抛 `DeveloperError` |
| `Core/RequestErrorEvent.js` | 网络请求失败的事件载体（状态码、响应、头） |
| `Core/TileProviderError.js` | 瓦片/地形/影像 Provider 错误事件载体，含重试计数与 `retry` 标志 |
| `Core/formatError.js` | 将任意 Error 对象格式化为可读字符串 |
| `Core/Request.js` | 请求抽象，配合 `RequestErrorEvent` 在失败时走事件路径 |

## 3. 架构与约定

1. **断言层 → 异常层 → 事件层** 的分工清晰：
   - 函数入口用 `Check.*` 做参数合法性检查，违反即抛 `DeveloperError`。
   - 内部逻辑遇到“外部不可控但可恢复”的问题（如 shader 编译失败、资源加载失败）抛 `RuntimeError`。
   - Provider 层（Imagery/Terrain）遇到瓦片级错误不抛异常，而是构造 `TileProviderError` 并通过事件系统上报，允许监听者设置 `retry = true` 实现重试。
2. **错误对象结构一致**：`DeveloperError` / `RuntimeError` 都暴露 `name`、`message`、`stack`，并自定义 `toString()` 输出 `name: message\nstack`，便于日志收集。
3. **Provider 错误可追踪重试次数**：`TileProviderError.reportError` 会复用上一次错误实例并递增 `timesRetried`；成功时通过 `reportSuccess` 重置为 `-1`，以便下次失败时从 0 开始计数。
4. **无全局 try/catch 兜底**：错误传播依赖调用链显式捕获 `RuntimeError` 或通过事件订阅 `RequestErrorEvent` / Provider 错误事件，不存在类似 Express middleware 的全局错误处理器。
5. **Rust 重实现（cesiumrust）** 位于仓库根目录 `cesiumrust/`，使用 Rust 原生 `Result<T, E>` 和 `std::error::Error` 体系，与 JS 引擎层的错误模型相互独立，不在本卡片范围内。

## 4. 约定与约束

- **`DeveloperError` 不应被调用方捕获**：其 JSDoc 明确要求“should never be caught”，仅应在开发阶段发现并修复调用方 bug。
- **`RuntimeError` 必须可捕获**：JSDoc 指出“calling code should be prepared to catch it”，是运行时契约的一部分。
- **Provider 错误必须经 `TileProviderError.reportError` 上报**：该方法统一管理重试计数、事件派发与 console 降级输出（无监听器时 fallback 到 `console.log`）。
- **网络错误走事件而非 Promise reject 主路径**：`RequestErrorEvent` 作为事件载荷，由上层组件订阅处理，避免阻塞渲染管线。
- **所有错误信息需可格式化**：通过 `formatError` 统一输出 `name: message\nstack`，保证日志一致性。
- **参数校验集中在 `Check.js`**：新增公共 API 时应通过 `Check.*` 进行入参校验，而不是手写 `if (!defined(x)) throw ...`，以保持错误消息风格一致。

## 5. 适用性说明

该错误处理体系覆盖 CesiumJS 核心引擎（`packages/engine/Source/Core`）的参数校验、异常类型、Provider 事件化错误以及网络请求错误，属于仓库内明确且广泛使用的模式，因此本卡片适用。