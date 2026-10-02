//! glTF 2.0 领域模型。
//!
//! 本模块定义用于解析与处理的核心 glTF JSON 结构，覆盖
//! 根对象、accessor/bufferView/buffer、mesh/primitive、node/scene、
//! texture/material/sampler、skin/animation 与稀疏 accessor 等实体。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 根 glTF 对象。
/// 汇聚场景/节点/网格/accessor/buffer/材质/纹理/动画等顶层集合。
///
/// 映射到 .gltf 文件的顶层 JSON 结构。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GltfModel {
    /// 资源元数据（必需）。
    pub asset: Asset,

    /// 默认 scene 索引。
    #[serde(default)]
    pub scene: Option<usize>,

    /// scene 数组。
    #[serde(default)]
    pub scenes: Vec<Scene>,

    /// node 数组。
    #[serde(default)]
    pub nodes: Vec<Node>,

    /// mesh 数组。
    #[serde(default)]
    pub meshes: Vec<GltfMesh>,

    /// accessor 数组。
    #[serde(default)]
    pub accessors: Vec<Accessor>,

    /// buffer view 数组。
    #[serde(default)]
    pub buffer_views: Vec<BufferView>,

    /// buffer 数组。
    #[serde(default)]
    pub buffers: Vec<Buffer>,

    /// material 数组。
    #[serde(default)]
    pub materials: Vec<Material>,

    /// texture 数组。
    #[serde(default)]
    pub textures: Vec<Texture>,

    /// image 数组。
    #[serde(default)]
    pub images: Vec<Image>,

    /// sampler 数组。
    #[serde(default)]
    pub samplers: Vec<Sampler>,

    /// skin 数组。
    #[serde(default)]
    pub skins: Vec<Skin>,

    /// animation 数组。
    #[serde(default)]
    pub animations: Vec<Animation>,

    /// 本 glTF 中使用的扩展。
    #[serde(default)]
    pub extensions_used: Vec<String>,

    /// 本 glTF 必需的扩展。
    #[serde(default)]
    pub extensions_required: Vec<String>,

    /// 扩展特定数据。
    #[serde(default)]
    pub extensions: Option<serde_json::Value>,

    /// 应用特定数据。
    #[serde(default)]
    pub extras: Option<serde_json::Value>,
}

impl GltfModel {
    /// 从 JSON 字符串解析一个 glTF 模型。
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// 从 JSON 字节解析一个 glTF 模型。
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// 将一个已解析的 JSON [`serde_json::Value`] 转换为强类型
    /// 模型。由 glTF 1.0 → 2.0 升级路径使用
    ///（[`crate::gltf_upgrade::update_version_with_buffers`]）：1.0 JSON 以
    /// 无类型值的形式升级（其以对象为键的集合无法反序列化到本基于
    /// 数组的模型中），仅在此之后才进转换。
    pub fn from_value(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }

    /// 返回默认 scene，若未设置默认则返回第一个 scene。
    pub fn default_scene(&self) -> Option<&Scene> {
        // scene 字段缺省视为索引 0，越界时 get 自然返回 None
        let index = self.scene.unwrap_or(0);
        self.scenes.get(index)
    }

    /// 返回所有 mesh 中的三角形总数。
    pub fn triangle_count(&self) -> usize {
        // 仅统计 Triangles 图元；索引数 / 3 即三角形数
        self.meshes
            .iter()
            .flat_map(|m| m.primitives.iter())
            .filter(|p| p.mode == PrimitiveMode::Triangles)
            .map(|p| {
                p.indices
                    .and_then(|i| self.accessors.get(i))
                    .map(|a| a.count / 3)
                    .unwrap_or(0)
            })
            .sum()
    }

    /// 返回所有 mesh 中的顶点总数。
    pub fn vertex_count(&self) -> usize {
        // 以各图元 POSITION accessor 的元素数累加（未去重共享顶点）
        self.meshes
            .iter()
            .flat_map(|m| m.primitives.iter())
            .filter_map(|p| p.attributes.get("POSITION"))
            .filter_map(|i| self.accessors.get(*i))
            .map(|a| a.count)
            .sum()
    }
}

/// 资源元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    /// glTF 版本（例如 "2.0"）。
    pub version: String,

    /// 所需的最低 glTF 版本。
    #[serde(default)]
    pub min_version: Option<String>,

    /// 生成本 glTF 的工具。
    #[serde(default)]
    pub generator: Option<String>,

    /// 版权信息。
    #[serde(default)]
    pub copyright: Option<String>,
}

impl Default for Asset {
    /// 默认 asset：version 为 "2.0"，其余元信息为空。
    fn default() -> Self {
        Self {
            version: "2.0".to_string(),
            min_version: None,
            generator: None,
            copyright: None,
        }
    }
}

/// 一个包含根 node 列表的 scene。
/// 可为空，具体选择交由渲染端的默认 scene 逻辑处理。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Scene {
    /// 可选名称。
    #[serde(default)]
    pub name: Option<String>,

    /// 根 node 的索引。
    #[serde(default)]
    pub nodes: Vec<usize>,
}

/// 场景图中的一個 node。
/// 既可用 matrix 直接给定变换，也可用 TRS 三分量组合。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    /// 可选名称。
    #[serde(default)]
    pub name: Option<String>,

    /// 子 node 的索引。
    #[serde(default)]
    pub children: Vec<usize>,

    /// 本 node 中 mesh 的索引。
    #[serde(default)]
    pub mesh: Option<usize>,

    /// 本 node 所引用的 skin 索引。
    #[serde(default)]
    pub skin: Option<usize>,

    /// 一个 4x4 变换矩阵（列主序）。
    #[serde(default)]
    pub matrix: Option<[f64; 16]>,

    /// 平移 [x, y, z]。
    #[serde(default)]
    pub translation: Option<[f64; 3]>,

    /// 作为四元数的旋转 [x, y, z, w]。
    #[serde(default)]
    pub rotation: Option<[f64; 4]>,

    /// 缩放 [x, y, z]。
    #[serde(default)]
    pub scale: Option<[f64; 3]>,

    /// 扩展特定数据。
    #[serde(default)]
    pub extensions: Option<serde_json::Value>,
}

