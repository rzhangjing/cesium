# 移植台账（Porting Ledger）

CesiumJS → cesium-rs 文件级移植状态总台账。

- **状态取值**：`not_started`（未开始）/ `ported`（已移植，待测）/ `tested`（已移植且镜像 spec 通过）。
- **更新规则**：状态只在对应 PR 合入时更新；`ported → tested` 需要镜像
  spec 全绿。规约见 [PORTING_CONVENTIONS.md](PORTING_CONVENTIONS.md)。

图例/约定：

- Rust 模块列填写移植后的完整模块路径（移植前可留 `—`）。
- Core 清单为 `packages/engine/Source/Core` 全部 294 个文件的完整镜像。

## Core（packages/engine/Source/Core → cesium-core）

<!-- 示例行 -->

| JS 文件 | Rust 模块 | 状态 | 备注 |
| --- | --- | --- | --- |
| `Core/Cartesian3.js` | `cesium_core::cartesian3` | not_started | 数学基础，首批移植候选 |

<!-- BEGIN CORE TABLE（由 M0 骨架生成脚本写入，状态全为 not_started） -->
| JS 文件 | Rust 模块 | 状态 | 备注 |
| --- | --- | --- | --- |
| `Core/addAllToArray.js` | `cesium_core::addAllToArray` | not_started |  |
| `Core/appendForwardSlash.js` | `cesium_core::appendForwardSlash` | tested | spec mirrored |
| `Core/ApproximateTerrainHeights.js` | `cesium_core::approximateTerrainHeights` | not_started |  |
| `Core/ArcGISTiledElevationTerrainProvider.js` | `cesium_core::arcGISTiledElevationTerrainProvider` | not_started |  |
| `Core/ArcType.js` | `cesium_core::arcType` | not_started |  |
| `Core/arrayRemoveDuplicates.js` | `cesium_core::arrayRemoveDuplicates` | not_started |  |
| `Core/ArticulationStageType.js` | `cesium_core::articulationStageType` | not_started |  |
| `Core/assert.js` | `cesium_core::assert` | ported | no upstream spec |
| `Core/AssociativeArray.js` | `cesium_core::associativeArray` | not_started |  |
| `Core/AttributeCompression.js` | `cesium_core::attributeCompression` | not_started |  |
| `Core/AxisAlignedBoundingBox.js` | `cesium_core::axisAlignedBoundingBox` | not_started |  |
| `Core/barycentricCoordinates.js` | `cesium_core::barycentricCoordinates` | not_started |  |
| `Core/binarySearch.js` | `cesium_core::binarySearch` | tested | spec mirrored |
| `Core/BingMapsGeocoderService.js` | `cesium_core::bingMapsGeocoderService` | not_started |  |
| `Core/BoundingRectangle.js` | `cesium_core::bounding_rectangle` | tested | spec mirrored |
| `Core/BoundingSphere.js` | `cesium_core::bounding_sphere` | tested | spec mirrored |
| `Core/BoxGeometry.js` | `cesium_core::boxGeometry` | not_started |  |
| `Core/BoxOutlineGeometry.js` | `cesium_core::boxOutlineGeometry` | not_started |  |
| `Core/buildModuleUrl.js` | `cesium_core::buildModuleUrl` | not_started |  |
| `Core/Cartesian2.js` | `cesium_core::cartesian2` | tested | spec mirrored |
| `Core/Cartesian3.js` | `cesium_core::cartesian3` | tested | spec mirrored |
| `Core/Cartesian4.js` | `cesium_core::cartesian4` | tested | spec mirrored (fromColor deferred → Color) |
| `Core/Cartographic.js` | `cesium_core::cartographic` | ported | used by projections |
| `Core/CartographicGeocoderService.js` | `cesium_core::cartographicGeocoderService` | not_started |  |
| `Core/CatmullRomSpline.js` | `cesium_core::catmullRomSpline` | not_started |  |
| `Core/Cesium3DTilesTerrainData.js` | `cesium_core::cesium3DTilesTerrainData` | not_started |  |
| `Core/Cesium3DTilesTerrainGeometryProcessor.js` | `cesium_core::cesium3DTilesTerrainGeometryProcessor` | not_started |  |
| `Core/Cesium3DTilesTerrainProvider.js` | `cesium_core::cesium3DTilesTerrainProvider` | not_started |  |
| `Core/CesiumTerrainProvider.js` | `cesium_core::cesiumTerrainProvider` | not_started |  |
| `Core/Check.d.ts` | `cesium_core::check.d` | not_started |  |
| `Core/Check.js` | `cesium_core::check` | tested | spec mirrored |
| `Core/CircleGeometry.js` | `cesium_core::circleGeometry` | not_started |  |
| `Core/CircleOutlineGeometry.js` | `cesium_core::circleOutlineGeometry` | not_started |  |
| `Core/Clock.js` | `cesium_core::clock` | not_started |  |
| `Core/ClockRange.js` | `cesium_core::clockRange` | not_started |  |
| `Core/ClockStep.js` | `cesium_core::clockStep` | not_started |  |
| `Core/clone.js` | `cesium_core::clone` | tested | spec mirrored |
| `Core/Color.js` | `cesium_core::color` | not_started |  |
| `Core/ColorGeometryInstanceAttribute.js` | `cesium_core::colorGeometryInstanceAttribute` | not_started |  |
| `Core/combine.js` | `cesium_core::combine` | tested | spec mirrored |
| `Core/ComponentDatatype.js` | `cesium_core::componentDatatype` | not_started |  |
| `Core/CompressedTextureBuffer.js` | `cesium_core::compressedTextureBuffer` | not_started |  |
| `Core/ConstantSpline.js` | `cesium_core::constantSpline` | not_started |  |
| `Core/CoplanarPolygonGeometry.js` | `cesium_core::coplanarPolygonGeometry` | not_started |  |
| `Core/CoplanarPolygonGeometryLibrary.js` | `cesium_core::coplanarPolygonGeometryLibrary` | not_started |  |
| `Core/CoplanarPolygonOutlineGeometry.js` | `cesium_core::coplanarPolygonOutlineGeometry` | not_started |  |
| `Core/CornerType.js` | `cesium_core::cornerType` | not_started |  |
| `Core/CorridorGeometry.js` | `cesium_core::corridorGeometry` | not_started |  |
| `Core/CorridorGeometryLibrary.js` | `cesium_core::corridorGeometryLibrary` | not_started |  |
| `Core/CorridorOutlineGeometry.js` | `cesium_core::corridorOutlineGeometry` | not_started |  |
| `Core/createColorRamp.js` | `cesium_core::createColorRamp` | not_started |  |
| `Core/createGuid.js` | `cesium_core::createGuid` | tested | spec mirrored |
| `Core/createWorldBathymetryAsync.js` | `cesium_core::createWorldBathymetryAsync` | not_started |  |
| `Core/createWorldTerrainAsync.js` | `cesium_core::createWorldTerrainAsync` | not_started |  |
| `Core/Credit.js` | `cesium_core::credit` | not_started |  |
| `Core/CubicRealPolynomial.js` | `cesium_core::cubicRealPolynomial` | not_started |  |
| `Core/CullingVolume.js` | `cesium_core::cullingVolume` | not_started |  |
| `Core/CustomHeightmapTerrainProvider.js` | `cesium_core::customHeightmapTerrainProvider` | not_started |  |
| `Core/CylinderGeometry.js` | `cesium_core::cylinderGeometry` | not_started |  |
| `Core/CylinderGeometryLibrary.js` | `cesium_core::cylinderGeometryLibrary` | not_started |  |
| `Core/CylinderOutlineGeometry.js` | `cesium_core::cylinderOutlineGeometry` | not_started |  |
| `Core/decodeGoogleEarthEnterpriseData.js` | `cesium_core::decodeGoogleEarthEnterpriseData` | not_started |  |
| `Core/decodeVectorPolylinePositions.js` | `cesium_core::decodeVectorPolylinePositions` | not_started |  |
| `Core/DefaultProxy.js` | `cesium_core::defaultProxy` | not_started |  |
| `Core/defer.js` | `cesium_core::defer` | not_started |  |
| `Core/defined.d.ts` | `cesium_core::defined.d` | not_started |  |
| `Core/defined.js` | `cesium_core::defined` | tested | spec mirrored |
| `Core/deprecationWarning.js` | `cesium_core::deprecationWarning` | tested | spec mirrored |
| `Core/destroyObject.js` | `cesium_core::destroyObject` | ported | no upstream spec |
| `Core/DeveloperError.js` | `cesium_core::developerError` | tested | spec mirrored |
| `Core/DistanceDisplayCondition.js` | `cesium_core::distanceDisplayCondition` | not_started |  |
| `Core/DistanceDisplayConditionGeometryInstanceAttribute.js` | `cesium_core::distanceDisplayConditionGeometryInstanceAttribute` | not_started |  |
| `Core/DoubleEndedPriorityQueue.js` | `cesium_core::doubleEndedPriorityQueue` | not_started |  |
| `Core/DoublyLinkedList.js` | `cesium_core::doublyLinkedList` | not_started |  |
| `Core/EarthOrientationParameters.js` | `cesium_core::earthOrientationParameters` | not_started |  |
| `Core/EarthOrientationParametersSample.js` | `cesium_core::earthOrientationParametersSample` | not_started |  |
| `Core/EasingFunction.js` | `cesium_core::easingFunction` | not_started |  |
| `Core/EllipseGeometry.js` | `cesium_core::ellipseGeometry` | not_started |  |
| `Core/EllipseGeometryLibrary.js` | `cesium_core::ellipseGeometryLibrary` | not_started |  |
| `Core/EllipseOutlineGeometry.js` | `cesium_core::ellipseOutlineGeometry` | not_started |  |
| `Core/Ellipsoid.js` | `cesium_core::ellipsoid` | tested | spec mirrored |
| `Core/EllipsoidalOccluder.js` | `cesium_core::ellipsoidalOccluder` | not_started |  |
| `Core/EllipsoidGeodesic.js` | `cesium_core::ellipsoidGeodesic` | not_started |  |
| `Core/EllipsoidGeometry.js` | `cesium_core::ellipsoidGeometry` | not_started |  |
| `Core/EllipsoidOutlineGeometry.js` | `cesium_core::ellipsoidOutlineGeometry` | not_started |  |
| `Core/EllipsoidRhumbLine.js` | `cesium_core::ellipsoidRhumbLine` | not_started |  |
| `Core/EllipsoidTangentPlane.js` | `cesium_core::ellipsoidTangentPlane` | not_started |  |
| `Core/EllipsoidTerrainProvider.js` | `cesium_core::ellipsoidTerrainProvider` | not_started |  |
| `Core/EncodedCartesian3.js` | `cesium_core::encodedCartesian3` | not_started |  |
| `Core/Event.js` | `cesium_core::event` | tested | spec mirrored |
| `Core/EventHelper.js` | `cesium_core::eventHelper` | not_started |  |
| `Core/ExtrapolationType.js` | `cesium_core::extrapolationType` | not_started |  |
| `Core/FeatureDetection.js` | `cesium_core::featureDetection` | tested | spec mirrored |
| `Core/formatError.js` | `cesium_core::formatError` | ported | no upstream spec |
| `Core/Frozen.js` | `cesium_core::frozen` | ported | no upstream spec |
| `Core/FrustumGeometry.js` | `cesium_core::frustumGeometry` | not_started |  |
| `Core/FrustumOutlineGeometry.js` | `cesium_core::frustumOutlineGeometry` | not_started |  |
| `Core/Fullscreen.js` | `cesium_core::fullscreen` | not_started |  |
| `Core/GeocoderService.js` | `cesium_core::geocoderService` | not_started |  |
| `Core/GeocodeType.js` | `cesium_core::geocodeType` | not_started |  |
| `Core/GeographicProjection.js` | `cesium_core::geographic_projection` | tested | spec mirrored |
| `Core/GeographicTilingScheme.js` | `cesium_core::geographicTilingScheme` | not_started |  |
| `Core/Geometry.js` | `cesium_core::geometry` | not_started |  |
| `Core/GeometryAttribute.js` | `cesium_core::geometryAttribute` | not_started |  |
| `Core/GeometryAttributes.js` | `cesium_core::geometryAttributes` | not_started |  |
| `Core/GeometryFactory.js` | `cesium_core::geometryFactory` | not_started |  |
| `Core/GeometryInstance.js` | `cesium_core::geometryInstance` | not_started |  |
| `Core/GeometryInstanceAttribute.js` | `cesium_core::geometryInstanceAttribute` | not_started |  |
| `Core/GeometryOffsetAttribute.js` | `cesium_core::geometryOffsetAttribute` | not_started |  |
| `Core/GeometryPipeline.js` | `cesium_core::geometryPipeline` | not_started |  |
| `Core/GeometryType.js` | `cesium_core::geometryType` | not_started |  |
| `Core/getAbsoluteUri.js` | `cesium_core::getAbsoluteUri` | tested | spec mirrored |
| `Core/getBaseUri.js` | `cesium_core::getBaseUri` | tested | spec mirrored |
| `Core/getExtensionFromUri.js` | `cesium_core::getExtensionFromUri` | tested | spec mirrored |
| `Core/getFilenameFromUri.js` | `cesium_core::getFilenameFromUri` | tested | spec mirrored |
| `Core/getImageFromTypedArray.js` | `cesium_core::getImageFromTypedArray` | not_started |  |
| `Core/getImagePixels.js` | `cesium_core::getImagePixels` | ported | no upstream spec; DEVIATION canvas |
| `Core/getJsonFromTypedArray.js` | `cesium_core::getJsonFromTypedArray` | not_started |  |
| `Core/getMagic.js` | `cesium_core::getMagic` | not_started |  |
| `Core/getStringFromTypedArray.js` | `cesium_core::getStringFromTypedArray` | not_started |  |
| `Core/getTimestamp.js` | `cesium_core::getTimestamp` | ported | no upstream spec |
| `Core/globalTypes.js` | `cesium_core::globalTypes` | not_started |  |
| `Core/GoogleEarthEnterpriseMetadata.js` | `cesium_core::googleEarthEnterpriseMetadata` | not_started |  |
| `Core/GoogleEarthEnterpriseTerrainData.js` | `cesium_core::googleEarthEnterpriseTerrainData` | not_started |  |
| `Core/GoogleEarthEnterpriseTerrainProvider.js` | `cesium_core::googleEarthEnterpriseTerrainProvider` | not_started |  |
| `Core/GoogleEarthEnterpriseTileInformation.js` | `cesium_core::googleEarthEnterpriseTileInformation` | not_started |  |
| `Core/GoogleGeocoderService.js` | `cesium_core::googleGeocoderService` | not_started |  |
| `Core/GoogleMaps.js` | `cesium_core::googleMaps` | not_started |  |
| `Core/GregorianDate.js` | `cesium_core::gregorian_date` | tested | part of JulianDate system |
| `Core/GroundPolylineGeometry.js` | `cesium_core::groundPolylineGeometry` | not_started |  |
| `Core/HeadingPitchRange.js` | `cesium_core::headingPitchRange` | not_started |  |
| `Core/HeadingPitchRoll.js` | `cesium_core::heading_pitch_roll` | ported | no upstream spec |
| `Core/Heap.js` | `cesium_core::heap` | not_started |  |
| `Core/HeightmapEncoding.js` | `cesium_core::heightmapEncoding` | not_started |  |
| `Core/HeightmapTerrainData.js` | `cesium_core::heightmapTerrainData` | not_started |  |
| `Core/HeightmapTessellator.js` | `cesium_core::heightmapTessellator` | not_started |  |
| `Core/HermitePolynomialApproximation.js` | `cesium_core::hermitePolynomialApproximation` | not_started |  |
| `Core/HermiteSpline.js` | `cesium_core::hermiteSpline` | not_started |  |
| `Core/HilbertOrder.js` | `cesium_core::hilbertOrder` | not_started |  |
| `Core/Iau2000Orientation.js` | `cesium_core::iau2000Orientation` | not_started |  |
| `Core/Iau2006XysData.js` | `cesium_core::iau2006XysData` | not_started |  |
| `Core/Iau2006XysSample.js` | `cesium_core::iau2006XysSample` | not_started |  |
| `Core/IauOrientationAxes.js` | `cesium_core::iauOrientationAxes` | not_started |  |
| `Core/IauOrientationParameters.js` | `cesium_core::iauOrientationParameters` | not_started |  |
| `Core/IndexDatatype.js` | `cesium_core::indexDatatype` | not_started |  |
| `Core/InterpolationAlgorithm.js` | `cesium_core::interpolationAlgorithm` | not_started |  |
| `Core/InterpolationType.js` | `cesium_core::interpolationType` | not_started |  |
| `Core/Intersect.js` | `cesium_core::intersect` | tested | spec mirrored |
| `Core/Intersections2D.js` | `cesium_core::intersections2D` | not_started |  |
| `Core/IntersectionTests.js` | `cesium_core::intersectionTests` | not_started |  |
| `Core/Interval.js` | `cesium_core::interval` | not_started |  |
| `Core/Ion.js` | `cesium_core::ion` | not_started |  |
| `Core/IonGeocodeProviderType.js` | `cesium_core::ionGeocodeProviderType` | not_started |  |
| `Core/IonGeocoderService.js` | `cesium_core::ionGeocoderService` | not_started |  |
| `Core/IonResource.js` | `cesium_core::ionResource` | not_started |  |
| `Core/isBitSet.js` | `cesium_core::isBitSet` | ported | no upstream spec |
| `Core/isBlobUri.js` | `cesium_core::isBlobUri` | tested | spec mirrored |
| `Core/isCrossOriginUrl.js` | `cesium_core::isCrossOriginUrl` | not_started |  |
| `Core/isDataUri.js` | `cesium_core::isDataUri` | tested | spec mirrored |
| `Core/isLeapYear.js` | `cesium_core::isLeapYear` | tested | spec mirrored |
| `Core/Iso8601.js` | `cesium_core::iso8601` | not_started |  |
| `Core/ITwinPlatform.js` | `cesium_core::iTwinPlatform` | not_started |  |
| `Core/JulianDate.js` | `cesium_core::julian_date` | tested | spec mirrored |
| `Core/KeyboardEventModifier.js` | `cesium_core::keyboardEventModifier` | not_started |  |
| `Core/KTX2Transcoder.js` | `cesium_core::kTX2Transcoder` | not_started |  |
| `Core/LagrangePolynomialApproximation.js` | `cesium_core::lagrangePolynomialApproximation` | not_started |  |
| `Core/LeapSecond.js` | `cesium_core::leap_second` | tested | part of JulianDate system |
| `Core/LinearApproximation.js` | `cesium_core::linearApproximation` | not_started |  |
| `Core/LinearSpline.js` | `cesium_core::linearSpline` | not_started |  |
| `Core/loadAndExecuteScript.js` | `cesium_core::loadAndExecuteScript` | ported | DEVIATION DOM |
| `Core/loadImageFromTypedArray.js` | `cesium_core::loadImageFromTypedArray` | not_started |  |
| `Core/loadKTX2.js` | `cesium_core::loadKTX2` | not_started |  |
| `Core/ManagedArray.js` | `cesium_core::managedArray` | not_started |  |
| `Core/MapProjection.js` | `cesium_core::mapProjection` | not_started |  |
| `Core/Math.js` | `cesium_core::math` | tested | spec mirrored |
| `Core/Matrix2.js` | `cesium_core::matrix2` | tested | spec mirrored |
| `Core/Matrix3.js` | `cesium_core::matrix3` | tested | spec mirrored |
| `Core/Matrix4.js` | `cesium_core::matrix4` | tested | spec mirrored |
| `Core/mergeSort.js` | `cesium_core::mergeSort` | not_started |  |
| `Core/MorphWeightSpline.js` | `cesium_core::morphWeightSpline` | not_started |  |
| `Core/MortonOrder.js` | `cesium_core::mortonOrder` | not_started |  |
| `Core/NearFarScalar.js` | `cesium_core::nearFarScalar` | not_started |  |
| `Core/objectToQuery.js` | `cesium_core::objectToQuery` | not_started |  |
| `Core/Occluder.js` | `cesium_core::occluder` | not_started |  |
| `Core/OffsetGeometryInstanceAttribute.js` | `cesium_core::offsetGeometryInstanceAttribute` | not_started |  |
| `Core/oneTimeWarning.js` | `cesium_core::oneTimeWarning` | tested | spec mirrored |
| `Core/OpenCageGeocoderService.js` | `cesium_core::openCageGeocoderService` | not_started |  |
| `Core/OrientedBoundingBox.js` | `cesium_core::orientedBoundingBox` | not_started |  |
| `Core/OrthographicFrustum.js` | `cesium_core::orthographicFrustum` | not_started |  |
| `Core/OrthographicOffCenterFrustum.js` | `cesium_core::orthographicOffCenterFrustum` | not_started |  |
| `Core/Packable.js` | `cesium_core::packable` | not_started |  |
| `Core/PackableForInterpolation.js` | `cesium_core::packableForInterpolation` | not_started |  |
| `Core/parseResponseHeaders.js` | `cesium_core::parseResponseHeaders` | not_started |  |
| `Core/PeliasGeocoderService.js` | `cesium_core::peliasGeocoderService` | not_started |  |
| `Core/PerspectiveFrustum.js` | `cesium_core::perspectiveFrustum` | not_started |  |
| `Core/PerspectiveOffCenterFrustum.js` | `cesium_core::perspectiveOffCenterFrustum` | not_started |  |
| `Core/PinBuilder.js` | `cesium_core::pinBuilder` | not_started |  |
| `Core/PixelFormat.js` | `cesium_core::pixelFormat` | not_started |  |
| `Core/Plane.js` | `cesium_core::plane` | tested | spec mirrored |
| `Core/PlaneGeometry.js` | `cesium_core::planeGeometry` | not_started |  |
| `Core/PlaneOutlineGeometry.js` | `cesium_core::planeOutlineGeometry` | not_started |  |
| `Core/pointInsideTriangle.js` | `cesium_core::pointInsideTriangle` | not_started |  |
| `Core/PolygonGeometry.js` | `cesium_core::polygonGeometry` | not_started |  |
| `Core/PolygonGeometryLibrary.js` | `cesium_core::polygonGeometryLibrary` | not_started |  |
| `Core/PolygonHierarchy.js` | `cesium_core::polygonHierarchy` | not_started |  |
| `Core/PolygonOutlineGeometry.js` | `cesium_core::polygonOutlineGeometry` | not_started |  |
| `Core/PolygonPipeline.js` | `cesium_core::polygonPipeline` | not_started |  |
| `Core/PolylineGeometry.js` | `cesium_core::polylineGeometry` | not_started |  |
| `Core/PolylinePipeline.js` | `cesium_core::polylinePipeline` | not_started |  |
| `Core/PolylineVolumeGeometry.js` | `cesium_core::polylineVolumeGeometry` | not_started |  |
| `Core/PolylineVolumeGeometryLibrary.js` | `cesium_core::polylineVolumeGeometryLibrary` | not_started |  |
| `Core/PolylineVolumeOutlineGeometry.js` | `cesium_core::polylineVolumeOutlineGeometry` | not_started |  |
| `Core/PrimitiveType.js` | `cesium_core::primitiveType` | not_started |  |
| `Core/Proxy.js` | `cesium_core::proxy` | not_started |  |
| `Core/QuadraticRealPolynomial.js` | `cesium_core::quadraticRealPolynomial` | not_started |  |
| `Core/QuantizedMeshTerrainData.js` | `cesium_core::quantizedMeshTerrainData` | not_started |  |
| `Core/QuarticRealPolynomial.js` | `cesium_core::quarticRealPolynomial` | not_started |  |
| `Core/Quaternion.js` | `cesium_core::quaternion` | tested | spec mirrored |
| `Core/QuaternionSpline.js` | `cesium_core::quaternionSpline` | not_started |  |
| `Core/queryToObject.js` | `cesium_core::queryToObject` | not_started |  |
| `Core/Queue.js` | `cesium_core::queue` | not_started |  |
| `Core/Ray.js` | `cesium_core::ray` | tested | spec mirrored |
| `Core/Rectangle.js` | `cesium_core::rectangle` | tested | spec mirrored |
| `Core/RectangleCollisionChecker.js` | `cesium_core::rectangleCollisionChecker` | not_started |  |
| `Core/RectangleGeometry.js` | `cesium_core::rectangleGeometry` | not_started |  |
| `Core/RectangleGeometryLibrary.js` | `cesium_core::rectangleGeometryLibrary` | not_started |  |
| `Core/RectangleOutlineGeometry.js` | `cesium_core::rectangleOutlineGeometry` | not_started |  |
| `Core/ReferenceFrame.js` | `cesium_core::referenceFrame` | not_started |  |
| `Core/Request.js` | `cesium_core::request` | not_started |  |
| `Core/RequestErrorEvent.js` | `cesium_core::requestErrorEvent` | not_started |  |
| `Core/RequestScheduler.js` | `cesium_core::requestScheduler` | not_started |  |
| `Core/RequestState.js` | `cesium_core::requestState` | not_started |  |
| `Core/RequestType.js` | `cesium_core::requestType` | not_started |  |
| `Core/resizeImageToNextPowerOfTwo.js` | `cesium_core::resizeImageToNextPowerOfTwo` | not_started |  |
| `Core/Resource.js` | `cesium_core::resource` | not_started |  |
| `Core/RuntimeError.js` | `cesium_core::runtimeError` | tested | spec mirrored |
| `Core/S2Cell.js` | `cesium_core::s2Cell` | not_started |  |
| `Core/sampleTerrain.js` | `cesium_core::sampleTerrain` | not_started |  |
| `Core/sampleTerrainMostDetailed.js` | `cesium_core::sampleTerrainMostDetailed` | not_started |  |
| `Core/scaleToGeodeticSurface.js` | `cesium_core::scale_to_geodetic_surface` | ported | used by Cartographic |
| `Core/ScreenSpaceEventHandler.js` | `cesium_core::screenSpaceEventHandler` | not_started |  |
| `Core/ScreenSpaceEventType.js` | `cesium_core::screenSpaceEventType` | not_started |  |
| `Core/ShowGeometryInstanceAttribute.js` | `cesium_core::showGeometryInstanceAttribute` | not_started |  |
| `Core/Simon1994PlanetaryPositions.js` | `cesium_core::simon1994PlanetaryPositions` | not_started |  |
| `Core/SimplePolylineGeometry.js` | `cesium_core::simplePolylineGeometry` | not_started |  |
| `Core/SphereGeometry.js` | `cesium_core::sphereGeometry` | not_started |  |
| `Core/SphereOutlineGeometry.js` | `cesium_core::sphereOutlineGeometry` | not_started |  |
| `Core/Spherical.js` | `cesium_core::spherical` | tested | spec mirrored |
| `Core/Spline.js` | `cesium_core::spline` | not_started |  |
| `Core/srgbToLinear.js` | `cesium_core::srgbToLinear` | not_started |  |
| `Core/SteppedSpline.js` | `cesium_core::steppedSpline` | not_started |  |
| `Core/Stereographic.js` | `cesium_core::stereographic` | not_started |  |
| `Core/subdivideArray.js` | `cesium_core::subdivideArray` | not_started |  |
| `Core/TaskProcessor.js` | `cesium_core::taskProcessor` | not_started |  |
| `Core/TerrainData.js` | `cesium_core::terrainData` | not_started |  |
| `Core/TerrainEncoding.js` | `cesium_core::terrainEncoding` | not_started |  |
| `Core/TerrainMesh.js` | `cesium_core::terrainMesh` | not_started |  |
| `Core/TerrainPicker.js` | `cesium_core::terrainPicker` | not_started |  |
| `Core/TerrainProvider.js` | `cesium_core::terrainProvider` | not_started |  |
| `Core/TerrainQuantization.js` | `cesium_core::terrainQuantization` | not_started |  |
| `Core/TexturePacker.js` | `cesium_core::texturePacker` | not_started |  |
| `Core/TileAvailability.js` | `cesium_core::tileAvailability` | not_started |  |
| `Core/TileEdge.js` | `cesium_core::tileEdge` | not_started |  |
| `Core/TileProviderError.js` | `cesium_core::tileProviderError` | not_started |  |
| `Core/TilingScheme.js` | `cesium_core::tilingScheme` | not_started |  |
| `Core/TimeConstants.js` | `cesium_core::time_constants` | tested | part of JulianDate system |
| `Core/TimeInterval.js` | `cesium_core::timeInterval` | not_started |  |
| `Core/TimeIntervalCollection.js` | `cesium_core::timeIntervalCollection` | not_started |  |
| `Core/TimeStandard.js` | `cesium_core::time_standard` | tested | part of JulianDate system |
| `Core/Tipsify.js` | `cesium_core::tipsify` | not_started |  |
| `Core/TrackingReferenceFrame.js` | `cesium_core::trackingReferenceFrame` | not_started |  |
| `Core/Transforms.js` | `cesium_core::transforms` | tested | spec mirrored |
| `Core/TranslationRotationScale.js` | `cesium_core::translationRotationScale` | not_started |  |
| `Core/TridiagonalSystemSolver.js` | `cesium_core::tridiagonalSystemSolver` | not_started |  |
| `Core/TrustedServers.js` | `cesium_core::trustedServers` | not_started |  |
| `Core/VectorPipeline.js` | `cesium_core::vectorPipeline` | not_started |  |
| `Core/VectorProvider.js` | `cesium_core::vectorProvider` | not_started |  |
| `Core/VertexFormat.js` | `cesium_core::vertexFormat` | not_started |  |
| `Core/VerticalExaggeration.js` | `cesium_core::verticalExaggeration` | not_started |  |
| `Core/VideoSynchronizer.js` | `cesium_core::videoSynchronizer` | not_started |  |
| `Core/Visibility.js` | `cesium_core::visibility` | not_started |  |
| `Core/VRTheWorldTerrainProvider.js` | `cesium_core::vRTheWorldTerrainProvider` | not_started |  |
| `Core/VulkanConstants.js` | `cesium_core::vulkanConstants` | not_started |  |
| `Core/WallGeometry.js` | `cesium_core::wallGeometry` | not_started |  |
| `Core/WallGeometryLibrary.js` | `cesium_core::wallGeometryLibrary` | not_started |  |
| `Core/WallOutlineGeometry.js` | `cesium_core::wallOutlineGeometry` | not_started |  |
| `Core/WebGLConstants.js` | `cesium_core::webGLConstants` | not_started |  |
| `Core/webGLConstantToGlslType.js` | `cesium_core::webGLConstantToGlslType` | not_started |  |
| `Core/WebMercatorProjection.js` | `cesium_core::web_mercator_projection` | tested | spec mirrored |
| `Core/WebMercatorTilingScheme.js` | `cesium_core::webMercatorTilingScheme` | not_started |  |
| `Core/WindingOrder.js` | `cesium_core::windingOrder` | not_started |  |
| `Core/WireframeIndexGenerator.js` | `cesium_core::wireframeIndexGenerator` | not_started |  |
| `Core/wrapFunction.js` | `cesium_core::wrapFunction` | not_started |  |
| `Core/writeTextToCanvas.js` | `cesium_core::writeTextToCanvas` | not_started |  |
<!-- END CORE TABLE -->

