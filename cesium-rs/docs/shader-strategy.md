# Shader 移植策略（Shader Strategy）

> **状态：占位（placeholder）** —— 本文档将在 **M2 里程碑的 GLSL→wgpu
> 穿刺实验（puncture experiment）** 完成后正式填写。

## 背景

CesiumJS 在 `packages/engine/Source/Shaders/` 下有大量 GLSL 源码
（材质、地形、3D Tiles、后处理等），渲染管线通过字符串拼接在运行时
组装 shader。cesium-rs 使用 wgpu，需在以下候选路线中做决策：

1. **GLSL 直通**：保留原 GLSL，经 `naga`（`glsl-in` feature）在加载期
   翻译/验证后交给 wgpu。
2. **翻译为 WGSL**：构建期批量将 GLSL 翻译为 WGSL，运行时直接加载。
3. **人工重写**：逐一手写 WGSL/Rust 生成代码（偏离一比一原则，需登记
   deviations）。

## 穿刺实验待验证项（M2 填写）

- [ ] naga `glsl-in` 对 CesiumJS GLSL（含 `#ifdef`、动态 uniform、
      多 texture unit）的支持程度
- [ ] CesiumJS 运行时 shader 拼接（`ShaderSource`/`createShaderSource`）
      在 wgpu pipeline 模型下的对应方案
- [ ] 精度差异：GLSL `highp float`（实际 fp32）与 wgpu 行为对照
- [ ] 性能基线：翻译开销 vs 运行时开销

## 结论

_（待 M2 穿刺实验完成后填写：选定路线、迁移批次划分、对
`cesium-shaders` crate 的 API 设计影响。）_
