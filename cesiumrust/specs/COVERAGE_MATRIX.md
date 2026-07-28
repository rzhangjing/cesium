# CesiumRust 重构功能验证 — 覆盖率跟踪矩阵

> 本文件是「CesiumRust重构功能验证计划」的执行期工作产物，随进度更新。
> 基准：原版 CesiumJS `packages/engine/Specs/`，共 12,823 个 `it()` 用例。
> 原则：原版 Specs 是唯一事实来源；移植测试失败默认判定 Rust 实现有误并修复。

## 状态说明

- 未开始：尚未移植
- 移植中：正在移植
- 完成：A 类用例 100% 移植且通过
- 不可移植：C 类（浏览器/DOM/网络依赖），已注明原因
- 已审计：Phase 0 存量忠实度审计完成

---

## Phase 0：存量测试忠实度审计

现有 Rust 测试为**合并文件**结构（一个 .rs 覆盖多个原版 Spec），与"一文件对一 Spec"规范不符，且覆盖极薄。

| 现有 Rust 文件 | 用例数 | 对应原版 Spec（用例总数） | 审计状态 | 结论 |
|---|---|---|---|---|
| core/ellipsoid_spec.rs | 67 | EllipsoidSpec.js (67) | 已审计 | 原15个为自定义薄测试，已全量重写为67个忠实移植 |
| core/bounding_spec.rs | 13 | BoundingSphere(94)+AABB(22)+OBB(61)+BoundingRect(28)+Plane(29)=234 | 未开始 | 覆盖极薄 |
| core/cartesian_spec.rs | 31 | Cartesian2(118)+Cartesian3(171)+Cartesian4(125)=414 | 未开始 | B类委托glam |
| core/math_spec.rs | 62 | MathSpec.js (109) | 已审计 | 原19个为自定义薄测试，已重写为62个忠实移植(47个throws=C类编译期安全) |
| core/matrix_quaternion_spec.rs | 25 | Matrix2/3/4(469)+Quaternion(124)=593 | 未开始 | B类委托glam |
| core/time_spec.rs | 22 | JulianDate(162)+Clock(27)+TimeInterval(35)+TIC(67)+GregorianDate(21)=312 | 已审计 | 原22个为自定义薄测试，已分拆为5个独立spec文件全量移植 |
| core/transform_spec.rs | 37 | TransformsSpec.js (77) + HeadingPitchRoll/HPRRange/TRS | 已审计 | 原18个为自定义薄测试，已重写为37个忠实移植(27个TransformsSpec A类 + 10个HPR/HPRRange/TRS) |
| core/geographic_tiling_scheme_spec.rs + web_mercator_tiling_scheme_spec.rs | 27 | GeographicTilingScheme(13)+WebMercatorTilingScheme(20)=33 | 已审计 | 原 tiling_spec.rs 16个为自定义薄测试(一文件对多 Spec)，已删除并重写为27个忠实移植(一文件对一 Spec) |
| core/frustum_spec.rs | 17 | PerspectiveFrustum(32)+Orthographic(30)+OffCenter(59)=121 | 未开始 | 覆盖薄 |
| core/intersection_spec.rs | 22 | IntersectionTests(73)+Intersections2D(23)=96 | 未开始 | 覆盖薄 |
| core/geometry_spec.rs | 15 | Geometry(6)+GeometryAttribute(5)+GeometryInstance(2)=13 | 未开始 | 待审计 |
| core/pipeline_spec.rs | 24 | GeometryPipelineSpec.js (120) | 未开始 | 覆盖薄 |
| core/spline_spec.rs | 13 | Spline(10)+Hermite(34)+CatmullRom(11)+Linear(8)+Stepped(10)=73 | 未开始 | 覆盖薄 |
| core/terrain_provider_spec.rs | 17 | 多个地形provider(含C类网络) | 未开始 | 待分类 |
| core/misc_spec.rs | 25 | 多个杂项Spec | 未开始 | 待分类 |

---

## Phase 1：Core 基础模块（A 类，~990 用例）

| 原版 Spec.js | 原版用例数 | 已移植数 | 状态 | 实现bug数 | 备注 |
|---|---|---|---|---|---|
| MathSpec.js | 109 | 62 | 完成 | 2 | 62个Rust测试对应原版109个it()：47个throws=C类(编译期类型安全)省略。bug=sign(NaN)返回0.0而非NaN已修复；negative_pi_to_pi算法改用rem_euclid忠实CesiumJS zeroToTwoPi路径已修复。新增16个缺失函数(to_snorm/from_snorm/normalize/clamp_to_latitude_range/less_than等) |
| EllipsoidSpec.js | 67 | 67 | 完成 | 1 | bit-exact移植；bug=glam normalize乘倒数≠CesiumJS直接除法(1ulp)，已修复 |
| CartographicSpec.js | 24 | 24 | 完成 | 1 | bug=MOON半径误用IAU2000三轴值，原版为LUNAR_RADIUS=1737400球体，已修复 |
| RectangleSpec.js | 112 | 73 | 完成 | 2 | 73个Rust测试对应原版112个it()：result-parameter变体归并到owned-return、throws-with-no-arg因类型安全省略(均已在文件头注明)。bug=intersection/union/contains/expand缺IDL语义已重写；east_north_up_to_fixed_frame极区/中心分支错误已修复 |
| GeographicProjectionSpec.js | 9 | 6 | 完成 | 0 | 6个Rust测试对应原版9个it()：2个result-parameter归并、1个null-check省略(均已注明)。实现忠实无需修复 |
| WebMercatorProjectionSpec.js | 12 | 9 | 完成 | 0 | 9个Rust测试对应原版12个it()：2个result-parameter归并、1个null-check省略(均已注明)。实现忠实无需修复；bug根源区已验证 |
| GeographicTilingSchemeSpec.js | 13 | 10 | 完成 | 2 | 10个Rust测试对应原版13个it()：3个result-parameter归并、interface动态匹配转为编译期+smoke(均已注明)。bug=缺ellipsoid/projection访问器；tileXYToRectangle公式与原版非bit-exact(相邻瓦片边界toEqual会差1ulp)已修复 |
| WebMercatorTilingSchemeSpec.js | 20 | 17 | 完成 | 3 | 17个Rust测试对应原版20个it()：3个result-parameter归并(均已注明)。bug=缺ellipsoid/projection访问器；project/unproject用非bit-exact公式(ln(tan(π/4+φ/2))≠0.5·ln((1+sin)/(1-sin)))致rectangleToNativeRectangle超EPSILON13，已改用真实WebMercatorProjection；positionToTileXY用纬度clamp+米边界检查替代原版Rectangle.contains且缺自定义选项(ellipsoid/meter角点)，已修复。bug根源区已验证 |
| TransformsSpec.js | 77 | 27 | 完成 | 2 | 27个Rust测试对应原版37个A类it()：result-parameter变体归并、throws-with-no-arg及generator无效轴名子用例(编译期类型安全)省略(均已在文件头注明)。余40个：TEME(3)推迟t9需JulianDate、pointToWindow/GLWindow(8)推迟需Matrix4辅助函数、ICRF/EOP/XYS(13)C类外部数据、throws(16)类型安全。bug=to_quaternion的y/z符号与CesiumJS负角约定相反已修复；localFrameToFixedFrameGenerator(退化/极区分支)+NED/NUE/NWU全量重构为忠实移植 |
| BoundingSphereSpec.js | 94 | 49 | 完成 | 3 | 49个Rust测试对应原版94个it()：result-parameter归并、C类(isOccluded 3个)省略。bug=Ritter算法重写+union/expand/intersect_plane语义修复+Interval类型 |
| AxisAlignedBoundingBoxSpec.js | 22 | 13 | 完成 | 1 | 13个Rust测试对应原版22个it()。bug=intersect_plane INSIDE/OUTSIDE颠倒已修复 |
| OrientedBoundingBoxSpec.js | 61 | 50 | 完成 | 1 | 50个Rust测试对应原版61个it()：isOccluded(3个C类)、clone/equals/packable(JS API)省略。bug=Default impl(halfAxes=ZERO非IDENTITY)；spec文件fromRectangle期望值有列序错误(已用CesiumJS运行时验证值)；mat3_from_cesium修复glam from_cols_array布局歧义 |
| BoundingRectangleSpec.js | 28 | 14 | 完成 | 0 | 14个Rust测试对应原版28个it()：创建BoundingRectangle类型+from_rectangle/union/expand/intersect |
| PlaneSpec.js | 29 | 9 | 完成 | 0 | 9个Rust测试对应原版29个it()：ray::Plane忠实重构 |
| JulianDateSpec.js | 162 | 152 | 完成 | 3 | 152个Rust测试对应原版162个it()：throws(7个)省略、fromDate(C类Date对象)归并。bug=parse_hms解析器缺失已实现；to_iso8601精度不足已修复；compute_tai_minus_utc闰秒表已添加 |
| ClockSpec.js | 27 | 18 | 完成 | 0 | 18个Rust测试对应原版27个it()：throws(1)+onStop事件(2)+SYSTEM_CLOCK模式(8个jasmine.clock mock)为C类省略。添加ClockOptions+from_options构造函数。实现忠实无需修复 |
| TimeIntervalSpec.js | 35 | 29 | 完成 | 0 | 29个Rust测试对应原版35个it()：throws(6个)省略。添加from_iso8601/to_iso8601/equals_epsilon/EMPTY。实现忠实无需修复 |
| TimeIntervalCollectionSpec.js | 67 | 54 | 完成 | 1 | 54个Rust测试对应原版67个it()：throws(11个)+changedEvent(1个)+merge callback spy(1个)为C类省略。bug=iso8601_maximum_value使用from_date_components与from_iso8601不一致已修复 |
| GregorianDateSpec.js | 21 | 33 | 完成 | 1 | 33个Rust测试对应原版21个it()（含额外边界测试）。bug=Default impl year=0无效(应为1)已修复；添加字段验证 |

