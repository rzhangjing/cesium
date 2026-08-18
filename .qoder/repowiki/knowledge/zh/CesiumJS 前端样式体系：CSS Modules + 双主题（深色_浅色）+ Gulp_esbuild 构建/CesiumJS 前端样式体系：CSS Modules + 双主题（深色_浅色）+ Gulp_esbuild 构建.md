---
kind: frontend_style
name: CesiumJS 前端样式体系：CSS Modules + 双主题（深色/浅色）+ Gulp/esbuild 构建
category: frontend_style
scope:
    - '**'
source_files:
    - packages/widgets/Source/widgets.css
    - packages/widgets/Source/shared.css
    - packages/widgets/Source/lighter.css
    - packages/widgets/Source/lighterShared.css
    - packages/widgets/Source/Animation/Animation.css
    - packages/widgets/Source/Animation/lighter.css
    - packages/engine/Source/Widget/CesiumWidget.css
    - Apps/CesiumViewer/CesiumViewer.css
    - gulpfile.apps.js
    - gulpfile.js
---

## 1. 整体方案

CesiumJS 的前端样式采用**纯 CSS + CSS @import 聚合**的方式，没有使用 Sass/Less、Tailwind 或任何 CSS-in-JS 框架。样式以 `.css` 文件形式存在于 `packages/widgets/Source` 与 `packages/engine/Source/Widget` 中，通过一个入口 `widgets.css` 统一 import 所有组件样式，再由应用层按需引入。

- **主题系统**：提供两套主题——默认深色主题和可选的“lighter”浅色主题。深色主题由 `shared.css` / `widgets.css` 定义；浅色主题通过 `lighterShared.css` 以及各组件下的 `lighter.css` 覆盖同名类名实现，宿主页面只需给根节点添加 `cesium-lighter` 类即可切换。
- **命名空间**：所有样式类均以 `cesium-` 前缀命名（如 `.cesium-button`、`.cesium-widget`、`.cesium-animation-*`），避免与宿主页面冲突。
- **字体**：默认使用 `sans-serif`；错误面板等特定 UI 显式指定 `"Open Sans", Verdana, Geneva, sans-serif`。

## 2. 关键文件与包

| 路径 | 作用 |
|---|---|
| `packages/widgets/Source/widgets.css` | 组件样式总入口，按组件目录逐一 `@import` |
| `packages/widgets/Source/shared.css` | 全局共享样式（`.cesium-button`、`.cesium-toolbar-button`、性能显示等） |
| `packages/widgets/Source/lighter.css` | 浅色主题入口，`@import` 各组件的 `lighter.css` |
| `packages/widgets/Source/lighterShared.css` | 浅色主题下对共享样式的覆盖 |
| `packages/widgets/Source/<组件>/` | 每个 UI 组件（Animation、BaseLayerPicker、Geocoder、InfoBox、Timeline、Viewer 等）各自拥有独立的 `.css` 与可选 `lighter.css` |
| `packages/engine/Source/Widget/CesiumWidget.css` | 引擎核心 Widget 容器、错误面板等基础样式 |
| `Apps/CesiumViewer/CesiumViewer.css` | 示例应用入口样式，仅 `@import` 两个 widgets 入口并设置全屏 canvas |
| `gulpfile.apps.js` | 构建 CesiumViewer 时把 `CesiumViewer.css` 作为 esbuild 入口之一，并把 InfoBox 描述样式单独打包到 `Build/CesiumViewer/Widgets/` |
| `gulpfile.js` | Karma 测试配置中将 `packages/engine/Source/Widget/*.css` 标记为 `included: false`，通过代理 `/base/Build/CesiumUnminified/Widgets/CesiumWidget/` → `/base/packages/engine/Source/Widget/` 在测试运行时直接引用源码 CSS |

## 3. 架构与约定

- **组件化组织**：每个 widget 是一个独立目录，包含 JS 逻辑与同名 `.css`，由 `widgets.css` 集中聚合。新增组件需在 `widgets.css` 中添加一行 `@import url(./Xxx/Xxx.css);`。
- **主题覆盖模式**：不通过 CSS 变量或预处理器继承，而是通过外层容器类 `.cesium-lighter` 提高选择器优先级来覆盖深色主题样式。例如 `shared.css` 定义 `.cesium-button { background: #303336; }`，`lighterShared.css` 用 `.cesium-lighter .cesium-button { background: #e2f0ff; }` 覆盖。
- **SVG 内联样式**：Animation 等复杂控件大量使用 SVG `<path>`、`<rect>`，并通过 CSS `fill`/`stroke` 控制颜色，配合 `user-select: none` 禁用文本选中。
- **构建集成**：
  - 生产构建使用 esbuild：`Apps/CesiumViewer/CesiumViewer.css` 作为 entry point 被 bundling 并 minify。
  - 测试环境通过 Karma proxy 将 `Build/CesiumUnminified/Widgets/CesiumWidget/` 映射回源码目录，使调试时可直接加载未合并的 CSS。
  - `gulpfile.js` 还复制 `node_modules/prismjs/themes/prism.min.css` 与 `Tools/jsdoc/cesium_template/static/styles/prism.css` 用于文档站点。

## 4. 约定与约束

- **类名前缀**：所有 Cesium 输出的 UI 类名必须以 `cesium-` 开头，确保不与宿主页面样式冲突（从 `shared.css`、`CesiumWidget.css`、`Animation.css` 等文件中可观察到一致的前缀约定）。
- **主题切换方式**：宿主页面需手动给包含 Cesium 的容器添加 `cesium-lighter` 类以启用浅色主题；该约定由各组件的 `lighter.css` 文件共同保证生效。
- **无 CSS 变量/预处理**：仓库中未发现 CSS 自定义属性、SCSS/Less 文件或 Tailwind 配置；样式是静态 CSS，通过 `@import` 组合。
- **响应式策略**：仅在根 `index.html` 中使用 `@media (prefers-color-scheme: dark)` 做简单的暗色模式适配；UI 本身没有媒体查询断点，布局依赖绝对定位与百分比尺寸以适应全屏 Canvas。
- **资源处理**：构建脚本将 `.gif`、`.png` 等图片以 text loader 处理；CSS 中的背景图（如 `ajax-loader.gif`）通过相对路径引用，由 gulp/esbuild 原样复制到输出目录。
- **测试隔离**：Karma 配置明确排除 `packages/engine/Source/Widget/*.css` 的自动 inclusion，转而通过 URL 代理注入，避免测试环境与生产构建行为不一致。
