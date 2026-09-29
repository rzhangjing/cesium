//! 3D Tiles 的点云渲染支持。
//!
//! 镜像 CesiumJS：
//! - `Scene/PointCloud.js`
//! - `Scene/PointCloudShading.js`
//! - `Scene/PointCloudEyeDomeLighting.js`
//! - `Scene/TimeDynamicPointCloud.js`

// 遗留的 CesiumJS 移植风格债务（deferred.md #18）；在 M13 lint-cleanup 或本文件在其里程碑被重写时重新审视
#![allow(clippy::field_reassign_with_default)]
use crate::batch_table::FeatureTable;
use glam::DVec3;

/// 点云着色配置。
///
/// 映射到 CesiumJS `Scene/PointCloudShading.js`
#[derive(Debug, Clone)]
pub struct PointCloudShading {
    /// 是否启用衰减（点随距离缩小）。
    pub attenuation: bool,
    /// 基准点大小，像素。
    pub base_resolution: f64,
    /// 是否启用 eye dome 光照。
    pub eye_dome_lighting: bool,
    /// eye dome 光照强度。
    pub eye_dome_lighting_strength: f64,
    /// eye dome 光照半径。
    pub eye_dome_lighting_radius: f64,
    /// 背面 culing 启用。
    pub back_face_culling: bool,
    /// 法线着色启用（使用法线进行光照）。
    pub normal_shading: bool,
}

impl Default for PointCloudShading {
    fn default() -> Self {
        Self {
            attenuation: false,
            base_resolution: 0.0,
            eye_dome_lighting: true,
            eye_dome_lighting_strength: 1.0,
            eye_dome_lighting_radius: 1.0,
            back_face_culling: false,
            normal_shading: true,
        }
    }
}

impl PointCloudShading {
    /// 根据距离计算衰减后的点大小。
    ///
    /// 映射到 CesiumJS 的点云衰减公式：
    /// `pointSize = baseSize * (attenuationFactor / distance)`
    pub fn compute_attenuated_size(
        &self,
        base_size: f64,
        distance: f64,
        viewport_height: f64,
    ) -> f64 {
        if !self.attenuation || distance <= 0.0 {
            return base_size;
        }

        // 来自 CesiumJS 的衰减公式
        let attenuation_factor = viewport_height * 0.5;
        let attenuated = base_size * (attenuation_factor / distance);

        // 鈐制到合理范围
        attenuated.clamp(1.0, 64.0)
    }

    /// 计算一个点的 eye dome 光照贡献。
    ///
    /// EDL 通过比较相邻像素的深度来增强边缘和轮廓。
    /// 这是用于 domain 逻辑的简化 CPU 侧计算。
    pub fn compute_edl_response(
        &self,
        point_depth: f64,
        neighbor_depths: &[f64],
    ) -> f64 {
        if !self.eye_dome_lighting || neighbor_depths.is_empty() {
            return 1.0;
        }

        // EDL：与邻居之间的 log2(depth) 差值
        // 当点比邻居更近（遮挡边缘）时，响应 < 1
        let log_depth = (point_depth.max(1e-10)).log2();
        let mut response = 0.0;

        for &neighbor_depth in neighbor_depths {
            let log_neighbor = (neighbor_depth.max(1e-10)).log2();
            // 当邻居更远时为正（点在遮挡）
            let diff = (log_neighbor - log_depth).max(0.0);
            response += diff;
        }

        response /= neighbor_depths.len() as f64;

        // 应用强度并转换为着色因子
        let shading = 1.0 - (response * self.eye_dome_lighting_strength * 0.1);
        shading.clamp(0.0, 1.0)
    }
}

