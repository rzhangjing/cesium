# Classifies every #[ignore] in specs/tests by file and by reason into three
# buckets: UNIGNORE (deferred dep / bug), GPU (Track B), DEVIATION (permanent).
# Emits per-file summary + reason summary as plain text for ledger authoring.

$ErrorActionPreference = 'Stop'
$dir = 'd:\Rust\cesium\cesium-rs\specs\tests'

function Get-Bucket([string]$reason) {
    if ($reason -match 'cesium-scene context|WebGL2 probe|GPU|wgpu') { return 'GPU' }
    if ($reason -match '^deferred:|usize wrapping bug|requires .+ worker \(Track') { return 'UNIGNORE' }
    return 'DEVIATION'
}

$files = Get-ChildItem -Recurse -Filter *.rs -Path $dir | Sort-Object Name
$perFile = New-Object System.Collections.Generic.List[string]
$reasonAgg = @{}
$totU = 0; $totG = 0; $totD = 0

foreach ($f in $files) {
    $ms = Select-String -Path $f.FullName -Pattern '^\s*#\[ignore\s*(=\s*"([^"]*)")?\]'
    if (-not $ms) { continue }
    $u = 0; $g = 0; $d = 0
    foreach ($m in $ms) {
        $reason = if ($m.Matches[0].Groups[2].Success) { $m.Matches[0].Groups[2].Value } else { '(no reason)' }
        $b = Get-Bucket $reason
        if ($b -eq 'UNIGNORE') { $u++ } elseif ($b -eq 'GPU') { $g++ } else { $d++ }
        if (-not $reasonAgg.ContainsKey($reason)) { $reasonAgg[$reason] = @('U',0) }
    }
    $totU += $u; $totG += $g; $totD += $d
    $perFile.Add(('{0}|{1}|{2}|{3}|{4}' -f $f.Name, ($u+$g+$d), $u, $g, $d))
}

Write-Host '=== PER FILE (file|total|unignore|gpu|deviation) ==='
$perFile | ForEach-Object { Write-Host $_ }
Write-Host ('=== TOTALS unignore={0} gpu={1} deviation={2} sum={3} ===' -f $totU, $totG, $totD, ($totU+$totG+$totD))

# Reason-level aggregation with bucket
$agg = @{}
foreach ($f in $files) {
    $ms = Select-String -Path $f.FullName -Pattern '^\s*#\[ignore\s*(=\s*"([^"]*)")?\]'
    foreach ($m in $ms) {
        $reason = if ($m.Matches[0].Groups[2].Success) { $m.Matches[0].Groups[2].Value } else { '(no reason)' }
        $b = Get-Bucket $reason
        $key = "$b|$reason"
        if ($agg.ContainsKey($key)) { $agg[$key]++ } else { $agg[$key] = 1 }
    }
}
Write-Host '=== REASONS (bucket|count|reason) ==='
$agg.GetEnumerator() | Sort-Object { $_.Value } -Descending | ForEach-Object {
    $parts = $_.Key -split '\|', 2
    Write-Host ('{0}|{1}|{2}' -f $parts[0], $_.Value, $parts[1])
}
