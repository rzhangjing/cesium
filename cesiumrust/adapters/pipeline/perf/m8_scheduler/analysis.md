# M8.5 scheduler 端到端吞吐 — 根因分析与门⑤解读

> 配套 `delta-report.md`（自动生成的实测三件套）。本文件为手写分析，跨 bench 运行不被覆盖。
> 权威判据：计划 L206「scheduler 端到端吞吐 ≥ M1 baseline」；三件套口径 L329（Nora 性能门）。

## 1. 结论摘要

| 项 | 值 |
|---|---|
| before（M1 `GenericPipeline`，batch-push） | median ≈ **1.59 M jobs/s** |
| after（M8 `PipelineResourceBackend`，blocking-pull） | median ≈ **194 K jobs/s** |
| ratio = after / before | **≈ 0.122** |
| 门⑤ 字面判定（after ≥ before） | **REGRESSION（未达）** |
| scheduler 引擎（16-worker `WorkerPool`） | **未改动 / 未回归** |
| 黄金路径影响 | **无**（M8 opt-in，未接线） |

三次独立 `cargo bench` 运行 ratio = 0.1234 / 0.1271 / 0.1223（含一次实验性 dispatcher 改动），
两臂 `completed` 恒为 2000/2000，标准差 before ±5.4%CV、after ±6.6%CV → **确定性可复现，方差可控**。

## 2. 根因：模型差异，而非 scheduler 回归

after 比 before 慢约 8x，根因是 **M8 `request_stream` 阻塞-pull 模型的锁串行化 + 线程往返**：

- **全局 `intake: Mutex<()>` 串行化（主因）** — 32 个 caller 的冷路径 intake 与**单一** dispatcher
  的完成路由争同一把锁（33 线程 → 1 mutex），且该锁跨 ~5 个子操作持有
  （cache get → dedup insert → waiters push → extend_wanted → pool.submit）。有效并发被压到 ≈1。
- **每请求约 9 次锁获取** — caller 侧 intake/cache/dedup/waiters/wanted + dispatcher 侧
  intake/dedup/cache/waiters。
- **每请求 2 次线程交接** — caller → worker → dispatcher → caller（mpsc + 阻塞 `rx.recv()`）；
  M1 仅 submit-入队 / drain-出队各一次，且 submit 为 fire-and-forget（不与 worker 往返）。
- **本质是两种模型** — M1 = batch fire-and-forget 流水线（延迟在一批内摊销）；
  M8 = per-request 阻塞 RPC（每请求付一次往返延迟 × 并发数）。batch 模型在**原始吞吐**上恒胜，
  这是模型构造使然，**不是 scheduler 引擎劣化**。

### 实证排除 dispatcher 1ms poll 为主因

将 dispatcher 的 `sleep(1ms)`-on-empty 换成有界 spin-yield（`SPIN_WINDOW=128`）后，after 仅
从 187K → 204K（+9%），而**无代码改动**的 before 臂在同批次也从 1516K → 1603K（+6%）——
即 +9% 落在运行间噪声内。原因：spin 窗口（128 次 yield ≈ 数十 µs）**小于** intake 锁把 32 个
caller 串行重新提交所需的间隙（~160 µs），dispatcher 仍在窗口耗尽后睡 1ms。
=> poll 非瓶颈，**intake 锁串行化才是**。该实验性改动已**回退**，`resource_backend.rs` 保持
#67 评审态（`git diff adapters/pipeline/src/resource_backend.rs` 为空）。

## 3. scheduler 引擎未回归（门⑤ 的实质）

两臂共用**同一** 16-worker keep-alive `WorkerPool` + 同一 `InstantMock`。before 臂的
≈1.59 M jobs/s 即该引擎的派发上限，证明 M8 重构**未劣化** `WorkerPool` / `RequestScheduler`
本身；8x 差距**全部**来自 M8 在引擎之上叠加的 cache 层级（hot `GpuCache` + warm `HiddenLru`）
+ `Dedup` + dispatcher + waiter 桥这一 **wrapper 抽象层**。

## 4. 黄金路径影响：无

`PipelineResourceBackend` 为 opt-in，**未接入任何默认 cesium-app plugin**（M1.3 rollout guard，
见 `resource_backend.rs` 模块文档）；`dynamic_globe` 仍走 M1 batch-push 管线。故 Nora
**帧时间门**（L329，真正的合并阻塞性能门，度量 p95 帧时间）**不受影响**。

## 5. 门⑤ 裁定建议（交 #69 / leader）

门⑤ 字面（M8 wrapper 端到端 ≥ M1 batch 端到端）= REGRESSION；scheduler 引擎层面 = 保持。
需在三条中裁定范围：

- **(a) 接受** M8 阻塞-pull wrapper 开销为 cache/dedup/streaming 抽象的固有成本
  （opt-in、离帧、不影响黄金路径），门⑤ 重定标到「scheduler 引擎吞吐保持」即达成；或
- **(b) 立项锁争用优化**（分片/striped intake 锁、缩小 intake 临界区、dispatcher 并行化
  或改 condvar 事件驱动）作 scoped follow-up——属 M8 wrapper 的实质并发重构，风险与工作量
  均超出 #68 bench 卡范围；或
- **(c) 明确门⑤ 仅约束 scheduler 引擎**（`WorkerPool` 派发），M8 wrapper 的 RPC 往返延迟
  另设延迟门（p50/p99 per-request）而非吞吐门。

**推荐 (a)+(c)**：门⑤ 的立法意图（风险表 L339：「ureq 阻塞池 16 worker + keep-alive」不被
劣化）已由 before 臂证明达成；M8 wrapper 是更高层抽象，其阻塞-pull 吞吐低于 batch-push 属预期。

## 6. 复现

```text
cargo bench -p cesium-pipeline --offline
```

逐 pass 原始样本见同目录 `before.csv` / `after.csv`
（列：run,jobs,completed,elapsed_ms,throughput_jobs_per_sec）。
