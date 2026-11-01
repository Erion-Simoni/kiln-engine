param([switch]$Verbose)

if ($Verbose) {
    cargo check --lib 2>&1 | Write-Output
} else {
    cargo check --lib 2>nul | Out-File -FilePath kiln-check-dbg.txt -Encoding utf8
    Write-Output "Check complete, see kiln-check-dbg.txt"
}