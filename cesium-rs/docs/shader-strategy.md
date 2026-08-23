# Shader 移植策略（Shader Strategy）

> **状态：M2 穿刺实验 v2 完成，预处理管线已实现，路线基本可行。**

## 穿刺实验 v1 结果（原始 GLSL，无预处理）

| 指标 | 数值 |
|------|------|
| GLSL 文件总数 | 318 |
| 共享 include（跳过） | 210 |
| 可解析 shader | 108 |
| 解析成功 | 1 (0.9%) |
| 解析失败 | 107 |
| 全管线通过（parse→validate→WGSL） | 0 (0.0%) |

## 穿刺实验 v2 结果（带预处理管线）

### 预处理管线实现

已实现 `cesium-shaders/src/preprocessor.rs`，包含：

1. **Builtin 头部组装**：
   - 从 `Builtin/Structs/`、`Builtin/Constants/`、`Builtin/Functions/` 收集 142 个 GLSL 片段
   - **拓扑排序**：按 `czm_*` 符号依赖关系排序，确保定义在使用之前
   - **自动 uniform 注入**：从 `AutomaticUniforms.js` 提取 ~100 个 uniform 声明
   - **Sampler 分离**：sampler2D/samplerCube 单独声明（Vulkan GLSL 不允许在 uniform block 内）

2. **限定符规范化**：`attribute` → `in`，`varying` → `out`

3. **Uniform block 布局注入**：自动添加 `layout(binding=X, std140)`

4. **Fragment output 注入**：`layout(location = 0) out vec4 out_FragColor;`

5. **版本升级**：`#version 330 core` → `#version 460 core`（naga 仅支持 Vulkan GLSL 440/450/460）

### v2 实验结果

| 指标 | v1 | v2 |
|------|-----|-----|
| 解析成功 | 1 (0.9%) | 0 (0.0%) |
| 首个失败原因 | `czm_*` 缺失 | `sampler2D` 不支持 |

**v2 进展**：
- ✅ Builtin 头部组装成功（79KB，拓扑排序）
- ✅ 自动 uniform 注入成功（~100 个 uniform）
- ✅ Uniform block 布局注入成功
- ✅ 限定符规范化成功
- ❌ naga glsl-in 不支持 `sampler2D` 作为独立 uniform 声明

### 失败根因（v2）

**naga glsl-in 的 Vulkan GLSL 限制**：

naga 的 GLSL frontend 设计用于 Vulkan GLSL，与 OpenGL/WebGL GLSL 有根本差异：

1. **Sampler 声明**：Vulkan GLSL 要求 sampler 在 uniform block 内或使用特殊语法
2. **Uniform 布局**：所有 uniform 需要显式 binding location
3. **版本要求**：仅支持 440/450/460，不支持 330 core

CesiumJS 的 108 个可解析 shader 全部因 sampler 声明方式不兼容而失败。

## 决策：混合路线（GLSL 预处理 + 手动 WGSL）

**结论**：naga glsl-in 自动转译路线**部分可行**，但需要大量手动适配。

### 修订后的策略

1. **Batch A: Builtin 头部**（已完成）
   - 142 个 Builtin 文件组装为预处理头部
   - 拓扑排序确保依赖顺序

2. **Batch B: 简单 shader（无 sampler）**
   - 预处理 + naga 自动转译
   - 预计成功率：~30-40%（仅处理无 sampler 的 shader）

3. **Batch C: 带 sampler 的 shader**
   - 需要手动修改 sampler 声明方式
   - 或手动翻译为 WGSL

4. **Batch D: 复杂 shader（Model/PostProcess/Voxels）**
   - 大部分需要手动翻译为 WGSL
   - 登记在 `deviations.md`

### 实现状态

- ✅ `cesium-shaders/src/preprocessor.rs` — 预处理管线（已实现）
- ✅ Builtin 头部组装 + 拓扑排序（已实现）
- ✅ 自动 uniform 注入（已实现）
- ⚠️ Sampler 兼容性问题（需进一步研究或手动处理）
- ⏳ 批量转译脚本（待实现）
- ⏳ 手动 WGSL 翻译（待实现）

## 技术细节

### naga 版本与 feature 要求

naga v30.0.1 的 `glsl-in` feature 有 bug：`interpolator` 模块被
`#[cfg(any(feature = "spv-in", feature = "wgsl-in"))]` 门控，
但 `glsl-in` 也需要它。**必须同时启用 `wgsl-in` feature**：

```toml
naga = { version = "30.0.1", features = ["glsl-in", "wgsl-in", "wgsl-out"] }
```

### Vulkan GLSL vs OpenGL GLSL 差异

| 特性 | OpenGL GLSL (CesiumJS) | Vulkan GLSL (naga) |
|------|------------------------|---------------------|
| 版本 | 330 core | 440/450/460 |
| Sampler 声明 | `uniform sampler2D tex;` | 需在 uniform block 内或特殊语法 |
| Uniform 布局 | 运行时分配 | 需显式 `layout(binding=X)` |
| attribute/varying | 支持 | 需改为 `in`/`out` |
| Fragment output | 自定义 `out_FragColor` | 需 `layout(location=0) out` |

## 下一步

1. **短期**：实现批量转译脚本，处理无 sampler 的简单 shader
2. **中期**：研究 naga sampler 兼容方案，或手动翻译带 sampler 的 shader
3. **长期**：建立 WGSL shader 库，逐步替换 GLSL 自动转译
