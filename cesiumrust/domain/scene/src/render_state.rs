//! 渲染状态与 GPU 资源的领域模型。
//!
//! 覆盖渲染状态（剔除/深度/混合/模板/多边形偏移/剪裁）、清除与计算命令、
//! 通道状态，以及纹理、帧缓冲、纹理图集与 GPU 缓冲等资源描述。

use glam::DVec4;
use serde::{Deserialize, Serialize};

/// 背面剔除模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CullFace {
    /// 剔除背面（默认）。
    #[default]
    Back,
    /// 剔除正面。
    Front,
    /// 正反面皆剔除。
    FrontAndBack,
}

/// 模板操作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StencilOp {
    /// 保持模板缓冲原值（默认）。
    #[default]
    Keep,
    /// 将模板值清零。
    Zero,
    /// 用参考值替换模板值。
    Replace,
    /// 模板值加一。
    Increment,
    /// 模板值减一。
    Decrement,
    /// 按位取反模板值。
    Invert,
}

/// 模板测试状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StencilState {
    /// 是否启用模板测试。
    pub enabled: bool,
    /// 正面模板操作。
    pub front_op: StencilOp,
    /// 背面模板操作。
    pub back_op: StencilOp,
    /// 参与比较与替换的参考值。
    pub ref_value: u32,
    /// 比较与写入时使用的位掩码。
    pub mask: u32,
}

impl Default for StencilState {
    /// 默认关闭模板测试，操作为 Keep，参考值 0，掩码取全 1。
    fn default() -> Self {
        Self {
            enabled: false,
            front_op: StencilOp::Keep,
            back_op: StencilOp::Keep,
            ref_value: 0,
            mask: 0xFFFFFFFF,
        }
    }
}

/// 多边形偏移状态。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PolygonOffsetState {
    /// 是否启用多边形偏移。
    pub enabled: bool,
    /// 斜率相关偏移因子。
    pub factor: f32,
    /// 最小深度可分单位偏移。
    pub units: f32,
}

impl Default for PolygonOffsetState {
    /// 默认关闭偏移，因子与单位均为 0。
    fn default() -> Self {
        Self { enabled: false, factor: 0.0, units: 0.0 }
    }
}

/// 剪裁测试状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ScissorState {
    /// 是否启用剪裁测试。
    pub enabled: bool,
    /// 剪裁矩形左下角 x 坐标（像素）。
    pub x: i32,
    /// 剪裁矩形左下角 y 坐标（像素）。
    pub y: i32,
    /// 剪裁矩形宽度（像素）。
    pub width: u32,
    /// 剪裁矩形高度（像素）。
    pub height: u32,
}

/// 完整的渲染状态。
///
/// 聚合剔除、深度、混合、模板、多边形偏移与剪裁等开关及参数。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RenderState {
    /// 是否启用背面剔除。
    pub cull_enabled: bool,
    /// 剔除面选择。
    pub cull_face: CullFace,
    /// 是否启用深度测试。
    pub depth_test_enabled: bool,
    /// 是否允许写入深度缓冲。
    pub depth_write_enabled: bool,
    /// 深度比较函数。
    pub depth_func: DepthFunc,
    /// 是否启用颜色混合。
    pub blend_enabled: bool,
    /// 模板测试状态。
    pub stencil: StencilState,
    /// 多边形偏移状态。
    pub polygon_offset: PolygonOffsetState,
    /// 剪裁测试状态。
    pub scissor: ScissorState,
    /// 线宽（用于线条图元）。
    pub line_width: f32,
}

/// 深度比较函数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DepthFunc {
    /// 从不通过。
    Never,
    /// 小于时通过。
    Less,
    /// 等于时通过。
    Equal,
    /// 小于或等于时通过。
    LessOrEqual,
    /// 大于时通过（默认）。
    #[default]
    Greater,
    /// 不等于时通过。
    NotEqual,
    /// 大于或等于时通过。
    GreaterOrEqual,
    /// 总是通过。
    Always,
}

