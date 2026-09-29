//! 用于选择性禁用渲染的裁剪平面。
//!
//! 映射到 CesiumJS：
//! - `Scene/ClippingPlane.js` —— 单个裁剪平面
//! - `Scene/ClippingPlaneCollection.js` —— 带 union/intersection 模式的集合
//!
//! 领域层——纯 Rust，f64 精度。

use glam::{DMat3, DMat4, DVec3};

/// 由一个法线与距离定义的单裁剪平面。
///
/// 映射到 CesiumJS `ClippingPlane`。
///
/// 平面方程为：dot(normal, point) + distance = 0
/// 位于正侧（dot + distance > 0）的点被保留。
/// 位于负侧的点被裁剪。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClippingPlane {
    /// 平面法线（已归一化）。
    pub normal: DVec3,
    /// 沿法线方向距原点的距离。
    /// 正距离表示平面沿法线方向偏移。
    pub distance: f64,
}

impl ClippingPlane {
    /// 创建一个新的裁剪平面。
    ///
    /// # 参数
    /// * `normal` - 平面法线（将被归一化）
    /// * `distance` - 距原点的带符号距离
    pub fn new(normal: DVec3, distance: f64) -> Self {
        Self {
            normal: normal.normalize(),
            distance,
        }
    }

    /// 计算从一个点到本平面的带符号距离。
    ///
    /// 正 = 点位于保留侧。
    /// 负 = 点位于裁剪侧。
    pub fn signed_distance(&self, point: DVec3) -> f64 {
        self.normal.dot(point) + self.distance
    }

    /// 返回一个点是否被本平面判定为内部（保留）。
    pub fn is_inside(&self, point: DVec3) -> bool {
        self.signed_distance(point) >= 0.0
    }

    /// 用一个 4x4 矩阵变换本平面。
    ///
    /// 使用矩阵的逆转置以获得正确的法线变换。
    pub fn transform(&self, matrix: &DMat4) -> Self {
        // 变换平面上的一个点（点由完整仿射 M 变换）。
        let point_on_plane = self.normal * (-self.distance);
        let transformed_point = matrix.transform_point3(point_on_plane);

        // 法线是余向量：正确的变换是左上角 3×3 的*逆转置*，从而在
        // 非均匀缩放下平面仍垂直于表面。先前仅用 `Mᵀ` 的形式只对刚性 /
        // 均匀缩放的矩阵成立，与本函数的文档注释相矛盾。
        // FIX-CLIP-TRANSFORM。对奇异 / 非有限的 3×3 守护其逆。
        let upper = DMat3::from_cols(
            matrix.x_axis.truncate(),
            matrix.y_axis.truncate(),
            matrix.z_axis.truncate(),
        );
        let det = upper.determinant();
        let normal_matrix = if det.is_finite() && det.abs() > 1.0e-12 {
            upper.inverse().transpose()
        } else {
            // 退化缩放：回退到刚性假设的 `Mᵀ` 形式。
            upper.transpose()
        };
        let transformed_normal = normal_matrix.mul_vec3(self.normal).normalize();

        let new_distance = -transformed_normal.dot(transformed_point);

        Self {
            normal: transformed_normal,
            distance: new_distance,
        }
    }

    /// 把平面打包成 vec4（normal.xyz, distance）以便上传 GPU。
    pub fn to_vec4(&self) -> [f64; 4] {
        [self.normal.x, self.normal.y, self.normal.z, self.distance]
    }

    /// 由一个打包的 vec4 创建平面。
    pub fn from_vec4(v: [f64; 4]) -> Self {
        Self {
            normal: DVec3::new(v[0], v[1], v[2]),
            distance: v[3],
        }
    }
}

/// 相交测试结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intersect {
    /// 对象完全在内部（保留）。
    Inside,
    /// 对象与平面相交。
    Intersecting,
    /// 对象完全在外部（裁剪）。
    Outside,
}