/// 从 pnts 内容解码出的点云。
///
/// 映射到 CesiumJS `Scene/PointCloud.js`
#[derive(Debug, Clone)]
pub struct PointCloud {
    /// 点数量。
    pub points_length: u32,
    /// 点位置（若存在则相对于 RTC_CENTER）。
    pub positions: Vec<[f32; 3]>,
    /// 点颜色（RGB 或 RGBA，归一化 0-1）。
    pub colors: Option<Vec<[f32; 4]>>,
    /// 点法线（用于光照）。
    pub normals: Option<Vec<[f32; 3]>>,
    /// Batch ID（用于 feature 关联）。
    pub batch_ids: Option<Vec<u16>>,
    /// 相对中心（RTC）平移。
    pub rtc_center: Option<[f64; 3]>,
    /// 常量 RGBA 颜色（若所有点共享同一颜色）。
    pub constant_rgba: Option<[f32; 4]>,
    /// 量子化位置（若使用量子化）。
    pub quantized_positions: Option<QuantizedPositions>,
}

/// 点云的量子化位置数据。
#[derive(Debug, Clone)]
pub struct QuantizedPositions {
    /// 量子化位置值（u16）。
    pub values: Vec<u16>,
    /// 用于反量子化的体积偏移。
    pub volume_offset: [f32; 3],
    /// 用于反量子化的体积缩放。
    pub volume_scale: [f32; 3],
}

impl QuantizedPositions {
    /// 对给定索引处的位置进行反量子化。
    pub fn dequantize(&self, index: usize) -> [f32; 3] {
        let base = index * 3;
        if base + 2 >= self.values.len() {
            return [0.0, 0.0, 0.0];
        }

        let qx = self.values[base] as f32 / 65535.0;
        let qy = self.values[base + 1] as f32 / 65535.0;
        let qz = self.values[base + 2] as f32 / 65535.0;

        [
            qx * self.volume_scale[0] + self.volume_offset[0],
            qy * self.volume_scale[1] + self.volume_offset[1],
            qz * self.volume_scale[2] + self.volume_offset[2],
        ]
    }
}

impl PointCloud {
    /// 从 feature table 解码点云。
    ///
    /// 映射到 CesiumJS `PntsParser.parse` + `PointCloud` 构造函数
    pub fn from_feature_table(feature_table: &FeatureTable) -> Option<Self> {
        let points_length = feature_table.get_global_u32("POINTS_LENGTH")?;

        // 获取位置（直接量子化两种之一）
        let positions = if let Some(pos) = feature_table.get_positions() {
            pos
        } else if feature_table.has_property("POSITION_QUANTIZED") {
            // 处理量子化位置
            let quantized = Self::decode_quantized_positions(feature_table)?;
            // 对所有位置反量子化
            let mut positions = Vec::with_capacity(points_length as usize);
            for i in 0..points_length as usize {
                positions.push(quantized.dequantize(i));
            }
            positions
        } else {
            return None;
        };

        // 获取颜色
        let colors = Self::decode_colors(feature_table, points_length);

        // 获取法线
        let normals = feature_table.get_normals();

        // 获取 batch ID
        let batch_ids = feature_table.get_batch_ids();

        // 获取 RTC 中心
        let rtc_center = feature_table.get_global_vec3("RTC_CENTER");

        // 获取常量 RGBA
        let constant_rgba = feature_table
            .get_global_property("CONSTANT_RGBA")
            .and_then(|v| v.as_array())
            .and_then(|arr| {
                if arr.len() >= 4 {
                    Some([
                        arr[0].as_u64()? as f32 / 255.0,
                        arr[1].as_u64()? as f32 / 255.0,
                        arr[2].as_u64()? as f32 / 255.0,
                        arr[3].as_u64()? as f32 / 255.0,
                    ])
                } else {
                    None
                }
            });

        // 获取量子化位置元数据（供参考）
        let quantized_positions = if feature_table.has_property("POSITION_QUANTIZED") {
            Self::decode_quantized_positions(feature_table)
        } else {
            None
        };

        Some(Self {
            points_length,
            positions,
            colors,
            normals,
            batch_ids,
            rtc_center,
            constant_rgba,
            quantized_positions,
        })
    }

