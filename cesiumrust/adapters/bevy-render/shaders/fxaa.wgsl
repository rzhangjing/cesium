// Blueprint: cesium-rs/crates/cesium-shaders/shaders/FXAA3_11.glsl L102-108 (preset 12 defines),
//            L261-650 (FxaaPixelShader core algorithm);
//            packages/engine/Source/Shaders/PostProcessStages/FXAA.glsl L1-21 (interface wrapper).
// Quality preset 12 ONLY (FXAA_QUALITY_PS=5, P0=1.0, P1=1.5, P2=2.0, P3=4.0, P4=12.0).
// CesiumJS params: subpix=0.5, edgeThreshold=0.125, edgeThresholdMin=0.0833.
// Green channel as luma (FXAA_GREEN_AS_LUMA=1). Early exit enabled (FXAA_EARLY_EXIT=1).
// DEVIATION: see docs/deviations.md#dev-017 (WGSL rewrite, preset 12 only)
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screenTexture: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

// ─── Quality preset 12 constants (FXAA3_11.glsl L102-108) ───────────────────
const QUALITY_PS: i32 = 5;
const QUALITY_P0: f32 = 1.0;
const QUALITY_P1: f32 = 1.5;
const QUALITY_P2: f32 = 2.0;
const QUALITY_P3: f32 = 4.0;
const QUALITY_P4: f32 = 12.0;

// ─── CesiumJS FXAA.glsl L5-7 parameters ─────────────────────────────────────
const SUBPIX_QUALITY: f32 = 0.5;
const EDGE_THRESHOLD: f32 = 0.125;
const EDGE_THRESHOLD_MIN: f32 = 0.0833;

// ─── Helpers (FXAA3_11.glsl L273-277) ───────────────────────────────────────
fn fxaa_sat(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
}

// Green-as-luma (FXAA_GREEN_AS_LUMA=1, L277: FxaaLuma returns rgba.y)
fn fxaa_luma(rgba: vec4<f32>) -> f32 {
    return rgba.y;
}

fn fxaa_tex_top(tex: texture_2d<f32>, s: sampler, p: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(tex, s, p, 0.0);
}

fn fxaa_tex_off(tex: texture_2d<f32>, s: sampler, p: vec2<f32>, o: vec2<f32>, r: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(tex, s, p + (o * r), 0.0);
}