/// 裁剪平面的集合。
///
/// 映射到 CesiumJS `ClippingPlaneCollection`。
#[derive(Debug, Clone)]
pub struct ClippingPlaneCollection {
    /// 各裁剪平面。
    planes: Vec<ClippingPlane>,
    /// 是否启用裁剪。
    pub enabled: bool,
    /// 应用于所有平面的额外变换。
    pub model_matrix: DMat4,
    /// 若为 true，则在任一平面之外即裁剪（union）。
    /// 若为 false，则仅在所有平面之外才裁剪（intersection）。
    pub union_clipping_regions: bool,
    /// 边缘高亮颜色 [R, G, B, A]。
    pub edge_color: [f64; 4],
    /// 边缘高亮宽度，以像素计。
    pub edge_width: f64,
}

impl Default for ClippingPlaneCollection {
    fn default() -> Self {
        Self {
            planes: Vec::new(),
            enabled: true,
            model_matrix: DMat4::IDENTITY,
            union_clipping_regions: false,
            edge_color: [1.0, 1.0, 1.0, 1.0],
            edge_width: 0.0,
        }
    }
}

impl ClippingPlaneCollection {
    /// 创建一个空集合。
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建带有初始平面的集合。
    pub fn with_planes(planes: Vec<ClippingPlane>) -> Self {
        Self {
            planes,
            ..Default::default()
        }
    }

    /// 添加一个裁剪平面。
    pub fn add(&mut self, plane: ClippingPlane) {
        self.planes.push(plane);
    }

    /// 按索引移除一个平面。
    ///
    /// 返回被移除的平面，若索引越界则返回 None。
    pub fn remove(&mut self, index: usize) -> Option<ClippingPlane> {
        if index < self.planes.len() {
            Some(self.planes.remove(index))
        } else {
            None
        }
    }

    /// 移除所有平面。
    pub fn remove_all(&mut self) {
        self.planes.clear();
    }

    /// 返回平面的数量。
    pub fn len(&self) -> usize {
        self.planes.len()
    }

    /// 返回集合是否为空。
    pub fn is_empty(&self) -> bool {
        self.planes.is_empty()
    }

    /// 按索引获取一个平面。
    pub fn get(&self, index: usize) -> Option<&ClippingPlane> {
        self.planes.get(index)
    }

    /// 按索引获取一个可变平面。
    pub fn get_mut(&mut self, index: usize) -> Option<&mut ClippingPlane> {
        self.planes.get_mut(index)
    }

    /// 返回裁剪平面状态值。
    ///
    /// 符号编码裁剪模式：
    /// - 正 = union 模式
    /// - 负 = intersection 模式
    ///
    /// 映射到 CesiumJS `clippingPlanesState`。
    pub fn clipping_planes_state(&self) -> i32 {
        let count = self.planes.len() as i32;
        if self.union_clipping_regions {
            count
        } else {
            -count
        }
    }

    /// 测试一个点是否被本集合裁剪。
    ///
    /// # 参数
    /// * `point` - 待测试的世界空间点
    ///
    /// # 返回
    /// 若该点应被裁剪（不渲染）则返回 `true`。
    pub fn is_clipped(&self, point: DVec3) -> bool {
        if !self.enabled || self.planes.is_empty() {
            return false;
        }

        // 将点变换到裁剪平面空间
        let inverse = self.model_matrix.inverse();
        let local_point = inverse.transform_point3(point);

        if self.union_clipping_regions {
            // Union：在任一平面之外即裁剪
            self.planes.iter().any(|p| !p.is_inside(local_point))
        } else {
            // Intersection：仅在所有平面之外才裁剪
            self.planes.iter().all(|p| !p.is_inside(local_point))
        }
    }

