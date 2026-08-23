# Spec Coverage Matrix

CesiumJS → Rust 移植的测试覆盖矩阵。

**统计时间**: 2026-08-23
**CesiumJS 原版 spec 文件**: 790 (engine 750 + widgets 40)
**Rust 测试**: 1295 passed, 0 failed, 235 ignored

---

## 总览

| 模块 | CesiumJS Specs | Rust Passed | Rust Ignored | 覆盖率 |
|------|--------------:|-----------:|------------:|-------:|
| Core | 212 | 1173 | 227 | 98/212 文件 |
| DataSources | 92 | 43 | 76 | 27/92 文件 |
| Renderer | 38 | 15 | 0 | 5/38 文件 (33 GPU-required) |
| Scene | 332 | 0 | 0 | 0/332 文件 (全部 GPU-required) |
| Model | 73 | 0 | 0 | 0/73 文件 |
| Workers | — | 8 | 0 | 单元测试 |
| Widgets | 40 | 0 | 0 | 0/40 文件 |
| Shaders | — | 3 | 0 | naga 验证 |
| Test Utils | — | 11 | 2 | 基础设施 |
| Precision | — | 16 | 0 | f64 精度验证 |
| **合计** | **790** | **1295** | **235** | — |

---

## Core (212 CesiumJS specs → 98 Rust test files, 1173 passed + 227 ignored)

### 已覆盖 (98 files)

