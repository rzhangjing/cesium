---
kind: frontend_style
name: 双轨前端样式体系：CesiumJS 遗留 CSS 与 Rust/GPUI 主题化 UI
category: frontend_style
scope:
    - '**'
source_files:
    - Apps/CesiumViewer/CesiumViewer.css
    - packages/widgets/Source/lighter.css
    - cesiumrust/crates/theme/src/lib.rs
    - cesiumrust/crates/ui/src/button.rs
    - cesiumrust/crates/workspace/src/workspace.rs
---

## 1. 整体方案

本仓库同时维护两套前端样式系统，分别服务于不同的运行时：

- **CesiumJS（JavaScript/浏览器端）**：沿用原版 CesiumJS 的 CSS 模块化方案，通过 `Source/Widgets/widgets.css` 与 `lighter.css` 聚合各 Widget 子模块样式，应用层通过 `@import` 引用。
- **CesiumRust（Rust + Bevy/GPUI 桌面端）**：使用 GPUI 的声明式 UI 框架，通过独立的 `theme` crate 集中管理设计令牌（Design Tokens），所有 UI 组件以 `RenderOnce` / `Render` trait 实现，零 CSS 文件。

两套系统互不依赖，由不同入口加载。

## 2. 关键文件与包

| 子系统 | 关键路径 | 作用 |
|---|---|---|
| CesiumJS 样式入口 | `Apps/CesiumViewer/CesiumViewer.css` | 页面级样式，引入 widgets/lighter 并设置全屏黑底 |
| CesiumJS 样式聚合 | `packages/widgets/Source/lighter.css` | 按需导入 Animation、BaseLayerPicker、Geocoder、Timeline 等 lighter 变体 |
| CesiumJS 引擎样式 | `engine/Source/Widget/lighter.css` | 底层 Widget 共享样式 |
| Rust 主题令牌 | `cesiumrust/crates/theme/src/lib.rs` | `AppColors`、`FontSizes` 单点定义 |
| Rust UI 组件 | `cesiumrust/crates/ui/src/*.rs` | Button、Input、Panel、TabBar、TitleBar、StatusBar 等 |
| Rust 工作区布局 | `cesiumrust/crates/workspace/src/workspace.rs` | 顶层窗口 chrome，组合 TitleBar / BevyView / StatusBar |

## 3. 架构与约定

### 3.1 CesiumJS 侧：CSS 模块化 + 轻量主题
- 采用 `widgets.css`（全量）和 `lighter.css`（精简）两种打包产物，应用通过 `@import url(...)` 引入。
- 每个 Widget 子目录提供自己的 `*.css` 与可选 `lighter.css`，由聚合文件统一组装。
- 示例应用 `CesiumViewer/index.html` 中直接引用 `../../Source/Widgets/widgets.css` 与 `lighter.css`。
- 页面级样式集中在 `CesiumViewer.css`：设置 `html/body` 全屏、`background: #000`、隐藏滚动条，以及 `.fullWindow`、`.loadingIndicator` 等容器样式。

### 3.2 Rust/GPUI 侧：Design Token + 声明式组件
- **设计令牌集中化**：`theme::AppColors` 提供 base/surface/overlay/text/accent/status/border 等语义色；`FontSizes` 提供 SM/BASE/LG/XL/XXL 字号常量。所有颜色通过 `rgb(0xRRGGBB)` 构造为 `gpui::Rgba`。
- **组件实现模式**：遵循 Zed 风格的 `RenderOnce` trait，如 `Button` 用链式 fluent API（`.flex().items_center().rounded_lg().bg(AppColors::accent())...`）描述样式，无外部 CSS 文件。
- **主题消费方式**：UI 组件 `use theme::AppColors;` 后通过 `AppColors::xxx()` 获取颜色，workspace 布局同样如此，确保全局一致。
- **布局策略**：使用 GPUI 内置的 flexbox（`.flex_col()`, `.flex_1()`, `.justify_between()`）+ 固定高度（`px(32.0)` title bar, `px(24.0)` status bar），响应式靠弹性布局而非媒体查询。

## 4. 开发者应遵守的规则

1. **新增 CesiumJS Widget 样式**：在 `packages/widgets/Source/<Widget>/` 下创建 `style.css`，并在 `lighter.css` 中按需 `@import`；若需精简版，额外提供 `lighter.css`。
2. **页面级样式只放 `CesiumViewer.css`**：不要在全局注入新规则，保持仅覆盖 body/fullWindow/loadingIndicator 级别。
3. **新增 Rust UI 组件**：放在 `cesiumrust/crates/ui/src/`，实现 `RenderOnce`，颜色一律通过 `AppColors::*` 获取，禁止硬编码十六进制色值。
4. **新增设计令牌**：在 `theme::lib.rs` 的 `AppColors` 或 `FontSizes` 中添加，不要新建 token 文件。
5. **布局优先使用 GPUI 的 flex 工具类**：避免手写像素布局，标题栏/状态栏高度用 `px()` 常量统一管理。
6. **Bevy 渲染侧的颜色转换**：领域层 `Color` 通过 `domain_color_to_bevy` 转换为 `bevy::prelude::Color`，不要在渲染适配器里直接写 RGB 字面量。
7. **样式与逻辑解耦**：CesiumJS 侧样式走 CSS 文件，Rust 侧样式走组件代码，两者不可混用——GPUI 组件不应依赖任何 `.css` 文件。

## 5. 置信度说明

该仓库存在明确的双轨样式体系：CesiumJS 部分保留原始 CSS 模块化结构，Rust/GPUI 部分建立了完整的 Design Token + 声明式组件规范。证据来自 `theme` crate 的集中配色、多个 UI 组件对 `AppColors` 的一致引用、以及 `CesiumViewer.css` 对 widgets 样式的聚合导入。因此置信度为 **high**。
