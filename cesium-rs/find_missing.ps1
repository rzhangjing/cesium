$js = Get-ChildItem d:\Rust\cesium\packages\engine\Source\Core\*.js -Name | ForEach-Object { $_ -replace '.js$','' }
$rs = Get-ChildItem d:\Rust\cesium\cesium-rs\crates\cesium-core\src\*.rs -Name | ForEach-Object { $_ -replace '.rs$','' } | Where-Object { $_ -ne 'lib' }

# Convert RS snake_case to lowered no-underscore form for comparison
$rsNorm = $rs | ForEach-Object { $_.ToLower() -replace '_','' }

$jsNorm = @{}
foreach ($f in $js) {
    $key = $f.ToLower() -replace '_',''
    $jsNorm[$key] = $f
}

$missing = @()
foreach ($key in $jsNorm.Keys) {
    if ($key -notin $rsNorm) {
        $missing += $jsNorm[$key]
    }
}

Write-Host "=== Missing RS files (JS source without RS port) ==="
$missing | Sort-Object | ForEach-Object { Write-Host $_ }
Write-Host ""
Write-Host "Total JS: $($js.Count), Total RS: $($rs.Count), Missing: $($missing.Count)"
