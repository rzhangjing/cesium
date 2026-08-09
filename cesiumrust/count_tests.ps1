foreach ($f in Get-ChildItem *_spec.rs) {
    $c = (Get-Content $f.FullName | Select-String '#\[test\]').Count
    Write-Output ($c.ToString().PadLeft(4) + ' ' + $f.Name)
}