impl Node {
    /// 从 TRS 或 matrix 计算局部变换矩阵。
    pub fn local_transform(&self) -> glam::DMat4 {
        // 显式 matrix 优先，直接按列主序构造，忽略 TRS
        if let Some(m) = self.matrix {
            return glam::DMat4::from_cols_array(&m);
        }

        // 缺省分量：平移 0、旋转恒等、缩放 1
        let translation = self
            .translation
            .map(|t| glam::DVec3::new(t[0], t[1], t[2]))
            .unwrap_or(glam::DVec3::ZERO);

        let rotation = self
            .rotation
            .map(|r| glam::DQuat::from_xyzw(r[0], r[1], r[2], r[3]))
            .unwrap_or(glam::DQuat::IDENTITY);

        let scale = self
            .scale
            .map(|s| glam::DVec3::new(s[0], s[1], s[2]))
            .unwrap_or(glam::DVec3::ONE);

        // 按 glTF 约定组合为 S * R * T
        glam::DMat4::from_scale_rotation_translation(scale, rotation, translation)
    }
}

/// 一个包含图元的 mesh。
/// 可附带 morph target 权重，用于形态渐变动画。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GltfMesh {
    /// 可选名称。
    #[serde(default)]
    pub name: Option<String>,

    /// 图元数组。
    pub primitives: Vec<Primitive>,

    /// morph target 权重。
    #[serde(default)]
    pub weights: Vec<f64>,
}

/// mesh 内的一个图元（几何）。
/// 属性表以语义名为键映射到 accessor 索引。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Primitive {
    /// 顶点属性（例如 "POSITION"、"NORMAL"、"TEXCOORD_0"）。
    pub attributes: HashMap<String, usize>,

    /// 包含索引的 accessor 索引。
    #[serde(default)]
    pub indices: Option<usize>,

    /// material 的索引。
    #[serde(default)]
    pub material: Option<usize>,

    /// 拓扑类型（默认：Triangles）。
    #[serde(default)]
    pub mode: PrimitiveMode,

    /// morph target。
    #[serde(default)]
    pub targets: Vec<HashMap<String, usize>>,

    /// 扩展特定数据。
    #[serde(default)]
    pub extensions: Option<serde_json::Value>,
}

/// 图元拓扑模式。
/// 以 OpenGL 绘制模式整数表达点/线/三角形及其变体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrimitiveMode {
    /// 点。
    Points = 0,
    /// 线。
    Lines = 1,
    /// 环线。
    LineLoop = 2,
    /// 线带。
    LineStrip = 3,
    /// 三角形（默认）。
    #[default]
    Triangles = 4,
    /// 三角形带。
    TriangleStrip = 5,
    /// 三角形扇。
    TriangleFan = 6,
}

impl Serialize for PrimitiveMode {
    /// 以 OpenGL 图元整数（0..6）序列化。
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(*self as u8)
    }
}

impl<'de> Deserialize<'de> for PrimitiveMode {
    /// 从整数还原图元模式，未知值兜底为 Triangles。
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u8::deserialize(deserializer)?;
        Ok(match value {
            0 => PrimitiveMode::Points,
            1 => PrimitiveMode::Lines,
            2 => PrimitiveMode::LineLoop,
            3 => PrimitiveMode::LineStrip,
            4 => PrimitiveMode::Triangles,
            5 => PrimitiveMode::TriangleStrip,
            6 => PrimitiveMode::TriangleFan,
            _ => PrimitiveMode::Triangles,
        })
    }
}

/// 用于覆盖特定元素的稀疏 accessor 数据。
///
/// 映射到 glTF 2.0 `accessor.sparse`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessorSparse {
    /// 被覆盖的元素数量。
    pub count: usize,

    /// 要覆盖的元素的索引。
    pub indices: AccessorSparseIndices,

    /// 替换值。
    pub values: AccessorSparseValues,
}

/// 稀疏 accessor 索引。
/// 定位待覆盖元素，索引本身以某种无符号整型分量存放。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessorSparseIndices {
    /// buffer view 的索引。
    pub buffer_view: usize,

    /// buffer view 内的字节偏移。
    #[serde(default)]
    pub byte_offset: usize,

    /// 索引的分量类型（5121=u8、5123=u16、5125=u32）。
    pub component_type: ComponentType,
}

/// 稀疏 accessor 值。
/// 提供与索引对应的替换数据，可缺省 byte_offset 紧密排布。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessorSparseValues {
    /// buffer view 的索引。
    pub buffer_view: usize,

    /// buffer view 内的字节偏移。
    #[serde(default)]
    pub byte_offset: usize,
}

/// 一个用于 buffer 数据的 accessor。
/// 定义如何从 bufferView 按类型/分量类型/count 解读出元素序列。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Accessor {
    /// 可选名称。
    #[serde(default)]
    pub name: Option<String>,

    /// buffer view 的索引。
    #[serde(default)]
    pub buffer_view: Option<usize>,

    /// buffer view 内的字节偏移。
    #[serde(default)]
    pub byte_offset: usize,

    /// 分量的数据类型。
    pub component_type: ComponentType,

    /// 数据是否归一化。
    #[serde(default)]
    pub normalized: bool,

    /// 元素数量。
    pub count: usize,

    /// accessor 的类型（例如 "VEC3"、"SCALAR"）。
    #[serde(rename = "type")]
    pub accessor_type: AccessorType,

    /// 最大值。
    #[serde(default)]
    pub max: Vec<f64>,

    /// 最小值。
    #[serde(default)]
    pub min: Vec<f64>,

    /// 稀疏 accessor 覆盖。
    #[serde(default)]
    pub sparse: Option<AccessorSparse>,
}

