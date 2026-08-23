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

# Gpm is at Model/Extensions/Gpm/
$gpmDir = "$jsRoot\Model\Extensions\Gpm"
$gpmSrcDir = "$srcRoot\model\extensions\gpm"
New-Item -ItemType Directory -Path $gpmSrcDir -Force | Out-Null

$gpmMods = @()
$gpmFiles = Get-ChildItem $gpmDir -Filter *.js | Sort-Object Name
foreach ($f in $gpmFiles) {
    $modName = To-RustMod $f.Name
    $structName = To-StructName $f.Name
    $lines = @(
        "//! Ported from ``packages/engine/Source/Scene/Model/Extensions/Gpm/$($f.Name)``.",
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
$gpmModLines = @("//! GPM (Generalized Polygon Mesh) subsystem.", "//! Ported from ``packages/engine/Source/Scene/Model/Extensions/Gpm/``.", "")
foreach ($m in $gpmMods) {
    $gpmModLines += "pub mod $m;"
}
$gpmModLines += ""
Set-Content -Path "$gpmSrcDir\mod.rs" -Value ($gpmModLines -join "`n") -Encoding UTF8

# Extensions mod.rs
$extSrcDir = "$srcRoot\model\extensions"
$extModLines = @("//! Model extensions.", "pub mod gpm;", "")
Set-Content -Path "$extSrcDir\mod.rs" -Value ($extModLines -join "`n") -Encoding UTF8

# Update model/mod.rs to add extensions
$modelModContent = Get-Content "$srcRoot\model\mod.rs" -Raw
$modelModContent = $modelModContent.TrimEnd() + "`npub mod extensions;`n"
Set-Content -Path "$srcRoot\model\mod.rs" -Value $modelModContent -Encoding UTF8

# Remove the empty gpm module at top level if it was created
if (Test-Path "$srcRoot\gpm") { Remove-Item "$srcRoot\gpm" -Recurse -Force }
# Remove gpm from lib.rs if it was added
$libContent = Get-Content "$srcRoot\lib.rs" -Raw
$libContent = $libContent -replace "pub mod gpm;\r?\n?", ""
Set-Content -Path "$srcRoot\lib.rs" -Value $libContent -Encoding UTF8

Write-Host "Added GPM module with $($gpmMods.Count) files under model/extensions/gpm."
