# Spec Coverage Matrix

CesiumJS → Rust 移植的测试覆盖矩阵。

**统计时间**: 2026-08-24（A10 收尾实测）；总量经任务 #31（R12）复核刷新
**CesiumJS 原版 spec 文件**: 790 (engine 750 + widgets 40)
**Rust 测试**: 3187 passed, 0 failed, 330 ignored（修复任务 #33–#37 完成后 workspace 全量实测；#40–#42 在途。2026-08-24 快照为 2844 passed / 326 ignored，EXIT=0）
**本次变更**: Specs/Data 空壳目录删除（引用指向父仓库 `../../Specs/Data`）+ viewer-demo 3D 模型集成

---

## 总览

| 模块 | CesiumJS Specs | Rust Passed | Rust Ignored | 覆盖率 |
|------|--------------:|-----------:|------------:|-------:|
| Core | 212 | 2101 | 221 | 98/212 文件 + 保真度/几何批次 |
| DataSources | 92 | 256 | 38 | Entity/Property/CZML/GeoJSON |
| Renderer | 38 | 23 | 0 | 5/38 文件 (33 GPU-required) + lib 8 |
| Scene | 332 | 228 | 0 | camera/quadtree/3D Tiles/表达式/glTF 批次 |
| Model | 73 | 0 | 0 | 0/73 文件（glTF 批次已含部分） |
| Workers | — | 18 | 0 | 单元 + task_processor 集成 |
| Widgets | 40 | 133 | 54 | ViewModel 镜像 spec 已落地（f9 实测，DomSurface 待集成） |
| Shaders | — | 17 | 0 | naga/WGSL 验证 |
| Test Utils | — | 10 | 0 | 基础设施 |
| Precision | — | 16 | 0 | f64 精度验证 |
| Smoke (ViewportQuad) | — | 4 | 0 | Track B 冒烟（scene 2 + specs 2） |
| Doc-tests | — | 4 | 8 | core 3+6ig / specs 1 / test-utils 1+2ig |
| **合计** | **790** | **3187** | **330** | — |

> 注：逐文件明细表的 Tests 数为 M1-W2 快照，总量以运行时实测为准（Core/Scene/DataSources 后续批次持续新增用例）。
> 总量基线（任务 #31 复核）：3187 passed / 330 ignored = specs 221 ig + widgets 54 ig + data-sources 42 ig + scene 5 ig + doc-tests 8 ig（#40–#42 在途，data-sources 行持续变动）。

---

## Core (212 CesiumJS specs → 98 Rust test files, 2101 passed + 221 ignored)

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

## DataSources (92 CesiumJS specs → 256 passed + 38 ignored)

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

## Widgets (40 CesiumJS specs → 133 tests，任务 #31 复核刷新)

> 复核注记（任务 #31 / R12）：A10 批（任务 #15/#17/#18）已移植 ViewModel 镜像 spec，实测 133 passed / 54 ignored（见 ignored_disposition.md widgets 节）；以下清单为该快照时点的未覆盖口径，DOM 面仍属 E 档待 DomSurface 集成。

### 未覆盖（2026-08-24 快照口径）

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

## Crate 分布（2026-08-24 `cargo test --workspace` 全绿实测，EXIT=0）

| Crate / 测试目标 | Passed | Ignored | 说明 |
|---|---:|---:|---|
| cesium-specs `core.rs` | 1771 | 219 | Core specs 镜像（回填后） |
| cesium-specs `core_fidelity_batch.rs` | 266 | 2 | Core 保真度批次（Track A） |
| cesium-specs `core_geometry_batch.rs` | 64 | 0 | Core 几何批次（Track A2/A3） |
| cesium-specs `renderer.rs` | 15 | 0 | Renderer 非 GPU specs |
| cesium-specs `precision_verification.rs` | 16 | 0 | f64 精度验证 |
| cesium-specs `smoke.rs` + lib | 3 | 0 | ViewportQuad 冒烟 + lib helper |
| cesium-scene tests（camera/quadtree/3D Tiles/表达式/glTF/smoke） | 228 | 0 | Track B4/A9 批次（12+9+26+165+14+2） |
| cesium-scene lib | 0 | 0 | 待 wgpu headless |
| cesium-data-sources（czml/geo_json/display/specs） | 256 | 38 | 16+65+62+86+27（CZML/GeoJSON/Display 已实质化） |
| cesium-core (lib) | 15 | 0 | 单元测试 |
| cesium-core Doc-tests | 3 | 6 | 文档示例 |
| cesium-renderer (lib) | 8 | 0 | wgpu 适配单元测试 |
| cesium-test-utils | 45 | 3 | 测试基础设施（lib + doc） |
| cesium-workers | 18 | 0 | mock_workers 8 + task_processor 10 |
| cesium-shaders | 17 | 0 | naga/WGSL 验证（14 lib + 2 + 1） |
| cesium-specs Doc-tests | 1 | 0 | |
| cesium-widgets | 133 | 54 | ViewModel 镜像 spec（f9 实测；任务 #31 复核刷新） |
| **合计** | **3187** | **330** | 0 failed（任务 #31 复核基线；#40–#42 在途，data-sources/scene 行持续变动） |

> 复核注记（任务 #31 / R12）：本表各行为 2026-08-24 快照口径，合计行与 cesium-widgets 行已按最新全量结果（3187 passed / 330 ignored，#33–#37 完成后）刷新；ignored 构成见总览注记。

---

## 下一步优先级

1. **Renderer GPU specs** (33 specs): wgpu headless 集成测试环境搭建
2. **Scene specs** (257 specs): 依赖 Renderer + Scene 集成
3. **Model specs** (73 specs): 依赖 Renderer + glTF 解析
4. **Widgets specs** (40 specs): 依赖 DomSurface trait + winit 集成
5. **Core 剩余** (130 specs): 几何体、地形、样条等纯数学模块