impl Accessor {
    /// 返回每个元素的分量数量。
    pub fn components_per_element(&self) -> usize {
        // 各类型分量数：矩阵按元素展开计数（mat2=4、mat3=9、mat4=16）
        match self.accessor_type {
            AccessorType::Scalar => 1,
            AccessorType::Vec2 => 2,
            AccessorType::Vec3 => 3,
            AccessorType::Vec4 => 4,
            AccessorType::Mat2 => 4,
            AccessorType::Mat3 => 9,
            AccessorType::Mat4 => 16,
        }
    }

    /// 返回每个分量的字节大小。
    pub fn component_byte_size(&self) -> usize {
        // 分量位宽：8 位=1、16 位=2、32 位=4
        match self.component_type {
            ComponentType::I8 | ComponentType::U8 => 1,
            ComponentType::I16 | ComponentType::U16 => 2,
            ComponentType::U32 | ComponentType::F32 => 4,
        }
    }

    /// 返回一个元素的总字节 stride。
    pub fn element_byte_size(&self) -> usize {
        // 元素 stride = 每元素分量数 × 每分量字节宽度
        self.components_per_element() * self.component_byte_size()
    }

    /// 若本 accessor 有稀疏覆盖则返回 true。
    pub fn is_sparse(&self) -> bool {
        // 存在 sparse 块即视为稀疏 accessor
        self.sparse.is_some()
    }

    /// 使用本 accessor 从二进制 buffer 读取 f32 数据。
    ///
    /// 逐元素按 stride 采样小端 f32，并在存在稀疏数据时用其覆盖对应项。
    pub fn read_f32_data(&self, buffers: &[Vec<u8>], buffer_views: &[BufferView]) -> Vec<f32> {
        // 输出长度 = 元素数 × 每元素分量数，先全部零初始化
        let total_components = self.count * self.components_per_element();
        let mut data = vec![0.0f32; total_components];

        // 从 buffer view 读取基础数据
        if let Some(bv_idx) = self.buffer_view {
            if let Some(bv) = buffer_views.get(bv_idx) {
                if let Some(buffer) = buffers.get(bv.buffer) {
                    // stride 缺省时按每元素字节数紧密排布
                    let stride = bv.byte_stride.unwrap_or(self.element_byte_size());
                    let base_offset = bv.byte_offset + self.byte_offset;

                    // 逐元素、逐分量按 4 字节小端读取 f32
                    for i in 0..self.count {
                        let elem_offset = base_offset + i * stride;
                        for c in 0..self.components_per_element() {
                            let byte_pos = elem_offset + c * 4;
                            if byte_pos + 4 <= buffer.len() {
                                let bytes = [
                                    buffer[byte_pos],
                                    buffer[byte_pos + 1],
                                    buffer[byte_pos + 2],
                                    buffer[byte_pos + 3],
                                ];
                                data[i * self.components_per_element() + c] =
                                    f32::from_le_bytes(bytes);
                            }
                        }
                    }
                }
            }
        }

        // 应用稀疏覆盖
        if let Some(ref sparse) = self.sparse {
            self.apply_sparse_f32(&mut data, sparse, buffers, buffer_views);
        }

        data
    }

    /// 从二进制 buffer 读取 u16 索引数据。
    pub fn read_u16_data(&self, buffers: &[Vec<u8>], buffer_views: &[BufferView]) -> Vec<u16> {
        // 索引每个元素占 2 字节，stride 缺省为 2
        let mut data = vec![0u16; self.count];

        if let Some(bv_idx) = self.buffer_view {
            if let Some(bv) = buffer_views.get(bv_idx) {
                if let Some(buffer) = buffers.get(bv.buffer) {
                    let stride = bv.byte_stride.unwrap_or(2);
                    let base_offset = bv.byte_offset + self.byte_offset;

                    for (i, item) in data.iter_mut().enumerate().take(self.count) {
                        let byte_pos = base_offset + i * stride;
                        if byte_pos + 2 <= buffer.len() {
                            *item = u16::from_le_bytes([
                                buffer[byte_pos],
                                buffer[byte_pos + 1],
                            ]);
                        }
                    }
                }
            }
        }

        data
    }

    /// 从二进制 buffer 读取 u32 索引数据。
    pub fn read_u32_data(&self, buffers: &[Vec<u8>], buffer_views: &[BufferView]) -> Vec<u32> {
        // 索引每个元素占 4 字节，stride 缺省为 4
        let mut data = vec![0u32; self.count];

        if let Some(bv_idx) = self.buffer_view {
            if let Some(bv) = buffer_views.get(bv_idx) {
                if let Some(buffer) = buffers.get(bv.buffer) {
                    let stride = bv.byte_stride.unwrap_or(4);
                    let base_offset = bv.byte_offset + self.byte_offset;

                    for (i, item) in data.iter_mut().enumerate().take(self.count) {
                        let byte_pos = base_offset + i * stride;
                        if byte_pos + 4 <= buffer.len() {
                            *item = u32::from_le_bytes([
                                buffer[byte_pos],
                                buffer[byte_pos + 1],
                                buffer[byte_pos + 2],
                                buffer[byte_pos + 3],
                            ]);
                        }
                    }
                }
            }
        }

        data
    }

