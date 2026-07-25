---
kind: design
name: 采用 DDD + 六边形架构按领域上下文拆分 crate
source: session
category: adr
---

# 采用 DDD + 六边形架构按领域上下文拆分 crate

_来源：56a9b8e → 12aeaaa 提交周期内记录的编码计划——内容为规划时意图，实现可能滞后或有出入。_

**状态：** accepted

## 背景
CesiumJS 源码庞大（Core/Scene/Workers 下数百个文件），需要将其重构为可维护的 Rust 工程，同时保持与 CesiumJS 功能的一一映射。

## 决策驱动
- 领域边界清晰、跨层依赖可控
- 纯 Domain 可独立测试
- Adapter 可替换实现

## 备选方案
- **传统分层（Controller/Service/Repository）** _（已否决）_ — 优点：熟悉；缺点：领域逻辑散落在各层，难以复用和测试
- **DDD + 六边形（Domain/Port/Adapter）** — 优点：编译期强制依赖方向；Domain 纯同步 f64 算法可独立验证；Adapter 可插拔 Bevy/IO 实现；缺点：初始建模成本高；trait 抽象带来少量运行时开销

## 决策
将代码拆分为 domain/*、ports/*、adapters/*、application/* 四个 crate 层级，通过 Cargo.toml 强制 Adapter → Port → Domain 的单向依赖，每个限界上下文（geospatial、terrain、imagery、tileset、model、scene、camera、entity、primitives、material、vector-tile、picking、event、resource、geocoding）对应一个 domain crate。

## 影响
新增领域需先定义 Port trait，再写 Domain 和 Adapter；编译期即可发现跨层依赖违规；但 trait 分发和装箱在热点路径上引入轻微开销，需在 bevy-render adapter 中用零拷贝优化。