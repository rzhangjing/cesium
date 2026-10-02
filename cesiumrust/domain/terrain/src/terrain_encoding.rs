//! 地形顶点编码/解码。
//!
//! 用于量化和打包地形网格的数据。位置可被解包以用于拾取，
//! 所有属性在顶点着色器中解包。

// 历史遗留的风格债（见 deferred.md #18）；在 M13 lint-cleanup 或本文件被重写时重新检视
#![allow(clippy::assign_op_pattern)]
use crate::TerrainQuantization;
use cesium_geospatial::attribute_compression::{
    compress_texture_coordinates, decompress_texture_coordinates, oct_pack_float,
};
use cesium_geospatial::bounding::AxisAlignedBoundingBox;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::transforms::inverse_transformation;
use cesium_geospatial::vertical_exaggeration;
use glam::{DMat4, DVec2, DVec3};

const SHIFT_LEFT_12: f64 = 4096.0;

/// 顶点缓冲区中分量的数据类型大小（以字节计）（FLOAT = 4 字节）。
const FLOAT_SIZE_IN_BYTES: usize = 4;

/// 存储在地形顶点缓冲区中单个属性的描述符。
///
/// 记录属性的着色器索引、分量数、字节偏移与步长，供着色器布局使用。
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainAttribute {
    /// 着色器中的属性索引（位置）。
    pub index: u32,
    /// 每个顶点属性的分量数。
    pub components_per_attribute: u32,
    /// 该属性在顶点内的字节偏移。
    pub offset_in_bytes: usize,
    /// 相邻顶点之间的字节步长。
    pub stride_in_bytes: usize,
}

/// 指向顶点缓冲区中属性位置的索引。
///
/// 按量化方式给出各属性使用的着色器索引，便于拾取时定位分量。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainAttributeLocations {
    /// position 3D + height 属性的索引（NONE 量化）。
    pub position_3d_and_height: u32,
    /// texture coordinate + encoded normals 属性的索引（NONE 量化）。
    pub texture_coord_and_encoded_normals: u32,
    /// compressed0 属性的索引（BITS12 量化）。
    pub compressed0: u32,
    /// compressed1 属性的索引（BITS12 量化）。
    pub compressed1: u32,
    /// geodetic surface normal 属性的索引。
    pub geodetic_surface_normal: u32,
}

const ATTRIBUTES_INDICES_NONE: TerrainAttributeLocations = TerrainAttributeLocations {
    position_3d_and_height: 0,
    texture_coord_and_encoded_normals: 1,
    geodetic_surface_normal: 2,
    compressed0: 0,
    compressed1: 1,
};

const ATTRIBUTES_INDICES_BITS12: TerrainAttributeLocations = TerrainAttributeLocations {
    position_3d_and_height: 0,
    texture_coord_and_encoded_normals: 1,
    compressed0: 0,
    compressed1: 1,
    geodetic_surface_normal: 2,
};

/// 用于量化和打包地形网格的数据。位置可被解包以用于拾取，
/// 所有属性在顶点着色器中解包。
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainEncoding {
    /// 网格顶点的压缩方式。
    pub quantization: TerrainQuantization,
    /// 图块的最小高度（含裙边）。
    pub minimum_height: Option<f64>,
    /// 图块的最大高度。
    pub maximum_height: Option<f64>,
    /// 图块的中心。
    pub center: Option<DVec3>,
    /// 一个矩阵，将顶点从图块变换到中心处的 east-north-up 坐标系，
    /// 并缩放使其每个分量处于 [0, 1] 范围。
    pub to_scaled_enu: Option<DMat4>,
    /// 一个矩阵，将经 toScaledENU 变换的顶点还原回地固参考系。
    pub from_scaled_enu: Option<DMat4>,
    /// 用于在着色器中为 RTE 渲染解压地形顶点的矩阵。
    pub matrix: Option<DMat4>,
    /// 地形网格包含法线。
    pub has_vertex_normals: bool,
    /// 地形网格包含遵循 Web Mercator 投影的垂直纹理坐标。
    pub has_web_mercator_t: bool,
    /// 地形网格包含大地测量表面法线，用于地形夸张。
    pub has_geodetic_surface_normals: bool,
    /// 用于夸张地形的标量。
    pub exaggeration: f64,
    /// 地形夸张所基于的相对高度。
    pub exaggeration_relative_height: f64,
    /// 每个顶点的分量数。该值随不同量化方式而异。
    pub stride: usize,

    /// 大地测量表面法线在顶点内的分量偏移（无该属性时为 0）。
    offset_geodetic_surface_normal: usize,
    /// oct 编码顶点法线在顶点内的分量偏移（无法线时为 0）。
    offset_vertex_normal: usize,
}