    /// 从 feature table 解码量子化位置。
    fn decode_quantized_positions(feature_table: &FeatureTable) -> Option<QuantizedPositions> {
        let bin_ref = feature_table.get_binary_ref("POSITION_QUANTIZED")?;
        let count = feature_table.features_length as usize;
        let values = feature_table.read_u16_array(bin_ref.byte_offset, count * 3)?;

        // 获取量子化体积
        let volume_offset = feature_table
            .get_global_property("QUANTIZED_VOLUME_OFFSET")
            .and_then(|v| v.as_array())
            .map(|arr| {
                [
                    arr.first().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    arr.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                ]
            })
            .unwrap_or([0.0, 0.0, 0.0]);

        let volume_scale = feature_table
            .get_global_property("QUANTIZED_VOLUME_SCALE")
            .and_then(|v| v.as_array())
            .map(|arr| {
                [
                    arr.first().and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                    arr.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                    arr.get(2).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                ]
            })
            .unwrap_or([1.0, 1.0, 1.0]);

        Some(QuantizedPositions {
            values,
            volume_offset,
            volume_scale,
        })
    }

    /// 从 feature table 解码颜色。
    fn decode_colors(
        feature_table: &FeatureTable,
        points_length: u32,
    ) -> Option<Vec<[f32; 4]>> {
        // 先尝试 RGBA
        if let Some(rgba) = feature_table.get_colors_rgba() {
            return Some(rgba);
        }

        // 尝试 RGB（添加 alpha = 1.0）
        if let Some(rgb) = feature_table.get_colors_rgb() {
            return Some(rgb.iter().map(|c| [c[0], c[1], c[2], 1.0]).collect());
        }

        // 尝试 RGB565（压缩格式）
        if let Some(bin_ref) = feature_table.get_binary_ref("RGB565") {
            let count = points_length as usize;
            let values = feature_table.read_u16_array(bin_ref.byte_offset, count)?;
            let colors: Vec<[f32; 4]> = values
                .iter()
                .map(|&v| {
                    let r = ((v >> 11) & 0x1F) as f32 / 31.0;
                    let g = ((v >> 5) & 0x3F) as f32 / 63.0;
                    let b = (v & 0x1F) as f32 / 31.0;
                    [r, g, b, 1.0]
                })
                .collect();
            return Some(colors);
        }

        None
    }

    /// 获取一个点的世界位置（若存在则应用 RTC 中心）。
    pub fn get_world_position(&self, index: usize) -> Option<DVec3> {
        if index >= self.positions.len() {
            return None;
        }

        let pos = self.positions[index];
        let mut world = DVec3::new(pos[0] as f64, pos[1] as f64, pos[2] as f64);

        if let Some(rtc) = self.rtc_center {
            world += DVec3::new(rtc[0], rtc[1], rtc[2]);
        }

        Some(world)
    }

    /// 获取一个点的颜色。
    pub fn get_color(&self, index: usize) -> [f32; 4] {
        // 若可用则使用逐点颜色
        if let Some(colors) = &self.colors {
            if index < colors.len() {
                return colors[index];
            }
        }

        // 若可用则使用常量颜色
        if let Some(rgba) = self.constant_rgba {
            return rgba;
        }

        // 默认：白色
        [1.0, 1.0, 1.0, 1.0]
    }

    /// 获取一个点的法线。
    pub fn get_normal(&self, index: usize) -> Option<[f32; 3]> {
        self.normals.as_ref().and_then(|n| n.get(index).copied())
    }

    /// 计算点云的包围球。
    pub fn compute_bounding_sphere(&self) -> Option<(DVec3, f64)> {
        if self.positions.is_empty() {
            return None;
        }

        // 计算中心
        let mut center = DVec3::ZERO;
        for pos in &self.positions {
            center += DVec3::new(pos[0] as f64, pos[1] as f64, pos[2] as f64);
        }
        center /= self.positions.len() as f64;

        // 加上 RTC 中心
        if let Some(rtc) = self.rtc_center {
            center += DVec3::new(rtc[0], rtc[1], rtc[2]);
        }

        // 计算半径
        let mut radius_sq = 0.0f64;
        for pos in &self.positions {
            let mut world = DVec3::new(pos[0] as f64, pos[1] as f64, pos[2] as f64);
            if let Some(rtc) = self.rtc_center {
                world += DVec3::new(rtc[0], rtc[1], rtc[2]);
            }
            let dist_sq = world.distance_squared(center);
            radius_sq = radius_sq.max(dist_sq);
        }

        Some((center, radius_sq.sqrt()))
    }
}

