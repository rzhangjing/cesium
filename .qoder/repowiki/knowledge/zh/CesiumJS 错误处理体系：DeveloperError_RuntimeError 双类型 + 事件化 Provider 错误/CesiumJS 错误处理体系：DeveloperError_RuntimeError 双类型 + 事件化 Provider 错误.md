---
kind: error_handling
name: CesiumJS 错误处理体系：DeveloperError/RuntimeError 双类型 + 事件化 Provider 错误
category: error_handling
scope:
    - '**'
source_files:
    - packages/engine/Source/Core/DeveloperError.js
    - packages/engine/Source/Core/RuntimeError.js
    - packages/engine/Source/Core/assert.js
    - packages/engine/Source/Core/Check.js
    - packages/engine/Source/Core/formatError.js
    - packages/engine/Source/Core/RequestErrorEvent.js
    - packages/engine/Source/Core/TileProviderError.js
    - packages/engine/Source/Core/Resource.js
    - packages/engine/Source/Core/Request.js
    - Specs/MockImageryProvider.js
    - Specs/MockTerrainProvider.js
    - Specs/BadGeometry.js
---

## 1. 使用的系统/方法

CesiumJS 在 `packages/engine/Source/Core` 中定义了一套自有的 JavaScript 错误体系，没有引入第三方错误库。核心思路是**用两种内置 Error 子类区分“开发者调用错误”和“运行时可恢复错误”**，并通过事件对象（`RequestErrorEvent`、`TileProviderError`）把网络/瓦片加载失败从异常流中解耦出来，由上层监听者决定重试或降级。

- 断言入口统一通过 `assert.js` 的 `assert(condition, msg)` 抛出 `DeveloperError`。
- 参数校验集中在 `Check.js`，所有 `Check.typeOf.*` 系列方法失败时抛 `DeveloperError`。
- 业务层运行期异常（如地形不可用、JSON 解析失败、HTTP 403/429 等）抛 `RuntimeError`。
- 网络请求失败不抛异常，而是构造 `RequestErrorEvent`（含 `statusCode`、`response`、`responseHeaders`），由 `Resource` / `RequestScheduler` 等组件触发事件。
- 影像/地形 Provider 的错误统一封装为 `TileProviderError`，通过 `reportError(previousError, provider, event, message, x, y, level, errorDetails)` 上报，支持 `retry` 标志与 `timesRetried` 计数。

## 2. 关键文件与包

| 文件 | 职责 |
|---|---|
| `packages/engine/Source/Core/DeveloperError.js` | 开发者错误类，`name='DeveloperError'`，文档明确“不应被捕获” |
| `packages/engine/Source/Core/RuntimeError.js` | 运行时错误类，`name='RuntimeError'`，调用方应准备捕获 |
| `packages/engine/Source/Core/assert.js` | 条件断言，失败抛 `DeveloperError` |
| `packages/engine/Source/Core/Check.js` | 类型/范围校验集合，全部抛 `DeveloperError` |
| `packages/engine/Source/Core/formatError.js` | 统一格式化 `name: message\nstack` 字符串 |
| `packages/engine/Source/Core/RequestErrorEvent.js` | HTTP 请求失败事件载荷（status、response、headers） |
| `packages/engine/Source/Core/TileProviderError.js` | Imagery/Terrain Provider 错误载体，提供 `reportError` / `reportSuccess` 静态方法 |
| `packages/engine/Source/Core/Resource.js` | 资源下载中心，使用 `retryCallback` + `retryAttempts` 实现请求级重试 |
| `packages/engine/Source/Core/Request.js` | 请求描述对象（URL、优先级、取消、状态） |

## 3. 架构与约定

### 3.1 两类异常的职责划分

- **`DeveloperError`**：用于 API 调用方传入非法参数、未定义值、越界数值等“调用方 bug”。`DeveloperError` 的 JSDoc 写明“this exception should never be caught; instead the calling code should strive not to generate it”，因此它适合在单元测试里断言，生产代码中不应 catch。
- **`RuntimeError`**：用于外部资源不可用、GPU 编译失败、服务器返回错误等“可能发生在运行时的异常”。JSDoc 强调“calling code should be prepared to catch it”，因此 Provider、Terrain 等对外暴露的方法会抛此类。

