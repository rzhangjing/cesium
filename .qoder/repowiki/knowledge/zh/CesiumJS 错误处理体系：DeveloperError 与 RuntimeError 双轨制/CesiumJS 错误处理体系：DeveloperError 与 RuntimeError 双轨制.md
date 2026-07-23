---
kind: error_handling
name: CesiumJS 错误处理体系：DeveloperError 与 RuntimeError 双轨制
category: error_handling
scope:
    - '**'
source_files:
    - Specs/addDefaultMatchers.js
    - Specs/BadGeometry.js
    - Specs/TestWorkers/throwError.js
    - packages/engine/Source/Core/Check.d.ts
---

## 1. 使用的系统/方法

CesiumJS 采用**双轨错误类型体系**：
- `DeveloperError`：用于开发期参数校验、API 使用错误，通过 `Check` 模块集中抛出
- `RuntimeError`：用于运行期异常（如 WebGL 上下文丢失、资源加载失败）

测试套件提供专用匹配器 `toThrowDeveloperError()` 和 `toBeRejectedWithDeveloperError()` 来断言这些错误。

## 2. 关键文件与包

- **错误类型定义**：位于 `packages/engine/Source/Core/` 下的 `DeveloperError.js`、`RuntimeError.js`（从 Specs 中 `@cesium/engine` 导入可知）
- **参数校验入口**：`packages/engine/Source/Core/Check.d.ts` 声明了所有 `check` 函数会抛出 `DeveloperError`
- **测试辅助**：`Specs/addDefaultMatchers.js` 提供 `toThrowDeveloperError` / `toBeRejectedWithDeveloperError` 等 Jasmine 扩展
- **测试数据**：`Specs/BadGeometry.js`、`Specs/TestWorkers/throwError.js` 演示如何抛出 `RuntimeError`/`Error`

## 3. 架构与约定

- **开发期错误**：所有 API 入参校验统一通过 `Check` 模块抛出 `DeveloperError`，便于在开发阶段快速定位调用方问题
- **运行期错误**：I/O、WebGL、网络等不可恢复异常抛出 `RuntimeError`，由上层 Promise `.catch` 或全局 `unhandledrejection` 处理
- **异步错误**：Promise 拒绝时返回 `DeveloperError`，测试通过 `toBeRejectedWithDeveloperError` 断言
- **Worker 线程**：`Specs/TestWorkers/` 下 Worker 直接 throw `Error`，由主进程捕获并转为 Cesium 错误类型

## 4. 开发者应遵循的规则

1. **API 参数校验**：使用 `Check` 模块而非手动 `if (x === undefined) throw new DeveloperError(...)`
2. **区分错误语义**：参数/用法错误用 `DeveloperError`，运行时故障用 `RuntimeError`
3. **异步路径**：Promise 拒绝时抛出 `DeveloperError`，以便测试统一断言
4. **测试覆盖**：对可能抛错的分支使用 `toThrowDeveloperError()` / `toBeRejectedWithDeveloperError()` 断言
5. **避免裸 Error**：库代码不应直接 `throw new Error(...)`，应使用上述两类错误类型