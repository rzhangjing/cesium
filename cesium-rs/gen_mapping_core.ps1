# Regenerates the Core ledger table in docs/MAPPING.md using two criteria:
#   1. rs/js line-count ratio > 0.5  -> ported
#   2. dedicated mirror spec exists  -> tested
# Pure ASCII to avoid PS 5.1 encoding issues. Writes docs/core_table.tmp.md
# and prints summary counts.

$ErrorActionPreference = 'Stop'

$repo      = 'd:\Rust\cesium'
$jsDir     = Join-Path $repo 'packages\engine\Source\Core'
$rsDir     = Join-Path $repo 'cesium-rs\crates\cesium-core\src'
$specDir   = Join-Path $repo 'cesium-rs\specs\tests\core'
$outFile   = Join-Path $repo 'cesium-rs\docs\core_table.tmp.md'

# JS basename (no ext) -> rs basename, for names the generic converter mangles.
$overrides = @{
    'Cesium3DTilesTerrainData'              = 'cesium3d_tiles_terrain_data'
    'Cesium3DTilesTerrainGeometryProcessor' = 'cesium3d_tiles_terrain_geometry_processor'
    'Cesium3DTilesTerrainProvider'          = 'cesium3d_tiles_terrain_provider'
    'Iau2000Orientation'                    = 'iau2000_orientation'
    'Iau2006XysData'                        = 'iau2006_xys_data'
    'Iau2006XysSample'                      = 'iau2006_xys_sample'
    'IauOrientationAxes'                    = 'iau_orientation_axes'
    'IauOrientationParameters'              = 'iau_orientation_parameters'
    'Intersections2D'                       = 'intersections2d'
    'Iso8601'                               = 'iso8601'
    'KTX2Transcoder'                        = 'ktx2_transcoder'
    'loadKTX2'                              = 'load_ktx2'
    'Simon1994PlanetaryPositions'           = 'simon1994_planetary_positions'
    'VRTheWorldTerrainProvider'             = 'vr_the_world_terrain_provider'
    'WebGLConstants'                        = 'webgl_constants'
    'webGLConstantToGlslType'               = 'webgl_constant_to_glsl_type'
    'S2Cell'                                = 's2_cell'
    'EncodedCartesian3'                     = 'encoded_cartesian3'
}

function Convert-ToSnake([string]$name) {
    if ($overrides.ContainsKey($name)) { return $overrides[$name] }
    # Insert _ between a lowercase/digit and an uppercase letter.
    $s = [regex]::Replace($name, '([a-z0-9])([A-Z])', '$1_$2')
    # Insert _ at acronym boundary: uppercase run followed by Title word.
    $s = [regex]::Replace($s, '([A-Z]+)([A-Z][a-z])', '$1_$2')
    return $s.ToLower()
}

function Count-Lines([string]$path) {
    if (-not (Test-Path $path)) { return 0 }
    return (Get-Content $path).Count
}

$jsFiles = Get-ChildItem $jsDir -File | Sort-Object Name
$rows    = New-Object System.Collections.Generic.List[string]
$tested  = 0; $ported = 0; $notStarted = 0

foreach ($f in $jsFiles) {
    $jsName  = $f.Name                 # e.g. Cartesian3.js / Check.d.ts
    $jsPath  = $f.FullName
    $jsLines = Count-Lines $jsPath

    # Base name without extension(s): strip .js / .d.ts
    $base = $jsName -replace '\.d\.ts$', '' -replace '\.js$', ''

    $snake  = Convert-ToSnake $base
    $rsPath = Join-Path $rsDir ($snake + '.rs')
    $rsExists = Test-Path $rsPath
    $rsLines  = if ($rsExists) { Count-Lines $rsPath } else { 0 }

    $specPath   = Join-Path $specDir ('core_' + $snake + '_spec.rs')
    $specExists = Test-Path $specPath

    $ratio = 0.0
    if ($jsLines -gt 0) { $ratio = [double]$rsLines / [double]$jsLines }

    if ($specExists) {
        $status = 'tested'; $remark = 'spec mirrored'; $tested++
    } elseif ($rsExists -and $ratio -gt 0.5) {
        $status = 'ported'; $remark = ('ratio {0:F2}' -f $ratio); $ported++
    } elseif ($rsExists) {
        # File exists but thin/partial: still not a substantive port.
        $status = 'not_started'; $remark = ('stub ratio {0:F2}' -f $ratio); $notStarted++
    } else {
        $status = 'not_started'; $remark = ''; $notStarted++
    }

    $module = 'cesium_core::' + $snake
    $rows.Add(('| `Core/{0}` | `{1}` | {2} | {3} |' -f $jsName, $module, $status, $remark))
}

$header = @(
    '| JS ' + [char]0x6587 + [char]0x4EF6 + ' | Rust ' + [char]0x6A21 + [char]0x5757 + ' | ' + [char]0x72B6 + [char]0x6001 + ' | ' + [char]0x5907 + [char]0x6CE8 + ' |',
    '| --- | --- | --- | --- |'
)
# Use plain ASCII-friendly header to avoid encoding issues; we patch CJK later.
$header = @(
    '| JS_FILE | RUST_MODULE | STATUS | REMARK |',
    '| --- | --- | --- | --- |'
)

$all = $header + $rows
[System.IO.File]::WriteAllLines($outFile, $all, (New-Object System.Text.UTF8Encoding($true)))

Write-Host ("Total JS files : {0}" -f $jsFiles.Count)
Write-Host ("tested        : {0}" -f $tested)
Write-Host ("ported        : {0}" -f $ported)
Write-Host ("not_started   : {0}" -f $notStarted)