/// 时间动态点云配置。
///
/// 映射到 CesiumJS `Scene/TimeDynamicPointCloud.js`
#[derive(Debug, Clone)]
pub struct TimeDynamicPointCloud {
    /// 点云是否为时间动态。
    pub is_time_dynamic: bool,
    /// 帧时间戳（以自历元起的秒计）。
    pub timestamps: Vec<f64>,
    /// 每帧的 URI。
    pub uris: Vec<String>,
    /// 是否在帧之间插值。
    pub interpolate: bool,
}

impl TimeDynamicPointCloud {
    /// 创建一个新的时间动态点云。
    pub fn new(timestamps: Vec<f64>, uris: Vec<String>) -> Self {
        Self {
            is_time_dynamic: !timestamps.is_empty(),
            timestamps,
            uris,
            interpolate: false,
        }
    }

    /// 获取给定时间对应的帧索引。
    pub fn get_frame_index(&self, time: f64) -> Option<usize> {
        if self.timestamps.is_empty() {
            return None;
        }

        // 查找在给定时间处或紧接其前的帧
        for (i, &ts) in self.timestamps.iter().enumerate() {
            if ts >= time {
                return Some(i);
            }
        }

        // 若时间晚于所有时间戳则返回最后一帧
        Some(self.timestamps.len() - 1)
    }

    /// 获取给定时间对应的 URI。
    pub fn get_uri(&self, time: f64) -> Option<&str> {
        let index = self.get_frame_index(time)?;
        self.uris.get(index).map(|s| s.as_str())
    }