    /// 将稀疏覆盖应用到 f32 数据。
    fn apply_sparse_f32(
        &self,
        data: &mut [f32],
        sparse: &AccessorSparse,
        buffers: &[Vec<u8>],
        buffer_views: &[BufferView],
    ) {
        // 读取稀疏索引
        // 索引个数由 sparse.count 决定，逐条映射到待覆盖的目标元素下标
        let indices = self.read_sparse_indices(sparse, buffers, buffer_views);

        // 读取稀疏值
        if let Some(values_bv) = buffer_views.get(sparse.values.buffer_view) {
            if let Some(buffer) = buffers.get(values_bv.buffer) {
                let components = self.components_per_element();
                let base_offset = values_bv.byte_offset + sparse.values.byte_offset;

                for (sparse_idx, &target_idx) in indices.iter().enumerate() {
                    // 越界目标下标直接跳过，防止写入非法位置
                    if target_idx >= self.count {
                        continue;
                    }
                    for c in 0..components {
                        let byte_pos =
                            base_offset + sparse_idx * components * 4 + c * 4;
                        if byte_pos + 4 <= buffer.len() {
                            let bytes = [
                                buffer[byte_pos],
                                buffer[byte_pos + 1],
                                buffer[byte_pos + 2],
                                buffer[byte_pos + 3],
                            ];
                            let target = target_idx * components + c;
                            // 目标位置仍越界则丢弃该分量写回
                            if target < data.len() {
                                data[target] = f32::from_le_bytes(bytes);
                            }
                        }
                    }
                }
            }
        }
    }

    /// 将稀疏索引读取为 usize 值。
    fn read_sparse_indices(
        &self,
        sparse: &AccessorSparse,
        buffers: &[Vec<u8>],
        buffer_views: &[BufferView],
    ) -> Vec<usize> {
        // 按索引分量类型（U8/U16/U32）小端读取，越界补 0
        let mut indices = Vec::with_capacity(sparse.count);

        if let Some(bv) = buffer_views.get(sparse.indices.buffer_view) {
            if let Some(buffer) = buffers.get(bv.buffer) {
                let base_offset = bv.byte_offset + sparse.indices.byte_offset;

                for i in 0..sparse.count {
                    let idx = match sparse.indices.component_type {
                        ComponentType::U8 => {
                            let pos = base_offset + i;
                            if pos < buffer.len() {
                                buffer[pos] as usize
                            } else {
                                0
                            }
                        }
                        ComponentType::U16 => {
                            let pos = base_offset + i * 2;
                            if pos + 2 <= buffer.len() {
                                u16::from_le_bytes([buffer[pos], buffer[pos + 1]])
                                    as usize
                            } else {
                                0
                            }
                        }
                        ComponentType::U32 => {
                            let pos = base_offset + i * 4;
                            if pos + 4 <= buffer.len() {
                                u32::from_le_bytes([
                                    buffer[pos],
                                    buffer[pos + 1],
                                    buffer[pos + 2],
                                    buffer[pos + 3],
                                ]) as usize
                            } else {
                                0
                            }
                        }
                        _ => 0,
                    };
                    indices.push(idx);
                }
            }
        }

        indices
    }
}

/// 分量数据类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    /// 有符号 8 位整数（5120）。
    I8,
    /// 无符号 8 位整数（5121）。
    U8,
    /// 有符号 16 位整数（5122）。
    I16,
    /// 无符号 16 位整数（5123）。
    U16,
    /// 无符号 32 位整数（5125）。
    U32,
    /// 32 位浮点（5126）。
    F32,
}

impl Serialize for ComponentType {
    /// 以 glTF 规定的 OpenGL 分量类型整数序列化。
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let value: u32 = match self {
            ComponentType::I8 => 5120,
            ComponentType::U8 => 5121,
            ComponentType::I16 => 5122,
            ComponentType::U16 => 5123,
            ComponentType::U32 => 5125,
            ComponentType::F32 => 5126,
        };
        serializer.serialize_u32(value)
    }
}

impl<'de> Deserialize<'de> for ComponentType {
    /// 从分量类型整数还原；未知时兜底为 F32。
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        Ok(match value {
            5120 => ComponentType::I8,
            5121 => ComponentType::U8,
            5122 => ComponentType::I16,
            5123 => ComponentType::U16,
            5125 => ComponentType::U32,
            5126 => ComponentType::F32,
            _ => ComponentType::F32, // 未知时默认为 F32
        })
    }
}

/// accessor 元素类型。
/// 以 SCALAR/VECn/MATn 声明每个元素的分量布局。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessorType {
    /// 单个标量值。
    #[serde(rename = "SCALAR")]
    Scalar,
    /// 2D 向量。
    #[serde(rename = "VEC2")]
    Vec2,
    /// 3D 向量。
    #[serde(rename = "VEC3")]
    Vec3,
    /// 4D 向量。
    #[serde(rename = "VEC4")]
    Vec4,
    /// 2x2 矩阵。
    #[serde(rename = "MAT2")]
    Mat2,
    /// 3x3 矩阵。
    #[serde(rename = "MAT3")]
    Mat3,
    /// 4x4 矩阵。
    #[serde(rename = "MAT4")]
    Mat4,
}

/// buffer 的一个视图。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BufferView {
    /// 可选名称。
    #[serde(default)]
    pub name: Option<String>,

    /// buffer 的索引。
    pub buffer: usize,

    /// buffer 内的字节偏移。
    #[serde(default)]
    pub byte_offset: usize,

    /// 字节长度。
    pub byte_length: usize,

    /// 字节步幅（用于交错数据）。
    #[serde(default)]
    pub byte_stride: Option<usize>,

    /// 目标 buffer 类型。
    #[serde(default)]
    pub target: Option<BufferTarget>,
}

/// buffer 目标类型。
///
/// glTF 将其编码为 OpenGL 枚举整数（34962 = ARRAY_BUFFER、
/// 34963 = ELEMENT_ARRAY_BUFFER），因此（反）序列化是数值式的——
/// derive 出的字符串变体实现会拒绝每一个真实的 `bufferView.target`。
///
/// DEVIATION(fix): 序列化由 derive 字符串变体改为手写数值 34962/34963，
/// 未知整数 Err(invalid_value) 不静默降级；see docs/deviations.md#dev-014
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferTarget {
    /// 数组 buffer（34962）。
    ArrayBuffer,
    /// 元素数组 buffer（34963）。
    ElementArrayBuffer,
}

