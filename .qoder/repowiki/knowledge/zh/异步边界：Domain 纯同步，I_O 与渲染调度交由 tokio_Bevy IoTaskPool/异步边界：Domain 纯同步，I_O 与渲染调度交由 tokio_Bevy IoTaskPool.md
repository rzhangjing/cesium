---
kind: design
name: 异步边界：Domain 纯同步，I/O 与渲染调度交由 tokio/Bevy IoTaskPool
source: session
category: adr
---

# 异步边界：Domain 纯同步，I/O 与渲染调度交由 tokio/Bevy IoTaskPool

_来源：b849cf0 → 56a9b8e 提交周期内记录的编码计划——内容为规划时意图，实现可能滞后或有出入。_

**状态：** accepted

## 背景
地形瓦片、影像图层、3D Tiles 内容均需网络请求与解码，CesiumJS 使用 Web Workers 并行处理。Rust 侧需决定异步模型放置位置，避免阻塞主线程或污染领域逻辑。

## 决策驱动
- Bevy 单线程 main loop 约束
- 并发 I/O 吞吐
- 领域逻辑可测试性

## 备选方案
- **Domain 暴露 async fn，调用方自行 await** _（已否决）_ — 优点：调用方可灵活选择并发策略；缺点：单元测试需引入 async runtime，破坏纯函数特性
- **Domain 纯同步，Async 封装在 Adapter 层 via tokio/IoTaskPool** — 优点：Domain 可同步单测；Bevy System 中 spawn_blocking 或 IoTaskPool 调度；符合六边形原则；缺点：同步阻塞调用需显式放入后台线程池

## 决策
所有 Domain API 均为同步函数（如 TerrainData::create_mesh、select_tiles、evaluate_style）；TileFetcher/Decoder/Cache 等外部依赖通过 #[async_trait] Port trait 定义，由 adapters/network 使用 reqwest + tokio 实现，并在 adapters/bevy-render 中通过 Bevy IoTaskPool 调度执行。

## 影响
领域逻辑完全可同步测试，无需 mock 异步运行时；但需注意同步阻塞操作不能出现在 Bevy 主循环中，必须通过 IoTaskPool 或 spawn_blocking 包装；未来如需流式解码，可在 Port 层扩展而不影响 Domain。