impl Default for TerrainEncoding {
    /// 返回 NONE 量化、无可选属性、夸张 1.0 的缺省编码。
    fn default() -> Self {
        // 先填字段，再据量化方式计算步长与各属性偏移。
        let mut encoding = Self {
            quantization: TerrainQuantization::None,
            minimum_height: None,
            maximum_height: None,
            center: None,
            to_scaled_enu: None,
            from_scaled_enu: None,
            matrix: None,
            has_vertex_normals: false,
            has_web_mercator_t: false,
            has_geodetic_surface_normals: false,
            exaggeration: 1.0,
            exaggeration_relative_height: 0.0,
            stride: 0,
            offset_geodetic_surface_normal: 0,
            offset_vertex_normal: 0,
        };
        encoding.calculate_stride_and_offsets();
        encoding
    }
}

impl TerrainEncoding {
    /// 使用默认选项（无 web mercator T、无大地测量表面法线、夸张 1.0）
    /// 从轴对齐包围盒创建地形编码。
    ///
    /// 等价于以六个位置参调用 [`new`](Self::new)，其余选项取默认值。
    #[allow(clippy::too_many_arguments)]
    pub fn from_aabb(
        center: DVec3,
        axis_aligned_bounding_box: &AxisAlignedBoundingBox,
        minimum_height: f64,
        maximum_height: f64,
        from_enu: DMat4,
        has_vertex_normals: bool,
    ) -> Self {
        Self::new(
            center,
            axis_aligned_bounding_box,
            minimum_height,
            maximum_height,
            from_enu,
            has_vertex_normals,
            false,
            false,
            1.0,
            0.0,
        )
    }