| Rust Test File | Tests | CesiumJS Counterpart |
|---|---|---|
| `core_cartesian2_spec` | 138 | `Core/Cartesian2Spec.js` |
| `core_cartesian3_spec` | 143 | `Core/Cartesian3Spec.js` |
| `core_cartesian4_spec` | 143 | `Core/Cartesian4Spec.js` |
| `core_math_spec` | 109 | `Core/MathSpec.js` |
| `core_julian_date_spec` | 71 | `Core/JulianDateSpec.js` |
| `core_matrix3_spec` | 45 | `Core/Matrix3Spec.js` |
| `core_quaternion_spec` | 45 | `Core/QuaternionSpec.js` |
| `core_matrix2_spec` | 37 | `Core/Matrix2Spec.js` |
| `core_bounding_rectangle_spec` | 23 | `Core/BoundingRectangleSpec.js` |
| `core_transforms_spec` | 24 | `Core/TransformsSpec.js` |
| `core_bounding_sphere_spec` | 19 | `Core/BoundingSphereSpec.js` |
| `core_event_spec` | 18 | `Core/EventSpec.js` |
| `core_feature_detection_spec` | 17 | `Core/FeatureDetectionSpec.js` |
| `core_check_spec` | 29 | `Core/CheckSpec.js` |
| `core_ellipsoid_spec` | 29 | `Core/EllipsoidSpec.js` |
| `core_plane_spec` | 20 | `Core/PlaneSpec.js` |
| `core_rectangle_spec` | 30 | `Core/RectangleSpec.js` |
| `core_matrix4_spec` | 30 | `Core/Matrix4Spec.js` |
| `core_web_mercator_projection_spec` | 13 | `Core/WebMercatorProjectionSpec.js` |
| `core_vertex_format_spec` | 10 | `Core/VertexFormatSpec.js` |
| `core_ray_spec` | 9 | `Core/RaySpec.js` |
| `core_geographic_projection_spec` | 8 | `Core/GeographicProjectionSpec.js` |
| `core_binary_search_spec` | 8 | (utility) |
| `core_geometry_enums_spec` | 14 | `Core/GeometrySpec.js` 等 |
| `core_spherical_spec` | 12 | `Core/SphericalSpec.js` |
| `core_append_forward_slash_spec` | 3 | (utility) |
| `core_combine_spec` | 3 | (utility) |
| `core_developer_error_spec` | 4 | `Core/DeveloperErrorSpec.js` |
| `core_runtime_error_spec` | 4 | (error handling) |
| `core_get_absolute_uri_spec` | 4 | `Core/getAbsoluteUriSpec.js` |
| `core_is_leap_year_spec` | 4 | (utility) |
| `core_defined_spec` | 3 | (utility) |
| `core_deprecation_warning_spec` | 3 | (logging) |
| `core_one_time_warning_spec` | 2 | (logging) |
| `core_intersect_spec` | 3 | (utility) |
| `core_is_blob_uri_spec` | 3 | (utility) |
| `core_is_data_uri_spec` | 3 | (utility) |
| `core_clone_spec` | 2 | (utility) |
| `core_create_guid_spec` | 1 | (utility) |
| `core_get_base_uri_spec` | 3 | `Core/getBaseUriSpec.js` |
| `core_get_extension_from_uri_spec` | 2 | `Core/getExtensionFromUriSpec.js` |
| `core_get_filename_from_uri_spec` | 2 | (utility) |
| `core_heading_pitch_range_spec` | 4 | `Core/HeadingPitchRangeSpec.js` |
| `core_near_far_scalar_spec` | 5 | `Core/NearFarScalarSpec.js` |
| `core_color_geometry_instance_attribute_spec` | 5 | `Core/ColorGeometryInstanceAttributeSpec.js` |
| `core_rectangle_collision_checker_spec` | 2 | `Core/RectangleCollisionCheckerSpec.js` |
| `core_request_error_event_spec` | 3 | `Core/RequestErrorEventSpec.js` |
| `core_interval_spec` | 2 | `Core/IntervalSpec.js` |
| `core_leap_second_spec` | 2 | `Core/LeapSecondSpec.js` |
| `core_iau2000_orientation_spec` | 1 | `Core/Iau2000OrientationSpec.js` |
| `core_plane_outline_geometry_spec` | 1 | `Core/PlaneOutlineGeometrySpec.js` |
| `core_plane_geometry_spec` | 3 | `Core/PlaneGeometrySpec.js` |
| `core_distance_display_condition_spec` | 7 | `Core/DistanceDisplayConditionSpec.js` |
| `core_distance_display_condition_geometry_instance_attribute_spec` | 2 | `Core/DistanceDisplayConditionGeometryInstanceAttributeSpec.js` |
| `core_constant_spline_spec` | 6 | `Core/ConstantSplineSpec.js` |
| `core_queue_spec` | 8 | `Core/QueueSpec.js` |
| `core_lagrange_polynomial_approximation_spec` | 3 | `Core/LagrangePolynomialApproximationSpec.js` |
| `core_linear_approximation_spec` | 4 | `Core/LinearApproximationSpec.js` |
| `core_quadratic_real_polynomial_spec` | 12 | `Core/QuadraticRealPolynomialSpec.js` |
| `core_hermite_polynomial_approximation_spec` | 3+1ig | `Core/HermitePolynomialApproximationSpec.js` |
| `core_associative_array_spec` | 5 | `Core/AssociativeArraySpec.js` |
| `core_encoded_cartesian3_spec` | 6 | `Core/EncodedCartesian3Spec.js` |
| `core_geometry_attribute_spec` | 1 | `Core/GeometryAttributeSpec.js` |
| `core_geometry_instance_attribute_spec` | 1 | `Core/GeometryInstanceAttributeSpec.js` |
| `core_geometry_instance_spec` | 2 | `Core/GeometryInstanceSpec.js` |
| `core_cubic_real_polynomial_spec` | 7 | `Core/CubicRealPolynomialSpec.js` |
| `core_linear_spline_spec` | 5 | `Core/LinearSplineSpec.js` |
| `core_heap_spec` | 7 | `Core/HeapSpec.js` |
| `core_doubly_linked_list_spec` | 6 | `Core/DoublyLinkedListSpec.js` |
| `core_managed_array_spec` | 10 | `Core/ManagedArraySpec.js` |
| `core_heading_pitch_roll_spec` | 5 | `Core/HeadingPitchRollSpec.js` |
| `core_catmull_rom_spline_spec` | 7 | `Core/CatmullRomSplineSpec.js` |
| `core_quartic_real_polynomial_spec` | 11 | `Core/QuarticRealPolynomialSpec.js` |
| `core_cartographic_spec` | 8 | `Core/CartographicSpec.js` |
| `core_component_datatype_spec` | 5 | `Core/ComponentDatatypeSpec.js` |
| `core_barycentric_coordinates_spec` | 6 | `Core/BarycentricCoordinatesSpec.js` |
| `core_axis_aligned_bounding_box_spec` | 6 | `Core/AxisAlignedBoundingBoxSpec.js` |
| `core_hermite_spline_spec` | 7 | `Core/HermiteSplineSpec.js` |
| `core_tridiagonal_system_solver_spec` | 2 | `Core/TridiagonalSystemSolverSpec.js` |
| `core_easing_function_spec` | 5 | (无 JS spec, 自行编写) |
| `core_color_spec` | 11 | `Core/ColorSpec.js` |
| `core_oriented_bounding_box_spec` | 6 | `Core/OrientedBoundingBoxSpec.js` |
| `core_double_ended_priority_queue_spec` | 8 | `Core/DoubleEndedPriorityQueueSpec.js` |
| `core_attribute_compression_spec` | 3 | `Core/AttributeCompressionSpec.js` |
| `core_ellipsoid_geodesic_spec` | 5 | `Core/EllipsoidGeodesicSpec.js` |
| `core_intersections2d_spec` | 8 | `Core/Intersections2DSpec.js` |
| `core_intersection_tests_spec` | 9 | `Core/IntersectionTestsSpec.js` |
| `core_clock_spec` | 5 | `Core/ClockSpec.js` |
| `core_ellipsoid_rhumb_line_spec` | 9 | `Core/EllipsoidRhumbLineSpec.js` |
| `core_ellipsoid_tangent_plane_spec` | 7 | `Core/EllipsoidTangentPlaneSpec.js` |
| `core_ellipsoidal_occluder_spec` | 6 | `Core/EllipsoidalOccluderSpec.js` |
| `core_culling_volume_spec` | 7 | `Core/CullingVolumeSpec.js` |
| `core_credit_spec` | 9 | `Core/CreditSpec.js` |
| `core_geographic_tiling_scheme_spec` | 8 | `Core/GeographicTilingSchemeSpec.js` |
| `core_tipsify_spec` | 8 | `Core/TipsifySpec.js` |
| `core_wireframe_index_generator_spec` | 11 | `Core/WireframeIndexGeneratorSpec.js` |
| `data` | 1 | (test data helper) |

