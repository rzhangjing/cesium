---
kind: logging_system
name: CesiumJS 日志系统：基于 console.warn/oneTimeWarning 的轻量级警告机制
category: logging_system
scope:
    - '**'
source_files:
    - packages/engine/Source/Core/oneTimeWarning.js
    - packages/engine/Source/Core/DeveloperError.js
    - packages/engine/Source/Renderer/ShaderProgram.js
    - packages/engine/Source/Core/ITwinPlatform.js
    - packages/engine/Source/Core/IonResource.js
    - Apps/CesiumViewer/CesiumViewer.js
    - Tools/rollup-plugin-strip-pragma/index.js
---

## 1. 使用的系统/方法

CesiumJS 引擎（`packages/engine/Source`）没有引入第三方日志框架，也没有统一的 `Log` 模块。代码中直接通过浏览器原生 `console` API 输出日志，主要使用 `console.warn`、`console.error` 等。仓库还提供了一个内部工具函数 `oneTimeWarning`（位于 `packages/engine/Source/Core/oneTimeWarning.js`），用于在循环或高频路径中避免重复打印相同警告。

此外，构建脚本 `Tools/rollup-plugin-strip-pragma/index.js` 支持 `//>>includeStart('debug', ...)` / `//>>includeEnd('debug')` 注释标记，可将调试用日志在发布构建中剥离，这是 Cesium 自定义的“条件编译”机制而非运行时日志级别控制。

## 2. 关键文件与位置

- `packages/engine/Source/Core/oneTimeWarning.js` — 唯一封装的日志辅助函数，提供去重警告能力，并通过预定义常量暴露常见一次性警告文案。
- `packages/engine/Source/Core/DeveloperError.js` — 抛出结构化错误（非日志），但常与警告配合用于异常路径。
- `packages/engine/Source/Core/deprecationWarning.js` — 弃用警告（若存在）。
- `packages/engine/Source/Renderer/ShaderProgram.js` — 着色器编译/链接失败时通过 `console.error` 输出详细日志（含源码片段）。
- `packages/engine/Source/Core/ITwinPlatform.js`、`IonResource.js` 等 — 在网络/平台调用失败时使用 `console.error` 输出错误码与消息。
- `Apps/CesiumViewer/CesiumViewer.js` — 示例应用层捕获并 `console.error` 用户可见的错误信息。
- `Tools/rollup-plugin-strip-pragma/index.js` — 构建期剥离 `debug` 块中的日志/断言代码。

## 3. 架构与约定

- **无集中式 Logger**：各模块直接调用 `console.*`，不存在全局 logger 实例、日志级别枚举或 sink 路由。
- **去重警告模式**：`oneTimeWarning` 维护一个 `warnings` 字典，以字符串标识符为键，确保同一标识符仅输出一次；适用于高频触发但语义相同的警告（如几何轮廓不支持地形等场景）。
- **调试代码剥离**：通过 `//>>includeStart('debug', pragmas.debug)` 包裹的 `console.log/debug` 等语句会在发布构建中被 Rollup 插件移除，从而在生产环境零开销。
- **错误 vs 警告**：可恢复的异常路径使用 `console.warn` + `oneTimeWarning`；不可恢复的错误使用 `throw new DeveloperError(...)` 或 `console.error`。

## 4. 约定与约束

- 所有核心库均直接依赖浏览器 `console` API，不引入外部日志库。
- 高频路径上的重复警告必须改用 `oneTimeWarning(identifier, message)`，以避免控制台被刷屏（见 `oneTimeWarning.js` 的 JSDoc 说明）。
- 调试用日志应放在 `//>>includeStart('debug', ...)` / `//>>includeEnd('debug')` 块内，以便发布构建自动剔除。
- 已定义的废弃行为（如 geometry outlines 与 terrain clamping、zIndex 与 heightReference 等）通过 `oneTimeWarning` 的预置常量统一输出，保证提示文案一致。
- 着色器编译/链接失败等渲染管线错误统一通过 `console.error` 输出，并附带着色器源码，便于开发者定位问题。

总体而言，该仓库的日志系统是**极简且分散的**：核心库直接使用 `console.*`，辅以 `oneTimeWarning` 做去重，并通过构建期 strip pragma 实现调试日志裁剪，没有运行时日志级别、结构化字段或远程 sink 等高级特性。