    /// 从轴对齐包围盒创建地形编码。
    ///
    /// 依据包围盒与中心构造缩放 ENU 变换矩阵，并计算步长与偏移。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        center: DVec3,
        axis_aligned_bounding_box: &AxisAlignedBoundingBox,
        minimum_height: f64,
        maximum_height: f64,
        from_enu: DMat4,
        has_vertex_normals: bool,
        has_web_mercator_t: bool,
        has_geodetic_surface_normals: bool,
        exaggeration: f64,
        exaggeration_relative_height: f64,
    ) -> Self {
        let minimum = axis_aligned_bounding_box.minimum;
        let maximum = axis_aligned_bounding_box.maximum;

        // 从 [0,1] 到 [ENU min, ENU max] 的缩放和偏移。
        // 同时计算缩放和偏移的逆。
        let dimensions = maximum - minimum;
        let h_dim = maximum_height - minimum_height;
        // 取三维尺寸与高度区间的最长者，据此判断能否用 12 位量化。
        let max_dim = dimensions.max_element().max(h_dim);

        // 尺寸小于 2^12-1 时可压入 12 位，否则保留完整精度。
        let quantization = if max_dim < SHIFT_LEFT_12 - 1.0 {
            TerrainQuantization::Bits12
        } else {
            TerrainQuantization::None
        };

        // st 把 [0,1] 单位立方体缩放平移至 ENU 包围盒。
        let mut st = DMat4::from_scale(dimensions);
        st.w_axis = minimum.extend(1.0);

        // 逆缩放与 st 相反，把包围盒还原回 [0,1] 单位空间。
        let inv_scale = DMat4::from_scale(DVec3::new(
            1.0 / dimensions.x,
            1.0 / dimensions.y,
            1.0 / dimensions.z,
        ));
        let inv_st = inv_scale * DMat4::from_translation(-minimum);

        // 以瓦片中心为参考点做 RTE，降低 float32 渲染精度损失。
        let rtc_offset = from_enu.w_axis.truncate() - center;
        let mut matrix = from_enu;
        matrix.w_axis = rtc_offset.extend(1.0);
        matrix = matrix * st;

        // 双向矩阵：地固↔缩放 ENU，供编码压缩与解码还原。
        let to_scaled_enu = inv_st * inverse_transformation(&from_enu);
        let from_scaled_enu = from_enu * st;

        let mut encoding = Self {
            quantization,
            minimum_height: Some(minimum_height),
            maximum_height: Some(maximum_height),
            center: Some(center),
            to_scaled_enu: Some(to_scaled_enu),
            from_scaled_enu: Some(from_scaled_enu),
            matrix: Some(matrix),
            has_vertex_normals,
            has_web_mercator_t,
            has_geodetic_surface_normals,
            exaggeration,
            exaggeration_relative_height,
            stride: 0,
            offset_geodetic_surface_normal: 0,
            offset_vertex_normal: 0,
        };
        encoding.calculate_stride_and_offsets();
        encoding
    }

    /// 计算采样顶点缓冲区的步长和偏移。
    ///
    /// 依量化方式确定位置分量数，再按可选属性依次累加步长并记录偏移。
    fn calculate_stride_and_offsets(&mut self) {
        // Bits12 位置压成 3 分量，完整精度则占 6 分量。
        let mut vertex_stride = 0usize;

        match self.quantization {
            TerrainQuantization::Bits12 => vertex_stride += 3,
            _ => vertex_stride += 6,
        }
        if self.has_web_mercator_t {
            vertex_stride += 1;
        }
        if self.has_vertex_normals {
            self.offset_vertex_normal = vertex_stride;
            vertex_stride += 1;
        }
        if self.has_geodetic_surface_normals {
            self.offset_geodetic_surface_normal = vertex_stride;
            vertex_stride += 3;
        }

        self.stride = vertex_stride;
    }

    /// 将给定位置处的地形信息编码进顶点缓冲区。
    /// 位置、纹理坐标、高度，以及（可选的）法线、投影
    /// 信息和大地测量表面法线都打包进同一个缓冲区。
    ///
    /// 值被推入 `vertex_buffer`。返回新的缓冲区长度。
    ///
    /// 按量化方式将位置/纹理/高度（及可选法线、投影、表面法线）打包进同一缓冲区。
    #[allow(clippy::too_many_arguments)]
    pub fn encode(
        &self,
        vertex_buffer: &mut Vec<f64>,
        position: DVec3,
        uv: DVec2,
        height: f64,
        normal_to_pack: Option<DVec2>,
        web_mercator_t: Option<f64>,
        geodetic_surface_normal: Option<DVec3>,
    ) -> usize {
        let u = uv.x;
        let v = uv.y;

        if self.quantization == TerrainQuantization::Bits12 {
            // BITS12：先把位置变换到缩放 ENU 空间并逐轴钳制到 [0,1]。
            let to_scaled_enu = self.to_scaled_enu.unwrap();
            let mut position = to_scaled_enu.transform_point3(position);
            position.x = position.x.clamp(0.0, 1.0);
            position.y = position.y.clamp(0.0, 1.0);
            position.z = position.z.clamp(0.0, 1.0);

            // 高度按最小/最大区间归一化到 [0,1]，以便与位置同法压缩。
            let minimum_height = self.minimum_height.unwrap();
            let maximum_height = self.maximum_height.unwrap();
            let h_dim = maximum_height - minimum_height;
            let h = ((height - minimum_height) / h_dim).clamp(0.0, 1.0);

            // 每两个 [0,1] 分量交错压缩进单个 f64。
            let compressed0 = compress_texture_coordinates(DVec2::new(position.x, position.y));
            let compressed1 = compress_texture_coordinates(DVec2::new(position.z, h));
            let compressed2 = compress_texture_coordinates(DVec2::new(u, v));

            vertex_buffer.push(compressed0);
            vertex_buffer.push(compressed1);
            vertex_buffer.push(compressed2);

            if self.has_web_mercator_t {
                let compressed3 =
                    compress_texture_coordinates(DVec2::new(web_mercator_t.unwrap_or(0.0), 0.0));
                vertex_buffer.push(compressed3);
            }
        } else {
            // NONE：位置减去中心做 RTE，连同高度与纹理直存原始分量。
            let center = self.center.unwrap();
            vertex_buffer.push(position.x - center.x);
            vertex_buffer.push(position.y - center.y);
            vertex_buffer.push(position.z - center.z);
            vertex_buffer.push(height);
            vertex_buffer.push(u);
            vertex_buffer.push(v);

            if self.has_web_mercator_t {
                vertex_buffer.push(web_mercator_t.unwrap_or(0.0));
            }
        }

        // 可选属性依步长顺序追加到顶点尾部。
        if self.has_vertex_normals {
            vertex_buffer.push(oct_pack_float(normal_to_pack.unwrap_or(DVec2::ZERO)));
        }

        if self.has_geodetic_surface_normals {
            let normal = geodetic_surface_normal.unwrap_or(DVec3::ZERO);
            vertex_buffer.push(normal.x);
            vertex_buffer.push(normal.y);
            vertex_buffer.push(normal.z);
        }

        vertex_buffer.len()
    }

    /// 从顶点缓冲区解码位置。
    ///
    /// Bits12 时解压缩分量并经 from_scaled_enu 还原；否则直接读三轴加中心。
    pub fn decode_position(&self, buffer: &[f64], index: usize) -> DVec3 {
        // 定位顶点起始处。
        let index = index * self.stride;

        if self.quantization == TerrainQuantization::Bits12 {
            let xy = decompress_texture_coordinates(buffer[index]);
            let zh = decompress_texture_coordinates(buffer[index + 1]);
            let mut result = DVec3::new(xy.x, xy.y, zh.x);
            let from_scaled_enu = self.from_scaled_enu.unwrap();
            result = from_scaled_enu.transform_point3(result);
            return result;
        }

        let result = DVec3::new(buffer[index], buffer[index + 1], buffer[index + 2]);
        result + self.center.unwrap()
    }

    /// 从顶点缓冲区解码位置并应用垂直夸张。
    ///
    /// 沿大地测量表面法线按夸张后的高度差平移位置，实现地形夸张效果。
    pub fn get_exaggerated_position(&self, buffer: &[f64], index: usize) -> DVec3 {
        let mut result = self.decode_position(buffer, index);

        let exaggeration = self.exaggeration;
        let exaggeration_relative_height = self.exaggeration_relative_height;
        let has_exaggeration = (exaggeration - 1.0).abs() > f64::EPSILON;
        if has_exaggeration && self.has_geodetic_surface_normals {
            let geodetic_surface_normal = self.decode_geodetic_surface_normal(buffer, index);
            let raw_height = self.decode_height(buffer, index);
            let height_difference = vertical_exaggeration::get_height(
                raw_height,
                exaggeration,
                exaggeration_relative_height,
            ) - raw_height;

            // 部分数学运算被展开以提升性能
            result.x += geodetic_surface_normal.x * height_difference;
            result.y += geodetic_surface_normal.y * height_difference;
            result.z += geodetic_surface_normal.z * height_difference;
        }

        result
    }

    /// 从顶点缓冲区解码纹理坐标。
    ///
    /// Bits12 时解 compressed2 分量，否则直接读取末尾两轴。
    pub fn decode_texture_coordinates(&self, buffer: &[f64], index: usize) -> DVec2 {
        // 定位顶点起始处。
        let index = index * self.stride;

        if self.quantization == TerrainQuantization::Bits12 {
            // BITS12：纹理存于 compressed2 分量，解压即得 (u,v)。
            return decompress_texture_coordinates(buffer[index + 2]);
        }

        DVec2::new(buffer[index + 4], buffer[index + 5])
    }

    /// 从顶点缓冲区解码高度。
    ///
    /// Bits12 时把归一化高度按最小/最大高度区间还原；否则直接读取第 4 分量。
    pub fn decode_height(&self, buffer: &[f64], index: usize) -> f64 {
        // 定位顶点起始处。
        let index = index * self.stride;

        if self.quantization == TerrainQuantization::Bits12 {
            // BITS12：取 compressed1 的 y 分量再映射回高度区间。
            let zh = decompress_texture_coordinates(buffer[index + 1]);
            let minimum_height = self.minimum_height.unwrap();
            let maximum_height = self.maximum_height.unwrap();
            return zh.y * (maximum_height - minimum_height) + minimum_height;
        }

        buffer[index + 3]
    }

    /// 从顶点缓冲区解码 web mercator T 坐标。
    ///
    /// Bits12 时取 compressed3 的分量 x，否则直接读取第 7 分量。
    pub fn decode_web_mercator_t(&self, buffer: &[f64], index: usize) -> f64 {
        let index = index * self.stride;

        if self.quantization == TerrainQuantization::Bits12 {
            return decompress_texture_coordinates(buffer[index + 3]).x;
        }

        buffer[index + 6]
    }

    /// 从顶点缓冲区解码 oct 编码的法线。
    ///
    /// 把打包为单字节的值按 256 进制拆回 (x, y) 两个八面体分量。
    pub fn get_oct_encoded_normal(&self, buffer: &[f64], index: usize) -> DVec2 {
        let index = index * self.stride + self.offset_vertex_normal;

        // 高 8 位作 x，低 8 位作 y，还原八面体投影的两个分量。
        let temp = buffer[index] / 256.0;
        let x = temp.floor();
        let y = (temp - x) * 256.0;

        DVec2::new(x, y)
    }

    /// 从顶点缓冲区解码大地测量表面法线。
    ///
    /// 依偏移读取连续三分量作为单位法线向量。
    pub fn decode_geodetic_surface_normal(&self, buffer: &[f64], index: usize) -> DVec3 {
        let index = index * self.stride + self.offset_geodetic_surface_normal;

        DVec3::new(buffer[index], buffer[index + 1], buffer[index + 2])
    }

    /// 向地形顶点缓冲区添加大地测量表面法线。
    /// 新缓冲区将比旧缓冲区更大。
    ///
    /// 扩大步长后逐顶点由位置求椭球面法线并写入新分量。
    pub fn add_geodetic_surface_normals(
        &mut self,
        old_buffer: &[f64],
        ellipsoid: &Ellipsoid,
    ) -> Vec<f64> {
        if self.has_geodetic_surface_normals {
            return old_buffer.to_vec();
        }

        let old_stride = self.stride;
        let vertex_count = old_buffer.len() / old_stride;
        self.has_geodetic_surface_normals = true;
        self.calculate_stride_and_offsets();
        let new_stride = self.stride;

        let mut new_buffer = vec![0.0f64; vertex_count * new_stride];
        for index in 0..vertex_count {
            // 旧分量逐拷贝到新步长布局的前段。
            for offset in 0..old_stride {
                let old_index = index * old_stride + offset;
                let new_index = index * new_stride + offset;
                new_buffer[new_index] = old_buffer[old_index];
            }
            // 由已拷贝的位置求椭球面法线。
            let position = self.decode_position(&new_buffer, index);
            let geodetic_surface_normal = ellipsoid
                .geodetic_surface_normal(position)
                .unwrap_or(DVec3::ZERO);

            let buffer_index = index * new_stride + self.offset_geodetic_surface_normal;
            new_buffer[buffer_index] = geodetic_surface_normal.x;
            new_buffer[buffer_index + 1] = geodetic_surface_normal.y;
            new_buffer[buffer_index + 2] = geodetic_surface_normal.z;
        }
        new_buffer
    }

    /// 从地形顶点缓冲区移除大地测量表面法线。
    ///
    /// 收缩步长后逐顶点拷贝保留分量，丢弃表面法线对应的尾部数据。
    pub fn remove_geodetic_surface_normals(&mut self, old_buffer: &[f64]) -> Vec<f64> {
        if !self.has_geodetic_surface_normals {
            return old_buffer.to_vec();
        }

        let old_stride = self.stride;
        let vertex_count = old_buffer.len() / old_stride;
        self.has_geodetic_surface_normals = false;
        self.calculate_stride_and_offsets();
        let new_stride = self.stride;

        let mut new_buffer = vec![0.0f64; vertex_count * new_stride];
        for index in 0..vertex_count {
            // 仅保留前 new_stride 个分量，丢弃表面法线。
            for offset in 0..new_stride {
                let old_index = index * old_stride + offset;
                let new_index = index * new_stride + offset;
                new_buffer[new_index] = old_buffer[old_index];
            }
        }
        new_buffer
    }

    /// 获取存储在顶点缓冲区中属性的描述符。
    ///
    /// 依量化方式与可选属性，逐个登记属性的索引、分量数与字节偏移。
    pub fn get_attributes(&self) -> Vec<TerrainAttribute> {
        // 步长按浮点分量数换算为字节。
        let stride_in_bytes = self.stride * FLOAT_SIZE_IN_BYTES;
        let mut offset_in_bytes = 0usize;
        let mut attributes = Vec::new();

        // 追加一个属性描述符并前移字节偏移。
        let mut add_attribute = |index: u32, components_per_attribute: u32| {
            attributes.push(TerrainAttribute {
                index,
                components_per_attribute,
                offset_in_bytes,
                stride_in_bytes,
            });
            offset_in_bytes += components_per_attribute as usize * FLOAT_SIZE_IN_BYTES;
        };

        if self.quantization == TerrainQuantization::None {
            add_attribute(ATTRIBUTES_INDICES_NONE.position_3d_and_height, 4);

            let mut components_tex_coord_and_normals = 2u32;
            if self.has_web_mercator_t {
                components_tex_coord_and_normals += 1;
            }
            if self.has_vertex_normals {
                components_tex_coord_and_normals += 1;
            }
            add_attribute(
                ATTRIBUTES_INDICES_NONE.texture_coord_and_encoded_normals,
                components_tex_coord_and_normals,
            );

            if self.has_geodetic_surface_normals {
                add_attribute(ATTRIBUTES_INDICES_NONE.geodetic_surface_normal, 3);
            }
        } else {
            // 当没有 webMercatorT 或顶点法线时，该属性只需 3 个
            // 分量：x/y、z/h、u/v。WebMercatorT 和顶点法线各占一个
            // 分量，因此若只存在其中一个，第一个属性会获得第 4 个
            // 分量。若两者都存在，我们需要一个额外的、含 1 个
            // 分量的属性。
            let using_attribute_0_component_4 = self.has_web_mercator_t || self.has_vertex_normals;
            let using_attribute_1_component_1 = self.has_web_mercator_t && self.has_vertex_normals;
            add_attribute(
                ATTRIBUTES_INDICES_BITS12.compressed0,
                if using_attribute_0_component_4 { 4 } else { 3 },
            );

            if using_attribute_1_component_1 {
                add_attribute(ATTRIBUTES_INDICES_BITS12.compressed1, 1);
            }

            if self.has_geodetic_surface_normals {
                add_attribute(ATTRIBUTES_INDICES_BITS12.geodetic_surface_normal, 3);
            }
        }

        attributes
    }

    /// 获取指向顶点缓冲区中属性位置的索引。
    ///
    /// 按量化方式返回 NONE 或 BITS12 的属性索引常量表。
    pub fn get_attribute_locations(&self) -> TerrainAttributeLocations {
        if self.quantization == TerrainQuantization::None {
            ATTRIBUTES_INDICES_NONE
        } else {
            ATTRIBUTES_INDICES_BITS12
        }
    }
}