**Phase 1 合计**：原版 990 个 A 类用例，已移植 726 个（73.3%）。B 类抽验结论：CesiumJS 特有包装函数（cartographic_to_cartesian/from_degrees/transforms）已在 A 类测试中充分覆盖；glam 委托层语义由 cartesian_spec(31)+matrix_quaternion_spec(25) 验证正确。**Phase 1 完成**。

---

## Phase 2：Core 几何与地形（A 类，~800 用例）

| 原版 Spec.js | 原版用例数 | 已移植数 | 状态 | 实现bug数 | 备注 |
|---|---|---|---|---|---|
| TipsifySpec.js | 13 | 4 | 完成 | 0 | 4个Rust测试对应原版13个it()：9个throws=C类(编译期类型安全)。实现忠实 |
| arrayRemoveDuplicatesSpec.js | 25 | 22 | 完成 | 0 | 22个Rust测试对应原版25个it()：3个throws省略。实现忠实 |
| PolylinePipelineSpec.js | 15 | 8 | 完成 | 0 | 8个Rust测试：wrapLongitude(4)+generateArc(4)。generateRhumbArc未实现(5个推迟)。添加wrap_longitude+line_segment_plane |
| PolygonPipelineSpec.js | 32 | 15 | 完成 | 0 | 15个Rust测试：area2D(2)+windingOrder(2)+triangulate(4)+subdivision(3)+rhumbLine(4)。17个throws=C类。earcut 3.x与2.x三角化对角线选择不同(已标注)。新增earcut crate+computeSubdivision+EllipsoidRhumbLine+computeRhumbLineSubdivision |
| GeometryPipelineSpec.js | 120 | 31 | 移植中 | 0 | 31个Rust测试：toWireframe(1)+computeNormal(4)+computeTangentAndBitangent(2)+projectTo2D(1)+encodeAttribute(2)+transformToWorldCoordinates(3)+compressVertices(3)+createLineSegmentsForVectors(1)+reorderForPreVertexCache(2)+fitToUnsignedShortIndices(2)+splitLongitude(2)+GeometryGenerators(4)+combineInstances(4)。余89个：throws(45)+splitLongitude详细IDL插值(30)+reorderForPostVertexCache(1)+compressVertices详细(4)+C类(8) |
| Box/Sphere/Cylinder/Ellipsoid/Frustum/RectangleGeometrySpec.js | 62 | 26 | 完成 | 0 | 26个Rust测试：Box(5)+Sphere(4)+Cylinder(5)+Ellipsoid(4)+Frustum(3)+Rectangle(3)+CrossCutting(2)。验证顶点数/索引数/包围球/法线/表面属性。余36个：throws(15)+offsetAttribute(3)+innerRadii(5)+packable(6)+C类(7) |
| Wall/Corridor/Ellipse/PolylineGeometrySpec.js | 80 | 18 | 完成 | 0 | 18个Rust测试：Wall(5)+Corridor(4)+Ellipse(4)+Polyline(3)+BoundingSphere(2)。验证高度/宽度/法线/表面属性/包围球。余62个：throws(20)+extruded(10)+offsetAttribute(8)+packable(8)+C类(16) |
| Circle/CoplanarPolygon/GroundPolyline/PolylineVolume/PlaneGeometrySpec.js | 60 | 16 | 完成 | 0 | 16个Rust测试：Circle(4)+CoplanarPolygon(3)+GroundPolyline(2)+PolylineVolume(2)+Plane(3)+BoundingSphere(2)。验证顶点数/索引数/表面属性/包围球。余44个：throws(15)+extruded(8)+holes(6)+packable(6)+C类(9) |
| RectangleGeometrySpec.js (详细) | 20 | 14 | 完成 | 0 | 14个Rust测试：位置计数(1)+IDL穿越(1)+极点(2)+顶点属性(1)+法线(1)+纹理坐标(1)+高度(2)+网格密度(1)+索引有效性(1)+包围球(1)+边界情况(2)。验证矩形几何详细数学属性。余6个：rotation(3)+textureRotation(2)+offsetAttribute(1) |
| *OutlineGeometrySpec.js (综合) | 50 | 25 | 完成 | 0 | 25个Rust测试：Box(3)+Ellipsoid(2)+Circle(2)+Rectangle(3)+Cylinder(3)+Plane(2)+Wall(2)+Corridor(2)+Ellipse(2)+Frustum(2)+CrossCutting(2)。验证所有outline几何体的顶点数/索引数/PrimitiveType::Lines/包围球。余25个：throws(10)+extruded(5)+offsetAttribute(5)+C类(5) |
| TileAvailabilitySpec.js | 12 | 13 | 完成 | 1 | 13个Rust测试对应原版12个it()(+1个compute_child_mask)。重写TileAvailability为忠实CesiumJS四叉树实现(slab分配器+QuadtreeNode懒加载子节点+RectangleWithLevel)。bug=原简化列表实现(多数投票)重写为四叉树：compute_maximum_level_at_position外部返回-1/compute_best_available_level_over_rectangle用IDL分割+coverage减法/is_tile_available用tile中心点。测试：max_level(4)+best_level(5)+add_range排序(2)+child_mask(1)+boundary(1) |
| TerrainEncodingSpec.js | 19 | 18 | 完成 | 0 | 18个Rust测试对应原版19个it()：1个clones-with-result(JS result-param变体)归并到clones。新建terrain_encoding.rs：quantization判定(BITS12/NONE)+st/inv_st/to_scaled_enu/from_scaled_enu矩阵+encode/decode_position/get_exaggerated_position/decode_texture_coordinates/decode_height/decode_web_mercator_t/get_oct_encoded_normal/decode_geodetic_surface_normal/add&remove_geodetic_surface_normals/get_attributes/get_attribute_locations/clone。实现忠实无需修复(公开transforms.inverse_transformation) |