impl RenderState {
    /// 创建一个默认的不透明渲染状态。
    pub fn opaque() -> Self {
        Self {
            cull_enabled: true,
            cull_face: CullFace::Back,
            depth_test_enabled: true,
            depth_write_enabled: true,
            depth_func: DepthFunc::Less,
            blend_enabled: false,
            ..Default::default()
        }
    }

    /// 创建一个带 alpha 混合的半透明渲染状态。
    pub fn translucent() -> Self {
        Self {
            cull_enabled: true,
            cull_face: CullFace::Back,
            depth_test_enabled: true,
            depth_write_enabled: false,
            depth_func: DepthFunc::Less,
            blend_enabled: true,
            ..Default::default()
        }
    }

    /// 为 2D 创建一个渲染状态（无深度测试）。
    pub fn state_2d() -> Self {
        Self {
            cull_enabled: false,
            depth_test_enabled: false,
            depth_write_enabled: false,
            blend_enabled: true,
            ..Default::default()
        }
    }
}

/// 一个清除命令。
///
/// 指定帧开始时对颜色、深度与模板缓冲的清除值；None 表示不清除该项。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClearCommand {
    /// 颜色清除值（RGBA）；None 表示不清除颜色。
    pub color: Option<DVec4>,
    /// 深度清除值；None 表示不清除深度。
    pub depth: Option<f32>,
    /// 模板清除值；None 表示不清除模板。
    pub stencil: Option<u32>,
}

impl Default for ClearCommand {
    /// 默认清除为不透明黑色、深度 1.0、模板 0。
    fn default() -> Self {
        Self {
            color: Some(DVec4::new(0.0, 0.0, 0.0, 1.0)),
            depth: Some(1.0),
            stencil: Some(0),
        }
    }
}

impl ClearCommand {
    /// 仅清除颜色。
    pub fn color_only(color: DVec4) -> Self {
        Self { color: Some(color), depth: None, stencil: None }
    }

    /// 仅清除深度。
    pub fn depth_only(depth: f32) -> Self {
        Self { color: None, depth: Some(depth), stencil: None }
    }

    /// 清除所有缓冲。
    pub fn all(color: DVec4, depth: f32, stencil: u32) -> Self {
        Self { color: Some(color), depth: Some(depth), stencil: Some(stencil) }
    }
}

/// 用于 GPU 计算操作的计算命令。
///
/// 记录待执行的着色器、线程组划分与逐命名 uniform 取值。
#[derive(Debug, Clone, PartialEq)]
pub struct ComputeCommand {
    /// 计算着色器程序 ID。
    pub shader_id: u64,
    /// 各维度线程组数量（x/y/z）。
    pub work_groups: [u32; 3],
    /// 按名称绑定的 uniform 取值列表。
    pub uniform_map: Vec<(String, ComputeUniformValue)>,
}

/// 计算 uniform 值的类型。
#[derive(Debug, Clone, PartialEq)]
pub enum ComputeUniformValue {
    /// 单精度浮点标量。
    Float(f32),
    /// 二维浮点向量。
    Vec2([f32; 2]),
    /// 三维浮点向量。
    Vec3([f32; 3]),
    /// 四维浮点向量。
    Vec4([f32; 4]),
    /// 有符号整型标量。
    Int(i32),
    /// 无符号整型标量。
    Uint(u32),
    /// 采样引用的纹理 ID。
    Texture(u64),
}

impl ComputeCommand {
    /// 创建计算命令，uniform 列表初始为空。
    pub fn new(shader_id: u64, work_groups: [u32; 3]) -> Self {
        Self {
            shader_id,
            work_groups,
            uniform_map: Vec::new(),
        }
    }

    /// 追加一条按名称绑定的 uniform 取值。
    pub fn set_uniform(&mut self, name: &str, value: ComputeUniformValue) {
        self.uniform_map.push((name.to_string(), value));
    }
}

/// 一个渲染通道的通道状态。
///
/// 封装当前生效的渲染状态、目标帧缓冲与视口矩形。
#[derive(Debug, Clone, PartialEq)]
pub struct PassState {
    /// 当前生效的渲染状态。
    pub render_state: RenderState,
    /// 目标帧缓冲 ID；None 表示默认交换链。
    pub framebuffer_id: Option<u64>,
    /// 视口矩形 [x, y, width, height]。
    pub viewport: [i32; 4],
}

