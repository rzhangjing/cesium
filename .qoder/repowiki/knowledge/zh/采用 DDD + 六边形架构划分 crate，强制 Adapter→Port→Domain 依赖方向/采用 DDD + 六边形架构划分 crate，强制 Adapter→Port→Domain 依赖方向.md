---
kind: design
name: 采用 DDD + 六边形架构划分 crate，强制 Adapter→Port→Domain 依赖方向
source: session
category: adr
---

# 采用 DDD + 六边形架构划分 crate，强制 Adapter→Port→Domain 依赖方向

_来源：b849cf0 → 56a9b8e 提交周期内记录的编码计划——内容为规划时意图，实现可能滞后或有出入。_

**状态：** accepted

## 背景
CesiumJS 源码庞大且耦合严重（Core/Scene/Workers 混编），需要将其重构为可维护、可测试的 Rust 项目。原计划需明确领域边界与外部系统交互方式，避免重蹈 CesiumJS 跨层直接调用的覆辙。

## 决策驱动
- 编译期依赖约束
- 纯算法可测试性
- Bevy 渲染引擎解耦
- CesiumJS 功能无遗漏映射

## 备选方案
- **按 CesiumJS 目录结构镜像 crate 划分** _（已否决）_ — 优点：迁移成本低，文件一一对应；缺点：保留 JS 时代的耦合关系，无法利用 Rust 模块系统优势
- **DDD 限界上下文 + 六边形端口抽象** — 优点：Domain 纯同步 f64 算法可独立单元测试；Adapter 层统一处理 Bevy/tokio/IO；Cargo.toml 强制依赖方向；缺点：初期 crate 数量多，Port trait 设计开销大

## 决策
将工作区划分为 domain/ports/adapters/application 四层，每个限界上下文（geospatial/terrain/imagery/tileset/model/scene/camera/time/entity/primitives/material/vector-tile/picking/event/resource/geocoding）作为独立 crate；通过 driven/driving Port trait 隔离外部依赖，由 Cargo.toml 禁止 Domain 反向引用 Adapter。

## 影响
编译期保证依赖方向正确，Domain 可脱离 Bevy 运行；新增外部能力只需实现对应 Port trait；但 crate 粒度细导致 cargo build 时间增加，需要通过 workspace 和增量编译缓解。