impl Serialize for BufferTarget {
    /// 以数值 34962/34963 序列化 buffer 目标。
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let value: u32 = match self {
            BufferTarget::ArrayBuffer => 34962,
            BufferTarget::ElementArrayBuffer => 34963,
        };
        serializer.serialize_u32(value)
    }
}

impl<'de> Deserialize<'de> for BufferTarget {
    /// 从 34962/34963 还原 buffer 目标，未知值报错而非静默降级。
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        match value {
            34962 => Ok(BufferTarget::ArrayBuffer),
            34963 => Ok(BufferTarget::ElementArrayBuffer),
            // 未知的整数意味着该 payload 已损坏，或来自我们
            // 未建模的 glTF 变体。直接失败而非静默将其强转为
            // `ArrayBuffer`，否则会掩盖数据损坏，让损坏的
            // `bufferView` 当作有效值继续流动。
            other => Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Unsigned(u64::from(other)),
                &"34962 (ARRAY_BUFFER) or 34963 (ELEMENT_ARRAY_BUFFER)",
            )),
        }
    }
}

#[cfg(test)]
mod buffer_target_tests {
    use super::*;

    fn target_from(json: &str) -> serde_json::Result<BufferTarget> {
        serde_json::from_str(json)
    }

    #[test]
    fn deserializes_array_buffer() {
        assert_eq!(target_from("34962").unwrap(), BufferTarget::ArrayBuffer);
    }

    #[test]
    fn deserializes_element_array_buffer() {
        assert_eq!(
            target_from("34963").unwrap(),
            BufferTarget::ElementArrayBuffer
        );
    }

    #[test]
    fn rejects_unknown_target() {
        // 未知的整数必须作为数据损坏上报，绝不静默
        // 降级为 `ArrayBuffer`。
        assert!(target_from("9999").is_err());
        assert!(target_from("0").is_err());
    }

    #[test]
    fn buffer_view_target_defaults_to_none() {
        // 一个没有 `target` 字段的 `bufferView` 必须保持为 `None`，而非报错。
        let view: BufferView =
            serde_json::from_str(r#"{"buffer":0,"byteLength":12}"#).unwrap();
        assert_eq!(view.target, None);
    }

    #[test]
    fn buffer_view_parses_numeric_target() {
        let view: BufferView =
            serde_json::from_str(r#"{"buffer":0,"byteLength":12,"target":34963}"#).unwrap();
        assert_eq!(view.target, Some(BufferTarget::ElementArrayBuffer));
    }

    #[test]
    fn serialize_roundtrips_numeric() {
        let json = serde_json::to_string(&BufferTarget::ElementArrayBuffer).unwrap();
        assert_eq!(json, "34963");
        assert_eq!(
            target_from(&json).unwrap(),
            BufferTarget::ElementArrayBuffer
        );
    }
}

/// 一个二进制数据 buffer。
/// 嵌入式 GLB 的 buffer 其 uri 为空，数据整体存放在 BIN chunk 中。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Buffer {
    /// 可选名称。
    #[serde(default)]
    pub name: Option<String>,

    /// buffer 数据的 URI（对于嵌入式 GLB 数据为 None）。
    #[serde(default)]
    pub uri: Option<String>,

    /// 字节长度。
    pub byte_length: usize,
}

/// 一个 material 定义。
/// 汇聚 PBR 参数、各纹理槽、自发光与 alpha 混合方式。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Material {
    /// 可选名称。
    #[serde(default)]
    pub name: Option<String>,

    /// PBR metallic-roughness 参数。
    #[serde(default)]
    pub pbr_metallic_roughness: Option<PbrMetallicRoughness>,

    /// 法线贴图 texture info。
    #[serde(default)]
    pub normal_texture: Option<TextureInfo>,

    /// 遮蔽贴图 texture info。
    #[serde(default)]
    pub occlusion_texture: Option<TextureInfo>,

    /// 自发光贴图 texture info。
    #[serde(default)]
    pub emissive_texture: Option<TextureInfo>,

    /// 自发光颜色 [r, g, b]。
    #[serde(default)]
    pub emissive_factor: Option<[f64; 3]>,

    /// alpha 模式。
    #[serde(default)]
    pub alpha_mode: Option<AlphaMode>,

    /// alpha 截断值。
    #[serde(default)]
    pub alpha_cutoff: Option<f64>,

    /// material 是否为双面。
    #[serde(default)]
    pub double_sided: bool,

    /// 扩展特定数据。
    #[serde(default)]
    pub extensions: Option<serde_json::Value>,
}

/// PBR metallic-roughness material 模型。
/// 各因子均为可选，缺省时由渲染端按规范默认值处理。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PbrMetallicRoughness {
    /// 基础颜色 [r, g, b, a]。
    #[serde(default)]
    pub base_color_factor: Option<[f64; 4]>,

    /// 基础颜色贴图。
    #[serde(default)]
    pub base_color_texture: Option<TextureInfo>,

    /// 金属度因子（0.0 到 1.0）。
    #[serde(default)]
    pub metallic_factor: Option<f64>,

    /// 粗糙度因子（0.0 到 1.0）。
    #[serde(default)]
    pub roughness_factor: Option<f64>,

    /// 金属度-粗糙度贴图。
    #[serde(default)]
    pub metallic_roughness_texture: Option<TextureInfo>,
}

/// 带坐标的 texture 引用。
/// 指向 texture 数组的索引并指定使用哪套 UV 坐标集。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureInfo {
    /// texture 的索引。
    pub index: usize,

    /// texture 坐标集。
    #[serde(default)]
    pub tex_coord: usize,
}

