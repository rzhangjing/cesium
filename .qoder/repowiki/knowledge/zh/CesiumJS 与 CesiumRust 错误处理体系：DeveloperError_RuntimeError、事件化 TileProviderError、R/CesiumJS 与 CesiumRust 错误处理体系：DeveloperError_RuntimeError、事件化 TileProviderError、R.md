---
kind: error_handling
name: CesiumJS 与 CesiumRust 错误处理体系：DeveloperError/RuntimeError、事件化 TileProviderError、Result 与 panic
category: error_handling
scope:
    - '**'
source_files:
    - packages/engine/Source/Core/DeveloperError.js
    - packages/engine/Source/Core/RuntimeError.js
    - packages/engine/Source/Core/assert.js
    - packages/engine/Source/Core/formatError.js
    - packages/engine/Source/Core/RequestErrorEvent.js
    - packages/engine/Source/Core/TileProviderError.js
    - cesiumrust/domain/resource/src/lib.rs
    - cesiumrust/domain/provider/src/imagery_provider.rs
    - cesiumrust/adapters/bevy-render/src/imagery/tile_loader.rs
    - cesiumrust/adapters/bevy-render/src/terrain/tile_loader.rs
---

## 1. 总体方案

本仓库包含两套并行实现，错误处理策略分别遵循各自语言约定：

- **CesiumJS（JavaScript）**：基于自定义 `Error` 子类 + 事件对象（`RequestErrorEvent`、`TileProviderError`）+ 统一断言/格式化工具。
- **CesiumRust（Rust）**：以 `Result<T, E>` 为主流返回类型，测试/适配器层使用 `panic!` / `unwrap()`，尚未发现统一的领域级 Error enum。

两者在“网络/瓦片加载失败”这一高频场景上形成了跨语言的对齐：JS 侧通过 `TileProviderError.reportError` 上报事件，Rust 侧通过 `Result<String>` 或 `Option` 表达失败。

## 2. 关键文件与包

### JavaScript（packages/engine/Source/Core）
| 文件 | 职责 |
|---|---|
| `DeveloperError.js` | 开发者错误（参数非法、调用方 bug），文档明确“不应被捕获” |
| `RuntimeError.js` | 运行时错误（内存不足、着色器编译失败等），调用方应准备捕获 |
| `assert.js` | 统一断言入口，失败时抛 `DeveloperError` |
| `formatError.js` | 将任意 error 对象格式化为 `name: message\nstack` 字符串 |
| `RequestErrorEvent.js` | HTTP 请求失败的**事件对象**（statusCode / response / responseHeaders） |
| `TileProviderError.js` | Imagery/Terrain Provider 的**事件化错误**，含 x/y/level/timesRetried/retry/error 字段及 `reportError` / `reportSuccess` 静态方法 |

### Rust（cesiumrust/domain）
| 文件 | 职责 |
|---|---|
| `domain/provider/src/imagery_provider.rs` | 定义多种 ImageryProvider 结构体，URL 生成函数不抛错，仅返回 String；错误由上层调度 |
| `domain/resource/src/lib.rs` | 定义 `RequestState::{Unissued, Issued, Active, Received, Failed, Cancelled}` 状态机，用 `Option<RequestId>` 表示调度拒绝 |
| `adapters/bevy-render/src/imagery/tile_loader.rs` | 网络/解码错误以 `Result<(Vec<u8>, u32, u32), String>` 形式返回 |
| `adapters/bevy-render/src/terrain/tile_loader.rs` | 地形解码错误以 `Result<QuantizedMeshTerrainData, String>` 形式返回 |

## 3. 架构与约定

