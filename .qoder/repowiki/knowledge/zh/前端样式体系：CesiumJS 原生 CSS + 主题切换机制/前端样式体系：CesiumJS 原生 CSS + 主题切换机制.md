---
kind: frontend_style
name: 前端样式体系：CesiumJS 原生 CSS + 主题切换机制
category: frontend_style
scope:
    - '**'
source_files:
    - Apps/CesiumViewer/CesiumViewer.css
    - Apps/CesiumViewer/index.html
    - packages/widgets/Source/lighter.css
    - gulpfile.js
    - package.json
---

本仓库是 CesiumJS 的 Rust 全量重写（CesiumRust），其前端 UI 部分仍沿用原始 CesiumJS 的样式架构，采用纯 CSS 文件组织、无 CSS-in-JS、无 Tailwind/SCSS/Less 等现代预处理方案。核心风格系统围绕以下要点构建：

1. **样式入口与加载方式**
   - 应用入口 `Apps/CesiumViewer/index.html` 通过 `<link>` 引入 `CesiumViewer.css`。
   - `CesiumViewer.css` 使用 `@import` 聚合两个关键样式包：`../../Source/Widgets/widgets.css`（默认浅色主题）和 `../../Source/Widgets/lighter.css`（轻量深色主题），由运行时 `theme=lighter` 选项控制是否启用后者。
   - 构建产物路径 `Build/CesiumUnminified/Widgets/widgets.css` 同样被 Sandcastle 模板引用，形成“源码 → 构建产物”双入口。

2. **主题机制**
   - 两套主题通过独立 CSS 文件提供：`widgets.css` 为完整默认主题，`lighter.css` 为“轻主题”变体，二者均按组件目录拆分（Animation、BaseLayerPicker、Geocoder、Timeline、NavigationHelpButton 等子目录各自维护同名 `lighter.css`）。
   - 运行时通过 `endUserOptions.theme === 'lighter'` 动态决定是否加载 light 变体，未实现暗色/多主题变量驱动，属于“文件级主题切换”。

3. **CSS 方法论与约定**
   - 全局样式集中在 `CesiumViewer.css`：`html/body` 高度 100%、`overflow: hidden`、背景 `#000`；`.fullWindow` 绝对定位铺满视口；`.loadingIndicator` 居中加载动画。
   - 组件样式遵循“每个 Widget 一个目录、同目录内 `*.css` 与 JS 一一对应”的组织模式（见文档 `Documentation/Contributors/CodingGuide` 对 `Source/Widgets` 的描述）。
   - 未使用 CSS Modules、BEM、CSS-in-JS 或设计 Token 系统，类名直接暴露给 JS 通过 `element.className` / `style.display` 操作。

4. **构建与打包集成**
   - `gulpfile.js` 在构建流程中复制 Prism 语法高亮样式到 `Tools/jsdoc/cesium_template/static/styles/prism.css`，并参与 `buildWidgets` 任务生成 `Build/Cesium*/Widgets/*.css`。
   - `package.json` 将 `./Source/Widgets/**/*.css` 纳入发布范围，确保 npm 包包含所有 widget 样式。

5. **测试与示例中的样式复用**
   - `Specs/e2e/cesium.html` 与 `Apps/HelloWorld.html` 均直接 `@import` 同一份 `widgets.css` / `lighter.css`，保证示例与 E2E 测试视觉一致性。

**开发者应遵循的规则**
- 新增 Widget 时，在对应目录下创建 `WidgetName.css` 与可选的 `WidgetName/lighter.css`，并在顶层 `widgets.css` 与 `lighter.css` 中 `@import` 引入。
- 不要引入 SCSS/Tailwind 等新工具链；保持纯 CSS 文件结构，以便与现有 Gulp 构建管线兼容。
- 主题扩展仅通过新增 `lighter.css` 变体文件实现，避免在运行时注入样式。
- 全局布局修改优先放在 `Apps/CesiumViewer/CesiumViewer.css`，而非在各 Widget 内部重复定义。