/// 组合了 image 与 sampler 的 texture。
/// sampler 与 source 均可缺省，交由渲染端使用默认采样器。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Texture {
    /// 可选名称。
    #[serde(default)]
    pub name: Option<String>,

    /// sampler 的索引。
    #[serde(default)]
    pub sampler: Option<usize>,

    /// image 的索引。
    #[serde(default)]
    pub source: Option<usize>,
}

/// 一个 image 资源。
/// 既可经 uri 引用外部文件，也可经 bufferView 内嵌二进制。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    /// 可选名称。
    #[serde(default)]
    pub name: Option<String>,

    /// image 文件的 URI。
    #[serde(default)]
    pub uri: Option<String>,

    /// MIME 类型。
    #[serde(default)]
    pub mime_type: Option<String>,

    /// 包含该 image 的 buffer view 的索引。
    #[serde(default)]
    pub buffer_view: Option<usize>,
}

/// 一个 texture sampler。
/// 以 OpenGL 枚举整数表达过滤与环绕模式。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sampler {
    /// 可选名称。
    #[serde(default)]
    pub name: Option<String>,

    /// 放大过滤器。
    #[serde(default)]
    pub mag_filter: Option<u32>,

    /// 缩小过滤器。
    #[serde(default)]
    pub min_filter: Option<u32>,

    /// S (U) 环绕模式。
    #[serde(default)]
    pub wrap_s: Option<u32>,

    /// T (V) 环绕模式。
    #[serde(default)]
    pub wrap_t: Option<u32>,
}

/// 用于骨骼动画的 skin。
/// joints 列出参与蒙皮的 node，逆变换绑定矩阵由 accessor 提供。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skin {
    /// 可选名称。
    #[serde(default)]
    pub name: Option<String>,

    /// 包含逆变换绑定矩阵的 accessor 索引。
    #[serde(default)]
    pub inverse_bind_matrices: Option<usize>,

    /// skeleton 根 node 的索引。
    #[serde(default)]
    pub skeleton: Option<usize>,

    /// joint node 的索引。
    pub joints: Vec<usize>,
}

/// 一个 animation。
/// 由若干 channel（目标绑定）与 sampler（关键帧曲线）组成。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Animation {
    /// 可选名称。
    #[serde(default)]
    pub name: Option<String>,

    /// animation channel。
    pub channels: Vec<AnimationChannel>,

    /// animation sampler。
    pub samplers: Vec<AnimationSampler>,
}

/// 一个 animation channel。
/// 将一个 sampler 的输出驱动到指定 node 的某条属性路径。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationChannel {
    /// sampler 的索引。
    pub sampler: usize,

    /// animation 的目标。
    pub target: AnimationTarget,
}

/// animation 目标。
/// 指定被驱动的 node 索引与其上的一条属性路径。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationTarget {
    /// 要驱动的 node 的索引。
    pub node: usize,

    /// 要驱动的属性。
    pub path: AnimationPath,
}

/// animation 属性路径。
/// 可驱动 node 的平移/旋转/缩放或 mesh 的 morph 权重。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnimationPath {
    /// 平移。
    Translation,
    /// 旋转。
    Rotation,
    /// 缩放。
    Scale,
    /// morph 权重。
    Weights,
}

/// 一个 animation sampler。
/// input/output 分别指向时间戳与关键帧值的 accessor。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationSampler {
    /// 包含关键帧时间戳的 accessor 索引。
    pub input: usize,

    /// 包含关键帧值的 accessor 索引。
    pub output: usize,

    /// 插值方法。
    #[serde(default)]
    pub interpolation: Interpolation,
}

/// 插值方法。
/// 决定关键帧之间的取值方式，默认为线性插值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Interpolation {
    /// 线性插值（默认）。
    #[default]
    Linear,
    /// 阶跃插值。
    Step,
    /// 三次样条插值。
    CubicSpline,
}