    /// GPU `apply_clipping_planes` 累加的 f64 CPU 参考
    /// （`adapters/bevy-render/shaders/clipping.wgsl`），逐条入口，用于
    /// 交叉验证 f32 shader。与 [`Self::is_clipped`] 不同，它还返回
    /// *带符号*的 clipAmount，并使用蓝图的 `<= 0.0` 外部测试
    /// （FIX-CLIP-LTE）：恰好位于平面上的点计为被裁剪。
    ///
    /// 返回 `(clipped, clip_amount)`，其中 `clip_amount` 在 union 模式下是
    /// 各平面上的带符号 min（蓝图 L59），在 intersection 模式下是以 `0.0`
    /// 为初值的带符号 max（蓝图 L64）——即驱动边缘光带的值。
    pub fn clip_signed(&self, point: DVec3) -> (bool, f64) {
        if !self.enabled || self.planes.is_empty() {
            return (false, 0.0);
        }
        let local_point = self.model_matrix.inverse().transform_point3(point);

        let mut any_outside = false;
        let mut all_outside = true;
        let mut clip_amount = 0.0;
        for (i, p) in self.planes.iter().enumerate() {
            let d = p.signed_distance(local_point);
            if d <= 0.0 {
                any_outside = true;
            } else {
                all_outside = false;
            }
            clip_amount = if self.union_clipping_regions {
                if i == 0 {
                    d
                } else {
                    d.min(clip_amount)
                }
            } else {
                d.max(clip_amount)
            };
        }
        let clipped = if self.union_clipping_regions {
            any_outside
        } else {
            all_outside
        };
        (clipped, clip_amount)
    }

    /// 测试一个包围球与裁剪平面的相交。
    ///
    /// 映射到 CesiumJS `ClippingPlaneCollection.prototype.computeIntersectionWithBoundingVolume`。
    ///
    /// # 参数
    /// * `center` - 球心（世界空间）
    /// * `radius` - 球半径
    ///
    /// # 返回
    /// 相交结果。
    pub fn intersect_bounding_sphere(&self, center: DVec3, radius: f64) -> Intersect {
        if !self.enabled || self.planes.is_empty() {
            return Intersect::Inside;
        }

        let inverse = self.model_matrix.inverse();
        let local_center = inverse.transform_point3(center);

        // 依据裁剪模式初始化（与 CesiumJS 一致）：
        // - Union 模式：从 INSIDE 开始；若任一平面在
        //   其负侧包含了球，则整个球被裁剪 → OUTSIDE。
        // - Intersection 模式：从 OUTSIDE 开始；若任一平面
        //   在其正侧包含了球，则没有点能位于所有
        //   平面之外 → INSIDE。
        let mut intersection = if self.union_clipping_regions {
            Intersect::Inside
        } else {
            Intersect::Outside
        };

        for plane in &self.planes {
            let dist = plane.signed_distance(local_center);

            let value = if dist < -radius {
                Intersect::Outside
            } else if dist > radius {
                Intersect::Inside
            } else {
                Intersect::Intersecting
            };

            if value == Intersect::Intersecting {
                intersection = Intersect::Intersecting;
            } else if self.union_clipping_regions {
                // Union 模式：若任一平面为 OUTSIDE，则整球被裁剪
                if value == Intersect::Outside {
                    return Intersect::Outside;
                }
            } else {
                // Intersection 模式：若任一平面为 INSIDE，则没有点能
                // 位于所有平面之外，故球被保留
                if value == Intersect::Inside {
                    return Intersect::Inside;
                }
            }
        }

        intersection
    }

    /// 把所有平面打包进一个扁平数组以便上传 GPU。
    ///
    /// 每个平面为 4 个浮点数：[normal.x, normal.y, normal.z, distance]。
    pub fn pack_planes(&self) -> Vec<f64> {
        let mut packed = Vec::with_capacity(self.planes.len() * 4);
        for plane in &self.planes {
            packed.extend_from_slice(&plane.to_vec4());
        }
        packed
    }

    /// 计算某个片元的边缘高亮因子。
    ///
    /// 返回 [0, 1] 内的值，指示该片元离裁剪边缘有多近。
    ///
    /// # 参数
    /// * `point` - 片元位置（世界空间）
    /// * `pixel_size` - 该深度下一个像素的世界空间尺寸
    pub fn edge_factor(&self, point: DVec3, pixel_size: f64) -> f64 {
        if self.edge_width <= 0.0 || !self.enabled || self.planes.is_empty() {
            return 0.0;
        }

        let inverse = self.model_matrix.inverse();
        let local_point = inverse.transform_point3(point);

        let edge_threshold = self.edge_width * pixel_size;

        for plane in &self.planes {
            let dist = plane.signed_distance(local_point).abs();
            if dist < edge_threshold {
                return 1.0 - dist / edge_threshold;
            }
        }

        0.0
    }

