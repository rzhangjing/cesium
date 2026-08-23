$jsRoot = "d:\Rust\cesium\packages\engine\Source\Scene"
$srcRoot = "d:\Rust\cesium\cesium-rs\crates\cesium-scene\src"

function To-RustMod($name) {
    $base = [System.IO.Path]::GetFileNameWithoutExtension($name)
    # Handle special cases
    if ($base -match '^\d') { $base = "_$base" }
    # Insert underscore before uppercase letters that follow lowercase/digits
    $result = $base -creplace '(?<=[a-z0-9])([A-Z])', '_$1'
    # Insert underscore between consecutive uppercase followed by lowercase (e.g., "GLTF" -> "G_L_T_F" is wrong, handle "KTX2" etc.)
    $result = $result -creplace '(?<=[A-Z])([A-Z][a-z])', '_$1'
    return $result.ToLower()
}

function To-StructName($name) {
    $base = [System.IO.Path]::GetFileNameWithoutExtension($name)
    return $base
}

# Clean src
if (Test-Path $srcRoot) { Remove-Item $srcRoot -Recurse -Force }
New-Item -ItemType Directory -Path $srcRoot -Force | Out-Null

# 1. Top-level Scene files
$topFiles = Get-ChildItem $jsRoot -Filter *.js | Sort-Object Name
$topMods = @()
foreach ($f in $topFiles) {
    $modName = To-RustMod $f.Name
    $structName = To-StructName $f.Name
    $lines = @(
        "//! Ported from ``packages/engine/Source/Scene/$($f.Name)``.",
        "//!",
        "//! Skeleton: requires Scene infrastructure.",
        "",
        "/// ``$structName`` — ported from CesiumJS.",
        "/// Skeleton implementation.",
        "pub struct $structName;",
        ""
    )
    Set-Content -Path "$srcRoot\$modName.rs" -Value ($lines -join "`n") -Encoding UTF8
    $topMods += $modName
}

# 2. Model/ subdirectory -> src/model/ module
$modelDir = "$jsRoot\Model"
$modelSrcDir = "$srcRoot\model"
New-Item -ItemType Directory -Path $modelSrcDir -Force | Out-Null

$modelMods = @()
if (Test-Path $modelDir) {
    $modelFiles = Get-ChildItem $modelDir -Filter *.js | Sort-Object Name
    foreach ($f in $modelFiles) {
        $modName = To-RustMod $f.Name
        $structName = To-StructName $f.Name
        $lines = @(
            "//! Ported from ``packages/engine/Source/Scene/Model/$($f.Name)``.",
            "//!",
            "//! Skeleton: requires glTF/Model infrastructure.",
            "",
            "/// ``$structName`` — ported from CesiumJS Model.",
            "/// Skeleton implementation.",
            "pub struct $structName;",
            ""
        )
        Set-Content -Path "$modelSrcDir\$modName.rs" -Value ($lines -join "`n") -Encoding UTF8
        $modelMods += $modName
    }
}

# Model mod.rs
$modelModLines = @("//! Model subsystem — glTF model loading and rendering.", "//! Ported from ``packages/engine/Source/Scene/Model/``.", "")
foreach ($m in $modelMods) {
    $modelModLines += "pub mod $m;"
}
$modelModLines += ""
Set-Content -Path "$modelSrcDir\mod.rs" -Value ($modelModLines -join "`n") -Encoding UTF8

# 3. GltfPipeline/ subdirectory -> src/gltf_pipeline/ module
$gltfDir = "$jsRoot\GltfPipeline"
$gltfSrcDir = "$srcRoot\gltf_pipeline"
New-Item -ItemType Directory -Path $gltfSrcDir -Force | Out-Null

$gltfMods = @()
if (Test-Path $gltfDir) {
    $gltfFiles = Get-ChildItem $gltfDir -Filter *.js | Sort-Object Name
    foreach ($f in $gltfFiles) {
        $modName = To-RustMod $f.Name
        $structName = To-StructName $f.Name
        $lines = @(
            "//! Ported from ``packages/engine/Source/Scene/GltfPipeline/$($f.Name)``.",
            "//!",
            "//! Skeleton: requires glTF pipeline infrastructure.",
            "",
            "/// ``$structName`` — ported from CesiumJS GltfPipeline.",
            "/// Skeleton implementation.",
            "pub struct $structName;",
            ""
        )
        Set-Content -Path "$gltfSrcDir\$modName.rs" -Value ($lines -join "`n") -Encoding UTF8
        $gltfMods += $modName
    }
}

# GltfPipeline mod.rs
$gltfModLines = @("//! glTF pipeline subsystem — glTF processing and optimization.", "//! Ported from ``packages/engine/Source/Scene/GltfPipeline/``.", "")
foreach ($m in $gltfMods) {
    $gltfModLines += "pub mod $m;"
}
$gltfModLines += ""
Set-Content -Path "$gltfSrcDir\mod.rs" -Value ($gltfModLines -join "`n") -Encoding UTF8

# 4. Generate lib.rs
$libLines = @(
    '//! One-to-one port of `packages/engine/Source/Scene`.',
    '//!',
    '//! Scene graph: Globe, Camera, Primitives, 3D Tiles, Model/glTF, etc.',
    '//! Skeleton implementations — full logic to be filled in progressively.',
    '',
    '#![forbid(unsafe_code)]',
    '#![allow(dead_code)]',
    ''
)
foreach ($m in $topMods) {
    $libLines += "pub mod $m;"
}
$libLines += "pub mod model;"
$libLines += "pub mod gltf_pipeline;"
$libLines += ""
Set-Content -Path "$srcRoot\lib.rs" -Value ($libLines -join "`n") -Encoding UTF8

$totalFiles = $topMods.Count + $modelMods.Count + $gltfMods.Count
Write-Host "Generated $totalFiles skeleton files (top: $($topMods.Count), model: $($modelMods.Count), gltf: $($gltfMods.Count))."
