# M8.5 scheduler 端到端吞吐 — delta report (门⑤)

> 权威判据：计划 L206「scheduler 端到端吞吐 ≥ M1 baseline」；三件套口径 L329（Nora 性能门）。
> 生成者：`adapters/pipeline/benches/scheduler_throughput.rs`（`cargo bench -p cesium-pipeline --offline`）。
> 离线确定性：`InstantMock` 对每个 URL 返回固定 64B，无真实网络 / 无 GPU / 无外网。

## 度量方法

| 臂 | 路径 | 驱动模型 | 计时区 |
|---|---|---|---|
| before (M1 baseline) | `GenericPipeline` (M1.2 `TilePipeline`) | 单驱动 `submit`×N + `poll_ready` drain | submit→drain 完成 |
| after (M8 path) | `PipelineResourceBackend` (M8 `ResourceBackend`) | 32 并发 caller `block_on(request_stream)` | barrier release→全 join |

两臂共用同一 16-worker keep-alive `WorkerPool` + 同一 `InstantMock` + N 个互异冷 key；
pool/backend/driver 线程 spawn 均排除在计时区外。delta = M8 cache 层级(hot `GpuCache`+warm `HiddenLru`)+`Dedup`+dispatcher+waiter 桥的净开销。

## 实测（N=2000 jobs/pass，RUNS=9，单位 jobs/sec）

| 统计 | before (M1) | after (M8) |
|---|---|---|
| median | 1556904.9 | 248815.0 |
| mean | 1591291.8 | 239533.0 |
| stddev | 102223.3 | 38964.0 |
| min | 1419245.0 | 185840.8 |
| max | 1722652.9 | 307309.4 |
| completed (median) | 2000 / 2000 | 2000 / 2000 |

## 门⑤ 判定

- **ratio = after_median / before_median = 0.1598**
- **结论：REGRESSION — after < 95% of M1 baseline**

## 复现

```text
 cargo bench -p cesium-pipeline --offline
 ```

逐 pass 原始样本见同目录 `before.csv` / `after.csv`（列：run,jobs,completed,elapsed_ms,throughput_jobs_per_sec）。