### JS 错误分层
1. **开发期错误**：`assert(condition, msg)` → `throw new DeveloperError(msg)`。用于参数校验、内部不变量检查。文档强调这类异常“should never be caught”，应由调用方避免触发。
2. **运行期错误**：`throw new RuntimeError(message)`。用于可恢复的外部故障（如资源不可用），调用方应 try/catch。
3. **Provider 错误事件**：`TileProviderError.reportError(previousError, provider, event, message, x, y, level, errorDetails)` 会复用同一个 `TileProviderError` 实例并递增 `timesRetried`；若无监听器则回退到 `console.log` 输出 `formatError(message)`。成功时用 `reportSuccess` 重置计数。
4. **HTTP 请求错误**：构造 `RequestErrorEvent(statusCode, response, responseHeaders)` 作为事件载荷，而非抛出异常。
5. **错误格式化**：`formatError(object)` 优先取 `name` + `message` + `stack`，否则回退 `toString()`。

### Rust 错误风格
- 领域层（`domain/*`）目前以纯数据结构和 URL 生成为主，不主动抛错；失败通过 `Option`（如 `TimeDynamicImagery::get_tile_url` 返回 `Option<String>`）或 `Result` 表达。
- 适配器层（`adapters/*`）在网络/IO 处返回 `Result<..., String>`，测试中大量使用 `.unwrap()` 和 `panic!` 快速失败。
- 尚未发现统一的 `thiserror`/`anyhow` 错误枚举；错误信息以 `String` 直接传播。

### 跨语言对齐点
- JS 的 `RequestState` 与 Rust 的 `RequestState` 枚举一一对应（Unissued/Issued/Active/Received/Failed/Cancelled）。
- JS 的 `TileProviderError.retry` 标志位对应 Rust 中由上层决定是否重试的策略。
- 两者都区分“调用方 bug”（JS 的 `DeveloperError`，Rust 的 `panic!` 在测试中）与“外部故障”（JS 的 `RuntimeError`/事件，Rust 的 `Result`）。

## 4. 约定与约束

| 规则 | 来源/证据 |
|---|---|
| 参数/不变量校验统一走 `assert.js`，失败抛 `DeveloperError` | `assert.js` 注释：“Checks that a condition is truthy, throwing a specified message if condition fails.” |
| `DeveloperError` 不应被捕获，应由调用方避免触发 | `DeveloperError.js` JSDoc：“This exception should only be thrown during development; ... This exception should never be caught” |
| 可能发生的运行时故障抛 `RuntimeError`，调用方应准备捕获 | `RuntimeError.js` JSDoc：“If a function may throw this exception, the calling code should be prepared to catch it.” |
| Imagery/Terrain Provider 的错误必须通过 `TileProviderError.reportError` 上报，以便追踪重试次数 | `TileProviderError.js` 注释：“Reports an error in an ImageryProvider or TerrainProvider by raising an event... also tracks the number of times the operation has been retried.” |
| 无监听器时的 Provider 错误降级为 `console.log` | `TileProviderError.reportError` 分支：`else if (defined(provider)) { console.log(...) }` |
| HTTP 请求错误使用 `RequestErrorEvent` 事件对象，而非抛异常 | `RequestErrorEvent.js` 注释：“An event that is raised when a request encounters an error.” |
| Rust 领域层失败优先用 `Option`/`Result` 而非 `panic!` | `TimeDynamicImagery::get_tile_url` 返回 `Option<String>`；`RequestScheduler::schedule` 返回 `Option<RequestId>` |
| Rust 测试/适配器层允许 `panic!`/`unwrap()` 快速失败 | 多处 `panic!("Expected ...")`、`.unwrap()` 出现在 `adapters/bevy-render` 测试代码中 |

## 5. 缺失与待完善

- Rust 侧缺少统一的领域级错误类型（如 `thiserror` 定义的 `enum XxxError`），当前错误以 `String` 散落传播，不利于模式匹配与错误分类。
- JS 侧 `DeveloperError`/`RuntimeError` 是简单构造函数，未形成继承层次（例如没有 `ValidationError`、`NetworkError` 等细分类型）。
- 未发现全局错误中间件或统一错误处理器（JS 依赖各 Provider 自行 raise Event；Rust 依赖调用链逐层 `match`）。