impl Default for PassState {
    /// 默认渲染状态、无帧缓冲、视口 1920x1080。
    fn default() -> Self {
        Self {
            render_state: RenderState::default(),
            framebuffer_id: None,
            viewport: [0, 0, 1920, 1080],
        }
    }
}

/// 纹理像素格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PixelFormat {
    #[default]
    Rgba,
    Rgb,
    Rg,
    Red,
    Depth,
    DepthStencil,
}

impl PixelFormat {
    /// 返回该格式每个像素的分量数量。
    ///
    /// 依像素布局返回 1~4，供步长与翻转计算复用。
    pub fn components_per_pixel(&self) -> usize {
        match self {
            Self::Rgba => 4,
            Self::Rgb => 3,
            Self::Rg => 2,
            Self::Red => 1,
            Self::Depth => 1,
            Self::DepthStencil => 1,
        }
    }

    /// 将像素数据沿垂直方向（Y 轴）翻转。
    ///
    /// 逐行倒序拷贝，用于图像来源与 GL 纹理坐标约定不一致时校正。
    pub fn flip_y(data: &[u8], format: PixelFormat, width: usize, height: usize) -> Vec<u8> {
        if height == 1 {
            return data.to_vec();
        }
        let components = format.components_per_pixel();
        let row_bytes = width * components;
        let mut result = vec![0u8; data.len()];
        for row in 0..height {
            let src_offset = row * row_bytes;
            let dst_offset = (height - 1 - row) * row_bytes;
            result[dst_offset..dst_offset + row_bytes]
                .copy_from_slice(&data[src_offset..src_offset + row_bytes]);
        }
        result
    }
}

/// 纹理数据类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PixelDatatype {
    /// 每分量 8 位无符号整数（默认）。
    #[default]
    UnsignedByte,
    /// 每分量 32 位浮点。
    Float,
    /// 每分量 16 位半精度浮点。
    HalfFloat,
    /// 每分量 16 位无符号整数。
    UnsignedShort,
    /// 每分量 32 位无符号整数。
    UnsignedInt,
}

/// 纹理过滤器。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TextureFilter {
    /// 线性过滤（默认）。
    #[default]
    Linear,
    /// 最近邻过滤。
    Nearest,
    /// 线性过滤 + 线性 Mipmap 过渡。
    LinearMipmapLinear,
    /// 线性过滤 + 最近邻 Mipmap 过渡。
    LinearMipmapNearest,
    /// 最近邻过滤 + 线性 Mipmap 过渡。
    NearestMipmapLinear,
    /// 最近邻过滤 + 最近邻 Mipmap 过渡。
    NearestMipmapNearest,
}

/// 纹理环绕模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TextureWrap {
    /// 超出范围时钳制到边缘（默认）。
    #[default]
    ClampToEdge,
    /// 超出范围时重复平铺。
    Repeat,
    /// 超出范围时镜像重复。
    MirroredRepeat,
}

/// 一个纹理资源（领域表示）。
///
/// 描述尺寸、像素格式/数据类型、过滤与环绕方式以及是否生成 Mipmap。
#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    /// 纹理资源 ID。
    pub id: u64,
    /// 像素宽度。
    pub width: u32,
    /// 像素高度。
    pub height: u32,
    /// 像素格式（通道布局）。
    pub format: PixelFormat,
    /// 每分量的数据类型。
    pub datatype: PixelDatatype,
    /// 缩小过滤器。
    pub min_filter: TextureFilter,
    /// 放大过滤器。
    pub mag_filter: TextureFilter,
    /// 横向（S）环绕方式。
    pub wrap_s: TextureWrap,
    /// 纵向（T）环绕方式。
    pub wrap_t: TextureWrap,
    /// 是否生成 Mipmap 链。
    pub generate_mipmaps: bool,
}