    /// 获取两帧之间的插值因子。
    pub fn get_interpolation_factor(&self, time: f64) -> Option<(usize, usize, f64)> {
        if !self.interpolate || self.timestamps.len() < 2 {
            return None;
        }

        // 查找相邻的帧
        for i in 0..self.timestamps.len() - 1 {
            let t0 = self.timestamps[i];
            let t1 = self.timestamps[i + 1];
            if time >= t0 && time <= t1 {
                let factor = if t1 > t0 {
                    (time - t0) / (t1 - t0)
                } else {
                    0.0
                };
                return Some((i, i + 1, factor));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batch_table::FeatureTable;
    use serde_json::json;

    fn create_feature_table_with_positions(count: u32) -> FeatureTable {
        let mut binary = Vec::new();
        for i in 0..count {
            let x = i as f32;
            let y = (i * 2) as f32;
            let z = (i * 3) as f32;
            binary.extend_from_slice(&x.to_le_bytes());
            binary.extend_from_slice(&y.to_le_bytes());
            binary.extend_from_slice(&z.to_le_bytes());
        }

        let json = json!({
            "POINTS_LENGTH": count,
            "POSITION": { "byteOffset": 0 }
        });

        FeatureTable::new(Some(json), binary)
    }

    #[test]
    fn test_point_cloud_shading_default() {
        let shading = PointCloudShading::default();
        assert!(!shading.attenuation);
        assert!(shading.eye_dome_lighting);
        assert!(shading.normal_shading);
    }

    #[test]
    fn test_attenuated_size_no_attenuation() {
        let shading = PointCloudShading::default();
        let size = shading.compute_attenuated_size(5.0, 100.0, 1080.0);
        assert_eq!(size, 5.0);
    }

    #[test]
    fn test_attenuated_size_with_attenuation() {
        let mut shading = PointCloudShading::default();
        shading.attenuation = true;

        // 更近的点应更大
        let size_near = shading.compute_attenuated_size(5.0, 100.0, 1080.0);
        let size_far = shading.compute_attenuated_size(5.0, 1000.0, 1080.0);
        assert!(size_near > size_far);
    }

    #[test]
    fn test_edl_response() {
        let shading = PointCloudShading::default();

        // 点与邻居深度相同：无边缘
        let response_flat = shading.compute_edl_response(100.0, &[100.0, 100.0, 100.0, 100.0]);
        assert!((response_flat - 1.0).abs() < 0.01);

        // 点比邻居更近：检测到边缘
        let response_edge = shading.compute_edl_response(50.0, &[100.0, 100.0, 100.0, 100.0]);
        assert!(response_edge < 1.0);
    }

    #[test]
    fn test_point_cloud_from_feature_table() {
        let ft = create_feature_table_with_positions(3);
        let pc = PointCloud::from_feature_table(&ft).unwrap();

        assert_eq!(pc.points_length, 3);
        assert_eq!(pc.positions.len(), 3);
        assert!((pc.positions[0][0] - 0.0).abs() < 1e-6);
        assert!((pc.positions[1][1] - 2.0).abs() < 1e-6);
        assert!((pc.positions[2][2] - 6.0).abs() < 1e-6);
    }

    #[test]
    fn test_point_cloud_with_colors() {
        let mut binary = Vec::new();
        // 位置
        for i in 0..2u32 {
            binary.extend_from_slice(&(i as f32).to_le_bytes());
            binary.extend_from_slice(&0.0f32.to_le_bytes());
            binary.extend_from_slice(&0.0f32.to_le_bytes());
        }
        // RGB 颜色（u8）
        binary.extend_from_slice(&[255u8, 0, 0]); // 红
        binary.extend_from_slice(&[0u8, 255, 0]); // 绿

        let json = json!({
            "POINTS_LENGTH": 2,
            "POSITION": { "byteOffset": 0 },
            "RGB": { "byteOffset": 24 }
        });

        let ft = FeatureTable::new(Some(json), binary);
        let pc = PointCloud::from_feature_table(&ft).unwrap();

        let colors = pc.colors.as_ref().unwrap();
        assert!((colors[0][0] - 1.0).abs() < 0.01); // 红
        assert!((colors[1][1] - 1.0).abs() < 0.01); // 绿
    }

    #[test]
    fn test_point_cloud_with_rtc_center() {
        let mut binary = Vec::new();
        binary.extend_from_slice(&1.0f32.to_le_bytes());
        binary.extend_from_slice(&2.0f32.to_le_bytes());
        binary.extend_from_slice(&3.0f32.to_le_bytes());

        let json = json!({
            "POINTS_LENGTH": 1,
            "POSITION": { "byteOffset": 0 },
            "RTC_CENTER": [1000.0, 2000.0, 3000.0]
        });

        let ft = FeatureTable::new(Some(json), binary);
        let pc = PointCloud::from_feature_table(&ft).unwrap();

        assert_eq!(pc.rtc_center, Some([1000.0, 2000.0, 3000.0]));

        let world_pos = pc.get_world_position(0).unwrap();
        assert!((world_pos.x - 1001.0).abs() < 1e-6);
        assert!((world_pos.y - 2002.0).abs() < 1e-6);
        assert!((world_pos.z - 3003.0).abs() < 1e-6);
    }

    #[test]
    fn test_point_cloud_get_color() {
        let ft = create_feature_table_with_positions(1);
        let mut pc = PointCloud::from_feature_table(&ft).unwrap();

        // 默认：白色
        assert_eq!(pc.get_color(0), [1.0, 1.0, 1.0, 1.0]);

        // 带常量颜色
        pc.constant_rgba = Some([1.0, 0.0, 0.0, 1.0]);
        assert_eq!(pc.get_color(0), [1.0, 0.0, 0.0, 1.0]);

        // 带逐点颜色
        pc.colors = Some(vec![[0.0, 1.0, 0.0, 1.0]]);
        assert_eq!(pc.get_color(0), [0.0, 1.0, 0.0, 1.0]);
    }

    #[test]
    fn test_point_cloud_bounding_sphere() {
        let ft = create_feature_table_with_positions(3);
        let pc = PointCloud::from_feature_table(&ft).unwrap();

        let (center, radius) = pc.compute_bounding_sphere().unwrap();

        // 中心应为位置的平均值
        assert!((center.x - 1.0).abs() < 1e-6); // (0+1+2)/3
        assert!((center.y - 2.0).abs() < 1e-6); // (0+2+4)/3
        assert!((center.z - 3.0).abs() < 1e-6); // (0+3+6)/3

        assert!(radius > 0.0);
    }

    #[test]
    fn test_quantized_positions() {
        let quantized = QuantizedPositions {
            values: vec![0, 32767, 65535], // 0%, 50%, 100%
            volume_offset: [0.0, 0.0, 0.0],
            volume_scale: [10.0, 20.0, 30.0],
        };

        let pos = quantized.dequantize(0);
        assert!((pos[0] - 0.0).abs() < 0.01);
        assert!((pos[1] - 10.0).abs() < 0.01); // 20 的 50%
        assert!((pos[2] - 30.0).abs() < 0.01); // 30 的 100%
    }

    #[test]
    fn test_time_dynamic_point_cloud() {
        let timestamps = vec![0.0, 1.0, 2.0, 3.0];
        let uris = vec![
            "frame0.pnts".to_string(),
            "frame1.pnts".to_string(),
            "frame2.pnts".to_string(),
            "frame3.pnts".to_string(),
        ];

        let tdpc = TimeDynamicPointCloud::new(timestamps, uris);

        assert!(tdpc.is_time_dynamic);
        assert_eq!(tdpc.get_frame_index(0.5), Some(1));
        assert_eq!(tdpc.get_frame_index(1.5), Some(2));
        assert_eq!(tdpc.get_frame_index(5.0), Some(3)); // 在所有时间戳之后

        assert_eq!(tdpc.get_uri(0.5), Some("frame1.pnts"));
        assert_eq!(tdpc.get_uri(2.5), Some("frame3.pnts"));
    }

    #[test]
    fn test_time_dynamic_interpolation() {
        let timestamps = vec![0.0, 1.0, 2.0];
        let uris = vec!["a.pnts".to_string(), "b.pnts".to_string(), "c.pnts".to_string()];

        let mut tdpc = TimeDynamicPointCloud::new(timestamps, uris);
        tdpc.interpolate = true;

        let (i0, i1, factor) = tdpc.get_interpolation_factor(0.5).unwrap();
        assert_eq!(i0, 0);
        assert_eq!(i1, 1);
        assert!((factor - 0.5).abs() < 1e-6);

        let (i0, i1, factor) = tdpc.get_interpolation_factor(1.75).unwrap();
        assert_eq!(i0, 1);
        assert_eq!(i1, 2);
        assert!((factor - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_point_cloud_with_normals() {
        let mut binary = Vec::new();
        // 位置
        binary.extend_from_slice(&0.0f32.to_le_bytes());
        binary.extend_from_slice(&0.0f32.to_le_bytes());
        binary.extend_from_slice(&0.0f32.to_le_bytes());
        // 法线
        binary.extend_from_slice(&0.0f32.to_le_bytes());
        binary.extend_from_slice(&0.0f32.to_le_bytes());
        binary.extend_from_slice(&1.0f32.to_le_bytes());

        let json = json!({
            "POINTS_LENGTH": 1,
            "POSITION": { "byteOffset": 0 },
            "NORMAL": { "byteOffset": 12 }
        });

        let ft = FeatureTable::new(Some(json), binary);
        let pc = PointCloud::from_feature_table(&ft).unwrap();

        let normal = pc.get_normal(0).unwrap();
        assert!((normal[2] - 1.0).abs() < 1e-6);
    }
}
