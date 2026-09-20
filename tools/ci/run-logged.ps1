# Runs one command of the CI gate and, when it fails, repeats the lines that say WHY as an error
# annotation on the run. Annotations show on the pull request's checks page, so the reason for a
# red build is readable without opening (or being allowed to open) the full log.
#   ./tools/ci/run-logged.ps1 -Title "Rust build" -Command "cargo build --tests"
# The command runs through cmd.exe so stderr is merged there: cargo writes its whole report to
# stderr, and PowerShell would wrap every such line in an error record.
param(
  [Parameter(Mandatory)][string]$Title,
  [Parameter(Mandatory)][string]$Command
)

$log = Join-Path ([IO.Path]::GetTempPath()) ("ci-" + [Guid]::NewGuid().ToString("N") + ".log")
cmd /c "$Command 2>&1" | Tee-Object -FilePath $log
$code = $LASTEXITCODE
if ($code -eq 0) { exit 0 }

$signal = '^\s*(error(\[E\d+\])?:|error TS\d+|CMake Error|Could NOT find|LINK : fatal|.*: fatal error |' +
  'test .+ \.\.\. FAILED|---- .+ ----|thread .+ panicked|failures:|FAIL |AssertionError|(left|right):)'
$lines = @(Get-Content $log)
$keep = [System.Collections.Generic.List[string]]::new()
for ($i = 0; $i -lt $lines.Count -and $keep.Count -lt 60; $i++) {
  if ($lines[$i] -notmatch $signal) { continue }
  $end = [Math]::Min($i + 5, $lines.Count - 1)
  for ($j = $i; $j -le $end -and $keep.Count -lt 60; $j++) { $keep.Add($lines[$j]) }
  $i = $end
}
if ($keep.Count -eq 0) { $lines | Select-Object -Last 40 | ForEach-Object { $keep.Add($_) } }

$text = ($keep | ForEach-Object { if ($_.Length -gt 240) { $_.Substring(0, 240) } else { $_ } }) -join "`n"
$text = $text.Replace("%", "%25").Replace("`r", "%0D").Replace("`n", "%0A")
Write-Host "::error title=$Title failed (exit code $code)::$text"
exit $code