两者都继承 `Error.prototype`，设置 `name`、`message`、`stack`，并覆写 `toString()` 输出 `name: message` 加堆栈。

### 3.2 断言与参数校验的统一入口

- `assert.js` 提供 `assert(condition, msg)`，内部直接 `throw new DeveloperError(msg)`，配合 TypeScript 的 `asserts condition` 类型收窄。
- `Check.js` 提供 `Check.defined`、`Check.typeOf.func/string/number/object/boolean/bigint`、`Check.typeOf.number.lessThan/greaterThan` 等，全部抛 `DeveloperError`，形成统一的入参守卫风格。

### 3.3 非异常的错误通道：事件化

对于网络与瓦片加载这类高频且可恢复的错误，Cesium 避免抛异常，改用事件对象：

- `RequestErrorEvent`：HTTP 层错误，携带 `statusCode`、`response`、`responseHeaders`（自动解析字符串头）。
- `TileProviderError`：Imagery/Terrain 层错误，携带 `provider`、`message`、`x/y/level`、`error`、`timesRetried`、`retry`。
- `TileProviderError.reportError` 是标准上报入口：首次创建实例，后续复用同一实例递增 `timesRetried`；若无监听器则回退到 `console.log` 并使用 `formatError` 格式化消息。
- `TileProviderError.reportSuccess` 将 `timesRetried` 重置为 `-1`，表示“最近一次成功”，下次失败时重新计数。

### 3.4 请求级重试模型

`Resource` 构造函数接受 `retryCallback(resource, error)` 与 `retryAttempts`：当下载失败时调用回调，若返回 `true`（或 Promise resolve true）则按 `retryAttempts` 次数重试。这是 Cesium 对网络错误的通用恢复策略，典型用法是刷新 token 后重试。

### 3.5 测试中的错误模式

- `Specs/BadGeometry.js`、`MockImageryProvider.js`、`MockTerrainProvider.js` 等测试桩故意抛 `RuntimeError`，验证上层 Provider 能正确捕获并上报。
- `Specs/getWebGLStub.js`、`Specs/addDefaultMatchers.js` 等测试工具抛 `DeveloperError`，作为开发期断言。

## 4. 约定与约束

| 约定 | 说明 | 依据 |
|---|---|---|
| 调用方参数错误 → `DeveloperError` | 所有 `Check.*`、`assert` 均抛此类型 | `Check.js`、`assert.js` 实现 |
| 外部资源/运行时故障 → `RuntimeError` | 地形、地球定向参数、ITwin 平台等抛此类型 | `CesiumTerrainProvider.js`、`EarthOrientationParameters.js`、`ITwinPlatform.js` 多处 `throw new RuntimeError(...)` |
| 网络/瓦片错误 → 事件而非异常 | 通过 `RequestErrorEvent`、`TileProviderError` 上报 | `RequestErrorEvent.js`、`TileProviderError.js` 设计 |
| Provider 错误必须可重试 | `TileProviderError.retry` 由监听者修改，`reportError` 跟踪 `timesRetried` | `TileProviderError.reportError` 逻辑 |
| 无监听器时降级为 console.log | `TileProviderError.reportError` 在无 event 监听时 `console.log` 输出 | 同文件第 140–148 行 |
| 错误信息统一格式化 | `formatError` 优先取 `name/message/stack`，否则 fallback `toString()` | `formatError.js` |
| 不要 catch `DeveloperError` | 文档明确“should never be caught” | `DeveloperError.js` JSDoc |

该体系在 CesiumJS 引擎层高度一致：API 边界用 `DeveloperError` 快速失败，运行时路径用 `RuntimeError` 上抛，IO 路径用事件+重试，形成三层互补的错误处理策略。