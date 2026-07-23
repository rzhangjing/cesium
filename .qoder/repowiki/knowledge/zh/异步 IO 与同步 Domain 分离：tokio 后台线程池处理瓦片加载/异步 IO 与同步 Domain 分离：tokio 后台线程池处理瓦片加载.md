---
kind: design
name: 异步 IO 与同步 Domain 分离：tokio 后台线程池处理瓦片加载
source: session
category: adr
---

# 异步 IO 与同步 Domain 分离：tokio 后台线程池处理瓦片加载

_来源：a5ed267 → 6049380 提交周期内记录的编码计划——内容为规划时意图，实现可能滞后或有出入。_

**状态：** accepted

## 背景
Cesium 需要大量异步操作：HTTP 请求瓦片数据、磁盘缓存读写、quantized-mesh/draco/gltf 解码、纹理压缩转码等。Bevy 主循环是同步的，直接阻塞会卡顿 UI。

## 决策驱动
- 主循环不阻塞
- 并发控制避免过多连接
- 解码不阻塞渲染

## 备选方案
- **Bevy IoTaskPool 单一线程池** _（已否决）_ — 优点：与 Bevy 生命周期集成好；缺点：无法精细控制并发策略，IO 与 CPU 任务混用可能互相影响
- **独立 tokio Runtime + 消息通道** — 优点：可配置多线程 Worker Pool；IO 与 CPU 任务分离；失败重试/超时控制灵活；缺点：跨运行时传递所有权复杂；需要 Event/Resource 桥接

## 决策
Domain 层保持纯同步接口，通过 Driven Port trait 定义异步契约；Adapter 层使用独立 tokio Runtime 管理 IO 任务（reqwest 并发池、解码器线程池），通过 Bevy Resource/Event 将结果回传主循环。

## 影响
主循环帧率稳定不受 IO 波动影响；可针对不同任务类型配置不同线程池大小；但增加了事件传递的延迟和复杂性，调试跨运行时状态较困难。