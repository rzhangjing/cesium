---
kind: frontend_style
name: CesiumJS 与 CesiumRust 双引擎前端样式系统
category: frontend_style
scope:
    - '**'
source_files:
    - Apps/CesiumViewer/CesiumViewer.css
    - packages/widgets/Source/shared.css
    - packages/widgets/Source/widgets.css
    - packages/engine/Source/Widget/lighter.css
    - packages/sandcastle/public/templates/bucket.css
    - cesiumrust/crates/theme/src/lib.rs
---

该仓库包含两套独立的前端样式体系，分别服务于 JavaScript 版 CesiumJS 引擎和 Rust 版 CesiumRust 引擎。

## CesiumJS（JavaScript）样式体系
- **CSS 模块化组织**：样式文件按功能域拆分到 `packages/widgets/Source/<组件>/` 目录下，每个组件拥有独立的 `.css` 文件，通过 `widgets.css` 统一聚合导入。
- **共享样式基座**：`shared.css` 定义全局按钮、工具栏、性能显示等通用样式类（如 `.cesium-button`、`.cesium-toolbar-button`），所有组件复用这些基础样式。
- **主题切换机制**：通过 `lighter.css` 覆盖默认深色主题，使用 `.cesium-lighter` 命名空间选择器实现浅色变体，应用层通过 `<body class="cesium-lighter">` 切换。
- **构建集成**：Gulp 构建流程将 Prism.js 主题 CSS 等资源打包，最终输出到 `Build/CesiumUnminified/Widgets/` 目录供 HTML 引用。
- **Sandcastle 示例框架**：使用独立的 `bucket.css` 模板，基于 CSS 变量（`--bg-dark`、`--bg-lighter`、`--ring-color`）实现暗色主题，支持运行时主题切换。

## CesiumRust（Rust/GPUI）样式体系
- **集中式颜色调色板**：`crates/theme/src/lib.rs` 中的 `AppColors` 结构体统一管理所有颜色值（背景、文本、强调色、状态色、边框），采用类似 Zed 编辑器的设计。
- **字体大小预设**：`FontSizes` 结构体提供 SM(12px)、BASE(14px)、LG(16px)、XL(20px)、XXL(24px) 五种字号规范。
- **跨 crate 依赖**：ui、workspace、app 等 crate 通过 `use theme::AppColors` 引入统一的视觉规范，确保整个 Rust UI 的一致性。
- **GPUI 原生样式**：使用 GPUI 框架的 `Rgba` 类型和 `rgb!` 宏定义颜色，遵循 Rust 生态的最佳实践。

## 设计决策与约定
- **无 CSS-in-JS 方案**：CesiumJS 部分完全使用传统 CSS 文件，未引入 styled-components、emotion 等现代方案。
- **无响应式设计**：样式主要针对桌面端浏览器优化，未使用媒体查询或移动优先策略。
- **命名空间约定**：所有 Cesium 相关类名以 `cesium-` 前缀开头，避免与其他库冲突。
- **主题扩展模式**：通过追加 `lighter` 后缀的 CSS 文件覆盖默认样式，而非创建完整的新主题文件。