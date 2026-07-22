---
kind: frontend_style
name: CesiumJS 前端样式系统：基于 CSS 的轻量级主题与组件样式
category: frontend_style
scope:
    - '**'
source_files:
    - Source/Widgets/widgets.css
    - Source/Widgets/lighter.css
    - Apps/CesiumViewer/CesiumViewer.css
    - Apps/HelloWorld.html
---

## 样式系统与架构

CesiumJS 的前端样式采用**纯 CSS + 轻量主题**方案，没有引入 SCSS、Tailwind 等现代样式框架。核心样式集中在 `Source/Widgets/` 目录下的 CSS 文件中（如 `widgets.css`、`lighter.css`），通过 `@import` 机制组织。

### 核心文件与结构
- **主样式文件**：`Source/Widgets/widgets.css` - 包含所有 UI 组件的基础样式
- **浅色主题**：`Source/Widgets/lighter.css` - 提供浅色背景上的深色文字主题变体
- **应用集成**：`Apps/CesiumViewer/CesiumViewer.css` - 示例应用的样式入口，通过 `@import` 引用组件样式
- **构建产物**：`Build/Cesium/Widgets/` - 构建后输出的样式文件路径

### 主题系统
CesiumJS 实现了简单的双主题模式：
- **默认主题**：深色背景 + 浅色文字（适合地图可视化）
- **Lighter 主题**：浅色背景 + 深色文字（适合文档和演示）

主题切换通过 JavaScript 控制：
```javascript
if (endUserOptions.theme === "lighter") {
  // 加载 lighter 主题样式
}
```

### 样式方法论
1. **CSS 模块化**：按功能模块拆分 CSS 文件，通过 `@import` 组合
2. **命名空间隔离**：Widget 组件使用特定的 CSS 类名前缀避免冲突
3. **响应式设计**：基础媒体查询支持不同屏幕尺寸
4. **无预处理器**：直接使用原生 CSS，便于浏览器兼容性和调试

### 设计决策
- **零依赖策略**：不依赖任何 CSS 框架或预处理器，保持库的轻量化
- **向后兼容**：CSS 类名保持稳定 API，确保升级兼容性
- **主题可定制**：通过覆盖 CSS 变量和类样式实现主题定制
- **按需加载**：示例应用通过选择性 `@import` 减少样式体积

### 开发者约定
- Widget 样式应放在 `Source/Widgets/` 对应子目录
- 新组件需遵循现有 CSS 命名规范和主题变量
- 主题相关样式应同时考虑 light/dark 两种场景
- 避免内联样式，优先使用 CSS 类选择器