### 未覆盖 (114 files, 主要类别)

- 几何体 (~59): BoxGeometry, CircleGeometry, CorridorGeometry, CylinderGeometry, EllipseGeometry, EllipsoidGeometry, FrustumGeometry, PolygonGeometry, PolylineGeometry, PolylineVolumeGeometry, SimplePolylineGeometry, WallGeometry 等
- 地形 (~10): CesiumTerrainProvider, ArcGISTiledElevationTerrainProvider, CustomHeightmapTerrainProvider, GoogleEarthEnterpriseTerrainProvider, HeightmapTerrainData 等
- 地理编码 (~8): BingMapsGeocoderService, CartographicGeocoderService, IonGeocoderService, OpenCageGeocoderService, PeliasGeocoderService 等
- 样条/插值 (~2): QuaternionSpline 等 (CatmullRomSpline/HermiteSpline/QuarticRealPolynomial/LinearSpline/ConstantSpline 已覆盖)
- 天体/时间 (~8): EarthOrientationParameters, Iau2006XysData, Simon1994PlanetaryPositions 等
- 资源/网络 (~5): Resource, IonResource, loadAndDeployScriptTag 等
- 图元管线 (~8): GeometryPipeline 等
- 其他 (~20): PinBuilder, Fullscreen 等 (Heap/DoublyLinkedList/ManagedArray/Cartographic/ComponentDatatype/BarycentricCoordinates/AABB/EasingFunction/TridiagonalSolver/Color/OBB/DEPQ/AttributeCompression/EllipsoidGeodesic/Intersections2D/IntersectionTests/Clock/EllipsoidRhumbLine/EllipsoidTangentPlane/EllipsoidalOccluder/CullingVolume/Credit/GeographicTilingScheme/Tipsify/WireframeIndexGenerator 已覆盖)

---

## DataSources (92 CesiumJS specs → 27 active + 76 ignored)

### 已覆盖 (27 active tests)

| 测试 | 状态 |
|---|---|
| Entity 基础构造/属性 | ✅ passed |
| PropertyBag set/get/has/remove/keys | ✅ passed |
| ConstantProperty get_value/equals | ✅ passed |
| CallbackProperty is_constant | ✅ passed |
| CompositeProperty | ✅ passed |
| SampledProperty | ✅ passed |
| EntityCollection add/remove/removeAll/getById | ✅ passed |
| DataSourceCollection add/remove/indexOf | ✅ passed |
| DataSourceDisplay update/get_bounding_sphere | ✅ passed |
| CustomDataSource name/changedEvent | ✅ passed |

### 已占位 (76 ignored tests)

完整覆盖 CzmlDataSource, GeoJsonDataSource, KmlDataSource, GpxDataSource 的 spec 占位。

### 未覆盖

