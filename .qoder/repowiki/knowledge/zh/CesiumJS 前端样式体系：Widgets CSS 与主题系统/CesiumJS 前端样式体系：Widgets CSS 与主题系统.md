---
kind: frontend_style
name: CesiumJS 前端样式体系：Widgets CSS 与主题系统
category: frontend_style
scope:
    - '**'
source_files:
    - packages/widgets/Source/lighter.css
    - packages/widgets/README.md
    - Apps/CesiumViewer/CesiumViewer.css
    - scripts/build.js
    - Specs/e2e/cesium.html
    - packages/sandcastle/templates/bucket.html
---

## 1. 使用的系统与工具
- **原生 CSS**：CesiumJS 的 UI 样式完全基于标准 CSS，未使用 Sass/Less/PostCSS/Tailwind 等预处理或原子化框架。
- **组件级 CSS 文件**：每个 Widget（BaseLayerPicker、Geocoder、Timeline、Animation、NavigationHelpButton 等）拥有独立的 `.css` 文件，通过聚合入口统一导出。
- **构建阶段处理**：通过 Gulp + `gulp-clean-css` 在构建时压缩合并 CSS；`scripts/build.js` 会读取并写入 `Source/Widgets/widgets.css` 和 `lighter.css`。
- **双主题策略**：提供两套样式入口——默认深色主题 `widgets.css` 与浅色主题 `lighter.css`，应用可通过切换引入实现明暗主题。
- **依赖管理**：根 `package.json` 声明 `@cesium/widgets` 为运行时依赖，并通过 `sideEffects` 标记 `./Source/Widgets/**/*.css` 以便打包器正确保留副作用。

## 2. 关键文件与包
- `packages/widgets/Source/lighter.css` — 浅色主题聚合入口，按组件子目录分别 `@import` 对应 `lighter.css`。
- `packages/widgets/README.md` — 文档中给出引入方式：`import "@cesium/widgets/Source/widgets.css"`。
- `Apps/CesiumViewer/CesiumViewer.css` — 示例应用的基础样式，`@import` 聚合后的 widgets 样式，并定义全屏容器与加载指示器。
- `scripts/build.js` — 构建脚本负责生成 `Source/Widgets/widgets.css` / `lighter.css` 聚合文件。
- `Specs/e2e/cesium.html` — E2E 测试直接引用 `Source/Widgets/widgets.css` 与 `lighter.css`。
- `packages/sandcastle/templates/bucket.html` — Sandcastle 模板通过 `__CESIUM_BASE_URL__/Widgets/widgets.css` 引用构建产物。

## 3. 架构与约定
- **按组件分文件**：每个 Widget 在独立目录下维护自己的 CSS（如 `BaseLayerPicker/lighter.css`、`Geocoder/lighter.css`），避免全局命名冲突。
- **聚合入口模式**：顶层 `widgets.css` / `lighter.css` 仅做 `@import` 聚合，不写具体样式，便于按需裁剪与主题切换。
- **主题变量未集中**：未发现统一的 CSS 自定义属性（CSS Variables）或设计令牌文件，颜色、尺寸等值在各组件文件中硬编码，主题切换通过整份样式替换而非变量覆盖实现。
- **无响应式断点集中管理**：未见 `@media` 查询的统一断点常量，响应式逻辑分散在各组件内部。
- **构建产物路径约定**：最终发布到 `Build/Cesium/Widgets/` 与 `Build/CesiumUnminified/Widgets/`，由 `index.html` 与文档示例统一引用。

## 4. 开发者应遵循的规则
- **新增 Widget 样式**：在对应组件目录下创建 `*.css` 文件，并在 `lighter.css` 中添加 `@import url(./xxx/lighter.css);` 以纳入浅色主题。
- **不要直接修改聚合文件**：`widgets.css` 与 `lighter.css` 由构建脚本生成，手动编辑会被覆盖。
- **主题选择**：默认使用 `widgets.css`（深色），需要浅色主题时改用 `lighter.css`，不建议在同一页面同时引入两份。
- **样式作用域**：Widget 类名以组件前缀命名（如 `.cesium-*`），避免与宿主页面样式冲突。
- **打包器集成**：确保将 `@cesium/widgets/Source/**/*.css` 加入 `sideEffects`，否则 Tree-shaking 可能误删样式。