    /// 返回由 [`Self::model_matrix`] 变换到世界空间的平面。
    ///
    /// 这是 CesiumJS 逐片元
    /// `czm_transformPlane(plane, clippingPlanesMatrix)`
    /// （`ModelClippingPlanesStageFS.glsl` L14/L35）的 CPU 侧等价物：集合的
    /// `model_matrix` 在 CPU 上被一次性烘焙进平面，而非每个片元都在 GPU 上
    /// 变换每个平面。两者符号等价——世界点 `p` 位于 `world_planes()[i]`
    /// 之内，当且仅当 `model_matrix.inverse() * p` 位于 `planes[i]` 之内——
    /// 这正是 [`Self::is_clipped`] 所依赖的不变式
    /// （参见 `test_world_planes_transform_equivalence`）。
    ///
    /// 返回的平面保持在**域内度量 f64** 空间（此处不做
    /// `METERS_PER_RENDER_UNIT` 重缩放——那一转换在适配器/GPU 边界施加，
    /// 属红线）。
    pub fn world_planes(&self) -> Vec<ClippingPlane> {
        self.planes
            .iter()
            .map(|p| p.transform(&self.model_matrix))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── ClippingPlane 测试 ────────────────────────────────────────────

    #[test]
    fn test_plane_creation() {
        let plane = ClippingPlane::new(DVec3::new(0.0, 1.0, 0.0), 5.0);
        assert!((plane.normal - DVec3::Y).length() < 1e-10);
        assert!((plane.distance - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_plane_normalizes() {
        let plane = ClippingPlane::new(DVec3::new(0.0, 2.0, 0.0), 5.0);
        assert!((plane.normal.length() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_plane_signed_distance() {
        let plane = ClippingPlane::new(DVec3::Y, 0.0);

        // 平面上方的点
        assert!((plane.signed_distance(DVec3::new(0.0, 5.0, 0.0)) - 5.0).abs() < 1e-10);
        // 平面下方的点
        assert!((plane.signed_distance(DVec3::new(0.0, -3.0, 0.0)) - (-3.0)).abs() < 1e-10);
        // 平面上的点
        assert!(plane.signed_distance(DVec3::new(1.0, 0.0, 1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_plane_is_inside() {
        let plane = ClippingPlane::new(DVec3::Y, 0.0);

        assert!(plane.is_inside(DVec3::new(0.0, 1.0, 0.0)));
        assert!(!plane.is_inside(DVec3::new(0.0, -1.0, 0.0)));
        assert!(plane.is_inside(DVec3::ZERO)); // 在平面上 = 内部
    }

    #[test]
    fn test_plane_with_offset() {
        // y = 5 处的平面（法线朝上，distance = -5）
        let plane = ClippingPlane::new(DVec3::Y, -5.0);

        assert!(plane.is_inside(DVec3::new(0.0, 10.0, 0.0))); // 上方
        assert!(!plane.is_inside(DVec3::new(0.0, 3.0, 0.0))); // 下方
    }

    #[test]
    fn test_plane_vec4_roundtrip() {
        let plane = ClippingPlane::new(DVec3::new(1.0, 2.0, 3.0), 4.0);
        let packed = plane.to_vec4();
        let unpacked = ClippingPlane::from_vec4(packed);

        assert!((plane.normal - unpacked.normal).length() < 1e-10);
        assert!((plane.distance - unpacked.distance).abs() < 1e-10);
    }

    #[test]
    fn test_plane_transform_translation() {
        let plane = ClippingPlane::new(DVec3::Y, 0.0);
        let translation = DMat4::from_translation(DVec3::new(0.0, 10.0, 0.0));

        let transformed = plane.transform(&translation);

        // 将平面的坐标系上移 10 之后，
        // 该平面（原本在 y=0）在世界空间中实际位于 y=10。
        // y=15 处的点应在内部（平面上方）。
        assert!(transformed.is_inside(DVec3::new(0.0, 15.0, 0.0)));
        // y=5 处的点应在外部（平面下方）。
        assert!(!transformed.is_inside(DVec3::new(0.0, 5.0, 0.0)));
    }

    #[test]
    fn test_plane_transform_inverse_transpose_under_non_uniform_scale() {
        // FIX-CLIP-TRANSFORM：法线必须用逆转置变换，而非 Mᵀ。
        // 平面法线 (1,1,0)/√2 在 scale(2,1,1) 下：
        //   正确  (S⁻ᵀ n) ∝ (1/2, 1, 0) -> (0.4472, 0.8944, 0)
        //   错误    (Sᵀ  n) ∝ (2,   1, 0) -> (0.8944, 0.4472, 0)
        let plane = ClippingPlane::new(DVec3::new(1.0, 1.0, 0.0), 0.0);
        let scaled = DMat4::from_scale(DVec3::new(2.0, 1.0, 1.0));
        let transformed = plane.transform(&scaled);

        let expected = DVec3::new(0.5, 1.0, 0.0).normalize();
        assert!(
            (transformed.normal - expected).length() < 1e-9,
            "inverse-transpose expected, got {:?}",
            transformed.normal
        );
    }

    #[test]
    fn test_clip_signed_boundary_on_plane_is_clipped() {
        // FIX-CLIP-LTE：恰好位于平面上的点（带符号距离 == 0）计为
        // 被裁剪，与蓝图的 `<= 0.0` GPU 测试一致。
        let collection = ClippingPlaneCollection::with_planes(vec![ClippingPlane::new(DVec3::Y, 0.0)]);
        let (clipped, amount) = collection.clip_signed(DVec3::new(3.0, 0.0, -2.0));
        assert!(clipped, "point on plane must be clipped");
        assert!(amount.abs() < 1e-12, "on-plane clip_amount is 0, got {amount}");
    }

    #[test]
    fn test_clip_signed_union_vs_intersection() {
        // 两个相对的平面：x >= 0 保留（法线 +X，distance 0），y >= 0 保留。
        let planes = vec![ClippingPlane::new(DVec3::X, 0.0), ClippingPlane::new(DVec3::Y, 0.0)];

        let mut union = ClippingPlaneCollection::with_planes(planes.clone());
        union.union_clipping_regions = true;
        // 在 +X 平面外（x<0）但在 +Y 平面内 ⇒ union 裁剪（任一在外）。
        let (u_clipped, u_amt) = union.clip_signed(DVec3::new(-1.0, 2.0, 0.0));
        assert!(u_clipped, "union clips when outside any plane");
        assert!(u_amt < 0.0, "union signed min is the most-negative, got {u_amt}");

        let inter = ClippingPlaneCollection::with_planes(planes);
        // 在 +Y 平面内 ⇒ intersection 不裁剪（需全部在外）。
        let (i_clipped, _) = inter.clip_signed(DVec3::new(-1.0, 2.0, 0.0));
        assert!(!i_clipped, "intersection keeps a point inside any plane");
        // 两者皆外 ⇒ intersection 裁剪；带符号 max 以 0 为初值 ⇒ 此处为 0。
        let (i2_clipped, i2_amt) = inter.clip_signed(DVec3::new(-1.0, -1.0, 0.0));
        assert!(i2_clipped, "intersection clips when outside all planes");
        assert!(i2_amt >= 0.0, "intersection signed max is floored at 0, got {i2_amt}");
    }

    // ─── ClippingPlaneCollection 测试 ──────────────────────────────────

    #[test]
    fn test_collection_default() {
        let collection = ClippingPlaneCollection::default();
        assert!(collection.enabled);
        assert!(!collection.union_clipping_regions);
        assert!(collection.is_empty());
        assert!((collection.edge_width).abs() < 1e-10);
    }

    #[test]
    fn test_collection_add_remove() {
        let mut collection = ClippingPlaneCollection::new();
        collection.add(ClippingPlane::new(DVec3::Y, 0.0));
        collection.add(ClippingPlane::new(DVec3::X, 0.0));

        assert_eq!(collection.len(), 2);

        let removed = collection.remove(0);
        assert!(removed.is_some());
        assert_eq!(collection.len(), 1);

        assert!(collection.remove(5).is_none());
    }

    #[test]
    fn test_collection_remove_all() {
        let mut collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
            ClippingPlane::new(DVec3::X, 0.0),
        ]);

        collection.remove_all();
        assert!(collection.is_empty());
    }

    #[test]
    fn test_clipping_planes_state() {
        let mut collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
            ClippingPlane::new(DVec3::X, 0.0),
        ]);

        // Intersection 模式（默认）：负
        assert_eq!(collection.clipping_planes_state(), -2);

        // Union 模式：正
        collection.union_clipping_regions = true;
        assert_eq!(collection.clipping_planes_state(), 2);
    }

    #[test]
    fn test_is_clipped_disabled() {
        let mut collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
        ]);
        collection.enabled = false;

        assert!(!collection.is_clipped(DVec3::new(0.0, -100.0, 0.0)));
    }

    #[test]
    fn test_is_clipped_intersection_mode() {
        // 两个平面构成一个角（intersection 模式）
        let collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0), // 保留 y >= 0
            ClippingPlane::new(DVec3::X, 0.0), // 保留 x >= 0
        ]);

        // 在两个平面内
        assert!(!collection.is_clipped(DVec3::new(1.0, 1.0, 0.0)));

        // 在一个平面外但在另一平面内 → 不裁剪（intersection 模式）
        assert!(!collection.is_clipped(DVec3::new(-1.0, 1.0, 0.0)));

        // 两个平面皆外 → 裁剪
        assert!(collection.is_clipped(DVec3::new(-1.0, -1.0, 0.0)));
    }

    #[test]
    fn test_is_clipped_union_mode() {
        let mut collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
            ClippingPlane::new(DVec3::X, 0.0),
        ]);
        collection.union_clipping_regions = true;

        // 在两个平面内
        assert!(!collection.is_clipped(DVec3::new(1.0, 1.0, 0.0)));

        // 在一个平面外 → 裁剪（union 模式）
        assert!(collection.is_clipped(DVec3::new(-1.0, 1.0, 0.0)));
    }

