---
kind: design
name: 采用 DDD + 六边形架构重构 CesiumRust
source: session
category: adr
---

# 采用 DDD + 六边形架构重构 CesiumRust

_来源：a5ed267 → 6049380 提交周期内记录的编码计划——内容为规划时意图，实现可能滞后或有出入。_

**状态：** accepted

## 背景
现有 cesiumrust/ 工作区基于 GPUI 框架，仅有软件渲染的 3D 立方体 Demo，无真实地理空间能力。需要完全脱离浏览器，以 Bevy 0.15+ 为底层引擎构建原生桌面 3D 地球应用，同时保持与 CesiumJS 功能等价。

## 决策驱动
- 领域逻辑可独立测试（零框架依赖）
- CesiumJS 天然存在多个子领域需明确边界
- Bevy ECS 作为编排层而非侵入领域层
- 编译期强制依赖方向

## 备选方案
- **传统分层架构（Controller-Service-Repository）** _（已否决）_ — 优点：简单直观，团队熟悉；缺点：领域逻辑与 Bevy/IO 耦合，难以单元测试；CesiumJS 复杂领域难以清晰分层
- **DDD + 六边形架构** — 优点：Domain Core 纯 Rust 零依赖，可独立测试；Port trait 抽象外部实现；限界上下文隔离 CesiumJS 各模块；缺点：初期学习成本较高；trait 抽象增加样板代码

## 决策
采用 DDD 战略设计划分限界上下文（Geospatial/Terrain/Imagery/Tileset/Scene/Camera/Time/Entity），结合六边形架构将 Domain（纯算法）与 Adapter（Bevy/网络/IO）通过 Port trait 解耦，Bevy ECS System 仅作为 Application 层编排用例。

## 影响
编译期可通过 Cargo.toml 强制依赖方向（Adapter→Port→Domain）；Domain 层 100% 可用 mock adapter 做单元测试；新增数据源/渲染后端只需实现 Port trait 无需修改领域逻辑；但增加了 trait 抽象和 crate 间通信的复杂度。