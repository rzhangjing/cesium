---
kind: design
name: 采用 DDD + 六边形架构，按限界上下文拆分 crate
source: session
category: adr
---

# 采用 DDD + 六边形架构，按限界上下文拆分 crate

_来源：6049380 → 112c418 提交周期内记录的编码计划——内容为规划时意图，实现可能滞后或有出入。_

**状态：** accepted

## 背景
CesiumJS 是一个超过 10000 行的大型地理空间引擎，包含大量数学、几何、渲染逻辑。原计划将其直接移植为 Rust 单仓结构会导致模块耦合严重、难以维护。需要一种能清晰隔离领域边界、约束依赖方向的架构模式。

## 决策驱动
- 领域边界清晰（geospatial/terrain/imagery/tileset/model 等）
- 编译期强制依赖方向（Adapter → Port → Domain）
- 可测试性（Domain 纯同步、无外部依赖）
- 与 CesiumJS 源码一一对应便于功能覆盖验证

## 备选方案
- **DDD + 六边形（被采纳）** — 优点：每个限界上下文独立 crate，Port trait 定义契约，Adapter 实现具体 IO；Domain 层零外部依赖，可单元测试；Cargo.toml 强制依赖方向；缺点：初始样板代码较多，trait 抽象带来少量运行时开销
- **传统分层架构（表现层/业务层/数据层）** _（已否决）_ — 优点：结构简单，学习成本低；缺点：跨层直接调用无法在编译期阻止；领域逻辑与 Bevy/IO 耦合，难以单独测试；无法按领域边界并行开发
- **事件总线 + 微服务** _（已否决）_ — 优点：解耦彻底；缺点：进程间通信引入额外复杂度；当前项目规模不需要分布式部署；Bevy 生态更适合单体应用内消息传递

## 决策
采用 DDD 结合六边形架构：每个限界上下文拆分为 domain/ports/adapters 三层，通过 Cargo workspace 组织为独立 crate，由 Cargo.toml 的依赖声明强制 Adapter → Port → Domain 的单向依赖。

## 影响
新增一个领域需新建三个目录并定义 Port trait，但换来的是各上下文可独立演进、Domain 可离线测试、Adapter 可替换（如网络从 reqwest 换到 hyper）。构建时间因多 crate 略有增加，但可通过增量编译缓解。