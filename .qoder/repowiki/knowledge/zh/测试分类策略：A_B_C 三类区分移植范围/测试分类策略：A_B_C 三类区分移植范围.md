---
kind: design
name: 测试分类策略：A/B/C 三类区分移植范围
source: session
category: adr
---

# 测试分类策略：A/B/C 三类区分移植范围

_来源：5a0e37f → aff355e 提交周期内记录的编码计划——内容为规划时意图，实现可能滞后或有出入。_

**状态：** accepted

## 背景
Rust 移植包含纯数学/地理/几何/时间等无副作用逻辑，也包含 glam 委托层和依赖浏览器/DOM/网络的渲染层，需要明确边界以避免把浏览器专属代码当纯逻辑移植或重复测试第三方库。

## 决策驱动
- 避免重复测试 glam 内部
- 不引入 mock 污染纯逻辑验证
- 聚焦可移植的纯算法

## 备选方案
- **全量移植所有 Spec** _（已否决）_ — 优点：覆盖面广；缺点：Renderer/WebGL/DOM/网络等 C 类无法在无浏览器环境运行，会引入大量 mock
- **A/B/C 分类：A 类 100% 移植，B 类只验包装，C 类标注不可移植** — 优点：边界清晰；glam 内部由 crate 自身保障；C 类不影响覆盖率统计；缺点：需维护分类清单

## 决策
将 Spec 划分为 A 类（纯逻辑，必须 100% 移植）、B 类（委托 glam 的 Cartesian/Matrix/Quaternion 等，仅验证 CesiumJS 特有包装如 `fromDegrees`、`multiply` 语义差异）、C 类（geocoder、影像/地形 provider、Renderer/WebGL、Widget/DOM、ScreenSpaceEventHandler、canvas 文本等，标注不可移植原因）。

## 影响
覆盖率矩阵仅统计 A/B 类；C 类单独列出不可移植原因；RequestScheduler 等纯算法部分可从 C 类抽离后按 A 类处理。