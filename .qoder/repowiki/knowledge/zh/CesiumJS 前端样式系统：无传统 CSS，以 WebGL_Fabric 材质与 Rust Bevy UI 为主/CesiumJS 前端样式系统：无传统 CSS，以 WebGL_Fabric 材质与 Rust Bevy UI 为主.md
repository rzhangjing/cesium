---
kind: frontend_style
name: CesiumJS 前端样式系统：无传统 CSS，以 WebGL/Fabric 材质与 Rust Bevy UI 为主
category: frontend_style
scope:
    - '**'
source_files:
    - Documentation/Schemas/Fabric/Material.schema.json
    - cesiumrust/crates/ui/src/
    - cesiumrust/crates/theme/src/
    - Apps/CesiumViewer/CesiumViewer.css
---

本仓库是 CesiumJS 三维地球引擎的源码工程，其前端样式体系与传统 Web 应用的 CSS/SCSS/Tailwind 等样式方案完全不同。经过对目录结构的全面检查，该仓库中不存在任何 .css、.scss、.less 或 tailwind.config.* 文件，也未发现任何基于 CSS 的 UI 样式定义。

CesiumJS 的前端视觉表现主要通过以下两种方式实现：

1. **WebGL 渲染管线**：核心渲染通过 WebGL 着色器（Shader）和 Fabric 材质系统完成，材质定义采用 JSON Schema（Documentation/Schemas/Fabric/Material.schema.json），而非 CSS。所有图形外观由 GLSL 着色器和材质属性控制。

2. **Rust Bevy UI 适配层**：cesiumrust 子模块使用 Bevy 框架构建 Rust 版引擎，UI 部分位于 `cesiumrust/crates/ui/src/` 和 `cesiumrust/crates/theme/src/`，采用 Rust 代码定义界面布局和主题，而非 CSS。

3. **示例应用**：Apps/CesiumViewer 中的 CesiumViewer.css 仅包含极少量的基础样式（如加载动画），主要 UI 仍由 CesiumJS 的 widgets 包动态生成。

因此，对于传统的 "frontend_style" 概念（CSS 方法论、组件库、设计令牌、响应式策略），本仓库不适用。CesiumJS 的视觉风格完全由其 WebGL 渲染管线、材质系统和 Rust Bevy UI 代码决定，不属于常规的前端样式范畴。