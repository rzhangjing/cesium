$srcBase = "d:\Rust\cesium\cesium-rs\crates"

function To-RustMod($name) {
    $base = [System.IO.Path]::GetFileNameWithoutExtension($name)
    if ($base -match '^\d') { $base = "_$base" }
    $result = $base -creplace '(?<=[a-z0-9])([A-Z])', '_$1'
    $result = $result -creplace '(?<=[A-Z])([A-Z][a-z])', '_$1'
    return $result.ToLower()
}

function To-StructName($name) {
    return [System.IO.Path]::GetFileNameWithoutExtension($name)
}

function Gen-Module($jsDir, $srcDir, $crateName, $portFrom) {
    if (-not (Test-Path $jsDir)) { Write-Host "SKIP: $jsDir not found"; return 0 }
    if (Test-Path $srcDir) { Remove-Item $srcDir -Recurse -Force }
    New-Item -ItemType Directory -Path $srcDir -Force | Out-Null

    $jsFiles = Get-ChildItem $jsDir -Filter *.js | Sort-Object Name
    $mods = @()
    foreach ($f in $jsFiles) {
        $modName = To-RustMod $f.Name
        $structName = To-StructName $f.Name
        $lines = @(
            "//! Ported from ``$portFrom/$($f.Name)``.",
            "//!",
            "//! Skeleton implementation.",
            "",
            "/// ``$structName`` — ported from CesiumJS.",
            "pub struct $structName;",
            ""
        )
        Set-Content -Path "$srcDir\$modName.rs" -Value ($lines -join "`n") -Encoding UTF8
        $mods += $modName
    }

    # Generate lib.rs
    $libLines = @(
        "//! One-to-one port of ``$portFrom``.",
        "//!",
        "//! Skeleton implementations.",
        "",
        "#![forbid(unsafe_code)]",
        "#![allow(dead_code)]",
        ""
    )
    foreach ($m in $mods) {
        $libLines += "pub mod $m;"
    }
    $libLines += ""
    Set-Content -Path "$srcDir\lib.rs" -Value ($libLines -join "`n") -Encoding UTF8

    Write-Host "$crateName : $($mods.Count) files"
    return $mods.Count
}

$total = 0

# Workers
$total += Gen-Module "d:\Rust\cesium\packages\engine\Source\Workers" "$srcBase\cesium-workers\src" "cesium-workers" "packages/engine/Source/Workers"

# DataSources
$total += Gen-Module "d:\Rust\cesium\packages\engine\Source\DataSources" "$srcBase\cesium-data-sources\src" "cesium-data-sources" "packages/engine/Source/DataSources"

# Widgets (combine engine Widget + widgets package)
$widgetSrcDir = "$srcBase\cesium-widgets\src"
if (Test-Path $widgetSrcDir) { Remove-Item $widgetSrcDir -Recurse -Force }
New-Item -ItemType Directory -Path $widgetSrcDir -Force | Out-Null

$widgetMods = @()

# Engine Widget/ (1 file: CesiumWidget.js)
$engineWidgetDir = "d:\Rust\cesium\packages\engine\Source\Widget"
if (Test-Path $engineWidgetDir) {
    $jsFiles = Get-ChildItem $engineWidgetDir -Filter *.js | Sort-Object Name
    foreach ($f in $jsFiles) {
        $modName = To-RustMod $f.Name
        $structName = To-StructName $f.Name
        $lines = @(
            "//! Ported from ``packages/engine/Source/Widget/$($f.Name)``.",
            "//!",
            "//! Skeleton implementation.",
            "",
            "/// ``$structName`` — ported from CesiumJS.",
            "pub struct $structName;",
            ""
        )
        Set-Content -Path "$widgetSrcDir\$modName.rs" -Value ($lines -join "`n") -Encoding UTF8
        $widgetMods += $modName
    }
}

# Widgets package
$widgetsPkgDir = "d:\Rust\cesium\packages\widgets\Source"
if (Test-Path $widgetsPkgDir) {
    $jsFiles = Get-ChildItem $widgetsPkgDir -Recurse -Filter *.js | Sort-Object Name
    foreach ($f in $jsFiles) {
        $modName = To-RustMod $f.Name
        $structName = To-StructName $f.Name
        # Avoid duplicate module names
        $relPath = $f.FullName.Replace("$widgetsPkgDir\", "")
        $lines = @(
            "//! Ported from ``packages/widgets/Source/$relPath``.",
            "//!",
            "//! Skeleton implementation.",
            "",
            "/// ``$structName`` — ported from CesiumJS widgets.",
            "pub struct $structName;",
            ""
        )
        $outPath = "$widgetSrcDir\$modName.rs"
        if (Test-Path $outPath) {
            # Prefix with parent dir name to avoid collision
            $parentDir = $f.Directory.Name
            if ($parentDir -ne "Source") {
                $modName = "$(To-RustMod $parentDir)_$modName"
            } else {
                $modName = "widget_$modName"
            }
            $outPath = "$widgetSrcDir\$modName.rs"
            $lines[5] = "/// ``$structName`` (from widgets/$relPath) — ported from CesiumJS."
        }
        Set-Content -Path $outPath -Value ($lines -join "`n") -Encoding UTF8
        $widgetMods += $modName
    }
}

# Widgets lib.rs
$libLines = @(
    '//! One-to-one port of `packages/engine/Source/Widget` + `packages/widgets/Source`.',
    '//!',
    '//! Widget/ViewModel layer — DOM adaptation via winit.',
    '',
    '#![forbid(unsafe_code)]',
    '#![allow(dead_code)]',
    ''
)
foreach ($m in $widgetMods) {
    $libLines += "pub mod $m;"
}
$libLines += ""
Set-Content -Path "$widgetSrcDir\lib.rs" -Value ($libLines -join "`n") -Encoding UTF8

Write-Host "cesium-widgets : $($widgetMods.Count) files"
$total += $widgetMods.Count

Write-Host ""
Write-Host "Total generated: $total files"
