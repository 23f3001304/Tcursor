# Puts the pinned FFmpeg build into src-tauri/resources/ (ffmpeg.exe + ffprobe.exe).
# The two executables ship inside the installer as Tauri resources but are not committed
# (they are about 100 MB each), so CI and a fresh clone fetch them with this script.
# The archive is pinned by version AND by SHA-256: bump both together, never one alone.
$ErrorActionPreference = "Stop"

$version = "8.1.1"
$sha256  = "6f58ce889f59c311410f7d2b18895b33c03456463486f3b1ebc93d97a0f54541"
$name    = "ffmpeg-$version-essentials_build"
$url     = "https://github.com/GyanD/codexffmpeg/releases/download/$version/$name.zip"

$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$dest = Join-Path $root "src-tauri\resources"
if ((Test-Path "$dest\ffmpeg.exe") -and (Test-Path "$dest\ffprobe.exe")) {
  Write-Host "FFmpeg is already in $dest"
  exit 0
}

$tmp = Join-Path ([IO.Path]::GetTempPath()) "tcursor-ffmpeg"
New-Item -ItemType Directory -Force -Path $tmp, $dest | Out-Null
$zip = Join-Path $tmp "$name.zip"

Write-Host "Downloading $url"
Invoke-WebRequest -Uri $url -OutFile $zip

$actual = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actual -ne $sha256) { throw "FFmpeg archive checksum mismatch: expected $sha256, got $actual" }

Expand-Archive -Path $zip -DestinationPath $tmp -Force
Copy-Item (Join-Path $tmp "$name\bin\ffmpeg.exe")  $dest -Force
Copy-Item (Join-Path $tmp "$name\bin\ffprobe.exe") $dest -Force
Write-Host "FFmpeg $version is in $dest"
