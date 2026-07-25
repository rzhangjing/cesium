---
kind: design
name: specs crate 作为独立测试 crate 移植全部 675 个 Jasmine Spec
source: session
category: adr
---

# specs crate 作为独立测试 crate 移植全部 675 个 Jasmine Spec

_来源：455a54c → 1ee1a51 提交周期内记录的编码计划——内容为规划时意图，实现可能滞后或有出入。_

**状态：** accepted

## 背景
CesiumJS 拥有 675 个 Jasmine 测试文件覆盖 Core/DataSources/Scene/Renderer/Widgets 五大类，需要系统性地移植为 Rust 集成测试以验证重构正确性。

## 决策驱动
- 测试覆盖率对齐原版
- 浮点精度断言替代 toEqualEpsilon
- 异步测试使用 tokio::test
- 不依赖 GPU/WebGL 的纯域模型测试

## 备选方案
- **独立 specs crate + 按功能分组测试文件（选定方案）** — 优点：Cargo workspace 管理，依赖所有 domain crate，测试文件按 core/datasources/scene/renderer/widgets 分组，便于增量验证
- **分散在各 crate 内写单元测试** _（已否决）_ — 优点：就近测试；缺点：无法覆盖跨 crate 的集成场景；难以保持与原版 Jasmine 测试的一一对应关系

## 决策
创建 cesiumrust/specs 独立 crate，将 675 个 Jasmine 测试映射为 ~97 个 Rust 测试文件（Core→~30, DataSources→~15, Scene→~40, Renderer→~5, Widgets→~5），使用 assert_epsilon!/assert_approx! 宏替代 toEqualEpsilon，#[should_panic] 替代 toThrowDeveloperError，tokio::test 处理异步用例。

## 影响
约 42,700 行测试代码，对应 ~2,770 个测试函数。Phase 1-7 分阶段实施，每阶段运行 cargo test 验证。测试仅依赖 Domain 层，不触发真实 IO 或 GPU 调用，确保快速反馈。