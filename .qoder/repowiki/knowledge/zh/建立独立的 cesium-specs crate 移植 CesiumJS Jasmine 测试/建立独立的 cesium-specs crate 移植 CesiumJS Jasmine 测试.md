---
kind: design
name: 建立独立的 cesium-specs crate 移植 CesiumJS Jasmine 测试
source: session
category: adr
---

# 建立独立的 cesium-specs crate 移植 CesiumJS Jasmine 测试

_来源：1ee1a51 → 3f8d99d 提交周期内记录的编码计划——内容为规划时意图，实现可能滞后或有出入。_

**状态：** accepted

## 背景
需要将 packages/engine/Specs/ 下 675 个 Jasmine 测试文件从 JavaScript 移植到 Rust，作为集成测试覆盖所有 31 个 domain crate。原计划采用按功能分组的目录结构（core/datasources/scene/renderer/widgets），每个原版 it(...) 对应一个 Rust #[test] 函数。

## 决策驱动
- 与 CesiumJS 源码一一对应的可追溯性
- 按功能分组便于维护
- 纯域模型测试不依赖 GPU/WebGL
- 浮点数断言使用 epsilon 比较

## 备选方案
- **独立 specs crate + tests/ 目录结构** — 优点：与源码目录对齐、便于增量移植、每个 Phase 可独立验证
- **分散在各 domain crate 的单元测试中** _（已否决）_ — 优点：测试靠近代码；缺点：无法保持与 CesiumJS Spec 的一一对应关系、难以统计覆盖率

## 决策
创建 cesiumrust/specs 独立 crate，tests/ 下按 core/datasources/scene/renderer/widgets 分组，每个原版 Spec 文件映射为 1-3 个 Rust 测试文件，通过 assert_epsilon!/assert_approx! 宏实现浮点近似断言。

## 影响
建立了完整的测试基础设施（Phase 1）后，后续按 Phase 2-6 逐步移植，最终目标 ~55 个测试文件、~940 个测试函数，覆盖全部 31 个域 crate。异步测试统一使用 tokio::test 或同步模拟。