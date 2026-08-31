// WGSL port of packages/engine/Source/Shaders/Builtin/Constants/*.glsl
// (all 41 czm_* GLSL constants mirrored 1:1, SH-01 task).
//
// DEVIATION: WGSL has no preprocessor/include machinery, so this module is
// self-contained; GLSL files that reference these constants inline their own
// copies in the sibling builtin WGSL modules of this crate.
//
// DEVIATION: GLSL `const float` maps to WGSL module-scope `const`.
// `czm_depthRange` is a struct-typed constant in GLSL; WGSL module-scope
// const expressions cannot carry runtime struct values in all drivers, so it
// is mirrored as a `var<private>` with the same initializer.
//
// DEVIATION: `czm_eyeHeight` is a CesiumJS automatic uniform (camera height
// in meters above the ellipsoid), referenced by GlobeFS.glsl /
// SkyAtmosphereCommon.glsl / VerticalExaggerationStageVS.glsl. It has no
// GLSL constant file; it is mirrored here as an explicit uniform binding so
// the SH-01 coverage gap is closed in WGSL.

const czm_epsilon1: f32 = 0.1;
const czm_epsilon2: f32 = 0.01;
const czm_epsilon3: f32 = 0.001;
const czm_epsilon4: f32 = 0.0001;
const czm_epsilon5: f32 = 0.00001;
const czm_epsilon6: f32 = 0.000001;
const czm_epsilon7: f32 = 0.0000001;

const czm_degreesPerRadian: f32 = 57.29577951308232;
const czm_radiansPerDegree: f32 = 0.017453292519943295;

const czm_pi: f32 = 3.141592653589793;
const czm_oneOverPi: f32 = 0.3183098861837907;
const czm_oneOverTwoPi: f32 = 0.15915494309189535;
const czm_piOverTwo: f32 = 1.5707963267948966;
const czm_piOverThree: f32 = 1.0471975511965976;
const czm_piOverFour: f32 = 0.7853981633974483;
const czm_piOverSix: f32 = 0.5235987755982988;
const czm_threePiOver2: f32 = 4.71238898038469;
const czm_twoPi: f32 = 6.283185307179586;

const czm_infinity: f32 = 5906376272000.0;
const czm_solarRadius: f32 = 695500000.0;
const czm_webMercatorMaxLatitude: f32 = 1.4844222297453324;

// Pass constants (czm_pass*.glsl)
const czm_passEnvironment: f32 = 0.0;
const czm_passCompute: f32 = 1.0;
const czm_passGlobe: f32 = 2.0;
const czm_passTerrainClassification: f32 = 3.0;
const czm_passCesium3DTileEdges: f32 = 4.0;
const czm_passCesium3DTile: f32 = 5.0;
const czm_passCesium3DTileClassification: f32 = 6.0;
const czm_passCesium3DTileClassificationIgnoreShow: f32 = 7.0;
const czm_passClassification: f32 = 7.0;
const czm_passOpaque: f32 = 8.0;
const czm_passTranslucent: f32 = 9.0;
const czm_passVoxels: f32 = 10.0;
const czm_passGaussianSplats: f32 = 11.0;
const czm_passCesium3DTileEdgesDirect: f32 = 12.0;
const czm_passOverlay: f32 = 13.0;

// SceneMode constants (czm_sceneMode*.glsl)
const czm_sceneModeMorphing: f32 = 0.0;
const czm_sceneModeColumbusView: f32 = 1.0;
const czm_sceneMode2D: f32 = 2.0;
const czm_sceneMode3D: f32 = 3.0;

// czm_depthRangeStruct / czm_depthRange (depthRange.glsl)
struct czm_depthRangeStruct {
    near: f32,
    far: f32,
}

var<private> czm_depthRange: czm_depthRangeStruct = czm_depthRangeStruct(0.0, 1.0);

// czm_eyeHeight: automatic uniform (camera height above the ellipsoid, meters)
@group(2) @binding(0) var<uniform> czm_eyeHeight: f32;

// Referencing function so every declared symbol is exercised by validation.
fn czm_constants_probe() -> f32 {
    var sum = czm_epsilon1 + czm_epsilon2 + czm_epsilon3 + czm_epsilon4
        + czm_epsilon5 + czm_epsilon6 + czm_epsilon7
        + czm_degreesPerRadian + czm_radiansPerDegree
        + czm_pi + czm_oneOverPi + czm_oneOverTwoPi + czm_piOverTwo
        + czm_piOverThree + czm_piOverFour + czm_piOverSix
        + czm_threePiOver2 + czm_twoPi
        + czm_infinity + czm_solarRadius + czm_webMercatorMaxLatitude
        + czm_passEnvironment + czm_passCompute + czm_passGlobe
        + czm_passTerrainClassification + czm_passCesium3DTileEdges
        + czm_passCesium3DTile + czm_passCesium3DTileClassification
        + czm_passCesium3DTileClassificationIgnoreShow + czm_passClassification
        + czm_passOpaque + czm_passTranslucent + czm_passVoxels
        + czm_passGaussianSplats + czm_passCesium3DTileEdgesDirect
        + czm_passOverlay
        + czm_sceneModeMorphing + czm_sceneModeColumbusView
        + czm_sceneMode2D + czm_sceneMode3D
        + czm_depthRange.near + czm_depthRange.far
        + czm_eyeHeight;
    return sum;
}
