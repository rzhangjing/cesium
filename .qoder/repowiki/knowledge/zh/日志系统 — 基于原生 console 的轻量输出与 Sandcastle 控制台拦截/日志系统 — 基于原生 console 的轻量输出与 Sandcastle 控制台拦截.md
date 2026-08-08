---
kind: logging_system
name: 日志系统 — 基于原生 console 的轻量输出与 Sandcastle 控制台拦截
category: logging_system
scope:
    - '**'
source_files:
    - .github/actions/update-tokens/iTwinShareUpdater.js
    - .github/actions/update-tokens/index.js
    - .github/actions/update-tokens/ionTokenDeleter.js
    - .github/actions/update-tokens/ionTokenUpdater.js
    - .github/actions/update-tokens/replacements.js
    - packages/sandcastle/src/util/ConsoleWrapper.ts
    - cesiumrust/app_out.log
    - cesiumrust/app_err.log
    - cesiumrust/test_output.txt
---

本仓库未引入第三方日志框架，JavaScript/TypeScript 引擎与示例代码统一使用浏览器原生 `console` API（`log`、`debug`、`info`、`warn`、`error`）进行输出；Rust 适配层通过标准 `println!` / `eprintln!` 以及 `.log` 文件输出调试信息。整体呈现“无中心化日志子系统”的状态：没有统一的 logger 初始化、级别控制或结构化字段规范。

**已发现的模式与约定**
- GitHub Actions 脚本（`.github/actions/update-tokens/*.js`）全部使用 `console.log` / `console.error` 直接输出构建与 token 管理流程信息，属于典型的 CLI 式日志。
- Sandcastle 示例应用通过 `packages/sandcastle/src/util/ConsoleWrapper.ts` 对 `console.log` 进行包装与拦截，将日志消息以事件形式转发给 AI Copilot 等组件，用于在沙箱中捕获并展示用户代码产生的日志。
- Rust 侧（`cesiumrust/`）采用 Bevy 生态，但未见集中式日志配置；部分示例应用直接写入 `app_out.log`、`app_err.log` 等文件，测试输出则落在 `test_output*.txt`、`specs_output.txt` 等文本文件中。

**架构与约定**
- 引擎核心（`Source/` 仅含版权头文件，实际源码由 npm workspace 的 `packages/engine` 提供）未暴露独立 logger 模块，调用方直接使用 `console.*`。
- 开发/调试阶段依赖浏览器开发者工具或 Node 控制台；生产环境需自行替换 `console` 实现（如接入 Sentry、LogRocket 等）。
- 日志级别完全依赖调用方选择对应方法，不存在全局 level 开关或过滤机制。

**开发者应遵循的规则**
1. 在引擎与 Widgets 源码中直接使用 `console.log/debug/info/warn/error`，不要引入额外日志库。
2. 避免在高频渲染路径中输出 `console.log`，以免阻塞主线程。
3. 如需在 Sandcastle 示例中捕获日志，可通过 `ConsoleWrapper` 提供的能力获取原始 `console.log` 引用并进行自定义处理。
4. Rust 侧调试优先使用 `println!` / `eprintln!` 配合 `.log` 文件，或在 CI 中收集 `*_output.txt` 产物。
5. 敏感信息（token、密钥、JWT 等）严禁写入任何日志输出——Sandcastle 的 AI 客户端注释中已明确此约束。