    #[test]
    fn test_intersect_bounding_sphere_inside() {
        let collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
        ]);

        // 球完全在平面上方
        let result = collection.intersect_bounding_sphere(DVec3::new(0.0, 10.0, 0.0), 1.0);
        assert_eq!(result, Intersect::Inside);
    }

    #[test]
    fn test_intersect_bounding_sphere_outside() {
        let collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
        ]);

        // 球完全在平面下方
        let result = collection.intersect_bounding_sphere(DVec3::new(0.0, -10.0, 0.0), 1.0);
        assert_eq!(result, Intersect::Outside);
    }

    #[test]
    fn test_intersect_bounding_sphere_intersecting() {
        let collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
        ]);

        // 球横跨平面
        let result = collection.intersect_bounding_sphere(DVec3::new(0.0, 0.5, 0.0), 1.0);
        assert_eq!(result, Intersect::Intersecting);
    }

    #[test]
    fn test_intersect_bounding_sphere_union_outside_any_plane() {
        // Union 模式：任一平面之外即裁剪
        let mut collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0), // y >= 0
            ClippingPlane::new(DVec3::X, 0.0), // x >= 0
        ]);
        collection.union_clipping_regions = true;

        // 球完全在 Y 平面内但完全在 X 平面外 → Outside
        let result = collection.intersect_bounding_sphere(DVec3::new(-10.0, 10.0, 0.0), 1.0);
        assert_eq!(result, Intersect::Outside);

        // 球完全在 X 平面内但完全在 Y 平面外 → Outside
        let result = collection.intersect_bounding_sphere(DVec3::new(10.0, -10.0, 0.0), 1.0);
        assert_eq!(result, Intersect::Outside);

        // 球完全在两者内 → Inside
        let result = collection.intersect_bounding_sphere(DVec3::new(10.0, 10.0, 0.0), 1.0);
        assert_eq!(result, Intersect::Inside);

        // 球完全在两者外 → Outside
        let result = collection.intersect_bounding_sphere(DVec3::new(-10.0, -10.0, 0.0), 1.0);
        assert_eq!(result, Intersect::Outside);
    }

    #[test]
    fn test_intersect_bounding_sphere_intersection_inside_any_plane() {
        // Intersection 模式：仅在所有平面之外才裁剪。
        // 若球完全在任一平面内，则被保留。
        let collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0), // y >= 0
            ClippingPlane::new(DVec3::X, 0.0), // x >= 0
        ]);

        // 球完全在 Y 内、完全在 X 外 → Inside（每个点都在 Y 内）
        let result = collection.intersect_bounding_sphere(DVec3::new(-10.0, 10.0, 0.0), 1.0);
        assert_eq!(result, Intersect::Inside);

        // 球完全在 X 内、完全在 Y 外 → Inside（每个点都在 X 内）
        let result = collection.intersect_bounding_sphere(DVec3::new(10.0, -10.0, 0.0), 1.0);
        assert_eq!(result, Intersect::Inside);

        // 球完全在两者外 → Outside
        let result = collection.intersect_bounding_sphere(DVec3::new(-10.0, -10.0, 0.0), 1.0);
        assert_eq!(result, Intersect::Outside);

        // 球完全在两者内 → Inside
        let result = collection.intersect_bounding_sphere(DVec3::new(10.0, 10.0, 0.0), 1.0);
        assert_eq!(result, Intersect::Inside);
    }

    #[test]
    fn test_intersect_bounding_sphere_intersecting_multi_plane() {
        // 球横跨所有平面 → Intersecting
        let mut collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
            ClippingPlane::new(DVec3::X, 0.0),
        ]);

        // Intersection 模式：球横跨两个平面
        let result = collection.intersect_bounding_sphere(DVec3::new(0.5, 0.5, 0.0), 1.0);
        assert_eq!(result, Intersect::Intersecting);

        // Union 模式：球横跨两个平面
        collection.union_clipping_regions = true;
        let result = collection.intersect_bounding_sphere(DVec3::new(0.5, 0.5, 0.0), 1.0);
        assert_eq!(result, Intersect::Intersecting);
    }

    #[test]
    fn test_pack_planes() {
        let collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 5.0),
            ClippingPlane::new(DVec3::X, -3.0),
        ]);

        let packed = collection.pack_planes();
        assert_eq!(packed.len(), 8); // 2 个平面 * 4 个值

        // 第一个平面：法线 Y，distance 5
        assert!((packed[1] - 1.0).abs() < 1e-10);
        assert!((packed[3] - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_edge_factor_no_edge() {
        let collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
        ]);

        // 远离边缘的点
        let factor = collection.edge_factor(DVec3::new(0.0, 10.0, 0.0), 0.1);
        assert!((factor).abs() < 1e-10);
    }

    #[test]
    fn test_edge_factor_near_edge() {
        let mut collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
        ]);
        collection.edge_width = 2.0;

        // 非常靠近裁剪平面的点
        let factor = collection.edge_factor(DVec3::new(0.0, 0.05, 0.0), 0.1);
        assert!(factor > 0.0);
        assert!(factor <= 1.0);
    }

    #[test]
    fn test_edge_factor_zero_width() {
        let collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
        ]);

        // edge_width = 0 → 无边缘高亮
        let factor = collection.edge_factor(DVec3::new(0.0, 0.01, 0.0), 0.1);
        assert!((factor).abs() < 1e-10);
    }

    // ─── M6.2 CPU 参考交叉校验 ────────────────────────────────
    //
    // 独立地对 union / intersection 裁剪判定做暴力参考实现，直接依据上游
    // CesiumJS 语义（`ClippingPlaneCollection.js` 的 `unionIntersectFunction` =
    // `v === OUTSIDE`、`defaultIntersectFunction` = `v === INSIDE`）书写，而非
    // 复用生产代码路径。它们守护 `is_clipped` / `intersect_bounding_sphere`
    // 免于静默的语义漂移，并钉住 M6.2 bevy-render 适配器所依赖的、CPU 侧
    // `world_planes()` 烘焙不变式——该适配器预先变换平面，而非逐片元做
    // `czm_transformPlane`。

    /// 暴力参考：先把点变换到平面局部空间，然后直接应用
    /// union（任一在外）/ intersection（全部在外）规则。
    fn reference_is_clipped(collection: &ClippingPlaneCollection, point: DVec3) -> bool {
        if !collection.enabled || collection.is_empty() {
            return false;
        }
        let local = collection.model_matrix.inverse().transform_point3(point);
        // 当一个点的带符号距离为负时，它在该平面之外。
        let outside: Vec<bool> = (0..collection.len())
            .map(|i| {
                let p = collection.get(i).unwrap();
                (p.normal.dot(local) + p.distance) < 0.0
            })
            .collect();
        if collection.union_clipping_regions {
            outside.iter().any(|&o| o)
        } else {
            outside.iter().all(|&o| o)
        }
    }

    #[test]
    fn test_is_clipped_matches_cpu_reference() {
        // 在两种模式和一个非单位模型矩阵下对一组确定性点做扫描，
        // 并与暴力参考交叉对照。
        let mut collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
            ClippingPlane::new(DVec3::X, -2.0),
            ClippingPlane::new(DVec3::new(1.0, 1.0, 0.0), 1.0),
        ]);
        collection.model_matrix = DMat4::from_translation(DVec3::new(3.0, -1.0, 2.0));

        let samples = [
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(5.0, 5.0, 5.0),
            DVec3::new(-4.0, 2.0, 0.0),
            DVec3::new(2.0, -3.0, 1.0),
            DVec3::new(-1.0, -1.0, -1.0),
            DVec3::new(10.0, -2.0, 4.0),
        ];

        for union in [false, true] {
            collection.union_clipping_regions = union;
            for &p in &samples {
                assert_eq!(
                    collection.is_clipped(p),
                    reference_is_clipped(&collection, p),
                    "is_clipped diverged from CPU reference (union={union}, point={p:?})"
                );
            }
        }
    }

    #[test]
    fn test_world_planes_transform_equivalence() {
        // 适配器通过 `world_planes()` 在 CPU 上把 `model_matrix` 烘焙进平面，
        // 而非在 GPU 上逐片元变换。证明该烘焙与 `is_clipped` 的局部空间
        // 测试符号等价：一个世界点被集合裁剪，当且仅当对*世界空间*平面
        // 应用 union/intersection 规则（直接对世界点测试、不做逆变换）
        // 得到相同的结果。
        let mut collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
            ClippingPlane::new(DVec3::X, -2.0),
        ]);
        collection.model_matrix = DMat4::from_translation(DVec3::new(1.0, 2.0, -3.0));

        let world = collection.world_planes();
        assert_eq!(world.len(), collection.len());

        let samples = [
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(3.0, 4.0, 5.0),
            DVec3::new(-2.0, 1.0, 0.0),
            DVec3::new(1.0, -5.0, 2.0),
        ];

        for union in [false, true] {
            collection.union_clipping_regions = union;
            let world = collection.world_planes();
            for &p in &samples {
                let outside: Vec<bool> = world.iter().map(|pl| pl.signed_distance(p) < 0.0).collect();
                let clipped_world = if union {
                    outside.iter().any(|&o| o)
                } else {
                    outside.iter().all(|&o| o)
                };
                assert_eq!(
                    collection.is_clipped(p),
                    clipped_world,
                    "world_planes() baking diverged from is_clipped (union={union}, point={p:?})"
                );
            }
        }
    }

    #[test]
    fn test_intersect_bounding_sphere_consistent_with_is_clipped() {
        // 仅中心的合理性检查：当球半径约为 0 时，体积测试必须与中心点上的
        // 点测试一致（Inside ⇒ 不裁剪，Outside ⇒ 裁剪）。
        let collection = ClippingPlaneCollection::with_planes(vec![
            ClippingPlane::new(DVec3::Y, 0.0),
        ]);
        let r = 1e-9;

        let centre_inside = DVec3::new(0.0, 5.0, 0.0);
        assert_eq!(
            collection.intersect_bounding_sphere(centre_inside, r),
            Intersect::Inside
        );
        assert!(!collection.is_clipped(centre_inside));

        let centre_outside = DVec3::new(0.0, -5.0, 0.0);
        assert_eq!(
            collection.intersect_bounding_sphere(centre_outside, r),
            Intersect::Outside
        );
        assert!(collection.is_clipped(centre_outside));
    }
}
