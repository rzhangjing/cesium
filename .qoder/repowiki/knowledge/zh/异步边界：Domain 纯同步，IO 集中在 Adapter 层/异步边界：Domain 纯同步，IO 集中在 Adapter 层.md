---
kind: design
name: 异步边界：Domain 纯同步，IO 集中在 Adapter 层
source: session
category: adr
---

# 异步边界：Domain 纯同步，IO 集中在 Adapter 层

_来源：6049380 → 112c418 提交周期内记录的编码计划——内容为规划时意图，实现可能滞后或有出入。_

**状态：** accepted

## 背景
CesiumJS 基于浏览器 Worker 模型将瓦片下载、解码、网格生成放在 Web Workers 中执行。Rust 侧需要决定哪些逻辑可以同步执行、哪些必须异步，以及如何在 Bevy 系统中协调。

## 决策驱动
- Domain 可单元测试（不依赖 tokio/Bevy）
- Bevy IoTaskPool 复用线程池
- 避免 Domain 层出现 async/await 污染

## 备选方案
- **Domain 纯同步 + Adapter 异步（被采纳）** — 优点：Domain 函数签名简单，可直接单元测试；IoTaskPool 统一管理网络/解码任务；Bevy System 通过命令队列与异步任务通信；缺点：需要设计命令/事件机制桥接同步 Domain 与异步 Adapter
- **Domain 也支持 async fn** _（已否决）_ — 优点：API 更简洁，无需命令队列；缺点：Domain 层引入 Future 生命周期，单元测试复杂化；Bevy 系统本身是同步的，async Domain 需要额外适配层
- **全部阻塞式 I/O** _（已否决）_ — 优点：最简单；缺点：主线程阻塞导致帧率抖动；无法利用多核并行解码瓦片

## 决策
所有 domain crate 的 API 均为纯同步函数；TileFetcher/Decoder/Clock 等外部依赖通过 ports/driver 中的 trait 暴露，由 adapters 层使用 tokio + Bevy IoTaskPool 实现异步调用，并通过命令/事件系统与 Bevy System 集成。

## 影响
Domain 层可脱离 Bevy 运行，便于编写快速单元测试；Adapter 层集中管理并发、重试、超时等横切关注点。代价是需要维护 Command/Event 协议来桥接同步与异步世界。