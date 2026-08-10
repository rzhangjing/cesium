---
kind: logging_system
name: CesiumJS 日志体系：基于 console.warn 的轻量一次性告警与弃用提示
category: logging_system
scope:
    - '**'
source_files:
    - packages/engine/Source/Core/oneTimeWarning.js
    - packages/engine/Source/Core/deprecationWarning.js
    - packages/engine/Source/Core/DeveloperError.js
    - cesiumrust/Cargo.toml
    - cesiumrust/app_err.log
    - cesiumrust/app_out.log
---

## 1. 使用的系统/方法

本仓库没有引入任何第三方日志框架（如 winston、bunyan、pino、log4j、tracing、slog 等）。JavaScript 引擎侧仅使用浏览器/Node 原生的 `console` API，并通过两个内部工具函数对输出进行结构化封装：
- `packages/engine/Source/Core/oneTimeWarning.js`：通过模块级 `warnings` Map 去重，确保同一 `identifier` 只输出一次 `console.warn`。
- `packages/engine/Source/Core/deprecationWarning.js`：复用 `oneTimeWarning`，用于弃用 API 的警告。

Rust 子工程 `cesiumrust/` 同样未引入日志 crate（Cargo.toml 依赖中无 `log`/`tracing`/`env_logger`/`slog`），领域层保持无日志依赖，仅在应用层通过标准输出文件 `app_err.log`、`app_out.log` 以及测试输出文件（`test_output.txt`、`specs_output.txt`）收集运行期信息。

## 2. 关键文件

- `packages/engine/Source/Core/oneTimeWarning.js`：一次性警告核心实现，维护全局 `warnings` 表，调用 `console.warn(message ?? identifier)`。
- `packages/engine/Source/Core/deprecationWarning.js`：弃用警告包装器，要求传入 `identifier` 和 `message`，否则抛出 `DeveloperError`。
- `packages/engine/Source/Core/DeveloperError.js`：被上述两个文件在 debug 构建下用于参数校验。
- `cesiumrust/Cargo.toml`：工作区配置，确认领域层不依赖任何日志 crate。
- `cesiumrust/app_err.log`、`cesiumrust/app_out.log`：Rust 应用运行时的错误/标准输出文件。

## 3. 架构与约定

- **无集中式 logger**：不存在统一的 logger 初始化、sink 路由或 log level 管理。各模块直接调用 `console` 或通过这两个工具函数输出。
- **一次性告警模式**：`oneTimeWarning(identifier, message?)` 以 `identifier` 为键缓存是否已输出过，避免在高频循环中刷屏；跨 Web Worker 时每个 worker 独立维护该表（注释明确说明“unless it is called from multiple workers”）。
- **弃用 API 专用通道**：`deprecationWarning(identifier, message)` 强制要求两个参数，借助 `defined()` 与 `DeveloperError` 在 debug 构建下做入参校验，生产构建下通过 `//>>includeStart('debug', pragmas.debug)` 宏剔除代码。
- **预置消息常量**：`oneTimeWarning` 暴露了若干固定 key（如 `geometryOutlines`、`geometryZIndex`、`geometryHeightReference`、`geometryExtrudedHeightReference`），供上层模块以字符串标识复用，形成隐式的“字段约定”。
- **Rust 侧零依赖**：领域 crate 刻意不引入日志依赖，遵循“纯领域逻辑不应耦合 I/O”的设计原则；运行时输出由应用层或外部脚本捕获到 `.log` 文件。

## 4. 约定与约束

- **禁止直接使用 `console.log` 输出业务告警**：`oneTimeWarning` 的 JSDoc 明确要求“Use this function instead of `console.log` directly since this does not log duplicate messages”，弃用警告也通过它实现，体现统一出口约定。
- **identifier 必须提供**：`oneTimeWarning` 缺失 `identifier` 会抛 `DeveloperError`；`deprecationWarning` 缺失任一参数也会抛错——这是开发期可检测的约束。
- **调试构建裁剪**：两处均使用 Cesium 自定义的 `//>>includeStart('debug', pragmas.debug)` / `//>>includeEnd('debug')` 包裹参数校验逻辑，意味着 release 构建不会执行 `defined()` 检查，也不会产生额外开销。
- **Rust 领域层无日志依赖**：`cesiumrust/Cargo.toml` 的 `[workspace.dependencies]` 中不包含任何日志 crate，表明领域层被设计为不可观测的纯计算单元，日志属于应用装配层职责。
- **输出级别单一**：当前实现仅使用 `console.warn`，没有 info/debug/error 分级；若需新增级别，应扩展 `oneTimeWarning` 或新增同构工具函数以保持去重策略一致。

## 5. 适用性判断

该仓库存在一个极轻量的日志/告警子系统（两个 JS 工具函数 + Rust 侧无日志约定），虽不构成企业级日志框架，但确实定义了统一的告警入口、去重策略与弃用提示规范，因此本类别适用，置信度为 medium。