impl Texture {
    /// 以 RGBA/UnsignedByte/Linear/ClampToEdge 默认值新建纹理。
    pub fn new(id: u64, width: u32, height: u32) -> Self {
        Self {
            id,
            width,
            height,
            format: PixelFormat::Rgba,
            datatype: PixelDatatype::UnsignedByte,
            min_filter: TextureFilter::Linear,
            mag_filter: TextureFilter::Linear,
            wrap_s: TextureWrap::ClampToEdge,
            wrap_t: TextureWrap::ClampToEdge,
            generate_mipmaps: false,
        }
    }

    /// 开启 Mipmap 生成并将缩小过滤器设为三线性。
    pub fn with_mipmaps(mut self) -> Self {
        self.generate_mipmaps = true;
        self.min_filter = TextureFilter::LinearMipmapLinear;
        self
    }
}

/// 一个帧缓冲资源（领域表示）。
///
/// 记录挂载的颜色/深度/模板纹理及整体尺寸，作为离屏渲染目标。
#[derive(Debug, Clone, PartialEq)]
pub struct Framebuffer {
    /// 帧缓冲资源 ID。
    pub id: u64,
    /// 挂载的颜色纹理 ID 列表。
    pub color_textures: Vec<u64>,
    /// 深度纹理 ID；None 表示无深度附件。
    pub depth_texture: Option<u64>,
    /// 模板纹理 ID；None 表示无模板附件。
    pub stencil_texture: Option<u64>,
    /// 帧缓冲像素宽度。
    pub width: u32,
    /// 帧缓冲像素高度。
    pub height: u32,
}

impl Framebuffer {
    /// 新建空附件的帧缓冲，附件经 attach_* 逐步挂载。
    pub fn new(id: u64, width: u32, height: u32) -> Self {
        Self {
            id,
            color_textures: Vec::new(),
            depth_texture: None,
            stencil_texture: None,
            width,
            height,
        }
    }

    /// 追加一个颜色附件纹理。
    pub fn attach_color(&mut self, texture_id: u64) {
        self.color_textures.push(texture_id);
    }

    /// 设置深度附件纹理。
    pub fn attach_depth(&mut self, texture_id: u64) {
        self.depth_texture = Some(texture_id);
    }
}

/// 用于批量处理小纹理的纹理图集。
///
/// 在一张大纹理内按行紧凑排布多个子区域，减少纹理切换开销。
#[derive(Debug, Clone, PartialEq)]
pub struct TextureAtlas {
    /// 图集资源 ID。
    pub id: u64,
    /// 承载全部子区域的底层大纹理。
    pub texture: Texture,
    /// 已排布的条目列表。
    pub entries: Vec<TextureAtlasEntry>,
    /// 条目之间及边缘的留白像素。
    pub padding: u32,
}

/// 纹理图集中的一个条目。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureAtlasEntry {
    /// 子区域左上角 x 坐标。
    pub x: u32,
    /// 子区域左上角 y 坐标。
    pub y: u32,
    /// 子区域宽度（像素）。
    pub width: u32,
    /// 子区域高度（像素）。
    pub height: u32,
}

impl TextureAtlas {
    /// 新建指定尺寸与留白的图集，内部大纹理随之创建。
    pub fn new(id: u64, width: u32, height: u32, padding: u32) -> Self {
        Self {
            id,
            texture: Texture::new(id, width, height),
            entries: Vec::new(),
            padding,
        }
    }

    /// 向图集添加一个条目（简单的基于行的packing）。
    pub fn add_entry(&mut self, width: u32, height: u32) -> Option<TextureAtlasEntry> {
        let mut x = self.padding;
        let mut y = self.padding;
        let mut row_height = 0u32;

        for entry in &self.entries {
            if x + width + self.padding <= self.texture.width {
                // 检查是否能放入当前行
                if entry.y == y {
                    x = x.max(entry.x + entry.width + self.padding);
                    row_height = row_height.max(entry.height);
                }
            }
        }

        if x + width + self.padding > self.texture.width {
            // 移动到下一行
            x = self.padding;
            y += row_height + self.padding;
        }

        if y + height + self.padding > self.texture.height {
            return None; // 图集已满
        }

        let entry = TextureAtlasEntry { x, y, width, height };
        self.entries.push(entry);
        Some(entry)
    }

