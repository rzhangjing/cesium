---
kind: frontend_style
name: CesiumJS 前端样式体系：CSS Modules + 双主题（深色/浅色）+ 组件级样式聚合
category: frontend_style
scope:
    - '**'
source_files:
    - packages/widgets/Source/widgets.css
    - packages/widgets/Source/shared.css
    - packages/widgets/Source/lighterShared.css
    - packages/engine/Source/Widget/CesiumWidget.css
    - packages/engine/Source/Widget/lighter.css
    - packages/widgets/Source/Animation/Animation.css
    - packages/widgets/Source/Animation/lighter.css
    - Apps/CesiumViewer/CesiumViewer.css
---

## 1. 系统/方法概述

CesiumJS 的前端 UI 样式采用**纯 CSS + 命名空间前缀**的传统方案，没有使用 Sass/Less、CSS-in-JS、Tailwind、PostCSS 或任何现代样式框架。样式以独立的 `.css` 文件组织在 `packages/widgets/Source/*` 与 `packages/engine/Source/Widget/` 中，通过 `@import` 聚合到入口 `widgets.css`，再由应用页面（如 `Apps/CesiumViewer/CesiumViewer.css`）引入。

主题系统基于**全局 CSS 类名切换**：默认是深色主题，通过在根元素添加 `cesium-lighter` 类来切换到浅色主题。每个组件的样式都提供一份“基础版”和一份 `lighter.css` 覆盖版，后者用 `.cesium-lighter .xxx` 选择器覆盖颜色、边框等视觉属性。

## 2. 关键文件

- `packages/widgets/Source/widgets.css` — 所有 widget 样式的统一入口，`@import` 聚合 shared、各子组件及 engine 中的 CesiumWidget 样式。
- `packages/widgets/Source/shared.css` — 全局共享样式：`.cesium-button`、`.cesium-toolbar-button`、`.cesium-performanceDisplay` 等通用控件样式。
- `packages/widgets/Source/lighterShared.css` — 浅色主题下对共享控件的颜色覆盖。
- `packages/engine/Source/Widget/CesiumWidget.css` — 引擎核心 Widget 容器、错误面板等样式。
- `packages/engine/Source/Widget/lighter.css` — 浅色主题下对引擎 Widget 的覆盖。
- `packages/widgets/Source/Animation/Animation.css` 与 `Animation/lighter.css` — 时间轴动画控件的完整样式实现，是主题模式的典型范例。
- `Apps/CesiumViewer/CesiumViewer.css` — 示例应用入口样式，演示如何 `@import` widgets.css 并设置全屏 canvas 背景。

## 3. 架构与约定

### 命名空间与前缀
所有 Cesium 自定义样式类均以 `cesium-` 为前缀（如 `.cesium-widget`、`.cesium-button`、`.cesium-animation-themeNormal`），避免与宿主页面样式冲突。这是仓库内唯一的全局命名约定，由样式文件自身强制体现。

### 组件级样式隔离
每个 widget 拥有独立目录与 CSS 文件（`Animation/`、`BaseLayerPicker/`、`Geocoder/`、`InfoBox/`、`Timeline/`、`Viewer/` 等），并通过 `widgets.css` 集中 `@import` 聚合。新增组件需在此处注册导入。

### 双主题机制
- **深色主题（默认）**：定义在 `*.css` 文件中，使用深灰背景（`#303336`）、亮色文字（`#edffff`）、蓝色高亮（`#48b`）。
- **浅色主题**：定义在对应 `lighter.css` 文件中，通过 `.cesium-lighter` 后代选择器覆盖颜色变量。例如 `.cesium-lighter .cesium-button` 将背景改为 `#e2f0ff`，文字改为 `#111`。
- 主题切换由宿主页面在根节点添加/移除 `cesium-lighter` 类完成，样式层面无 JS 逻辑。

### 颜色与字体
- 颜色以硬编码十六进制值直接写入 CSS，未使用 CSS 自定义属性（CSS Variables）或设计令牌文件。
- 字体统一使用 `sans-serif`，错误面板使用 `Open Sans, Verdana, Geneva, sans-serif`；字体资源位于 `Specs/Data/Fonts/`，但引擎运行时不依赖它。

### SVG/内联样式混合
部分交互控件（如 Animation 时间轴）使用 SVG 路径 + CSS 类组合渲染，样式同时作用于 `<svg>` 内的 `<path>`、`<text>`、`<line>` 等元素（如 `.cesium-animation-svgText`、`.cesium-animation-shuttleRingBack`）。

## 4. 约定与约束

- **无前处理器**：仓库中不存在 `.scss`、`.less`、`postcss.config.*`、`tailwind.config.*` 等文件，所有样式均为原生 CSS。
- **无 CSS 模块化**：不使用 CSS Modules、CSS-in-JS 或 Shadow DOM 隔离，依赖 `cesium-` 前缀命名空间规避冲突。
- **主题扩展必须成对**：新增样式应遵循“基础样式 + lighter 覆盖”模式，确保浅色主题可用。
- **按钮状态一致**：`.cesium-button` 定义了统一的 `normal/hover/active/disabled/focus` 状态样式，新控件应复用该基类而非重新定义。
- **性能显示面板固定位置**：`.cesium-performanceDisplay` 固定在右上角（`top: 50px; right: 10px`），属于调试工具的标准布局。
- **构建阶段不转换 CSS**：从 `gulpfile.js`、`scripts/build.js` 及 `package.json` 可见，构建流程主要处理 JS/TypeScript 打包与文档生成，CSS 以原始文件形式分发，未被预处理或压缩。
- **测试与示例引用方式**：示例页面通过 `<link>` 或 `@import` 引入 `widgets.css` 作为单一入口，新增 widget 样式需同步更新该聚合文件。

## 5. 评估

该样式体系简单直接、易于理解，适合一个以 WebGL 渲染为核心的地理信息引擎——UI 仅包含少量工具栏与调试面板。缺点是缺乏设计令牌、无法按组件热重载、主题定制需要复制整个 `lighter.css` 覆盖链，且随着 widget 数量增长，`widgets.css` 的 `@import` 列表可能成为维护瓶颈。对于 CesiumJS 的定位而言，这是一个务实但非现代化的 CSS 组织方式。