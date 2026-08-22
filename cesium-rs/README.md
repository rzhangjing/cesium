# cesium-rs

**CesiumJS 一比一 Rust 移植**。将 `packages/engine/Source` 与
`packages/widgets/Source` 的 CesiumJS 引擎逐文件移植为 Rust workspace，
渲染后端使用 **wgpu**（不使用 Bevy），测试镜像 CesiumJS 的 Jasmine Spec。

> 本目录（`cesium-rs/`）与仓库中的 `cesiumrust/`（DDD/Bevy 实验工程）
> **完全隔离**：互不引用代码与依赖。

## 硬性约束

- 渲染后端：wgpu（含 `webgl` feature，覆盖 Web 端 WebGL2 回退）。
- 数值精度：领域计算一律 `f64`，仅在 GPU 提交边界降为 `f32`；不引入 glam。
- 移植规约：见 [docs/PORTING_CONVENTIONS.md](docs/PORTING_CONVENTIONS.md)。

## crate 划分

```text
crates/
  cesium-core           ← packages/engine/Source/Core（无内部依赖）
  cesium-shaders        ← packages/engine/Source/Shaders      (依赖 core)
  cesium-workers        ← packages/engine/Source/Workers      (依赖 core)
  cesium-renderer       ← packages/engine/Source/Renderer     (依赖 core, shaders; wgpu)
  cesium-scene          ← packages/engine/Source/Scene        (依赖 core, renderer, shaders)
  cesium-data-sources   ← packages/engine/Source/DataSources  (依赖 core, scene)
  cesium-widgets        ← Source/Widget + packages/widgets    (依赖 core, scene, data-sources)
  cesium-test-utils     ← 测试支撑（epsilon/ULP 断言、DeveloperError 辅助）
specs/                  ← Jasmine Spec 镜像测试容器（只读引用 ../Specs/Data）
examples/viewer-demo/   ← 最小查看器入口（winit + wgpu 帧循环在 M5 实现）
```

## 依赖方向规则

严格自底向上，**禁止任何反向依赖**（由 Cargo 依赖声明强制）：

```text
cesium-core ──► cesium-shaders ──► cesium-renderer ──► cesium-scene
                                                          │
              cesium-data-sources ◄───────────────────────┤
                       │                                  │
              cesium-widgets ◄────────────────────────────┘
```

即：renderer 不得依赖 scene；scene 不得依赖 data-sources/widgets；
core 不得依赖任何上层 crate。CesiumJS 源码中存在的少量 Core 逆向引用
已登记在 [docs/deferred.md](docs/deferred.md)（M3-S4 回填）。

## 构建与测试

```powershell
cd cesium-rs
cargo build --workspace      # 构建全部 crate
cargo test --workspace       # 运行镜像 spec（含 cesium-test-utils 自测）
cargo run -p viewer-demo     # 占位查看器（M5 起接入真实渲染）
```

测试数据：`specs` crate 通过 `specs_data_root()` 解析
`CESIUM_SPECS_DATA` 环境变量，缺省回退到 workspace 上层的 `Specs/Data`
（只读引用，不复制）。

## 文档

- [docs/PORTING_CONVENTIONS.md](docs/PORTING_CONVENTIONS.md) — 7 条移植规约
- [docs/MAPPING.md](docs/MAPPING.md) — 文件级移植台账（Core 294 文件全清单）
- [docs/deviations.md](docs/deviations.md) — 偏差登记
- [docs/deferred.md](docs/deferred.md) — 推迟事项（Core 逆向依赖回填等）
- [docs/shader-strategy.md](docs/shader-strategy.md) — shader 策略（M2 穿刺实验后定稿）