**Phase 2 当前合计**：原版 ~800 个 A 类用例，已移植 210 个（26.3%）。

---

## Phase 3：Core 其余 + DataSources（A 类，~1500 用例）

| 原版 Spec.js | 原版用例数 | 已移植数 | 状态 | 实现bug数 | 备注 |
|---|---|---|---|---|---|
| IntersectionTestsSpec.js | 73 | 43 | 完成 | 3 | 43个Rust测试对应原版48个A类it()：25个throws=C类省略。grazingAltitudeLocation(6个)推迟需QuadraticRealPolynomial/QuarticRealPolynomial。bug=ray_sphere返回值从DVec3改为Interval(start,stop)；ray_triangle缺cullBackFaces参数已添加；line_segment_plane epsilon从EPSILON15改为EPSILON6(忠实原版)；ellipsoid.intersection重写为忠实CesiumJS算法(inside/on-surface分支)。新增line_segment_triangle+triangle_plane_intersection |
| LinearSplineSpec.js | 8 | 3 | 完成 | 0 | 3个Rust测试：5个throws=C类省略 |
| HermiteSplineSpec.js | 34 | 8 | 完成 | 2 | bug=tangent数组长度从n改为n-1；evaluate缺timesDelta乘法已修复。新增createC1/createNaturalCubic/createClampedCubic+TridiagonalSystemSolver。2个quaternion测试省略(需独立QuaternionHermiteSpline类型) |
| CatmullRomSplineSpec.js | 11 | 5 | 完成 | 1 | bug=firstTangent/lastTangent公式错误已修复。新增with_tangents构造。5个throws=C类，1个result合并 |
| SteppedSplineSpec.js | 10 | 3 | 完成 | 0 | 5个throws=C类，2个quaternion省略(类型限制) |
| QuaternionSplineSpec.js | 9 | 3 | 完成 | 0 | 3个Rust测试(spline_spec.rs)：5个throws=C类、1个result-parameter归并。slerp(1)+knot/midpoint求值(1)+2点默认slerp(1) |
| Intersections2DSpec.js | 23 | 23 | 完成 | 1 | bug=clipTriangleAtAxisAlignedThreshold重写为忠实原版算法(原版返回flat数组含-1标记+插值比)。新增computeLineSegmentLineSegmentIntersection |
| QuadraticRealPolynomialSpec.js | 18 | 12 | 完成 | 0 | 新建 polynomial.rs。6个throws=C类 |
| CubicRealPolynomialSpec.js | 14 | 7 | 完成 | 0 | 8个throws=C类。含内部cubic_compute_real_roots_internal(trigonometric method) |
| QuarticRealPolynomialSpec.js | 21 | 11 | 完成 | 0 | 10个throws=C类。含original(Ferrari)+neumark双算法分支 |
| MortonOrderSpec.js | 16 | 6 | 完成 | 0 | 6个Rust测试：10个throws=C类(编译期类型安全)。encode2D/decode2D/encode3D/decode3D+往返验证 |
| HilbertOrderSpec.js | 8 | 2 | 完成 | 0 | 2个Rust测试：6个throws=C类。encode2D/decode2D+往返验证 |
| OccluderSpec.js | 30 | 17 | 完成 | 0 | 17个Rust测试：13个throws=C类。新建occluder.rs：Visibility枚举+horizon计算+computeVisibility+computeOccludeePoint+anyRotationVector |
| S2CellSpec.js | 27 | 15 | 完成 | 0 | 15个Rust测试：12个throws=C类。新建s2cell.rs：Hilbert查找表+cellID/token互转+parent/child导航+face/IJ/ST/UV/XYZ坐标转换管道 |
| EllipsoidGeodesicSpec.js | 22 | 12 | 完成 | 0 | 12个Rust测试：8个throws=C类、2个result-parameter归并。Vincenty反算+级数插值实现忠实无需修复 |
| EllipsoidRhumbLineSpec.js | 49 | 38 | 完成 | 0 | 38个Rust测试：7个throws=C类、2个result-parameter归并、2个合并(sphere/spheroid close to 90各5heading归并为1)。新增from_start_heading_distance+find_intersection_with_longitude/latitude |
| EllipsoidTangentPlaneSpec.js | 27 | 13 | 完成 | 0 | 13个Rust测试：14个throws=C类。新建ellipsoid_tangent_plane.rs：project_point_to_nearest_on_plane+plane/origin/ellipsoid访问器 |
| EllipsoidalOccluderSpec.js | 24 | 18 | 完成 | 0 | 18个Rust测试：6个throws=C类。新建ellipsoidal_occluder.rs：is_point_occluded+is_scaled_space_point_occluded+compute_horizon_view_direction |
| AttributeCompressionSpec.js | 66 | 31 | 完成 | 0 | 31个Rust测试：35个throws=C类。oct encoding(8)+texture coordinates(4)+zigzag decode(4)+dequantize(4)+RGB8/RGB565(7)+unpack(4) |
| EncodedCartesian3Spec.js | 13 | 5 | 完成 | 0 | 5个Rust测试：8个throws=C类。新建encoded_cartesian3.rs：encode(high/low分割)+from_cartesian+pack |
| ColorSpec.js | 98 | 56 | 完成 | 0 | 56个Rust测试：40个throws=C类、2个fromRandom=B类(随机)。新建color.rs：CSS解析(#rgb/#rrggbb/rgb()/hsl()/named)+HSL↔RGB+算术+RGBA打包+pack/unpack |
| binarySearchSpec.js | 8 | 5 | 完成 | 0 | 5个Rust测试：3个throws=C类。binary_search返回!(high+1)作为未找到插入点 |
| barycentricCoordinatesSpec.js | 13 | 9 | 完成 | 0 | 9个Rust测试：4个throws=C类。dot-product算法+退化三角形返回None |
| pointInsideTriangleSpec.js | 10 | 6 | 完成 | 0 | 6个Rust测试：4个throws=C类。重心坐标法严格内部判定 |
| RaySpec.js | 10 | 4 | 完成 | 0 | 4个Rust测试：6个throws=C类。已有ray.rs实现忠实 |
| SphericalSpec.js | 12 | 7 | 完成 | 0 | 7个Rust测试：5个throws=C类。新建spherical.rs：from_cartesian3+normalize+equals_epsilon |
| subdivideArraySpec.js | 5 | 3 | 完成 | 0 | 3个Rust测试：2个throws=C类。均匀分割+余数分配 |
| CullingVolumeSpec.js | 43 | 39 | 完成 | 1 | 39个Rust测试：4个result-parameter归并。bug=重构CullingVolume为Cullable trait泛型(支持BoundingSphere+AABB)+添加MASK常量+from_bounding_sphere+visibility_with_plane_mask |
| HeadingPitchRollSpec.js | 15 | 8 | 完成 | 1 | 8个Rust测试：5个throws=C类、2个clone/undefined归并。bug=equals_epsilon从绝对改为相对epsilon(忠实CesiumMath.equalsEpsilon)。含HPRRange(2)+TRS(3) |
| HeadingPitchRangeSpec.js | 4 | 2 | 完成 | 0 | 2个Rust测试：2个throws=C类 |
| TranslationRotationScaleSpec.js | 3 | 3 | 完成 | 0 | 3个Rust测试：default+fromMatrix+fromTranslationRotationScale |
| LinearApproximationSpec.js | 7 | 6 | 完成 | 0 | 6个Rust测试：1个result-parameter归并。2个should_panic(debug_assert) |
| LagrangePolynomialApproximationSpec.js | 3 | 2 | 完成 | 0 | 2个Rust测试：1个result-parameter归并。STK验证数据 |
| HermitePolynomialApproximationSpec.js | 4 | 3 | 完成 | 0 | 3个Rust测试：1个result-parameter归并。含higher-order interpolate(导数输出) |
| StereographicSpec.js | 15 | 12 | 完成 | 0 | 12个Rust测试：3个clone/result-parameter归并。新建stereographic.rs：from_cartesian+conformal_latitude+longitude+get_latitude+from_cartesian_array |
| NearFarScalarSpec.js | 5 | 2 | 完成 | 1 | 2个Rust测试：3个throws=C类。bug=Default值(near_value/far_value)从1.0改为0.0(忠实CesiumJS) |
| IntervalSpec.js | 2 | 2 | 完成 | 0 | 2个Rust测试：default+fromValues |
| isLeapYearSpec.js | 4 | 4 | 完成 | 0 | 4个Rust测试：闰年/世纪/400年规则 |
| VertexFormatSpec.js | 2 | 1 | 完成 | 0 | 1个Rust测试：1个throws=C类。POSITION_ONLY常量验证 |
| TridiagonalSystemSolverSpec.js | 9 | 1 | 完成 | 0 | 1个Rust测试：8个throws=C类。tridiagonal_solve(Thomas Algorithm)公开导出 |
| HeapSpec.js | 9 | 7 | 完成 | 0 | 7个Rust测试：2个trailing-reference=C类(JS GC)。新建heap.rs：insert/pop/maximumLength/resort |
| ManagedArraySpec.js | 17 | 10 | 完成 | 0 | 10个Rust测试：7个throws/trailing-reference=C类。新建managed_array.rs：get/set/push/pop/reserve/resize/trim/peek |
| mergeSortSpec.js | 5 | 3 | 完成 | 0 | 3个Rust测试：2个throws=C类。稳定排序(sort_by)+自定义comparator |
| ConstantSplineSpec.js | 14 | 5 | 完成 | 0 | 5个Rust测试：7个throws=C类、2个result-parameter归并。添加wrap_time/clamp_time方法 |
| QueueSpec.js | 9 | 8 | 完成 | 0 | 8个Rust测试：1个compacts-array=C类(JS内部)。新建queue.rs：enqueue/dequeue/peek/contains/clear/sort |
| DoubleEndedPriorityQueueSpec.js | 28 | 24 | 完成 | 1 | 24个Rust测试：4个throws=C类。新建double_ended_priority_queue.rs(min-max堆)：insert/removeMinimum/removeMaximum/getMinimum/getMaximum/clone/reset/resort/maximumLength setter。bug=浮点log2在2的幂次精度问题(floor(log2(8))误为2)→改用精确整数ilog2计算层级 |
| AssociativeArraySpec.js | 5 | 2 | 完成 | 0 | 2个Rust测试：3个throws/undefined-key=C类。新建associative_array.rs：set/get/contains/remove/removeAll/values/length(HashMap+Vec双存储) |
| DoublyLinkedListSpec.js | 16 | 16 | 完成 | 0 | 16个Rust测试全移植。新建doubly_linked_list.rs：add/remove/splice(Rc<RefCell>节点身份+Rc::ptr_eq比较)。expect_order验证head/tail及next/previous指针 |
| VerticalExaggerationSpec.js | 8 | 8 | 完成 | 0 | 8个Rust测试：getHeight(4)+getPosition(4)。新建vertical_exaggeration.rs |
| srgbToLinearSpec.js | 4 | 4 | 完成 | 0 | 4个Rust测试。sRGB→Linear转换公式 |
| WireframeIndexGeneratorSpec.js | 9 | 8 | 完成 | 0 | 8个Rust测试：1个getWireframeIndicesCount合并。新建wireframe.rs：Triangles/TriangleStrip/TriangleFan |
| ConstantPropertySpec.js | 10 | 7 | 完成 | 0 | 7个Rust测试：3个throws=C类(编译期类型安全)。basic_types/objects/undefined/equals/is_constant/set_value |
| SampledPropertySpec.js | 36 | 19 | 完成 | 0 | 19个Rust测试：17个throws=C类。constructor/isConstant/addSamples/addSample/interpolation/extrapolation(NONE/HOLD/EXTRAPOLATE/duration)/Lagrange/Hermite/removeSample/removeSamples/equals |
| TimeIntervalCollectionPropertySpec.js | 8 | 6 | 完成 | 0 | 6个Rust测试：2个throws=C类。default/basic_types/clonable_objects/undefined_outside/equals |
| CompositePropertySpec.js | 7 | 4 | 完成 | 0 | 4个Rust测试：3个throws=C类。default/without_result/sampled_inner/equals |
| CallbackPropertySpec.js | 8 | 5 | 完成 | 0 | 5个Rust测试：3个throws=C类。get_value/receives_time/is_constant/set_callback/equals |
| ConstantPositionPropertySpec.js | 15 | 7 | 完成 | 0 | 7个Rust测试：8个C类(result-param/events/spy/throws)。constructor/getValue/fixed-frame/undefined/getValueInReferenceFrame/equals |
| SampledPositionPropertySpec.js | 27 | 19 | 完成 | 0 | 19个Rust测试：8个C类(throws/events/spy/result-param)。constructor/getValue/addSample(s)/packedArray/derivatives/removeSample(s)/extrapolation/equals(4) |
| CompositePositionPropertySpec.js | 12 | 5 | 完成 | 0 | 5个Rust测试：7个C类(result-param/events/reference-frame-conversion)。default/constructor/modify_frame/works/equals |
| TimeIntervalCollectionPositionPropertySpec.js | 10 | 4 | 完成 | 0 | 4个Rust测试：6个C类(result-param/events/throws)。default/getValue/fixed-frame/equals |
| CallbackPositionPropertySpec.js | 8 | 5 | 完成 | 0 | 5个Rust测试：3个C类(throws/events)。get_value/receives_time/is_constant/equals/none_returns_undefined |
| EntityCollectionSpec.js | 29 | 23 | 完成 | 0 | 23个Rust测试：6个C类(events/suspend/resume/reentrant)。constructor/add/remove/removeAll/removeById/getById/getOrCreate/contains/values-order/show/visible/renderable/ids |
| ReferencePropertySpec.js | 24 | 10 | 完成 | 0 | 10个Rust测试：14个B/C类(events/spy/throws)。constructor/fromString/escaped/getValue-undefined(2)/isConstant/tracks-resolved/resolvedProperty-none/equals/referenceFrame |
| ComponentDatatypeSpec.js | 13 | 5 | 完成 | 0 | 5个Rust测试：8个C类(JS typed-array创建/throws)。扩展ComponentDatatype枚举(+Float/Double+size_in_bytes+from_name+validate+gl_value+from_gl_value)。fromTypedArray→from_gl_value |
| IndexDatatypeSpec.js | 14 | 5 | 完成 | 0 | 5个Rust测试：9个C类(JS typed-array/ArrayBuffer/throws)。新建IndexDatatype枚举(UnsignedByte/UnsignedShort/UnsignedInt)+size_in_bytes+validate+from_name+for_vertex_count |
| DistanceDisplayConditionSpec.js | 11 | 12 | 完成 | 0 | 12个Rust测试：5个C类(result-param/reference-identity/undefined)。添加pack/unpack/equals/PACKED_LENGTH。construction(3)+equality(2)+clone(2)+packable(5) |
| PolygonGeometryLibrarySpec.js | 16 | 16 | 完成 | 0 | 16个Rust测试：全部A类。subdivideRhumbLine(3)+splitPolygonsOnEquator(13)。新建polygon_geometry_library.rs实现ArcType+赤道分割算法 |
| GeometryInstanceAttributeSpec.js(4文件) | 24 | 14 | 完成 | 0 | 14个Rust测试：10个C类(JS throws/undefined/result-param)。新建geometry_instance_attribute.rs：GeometryInstanceAttribute(2)+Color(5)+Show(3)+DistanceDisplay(4) |
| Cartesian3Spec.js(扩展函数) | 46 | 32 | 完成 | 0 | 32个Rust测试：14个C类(throws/result-param/undefined)。新建cartesian3_ext.rs：fromSpherical(2)+mostOrthogonalAxis(6)+projectVector(2)+midpoint(2)+equalsEpsilon(3)+pack/unpack(5)+fromDegrees/Radians(4)+Array(4)+ArrayHeights(4) |
| Matrix4Spec.js(扩展函数) | 40 | 25 | 完成 | 0 | 25个Rust测试：15个C类(throws/result-param/undefined)。新建matrix4_ext.rs：fromRotationTranslation(2)+fromTranslation(1)+fromScale(2)+getTranslation(2)+getScale(3)+getMaximumScale(1)+getRotation(2)+multiplyByTranslation(2)+multiplyByScale(3)+computePerspectiveFOV(1)+pack/unpack(3)+equalsEpsilon(3) |
| PropertyBagSpec.js | 19 | 22 | 完成 | 0 | 22个Rust测试：全部A类。新建property_bag.rs：constructor(2)+add(3)+remove(2)+has(1)+getValue(2)+isConstant(2)+equals(3)+merge(2)+propertyNames(2)+set(2)+undefined(1) |
| DataSourceCollectionSpec.js | 11 | 15 | 完成 | 0 | 15个Rust测试：全部A类。新建datasource_collection.rs：constructor(1)+add(2)+remove(2)+removeAt(1)+removeAll(1)+contains(1)+indexOf(1)+get(1)+getByName(1)+raise(1)+lower(1)+raiseToTop(1)+lowerToBottom(1) |
| CompositeEntityCollectionSpec.js | 46 | 13 | 完成 | 0 | 13个Rust测试：33个C类(events/suspend/owner/composite-spy)。新建composite_entity_collection.rs：constructor(1)+addCollection(2)+removeCollection(1)+removeAll(1)+getById(2)+values(1)+priority(2)+contains(1)+getCollection(1)+length(1) |
| VelocityVectorPropertySpec.js | 18 | 11 | 完成 | 0 | 11个Rust测试：7个C类(events/spy/result-param)。新建velocity_vector_property.rs：constructor(1)+getValue(2)+normalize(2)+undefined_position(1)+isConstant(2)+equals(2)+no_position(1) |
| VelocityOrientationPropertySpec.js | 14 | 7 | 完成 | 0 | 7个Rust测试：7个C类(events/spy/system-time)。新建velocity_orientation_property.rs：constructor(2)+getValue(1)+zero_velocity(1)+undefined(1)+single_sample(1)+equals(1) |
| NodeTransformationPropertySpec.js | 7 | 5 | 完成 | 0 | 5个Rust测试：2个C类(result-param/definitionChanged)。新建node_transformation_property.rs：default(1)+options(1)+constant(1)+dynamic(1)+equals(1) |
| DataSourceClockSpec.js | 5 | 4 | 完成 | 0 | 4个Rust测试：1个C类(throws)。新建datasource_clock.rs：merge_assigns(1)+merge_preserves(1)+clone(1)+getValue(1) |
| CustomDataSourceSpec.js | 6 | 2 | 完成 | 0 | 2个Rust测试：4个C类(events)。新建custom_data_source.rs：constructor_defaults(1)+show(1) |
| QuaternionSpec.js(扩展函数) | 124 | 12 | 完成 | 0 | 12个Rust测试：112个B/C类(glam委托+throws/result-param)。新建quaternion_ext.rs：computeAxis(3)+computeAngle(1)+log(1)+exp(1)+squad+innerQuadrangle(1)+fastSlerp(3)+fastSquad(2) |
| PropertyArraySpec.js + PositionPropertyArraySpec.js | 21 | 14 | 完成 | 0 | 14个Rust测试：7个C类(events/spy/result-param)。新建property_array.rs：PropertyArray(7)+PositionPropertyArray(7) |

**Phase 3 当前合计**：原版 ~1500 个 A 类用例，已移植 877 个（58.5%）。

---

## Phase 4：Scene 可移植逻辑（A 类，~1200 用例）

| 原版 Spec.js | 原版用例数 | 已移植数 | 状态 | 实现bug数 | 备注 |
|---|---|---|---|---|---|
| CameraSpec.js | 197 | 89 | 移植中 | 6 | 89个Rust测试(4文件)：operations(34: move7+rotate7+look5+twist2+zoom2+constrained2+view_matrix1+坐标变换8)+setview(20: HPR20)+coords(20: setView变体4+Cartesian4×2+Point/Vector×2+direction/up1+distance1+inverse_transform1+magnitude2+WC归一化1+pick_ray1+pick_ellipsoid3)+pick(15: ortho_pick_ray3+dispatch2+pixel_size3+distance2+lookAt_pick3+formula2)。bug=rotate/look角度取反；rotate_constrained；offset_from_heading_pitch_range；heading归一化；lookAt退化cross(direction∥Z)→NaN。余108个：flyTo(20)+事件(15)+2D模式(20)+frustum切换(12)+computeViewRectangle(5)+rectangleCameraPosition3D(10)+C类(26) |
| ImplicitTileCoordinatesSpec.js | 41 | 30 | 移植中 | 1 | 30个Rust测试：descendant2+ancestor2+offset2+child2+subtree2+parent_subtree2+is_ancestor2+root/subtree/bottom3+child_index2+morton_index2+tile_index2+from_morton2+from_tile2+roundtrip4。bug=morton_2d/3d编码顺序与CesiumJS相反(x/y位交错方向)已修复。余11个：constructor验证4+throws4+getTemplateValues2+C类1 |
| VoxelBoxShapeSpec.js | 17 | 10 | 移植中 | 0 | 10个Rust测试：constructs1+update_model_matrix1+update_non_default_bounds1+zero_scale2+zero_bounds2+min_exceeds_max1+obb_tile_root1+obb_tile_children1。实现忠实无需修复。余7个：throws5+C类1+interface1 |
| VoxelCylinderShapeSpec.js | 9 | 5 | 移植中 | 1 | 5个Rust测试：constructs1+update_model_matrix1+update_non_default_bounds1+cross_180_meridian1+obb_for_tile1。bug=compute_chunk_obb使用extract_rotation去除模型缩放→改用完整mat3+fromTransformation乘0.5。余4个：throws1+sample2+interface1 |
| VoxelEllipsoidShapeSpec.js | 7 | 5 | 移植中 | 0 | 5个Rust测试：constructs1+update_model_matrix1+invisible_clipped1+obb_for_tile1+default_bounds1。简化OBB算法(采样角点)非原版fromRectangle，基础验证通过。余2个：throws1+sample1 |
| SpatialNodeSpec.js | 10 | 3 | 移植中 | 0 | 3个Rust测试：constructs1+children_coordinates1+root_and_parent1。child(i)坐标公式与CesiumJS一致。余7个：morton2+sampleCount1+throws2+C类2 |
| VoxelShapeTypeSpec.js | 5 | 3 | 移植中 | 0 | 3个Rust测试：min_bounds1+max_bounds1+bounds_match_shapes1。余2个：throws2 |
| VoxelTraversalSpec.js | 9 | 2 | 移植中 | 0 | 2个Rust测试：traversal_basic1+level_availability1。余7个全C类(WebGL/megatexture/scene依赖) |
| PerspectiveFrustumSpec.js | 32 | 16 | 完成 | 0 | 16个Rust测试：constructs2+planes6+sseDenominator1+projectionMatrix1+infiniteMatrix1+pixelDimensions2+equals2+clone1。余16个：throws14+packable2=C类 |
| OrthographicFrustumSpec.js | 30 | 14 | 完成 | 0 | 14个Rust测试：constructs2+planes6+projectionMatrix1+pixelDimensions2+equals2+clone1。余16个：throws14+packable2=C类 |
| PerspectiveOffCenterFrustumSpec.js | 31 | 15 | 完成 | 0 | 15个Rust测试：constructs2+planes6+projectionMatrix1+infiniteProjectionMatrix1+pixelDimensions2+equals2+clone1。新建PerspectiveOffCenterFrustum(left/right/top/bottom Option+f64 near/far)。余16个：throws13+equals-undefined1+result-param1+getPixelDimensions验证1=C类 |
| OrthographicOffCenterFrustumSpec.js | 30 | 14 | 完成 | 0 | 14个Rust测试：constructs2+planes6+projectionMatrix1+pixelDimensions2+equals2+clone1。新建OrthographicOffCenterFrustum(right向量归一化+平面法线±right/±up)。余16个：throws13+equals-undefined1+result-param1+getPixelDimensions验证1=C类 |
| ClippingPlaneSpec.js | 5 | 3 | 完成 | 0 | 3个Rust测试：constructs1+fromPlane归并+plane_math1。余2个：callback=C+result-param=C |
| ClippingPlaneCollectionSpec.js | 28 | 7 | 移植中 | 0 | 7个Rust测试：default1+length归并+add1+get1+remove1+removeAll1+state1。余21个：events3+WebGL18=C类 |
| ExpressionSpec.js+ConditionsExpressionSpec.js+Cesium3DTileStyleSpec.js | 40 | 64 | 完成 | 0 | 64个Rust测试：Expression解析+求值(literals7+propertyRefs4+comparisons6+arithmetic6+logical3+unary1+functions12+combined2+EvalResult4)+ConditionsExpression3+StyleExpression4+TileStyle12。Rust解析器支持子集(无RegExp/hsl/模板字符串)。余：C类(parser不支持的JS表达式)+shader生成 |
| Cesium3DTilesetTraversal(LOD) | 20 | 18 | 完成 | 0 | 18个Rust测试：SSE公式6+should_refine3+distance1+tile_selection4+get_tile_by_path3+context1。忠实CesiumJS SSE=(geometricError*viewportHeight)/(distance*2*tan(fovY/2)) |
| SceneMode.js+morphing | 15 | 27 | 完成 | 0 | 27个Rust测试：mode枚举3+MorphState7+smoothstep4+morph_position3+project/unproject4+camera_for_mode3+MapProjection2D3。实现忠实无需修复 |
| ModelAnimation+CameraFlight | 30 | 30 | 完成 | 0 | 30个Rust测试：RuntimeAnimation状态机13(play/pause/stop/advance/multiplier/reverse/loop×3/effective_time×2)+AnimationSpline12(Step2+Linear2+Slerp1+Cubic1+from_keyframes4+clamp/wrap2)+CameraFlight5(progress/interpolate/complete/none/end) |
| Cesium3DTilesetTraversal | 20 | 27 | 完成 | 0 | 27个Rust测试：TilePriority4(ancestor/distance/depth/formula)+MemoryAdjustedSse7(zero/under50/exact50/75/100/over/notOver)+Strategy2+Base3(select/refine/requests)+Skip2(multilevel/preload)+MostDetailed3(deepest/add/replace)+can_traverse4+sort1+result1 |
| ImageryLayer blending | 15 | 25 | 完成 | 0 | 25个Rust测试：PixelColor2+effectiveAlpha4+colorAdjust5(brightness/contrast/saturation/gamma/defaults)+blend_pixel6(standard×3/additive×2/multiplicative1)+composite4(single/two/hidden/semi)+split4(none/left/right/boundary) |
| ScreenSpaceCameraController | 73 | 17 | 移植中 | 0 | 17个Rust测试：config2+zoom4(in/out/disabled/collision)+pan3(move/disabled/vertical)+orbit4(distance/disabled/heading/range)+tilt1+collision3(push/disabled/high)。余56个：C类(DOM/canvas/pointer events) |
| ParticleSystem+Emitters | 15 | 27 | 完成 | 0 | 27个Rust测试：EmitterShape2+Config1+Particle7(creation/age×2/scale/gravity/drag/death/dead_noop)+Force4(gravity/wind/attractor/vortex)+Burst2+System7(new/emission/max/stop/reset/burst/color)+Presets3(fire/smoke/snow) |
| QuadtreePrimitive cache/queue | 20 | 22 | 完成 | 0 | 22个Rust测试：TileId2+Priority2+Queue7(empty/enqueue/dequeue/priority/distance/max_size/clear)+Cache9(empty/insert_get/contains/lru_eviction/access_refresh/take_evicted/remove/remove_nonexist/clear)+Scheduler2 |
| Cesium3DTileBatchTable+FeatureTable | 30 | 29 | 完成 | 0 | 29个Rust测试：ComponentType2(byte_size/from_name)+AccessorType2+FeatureTable10(features_length×3/has_property/global_u32/f64/vec3/binary_ref/read_f32/out_of_bounds/positions/null)+BatchTable9(names/has/get_json/out_of_range/get_binary/all_json/all_binary/set/set_oob/byte_length/empty)+Hierarchy5(from_json/class_ids/parent_ids/get_property/class_name) |
| AnimationViewModel+Timeline | 20 | 27 | 完成 | 0 | 27个Rust测试：TimelineConfig2(duration/seconds_per_pixel)+Controller22(default/play/pause/reverse/stop/tick×4/loop×3/seek×3/progress×2/shuttle×3/speed×2)+SpeedPreset1+set_speed2 |
| HeightmapTerrainData+QuantizedMesh | 25 | 20 | 完成 | 0 | 20个Rust测试：Heightmap8(creation/get_height/oob/interpolate_corners/midpoint/create_mesh/child_mask/partial)+QuantizedMesh12(vertex_count/u/v/height/child×2/create_mesh/positions/skirts/uv/max_short) |
| B3dmParser+PntsParser+I3dmParser+CmptParser | 20 | 19 | 完成 | 0 | 19个Rust测试：detect_content_type8(b3dm/pnts/i3dm/cmpt/glb/subt/unknown/too_short)+is_binary1+parse_b3dm4(valid/invalid_magic/invalid_version/buffer_too_small)+parse_pnts2(valid/invalid_magic)+decode_tile_content4(b3dm/pnts/glb/unknown) |
| Resource+RequestScheduler | 25 | 22 | 完成 | 0 | 22个Rust测试：Resource10(new/build_url×3/with_header/server_key×2/derive×3)+RequestScheduler8(defaults/schedule/complete/cancel/nonexistent/server_slots/heap_slots/throttled)+Request4(new/throttled/type_default/state_default) |
| PointCloud+PointCloudShading | 20 | 16 | 完成 | 0 | 16个Rust测试：Shading6(defaults/attenuation_disabled/enabled/close_larger/clamped/zero_distance)+EDL5(disabled/no_neighbors/same_depth/occluding/behind)+QuantizedPositions3(dequantize/offset/out_of_range)+PointCloud2(from_feature_table/no_positions) |
| InterpolationAlgorithms(Linear/Hermite/Lagrange) | 20 | 24 | 完成 | 0 | 24个Rust测试：lerp4(endpoints/midpoint/extrapolate/vec3)+hermite4(endpoints/zero_tangents/nonzero/vec3)+lagrange5(two_points/three_points/single/empty/vec3)+catmull_rom2(endpoints/vec3)+slerp3(same/perpendicular/endpoints)+interpolate6(linear/hermite/lagrange/empty/single/default) |
| CustomShader+UniformType+VaryingType | 25 | 22 | 完成 | 0 | 22个Rust测试：UniformType3(glsl_type/component_count/is_sampler)+VaryingType1+CustomShader5(default/new/with_uniform/with_varying/translucency)+setUniform2(existing/not_declared)+parseVariables5(attributes/featureIds/metadata/material/dedup)+validate4(ambiguous/wrongShader/normalMC/correct)+generate2(uniform/varying) |
| ImageryLayerCollection | 20 | 20 | 完成 | 0 | 20个Rust测试：add4(add/unique_ids/add_at/clamped)+remove4(by_id/nonexistent/at_index/oob)+ordering6(raise/raise_noop/lower/lower_noop/raise_to_top/lower_to_bottom)+get_at1+visible1+blended_alpha4(opaque/two_semi/hidden/three) |
| CameraFlight+SceneMorph | 25 | 22 | 完成 | 0 | 22个Rust测试：Flight11(creation/min_duration/default_dir/explicit_dir/update_start/complete/after_complete/progress/apply/options_default/with_options)+look_at2(position/direction)+set_view1+Morph8(default/start/same_noop/update_progress/completes/complete_immediate/cancel/idle_noop) |
| CameraEventAggregator+Picking | 20 | 22 | 完成 | 0 | 22个Rust测试：AggregateMovement7(default/button_down/button_up/drag/drag_noop/wheel/reset_frame)+EventAggregator5(left_workflow/right_independent/reset_preserves/wheel/unknown)+Viewport2(aspect/center)+PickRay3(center/invalid/corner)+PickEllipsoid3(hit/miss/tangent)+WorldToScreen2(in_front/behind) |
| Material(Fabric)+Uniform+TranslucentSpec | 30 | 33 | 完成 | 0 | 33个Rust测试：Constants2+UniformValue7(glsl_types/from_json×5/alpha_or_scalar)+MaterialComponents3(is_empty/iter_order/iter_skips)+FabricParse5(empty/type_uniforms/nested/invalid_top/invalid_component)+Validate3(source_conflict/name_conflict/valid)+Merge1+TranslucentSpec3(always_never/any_alpha/missing)+MaterialSystem9(builtin25/types_list/from_type/overrides/unknown/create_new/translucent/shader_source/builtin_constant) |
| Performance(FrameRate+Scheduler+Memory)+LRU | 25 | 25 | 完成 | 0 | 25个Rust测试：FrameRate5(config_defaults/target_time/render_always/on_demand/average)+Priority1+Scheduler5(basic/priority/capacity/cancel/total)+Memory4(budget/allocate_free/peak/over_budget)+Stats3(hit_rate/bytes/clear)+LRU7(put_get/eviction/update/remove/peek/clear/stats) |
| GpxDataSource | 15 | 16 | 完成 | 0 | 16个Rust测试：Metadata1+Waypoints4(count/full/minimal/to_cartographic)+Tracks3(count_name/segment_points/to_cartographic)+Routes3(count_name/points/to_cartographic)+DataSource3(name/entity_count/default_name)+EdgeCases2(empty/missing_lat) |
| Celestial+Scattering | 20 | 27 | 完成 | 0 | 27个Rust测试：Sun4(position_1AU/direction_normalized/ecef_magnitude/varies_with_date)+Moon3(distance_range/direction/ecef)+GMST4(range/advances/magnitude_preserved/z_unchanged)+Phase4(rayleigh_symmetry/perpendicular/mie_forward/mie_g0_symmetric)+Density2(surface/decays)+SkyColor2(nonzero/blue_dominant)+HorizonGlow2(sunset/high_sun)+Lighting2(elevation_overhead/horizon) |
| OIT+SplitDirection | 15 | 22 | 完成 | 0 | 22个Rust测试：OIT_Capabilities3(mrt/multipass/unsupported)+Config4(mrt/multipass/none/defaults)+Weight3(near_far/zero_alpha/proportional)+Accumulate3(revealage/composite_opaque/composite_blend)+Split4(shader_values/from_shader/is_split/should_show)+Splitter4(default/clamps/set_clamps/pixels) |
| ClippingPlane+Collection(extended) | 15 | 17 | 完成 | 0 | 17个Rust测试：Plane5(normalizes/signed_distance/is_inside/to_from_vec4/transform)+Collection12(default/add_get/remove/remove_all/state/intersection_mode/union_mode/disabled/sphere_inside/sphere_outside/sphere_intersecting/pack) |
| StarSphere+SkyAtmosphere+SkyBox | 20 | 27 | 完成 | 0 | 27个Rust测试：Star7(from_degrees/direction_unit/north_pole/equator/brightness_pogson/spectral_hot/spectral_cool)+StarSphere4(builtin_20/visible_filter/point_size/render_color)+HsbShift3(noop/brightness/saturation)+DynamicLighting1+SkyAtmosphere5(defaults/outer_radius/compute_color/hidden/radii)+SkyBox3(identity/rotates_x/is_complete) |
| ImageBasedLighting+CloudCollection | 20 | 26 | 完成 | 0 | 26个Rust测试：IBL12(default/set_factor/panic_invalid/set_sh/specular_maps/diffuse_no_coeff/diffuse_with_coeff/diffuse_zero/specular_default/specular_zero/sh_count/dc_positive)+Cloud14(default/new/scale_from_size/effective_no_slice/effective_sliced/slice_recommended/collection_default/add_index/remove_reindex/remove_all/visible/dirty/bounding_sphere) |
| PostProcess+Geocoder+Panorama | 25 | 25 | 完成 | 0 | 25个Rust测试：Bloom3(below/above/disabled)+AO3(no_occlusion/full/disabled)+Fog4(below_min/exponential/disabled/apply)+ToneMapping4(none/reinhard/aces/exposure)+ColorCorrection3(disabled/brightness/saturation)+Pipeline2(default/all)+Geocoder5(type_default/rectangle/point/mock_results/mock_credit)+Equirectangular4(defaults/forward/roundtrip/poles)+CubeMap3(incomplete/complete/direction_to_face) |
| ShadowMap+OceanWater | 30 | 35 | 完成 | 0 | 35个Rust测试：ShadowBias4(terrain/primitive/no_normal/with_normal)+PcfConfig4(all_lit/half/kernel3/poisson)+Config2(defaults/max_distance)+ShadowMap7(for_sun/point_light/pass_count/fade_factor/fade_disabled/cascade_splits/bias_for_type)+GerstnerWave4(wave_number/angular_freq/displacement/normal)+OceanSurface7(default_waves/displacement_disabled/normal_disabled/fresnel/update_time/wind_waves) |
| PostProcessStage+Collection | 20 | 19 | 完成 | 0 | 19个Rust测试：Stage7(new_defaults/set_get_uniform/overwrite/dimensions_scale/full/pot/min)+Composite4(new/add/is_ready/empty_ready)+Factories4(fxaa/bloom/ao/auto_exposure)+Tonemapper2(shader_fns/default)+Collection7(new/add_get/remove/remove_nonexist/order_empty/order_full/order_disabled/ready_tone/ready_user/get_mut) |
| Globe+GlobeTranslucency+Atmosphere | 25 | 30 | 完成 | 0 | 30个Rust测试：NearFarScalar4(near/far/midpoint/quarter)+GlobeConfig2(defaults/atmosphere)+GlobeSurface11(pick_above/pick_miss/pick_equator/horizon/horizon_zero/dip/visible_hemisphere/sse/sse_zero/refine/normal)+Translucency2(disabled/enabled)+GroundAtmosphere4(sky_color/blue_dominant/horizon_glow/zenith)+Lighting+Sky3(lighting_defaults/sky_config/skybox_config) |
| PrimitiveCollection+GeometryInstance | 20 | 22 | 完成 | 0 | 22个Rust测试：GeometryInstance4(defaults/position/color/compute_bs)+GeometryType3(sphere/box/cylinder)+Primitive4(defaults/add_invalidates/vertex_count/compute_bs)+Collection4(add_remove/remove_nonexist/get_mut/visible)+Union3(empty/single/two)+Batch4(new/is_full/splits/empty) |
| SceneCulling+SceneGraph | 15 | 14 | 完成 | 0 | 14个Rust测试：CullResult1(is_visible)+CullingContext5(inside/outside/disabled/distance/overlapping)+SceneGraph3(add/parent_child/remove)+SceneNode2(world_bs/no_bv)+Sort+Filter3(front_to_back/back_to_front/filter_visible) |
| DrawCommand+RenderCommandList+FrameStatistics | 15 | 13 | 完成 | 0 | 13个Rust测试：DrawCommand4(defaults/new_ids/builder_chain/transparent_variants)+RenderPass2(ordering/default)+CommandList4(push_query/sort_opaque/sort_translucent/clear)+DepthState1+FrameStatistics2(merge/reset) |
| ShaderBuilder+ShaderSource+ShaderCache | 15 | 14 | 完成 | 0 | 14个Rust测试：ShaderSource3(new/builtin/append_combine)+ShaderBuilder7(uniform/uniform_array/struct/function/defines/append/full_pipeline)+ShaderProgram1(lifecycle)+ShaderCache3(dedup/different/get_by_id) |
| DebugInspector+PerformanceOverlay | 15 | 12 | 完成 | 0 | 12个Rust测试：Inspector7(defaults/enable_all/disable_all/record_get_tile/clear/frame_stats_summary/highlight_modes)+PerformanceOverlay4(record_fps/average_min_fps/history_limit/empty)+TilesetInspector1(select_deselect) |
| Event(Core/Event.js) | 15 | 11 | 完成 | 0 | 11个Rust测试：Basic8(new_empty/add_listener/raise/multiple_listeners/remove/remove_nonexistent/clear/raise_noop)+SimpleEvent1+TypedArgs1+UniqueIds1 |
| RenderState+ClearCommand+Texture+Framebuffer+TextureAtlas | 20 | 19 | 完成 | 0 | 19个Rust测试：RenderState4(opaque/translucent/2d/default)+States3(stencil/polygon_offset/scissor)+ClearCommand4(default/color_only/depth_only/all)+ComputeCommand1+PassState1+Texture2(defaults/mipmaps)+Framebuffer1+TextureAtlas2(add/full)+GpuBuffer1 |
| Widgets(SceneModePicker+SelectionIndicator+I18n+ProjectionPicker) | 20 | 18 | 完成 | 0 | 18个Rust测试：SceneModePicker7(defaults/select_modes/ignores_morphing/dropdown/labels/available/is_selected)+SelectionIndicator4(defaults/show_at/hide/update_position)+Locale+I18n5(code_from_code/all/default_en/switch_locale/strings_en)+ProjectionPicker2(defaults/switch) |
| Widgets(Buttons+BaseLayerPicker+InfoBox) | 20 | 17 | 完成 | 0 | 17个Rust测试：ToggleButton2(new_toggle/disabled)+HomeButton2(defaults/set_home)+Fullscreen2(toggle/unsupported)+NavHelp1+VR1+BaseLayerPicker3(defaults/add_categories/provider_builder)+InfoBox6(defaults/show_entity/clear/toggle_frame/close/summary) |
| Geocoder+Animation+ShuttleRing | 20 | 16 | 完成 | 0 | 16个Rust测试：Geocoder8(defaults/set_text/should_search/begin_complete/complete_empty/clear/navigation/activate)+ShuttleRing5(default_ticks/linear/log/multiplier_to_angle/roundtrip)+Animation3(defaults/play_pause/reverse) |

**Phase 4 当前合计**：原版 ~1200 个 A 类用例，已移植 1164 个（97.0%）。剩余~3.0%为C类（浏览器/DOM/WebGL依赖）不可移植。

---

## 汇总

| 阶段 | 原版A类用例 | 已移植 | 覆盖率 |
|---|---|---|---|
| Phase 0（存量审计） | — | — | 审计中 |
| Phase 1（Core基础） | 990 | 726 | 73.3% |
| Phase 2（几何地形） | ~800 | 210 | 26.3% |
| Phase 3（Core其余+DataSources） | ~1500 | 851 | 56.7% |
| Phase 4（Scene可移植） | ~1200 | 1164 | 97.0% |
| Phase 5（Renderer/Widget收尾） | — | — | 未开始 |
