$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
Set-Location "d:\rustworkspace\meridianops\gateway"
cargo build 2>&1 | Out-File -FilePath "d:\rustworkspace\meridianops\gateway\build_output.log" -Encoding UTF8
$exitCode = $LASTEXITCODE
"EXIT_CODE: $exitCode" | Out-File -FilePath "d:\rustworkspace\meridianops\gateway\build_done.log" -Encoding UTF8
