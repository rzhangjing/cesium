---
kind: logging_system
name: 日志系统 — 基于原生 console 的轻量输出
category: logging_system
scope:
    - '**'
---

本仓库未引入专用日志框架（如 winston、pino、log4js 等），也未在 CesiumJS 引擎源码中定义统一的 Logger 模块或结构化日志字段。核心 JavaScript/TypeScript 代码中的日志输出主要依赖浏览器/Node 原生的 `console.log`、`console.warn`、`console.error`、`console.info`、`console.debug`，以及 GitHub Actions 工作流脚本中的同类调用。

- **使用方式**：各模块直接调用 `console.*` 方法，无集中配置、无级别过滤、无统一格式化；构建产物与示例应用同样如此。
- **架构决策**：作为浏览器端 WebGL 图形库，CesiumJS 选择零运行时依赖，将诊断输出委托给宿主环境的控制台，避免为日志引入额外包体积与初始化开销。
- **开发者约定**：无需注册 logger、无需切换 sink；在需要调试时直接使用 `console.*`，错误信息通过 `console.error` 输出以便在浏览器 DevTools 中快速定位。

由于不存在独立的日志子系统，该类别在本仓库中属于“不适用”情形。