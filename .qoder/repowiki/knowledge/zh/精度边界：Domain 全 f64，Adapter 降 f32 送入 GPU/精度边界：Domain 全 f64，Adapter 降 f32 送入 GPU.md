---
kind: design
name: 精度边界：Domain 全 f64，Adapter 降 f32 送入 GPU
source: session
category: adr
---

# 精度边界：Domain 全 f64，Adapter 降 f32 送入 GPU

_来源：b849cf0 → 56a9b8e 提交周期内记录的编码计划——内容为规划时意图，实现可能滞后或有出入。_

**状态：** accepted

## 背景
地理空间计算（椭球投影、射线相交、三角剖分）对精度敏感，而 GPU 顶点缓冲普遍使用 f32。需要在领域模型与渲染管线之间建立明确的精度转换点，避免在核心算法中混用 f32/f64。

## 决策驱动
- 地理计算精度
- GPU 内存带宽
- 算法一致性

## 备选方案
- **全局 f64，仅在最终写入 GPU 时 cast 到 f32** — 优点：算法精度最高，无需在中间步骤做精度权衡；缺点：GPU 内存占用翻倍，f32 纹理/缓冲区仍需额外转换
- **根据场景动态选择 f32/f64** _（已否决）_ — 优点：小范围场景可用 f32 节省内存；缺点：同一数据结构存在两种版本，代码分支爆炸

## 决策
所有 Domain 类型（Ellipsoid/Cartographic/BoundingSphere/TerrainVertex 等）内部字段统一使用 f64；仅在 adapters/bevy-render 层将 GeometryData/TerrainMesh 转换为 Bevy Mesh 时降精度为 f32，形成清晰的精度边界。

## 影响
地理算法精度与 CesiumJS 一致，便于回归验证；但内存占用约为纯 f32 方案的两倍，需在瓦片缓存策略中考虑；后续若发现热点路径性能瓶颈，可在局部函数内做 f64→f32 优化而非全局改动。