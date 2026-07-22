---
kind: logging_system
name: 日志系统：未实现专用日志框架，依赖浏览器 console
category: logging_system
scope:
    - '**'
---

经全面检索仓库代码，未发现 CesiumJS 实现了专用的日志系统。仓库中不存在 `log/`、`logging/` 或任何自定义 logger 模块；所有调试输出均直接使用浏览器原生的 `console.log` / `console.warn` / `console.error`（例如 GitHub Actions 脚本、Gulp 构建任务等），引擎核心代码也未暴露如 `Cesium.log` 之类的统一 API。因此本仓库不具备“结构化日志、日志级别管理、多 sink 路由”等日志系统特征，该类别不适用于此仓库。