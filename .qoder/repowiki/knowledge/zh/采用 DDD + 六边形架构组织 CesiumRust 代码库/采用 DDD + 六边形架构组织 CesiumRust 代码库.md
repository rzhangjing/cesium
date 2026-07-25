---
kind: design
name: 采用 DDD + 六边形架构组织 CesiumRust 代码库
source: session
category: adr
---

# 采用 DDD + 六边形架构组织 CesiumRust 代码库

_来源：455a54c → 1ee1a51 提交周期内记录的编码计划——内容为规划时意图，实现可能滞后或有出入。_

**状态：** accepted

## 背景
CesiumJS 引擎包含 ~180 个 Core 数学/几何文件、675 个 Jasmine 测试，需要从零重构为 Rust。原计划直接按模块拆分 crate，但面对庞大的功能集合必须建立清晰的边界和依赖方向，避免重蹈 JS 版本中跨层直接依赖的覆辙。

## 决策驱动
- 领域边界清晰（DDD 限界上下文）
- 编译期强制依赖方向（Adapter→Port→Domain）
- GPU 精度边界（Domain f64 / Adapter f32）
- 异步边界隔离（Domain 纯同步，Adapter 处理 tokio/Bevy IoTaskPool）

## 备选方案
- **DDD + 六边形架构（选定方案）** — 优点：每个限界上下文独立 crate，Port trait 契约明确，Adapter 可替换实现，编译期保证依赖方向
- **按功能模块扁平拆分 crate** _（已否决）_ — 优点：结构简单直观；缺点：无法强制依赖方向，容易形成循环依赖；Domain 与 Bevy 耦合难以替换渲染后端

## 决策
将代码划分为 domain/ports/adapters/application 四层：domain 存放纯 Rust 算法（geospatial/terrain/imagery/tileset/model/scene/camera/time/entity/primitives/material/vector-tile/picking/event/resource/geocoding），ports 定义 Port trait 契约（driven/driving 两类），adapters 提供 Bevy/IO 具体实现，application 组装 Bevy App。通过 Cargo.toml 强制 Adapter→Port→Domain 的单向依赖。

## 影响
新增 crate 数量达 31 个 domain crate + ports + adapters + application，构建时间增加；但换来清晰的领域边界、可替换的适配器实现、以及 Domain 层无需 GPU/IO 即可单元测试的能力。精度转换集中在 Adapter 边界，避免 f64/f32 混用污染核心算法。