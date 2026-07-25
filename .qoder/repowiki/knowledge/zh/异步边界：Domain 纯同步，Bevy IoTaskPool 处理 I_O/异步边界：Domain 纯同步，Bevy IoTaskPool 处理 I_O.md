---
kind: design
name: 异步边界：Domain 纯同步，Bevy IoTaskPool 处理 I/O
source: session
category: adr
---

# 异步边界：Domain 纯同步，Bevy IoTaskPool 处理 I/O

_来源：56a9b8e → 12aeaaa 提交周期内记录的编码计划——内容为规划时意图，实现可能滞后或有出入。_

**状态：** accepted

## 背景
瓦片加载、网络请求、解码是阻塞操作，但 Domain 必须是可确定性测试的纯函数；Bevy 生态推荐用 IoTaskPool 而非 tokio runtime。

## 决策驱动
- Domain 可单测无外部依赖
- 与 Bevy 生命周期集成
- 避免在渲染线程阻塞

## 备选方案
- **Domain 内直接 async/await 调用 reqwest** _（已否决）_ — 优点：简单直接；缺点：破坏纯函数性质；无法离线单测；耦合具体 HTTP 库
- **Domain 同步 + Port trait + adapters 用 IoTaskPool 调度** — 优点：Domain 可独立测试；I/O 细节隔离；与 Bevy 调度器协作；缺点：需要编写大量 trait 样板代码

## 决策
所有 I/O 相关能力通过 ports/driven 中的 trait 暴露（如 TerrainTileFetcher、ImageryFetcher、TilesetFetcher），由 adapters/network 使用 Bevy 的 IoTaskPool 并发调度 reqwest 请求，再通过事件或命令回传结果给 Domain。

## 影响
测试时可用 Mock 实现 Port 快速验证领域逻辑；但增加了 trait 数量和消息传递开销，需在 TilesetTraversal 等高吞吐路径评估是否引入零拷贝通道。