- 完整 CZML 解析 (~20 specs)
- 完整 GeoJSON 解析 (~15 specs)
- 完整 KML 解析 (~15 specs)
- Visualizer 系列 (~15 specs)
- DynamicGeometry 系列 (~10 specs)

---

## Renderer (38 CesiumJS specs → 5 files ported, 15 passed + 33 GPU-required)

### 已移植 (5 files, 15 tests)

| Rust Test File | Tests | CesiumJS Counterpart |
|---|---|---|
| `renderer_shader_destination_spec` | 3 | `Renderer/ShaderDestinationSpec.js` |
| `renderer_pass_state_spec` | 2 | `Renderer/PassStateSpec.js` |
| `renderer_clear_command_spec` | 3 | `Renderer/ClearCommandSpec.js` |
| `renderer_draw_command_spec` | 5 | `Renderer/DrawCommandSpec.js` |
| `renderer_pass_spec` | 2 | `Renderer/PassStateSpec.js` (Pass enum) |

### GPU-required (33 files, 需 wgpu headless)

以下 spec 需要真实 GPU 上下文（wgpu device/queue），标记为 wgpu headless 集成测试候选：

- **Buffer/Texture**: BufferSpec(850L), TextureSpec(1566L), CubeMapSpec(1752L), Texture3DSpec(273L), SamplerSpec(85L), RenderbufferSpec(122L)
- **Framebuffer**: FramebufferSpec(1026L), FramebufferManagerSpec(525L), MultisampleFramebufferSpec(385L)
- **Shader**: ShaderProgramSpec(570L), ShaderSourceSpec(167L), ShaderCacheSpec(375L), ShaderBuilderSpec(860L), demodernizeShaderSpec(146L)
- **Render**: ContextSpec(413L), RenderStateSpec(955L), freezeRenderStateSpec(20L), DrawSpec(1466L), ClearSpec(136L)
- **Command**: ComputeCommandSpec(162L), SyncSpec(156L), SharedContextSpec(177L)
- **VertexArray**: VertexArraySpec(912L), VertexArrayFacadeSpec(440L), VertexArrayFactorySpec(736L)
- **Texture ops**: TextureAtlasSpec(1548L), TextureCacheSpec(125L), loadCubeMapSpec(296L)
- **Uniform**: UniformSpec(848L), AutomaticUniformSpec(2441L), BuiltinFunctionsSpec(529L)
- **Other**: PassStateSpec.js (部分), PickIdSpec(55L), ShaderFunctionSpec(51L), ShaderStructSpec(46L)

---

## Scene (332 CesiumJS specs → 0 tests, 全部 GPU-required)

所有 Scene spec 均依赖 `createScene()` (WebGL context)，无法无 GPU 移植。
按 wgpu headless 集成优先级分类：

### P0: 瓦片管线核心 (历史 bug 高发, 优先集成)

| CesiumJS Spec | Lines | 对应 Rust 模块 |
|---|---|---|
| GlobeSurfaceTileProviderSpec | 1641 | `globe_surface_tile_provider.rs` |
| GlobeSurfaceTileSpec | 392 | `globe_surface_tile_provider.rs` |
| GlobeSpec | 498 | `globe.rs` |
| QuadtreePrimitiveSpec | — | `quadtree_primitive.rs` |
| ImageryLayerSpec | 821 | `imagery_layer.rs` |
| ImageryLayerCollectionSpec | 757 | `imagery_layer_collection.rs` |

### P1: Camera/Scene 主干

| CesiumJS Spec | Lines | 对应 Rust 模块 |
|---|---|---|
| CameraSpec | 4496 | `camera.rs` |
| CameraFlightPathSpec | 721 | `camera_flight_path.rs` |
| CameraEventAggregatorSpec | 321 | `camera_event_aggregator.rs` |
| SceneSpec | — | `scene.rs` |
| SceneModeSpec | — | `scene_mode.rs` |
| SceneTransformsSpec | — | `scene_transforms.rs` |
| CreditDisplaySpec | 616 | `credit_display.rs` |

### P2: Primitive/Appearance/Material

| CesiumJS Spec | Lines | 对应 Rust 模块 |
|---|---|---|
| PrimitiveSpec | — | `primitive.rs` |
| AppearanceSpec | 123 | `appearance.rs` |
| MaterialSpec | 1126 | `material.rs` |
| BillboardCollectionSpec | 2809 | `billboard_collection.rs` |
| LabelCollectionSpec | 2545 | `label_collection.rs` |
| PointPrimitiveCollectionSpec | — | `point_primitive_collection.rs` |
| GroundPrimitiveSpec | 1561 | `ground_primitive.rs` |
| ClassificationPrimitiveSpec | 1186 | `classification_primitive.rs` |

