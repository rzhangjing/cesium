---
kind: frontend_style
name: CesiumJS 前端样式体系：CSS 模块化与主题系统
category: frontend_style
scope:
    - '**'
source_files:
    - packages/widgets/Source/widgets.css
    - packages/widgets/Source/shared.css
    - packages/widgets/Source/Animation/Animation.css
    - packages/widgets/Source/Animation/lighter.css
    - Apps/CesiumViewer/CesiumViewer.css
---

## 样式方法与架构

CesiumJS 采用**纯 CSS + 命名空间前缀**的轻量级样式方案，未引入 SCSS、Tailwind 等现代预处理框架。所有 UI 样式通过 `.css` 文件组织，并使用 `cesium-` 前缀避免与宿主页面冲突。

### 核心样式入口
- **聚合入口**: `packages/widgets/Source/widgets.css` 作为 widgets 包统一入口，通过 `@import` 汇总所有组件样式
- **共享基础样式**: `packages/widgets/Source/shared.css` 定义通用按钮、工具栏、性能显示等基础组件样式
- **引擎层样式**: `engine/Source/Widget/CesiumWidget.css` 提供核心地图控件样式

### 主题系统
CesiumJS 实现了**双主题系统**（深色/浅色）：
- **默认深色主题**: 在组件 CSS 中直接定义，使用 `#303336` 背景色、`#edffff` 文字色
- **浅色主题覆盖**: 通过 `lighter.css` 文件，以 `.cesium-lighter` 类名选择器覆盖默认样式
- **主题切换机制**: 应用需在根元素添加 `cesium-lighter` 类来启用浅色主题

### 组件样式组织模式
每个 UI 组件采用**独立 CSS 文件 + 命名空间前缀**的组织方式：
- `Animation/Animation.css` - 时间轴动画控件
- `BaseLayerPicker/BaseLayerPicker.css` - 底图选择器
- `Viewer/Viewer.css` - 主视图容器
- `Timeline/Timeline.css` - 时间线控件
- 其他各组件均遵循此模式

### 设计令牌与颜色约定
- **品牌色**: `#48b` (hover)、`#adf` (active)、`#aef` (边框高亮)
- **文本色**: `#edffff` (默认)、`#fff` (焦点态)、`#646464` (禁用态)
- **背景色**: `#303336` (按钮背景)、`rgba(40,40,40,0.7)` (半透明面板)
- **字体**: 全局使用 `sans-serif`，无自定义字体加载

### 构建集成
样式文件通过 Node.js 构建脚本打包到最终产物中，示例应用通过 `<link>` 标签引入聚合后的 CSS 文件。