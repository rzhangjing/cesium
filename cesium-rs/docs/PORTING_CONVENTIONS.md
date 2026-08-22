# 移植规约（Porting Conventions）

本文档是 **CesiumJS → cesium-rs 一比一移植** 的强制规约。所有移植 PR
必须逐条对照执行；与规约冲突的实现必须在代码中标注 `// DEVIATION:` 并
登记到 [deviations.md](deviations.md)。

硬性技术约束（不可协商）：

- 渲染后端为 **wgpu**（不使用 Bevy）。
- 领域计算一律 **`f64`**；仅在 GPU 提交边界（`cesium-renderer` 写缓冲
  / uniform 时）降为 `f32`。不引入 `glam`，数学类型自建于 `cesium-core`。
- 与 `cesiumrust/`（DDD/Bevy 实验工程）**完全隔离**：不得引用或复制其
  代码与依赖，仅可参考思路。

---

## 1. 文件镜像（File Mirroring）

CesiumJS 源文件与 Rust 模块 **一一对应**，目录结构镜像
`packages/engine/Source/<模块>`，文件名转 `snake_case`：

| CesiumJS | cesium-rs |
| --- | --- |
| `packages/engine/Source/Core/Cartesian3.js` | `crates/cesium-core/src/cartesian3.rs` |
| `packages/engine/Source/Scene/Primitive.js` | `crates/cesium-scene/src/primitive.rs` |

**示例：** `Source/Core/Cartesian3.js` → `crates/cesium-core/src/cartesian3.rs`，
并在 `cesium-core/src/lib.rs` 中 `pub mod cartesian3;`。

**超大文件例外**：体积 100KB+ 的文件（如 `Scene.js`、`Cesium3DTileset.js`、
`Camera.js`、`GlobeSurfaceTileProvider.js`、`GeometryPipeline.js`、
`Resource.js` 等）允许拆分为 **同名目录子模块**，保持前缀同名：

```text
Source/Scene/Cesium3DTileset.js
  → crates/cesium-scene/src/cesium3d_tileset.rs        # 主入口（mod 聚合 + 核心类型）
  或
  → crates/cesium-scene/src/cesium3d_tileset/
        mod.rs              # 聚合导出，保持与原文件同名入口
        loading.rs
        styling.rs
        ...
```

拆分仅允许按原文件内的逻辑区块切分，**不得**跨文件合并或重命名语义单元。

## 2. 文件头锚定（Source Anchoring）

每个移植后的 `.rs` 文件必须以 crate/模块级注释锚定原始 JS 文件，便于
diff 追踪与后续 CesiumJS 上游同步：

```rust
//! Ported from packages/engine/Source/Core/Cartesian3.js
```

拆分子模块时，`mod.rs` 锚定原 JS 文件，子模块注明所对应的原文件区块：

```rust
//! Ported from packages/engine/Source/Scene/Cesium3DTileset.js
//! (section: tile loading pipeline)
```

## 3. debug 断言裁剪（Debug Pragma Stripping）

CesiumJS 用 pragma 在 release 构建中剥离参数检查：

```js
//>>includeStart('debug', pragmas.debug);
if (!defined(result)) {
  throw new DeveloperError("result is required.");
}
//>>includeEnd('debug');
```

Rust 对应物是编译期条件编译：

```rust
#[cfg(debug_assertions)]
debug_assert!(/* ... */, "result is required.");
// 或者需要抛错语义时：
if cfg!(debug_assertions) {
    throw_developer_error("result is required.");
}
```

原则：`includeStart('debug', ...)` 区块 → `#[cfg(debug_assertions)]` /
`debug_assert!`；**非 debug 区块**的运行时检查必须无条件保留。

## 4. result 复用参数模式（Out-Parameter Pattern）

CesiumJS 为避免 GC 压力大量使用 `result` 出参：

```js
Cartesian3.add(left, right, result); // result 由调用方提供，返回值即 result
```

Rust 映射为 `&mut` 出参 + 单元返回，并配套 `_new` 分配变体：

```rust
impl Cartesian3 {
    /// Port of Cartesian3.add(left, right, result).
    pub fn add(left: &Self, right: &Self, result: &mut Self) {
        result.x = left.x + right.x;
        result.y = left.y + right.y;
        result.z = left.z + right.z;
    }

    /// Allocating variant of [`Self::add`].
    #[must_use]
    pub fn add_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::add(left, right, &mut result);
        result
    }
}
```

规则：

- 出参版本名与 JS 函数名一致（`add`），返回 `()`；
- 分配版本统一加 `_new` 后缀（`add_new`）；
- JS 中 `result` 可选（未传则新建）的动态行为**不**用 `Option<&mut Self>`
  模拟，直接提供两个函数。

## 5. JS 动态特性映射（Dynamic Feature Mapping）

| JS 特性 | Rust 映射 |
| --- | --- |
| `mixin(Class)` | `trait` + blanket/派生实现 |
| getter/setter 属性 | 方法（`fn x(&self)` / `fn set_x(&mut self, ...)`) |
| duck typing | `enum`（封闭集合）或 `trait object`（开放集合） |
| 原型扩展 / monkey patch | 组合（持有被扩展对象的字段/实例） |

**示例（mixin → trait）：**

```js
// CesiumJS: destroyObject is mixed into classes
defineProperties(CesiumWidget.prototype, { canvas: { get: ... } });
```

```rust
pub trait Destroyable {
    fn is_destroyed(&self) -> bool;
    fn destroy(self);
}
pub trait HasCanvas {
    fn canvas(&self) -> &CanvasHandle; // getter -> 方法
}
```

## 6. 偏差标注（Deviation Tagging）

任何无法一比一的实现（语义差异、性能取舍、wgpu 与 WebGL 行为差异、
上游 bug 修正等）必须：

1. 在代码处紧邻标注：

   ```rust
   // DEVIATION: CesiumJS clamps latitude after conversion; we clamp before,
   // see docs/deviations.md#core-cartesian3.
   ```

2. 在 [deviations.md](deviations.md) 登记条目（模块 / 文件 / 偏差描述 /
   原因 / 日期）。

未登记的偏差视为移植缺陷，评审必须打回。

## 7. spec 成对移植（Spec Pairing）

每个源文件与其 Jasmine Spec **同批移植**：移植
`Source/Core/Cartesian3.js` 的同一 PR 必须包含
`specs/tests/core/cartesian3.rs`（镜像
`packages/engine/Specs/Core/Cartesian3Spec.js`）。

- 每个原版 `it(...)` 对应一个 `#[test] fn`，命名 snake_case 直译；
- `toEqualEpsilon` → `assert_approx_eq_f64!`（见 `cesium-test-utils`）；
- `toThrowDeveloperError` → `expect_to_throw_dev_error`；
- 需要 WebGL 上下文的 spec（Renderer/Scene 渲染类）推迟到有 wgpu 离屏
  能力后再移植，并在 [deferred.md](deferred.md) 登记。