### P3: 3D Tiles / Model / glTF

| CesiumJS Spec | Lines | 对应 Rust 模块 |
|---|---|---|
| Cesium3DTilesetSpec | 6845 | `cesium3_d_tileset.rs` |
| Cesium3DTileSpec | 886 | `cesium3_d_tile.rs` |
| GltfLoaderSpec | 4810 | `gltf_loader.rs` |
| ModelSpec | — | `model.rs` |
| Cesium3DTileStyleSpec | 3778 | `cesium3_d_tile_style.rs` |
| ExpressionSpec | 4235 | `expression.rs` |

### P4: Sky/Shadow/PostProcess/Particles

| CesiumJS Spec | Lines | 对应 Rust 模块 |
|---|---|---|
| SkyBoxSpec | — | `sky_box.rs` |
| SkyAtmosphereSpec | — | `sky_atmosphere.rs` |
| SunSpec | — | `sun.rs` |
| MoonSpec | — | `moon.rs` |
| ShadowMapSpec | — | `shadow_map.rs` |
| PostProcessStageSpec | — | `post_process_stage.rs` |
| ParticleSystemSpec | — | `particle_system.rs` |

### P5: 其他 (~150 specs)

- ImageryProvider 系列 (~15): BingMaps, ArcGIS, Mapbox, Ion, Google, Azure 等
- Metadata 系列 (~20): MetadataClass, MetadataEntity, ImplicitSubtree 等
- Light/Environment (~10): DirectionalLight, PointLight, ImageBasedLighting 等
- Geometry 系列 (~10): HeightmapTessellator, GeometryRendering 等
- 其他: Fog, Pick, JobScheduler, FrameRateMonitor 等

---

## Model (73 CesiumJS specs → 0 tests)

### 未覆盖

- ModelSpec, ModelAnimationSpec, ModelAnimationCollectionSpec
- GltfLoaderSpec, GltfPipelineSpec
- ModelMaterialsSpec, ModelMeshSpec, ModelNodeSpec
- ModelInstanceSpec, ModelOutlineSpec
- 等 73 个 spec 文件

---

## Widgets (40 CesiumJS specs → 0 tests)

### 未覆盖

- Viewer/ViewerSpec.js (925 lines)
- Viewer/viewerDragDropMixinSpec.js (475 lines)
- Animation/AnimationViewModelSpec.js (672 lines)
- BaseLayerPicker/BaseLayerPickerViewModelSpec.js (374 lines)
- Geocoder/GeocoderViewModelSpec.js (392 lines)
- Cesium3DTilesInspector/Cesium3DTilesInspectorViewModelSpec.js (351 lines)
- CesiumInspector/CesiumInspectorViewModelSpec.js (317 lines)
- 等 40 个 spec 文件 (主要依赖 DOM/Knockout，需要 DomSurface trait 集成)

---

## Workers (8 tests, 单元测试)

| 测试 | 状态 |
|---|---|
| TaskProcessor 构造/销毁 | ✅ passed |
| createGeometry 分发 | ✅ passed |
| 各 worker 纯函数 | ✅ passed (6) |

---

## Crate 分布

| Crate | Passed | Ignored | 说明 |
|---|---:|---:|---|
| cesium-specs | 997 | 226 | Core + Renderer specs 集成测试 |
| cesium-data-sources | 43 | 76 | DataSource specs + 集成测试 |
| cesium-core | 12 | 6 | 单元测试 |
| cesium-test-utils | 11 | 2 | 测试基础设施 |
| cesium-workers | 8 | 0 | Worker 单元测试 |
| cesium-shaders | 3 | 0 | naga 着色器验证 |
| cesium-renderer | 0 | 0 | 33 specs 待 wgpu headless |
| cesium-scene | 0 | 0 | 待 wgpu headless |
| cesium-widgets | 0 | 0 | 待 DomSurface 集成 |

---

## 下一步优先级

1. **Renderer GPU specs** (33 specs): wgpu headless 集成测试环境搭建
2. **Scene specs** (257 specs): 依赖 Renderer + Scene 集成
3. **Model specs** (73 specs): 依赖 Renderer + glTF 解析
4. **Widgets specs** (40 specs): 依赖 DomSurface trait + winit 集成
5. **Core 剩余** (130 specs): 几何体、地形、样条等纯数学模块