    /// 返回已排布条目数量。
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

/// GPU 缓冲用途。
///
/// 提示驱动的存储策略：按数据更新频率选择静态/动态/流式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BufferUsage {
    /// 很少更新，存于最优读路径（默认）。
    #[default]
    StaticDraw,
    /// 频繁更新。
    DynamicDraw,
    /// 每帧或一次性上传后即弃用。
    StreamDraw,
}

/// 一个 GPU 缓冲（领域表示）。
///
/// 以字节大小与用途描述一块顶点/索引/统一缓冲。
#[derive(Debug, Clone, PartialEq)]
pub struct GpuBuffer {
    /// 缓冲资源 ID。
    pub id: u64,
    /// 缓冲总字节数。
    pub size_in_bytes: usize,
    /// 更新频率与存储提示。
    pub usage: BufferUsage,
}

impl GpuBuffer {
    /// 新建指定大小与用途的 GPU 缓冲。
    pub fn new(id: u64, size_in_bytes: usize, usage: BufferUsage) -> Self {
        Self { id, size_in_bytes, usage }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_state_presets() {
        let opaque = RenderState::opaque();
        assert!(opaque.cull_enabled);
        assert!(opaque.depth_test_enabled);
        assert!(opaque.depth_write_enabled);
        assert!(!opaque.blend_enabled);

        let translucent = RenderState::translucent();
        assert!(translucent.blend_enabled);
        assert!(!translucent.depth_write_enabled);

        let state_2d = RenderState::state_2d();
        assert!(!state_2d.cull_enabled);
        assert!(!state_2d.depth_test_enabled);
    }

    #[test]
    fn test_clear_command() {
        let clear = ClearCommand::default();
        assert!(clear.color.is_some());
        assert!(clear.depth.is_some());
        assert!(clear.stencil.is_some());

        let color_only = ClearCommand::color_only(DVec4::new(1.0, 0.0, 0.0, 1.0));
        assert!(color_only.color.is_some());
        assert!(color_only.depth.is_none());
    }

    #[test]
    fn test_compute_command() {
        let mut cmd = ComputeCommand::new(0, [64, 1, 1]);
        cmd.set_uniform("u_scale", ComputeUniformValue::Float(2.0));
        assert_eq!(cmd.work_groups, [64, 1, 1]);
        assert_eq!(cmd.uniform_map.len(), 1);
    }

    #[test]
    fn test_pass_state() {
        let state = PassState::default();
        assert_eq!(state.viewport, [0, 0, 1920, 1080]);
        assert!(state.framebuffer_id.is_none());
    }

    #[test]
    fn test_texture() {
        let tex = Texture::new(0, 256, 256).with_mipmaps();
        assert_eq!(tex.width, 256);
        assert!(tex.generate_mipmaps);
        assert_eq!(tex.min_filter, TextureFilter::LinearMipmapLinear);
    }

    #[test]
    fn test_framebuffer() {
        let mut fb = Framebuffer::new(0, 1024, 768);
        fb.attach_color(1);
        fb.attach_depth(2);
        assert_eq!(fb.color_textures.len(), 1);
        assert_eq!(fb.depth_texture, Some(2));
    }

    #[test]
    fn test_texture_atlas() {
        let mut atlas = TextureAtlas::new(0, 512, 512, 2);
        let e1 = atlas.add_entry(64, 64);
        assert!(e1.is_some());
        let e2 = atlas.add_entry(64, 64);
        assert!(e2.is_some());
        assert_eq!(atlas.entry_count(), 2);
        // 条目不应重叠
        let e1 = e1.unwrap();
        let e2 = e2.unwrap();
        assert!(e1.x + e1.width + 2 <= e2.x || e2.x + e2.width + 2 <= e1.x || e1.y != e2.y);
    }

    #[test]
    fn test_gpu_buffer() {
        let buf = GpuBuffer::new(0, 1024, BufferUsage::DynamicDraw);
        assert_eq!(buf.size_in_bytes, 1024);
        assert_eq!(buf.usage, BufferUsage::DynamicDraw);
    }
}