// ─── FXAA 3.11 Pixel Shader — Quality Preset 12 (unrolled, 5 search steps) ──
// Translated from FXAA3_11.glsl L279-650. Only the PS=5 path is compiled
// (presets 10-11 have PS<5; presets 13+ have PS>5 — all excluded per plan).
fn fxaa_pixel_shader(
    pos: vec2<f32>,
    tex: texture_2d<f32>,
    s: sampler,
    rcp_frame: vec2<f32>,
) -> vec4<f32> {
    // L336-344: center + 4-neighbor luma
    var posM = pos;
    let rgbyM = fxaa_tex_top(tex, s, posM);
    let lumaM = rgbyM.y;
    let lumaS = fxaa_luma(fxaa_tex_off(tex, s, posM, vec2<f32>( 0.0,  1.0), rcp_frame));
    let lumaE = fxaa_luma(fxaa_tex_off(tex, s, posM, vec2<f32>( 1.0,  0.0), rcp_frame));
    let lumaN = fxaa_luma(fxaa_tex_off(tex, s, posM, vec2<f32>( 0.0, -1.0), rcp_frame));
    let lumaW = fxaa_luma(fxaa_tex_off(tex, s, posM, vec2<f32>(-1.0,  0.0), rcp_frame));

    // L346-357: local contrast range + early exit
    let maxSM = max(lumaS, lumaM);
    let minSM = min(lumaS, lumaM);
    let maxESM = max(lumaE, maxSM);
    let minESM = min(lumaE, minSM);
    let maxWN = max(lumaN, lumaW);
    let minWN = min(lumaN, lumaW);
    let rangeMax = max(maxWN, maxESM);
    let rangeMin = min(minWN, minESM);
    let rangeMaxScaled = rangeMax * EDGE_THRESHOLD;
    let range = rangeMax - rangeMin;
    let rangeMaxClamped = max(EDGE_THRESHOLD_MIN, rangeMaxScaled);

    // L359-360: FXAA_EARLY_EXIT
    if range < rangeMaxClamped {
        return rgbyM;
    }

    // L362-365: diagonal neighbors
    let lumaNW = fxaa_luma(fxaa_tex_off(tex, s, posM, vec2<f32>(-1.0, -1.0), rcp_frame));
    let lumaSE = fxaa_luma(fxaa_tex_off(tex, s, posM, vec2<f32>( 1.0,  1.0), rcp_frame));
    let lumaNE = fxaa_luma(fxaa_tex_off(tex, s, posM, vec2<f32>( 1.0, -1.0), rcp_frame));
    let lumaSW = fxaa_luma(fxaa_tex_off(tex, s, posM, vec2<f32>(-1.0,  1.0), rcp_frame));

    // L367-372: subpixel + edge detection
    let lumaNS = lumaN + lumaS;
    let lumaWE = lumaW + lumaE;
    let subpixRcpRange = 1.0 / range;
    let subpixNSWE = lumaNS + lumaWE;
    let edgeHorz1 = (-2.0 * lumaM) + lumaNS;
    let edgeVert1 = (-2.0 * lumaM) + lumaWE;

    // L374-377
    let lumaNESE = lumaNE + lumaSE;
    let lumaNWNE = lumaNW + lumaNE;
    let edgeHorz2 = (-2.0 * lumaE) + lumaNESE;
    let edgeVert2 = (-2.0 * lumaN) + lumaNWNE;

    // L379-386
    let lumaNWSW = lumaNW + lumaSW;
    let lumaSWSE = lumaSW + lumaSE;
    let edgeHorz4 = (abs(edgeHorz1) * 2.0) + abs(edgeHorz2);
    let edgeVert4 = (abs(edgeVert1) * 2.0) + abs(edgeVert2);
    let edgeHorz3 = (-2.0 * lumaW) + lumaNWSW;
    let edgeVert3 = (-2.0 * lumaS) + lumaSWSE;
    let edgeHorz = abs(edgeHorz3) + edgeHorz4;
    let edgeVert = abs(edgeVert3) + edgeVert4;

    // L388-391: span direction
    let subpixNWSWNESE = lumaNWSW + lumaNESE;
    var lengthSign = rcp_frame.x;
    let horzSpan = edgeHorz >= edgeVert;
    let subpixA = subpixNSWE * 2.0 + subpixNWSWNESE;

    // L393-396
    var lumaN_sel = lumaN;
    var lumaS_sel = lumaS;
    if !horzSpan {
        lumaN_sel = lumaW;
    }
    if !horzSpan {
        lumaS_sel = lumaE;
    }
    if horzSpan {
        lengthSign = rcp_frame.y;
    }
    let subpixB = (subpixA * (1.0 / 12.0)) - lumaM;

    // L398-405: gradient + pair direction
    let gradientN = lumaN_sel - lumaM;
    let gradientS = lumaS_sel - lumaM;
    var lumaNN = lumaN_sel + lumaM;
    let lumaSS = lumaS_sel + lumaM;
    let pairN = abs(gradientN) >= abs(gradientS);
    let gradient = max(abs(gradientN), abs(gradientS));
    if pairN {
        lengthSign = -lengthSign;
    }
    let subpixC = fxaa_sat(abs(subpixB) * subpixRcpRange);

    // L407-414: position setup
    var posB = posM;
    var offNP = vec2<f32>(0.0, 0.0);
    if !horzSpan {
        offNP.x = 0.0;
        offNP.y = rcp_frame.y;
    } else {
        offNP.x = rcp_frame.x;
        offNP.y = 0.0;
    }
    if !horzSpan {
        posB.x = posB.x + lengthSign * 0.5;
    }
    if horzSpan {
        posB.y = posB.y + lengthSign * 0.5;
    }

    // L416-425: first search step (P0 = 1.0)
    var posN = posB - offNP * QUALITY_P0;
    var posP = posB + offNP * QUALITY_P0;
    let subpixD = ((-2.0) * subpixC) + 3.0;
    var lumaEndN = fxaa_luma(fxaa_tex_top(tex, s, posN));
    let subpixE = subpixC * subpixC;
    var lumaEndP = fxaa_luma(fxaa_tex_top(tex, s, posP));

    // L427-431
    if !pairN {
        lumaNN = lumaSS;
    }
    let gradientScaled = gradient * (1.0 / 4.0);
    let lumaMM = lumaM - lumaNN * 0.5;
    let subpixF = subpixD * subpixE;
    let lumaMLTZero = lumaMM < 0.0;

    // L433-441: step 1 (P1 = 1.5)
    lumaEndN = lumaEndN - lumaNN * 0.5;
    lumaEndP = lumaEndP - lumaNN * 0.5;
    var doneN = abs(lumaEndN) >= gradientScaled;
    var doneP = abs(lumaEndP) >= gradientScaled;
    if !doneN {
        posN = posN - offNP * QUALITY_P1;
    }
    if !doneP {
        posP = posP + offNP * QUALITY_P1;
    }

    // L443-454: step 2 (P2 = 2.0)
    var doneNP = (!doneN) || (!doneP);
    if doneNP {
        if !doneN { lumaEndN = fxaa_luma(fxaa_tex_top(tex, s, posN)); }
        if !doneP { lumaEndP = fxaa_luma(fxaa_tex_top(tex, s, posP)); }
        if !doneN { lumaEndN = lumaEndN - lumaNN * 0.5; }
        if !doneP { lumaEndP = lumaEndP - lumaNN * 0.5; }
        doneN = abs(lumaEndN) >= gradientScaled;
        doneP = abs(lumaEndP) >= gradientScaled;
        if !doneN { posN = posN - offNP * QUALITY_P2; }
        if !doneP { posP = posP + offNP * QUALITY_P2; }
        doneNP = (!doneN) || (!doneP);
    }

    // L456-468: step 3 (P3 = 4.0) — QUALITY_PS > 3 ✓ (PS=5)
    if doneNP {
        if !doneN { lumaEndN = fxaa_luma(fxaa_tex_top(tex, s, posN)); }
        if !doneP { lumaEndP = fxaa_luma(fxaa_tex_top(tex, s, posP)); }
        if !doneN { lumaEndN = lumaEndN - lumaNN * 0.5; }
        if !doneP { lumaEndP = lumaEndP - lumaNN * 0.5; }
        doneN = abs(lumaEndN) >= gradientScaled;
        doneP = abs(lumaEndP) >= gradientScaled;
        if !doneN { posN = posN - offNP * QUALITY_P3; }
        if !doneP { posP = posP + offNP * QUALITY_P3; }
        doneNP = (!doneN) || (!doneP);
    }

    // L470-482: step 4 (P4 = 12.0) — QUALITY_PS > 4 ✓ (PS=5)
    if doneNP {
        if !doneN { lumaEndN = fxaa_luma(fxaa_tex_top(tex, s, posN)); }
        if !doneP { lumaEndP = fxaa_luma(fxaa_tex_top(tex, s, posP)); }
        if !doneN { lumaEndN = lumaEndN - lumaNN * 0.5; }
        if !doneP { lumaEndP = lumaEndP - lumaNN * 0.5; }
        doneN = abs(lumaEndN) >= gradientScaled;
        doneP = abs(lumaEndP) >= gradientScaled;
        if !doneN { posN = posN - offNP * QUALITY_P4; }
        if !doneP { posP = posP + offNP * QUALITY_P4; }
        // No step 5: QUALITY_PS > 5 is FALSE for preset 12 (PS=5).
    }

    // L628-649: final blend
    var dstN = posM.x - posN.x;
    var dstP = posP.x - posM.x;
    if !horzSpan {
        dstN = posM.y - posN.y;
    }
    if !horzSpan {
        dstP = posP.y - posM.y;
    }

    let goodSpanN = (lumaEndN < 0.0) != lumaMLTZero;
    let spanLength = dstP + dstN;
    let goodSpanP = (lumaEndP < 0.0) != lumaMLTZero;
    let spanLengthRcp = 1.0 / spanLength;

    let directionN = dstN < dstP;
    let dst = min(dstN, dstP);
    var goodSpan = goodSpanP;
    if directionN {
        goodSpan = goodSpanN;
    }
    let subpixG = subpixF * subpixF;
    let pixelOffset = (dst * (-spanLengthRcp)) + 0.5;
    let subpixH = subpixG * SUBPIX_QUALITY;

    let pixelOffsetGood = select(0.0, pixelOffset, goodSpan);
    let pixelOffsetSubpix = max(pixelOffsetGood, subpixH);
    if !horzSpan {
        posM.x = posM.x + pixelOffsetSubpix * lengthSign;
    }
    if horzSpan {
        posM.y = posM.y + pixelOffsetSubpix * lengthSign;
    }

    let finalColor = fxaa_tex_top(tex, s, posM);
    return vec4<f32>(finalColor.xyz, lumaM);
}

// ─── Fragment entry point ────────────────────────────────────────────────────
@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let resolution = vec2<f32>(textureDimensions(screenTexture));
    let rcp_frame = vec2<f32>(1.0) / resolution;
    let tex_coord = in.position.xy * rcp_frame;

    let color = fxaa_pixel_shader(tex_coord, screenTexture, samp, rcp_frame);
    // Preserve original alpha (FXAA.glsl L19-20)
    let alpha = textureSampleLevel(screenTexture, samp, tex_coord, 0.0).a;
    return vec4<f32>(color.rgb, alpha);
}