/// alpha 混合模式。
/// 控制片元透明度处理：不透明、阈值遮罩或 alpha 混合。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AlphaMode {
    /// 不透明（默认）。
    #[default]
    Opaque,
    /// 遮罩（二值透明）。
    Mask,
    /// 混合（alpha 混合）。
    Blend,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_minimal_gltf_json() -> &'static str {
        r#"{
            "asset": { "version": "2.0" },
            "scene": 0,
            "scenes": [{ "nodes": [0] }],
            "nodes": [{ "mesh": 0, "name": "TestNode" }],
            "meshes": [{
                "primitives": [{
                    "attributes": { "POSITION": 0 },
                    "indices": 1,
                    "mode": 4
                }]
            }],
            "accessors": [
                { "componentType": 5126, "count": 3, "type": "VEC3" },
                { "componentType": 5123, "count": 3, "type": "SCALAR" }
            ],
            "bufferViews": [{ "buffer": 0, "byteLength": 44 }],
            "buffers": [{ "byteLength": 44 }]
        }"#
    }

    #[test]
    fn test_parse_minimal_gltf() {
        let json = create_minimal_gltf_json();
        let model = GltfModel::from_json(json).unwrap();

        assert_eq!(model.asset.version, "2.0");
        assert_eq!(model.scenes.len(), 1);
        assert_eq!(model.nodes.len(), 1);
        assert_eq!(model.meshes.len(), 1);
    }

    #[test]
    fn test_default_scene() {
        let json = create_minimal_gltf_json();
        let model = GltfModel::from_json(json).unwrap();

        let scene = model.default_scene().unwrap();
        assert_eq!(scene.nodes, vec![0]);
    }

    #[test]
    fn test_vertex_count() {
        let json = create_minimal_gltf_json();
        let model = GltfModel::from_json(json).unwrap();

        assert_eq!(model.vertex_count(), 3);
    }

    #[test]
    fn test_triangle_count() {
        let json = create_minimal_gltf_json();
        let model = GltfModel::from_json(json).unwrap();

        assert_eq!(model.triangle_count(), 1);
    }

    #[test]
    fn test_node_local_transform_identity() {
        let node = Node::default();
        assert_eq!(node.local_transform(), glam::DMat4::IDENTITY);
    }

    #[test]
    fn test_node_local_transform_trs() {
        let node = Node {
            translation: Some([1.0, 2.0, 3.0]),
            ..Default::default()
        };

        let transform = node.local_transform();
        let translation = transform.w_axis.truncate();
        assert!((translation.x - 1.0).abs() < 1e-10);
        assert!((translation.y - 2.0).abs() < 1e-10);
        assert!((translation.z - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_accessor_element_size() {
        let accessor = Accessor {
            name: None,
            buffer_view: Some(0),
            byte_offset: 0,
            component_type: ComponentType::F32,
            normalized: false,
            count: 100,
            accessor_type: AccessorType::Vec3,
            max: vec![],
            min: vec![],
            sparse: None,
        };

        assert_eq!(accessor.components_per_element(), 3);
        assert_eq!(accessor.component_byte_size(), 4);
        assert_eq!(accessor.element_byte_size(), 12);
    }

    #[test]
    fn test_material_parsing() {
        let json = r#"{
            "asset": { "version": "2.0" },
            "materials": [{
                "name": "TestMaterial",
                "pbrMetallicRoughness": {
                    "baseColorFactor": [1.0, 0.0, 0.0, 1.0],
                    "metallicFactor": 0.5,
                    "roughnessFactor": 0.8
                },
                "doubleSided": true
            }]
        }"#;

        let model = GltfModel::from_json(json).unwrap();
        assert_eq!(model.materials.len(), 1);

        let mat = &model.materials[0];
        assert_eq!(mat.name, Some("TestMaterial".to_string()));
        assert!(mat.double_sided);

        let pbr = mat.pbr_metallic_roughness.as_ref().unwrap();
        assert_eq!(pbr.metallic_factor, Some(0.5));
        assert_eq!(pbr.roughness_factor, Some(0.8));
    }

    #[test]
    fn test_serde_roundtrip() {
        let json = create_minimal_gltf_json();
        let model = GltfModel::from_json(json).unwrap();

        let serialized = serde_json::to_string(&model).unwrap();
        let reparsed = GltfModel::from_json(&serialized).unwrap();

        assert_eq!(model.asset.version, reparsed.asset.version);
        assert_eq!(model.nodes.len(), reparsed.nodes.len());
    }

    #[test]
    fn test_accessor_read_f32_data() {
        // 创建一个包含 3 个 f32 值的 buffer：1.0、2.0、3.0
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&1.0f32.to_le_bytes());
        buffer.extend_from_slice(&2.0f32.to_le_bytes());
        buffer.extend_from_slice(&3.0f32.to_le_bytes());

        let buffers = vec![buffer];
        let buffer_views = vec![BufferView {
            name: None,
            buffer: 0,
            byte_offset: 0,
            byte_length: 12,
            byte_stride: None,
            target: None,
        }];

        let accessor = Accessor {
            name: None,
            buffer_view: Some(0),
            byte_offset: 0,
            component_type: ComponentType::F32,
            normalized: false,
            count: 3,
            accessor_type: AccessorType::Scalar,
            max: vec![],
            min: vec![],
            sparse: None,
        };

        let data = accessor.read_f32_data(&buffers, &buffer_views);
        assert_eq!(data.len(), 3);
        assert!((data[0] - 1.0).abs() < 1e-6);
        assert!((data[1] - 2.0).abs() < 1e-6);
        assert!((data[2] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_accessor_read_f32_vec3() {
        // 2 个 VEC3 元素：(1,2,3) 与 (4,5,6)
        let mut buffer = Vec::new();
        for v in [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0] {
            buffer.extend_from_slice(&v.to_le_bytes());
        }

        let buffers = vec![buffer];
        let buffer_views = vec![BufferView {
            name: None,
            buffer: 0,
            byte_offset: 0,
            byte_length: 24,
            byte_stride: None,
            target: None,
        }];

        let accessor = Accessor {
            name: None,
            buffer_view: Some(0),
            byte_offset: 0,
            component_type: ComponentType::F32,
            normalized: false,
            count: 2,
            accessor_type: AccessorType::Vec3,
            max: vec![],
            min: vec![],
            sparse: None,
        };

        let data = accessor.read_f32_data(&buffers, &buffer_views);
        assert_eq!(data.len(), 6);
        assert!((data[0] - 1.0).abs() < 1e-6);
        assert!((data[3] - 4.0).abs() < 1e-6);
        assert!((data[5] - 6.0).abs() < 1e-6);
    }

    #[test]
    fn test_accessor_read_u16_data() {
        let mut buffer = Vec::new();
        for v in [10u16, 20, 30, 40] {
            buffer.extend_from_slice(&v.to_le_bytes());
        }

        let buffers = vec![buffer];
        let buffer_views = vec![BufferView {
            name: None,
            buffer: 0,
            byte_offset: 0,
            byte_length: 8,
            byte_stride: None,
            target: None,
        }];

        let accessor = Accessor {
            name: None,
            buffer_view: Some(0),
            byte_offset: 0,
            component_type: ComponentType::U16,
            normalized: false,
            count: 4,
            accessor_type: AccessorType::Scalar,
            max: vec![],
            min: vec![],
            sparse: None,
        };

        let data = accessor.read_u16_data(&buffers, &buffer_views);
        assert_eq!(data, vec![10, 20, 30, 40]);
    }

    #[test]
    fn test_accessor_read_u32_data() {
        let mut buffer = Vec::new();
        for v in [100u32, 200, 300] {
            buffer.extend_from_slice(&v.to_le_bytes());
        }

        let buffers = vec![buffer];
        let buffer_views = vec![BufferView {
            name: None,
            buffer: 0,
            byte_offset: 0,
            byte_length: 12,
            byte_stride: None,
            target: None,
        }];

        let accessor = Accessor {
            name: None,
            buffer_view: Some(0),
            byte_offset: 0,
            component_type: ComponentType::U32,
            normalized: false,
            count: 3,
            accessor_type: AccessorType::Scalar,
            max: vec![],
            min: vec![],
            sparse: None,
        };

        let data = accessor.read_u32_data(&buffers, &buffer_views);
        assert_eq!(data, vec![100, 200, 300]);
    }

    #[test]
    fn test_accessor_sparse() {
        // 基础数据：[0.0, 0.0, 0.0]（3 个标量）
        // 稀疏：将索引 1 覆盖为值 5.0
        let mut base_buffer = Vec::new();
        base_buffer.extend_from_slice(&0.0f32.to_le_bytes());
        base_buffer.extend_from_slice(&0.0f32.to_le_bytes());
        base_buffer.extend_from_slice(&0.0f32.to_le_bytes());

        // 稀疏索引 buffer：[1u16]
        let mut idx_buffer = Vec::new();
        idx_buffer.extend_from_slice(&1u16.to_le_bytes());

        // 稀疏值 buffer：[5.0f32]
        let mut val_buffer = Vec::new();
        val_buffer.extend_from_slice(&5.0f32.to_le_bytes());

        let buffers = vec![base_buffer, idx_buffer, val_buffer];
        let buffer_views = vec![
            BufferView {
                name: None,
                buffer: 0,
                byte_offset: 0,
                byte_length: 12,
                byte_stride: None,
                target: None,
            },
            BufferView {
                name: None,
                buffer: 1,
                byte_offset: 0,
                byte_length: 2,
                byte_stride: None,
                target: None,
            },
            BufferView {
                name: None,
                buffer: 2,
                byte_offset: 0,
                byte_length: 4,
                byte_stride: None,
                target: None,
            },
        ];

        let accessor = Accessor {
            name: None,
            buffer_view: Some(0),
            byte_offset: 0,
            component_type: ComponentType::F32,
            normalized: false,
            count: 3,
            accessor_type: AccessorType::Scalar,
            max: vec![],
            min: vec![],
            sparse: Some(AccessorSparse {
                count: 1,
                indices: AccessorSparseIndices {
                    buffer_view: 1,
                    byte_offset: 0,
                    component_type: ComponentType::U16,
                },
                values: AccessorSparseValues {
                    buffer_view: 2,
                    byte_offset: 0,
                },
            }),
        };

        assert!(accessor.is_sparse());
        let data = accessor.read_f32_data(&buffers, &buffer_views);
        assert!((data[0] - 0.0).abs() < 1e-6);
        assert!((data[1] - 5.0).abs() < 1e-6); // 已被覆盖
        assert!((data[2] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_accessor_with_byte_stride() {
        // 交错布局：position(3f) + normal(3f) = 24 字节 stride
        // 只读取 positions（每个 6-float stride 的前 3 个 float）
        let mut buffer = Vec::new();
        // 元素 0：pos(1,2,3) + normal(0,0,1)
        for v in [1.0f32, 2.0, 3.0, 0.0, 0.0, 1.0] {
            buffer.extend_from_slice(&v.to_le_bytes());
        }
        // 元素 1：pos(4,5,6) + normal(0,1,0)
        for v in [4.0f32, 5.0, 6.0, 0.0, 1.0, 0.0] {
            buffer.extend_from_slice(&v.to_le_bytes());
        }

        let buffers = vec![buffer];
        let buffer_views = vec![BufferView {
            name: None,
            buffer: 0,
            byte_offset: 0,
            byte_length: 48,
            byte_stride: Some(24), // 6 个 float * 4 字节
            target: None,
        }];

        let accessor = Accessor {
            name: None,
            buffer_view: Some(0),
            byte_offset: 0,
            component_type: ComponentType::F32,
            normalized: false,
            count: 2,
            accessor_type: AccessorType::Vec3,
            max: vec![],
            min: vec![],
            sparse: None,
        };

        let data = accessor.read_f32_data(&buffers, &buffer_views);
        assert_eq!(data.len(), 6);
        assert!((data[0] - 1.0).abs() < 1e-6);
        assert!((data[1] - 2.0).abs() < 1e-6);
        assert!((data[2] - 3.0).abs() < 1e-6);
        assert!((data[3] - 4.0).abs() < 1e-6);
        assert!((data[4] - 5.0).abs() < 1e-6);
        assert!((data[5] - 6.0).abs() < 1e-6);
    }

    #[test]
    fn test_sparse_accessor_json_parsing() {
        let json = r#"{
            "asset": { "version": "2.0" },
            "accessors": [{
                "componentType": 5126,
                "count": 3,
                "type": "SCALAR",
                "sparse": {
                    "count": 1,
                    "indices": {
                        "bufferView": 1,
                        "componentType": 5123
                    },
                    "values": {
                        "bufferView": 2
                    }
                }
            }],
            "bufferViews": [
                { "buffer": 0, "byteLength": 12 },
                { "buffer": 0, "byteLength": 2, "byteOffset": 12 },
                { "buffer": 0, "byteLength": 4, "byteOffset": 14 }
            ],
            "buffers": [{ "byteLength": 18 }]
        }"#;

        let model = GltfModel::from_json(json).unwrap();
        let accessor = &model.accessors[0];
        assert!(accessor.is_sparse());

        let sparse = accessor.sparse.as_ref().unwrap();
        assert_eq!(sparse.count, 1);
        assert_eq!(sparse.indices.buffer_view, 1);
        assert_eq!(sparse.indices.component_type, ComponentType::U16);
        assert_eq!(sparse.values.buffer_view, 2);
    }
}