## Renderer（packages/engine/Source/Renderer → cesium-renderer）

| JS 文件 | Rust 模块 | 状态 | 备注 |
| --- | --- | --- | --- |
| `Renderer/Buffer.js` | `cesium_renderer::buffer` | not_started | 示例行；完整清单于 M1 生成 |

## Scene（packages/engine/Source/Scene → cesium-scene）

| JS 文件 | Rust 模块 | 状态 | 备注 |
| --- | --- | --- | --- |
| `Scene/Primitive.js` | `cesium_scene::primitive` | not_started | 示例行；完整清单于 M1 生成 |

## DataSources（packages/engine/Source/DataSources → cesium-data-sources）

| JS 文件 | Rust 模块 | 状态 | 备注 |
| --- | --- | --- | --- |
| `DataSources/Entity.js` | `cesium_data_sources::entity` | not_started | 示例行；完整清单于 M1 生成 |

## Shaders（packages/engine/Source/Shaders → cesium-shaders）

| JS 文件 | Rust 模块 | 状态 | 备注 |
| --- | --- | --- | --- |
| `Shaders/Appearances/*.glsl` | `cesium_shaders::appearances` | not_started | 策略见 shader-strategy.md（M2 定稿） |

## Workers（packages/engine/Source/Workers → cesium-workers）

| JS 文件 | Rust 模块 | 状态 | 备注 |
| --- | --- | --- | --- |
| `Workers/createBoxGeometry.js` | `cesium_workers::create_box_geometry` | not_started | 示例行；完整清单于 M1 生成 |

## Widget（packages/engine/Source/Widget → cesium-widgets）

| JS 文件 | Rust 模块 | 状态 | 备注 |
| --- | --- | --- | --- |
| `Widget/CesiumWidget.js` | `cesium_widgets::cesium_widget` | not_started | 示例行 |

## widgets（packages/widgets/Source → cesium-widgets）

| JS 文件 | Rust 模块 | 状态 | 备注 |
| --- | --- | --- | --- |
| `widgets/Viewer/Viewer.js` | `cesium_widgets::viewer` | not_started | 示例行 |
