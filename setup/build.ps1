# Build the branded "TCursor Setup" installer end to end.
#   1) Build the app + NSIS installer (main project).
#   2) Stage that NSIS .exe so the Setup app can embed it.
#   3) Build the Setup app (plain cargo build - embeds the NSIS payload, bundles the static UI).
# Output: setup\dist\TCursorSetup.exe  (a single offline installer)
$ErrorActionPreference = "Stop"
$setup = $PSScriptRoot
$root  = Split-Path -Parent $setup

Write-Host "[1/3] Building app + NSIS installer..." -ForegroundColor Cyan
Push-Location $root
try { npm run tauri build } finally { Pop-Location }

Write-Host "[2/3] Staging NSIS payload..." -ForegroundColor Cyan
$nsis = Get-ChildItem "$root\src-tauri\target\release\bundle\nsis\TCursor_*_x64-setup.exe" -ErrorAction SilentlyContinue |
  Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $nsis) { throw "NSIS installer not found - did step 1 succeed?" }
Copy-Item $nsis.FullName "$setup\src-tauri\assets\nsis-setup.exe" -Force
Write-Host ("    payload: {0} ({1:N1} MB)" -f $nsis.Name, ($nsis.Length / 1MB))

Write-Host "[3/3] Building TCursor Setup (embeds the payload)..." -ForegroundColor Cyan
Push-Location "$setup\src-tauri"
try { cargo build --release } finally { Pop-Location }

$out = "$setup\src-tauri\target\release\tcursor-setup.exe"
if (-not (Test-Path $out)) { throw "Setup exe not produced at $out" }
New-Item -ItemType Directory -Force -Path "$setup\dist" | Out-Null
Copy-Item $out "$setup\dist\TCursorSetup.exe" -Force
Write-Host ("Done -> setup\dist\TCursorSetup.exe ({0:N1} MB)" -f ((Get-Item "$setup\dist\TCursorSetup.exe").Length / 1MB)) -ForegroundColor Green
