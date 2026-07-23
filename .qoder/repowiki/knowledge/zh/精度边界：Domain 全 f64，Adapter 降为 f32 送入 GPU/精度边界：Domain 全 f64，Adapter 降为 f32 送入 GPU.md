---
kind: design
name: 精度边界：Domain 全 f64，Adapter 降为 f32 送入 GPU
source: session
category: adr
---

# 精度边界：Domain 全 f64，Adapter 降为 f32 送入 GPU

_来源：6049380 → 112c418 提交周期内记录的编码计划——内容为规划时意图，实现可能滞后或有出入。_

**状态：** accepted

## 背景
CesiumJS 使用 JavaScript 的 Number（双精度）处理所有坐标计算，而 GPU shader 普遍使用 f32。Rust 中 glam 同时提供 f32（Vec3）和 f64（DVec3）类型，需要在何处进行精度转换成为关键设计点。

## 决策驱动
- 数值精度一致性（避免 f32 累积误差导致瓦片接缝）
- GPU 兼容性（shader 原生 f32）
- 内存带宽（f32 比 f64 省一半显存）

## 备选方案
- **Domain 全 f64 + Adapter 边界降精度（被采纳）** — 优点：算法层保持与 CesiumJS 一致的精度；仅在最终写入 GPU 前做一次转换；方便做数值对比测试；缺点：每帧在 Adapter 边界有 f64→f32 转换开销
- **全局统一 f32** _（已否决）_ — 优点：无需精度转换，内存占用减半；缺点：大范围坐标（经纬度转笛卡尔）会丢失精度，导致瓦片拼接出现可见缝隙；与 CesiumJS 行为不一致，回归测试困难
- **按需选择精度（核心 f64，局部 f32）** _（已否决）_ — 优点：理论上最优性能；缺点：边界模糊，容易在不知不觉中混用精度；调试时难以定位精度问题来源

## 决策
在所有 domain crate 中使用 glam 的 DVec3/DMat4/DQuat（f64）进行计算，仅在 adapters/bevy-render 中将 GeometryData/TerrainMesh 等中间表示转换为 f32 后交给 Bevy Mesh/Material。

## 影响
算法正确性与 CesiumJS 对齐，瓦片接缝问题最小化；Adapter 层承担精度转换责任，形成清晰的精度边界。f64→f32 转换发生在批量顶点上传路径，对整体性能影响有限。