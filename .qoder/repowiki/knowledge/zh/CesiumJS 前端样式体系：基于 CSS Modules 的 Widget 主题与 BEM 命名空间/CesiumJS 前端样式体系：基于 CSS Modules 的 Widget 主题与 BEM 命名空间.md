---
kind: frontend_style
name: CesiumJS 前端样式体系：基于 CSS Modules 的 Widget 主题与 BEM 命名空间
category: frontend_style
scope:
    - '**'
source_files:
    - packages/widgets/Source/widgets.css
    - packages/widgets/Source/lighter.css
    - packages/widgets/Source/shared.css
    - packages/widgets/Source/lighterShared.css
    - packages/widgets/Source/InfoBox/InfoBox.css
    - packages/engine/Source/Widget/CesiumWidget.css
    - Apps/CesiumViewer/CesiumViewer.css
    - gulpfile.js
    - gulpfile.apps.js
---

## 1. 采用的样式系统

CesiumJS 的前端 UI 完全使用**原生 CSS + CSS @import 聚合**的方式组织，没有引入 Sass/Less、CSS-in-JS（如 styled-components/emotion）、Tailwind 等现代样式框架。样式以 `.css` 文件形式直接编写，通过 Gulp 构建流程复制到 `Build/` 目录并随包发布。

- **组件库**：`packages/widgets` 提供一组可复用的 UI 组件（Animation、BaseLayerPicker、Geocoder、InfoBox、NavigationHelpButton、Timeline、Viewer 等），每个组件拥有独立的 CSS 文件。
- **主题系统**：通过两套入口实现明/暗主题切换——`widgets.css`（默认深色主题）和 `lighter.css`（浅色主题）。应用只需引入其中一个即可切换整体外观。
- **引擎层样式**：`packages/engine/Source/Widget/CesiumWidget.css` 是底层 CesiumWidget 的基础样式，被 widgets 包通过相对路径 `../../engine/Source/Widget/CesiumWidget.css` 引用。

## 2. 关键文件与位置

| 文件 | 作用 |
|---|---|
| `packages/widgets/Source/widgets.css` | 深色主题入口，@import 聚合所有组件样式及 engine 基础样式 |
| `packages/widgets/Source/lighter.css` | 浅色主题入口，仅 @import 需要覆盖的轻量组件样式 |
| `packages/widgets/Source/shared.css` | 全局共享样式（`.cesium-button`、`.cesium-toolbar-button`、`.cesium-performanceDisplay` 等） |
| `packages/widgets/Source/lighterShared.css` | 浅色主题对共享样式的覆盖 |
| `packages/widgets/Source/*/` | 各组件独立 CSS（如 `InfoBox/InfoBox.css`、`Geocoder/Geocoder.css`、`Timeline/Timeline.css` 等） |
| `packages/engine/Source/Widget/CesiumWidget.css` | 引擎级基础 widget 样式 |
| `Apps/CesiumViewer/CesiumViewer.css` | 示例应用样式，同时引入 `widgets.css` 与 `lighter.css` |
| `gulpfile.js` / `gulpfile.apps.js` | 构建时复制/处理 CSS 资源 |

## 3. 架构与约定

### 3.1 命名空间约定
所有 widget 类名统一以 `cesium-` 前缀命名（BEM 风格），例如：
- `.cesium-button`、`.cesium-toolbar-button`
- `.cesium-infoBox`、`.cesium-infoBox-title`、`.cesium-infoBox-bodyless`
- `.cesium-performanceDisplay`、`.cesium-performanceDisplay-fps`
- `.cesium-svgPath-svg`

这种命名空间隔离确保 CesiumJS 的样式不会与应用全局样式冲突，也便于用户通过选择器覆盖。

### 3.2 主题分层
- **共享层**：`shared.css` 定义通用按钮、工具栏、性能显示等基础样式。
- **组件层**：每个组件目录包含自己的 CSS，按功能自包含。
- **主题层**：`widgets.css` 聚合深色主题；`lighter.css` 通过只引入需要覆盖的组件样式实现浅色变体，避免重复定义。

### 3.3 构建集成
Gulp 构建脚本（`gulpfile.js`）在构建过程中会：
- 复制第三方 CSS（如 prismjs/themes/prism.min.css）到 `Tools/jsdoc/cesium_template/static/styles/`
- 将 `packages/engine/Source/Widget/*.css` 作为非打包资源单独处理
- 在 `gulpfile.apps.js` 中明确排除 `Apps/CesiumViewer/**/*.css` 不被打包进主 bundle，保持示例应用的独立性

## 4. 约定与约束

### 已观察到的约定
- 所有 widget 类名必须使用 `cesium-` 前缀，避免污染全局命名空间。
- 主题切换通过替换整个 CSS 入口实现，而非运行时动态切换。
- 组件样式采用“每个组件一个 CSS 文件”的组织方式，由顶层 `widgets.css` 统一聚合。
- 颜色值直接使用十六进制或 rgba 硬编码（如 `#303336`、`#edffff`、`rgba(38, 38, 38, 0.95)`），未使用 CSS 变量或设计令牌文件。
- 字体使用系统字体栈（`sans-serif`、`Source Sans Pro`），文档生成模板使用 `Source Sans Pro`，测试数据中的字体为 Open Sans。

### 无强制约束的证据
- 未发现 ESLint/Prettier 规则专门针对 CSS 格式（`.prettierrc` 存在但未在此处验证其 CSS 配置）。
- 未发现 CSS 变量（`:root { --xxx }`）或设计令牌集中管理文件。
- 未发现响应式媒体查询（未在样本 CSS 中发现 `@media`）。
- 未发现 CSS 模块化方案（如 CSS Modules、CSS-in-JS），全部为传统全局样式。

### 总结
CesiumJS 的样式体系是一个**传统的、基于 CSS 文件聚合的 widget 主题系统**，通过统一的 `cesium-` 命名空间、深浅双主题入口、以及按组件拆分+顶层聚合的组织方式，保证 UI 的一致性与可定制性。它不依赖任何现代 CSS 框架或预处理工具链，保持了最小化的外部依赖。