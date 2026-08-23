$jsRoot = "d:\Rust\cesium\packages\engine\Source\Scene"
$srcRoot = "d:\Rust\cesium\cesium-rs\crates\cesium-scene\src"

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

# Gpm/ subdirectory -> src/gpm/ module
$gpmDir = "$jsRoot\Gpm"
$gpmSrcDir = "$srcRoot\gpm"
New-Item -ItemType Directory -Path $gpmSrcDir -Force | Out-Null

$gpmMods = @()
$gpmFiles = Get-ChildItem $gpmDir -Filter *.js | Sort-Object Name
foreach ($f in $gpmFiles) {
    $modName = To-RustMod $f.Name
    $structName = To-StructName $f.Name
    $lines = @(
        "//! Ported from ``packages/engine/Source/Scene/Gpm/$($f.Name)``.",
        "//!",
        "//! Skeleton: requires GPM infrastructure.",
        "",
        "/// ``$structName`` — ported from CesiumJS GPM.",
        "/// Skeleton implementation.",
        "pub struct $structName;",
        ""
    )
    Set-Content -Path "$gpmSrcDir\$modName.rs" -Value ($lines -join "`n") -Encoding UTF8
    $gpmMods += $modName
}

# Gpm mod.rs
$gpmModLines = @("//! GPM subsystem.", "//! Ported from ``packages/engine/Source/Scene/Gpm/``.", "")
foreach ($m in $gpmMods) {
    $gpmModLines += "pub mod $m;"
}
$gpmModLines += ""
Set-Content -Path "$gpmSrcDir\mod.rs" -Value ($gpmModLines -join "`n") -Encoding UTF8

# Update lib.rs to add gpm module
$libContent = Get-Content "$srcRoot\lib.rs" -Raw
$libContent = $libContent -replace 'pub mod gltf_pipeline;', "pub mod gltf_pipeline;`npub mod gpm;"
Set-Content -Path "$srcRoot\lib.rs" -Value $libContent -Encoding UTF8

Write-Host "Added GPM module with $($gpmMods.Count) files."
