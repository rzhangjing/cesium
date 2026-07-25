---
kind: frontend_style
name: CesiumJS 测试套件的前端样式系统
category: frontend_style
scope:
    - '**'
source_files:
    - Apps/CesiumViewer/CesiumViewer.css
    - Specs/e2e/cesium.html
    - index.html
    - Specs/Data/Fonts/OpenSans-Main.css
    - Specs/SpecRunner.html
---

该仓库是 CesiumJS 的测试套件与 Rust 移植集成测试项目，前端样式系统相对简单且主要用于测试环境展示。

## 样式系统与架构

**核心样式来源：**
- 主要依赖 CesiumJS 自身的 Widgets CSS 框架（`widgets.css` 和 `lighter.css`）
- 通过 `@import` 方式引入，遵循 CesiumJS 组件库的样式约定
- 测试页面使用内联 `<style>` 标签定义基础样式

**样式组织模式：**
- Apps/CesiumViewer/ 目录下的独立 CSS 文件采用模块化命名（如 `.fullWindow`、`.loadingIndicator`）
- Specs/e2e/cesium.html 中直接嵌入样式，包含加载遮罩、工具栏和信息面板等测试专用样式
- 根级 index.html 使用内联样式并支持深色模式（`prefers-color-scheme: dark`）

**设计令牌与主题：**
- 颜色方案：黑色背景（#000）、浅灰色文字（#eee）、蓝色链接（#6dabe4）
- 字体：Open Sans（woff格式），通过 `Specs/Data/Fonts/OpenSans-Main.css` 引入
- 响应式：使用 CSS media queries 支持暗色主题切换
- 布局：绝对定位的全屏容器（`.fullSize`、`.fullWindow`）

**构建系统集成：**
- 构建脚本会处理 Widget 资源复制和 CSS 内容注入
- Sandcastle 模板系统使用 `bucket.css` 作为基础样式
- Jasmine 测试框架自带 jasmine.css 用于测试界面样式

## 开发者规范

1. **样式导入顺序**：先引入 widgets.css，再引入 lighter.css
2. **全屏容器**：使用 `.fullSize` 或 `.fullWindow` 类确保 Canvas 占满视口
3. **深色模式**：利用 `@media (prefers-color-scheme: dark)` 提供主题适配
4. **字体管理**：通过独立的 CSS 文件管理字体加载，避免重复声明
5. **测试专用样式**：保持简洁，专注于功能验证而非视觉设计